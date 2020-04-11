from typing import *

from wafel.variable import Variable


# TODO: Move this somewhere
# Don't edit hidden variables, e.g. buttons instead of A, B, Z, as then
# the edits won't be visible to the user
# TODO: Maybe implement Variable#contains(Variable) to handle this case instead?
# This might also help with input modes
# assert variable.group != VariableGroup.hidden()


class Edit:
  def __init__(self, variable: Variable, value: Any) -> None:
    self.variable = variable
    self.value = value


class Edits:
  def __init__(self):
    self._frames: List[List[Edit]] = []
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

  def get_edits(self, frame: int) -> List[Edit]:
    while frame >= len(self._frames):
      self._frames.append([])
    return self._frames[frame]

  def edit(self, variable: Variable, value: object) -> None:
    frame: int = variable.args['frame']
    edits = self.get_edits(frame)
    for edit in list(edits):
      if edit.variable == variable:
        edits.remove(edit)
    edits.append(Edit(variable, value))
    self._invalidate(frame)

  def unsafe_edit(self, variable: Variable, value: object) -> None:
    frame: int = variable.args['frame']
    self.get_edits(frame).append(Edit(variable, value))

  def edited(self, variable: Variable) -> bool:
    frame: int = variable.args['frame']
    return any(edit.variable == variable for edit in self.get_edits(frame))

  def reset(self, variable: Variable) -> None:
    frame: int = variable.args['frame']
    edits = self.get_edits(frame)
    for edit in list(edits):
      if edit.variable == variable:
        edits.remove(edit)
    self._invalidate(frame)
