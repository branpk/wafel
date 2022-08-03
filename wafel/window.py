from typing import *

import glfw

import wafel_core as core

import wafel.imgui as ig
import wafel.config as config
from wafel.util import *
import wafel.graphics as graphics


first_render = True


def _render_window(render: Callable[[str], None]) -> Tuple[object, List[core.Scene]]:
  global first_render

  # TODO: clipboard length
  ig.set_clipboard_length(0)

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
  if first_render:
    # First render should be quick to avoid showing garbage for too long
    first_render = False
  else:
    render('root')
  ig.end()

  ig.end_frame()
  ig.render()

  draw_data = ig.get_draw_data()
  # ig_renderer.render(draw_data)

  return draw_data, graphics.take_scenes(), []


def open_window_and_run(render: Callable[[str], None], maximize = False) -> None:
  glfw.init()

  ig_context = ig.create_context()

  core.open_window_and_run(
    'Wafel ' + config.version_str('.'),
    lambda: _render_window(render),
  )

  ig.destroy_context(ig_context)
  glfw.destroy()
