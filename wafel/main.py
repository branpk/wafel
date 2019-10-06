from typing import *
from ctypes import *
import json
import math
import sys
import traceback

import glfw
import imgui as ig
from imgui.integrations.glfw import GlfwRenderer
from OpenGL import GL as gl

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
    self.selected_frame = 0
    # self.timeline.add_hotspot(self.selected_frame)

    self.frame_sheets: List[FrameSheet] = [FrameSheet(self)]
    self.variable_explorer = VariableExplorer(self)
    self.game_views: List[GameView] = [
      GameView(self, CameraMode.ROTATE),
      GameView(self, CameraMode.BIRDS_EYE),
    ]

    # self.dbg_reload_graphics = ReactiveValue(())

  def get_object_type(self, state: GameState, object_id: ObjectId) -> Optional[ObjectType]:
    active = self.variables['obj-active-flags-active'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    if not active:
      return None

    behavior = self.variables['obj-behavior-ptr'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    return self.lib.get_object_type(behavior)

  def path(self, path: str) -> DataPath:
    return DataPath.parse(self.lib, path)

  def render(self, window_size: Tuple[int, int]) -> None:
    ig.set_next_window_position(0, 0)
    ig.set_next_window_size(*window_size)
    ig.begin(
      'Main',
      False,
      ig.WINDOW_NO_SAVED_SETTINGS | ig.WINDOW_NO_RESIZE | ig.WINDOW_NO_TITLE_BAR,
    )

    ig.columns(2)

    ig.begin_child('Game View 1', height=int(window_size[1] // 2), border=True)
    self.game_views[0].render(window_size)
    ig.end_child()

    ig.begin_child('Game View 2', border=True)
    self.game_views[1].render(window_size)
    ig.end_child()

    ig.next_column()

    frame_sheet = self.frame_sheets[0]
    ig.set_next_window_content_size(frame_sheet.get_content_width(), 0)
    ig.begin_child(
      'Frame Sheet',
      height=int(window_size[1] * 0.7),
      flags=ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
    )
    frame_sheet.render()
    ig.end_child()

    if ig.begin_drag_drop_target():
      payload = ig.accept_drag_drop_payload('ve-var')
      if payload is not None:
        variable = self.variables[VariableId.from_bytes(payload)]
        frame_sheet.append_variable(variable)
      ig.end_drag_drop_target()

    ig.begin_child('Variable Explorer', border=True)
    self.variable_explorer.render()
    ig.end_child()

    ig.columns(1)

    ig.end()


class FrameSheetColumn:
  def __init__(
    self,
    variable: Variable,
    object_type: Optional[ObjectType] = None,
  ) -> None:
    self.variable = variable
    # TODO: Semantics object ids should make object_type unnecessary
    self.object_type = object_type
    self.width = 100

  def __eq__(self, other) -> bool:
    return isinstance(other, FrameSheetColumn) and \
      self.variable == other.variable and \
      self.object_type == other.object_type


class CellEditInfo:
  def __init__(
    self, row: int, column: FrameSheetColumn) -> None:
    self.row = row
    self.column = column
    self.initial_focus = False
    self.error = False


class FrameSheet:
  def __init__(self, model: Model) -> None:
    super().__init__()
    self.model = model
    self.columns: List[FrameSheetColumn] = []
    self.row_height = 30
    self.frame_column_width = 60
    self.cell_edit: Optional[CellEditInfo] = None

  def insert_variable(self, index: int, variable: Variable) -> None:
    object_id = variable.get_object_id()
    if object_id is None:
      column = FrameSheetColumn(variable)
    else:
      # TODO: This should use the state that the drop began
      state = self.model.timeline.frame(self.model.selected_frame).value
      column = FrameSheetColumn(variable, self.model.get_object_type(state, object_id))
    if column not in self.columns:
      self.columns.insert(index, column)

  def append_variable(self, variable: Variable) -> None:
    self.insert_variable(len(self.columns), variable)

  def move_column(self, source: int, dest: int) -> None:
    column = self.columns[source]
    del self.columns[source]
    self.columns.insert(dest, column)

  def get_row_count(self) -> int:
    return len(self.model.timeline)

  def get_header_label(self, column: FrameSheetColumn) -> str:
    variable = column.variable
    object_id = variable.get_object_id()

    if object_id is None:
      return variable.display_name

    if column.object_type is None:
      return str(object_id) + '\n' + variable.display_name

    return str(object_id) + ' - ' + column.object_type.name + '\n' + variable.display_name

  def get_data(self, row: int, column: FrameSheetColumn) -> Any:
    variable = column.variable
    state = self.model.timeline.frame(row).value

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      row_object_type = self.model.get_object_type(state, object_id)
      if row_object_type != column.object_type:
        return ''

    args = { VariableParam.STATE: state }
    formatter = self.model.formatters[variable]
    return formatter.output(variable.get(args))

  def set_data(self, row: int, column: FrameSheetColumn, value: Any) -> bool:
    variable = column.variable

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      state = self.model.timeline.frame(row).value
      row_object_type = self.model.get_object_type(state, object_id)
      if row_object_type != column.object_type:
        return False
      del state

    formatter = self.model.formatters[variable]
    assert isinstance(value, str)

    try:
      value = formatter.input(value)
    except:
      sys.stderr.write(traceback.format_exc())
      sys.stderr.flush()
      return False

    self.model.edits.add(row, VariableEdit(variable, value))
    return True

  def get_content_width(self) -> int:
    if len(self.columns) == 0:
      return 0
    return self.frame_column_width + sum(column.width for column in self.columns)

  def render_headers(self) -> None:
    header_labels = [self.get_header_label(column) for column in self.columns]
    header_lines = max((len(label.split('\n')) for label in header_labels), default=1)

    ig.columns(len(self.columns) + 1)
    ig.set_column_width(-1, self.frame_column_width)
    ig.next_column()

    for index, column in list(enumerate(self.columns)):
      initial_cursor_pos = ig.get_cursor_pos()
      ig.selectable(
        '##fs-col-' + str(id(column)),
        height = header_lines * ig.get_text_line_height(),
      )

      # TODO: Width adjusting
      ig.set_column_width(-1, column.width)

      if ig.begin_drag_drop_source():
        ig.text(header_labels[index])
        ig.set_drag_drop_payload('fs-col', str(index).encode('utf-8'))
        ig.end_drag_drop_source()

      if ig.begin_drag_drop_target():
        payload = ig.accept_drag_drop_payload('fs-col')
        if payload is not None:
          source = int(payload.decode('utf-8'))
          self.move_column(source, index)

        payload = ig.accept_drag_drop_payload('ve-var')
        if payload is not None:
          variable = self.model.variables[VariableId.from_bytes(payload)]
          self.insert_variable(index, variable)

        ig.end_drag_drop_target()

      ig.set_cursor_pos(initial_cursor_pos)
      ig.text(header_labels[index])

      ig.next_column()
    ig.separator()
    ig.columns(1)

  def render_cell(self, row: int, column: FrameSheetColumn) -> None:
    data = self.get_data(row, column)
    assert isinstance(data, str)

    cursor_pos = ig.get_cursor_pos()
    cursor_pos = (
      ig.get_window_position()[0] + cursor_pos[0],
      ig.get_window_position()[1] + cursor_pos[1] - ig.get_scroll_y(),
    )

    if self.cell_edit is not None and \
        self.cell_edit.row == row and \
        self.cell_edit.column is column:
      input_width = column.width - 2 * ig.get_style().item_spacing[0]

      ig.push_item_width(input_width)
      _, value = ig.input_text(
        '##fs-cell-' + str(row) + '-' + str(id(column)),
        data,
        32,
      )
      ig.pop_item_width()

      if value != data:
        self.cell_edit.error = not self.set_data(row, column, value)

      if self.cell_edit.error:
        dl = ig.get_window_draw_list()
        dl.add_rect(
          cursor_pos[0],
          cursor_pos[1],
          cursor_pos[0] + input_width,
          cursor_pos[1] + ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1],
          0xFF0000FF,
        )
        # TODO: Show error message?

      if not self.cell_edit.initial_focus:
        ig.set_keyboard_focus_here(-1)
        self.cell_edit.initial_focus = True
      elif not ig.is_item_active():
        self.cell_edit = None

    else:
      clicked, selected = ig.selectable(
        data + '##fs-cell-' + str(row) + '-' + str(id(column)),
        height=self.row_height - 8, # TODO: Compute padding
        flags=ig.SELECTABLE_ALLOW_DOUBLE_CLICK,
      )

      if clicked or selected:
        if ig.is_mouse_double_clicked():
          self.cell_edit = CellEditInfo(row, column)
        else:
          self.model.selected_frame = row

  def render_rows(self) -> None:
    ig.columns(len(self.columns) + 1)

    min_row = int(ig.get_scroll_y()) // self.row_height
    min_row = max(min_row, 0)
    max_row = int(ig.get_scroll_y() + ig.get_window_height()) // self.row_height
    max_row = min(max_row, self.get_row_count() - 1)

    for row in range(min_row, max_row + 1):
      ig.set_column_width(-1, self.frame_column_width)
      ig.text(str(row))
      ig.next_column()

      for column in self.columns:
        if row == min_row:
          initial_pos = ig.get_cursor_pos()
          ig.set_cursor_pos((initial_pos[0], row * self.row_height))

        self.render_cell(row, column)

        ig.set_column_width(-1, column.width)
        ig.next_column()
      ig.separator()

    ig.set_cursor_pos((0, self.get_row_count() * self.row_height))

    ig.columns(1)

  def render(self) -> None:
    if len(self.columns) == 0:
      ig.begin_child('Empty Frame Sheet')
      ig.text('Drag a variable from below to watch it')
      ig.end_child()
    else:
      self.render_headers()
      # TODO: Make the vertical scrollbar always visible?
      ig.begin_child('Frame Sheet Rows', flags=ig.WINDOW_ALWAYS_VERTICAL_SCROLLBAR)
      self.render_rows()
      ig.end_child()


class ExplorerTabId:
  def __init__(self, name: str, object_id: Optional[ObjectId] = None) -> None:
    self.name = name
    self.object_id = object_id

  def __eq__(self, other: object) -> bool:
    if not isinstance(other, ExplorerTabId):
      return False
    return self.name == other.name and self.object_id == other.object_id

  def __hash__(self) -> int:
    return hash((self.name, self.object_id))


class VariableExplorer:
  def __init__(self, model: Model) -> None:
    self.model = model
    self.open_tabs: List[ExplorerTabId] = []
    self.rendered = False

    fixed_tabs = [
      ExplorerTabId('Input'),
      ExplorerTabId('Misc'),
      ExplorerTabId('Objects'),
    ]
    for tab in fixed_tabs:
      self.open_tab(tab)

    self.current_tab = self.open_tabs[0]

  def open_tab(self, tab: ExplorerTabId) -> None:
    if tab not in self.open_tabs:
      self.open_tabs.append(tab)
    self.current_tab = tab

  def close_tab(self, tab: ExplorerTabId) -> None:
    if self.current_tab == tab:
      # TODO
      pass
    if tab in self.open_tabs:
      self.open_tabs.remove(tab)

  def get_tab_label(self, tab: ExplorerTabId) -> str:
    if tab.object_id is not None:
      state = self.model.timeline.frame(self.model.selected_frame).value
      object_type = self.model.get_object_type(state, tab.object_id)
      if object_type is None:
        return str(tab.object_id)
      else:
        return str(tab.object_id) + ': ' + object_type.name

    return tab.name

  def render_objects_tab(self) -> None:
    button_size = 50
    window_left = ig.get_window_position()[0]
    window_right = window_left + ig.get_window_content_region_max()[0]
    prev_item_right = window_left
    style = ig.get_style()

    for slot in range(240):
      item_right = prev_item_right + style.item_spacing[0] + button_size
      if item_right > window_right:
        prev_item_right = window_left
      elif slot != 0:
        ig.same_line()
      prev_item_right = prev_item_right + style.item_spacing[0] + button_size

      object_id = slot
      object_type = self.model.get_object_type(
        self.model.timeline.frame(self.model.selected_frame).value,
        object_id,
      )
      if object_type is None:
        label = str(slot)
      else:
        label = str(slot) + '\n' + object_type.name

      if ig.button(label + '##slot-' + str(slot), 50, 50):
        self.open_tab(ExplorerTabId('_object', object_id))

  def get_variables_for_tab(self, tab: ExplorerTabId) -> List[Variable]:
    if tab.object_id is None:
      return self.model.variables.group(VariableGroup(tab.name))

    state = self.model.timeline.frame(self.model.selected_frame).value
    object_type = self.model.get_object_type(state, tab.object_id)
    if object_type is None:
      return []

    return [
      var.at_object(tab.object_id)
        for var in self.model.variables.group(VariableGroup.object(object_type.name))
    ]

  def render_variable_tab(self, tab: ExplorerTabId) -> None:
    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      # TODO: Variable id
      ig.selectable(variable.display_name, width=80)

      if ig.begin_drag_drop_source():
        ig.text(variable.display_name)
        ig.set_drag_drop_payload('ve-var', variable.id.to_bytes())
        ig.end_drag_drop_source()

      # TODO: Reuse display/edit code with frame sheet
      ig.same_line()
      ig.push_item_width(80)
      ig.input_text('##' + variable.display_name, 'hey', 32, False)
      ig.pop_item_width()

  def render_tab_contents(self, tab: ExplorerTabId) -> None:
    if tab.name == 'Objects':
      self.render_objects_tab()
    else:
      self.render_variable_tab(tab)

  def render(self) -> None:
    ig.columns(2)
    if not self.rendered:
      self.rendered = True
      ig.set_column_width(-1, 120)

    ig.begin_child('Variable Explorer Tabs')
    for tab in self.open_tabs:
      _, selected = ig.selectable(
        self.get_tab_label(tab) + '##' + str(id(tab)),
        self.current_tab == tab,
      )
      if selected:
        self.current_tab = tab
    ig.end_child()

    ig.next_column()

    ig.begin_child('Variable Explorer Content')
    self.render_tab_contents(self.current_tab)
    ig.end_child()

    ig.columns(1)


class MouseTracker:
  def __init__(self) -> None:
    self.dragging = False
    self.mouse_down = False
    self.mouse_pos = (0.0, 0.0)

  def is_mouse_in_window(self) -> bool:
    window_x, window_y = ig.get_window_position()
    window_w, window_h = ig.get_window_size()
    return self.mouse_pos[0] >= window_x and self.mouse_pos[0] < window_x + window_w and \
        self.mouse_pos[1] >= window_y and self.mouse_pos[1] < window_y + window_h

  def get_drag_amount(self) -> Tuple[float, float]:
    mouse_was_down = self.mouse_down
    last_mouse_pos = self.mouse_pos
    self.mouse_down = ig.is_mouse_down()
    self.mouse_pos = ig.get_mouse_pos()

    if self.dragging:
      if not self.mouse_down:
        self.dragging = False
      return (
        self.mouse_pos[0] - last_mouse_pos[0],
        self.mouse_pos[1] - last_mouse_pos[1],
      )

    elif not mouse_was_down and self.mouse_down:
      window_x, window_y = ig.get_window_position()
      window_w, window_h = ig.get_window_size()
      if self.mouse_pos[0] >= window_x and self.mouse_pos[0] < window_x + window_w and \
          self.mouse_pos[1] >= window_y and self.mouse_pos[1] < window_y + window_h:
        self.dragging = True

    return (0, 0)

  def get_wheel_amount(self) -> float:
    if self.is_mouse_in_window():
      return ig.get_io().mouse_wheel
    else:
      return 0


class GameView:
  def __init__(self, model: Model, camera_mode: CameraMode) -> None:
    self.model = model
    self.camera_mode = camera_mode

    self.renderer = Renderer()
    self.mouse_tracker = MouseTracker()

    self.total_drag = [0.0, 0.0]
    self.zoom = 0

  def compute_camera(self) -> Camera:
    args = { VariableParam.STATE: self.model.timeline.frame(self.model.selected_frame).value }
    mario_pos = [
      self.model.variables['mario-pos-x'].get(args),
      self.model.variables['mario-pos-y'].get(args),
      self.model.variables['mario-pos-z'].get(args),
    ]

    if self.camera_mode == CameraMode.ROTATE:
      target = mario_pos
      offset_dist = 1500 * math.pow(0.5, self.zoom)
      camera = RotateCamera(
        pos = [0.0, 0.0, 0.0],
        pitch = -self.total_drag[1] / 200,
        yaw = -self.total_drag[0] / 200,
        fov_y = math.radians(45)
      )
      face_dir = camera.face_dir()
      camera.pos = [target[i] - offset_dist * face_dir[i] for i in range(3)]
      return camera

    elif self.camera_mode == CameraMode.BIRDS_EYE:
      target = mario_pos
      return BirdsEyeCamera(
        pos = [target[0], target[1] + 500, target[2]],
        span_y = 200 / math.pow(2, self.zoom),
      )

    else:
      raise NotImplementedError(self.camera_mode)

  def render(self, window_size: Tuple[int, int]) -> None:
    viewport_x, viewport_y = ig.get_window_position()
    viewport_w, viewport_h = ig.get_window_size()
    viewport_y = window_size[1] - viewport_y - viewport_h

    drag_amount = self.mouse_tracker.get_drag_amount()
    self.total_drag = (
      self.total_drag[0] + drag_amount[0],
      self.total_drag[1] + drag_amount[1],
    )
    self.zoom += self.mouse_tracker.get_wheel_amount() / 5

    self.renderer.render(RenderInfo(
      Viewport(viewport_x, viewport_y, viewport_w, viewport_h),
      self.compute_camera(),
      self.model.timeline.frame(self.model.selected_frame).value,
      [
        self.model.timeline.frame(self.model.selected_frame + i).value
          for i in range(-5, 31)
            if self.model.selected_frame + i in range(len(self.model.timeline))
      ],
    ))


def render(window, ig_renderer, model: Model) -> None:
  style = ig.get_style()
  style.window_rounding = 0

  window_size = glfw.get_window_size(window)

  gl.glScissor(0, 0, *window_size)
  gl.glClearColor(0.06, 0.06, 0.06, 1.0)
  gl.glClear(gl.GL_COLOR_BUFFER_BIT)

  ig.get_style().colors[ig.COLOR_WINDOW_BACKGROUND] = (0, 0, 0, 0)
  ig.new_frame()
  model.render(window_size)
  ig.end_frame()
  ig.render()

  draw_data = ig.get_draw_data()
  ig_renderer.render(draw_data)

  glfw.swap_buffers(window)


def run() -> None:
  glfw.init()

  glfw.window_hint(glfw.CONTEXT_VERSION_MAJOR, 3)
  glfw.window_hint(glfw.CONTEXT_VERSION_MINOR, 3)
  glfw.window_hint(glfw.OPENGL_PROFILE, glfw.OPENGL_COMPAT_PROFILE) # TODO: Core
  glfw.window_hint(glfw.OPENGL_FORWARD_COMPAT, True)
  glfw.window_hint(glfw.SAMPLES, 4)

  glfw.window_hint(glfw.VISIBLE, False)
  window = glfw.create_window(800, 600, 'Wafel', None, None)
  glfw.set_window_size_limits(window, 1, 1, glfw.DONT_CARE, glfw.DONT_CARE)
  glfw.maximize_window(window)
  glfw.show_window(window)

  glfw.make_context_current(window)

  ig_context = ig.create_context()
  ig_renderer = GlfwRenderer(window)
  ig_renderer.io.ini_filename = None

  def refresh_callback(window):
    render(window, ig_renderer, model)
  glfw.set_window_refresh_callback(window, refresh_callback)

  model = Model()

  while not glfw.window_should_close(window):
    glfw.poll_events()
    ig_renderer.process_inputs()
    render(window, ig_renderer, model)

  ig_renderer.shutdown()
  ig.destroy_context(ig_context)

  glfw.destroy_window(window)
  glfw.terminate()
