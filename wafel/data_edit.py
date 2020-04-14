from typing import *

from wafel.variable import Variable, VariableWriter, VariableReader
from wafel.core import Controller, SlotState, Timeline
from wafel.data_variables import DataVariables


# TODO: Move this somewhere:
#   Don't edit hidden variables, e.g. buttons instead of A, B, Z, as then
#   the edits won't be visible to the user
# TODO: Maybe implement Variable#contains(Variable) to handle this case instead?
# This might also help with input modes
# assert variable.group != VariableGroup.hidden()


class DataEdit:
  def __init__(self, variable: Variable, value: Any) -> None:
    self.variable = variable
    self.value = value


class DataEdits(VariableWriter, Controller):
  def __init__(self, data_variables: DataVariables):
    super().__init__()
    self._frames: List[List[DataEdit]] = []
    self._data_variables = data_variables

  def insert_frame(self, index: int) -> None:
    while len(self._frames) <= index:
      self._frames.append([])
    self._frames.insert(index, [])
    self.notify(index)

  def delete_frame(self, index: int) -> None:
    if index < len(self._frames):
      del self._frames[index]
      self.notify(index - 1)

  def get_edits(self, frame: int) -> List[DataEdit]:
    while frame >= len(self._frames):
      self._frames.append([])
    return self._frames[frame]

  def write(self, variable: Variable, value: object) -> None:
    frame: int = variable.args['frame']
    edits = self.get_edits(frame)
    for edit in list(edits):
      if edit.variable == variable:
        edits.remove(edit)
    edits.append(DataEdit(variable.without('frame'), value))
    self.notify(frame)

  def unsafe_edit(self, variable: Variable, value: object) -> None:
    frame: int = variable.args['frame']
    self.get_edits(frame).append(DataEdit(variable.without('frame'), value))

  def edited(self, variable: Variable) -> bool:
    frame: int = variable.args['frame']
    variable = variable.without('frame')
    return any(edit.variable == variable for edit in self.get_edits(frame))

  def reset(self, variable: Variable) -> None:
    frame: int = variable.args['frame']
    variable = variable.without('frame')
    edits = self.get_edits(frame)
    for edit in list(edits):
      if edit.variable == variable:
        edits.remove(edit)
    self.notify(frame)

  def apply(self, state: SlotState) -> None:
    for edit in self.get_edits(state.frame):
      self._data_variables.set(state, edit.variable, edit.value)


class DataReader(VariableReader):
  def __init__(self, data_variables: DataVariables, timeline: Timeline) -> None:
    self._data_variables = data_variables
    self._timeline = timeline

  def read(self, variable: Variable) -> object:
    frame: int = variable.args['frame']
    return self._data_variables.get(self._timeline[frame], variable)
