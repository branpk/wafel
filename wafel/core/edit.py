from typing import *
from typing import IO
import struct

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


class Edits:
  def __init__(self):
    self._items: List[List[Edit]] = []
    self.edit_frame_callbacks: List[Callable[[int], None]] = []

  def on_edit(self, callback: Callable[[int], None]) -> None:
    self.edit_frame_callbacks.append(callback)

  def extend(self, new_len: int) -> None:
    while len(self._items) < new_len:
      self._items.append([])

  def insert_frame(self, index: int) -> None:
    self.extend(index)
    self._items.insert(index, [])
    self._invalidate(index)

  def delete_frame(self, index: int) -> None:
    if index < len(self._items):
      del self._items[index]
      self._invalidate(index - 1)

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
