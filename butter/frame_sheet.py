from typing import *

from PyQt5.QtCore import *

from butter.timeline import Timeline
from butter.variable import Variable


class FrameSheet(QAbstractTableModel):
  def __init__(self, timeline: Timeline, variables: List[Variable]) -> None:
    super().__init__()
    self._timeline = timeline
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

  def data(self, index, role=Qt.DisplayRole):
    if role == Qt.DisplayRole or role == Qt.EditRole:
      state = self._timeline.frame(index.row()).value
      variable = self._variables[index.column()]
      # TODO: Argument handling
      # TODO: Formatting
      return str(variable.get(state))

  def setData(self, index, value, role=Qt.EditRole):
    if role == Qt.EditRole:
      state = self._timeline.frame(index.row()).value
      variable = self._variables[index.column()]
      # TODO: Argument handling
      # TODO: Formatting
      variable.set(int(value), state)

      del state
      self.dataChanged.emit(index, index)
      return True
