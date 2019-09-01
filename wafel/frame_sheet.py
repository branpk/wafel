from typing import *
import traceback
import sys

import PyQt5.Qt as Qt
from PyQt5.QtCore import *

from wafel.timeline import Timeline
from wafel.variable import VariableParam, Variable
from wafel.variable_format import Formatters, CheckboxFormatter
from wafel.edit import Edits, VariableEdit
from wafel.util import *


class FrameSheet(QAbstractTableModel):

  # TODO: Let depend on model
  def __init__(self, timeline: Timeline, edits: Edits, formatters: Formatters, variables: List[Variable]) -> None:
    super().__init__()
    self._timeline = timeline
    self._edits = edits
    self._formatters = formatters
    self._variables = variables # TODO: Reactive variable list

    self._edits.latest_edited_frame.on_change(lambda:
      self.dataChanged.emit(
        self.index(0, 0),
        self.index(self.rowCount() - 1, self.columnCount() - 1)
      )
    )

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
    variable = self._variables[index.column()]
    if isinstance(self._formatters[variable], CheckboxFormatter):
      return Qt.ItemIsSelectable | Qt.ItemIsUserCheckable | Qt.ItemIsEnabled
    else:
      return Qt.ItemIsSelectable | Qt.ItemIsEditable | Qt.ItemIsEnabled

  def data(self, index, role=Qt.DisplayRole):
    variable = self._variables[index.column()]
    state = self._timeline.frame(index.row()).value
    args = { VariableParam.STATE: state }
    formatter = self._formatters[variable]
    value = formatter.output(variable.get(args))

    if role == Qt.DisplayRole or role == Qt.EditRole:
      if not isinstance(formatter, CheckboxFormatter):
        return value

    elif role == Qt.CheckStateRole:
      if isinstance(formatter, CheckboxFormatter):
        return Qt.Checked if value else Qt.Unchecked

  def setData(self, index, value, role=Qt.EditRole):
    variable = self._variables[index.column()]
    formatter = self._formatters[variable]
    rep = None

    if role == Qt.EditRole:
      if not isinstance(formatter, CheckboxFormatter):
        rep = value

    elif role == Qt.CheckStateRole:
      if isinstance(formatter, CheckboxFormatter):
        rep = value == Qt.Checked

    if rep is not None:
      try:
        value = formatter.input(rep)
      except Exception:
        sys.stderr.write(traceback.format_exc())
        sys.stderr.flush()
        return False
      self._edits.add(index.row(), VariableEdit(variable, value))
      self.dataChanged.emit(index, index)
      return True

    return False