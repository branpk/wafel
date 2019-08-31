from typing import *
from ctypes import *
import json
import math
import sys
import traceback

from PyQt5.QtWidgets import *
from PyQt5.QtCore import *
from PyQt5.QtGui import *

from wafel.graphics import *
from wafel.timeline import Timeline
from wafel.edit import Edits, VariableEdit
from wafel.reactive import Reactive, ReactiveValue
from wafel.frame_sheet import FrameSheet
from wafel.variables import create_variables
from wafel.game_state import GameState
from wafel.data_path import DataPath
from wafel.variable import *
from wafel.variable_format import Formatters, VariableFormatter


class Model:
  def __init__(self):
    self.lib = cdll.LoadLibrary('lib/libsm64/jp/sm64')
    with open('lib/libsm64/jp/libsm64.json', 'r') as f:
      self.spec: dict = json.load(f)

    self.variables = create_variables(self.spec)
    self.formatters = Formatters()

    with open('test_files/1key_j.m64', 'rb') as m64:
      self.edits = Edits.from_m64(m64, self.variables)

    self.timeline = Timeline(self.lib, self.spec, self.edits)
    self.selected_frame = ReactiveValue(0)
    self.timeline.add_hotspot(self.selected_frame)

    self.frame_sheets: List[FrameSheet] = []

    self.dbg_reload_graphics = ReactiveValue(())

  def path(self, path: str) -> DataPath:
    return DataPath.parse(self.spec, path)


class Window(QWidget):
  def __init__(self, parent=None):
    super().__init__(parent)

    self.setWindowTitle('Wafel')

    self.model = Model()

    frame_sheet_variables = self.model.variables
    self.model.frame_sheets.append(FrameSheet(self.model.timeline, self.model.edits, self.model.formatters, frame_sheet_variables))

    layout = QHBoxLayout()
    layout.setContentsMargins(0, 0, 0, 0)

    visualizer_layout = QVBoxLayout()
    visualizer_layout.addWidget(GameView(self.model, CameraMode.ROTATE))
    visualizer_layout.addWidget(GameView(self.model, CameraMode.BIRDS_EYE))
    visualizer_layout.addWidget(FrameSlider(self.model))
    layout.addLayout(visualizer_layout)

    variable_layout = QVBoxLayout()
    variable_layout.addWidget(FrameSheetView(self.model, self.model.frame_sheets[0]))
    variable_layout.addWidget(VariableExplorer(self.model))
    layout.addLayout(variable_layout)

    self.setLayout(layout)

    self.balance_timer = QTimer()
    self.balance_timer.setInterval(1000 // 60)
    self.balance_timer.timeout.connect(lambda: self.model.timeline.balance_distribution(1/120))
    self.balance_timer.start()

  def keyPressEvent(self, event):
    if event.key() == Qt.Key_R:
      self.model.dbg_reload_graphics.value = ()
    elif event.key() == Qt.Key_1:
      self.model.selected_frame.value = 21064
    elif event.key() == Qt.Key_2:
      self.model.selected_frame.value = 107775


class FrameSheetView(QTableView):
  def __init__(self, model: Model, frame_sheet: FrameSheet, parent=None):
    super().__init__(parent)
    self.model = model

    self.setModel(frame_sheet)
    self.setVerticalScrollMode(QAbstractItemView.ScrollPerPixel)
    self.setHorizontalScrollMode(QAbstractItemView.ScrollPerPixel)
    self.setSelectionMode(QAbstractItemView.SingleSelection)

    # self.setMinimumWidth(640)
    # self.setMinimumHeight(480)

    def set_selection(frame):
      index = frame_sheet.index(frame, 0)
      self.selectionModel().select(index, QItemSelectionModel.ClearAndSelect)
      self.scrollTo(index)
    set_selection(self.model.selected_frame.value)
    self.model.selected_frame.on_change(set_selection)

    self.focus_frame = ReactiveValue(0)
    def set_focus_frame():
      self.focus_frame.value = self.rowAt(self.contentsRect().y())
    self.verticalScrollBar().valueChanged.connect(set_focus_frame)
    set_focus_frame()

    self.model.timeline.add_hotspot(self.focus_frame)

  def selectionChanged(self, selected, deselected):
    frame = min((index.row() for index in selected.indexes()), default=0)
    self.model.selected_frame.value = frame


class VariableExplorer(QWidget):

  def __init__(self, model: Model, parent=None):
    super().__init__(parent)
    self.model = model
    self.state = self.model.timeline.frame(self.model.selected_frame)

    # self.setMinimumWidth(640)
    # self.setMinimumHeight(480)

    layout = QFormLayout()
    layout.setLabelAlignment(Qt.AlignRight)
    self.setLayout(layout)

    var_widgets = {}

    def show_var(variable: Variable, editor):
      # TODO: Remove str after handling checkboxes
      args = { VariableParam.STATE: self.state.value }
      value = str(self.model.formatters[variable].output(variable.get(args)))
      editor.setText(value)
      editor.setCursorPosition(0)

    def update():
      for variable in self.model.variables:
        editor = var_widgets.get(variable)

        if editor is None:
          editor = QLineEdit()
          editor.setMaximumWidth(80)

          def edit_func(variable: Variable, editor):
            def edit():
              try:
                value = self.model.formatters[variable].input(editor.text())
              except Exception:
                sys.stderr.write(traceback.format_exc())
                sys.stderr.flush()
                show_var(variable, editor)
                return
              self.model.edits.add(self.state.value.frame, VariableEdit(variable, value))
            return edit

          editor.editingFinished.connect(edit_func(variable, editor))

          layout.addRow(QLabel(variable.display_name), editor)
          var_widgets[variable] = editor

        show_var(variable, editor)


    self.state.on_change(update)
    update()


class FrameSlider(QSlider):

  def __init__(self, model: Model, parent=None):
    super().__init__(Qt.Horizontal, parent)
    self.model = model

    self.setMinimum(0)
    self.setMaximum(len(self.model.timeline))
    self.setMaximumWidth(800)

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

  def __init__(self, model: Model, camera_mode: CameraMode, parent=None):
    super().__init__(parent)
    self.model = model

    # self.setMinimumSize(480, 320)
    # self.setMaximumWidth(640)
    self.setMouseTracking(True)

    self.state = self.model.timeline.frame(self.model.selected_frame)

    # TODO: Instead of loading all these states at once, just extract render
    # info from each one-by-one (to allow longer paths without running out of cells)
    def plus(i):
      return lambda f: max(f + i, 0)
    self.path_states = [
      self.model.timeline.frame(self.model.selected_frame.map(plus(i)))
        for i in range(-5, 31)
    ]

    self.mouse_down = ReactiveValue(False)
    self.mouse_pos: ReactiveValue[Optional[Tuple[int, int]]] = ReactiveValue(None)
    self.zoom = ReactiveValue(0.0)
    self.total_drag = ReactiveValue((0, 0))

    def compute_camera(state: GameState, zoom: float, total_drag: Tuple[int, int]) -> Camera:
      if camera_mode == CameraMode.ROTATE:
        camera = RotateCamera(
          pos = [0.0, 0.0, 0.0],
          pitch = -total_drag[1] / 200,
          yaw = -total_drag[0] / 200,
          fov_y = math.radians(45),
        )

        target = self.model.path('$state.gMarioState[].pos').get({
          VariableParam.STATE: state,
        })
        face_dir = camera.face_dir()
        offset_dist = 1500 * math.pow(0.5, zoom)
        camera.pos = [target[i] - offset_dist * face_dir[i] for i in range(3)]

        return camera

      elif camera_mode == CameraMode.BIRDS_EYE:
        target = self.model.path('$state.gMarioState[].pos').get({
          VariableParam.STATE: state,
        })
        return BirdsEyeCamera(
          pos = [target[0], target[1] + 500, target[2]],
          span_y = 200 / math.pow(2, zoom),
        )

      else:
        raise NotImplementedError(camera_mode)
    self.camera = Reactive.tuple(self.state, self.zoom, self.total_drag).mapn(compute_camera)

    self.draw_timer = QTimer()
    self.draw_timer.timeout.connect(self.update)
    self.draw_timer.start()

    def reload():
      self.renderer = None
    self.model.dbg_reload_graphics.on_change(reload)

  def initializeGL(self):
    self.renderer = Renderer()
    # TODO: For some reason the opengl context is destroyed or non-current
    # in destructor call, causing errors

  def paintGL(self):
    self.makeCurrent()

    if self.renderer is None:
      self.renderer = Renderer()

    self.renderer.render(RenderInfo(
      Viewport(0, 0, self.size().width(), self.size().height()),
      self.camera.value,
      self.state.value,
      [st.value for st in self.path_states],
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
  window.move(app.desktop().screen().rect().center() - window.rect().center())
  window.showMaximized()
  app.exec_()
