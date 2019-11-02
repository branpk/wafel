from typing import *
from typing import IO

from wafel.core.game_state import GameState
from wafel.core.variable import Variable, VariableParam, Variables, \
  VariableGroup, _FlagVariable


class Edit:
  def apply(self, state: GameState) -> None:
    raise NotImplementedError()


class VariableEdit(Edit):
  def __init__(self, variable: Variable, value: Any) -> None:
    self.variable = variable
    self.value = value

    # Don't edit hidden variables, e.g. buttons instead of A, B, Z, as then
    # the edits won't be visible to the user
    # TODO: Maybe implement Variable#contains(Variable) to handle this case instead?
    assert variable.group != VariableGroup.hidden()

  def apply(self, state: GameState) -> None:
    self.variable.set(self.value, { VariableParam.STATE: state })


def read_byte(f: IO[bytes]) -> Optional[int]:
  return (f.read(1) or [None])[0]


def read_big_short(f: IO[bytes]) -> Optional[int]:
  byte1 = read_byte(f)
  byte2 = read_byte(f)
  if byte1 is None or byte2 is None:
    return None
  return byte1 << 8 | byte2


class Edits:
  @staticmethod
  def from_m64(m64: IO[bytes], variables: Variables) -> 'Edits':
    input_button_vars = {}
    for variable in variables:
      if isinstance(variable, _FlagVariable) and variable.flags == variables['input-buttons']:
        input_button_vars[variable] = variable.flag

    edits = Edits()

    prev_buttons = 0
    prev_stick_x = 0
    prev_stick_y = 0

    m64.seek(0x400)
    frame = 0
    while True:
      buttons = read_big_short(m64)
      stick_x = read_byte(m64)
      stick_y = read_byte(m64)

      if buttons is None or stick_x is None or stick_y is None:
        break
      else:
        for variable, flag in input_button_vars.items():
          if (prev_buttons & flag) != (buttons & flag):
            edits.add(frame, VariableEdit(variable, bool(buttons & flag)))
        if stick_x != prev_stick_x:
          edits.add(frame, VariableEdit(variables['input-stick-x'], stick_x))
        if stick_y != prev_stick_y:
          edits.add(frame, VariableEdit(variables['input-stick-y'], stick_y))

      prev_buttons = buttons
      prev_stick_x = stick_x
      prev_stick_y = stick_y
      frame += 1

    return edits

  def __init__(self):
    self._items: List[List[Edit]] = []
    self.edit_frame_callbacks: List[Callable[[int], None]] = []

  def on_edit(self, callback: Callable[[int], None]) -> None:
    self.edit_frame_callbacks.append(callback)

  def extend(self, new_len: int) -> None:
    while len(self._items) < new_len:
      self._items.append([])

  def _invalidate(self, frame: int) -> None:
    for callback in list(self.edit_frame_callbacks):
      callback(frame)

  def add(self, frame: int, edit: Edit) -> None:
    # TODO: Remove overwritten edits
    self._get_edits(frame).append(edit)
    self._invalidate(frame)

  def _get_edits(self, frame: int) -> List[Edit]:
    while frame >= len(self._items):
      self._items.append([])
    return self._items[frame]

  def apply(self, state: GameState) -> None:
    for edit in self._get_edits(state.frame):
      edit.apply(state)

  def is_edited(self, frame: int, variable: Variable) -> bool:
    for edit in self._get_edits(frame):
      if isinstance(edit, VariableEdit) and edit.variable == variable:
        return True
    return False

  def reset(self, frame: int, variable: Variable) -> None:
    edits = self._get_edits(frame)
    for edit in list(edits):
      if isinstance(edit, VariableEdit) and edit.variable == variable:
        edits.remove(edit)
    self._invalidate(frame)
