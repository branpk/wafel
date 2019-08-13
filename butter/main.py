from typing import *
from ctypes import *
import json
import math

from PyQt5.QtWidgets import *
from PyQt5.QtCore import *
from PyQt5.QtGui import *

import butter.graphics as graphics
from butter.graphics import CameraMode, RenderInfo, Camera, RotateCamera
from butter.timeline import Timeline
from butter.edit import Edits
from butter.reactive import Reactive, ReactiveValue
from butter.frame_sheet import FrameSheet
from butter.variables import create_variables
from butter.game_state import GameState
from butter.data_path import DataPath


class Model:
  def __init__(self):
    self.lib = cdll.LoadLibrary('lib/libsm64/us/sm64')
    with open('lib/libsm64/us/libsm64.json', 'r') as f:
      self.spec: dict = json.load(f)

    self.variables = create_variables(self.spec)

    with open('test_files/120_u.m64', 'rb') as m64:
      self.edits = Edits.from_m64(m64, self.variables)

    self.timeline = Timeline(self.lib, self.spec, self.edits)
    self.selected_frame = ReactiveValue(0)
    self.timeline.add_hotspot(self.selected_frame)

    # TODO: Frame sheet var list
    self.frame_sheet = FrameSheet(self.timeline, self.edits, self.variables.variables)

  def path(self, path: str) -> DataPath:
    return DataPath.parse(self.spec, path)


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
    self.setMouseTracking(True)

    self.state = self.model.timeline.frame(self.model.selected_frame)

    self.mouse_down = ReactiveValue(False)
    self.mouse_pos: ReactiveValue[Optional[Tuple[int, int]]] = ReactiveValue(None)
    self.zoom = ReactiveValue(0.0)
    self.total_drag = ReactiveValue((0, 0))

    def compute_camera(state: GameState, zoom: float, total_drag: Tuple[int, int]) -> Camera:
      camera = RotateCamera(
        [0.0, 0.0, 0.0],
        -total_drag[1] / 200,
        -total_drag[0] / 200,
        math.radians(45),
      )

      target = self.model.path('$state.gMarioState[].pos').get(state)
      face_dir = camera.face_dir()
      offset_dist = 1500 * math.pow(0.5, zoom)
      camera.pos = [target[i] - offset_dist * face_dir[i] for i in range(3)]

      return camera
    self.camera = Reactive.tuple(self.state, self.zoom, self.total_drag).mapn(compute_camera)

    Reactive.tuple(self.state, self.camera).on_change(lambda: self.update())

  def initializeGL(self):
    graphics.load_gl()

  def paintGL(self):
    self.makeCurrent()
    graphics.render(RenderInfo(
      self.camera.value,
      self.state.value,
    ))

  def wheelEvent(self, event):
    self.zoom.value += event.angleDelta().y() / 500

  def mousePressEvent(self, event):
    if event.button() == Qt.LeftButton:
      self.mouse_down.value = True

  def mouseReleaseEvent(self, event):
    if event.button() == Qt.LeftButton:
      self.mouse_down.value = False

  def mouseMoveEvent(self, event):
    last_mouse_pos = self.mouse_pos.value
    mouse_pos = (event.pos().x(), event.pos().y())

    if self.mouse_down.value and last_mouse_pos is not None:
      drag = (mouse_pos[0] - last_mouse_pos[0], mouse_pos[1] - last_mouse_pos[1])
      total_drag = self.total_drag.value
      self.total_drag.value = (total_drag[0] + drag[0], total_drag[1] + drag[1])

    self.mouse_pos.value = mouse_pos


def run():
  fmt = QSurfaceFormat()
  fmt.setSamples(4)
  QSurfaceFormat.setDefaultFormat(fmt)

  app = QApplication([])
  window = Window()
  window.adjustSize()
  window.show()
  app.exec_()
