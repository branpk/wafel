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
  is_edited: bool,
  is_selected: bool,
) -> Tuple[Maybe[T], bool, bool]:
  ig.push_id(id)

  window_pos = ig.get_window_position()
  item_spacing = ig.get_style().item_spacing

  cell_cursor_pos = ig.get_cursor_pos()
  cell_cursor_pos = (
    cell_cursor_pos.x + window_pos.x - item_spacing.x,
    cell_cursor_pos.y + window_pos.y - ig.get_scroll_y() - item_spacing.y,
  )

  changed_data, selected = render_variable_value(
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

  if is_edited:
    dl = ig.get_window_draw_list()
    dl.add_rect(
      cell_cursor_pos[0],
      cell_cursor_pos[1],
      cell_cursor_pos[0] + cell_size[0],
      cell_cursor_pos[1] + cell_size[1],
      ig.get_color_u32_rgba(0.8, 0.6, 0, 1),
    )

  ig.pop_id()
  return changed_data, clear_edit, selected


__all__ = ['render_variable_cell']
