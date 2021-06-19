from typing import *

import wafel.imgui as ig
from wafel.util import *
from wafel.local_state import use_state


T = TypeVar('T')


def render_input_text_with_error(
  id: str,
  value: str,
  buffer_size: int,
  width: int,
  validate: Callable[[str], T],
) -> Maybe[T]:
  ig.push_id(id)

  cursor_pos = ig.get_cursor_pos()
  cursor_pos = (
    ig.get_window_position()[0] + cursor_pos[0],
    ig.get_window_position()[1] + cursor_pos[1] - ig.get_scroll_y(),
  )

  ig.push_item_width(width)
  changed, new_value = ig.input_text('##input', value, buffer_size)
  ig.pop_item_width()

  result_value = None
  if changed:
    try:
      result_value = Just(validate(new_value))
    except:
      # TODO: Show error message
      dl = ig.get_window_draw_list()
      dl.add_rect(
        cursor_pos[0],
        cursor_pos[1],
        cursor_pos[0] + width,
        cursor_pos[1] + ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1],
        ig.get_color_u32_rgba(1, 0, 0, 1),
      )
      new_value = value

  ig.pop_id()
  return result_value
