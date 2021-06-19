from typing import *

import wafel.imgui as ig
from wafel.loading import Progress


def render_loading_bar(id: str, progress: Progress, width: float) -> None:
  ig.push_id(id)
  ig.begin_child(id)

  ig.text(progress.status)

  dl = ig.get_window_draw_list()

  initial_cursor_pos = ig.get_cursor_pos()
  top_left = (
    initial_cursor_pos[0] + ig.get_window_position()[0],
    initial_cursor_pos[1] + ig.get_window_position()[1] - ig.get_scroll_y(),
  )
  size = (width, 30)

  dl.add_rect_filled(
    top_left[0],
    top_left[1],
    top_left[0] + size[0],
    top_left[1] + size[1],
    ig.get_color_u32_rgba(0, 0, 0, 0.3),
  )

  padding = 3
  dl.add_rect_filled(
    top_left[0] + padding,
    top_left[1] + padding,
    max(top_left[0] + size[0] * progress.progress - padding, top_left[0] + padding + 3),
    top_left[1] + size[1] - padding,
    ig.get_color_u32_rgba(*ig.get_style().colors[ig.COLOR_FRAME_BACKGROUND_ACTIVE]),
  )

  ig.set_cursor_pos((
    initial_cursor_pos[0],
    initial_cursor_pos[1] + size[1],
  ))

  ig.end_child()
  ig.pop_id()


__all__ = ['render_loading_bar']
