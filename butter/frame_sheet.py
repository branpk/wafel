from PyQt5.QtCore import *

from butter.timeline import Timeline


class FrameSheet(QAbstractTableModel):
  def __init__(self, timeline: Timeline) -> None:
    super().__init__()
    self._timeline = timeline

  def rowCount(self, parent=None) -> int:
    return len(self._timeline)

  def columnCount(self, parent=None) -> int:
    return 2

  def flags(self, index):
    return Qt.ItemIsSelectable | Qt.ItemIsEditable | Qt.ItemIsEnabled

  def data(self, index, role=Qt.DisplayRole):
    if role == Qt.DisplayRole or role == Qt.EditRole:
      state = self._timeline.frame(index.row()).value
      if index.column() == 0:
        return str(state.frame)
      else:
        return 'X'

  def setData(self, index, value, role=Qt.EditRole):
    # self.data[(index.row(), index.column())] = value
    self.dataChanged.emit(index, index)
    return True
