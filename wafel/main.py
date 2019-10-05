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

    # self.dbg_reload_graphics = ReactiveValue(())

  def get_object_type(self, state: GameState, object_id: ObjectId) -> Optional[ObjectType]:
    active = self.variables['active'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    if not active:
      return None

    behavior = self.variables['behavior'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    return self.lib.get_object_type(behavior)

  def path(self, path: str) -> DataPath:
    return DataPath.parse(self.lib, path)


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
    self.cell_edit: Optional[CellEditInfo] = None

    for _ in range(1):
      self.append_variable(self.model.variables['global timer'], self.model.timeline.frame(100).value)
      self.append_variable(self.model.variables['mario x'], self.model.timeline.frame(100).value)
      self.append_variable(self.model.variables['mario y'], self.model.timeline.frame(100).value)

  def append_variable(self, variable: Variable, state: GameState) -> None:
    object_id = variable.get_object_id()
    if object_id is None:
      column = FrameSheetColumn(variable)
    else:
      column = FrameSheetColumn(variable, self.model.get_object_type(state, object_id))
    self.columns.append(column)

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

      # TODO
      # if row_object_type != column.object_type:
      #   return ''

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
    return sum(column.width for column in self.columns)

  def render_headers(self) -> None:
    header_labels = [self.get_header_label(column) for column in self.columns]
    header_lines = max((len(label.split('\n')) for label in header_labels), default=1)

    ig.columns(len(self.columns))

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
      cursor_pos[0] - 8, # TODO: Compute padding
      cursor_pos[1] - 4,
    )
    cursor_pos = (
      ig.get_window_position()[0] + cursor_pos[0],
      ig.get_window_position()[1] + cursor_pos[1] - ig.get_scroll_y(),
    )

    if self.cell_edit is not None and \
        self.cell_edit.row == row and \
        self.cell_edit.column is column:
      _, value = ig.input_text(
        '##fs-cell-' + str(row) + '-' + str(id(column)),
        data,
        32,
      )

      if value != data:
        self.cell_edit.error = not self.set_data(row, column, value)

      if self.cell_edit.error:
        dl = ig.get_window_draw_list()
        dl.add_rect(
          cursor_pos[0],
          cursor_pos[1],
          cursor_pos[0] + column.width - 1,
          cursor_pos[1] + self.row_height,
          0xFF0000FF,
        )
        # TODO: Show error message?

      if not self.cell_edit.initial_focus:
        ig.set_keyboard_focus_here(-1)
        self.cell_edit.initial_focus = True
      elif not ig.is_item_active():
        self.cell_edit = None

    else:
      clicked, _ = ig.selectable(
        data,
        height=self.row_height - 8, # TODO: Compute padding
        flags=ig.SELECTABLE_ALLOW_DOUBLE_CLICK,
      )

      if clicked:
        if ig.is_mouse_double_clicked():
          self.cell_edit = CellEditInfo(row, column)

  def render_rows(self) -> None:
    ig.columns(len(self.columns))

    min_row = int(ig.get_scroll_y()) // self.row_height
    min_row = max(min_row, 0)
    max_row = int(ig.get_scroll_y() + ig.get_window_height()) // self.row_height
    max_row = min(max_row, self.get_row_count() - 1)

    for row in range(min_row, max_row + 1):
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
    self.render_headers()
    # TODO: Make the vertical scrollbar always visible?
    ig.begin_child('Frame Sheet Rows', flags=ig.WINDOW_ALWAYS_VERTICAL_SCROLLBAR)
    self.render_rows()
    ig.end_child()


def render_ui(model: Model, window_size: Tuple[int, int]) -> None:
  ig.set_next_window_position(0, 0)
  ig.set_next_window_size(*window_size)
  ig.begin(
    'Main',
    False,
    ig.WINDOW_NO_SAVED_SETTINGS | ig.WINDOW_NO_RESIZE | ig.WINDOW_NO_TITLE_BAR,
  )

  ig.columns(2)
  ig.next_column()

  frame_sheet = model.frame_sheets[0]
  ig.set_next_window_content_size(frame_sheet.get_content_width(), 0)
  ig.begin_child(
    'Frame Sheet',
    height=int(window_size[1] * 0.7),
    flags=ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
  )
  frame_sheet.render()
  ig.end_child()

  ig.begin_child(
    'Variable Explorer',
    flags=ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
  )
  ig.end_child()

  ig.columns(1)

  ig.end()


def render(window, ig_renderer, model: Model) -> None:
  ig.new_frame()
  render_ui(model, glfw.get_window_size(window))
  ig.end_frame()
  ig.render()

  gl.glClearColor(1.0, 1.0, 1.0, 1.0)
  gl.glClear(gl.GL_COLOR_BUFFER_BIT)

  draw_data = ig.get_draw_data()
  ig_renderer.render(draw_data)

  glfw.swap_buffers(window)


def run() -> None:
  glfw.init()

  glfw.window_hint(glfw.CONTEXT_VERSION_MAJOR, 3)
  glfw.window_hint(glfw.CONTEXT_VERSION_MINOR, 3)
  glfw.window_hint(glfw.OPENGL_PROFILE, glfw.OPENGL_COMPAT_PROFILE) # TODO: Core
  glfw.window_hint(glfw.OPENGL_FORWARD_COMPAT, True)

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
