from typing import *

from wafel.core.game_state import GameState
from wafel.core.variable import Variable, VariableParam, VariableGroup


class _Edit:
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
    self._frames: List[List[_Edit]] = []
    self._on_edit_callbacks: List[Callable[[int], None]] = []

  def on_edit(self, callback: Callable[[int], None]) -> None:
    self._on_edit_callbacks.append(callback)

  def _invalidate(self, frame: int) -> None:
    for callback in list(self._on_edit_callbacks):
      callback(frame)

  def __len__(self) -> int:
    return len(self._frames)

  def extend(self, new_len: int) -> None:
    while len(self._frames) < new_len:
      self._frames.append([])

  def insert_frame(self, index: int) -> None:
    self.extend(index)
    self._frames.insert(index, [])
    self._invalidate(index)

  def delete_frame(self, index: int) -> None:
    if index < len(self._frames):
      del self._frames[index]
      self._invalidate(index - 1)

  def _get_edits(self, frame: int) -> List[_Edit]:
    while frame >= len(self._frames):
      self._frames.append([])
    return self._frames[frame]

  def get_edits(self, frame: int) -> List[Tuple[Variable, Any]]:
    return [(e.variable, e.value) for e in self._get_edits(frame)]

  def edit(self, frame: int, variable: Variable, value: Any) -> None:
    # TODO: Remove overwritten edits
    self._get_edits(frame).append(_Edit(variable, value))
    self._invalidate(frame)

  def is_edited(self, frame: int, variable: Variable) -> bool:
    for edit in self._get_edits(frame):
      if edit.variable == variable:
        return True
    return False

  def reset(self, frame: int, variable: Variable) -> None:
    edits = self._get_edits(frame)
    for edit in list(edits):
      if edit.variable == variable:
        edits.remove(edit)
    self._invalidate(frame)

  def apply(self, state: GameState) -> None:
    for edit in self._get_edits(state.frame):
      edit.apply(state)
