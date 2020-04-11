from typing import *

import wafel.imgui as ig

from wafel.util import *
from wafel.variable_format import VariableFormatter
from wafel.ui.variable_value import render_variable_value


T = TypeVar('T')

def render_variable_cell(
  id: str,
  value: T,
  formatter: VariableFormatter,
  cell_size: Tuple[int, int],
  is_selected: bool,
  frame: Optional[int] = None,
  highlight_range: Optional[range] = None,
) -> Tuple[Maybe[T], bool, bool, bool]:
  ig.push_id(id)

  window_pos = ig.get_window_position()
  item_spacing = ig.get_style().item_spacing

  cell_cursor_pos = ig.get_cursor_pos()
  cell_cursor_pos = (
    cell_cursor_pos.x + window_pos.x - item_spacing.x,
    cell_cursor_pos.y + window_pos.y - ig.get_scroll_y() - item_spacing.y,
  )

  if highlight_range is not None:
    assert frame is not None
    margin = 5
    offset_top = margin if frame == highlight_range.start else 0
    offset_bottom = margin if frame == highlight_range.stop - 1 else 0
    dl = ig.get_window_draw_list()
    dl.add_rect_filled(
      cell_cursor_pos[0] + margin,
      cell_cursor_pos[1] + offset_top,
      cell_cursor_pos[0] + cell_size[0] - margin,
      cell_cursor_pos[1] + cell_size[1] - offset_bottom,
      ig.get_color_u32_rgba(0.2, 0.6, 0, 0.3),
    )

  changed_data, selected, pressed = render_variable_value(
    'value',
    value,
    formatter,
    (
      cell_size[0] - 2 * item_spacing.x,
      cell_size[1] - 2 * item_spacing.y,
    ),
    highlight = is_selected,
  )

  clear_edit = ig.is_item_hovered() and ig.is_mouse_down(2)

  ig.pop_id()
  return changed_data, clear_edit, selected, pressed


__all__ = ['render_variable_cell']
