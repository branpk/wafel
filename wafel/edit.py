import ctypes
from typing import IO, Any, Optional, List

from wafel.game_state import GameState
from wafel.variable import Variable, VariableParam, Variables
from wafel.reactive import Reactive, ReactiveValue


class Edit:
  def apply(self, state: GameState) -> None:
    raise NotImplementedError()


class VariableEdit(Edit):
  def __init__(self, variable: Variable, value: Any) -> None:
    # TODO: Extra args (e.g. object id)
    assert variable.params == [VariableParam.STATE]
    self.variable = variable
    self.value = value

  def apply(self, state: GameState) -> None:
    self.variable.set(self.value, state)


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
    edits = Edits()

    m64.seek(0x400)
    frame = 0
    while True:
      buttons = read_big_short(m64)
      stick_x = read_byte(m64)
      stick_y = read_byte(m64)
      if buttons is None or stick_x is None or stick_y is None:
        break
      else:
        edits.add(frame, VariableEdit(variables.by_name['buttons'], buttons))
        edits.add(frame, VariableEdit(variables.by_name['stick x'], stick_x))
        edits.add(frame, VariableEdit(variables.by_name['stick y'], stick_y))
      frame += 1

    return edits

  def __init__(self):
    self._items: List[List[Edit]] = []
    self.latest_edited_frame = ReactiveValue(-1)

  def add(self, frame: int, edit: Edit) -> None:
    # TODO: Remove overwritten edits
    self._get_edits(frame).append(edit)
    self.latest_edited_frame.value = frame

  def _get_edits(self, frame: int) -> List[Edit]:
    while frame >= len(self._items):
      self._items.append([])
    return self._items[frame]

  def apply(self, state: GameState) -> None:
    for edit in self._get_edits(state.frame):
      edit.apply(state)
