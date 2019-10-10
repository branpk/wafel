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
from wafel.core import *
from wafel.model import Model
from wafel.frame_sheet import FrameSheet
from wafel.variable_explorer import VariableExplorer
from wafel.game_view import GameView
from wafel.frame_slider import *
from wafel.variable_format import Formatters


class View:

  def __init__(self, model: Model) -> None:
    self.model = model

    self.formatters = Formatters()

    self.frame_sheets: List[FrameSheet] = [FrameSheet(self.model, self.formatters)]
    self.variable_explorer = VariableExplorer(self.model, self.formatters)
    self.game_views: List[GameView] = [
      GameView(self.model, CameraMode.ROTATE),
      GameView(self.model, CameraMode.BIRDS_EYE),
    ]
    self.frame_slider = FrameSlider(self.model)


  def render_left_column(self, window_size: Tuple[int, int]) -> None:
    slider_space = 45

    ig.begin_child(
      'Game View 1',
      height=int(window_size[1] // 2) - slider_space // 2,
      border=True,
    )
    self.game_views[0].render(window_size)
    ig.end_child()

    ig.begin_child(
      'Game View 2',
      height=int(window_size[1] // 2) - slider_space // 2,
      border=True,
    )
    self.game_views[1].render(window_size)
    ig.end_child()

    self.frame_slider.render()


  def render_right_column(self, window_size: Tuple[int, int]) -> None:
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
        variable = self.model.variables[VariableId.from_bytes(payload)]
        frame_sheet.append_variable(variable)
      ig.end_drag_drop_target()

    ig.begin_child('Variable Explorer', border=True)
    self.variable_explorer.render()
    ig.end_child()


  def render(self, window_size: Tuple[int, int]) -> None:
    ig.set_next_window_position(0, 0)
    ig.set_next_window_size(*window_size)
    ig.begin(
      'Main',
      False,
      ig.WINDOW_NO_SAVED_SETTINGS | ig.WINDOW_NO_RESIZE | ig.WINDOW_NO_TITLE_BAR,
    )

    ig.columns(2)
    self.render_left_column(window_size)
    ig.next_column()
    self.render_right_column(window_size)
    ig.columns(1)

    ig.end()


def render(window, ig_renderer, view: View) -> None:
  # TODO: Move keyboard handling somewhere else
  # TODO: Make this work when holding down mouse button
  model = view.model
  ig.get_io().key_repeat_rate = 1/30
  if not ig.get_io().want_capture_keyboard:
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_DOWN_ARROW)) or \
        ig.is_key_pressed(ig.get_key_index(ig.KEY_RIGHT_ARROW)):
      model.selected_frame += 1
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_UP_ARROW)) or \
        ig.is_key_pressed(ig.get_key_index(ig.KEY_LEFT_ARROW)):
      model.selected_frame -= 1
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_PAGE_DOWN)):
      model.selected_frame += 5
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_PAGE_UP)):
      model.selected_frame -= 5

  style = ig.get_style()
  style.window_rounding = 0

  window_size = glfw.get_window_size(window)

  gl.glScissor(0, 0, *window_size)
  gl.glClearColor(0.06, 0.06, 0.06, 1.0)
  gl.glClear(gl.GL_COLOR_BUFFER_BIT)

  ig.get_style().colors[ig.COLOR_WINDOW_BACKGROUND] = (0, 0, 0, 0)
  ig.new_frame()
  view.render(window_size)
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
    render(window, ig_renderer, view)
  glfw.set_window_refresh_callback(window, refresh_callback)

  model = Model()
  view = View(model)

  while not glfw.window_should_close(window):
    glfw.poll_events()
    ig_renderer.process_inputs()
    model.timeline.balance_distribution(1/120)
    render(window, ig_renderer, view)

  ig_renderer.shutdown()
  ig.destroy_context(ig_context)

  glfw.destroy_window(window)
  glfw.terminate()
