import ctypes
import random
from typing import List, IO, Optional

from butter.util import dcast


class State:

  def __init__(self, manager: 'StateManager', frame: int, addr: int) -> None:
    self._manager = manager
    self._frame = frame
    self._addr = addr
    self._valid = True

  def advance(self) -> None:
    self._manager._advance_state(self)

  def touch(self) -> None:
    self._manager._touch_state(self)

  @property
  def lib(self) -> ctypes.CDLL:
    return self._manager._lib

  @property
  def spec(self) -> dict:
    return self._manager._spec

  @property
  def valid(self) -> bool:
    return self._valid

  @property
  def frame(self) -> int:
    return self._frame

  @property
  def addr(self) -> int:
    return self._addr


class SequenceItem:
  def apply(self, state: State) -> None:
    raise NotImplementedError()


class Input(SequenceItem):
  def __init__(self, stick_x: int, stick_y: int, buttons: int):
    self.stick_x = stick_x
    self.stick_y = stick_y
    self.buttons = buttons

  def apply(self, state: State) -> None:
    # TODO: Better system for this
    globals = state.spec['types']['struct']['SM64State']['fields']

    controller = state.addr + globals['gControllerPads']['offset']
    os_cont_pad = state.spec['types']['typedef']['OSContPad']['fields']
    controller_button = ctypes.cast(controller + os_cont_pad['button']['offset'], ctypes.POINTER(ctypes.c_uint16))
    controller_stick_x = ctypes.cast(controller + os_cont_pad['stick_x']['offset'], ctypes.POINTER(ctypes.c_int8))
    controller_stick_y = ctypes.cast(controller + os_cont_pad['stick_y']['offset'], ctypes.POINTER(ctypes.c_int8))

    controller_button[0] = self.buttons
    controller_stick_x[0] = self.stick_x
    controller_stick_y[0] = self.stick_y


def read_byte(f: IO[bytes]) -> Optional[int]:
  return (f.read(1) or [None])[0]

def read_big_short(f: IO[bytes]) -> Optional[int]:
  byte1 = read_byte(f)
  byte2 = read_byte(f)
  if byte1 is None or byte2 is None:
    return None
  return byte1 << 8 | byte2


class InputSequence:
  @staticmethod
  def from_m64(m64: IO[bytes]) -> 'InputSequence':
    inputs = InputSequence()

    m64.seek(0x400)
    i = 0
    while True:
      buttons = read_big_short(m64)
      stick_x = read_byte(m64)
      stick_y = read_byte(m64)
      if buttons is None or stick_x is None or stick_y is None:
        break
      else:
        inputs[i].append(Input(stick_x, stick_y, buttons))
      i += 1

    return inputs

  def __init__(self):
    self._items: List[List[SequenceItem]] = []

  def __getitem__(self, frame: int) -> List[SequenceItem]:
    while frame >= len(self._items):
      self._items.append([])
    return self._items[frame]

  def apply(self, state: State) -> None:
    for item in self[state.frame]:
      item.apply(state)


class StateManager:

  def __init__(self, lib: ctypes.CDLL, spec: dict, inputs: InputSequence, capacity: int) -> None:
    self._lib = lib
    self._spec = spec
    self._inputs = inputs

    self._all_cells = [dcast(int, lib.sm64_state_new()) for _ in range(capacity)]
    self._free_cells = list(self._all_cells)
    self._allocated_states = []

    self._base_cell = self._free_cells.pop()
    self._temp_cell = self._free_cells.pop()

    self._power_on_state = State(self, 0, self._base_cell)
    self._loaded_state = self._power_on_state


  def __del__(self) -> None:
    for cell in self._all_cells:
      self._lib.sm64_state_delete(cell)


  def _load_state(self, state: State) -> None:
    assert state._valid
    assert state._manager is self

    if self._loaded_state is not state:
      # Swap the base cell with state
      self._lib.sm64_state_raw_copy(self._temp_cell, self._base_cell)
      self._lib.sm64_state_raw_copy(self._base_cell, state._addr)
      self._lib.sm64_state_raw_copy(state._addr, self._temp_cell)

      self._loaded_state._addr = state._addr
      state._addr = self._base_cell

      self._loaded_state = state


  def _advance_state(self, state: State) -> None:
    assert state._valid
    assert state in self._allocated_states

    self._load_state(state)
    # TODO: Input/hack sequence
    self._inputs.apply(state)
    self._lib.sm64_state_update(self._base_cell)
    state._frame += 1


  def _touch_state(self, state: State) -> None:
    assert state._valid
    assert state in self._allocated_states

    # Delete all later states, as well as other states on the same frame
    for later_state in self._allocated_states:
      if later_state is not state and later_state._frame >= state._frame:
        later_state._valid = False

    self._allocated_states = [st for st in self._allocated_states if st._valid]
    # TODO: Check if loaded_state got removed


  def _allocate_state(self) -> State:
    if len(self._free_cells) > 0:
      # It doesn't really matter which state we use as a template, so we use
      # the loaded state
      new_cell = self._free_cells.pop()
      self._lib.sm64_state_raw_copy(new_cell, self._loaded_state._addr)

      new_state = State(self, self._loaded_state._frame, new_cell)

    else:
      # Select a state to delete
      # TODO: Better selection algorithm
      old_state = random.choice(self._allocated_states)

      old_state._valid = False
      self._allocated_states.remove(old_state)

      # Create a new state with the deleted state's cell
      new_state = State(self, old_state._frame, old_state._addr)

      if self._loaded_state is old_state:
        self._loaded_state = new_state

    self._allocated_states.append(new_state)
    return new_state


  def new_state(self, frame: int) -> State:
    new_state = self._allocate_state()

    if new_state._frame > frame or new_state._frame < frame - 50: # TODO: Tweak
      # We can't or don't want to use this state. Find a good state to use
      pred_state = max(
          [self._power_on_state] + [st for st in self._allocated_states if st._frame <= frame],
          key=lambda st: st._frame)

      # Copy it to the new state
      self._lib.sm64_state_raw_copy(new_state._addr, pred_state._addr)
      new_state._frame = pred_state._frame

    while new_state._frame < frame:
      new_state.advance()

    return new_state


  def get_loaded_frames(self) -> List[int]:
    return [state._frame for state in self._allocated_states]


  # def debug_check_frame(self, state: State) -> bool:
  #   assert state.valid
  #   globals = self._spec['types']['struct']['SM64State']['fields']
  #   global_timer = ctypes.cast(state.addr + globals['gGlobalTimer']['offset'], ctypes.POINTER(ctypes.c_uint32))
  #   return state.frame + 1 == global_timer[0]
