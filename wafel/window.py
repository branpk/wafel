from typing import *

import glfw
from imgui.integrations.glfw import GlfwRenderer
from OpenGL import GL as gl

import wafel.imgui as ig
import wafel.config as config
from wafel.util import *


rendering = False


def _render_window(window, ig_renderer, render: Callable[[str], None]) -> None:
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

  window_size = glfw.get_window_size(window)

  gl.glScissor(0, 0, *window_size)
  gl.glClearColor(0.06, 0.06, 0.06, 1.0)
  gl.glClear(gl.GL_COLOR_BUFFER_BIT)

  ig.get_style().colors[ig.COLOR_WINDOW_BACKGROUND] = (0, 0, 0, 0)
  ig.new_frame()

  ig.set_next_window_position(0, 0)
  ig.set_next_window_size(*window_size)
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
  ig_renderer.render(draw_data)

  glfw.swap_buffers(window)
  rendering = False


def open_window_and_run(render: Callable[[str], None], maximize = False) -> None:
  glfw.init()

  glfw.window_hint(glfw.CONTEXT_VERSION_MAJOR, 3)
  glfw.window_hint(glfw.CONTEXT_VERSION_MINOR, 3)
  glfw.window_hint(glfw.OPENGL_PROFILE, glfw.OPENGL_COMPAT_PROFILE) # TODO: Core
  glfw.window_hint(glfw.OPENGL_FORWARD_COMPAT, True)
  glfw.window_hint(glfw.SAMPLES, 4)

  glfw.window_hint(glfw.VISIBLE, False)
  window = glfw.create_window(800, 600, 'Wafel ' + config.version_str('.'), None, None)
  glfw.set_window_size_limits(window, 1, 1, glfw.DONT_CARE, glfw.DONT_CARE)
  if maximize:
    glfw.maximize_window(window)
  glfw.show_window(window)

  glfw.make_context_current(window)
  glfw.swap_interval(0)

  ig_context = ig.create_context()
  ig_renderer = GlfwRenderer(window)
  ig_renderer.io.ini_filename = None

  def refresh_callback(window):
    _render_window(window, ig_renderer, render)
  glfw.set_window_refresh_callback(window, refresh_callback)

  while not glfw.window_should_close(window):
    glfw.poll_events()
    ig_renderer.process_inputs()
    _render_window(window, ig_renderer, render)

  ig_renderer.shutdown()
  ig.destroy_context(ig_context)

  glfw.destroy_window(window)
  glfw.terminate()
