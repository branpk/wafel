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
from wafel.game_state import GameState
from wafel.data_path import DataPath
from wafel.variable import *
from wafel.variable_format import Formatters, VariableFormatter
from wafel.game_lib import GameLib
from wafel.object_type import ObjectType
from wafel.variable_format import Formatters, CheckboxFormatter


class Model:
  def __init__(self):
    dll = cdll.LoadLibrary('lib/libsm64/jp/sm64')
    with open('lib/libsm64/jp/libsm64.json', 'r') as f:
      spec: dict = json.load(f)
    self.lib = GameLib(spec, dll)

    self.variables = Variable.create_all(self.lib)
    self.formatters = Formatters()

    with open('test_files/1key_j.m64', 'rb') as m64:
      self.edits = Edits.from_m64(m64, self.variables)

    self.timeline = Timeline(self.lib, self.edits)
    self.selected_frame = ReactiveValue(0)
    self.timeline.add_hotspot(self.selected_frame)

    self.frame_sheets: List[FrameSheet] = []

    self.dbg_reload_graphics = ReactiveValue(())

  def path(self, path: str) -> DataPath:
    return DataPath.parse(self.lib, path)


class Window(QWidget):
  def __init__(self, parent=None):
    super().__init__(parent)

    self.setWindowTitle('Wafel')

    self.model = Model()

    self.model.frame_sheets.append(FrameSheet(self.model))

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


class FrameSheetColumn:
  def __init__(
    self,
    variable: Variable,
    object_type: Optional[ObjectType] = None,
  ) -> None:
    self.variable = variable
    # TODO: Semantics object ids should make object_type unnecessary
    self.object_type = object_type


class FrameSheet(QAbstractTableModel):

  def __init__(self, model: Model) -> None:
    super().__init__()
    self.model = model
    self.columns: List[FrameSheetColumn] = []

    self.model.edits.latest_edited_frame.on_change(lambda:
      self.dataChanged.emit(
        self.index(0, 0),
        self.index(self.rowCount() - 1, self.columnCount() - 1)
      )
    )

    self.header_state = self.model.timeline.frame(self.model.selected_frame)
    # self.header_labels: Dict[int, str] = {}
    # self.header_state.on_change(self.refresh_headers)

  def add_variable(self, variable: Variable, state: GameState) -> None:
    index = self.columnCount()
    self.beginInsertColumns(QModelIndex(), index, index)
    self.columns.append(self.column_for_variable(variable, state))
    self.endInsertColumns()

  def column_for_variable(self, variable: Variable, state: GameState) -> FrameSheetColumn:
    object_id = variable.get_object_id()
    if object_id is None:
      return FrameSheetColumn(variable)
    else:
      return FrameSheetColumn(variable, get_object_type(self.model, state, object_id))

  def remove_variable_index(self, index: int) -> None:
    self.beginRemoveColumns(QModelIndex(), index, index)
    del self.columns[index]
    self.endRemoveColumns()

  def rowCount(self, parent=None) -> int:
    return len(self.model.timeline)

  def columnCount(self, parent=None) -> int:
    return len(self.columns)

  def headerData(self, section, orientation, role=Qt.DisplayRole):
    if role == Qt.DisplayRole:
      if orientation == Qt.Horizontal:
        label = self.get_header_label(self.columns[section])
        # self.header_labels[section] = label
        return label
      else:
        return str(section)

  def get_header_label(self, column: FrameSheetColumn) -> str:
    variable = column.variable
    object_id = variable.get_object_id()

    if object_id is None:
      return variable.display_name

    if column.object_type is None:
      return str(object_id) + '\n' + variable.display_name

    return str(object_id) + ' - ' + column.object_type.name + '\n' + variable.display_name

  # def refresh_headers(self) -> None:
  #   for i in range(self.columnCount()):
  #     label = self.get_header_label(self.variables[i])
  #     if label != self.header_labels.get(i):
  #       self.headerDataChanged.emit(Qt.Horizontal, i, i)

  def flags(self, index):
    column = self.columns[index.column()]
    variable = column.variable

    # object_id = variable.get_object_id()
    # if column.object_type is not None and object_id is not None:
    #   if self.visible_rows is not None and index.row() not in self.visible_rows.value:
    #     return Qt.ItemFlags()

    #   row_object_type = self.cached_object_types.get((index.row(), object_id))

    #   if row_object_type is None:
    #     def get_type(object_id: ObjectId) -> Callable[[GameState], Optional[ObjectType]]:
    #       return lambda state: get_object_type(self.model, state, object_id)
    #     row_object_type = self.model.timeline.frame(index.row()).map(get_type(object_id)).cached()
    #     self.cached_object_types[(index.row(), object_id)] = row_object_type

    #   if row_object_type.value != column.object_type:
    #     return Qt.ItemFlags()

    if isinstance(self.model.formatters[variable], CheckboxFormatter):
      return Qt.ItemIsSelectable | Qt.ItemIsUserCheckable | Qt.ItemIsEnabled
    else:
      return Qt.ItemIsSelectable | Qt.ItemIsEditable | Qt.ItemIsEnabled

  def data(self, index, role=Qt.DisplayRole):
    column = self.columns[index.column()]
    variable = column.variable
    state = self.model.timeline.frame(index.row()).value

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      row_object_type = get_object_type(self.model, state, object_id)

      if row_object_type != column.object_type:
        return QVariant()

    args = { VariableParam.STATE: state }
    formatter = self.model.formatters[variable]
    value = formatter.output(variable.get(args))

    if role == Qt.DisplayRole or role == Qt.EditRole:
      if not isinstance(formatter, CheckboxFormatter):
        return value

    elif role == Qt.CheckStateRole:
      if isinstance(formatter, CheckboxFormatter):
        return Qt.Checked if value else Qt.Unchecked

  def setData(self, index, value, role=Qt.EditRole):
    column = self.columns[index.column()]
    variable = column.variable

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      state = self.model.timeline.frame(index.row()).value
      row_object_type = get_object_type(self.model, state, object_id)
      if row_object_type != column.object_type:
        return False
      del state

    formatter = self.model.formatters[variable]
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
      self.model.edits.add(index.row(), VariableEdit(variable, value))
      self.dataChanged.emit(index, index)
      return True

    return False


class FrameSheetView(QTableView):
  def __init__(self, model: Model, frame_sheet: FrameSheet, parent=None):
    super().__init__(parent)
    self.model = model
    self.frame_sheet = frame_sheet

    self.setModel(frame_sheet)
    self.setVerticalScrollMode(QAbstractItemView.ScrollPerPixel)
    self.setHorizontalScrollMode(QAbstractItemView.ScrollPerPixel)
    self.setSelectionMode(QAbstractItemView.SingleSelection)

    self.horizontalHeader().setSectionsMovable(True)
    self.horizontalHeader().setSectionsClickable(True)
    self.horizontalHeader().sectionDoubleClicked.connect(self.frame_sheet.remove_variable_index)

    # self.setMinimumWidth(640)
    # self.setMinimumHeight(480)

    def set_selection(frame):
      index = frame_sheet.index(frame, 0)
      self.selectionModel().select(index, QItemSelectionModel.ClearAndSelect)
      self.scrollTo(index)
    set_selection(self.model.selected_frame.value)
    self.model.selected_frame.on_change(set_selection)

    # TODO: Below code doesn't quite work because of initial sizing
    # self.visible_rows = ReactiveValue(range(0))
    # def set_visible_rows():
    #   low = self.rowAt(self.contentsRect().top())
    #   high = self.rowAt(self.contentsRect().bottom())
    #   if high < 0:
    #     high = self.frame_sheet.rowCount() - 1
    #   self.visible_rows.value = range(low, high + 1)
    # self.verticalScrollBar().valueChanged.connect(set_visible_rows)
    # set_visible_rows()
    # self.frame_sheet.visible_rows = self.visible_rows

    self.focus_frame = ReactiveValue(0)
    def set_focus_frame():
      self.focus_frame.value = self.rowAt(self.contentsRect().y())
    self.verticalScrollBar().valueChanged.connect(set_focus_frame)
    set_focus_frame()

    self.model.timeline.add_hotspot(self.focus_frame)

  def selectionChanged(self, selected, deselected):
    frame = min((index.row() for index in selected.indexes()), default=0)
    self.model.selected_frame.value = frame

  def add_variable(self, variable: Variable, state: GameState) -> None:
    self.frame_sheet.add_variable(variable, state)


class VerticalTabBar(QTabBar):

  def __init__(self, parent=None):
    super().__init__(parent)

  def tabSizeHint(self, index):
    size = super().tabSizeHint(index)
    size.transpose()
    return size

  # def wheelEvent(self, event):
  #   pass

  def paintEvent(self, event):
    painter = QStylePainter(self)
    option_tab = QStyleOptionTab()

    for i in range(self.count()):
      self.initStyleOption(option_tab, i)
      painter.drawControl(QStyle.CE_TabBarTabShape, option_tab)
      painter.save()

      size = option_tab.rect.size()
      size.transpose()
      rect = QRect(QPoint(), size)
      rect.moveCenter(option_tab.rect.center())
      option_tab.rect = rect

      center = self.tabRect(i).center()
      painter.translate(center)
      painter.rotate(90)
      painter.translate(-center)
      painter.drawControl(QStyle.CE_TabBarTabLabel, option_tab)
      painter.restore()


class ExplorerTabKey:

  def __init__(self, name: str, object_id: Optional[ObjectId] = None) -> None:
    self.name = name
    self.object_id = object_id

  def __eq__(self, other: object) -> bool:
    if not isinstance(other, ExplorerTabKey):
      return False
    return self.name == other.name and self.object_id == other.object_id

  def __hash__(self) -> int:
    return hash((self.name, self.object_id))


def get_object_type(model: Model, state: GameState, object_id: ObjectId) -> Optional[ObjectType]:
  active = model.variables['active'].at_object(object_id).get({
    VariableParam.STATE: state,
  })
  if not active:
    return None

  behavior = model.variables['behavior'].at_object(object_id).get({
    VariableParam.STATE: state,
  })
  return model.lib.get_object_type(behavior)


class VariableExplorer(QTabWidget):

  def __init__(self, model: Model, parent=None):
    super().__init__(parent)
    self.model = model
    self.state = self.model.timeline.frame(self.model.selected_frame)

    tab_bar = VerticalTabBar(self)
    self.setTabBar(tab_bar)
    self.setTabPosition(QTabWidget.West)

    self.setMaximumHeight(300)

    self.open_tabs: List[ExplorerTabKey] = []

    fixed_tabs = [
      ExplorerTabKey('Input'),
      ExplorerTabKey('Misc'),
      ExplorerTabKey('Objects'),
    ]
    for tab in fixed_tabs:
      self.open_tab(tab)

    self.state.on_change(self.update_tabs)

    self.setCurrentIndex(0)

  def open_tab(self, tab: ExplorerTabKey) -> None:
    if tab in self.open_tabs:
      index = self.open_tabs.index(tab)
    else:
      index = len(self.open_tabs)
      self.addTab(self.create_tab_widget(tab), self.get_tab_name(tab))
      self.open_tabs.append(tab)
    self.setCurrentIndex(index)

  def close_tab(self, tab: ExplorerTabKey) -> None:
    if tab in self.open_tabs:
      index = self.open_tabs.index(tab)
      self.removeTab(index)
      del self.open_tabs[index]

  def update_tabs(self) -> None:
    for index, tab in enumerate(self.open_tabs):
      self.setTabText(index, self.get_tab_name(tab))

  def get_tab_name(self, tab: ExplorerTabKey) -> str:
    if tab.object_id is not None:
      object_type = get_object_type(self.model, self.state.value, tab.object_id)
      if object_type is None:
        return str(tab.object_id)
      else:
        return str(tab.object_id) + ': ' + object_type.name

    return tab.name

  def create_tab_widget(self, tab: ExplorerTabKey) -> QWidget:
    if tab.name == 'Objects':
      def open_object_tab(object_id: ObjectId) -> None:
        self.open_tab(ExplorerTabKey('_object', object_id))
      return ObjectsTab(self.model, open_object_tab, self)

    if tab.object_id is None:
      close_tab = None
    else:
      close_tab = lambda: self.close_tab(tab)

    return VariableTab(self.model, tab, close_tab, self)


class VariableTab(QWidget):

  def __init__(
    self,
    model: Model,
    tab: ExplorerTabKey,
    close_tab: Optional[Callable[[], None]],
    parent=None
  ) -> None:
    super().__init__(parent)
    self.model = model
    self.state = self.model.timeline.frame(self.model.selected_frame)
    self.tab = tab

    # self.setMinimumWidth(640)
    # self.setMinimumHeight(480)

    self.variables = self.state.map(self.get_variables)

    layout = FlowLayout(10, 5, Qt.Vertical, self)
    self.setLayout(layout)

    # TODO: Change to id/something
    # var_editors: Dict[str, QLineEdit] = {}

    def show_var(variable: Variable, editor):
      # TODO: Remove str after handling checkboxes
      args = { VariableParam.STATE: self.state.value }
      value = str(self.model.formatters[variable].output(variable.get(args)))
      editor.setText(value)
      editor.setCursorPosition(0)

    def update():
      # TODO: Only recreate on behavior change
      while layout.count() > 0:
        item = layout.takeAt(0)
        item.widget().setParent(None)

      if close_tab is not None:
        close_button = QPushButton('Close tab')
        close_button.setMaximumWidth(100)
        close_button.clicked.connect(close_tab)
        layout.addWidget(close_button)

      for variable in self.variables.value:
        editor = None#var_widgets.get(variable.display_name)

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

        label = QLabel(variable.display_name)
        label.setFixedWidth(80)
        # label.setFixedHeight(10)
        label.setAlignment(Qt.AlignRight)
        def add_var_to_frame_sheet(var: Variable) -> Callable[[Any], None]:
          return lambda _: self.model.frame_sheets[0].add_variable(
            var,
            self.state.value,
          )
        label.mouseDoubleClickEvent = add_var_to_frame_sheet(variable)

        var_layout = QHBoxLayout()
        var_layout.setContentsMargins(0, 0, 0, 0)
        var_layout.addWidget(label)
        var_layout.addWidget(editor)

        var_widget = QWidget()
        var_widget.setLayout(var_layout)

        layout.addWidget(var_widget)
        # var_widgets[variable.display_name] = editor

        show_var(variable, editor)

    self.state.on_change(update)
    update()

  def get_variables(self, state: GameState) -> List[Variable]:
    if self.tab.object_id is None:
      return self.model.variables.group(VariableGroup(self.tab.name))

    object_type = get_object_type(self.model, state, self.tab.object_id)
    if object_type is None:
      return []

    return [
      var.at_object(self.tab.object_id)
        for var in self.model.variables.group(VariableGroup.object(object_type.name))
    ]


# class VarLabel(QLabel):
#   def __init__(self, variable: Variable, parent=None):
#     super().__init__(parent)
#     self.variable = variable

#   def 


class FlowLayout(QLayout):
  """From https://doc.qt.io/qt-5/qtwidgets-layouts-flowlayout-example.html"""

  def __init__(self, margin, spacing, orientation, parent=None):
    super().__init__(parent)
    self.spacing = spacing
    self.items = []
    self.orientation = orientation

    self.setContentsMargins(margin, margin, margin, margin)

  def addItem(self, item):
    self.items.append(item)

  def count(self):
    return len(self.items)

  def itemAt(self, index):
    if index not in range(len(self.items)):
      return None
    return self.items[index]

  def takeAt(self, index):
    if index not in range(len(self.items)):
      return None
    item = self.items[index]
    del self.items[index]
    return item

  def expandingDirections(self):
    return {
      Qt.Vertical: Qt.Horizontal,
      Q.Horizontal: Qt.Vertical,
    }[self.orientation]

  def hasHeightForWidth(self):
    return self.orientation == Qt.Horizontal

  def heightForWidth(self, width):
    height = self.do_layout(QRect(0, 0, width, 0), True)
    return height

  def setGeometry(self, rect):
    super().setGeometry(rect)
    self.do_layout(rect, False)

  def sizeHint(self):
    return self.minimumSize()

  def minimumSize(self):
    size = QSize(0, 0)
    for item in self.items:
      size = size.expandedTo(item.minimumSize())

    margins = self.contentsMargins()
    size += QSize(margins.left() + margins.right(), margins.top() + margins.bottom())
    return size

  def do_layout(self, rect, test_only):
    margins = self.contentsMargins()
    effective_rect = rect.adjusted(margins.left(), margins.top(), -margins.right(), -margins.bottom())

    x = effective_rect.x()
    y = effective_rect.y()
    line_size = 0

    if self.orientation == Qt.Horizontal:
      for item in self.items:
        next_x = x + item.sizeHint().width() + self.spacing
        if next_x - self.spacing > effective_rect.right() and line_size > 0:
          x = effective_rect.x()
          y = y + line_size + self.spacing
          next_x = x + item.sizeHint().width() + self.spacing
          line_size = 0

        if not test_only:
          item.setGeometry(QRect(QPoint(x, y), item.sizeHint()))

        x = next_x
        line_size = max(line_size, item.sizeHint().height())

      return y + line_size - rect.y() + margins.bottom()

    else:
      for item in self.items:
        next_y = y + item.sizeHint().height() + self.spacing
        if next_y - self.spacing > effective_rect.bottom() and line_size > 0:
          y = effective_rect.y()
          x = x + line_size + self.spacing
          next_y = y + item.sizeHint().height() + self.spacing
          line_size = 0

        if not test_only:
          item.setGeometry(QRect(QPoint(x, y), item.sizeHint()))

        y = next_y
        line_size = max(line_size, item.sizeHint().width())

      return x + line_size - rect.x() + margins.right()


class ObjectsTab(QScrollArea):

  def __init__(self, model: Model, open_object_tab: Callable[[ObjectId], None], parent=None):
    super().__init__(parent)
    self.model = model
    self.state = self.model.timeline.frame(self.model.selected_frame)

    self.objects_layout = FlowLayout(10, 5, Qt.Horizontal, self)

    for i in range(240):
      button = QPushButton(str(i + 1), self)
      button.setFixedSize(50, 50)
      button.clicked.connect((lambda id: lambda: open_object_tab(id))(i))
      self.objects_layout.addWidget(button)

    objects_widget = QWidget()
    objects_widget.setLayout(self.objects_layout)

    self.setWidget(objects_widget)
    self.setWidgetResizable(True)

    variables = [
      {
        'active': self.model.variables['active'].at_object(i),
        'behavior': self.model.variables['behavior'].at_object(i),
      }
      for i in range(240)
    ]

    def update():
      args = {
        VariableParam.STATE: self.state.value,
      }

      for i in range(240):
        active = variables[i]['active'].get(args)
        if active:
          behavior = variables[i]['behavior'].get(args)
          label = self.model.lib.get_object_type(behavior).name
        else:
          label = None

        text = str(i) + '\n' + (label or '')

        item = self.objects_layout.itemAt(i)
        item.widget().setText(text)

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
