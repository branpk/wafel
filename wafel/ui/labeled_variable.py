from typing import *

import imgui as ig

from wafel.core import VariableId
from wafel.variable_format import VariableFormatter
from wafel.util import *
from wafel.ui.variable_value import render_variable_value


T = TypeVar('T')

def render_labeled_variable(
  id: str,
  label: str,
  variable_id: VariableId,
  value: T,
  formatter: VariableFormatter,
  is_edited: bool,
) -> Tuple[Maybe[T], bool]:
  ig.push_id(id)

  ig.selectable(label + '##label', width=80)

  if ig.begin_drag_drop_source():
    ig.text(label)
    ig.set_drag_drop_payload('ve-var', variable_id.to_bytes())
    ig.end_drag_drop_source()

  ig.same_line()

  cell_width = 80
  cell_height = ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1]

  cell_cursor_pos = ig.get_cursor_pos()
  cell_cursor_pos = (
    cell_cursor_pos[0] + ig.get_window_position()[0],
    cell_cursor_pos[1] + ig.get_window_position()[1] - ig.get_scroll_y(),
  )

  changed_data, _ = render_variable_value(
    'value',
    value,
    formatter,
    (cell_width, cell_height),
  )

  clear_edit = is_edited and ig.is_item_hovered() and ig.is_mouse_down(2)

  if is_edited:
    dl = ig.get_window_draw_list()
    spacing = ig.get_style().item_spacing
    spacing = (spacing[0] / 2, spacing[1] / 2)
    dl.add_rect(
      cell_cursor_pos[0] - spacing[0],
      cell_cursor_pos[1] - spacing[1],
      cell_cursor_pos[0] + cell_width + spacing[0] - 1,
      cell_cursor_pos[1] + cell_height + spacing[1] - 1,
      ig.get_color_u32_rgba(0.8, 0.6, 0, 1),
    )

  ig.pop_id()
  return changed_data, clear_edit


__all__ = ['render_labeled_variable']
