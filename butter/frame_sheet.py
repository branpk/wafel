from typing import *

from PyQt5.QtCore import *

from butter.timeline import Timeline
from butter.variable import Variable, VariableParam, VariableDataType
from butter.input_sequence import InputSequence


class FrameSheet(QAbstractTableModel):
  def __init__(self, timeline: Timeline, inputs: InputSequence, variables: List[Variable]) -> None:
    super().__init__()
    self._timeline = timeline
    self._inputs = inputs
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
    elif param == VariableParam.INPUT:
      return self._inputs.get_input(frame)
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
      args = self.get_variable_args(index.row(), variable)
      # TODO: Formatting
      if variable.data_type == VariableDataType.BOOL:
        variable.set(bool(value), *args)
      else:
        variable.set(int(value), *args)

      del args
      self.dataChanged.emit(index, index)
      return True
