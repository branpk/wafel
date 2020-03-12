from typing import *

import wafel.imgui as ig
from wafel.util import *


def render_frame_slider(
  id: str,
  current_frame: int,
  num_frames: int,
  loaded_frames: List[int] = [],
) -> Maybe[int]:
  ig.push_id(id)

  pos = ig.get_cursor_pos()
  pos = (
    pos[0] + ig.get_window_position()[0],
    pos[1] + ig.get_window_position()[1] - ig.get_scroll_y(),
  )
  width = ig.get_content_region_available_width()

  ig.push_item_width(width)
  changed, new_frame = ig.slider_int(
    '##slider',
    current_frame,
    0,
    num_frames - 1,
  )
  ig.pop_item_width()

  dl = ig.get_window_draw_list()
  for frame in loaded_frames:
    line_pos = pos[0] + frame / num_frames * width
    dl.add_line(
      line_pos, pos[1] + 13,
      line_pos, pos[1] + 18,
      ig.get_color_u32_rgba(1, 0, 0, 1),
    )

  ig.pop_id()

  if changed:
    return Just(new_frame)
  else:
    return None


__all__ = ['render_frame_slider']
