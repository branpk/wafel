from typing import *
import traceback

from PyQt5.QtCore import *

from wafel.timeline import Timeline
from wafel.variable import VariableParam, VariableInstance, CheckboxFormatter
from wafel.edit import Edits, VariableEdit
from wafel.util import *


class FrameSheet(QAbstractTableModel):
  def __init__(self, timeline: Timeline, edits: Edits, variables: List[VariableInstance]) -> None:
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
        return self._variables[section].display_name
      else:
        return str(section)

  def flags(self, index):
    var = self._variables[index.column()]
    if isinstance(var.formatter, CheckboxFormatter):
      return Qt.ItemIsSelectable | Qt.ItemIsUserCheckable | Qt.ItemIsEnabled
    else:
      return Qt.ItemIsSelectable | Qt.ItemIsEditable | Qt.ItemIsEnabled

  def data(self, index, role=Qt.DisplayRole):
    var = self._variables[index.column()]
    state = self._timeline.frame(index.row()).value
    value = var.formatter.output(var.variable.get(state))

    if role == Qt.DisplayRole or role == Qt.EditRole:
      if not isinstance(var.formatter, CheckboxFormatter):
        return value

    elif role == Qt.CheckStateRole:
      if isinstance(var.formatter, CheckboxFormatter):
        return Qt.Checked if value else Qt.Unchecked

  def setData(self, index, value, role=Qt.EditRole):
    var = self._variables[index.column()]
    rep = None

    if role == Qt.EditRole:
      if not isinstance(var.formatter, CheckboxFormatter):
        rep = value

    elif role == Qt.CheckStateRole:
      if isinstance(var.formatter, CheckboxFormatter):
        rep = value == Qt.Checked

    if rep is not None:
      try:
        value = var.formatter.input(rep)
      except Exception:
        sys.stderr.write(traceback.format_exc())
        sys.stderr.flush()
      self._edits.add(index.row(), VariableEdit(var.variable, value))
      self.dataChanged.emit(index, index)
      return True

    return False
