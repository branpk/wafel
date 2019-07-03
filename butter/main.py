from typing import cast
from ctypes import *
import json

from PyQt5.QtWidgets import *
from PyQt5.QtCore import *
from PyQt5.QtGui import *

import butter.graphics as graphics
from butter.timeline import Timeline
from butter.input_sequence import InputSequence
from butter.reactive import ReactiveValue
from butter.frame_sheet import FrameSheet
from butter.variable import create_variables


class Model:
  def __init__(self):
    self.lib = cdll.LoadLibrary('lib/sm64plus/us/sm64plus')
    with open('lib/sm64plus/us/sm64plus.json', 'r') as f:
      self.spec: dict = json.load(f)

    with open('test_files/120_u.m64', 'rb') as m64:
      self.inputs = InputSequence.from_m64(m64)

    self.timeline = Timeline(self.lib, self.spec, self.inputs)
    self.selected_frame = ReactiveValue(0)
    self.timeline.add_hotspot(self.selected_frame)

    self.variables = create_variables(self.spec)

    # TODO: Frame sheet var list
    self.frame_sheet = FrameSheet(self.timeline, self.inputs, self.variables.variables)


class Window(QWidget):
  def __init__(self, parent=None):
    super().__init__(parent)

    self.setWindowTitle('SM64')

    self.model = Model()

    layout = QHBoxLayout()
    layout.setContentsMargins(0, 0, 0, 0)

    visualizer_layout = QVBoxLayout()
    visualizer_layout.addWidget(GameView(self.model))
    visualizer_layout.addWidget(FrameSlider(self.model))
    layout.addLayout(visualizer_layout)

    layout.addWidget(FrameSheetView(self.model))

    self.setLayout(layout)

    self.balance_timer = QTimer()
    self.balance_timer.setInterval(1000 // 30)
    self.balance_timer.timeout.connect(lambda: self.model.timeline.balance_distribution(1/60))
    self.balance_timer.start()


class FrameSheetView(QTableView):
  def __init__(self, model: Model, parent=None):
    super().__init__(parent)
    self.model = model

    self.setModel(self.model.frame_sheet)
    self.setVerticalScrollMode(QAbstractItemView.ScrollPerPixel)
    self.setHorizontalScrollMode(QAbstractItemView.ScrollPerPixel)
    self.setSelectionMode(QAbstractItemView.SingleSelection)

    self.setMinimumWidth(640)
    self.setMinimumHeight(480)

    def set_selection(frame):
      self.selectionModel().select(self.model.frame_sheet.index(frame, 0), QItemSelectionModel.ClearAndSelect)
    set_selection(self.model.selected_frame.value)
    # self.model.selected_frame.on_change(set_selection)

    self.focus_frame = ReactiveValue(0)
    def set_focus_frame():
      self.focus_frame.value = self.rowAt(self.contentsRect().y())
    self.verticalScrollBar().valueChanged.connect(set_focus_frame)
    set_focus_frame()

    self.model.timeline.add_hotspot(self.focus_frame)

  def selectionChanged(self, selected, deselected):
    frame = min((index.row() for index in selected.indexes()), default=0)
    self.model.selected_frame.value = frame


# TODO: Either make frame slider not controllable, or make frame sheet change
# selection to match
class FrameSlider(QSlider):

  def __init__(self, model: Model, parent=None):
    super().__init__(Qt.Horizontal, parent)
    self.model = model

    self.setMinimum(0)
    self.setMaximum(len(self.model.timeline))

    # slider value <- selected frame
    self.model.selected_frame.on_change(lambda value: self.setValue(value))
    self.setValue(self.value())

    # selected frame <- value
    def slider_value_changed(value):
      self.model.selected_frame.value = value
    self.valueChanged.connect(slider_value_changed)

  def paintEvent(self, event):
    super().paintEvent(event)

    painter = QPainter(self)
    for frame in self.model.timeline.get_loaded_frames():
      x = (self.contentsRect().width() - 11) / self.maximum() * frame + 5
      painter.fillRect(x, 0, 1, 20, Qt.red)

    self.update()


class GameView(QOpenGLWidget):

  def __init__(self, model: Model, parent=None):
    super().__init__(parent)
    self.model = model

    self.setMinimumSize(640, 480)

    self.state = self.model.timeline.frame(self.model.selected_frame)
    self.state.on_change(lambda _: self.update())

  def initializeGL(self):
    graphics.load_gl()

  def paintGL(self):
    self.makeCurrent()
    graphics.render(self.state.value)


def run():
  app = QApplication([])
  window = Window()
  window.adjustSize()
  window.show()
  app.exec_()
