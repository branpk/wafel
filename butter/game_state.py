import ctypes
import random
from typing import List, IO, Optional
import weakref

from butter.util import dcast


class Cell:
  def __init__(self, frame: int, addr: int) -> None:
    self.frame = frame
    self.addr = addr


class GameState:
  def __init__(self, spec: dict, cell: Cell) -> None:
    self.spec = spec
    self.cell = cell

  @property
  def frame(self) -> int:
    return self.cell.frame

  @property
  def addr(self) -> int:
    return self.cell.addr


class SequenceItem:
  def apply(self, state: GameState) -> None:
    raise NotImplementedError()


class Input(SequenceItem):
  def __init__(self, stick_x: int, stick_y: int, buttons: int):
    self.stick_x = stick_x
    self.stick_y = stick_y
    self.buttons = buttons

  def apply(self, state: GameState) -> None:
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

  def apply(self, state: GameState) -> None:
    for item in self[state.frame]:
      item.apply(state)


class GameStateManager:

  def __init__(
    self,
    lib: ctypes.CDLL,
    spec: dict,
    inputs: InputSequence,
    capacity: int,
  ) -> None:
    self.lib = lib
    self.spec = spec
    self.inputs = inputs
    self.capacity = capacity

    self.cells = [self.new_cell() for _ in range(capacity)]
    self.temp_cell = self.cells.pop()

    self.base_cell = self.cells[0]
    for cell in self.cells:
      self.copy_cell(cell, self.base_cell, unsafe=True)
    self.loaded_states: List[weakref.ref[GameState]] = []

    self.power_on_cell = self.cells[1]

  def any_states_loaded(self) -> bool:
    self.loaded_states = [st for st in self.loaded_states if st() is not None]
    return len(self.loaded_states) > 0

  def can_modify_cell(self, cell: Cell) -> bool:
    if cell is self.base_cell:
      return not self.any_states_loaded()
    elif cell is self.power_on_cell:
      return False
    else:
      return True

  def new_cell(self) -> Cell:
    addr = dcast(int, self.lib.sm64_state_new())
    return Cell(0, addr)

  def copy_cell(self, dst: Cell, src: Cell, unsafe: bool = False) -> None:
    if not unsafe:
      assert self.can_modify_cell(dst)
    if src is not dst:
      self.lib.sm64_state_raw_copy(dst.addr, src.addr)
      dst.frame = src.frame

  def advance_base_cell(self) -> None:
    assert self.can_modify_cell(self.base_cell)

    temp_state = GameState(self.spec, self.base_cell)
    self.inputs.apply(temp_state)

    self.lib.sm64_state_update(self.base_cell.addr)
    self.base_cell.frame += 1

  def swap_cell_contents(self, cell1: Cell, cell2: Cell) -> None:
    if cell1 is not cell2:
      self.copy_cell(self.temp_cell, cell1)
      self.copy_cell(cell1, cell2)
      self.copy_cell(cell2, self.temp_cell)

  def request_frame(self, frame: int) -> Optional[GameState]:
    # Move contents of base_cell to a random location
    # TODO: Can probably remove if we use a background distribution algorithm
    free_cells = [cell for cell in self.cells if self.can_modify_cell(cell)]
    assert len(free_cells) > 0
    self.swap_cell_contents(self.base_cell, random.choice(free_cells))

    # Load a state as close to the desired frame as possible
    usable_cells = [cell for cell in self.cells if cell.frame <= frame]
    self.copy_cell(self.base_cell, max(usable_cells, key=lambda cell: cell.frame))

    while self.base_cell.frame < frame:
      self.advance_base_cell()

    state = GameState(self.spec, self.base_cell)
    self.loaded_states.append(weakref.ref(state))
    return state

  def get_loaded_frames(self) -> List[int]:
    return [cell.frame for cell in self.cells]
