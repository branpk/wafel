import ctypes

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
  def valid(self) -> bool:
    return self._valid

  @property
  def frame(self) -> int:
    return self._frame

  @property
  def addr(self) -> int:
    return self._addr


class StateManager:

  def __init__(self, lib: ctypes.CDLL, spec: dict, capacity: int) -> None:
    self._lib = lib
    self._spec = spec

    self._all_cells = [dcast(int, lib.sm64_state_new()) for _ in range(capacity)]
    self._free_cells = list(self._all_cells)
    self._allocated_states = []

    self._base_cell = self._free_cells.pop()
    self._temp_cell = self._free_cells.pop()

    self._power_on_state = State(self, 0, self._base_cell)
    self._loaded_state = self._power_on_state


  def __del__(self):
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


  def _allocate_state(self) -> State:
    if len(self._free_cells) > 0:
      # It doesn't really matter which state we use as a template, so we use
      # the loaded state
      new_cell = self._free_cells.pop()
      self._lib.sm64_state_raw_copy(new_cell, self._loaded_state._addr)

      new_state = State(self, self._loaded_state._frame, new_cell)

    else:
      # Delete the currently loaded state, unless it's the power-on state
      # TODO: Better selection algorithm (use target frame hint)
      if self._loaded_state in self._allocated_states:
        old_state = self._loaded_state
      else:
        old_state = self._allocated_states[0]

      old_state._valid = False
      self._allocated_states.remove(old_state)

      # Create a new state with the deleted state's cell
      new_state = State(self, old_state._frame, old_state._addr)

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
