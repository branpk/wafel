from typing import *

from PyQt5.QtCore import *

from butter.timeline import Timeline
from butter.variable import Variable, VariableParam, VariableDataType
from butter.edit import Edits, VariableEdit
from butter.game_state import GameState
from butter.util import *


class FrameSheet(QAbstractTableModel):
  def __init__(self, timeline: Timeline, edits: Edits, variables: List[Variable]) -> None:
    super().__init__()
    self._timeline = timeline
    self._edits = edits
    self._variables = variables # TODO: Reactive variable list

  def rowCount(self, parent=None) -> int:
    return len(self._timeline)

  def columnCount(self, parent=None) -> int:
    return len(self._variables)

  def headerData(self, section, orientation, role=Qt.DisplayRole):
    if role == Qt.DisplayRole:
      if orientation == Qt.Horizontal:
        return self._variables[section].name
      else:
        return str(section)

  def flags(self, index):
    return Qt.ItemIsSelectable | Qt.ItemIsEditable | Qt.ItemIsEnabled

  def get_variable_arg(self, frame: int, param: VariableParam) -> Any:
    if param == VariableParam.STATE:
      return self._timeline.frame(frame).value
    else:
      raise NotImplementedError

  def get_variable_args(self, frame: int, variable: Variable) -> List[Any]:
    return [self.get_variable_arg(frame, param) for param in variable.params]

  def data(self, index, role=Qt.DisplayRole):
    if role == Qt.DisplayRole or role == Qt.EditRole:
      variable = self._variables[index.column()]
      args = self.get_variable_args(index.row(), variable)
      value = variable.get(*args)
      # TODO: Formatting
      if variable.data_type == VariableDataType.BOOL:
        return '1' if value else ''
      else:
        return str(value)

  def setData(self, index, value, role=Qt.EditRole):
    if role == Qt.EditRole:
      variable = self._variables[index.column()]
      # TODO: Formatting
      if variable.data_type == VariableDataType.BOOL:
        value = bool(value)
      else:
        value = int(value)

      self._edits.add(index.row(), VariableEdit(variable, value))

      self.dataChanged.emit(index, index)
      return True
