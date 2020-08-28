from typing import *

import glfw
from imgui.integrations.glfw import GlfwRenderer
from OpenGL import GL as gl

import ext_modules.core as core

import wafel.imgui as ig
import wafel.config as config
from wafel.util import *


rendering = False


def _render_window(render: Callable[[str], None]) -> object:
  global rendering
  if rendering:
    return
  rendering = True

  try:
    clipboard_length = len(glfw.get_clipboard_string(window))
  except:
    # Fails if the user has non-text copied
    clipboard_length = 0
  ig.set_clipboard_length(clipboard_length)

  style = ig.get_style()
  style.window_rounding = 0

  ig.get_style().colors[ig.COLOR_WINDOW_BACKGROUND] = (0, 0, 0, 0)
  ig.new_frame()

  ig.set_next_window_position(0, 0)
  ig.set_next_window_size(*ig.get_io().display_size)
  ig.begin(
    'Main',
    False,
    ig.WINDOW_NO_SAVED_SETTINGS | ig.WINDOW_NO_RESIZE | ig.WINDOW_NO_TITLE_BAR | ig.WINDOW_MENU_BAR |
      ig.WINDOW_NO_BRING_TO_FRONT_ON_FOCUS,
  )
  render('root')
  ig.end()

  ig.end_frame()
  ig.render()

  draw_data = ig.get_draw_data()
  # ig_renderer.render(draw_data)

  rendering = False
  return draw_data


def open_window_and_run(render: Callable[[str], None], maximize = False) -> None:
  glfw.init()

  # glfw.window_hint(glfw.CONTEXT_VERSION_MAJOR, 3)
  # glfw.window_hint(glfw.CONTEXT_VERSION_MINOR, 3)
  # glfw.window_hint(glfw.OPENGL_PROFILE, glfw.OPENGL_COMPAT_PROFILE) # TODO: Core
  # glfw.window_hint(glfw.OPENGL_FORWARD_COMPAT, True)
  # glfw.window_hint(glfw.SAMPLES, 4)

  # glfw.window_hint(glfw.VISIBLE, False)
  # window = glfw.create_window(800, 600, 'Wafel ' + config.version_str('.'), None, None)
  # glfw.set_window_size_limits(window, 1, 1, glfw.DONT_CARE, glfw.DONT_CARE)
  # if maximize:
  #   glfw.maximize_window(window)
  # glfw.show_window(window)

  # glfw.make_context_current(window)
  # glfw.swap_interval(0)

  ig_context = ig.create_context()
  ig.get_io().ini_filename = None

  core.open_window_and_run(
    'Wafel ' + config.version_str('.'),
    lambda: _render_window(render),
  )

  ig.destroy_context(ig_context)
  glfw.destroy()
