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
    self.selected_frame = ReactiveValue(0)
    self.timeline.add_hotspot(self.selected_frame)

    self.frame_sheets: List[FrameSheet] = []

    self.dbg_reload_graphics = ReactiveValue(())

  def path(self, path: str) -> DataPath:
    return DataPath.parse(self.lib, path)


class FrameSheet:
  pass


class SpreadsheetColumn:
  def __init__(
    self,
    label: str,
  ) -> None:
    self.label = label
    self.width = 100
    self.rendered = False


class SpreadsheetView:
  def __init__(self):
    self.columns = [
      SpreadsheetColumn('C' + str(i))
        for i in range(30)
    ]
    self.row_size = 20
    self.num_rows = 1000000

  def width(self) -> int:
    return sum(col.width for col in self.columns)

  def render(self) -> None:
    ig.columns(len(self.columns))
    for column in self.columns:
      ig.text(column.label)
      # TODO: Figure out how to adjust content size to match
      if not column.rendered:
        ig.set_column_width(-1, column.width)
        column.rendered = True
      else:
        column.width = ig.get_column_width(-1)
      ig.next_column()
    ig.separator()
    ig.columns(1)

    # TODO: Set child content size to make vertical scrollbar always present on right?
    ig.begin_child('Frame Sheet Rows', flags=ig.WINDOW_ALWAYS_VERTICAL_SCROLLBAR)
    ig.columns(len(self.columns))

    min_row = int(ig.get_scroll_y()) // self.row_size
    min_row = max(min_row, 0)
    max_row = int(ig.get_scroll_y() + ig.get_window_height()) // self.row_size
    max_row = min(max_row, self.num_rows - 1)

    x = 0
    for row in range(min_row, max_row + 1):
      for column in self.columns:
        if row == min_row:
          ig.set_cursor_pos((x, row * self.row_size))
        padding = 8  # TODO: Compute
        ig.selectable(str(row) + ' ' + column.label, height=self.row_size - padding)
        ig.set_column_width(-1, column.width)
        x += column.width
        ig.next_column()
      ig.separator()

    ig.set_cursor_pos((0, self.num_rows * self.row_size))

    ig.columns(1)
    ig.end_child()


spreadsheet = SpreadsheetView()

def render_ui(window_dims: Tuple[int, int]) -> None:
  ig.set_next_window_position(0, 0)
  ig.set_next_window_size(*window_dims)
  ig.set_next_window_content_size(spreadsheet.width(), 0)
  ig.begin(
    'Frame Sheet',
    False,
    ig.WINDOW_NO_SAVED_SETTINGS | ig.WINDOW_NO_RESIZE | ig.WINDOW_NO_TITLE_BAR | \
      ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
  )
  spreadsheet.render()
  ig.end()


def run():
  glfw.init()

  glfw.window_hint(glfw.CONTEXT_VERSION_MAJOR, 3)
  glfw.window_hint(glfw.CONTEXT_VERSION_MINOR, 3)
  glfw.window_hint(glfw.OPENGL_PROFILE, glfw.OPENGL_COMPAT_PROFILE) # TODO: Core
  glfw.window_hint(glfw.OPENGL_FORWARD_COMPAT, True)

  glfw.window_hint(glfw.VISIBLE, False)
  window = glfw.create_window(800, 600, 'Wafel', None, None)
  # glfw.maximize_window(window)
  glfw.show_window(window)

  glfw.make_context_current(window)

  ig_context = ig.create_context()
  ig_renderer = GlfwRenderer(window)

  # TODO: Disable imgui.ini

  while not glfw.window_should_close(window):
    glfw.poll_events()
    ig_renderer.process_inputs()

    ig.new_frame()
    render_ui(glfw.get_window_size(window))
    ig.end_frame()
    ig.render()

    gl.glClearColor(1.0, 1.0, 1.0, 1.0)
    gl.glClear(gl.GL_COLOR_BUFFER_BIT)

    draw_data = ig.get_draw_data()
    ig_renderer.render(draw_data)

    glfw.swap_buffers(window)

  ig_renderer.shutdown()
  ig.destroy_context(ig_context)

  glfw.destroy_window(window)
  glfw.terminate()
