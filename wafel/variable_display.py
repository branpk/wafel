from typing import *
from enum import Enum, auto

import imgui as ig

from wafel.variable_format import VariableFormatter, TextFormatter, CheckboxFormatter
from wafel.util import *


class VariableDisplayAction(Enum):
  NONE = auto()
  BEGIN_EDIT = auto()
  END_EDIT = auto()


# TODO: Try to get rid of this
class VariableDisplayEditState:
  def __init__(self) -> None:
    self.initial_focus = False


def _display_text(
  id: str,
  data: Any,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  highlight: bool,
  on_select: Callable[[], None],
) -> VariableDisplayAction:
  clicked, _ = ig.selectable(
    dcast(str, formatter.output(data)) + '##t-' + id,
    highlight,
    width=size[0],
    height=size[1],
    flags=ig.SELECTABLE_ALLOW_DOUBLE_CLICK,
  )

  if clicked:
    on_select()
    if ig.is_mouse_double_clicked():
      return VariableDisplayAction.BEGIN_EDIT

  return VariableDisplayAction.NONE


def _display_text_edit(
  id: str,
  data: Any,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  edit_state: VariableDisplayEditState,
  on_edit: Callable[[Any], None] = lambda _: None,
) -> VariableDisplayAction:
  cursor_pos = ig.get_cursor_pos()
  cursor_pos = (
    ig.get_window_position()[0] + cursor_pos[0],
    ig.get_window_position()[1] + cursor_pos[1] - ig.get_scroll_y(),
  )

  ig.push_item_width(size[0])
  _, input = ig.input_text('##te-' + id, dcast(str, formatter.output(data)), 32)
  ig.pop_item_width()

  if not edit_state.initial_focus:
    ig.set_keyboard_focus_here(-1)
    edit_state.initial_focus = True
  elif not ig.is_item_active():
    return VariableDisplayAction.END_EDIT

  try:
    input_data = formatter.input(input)
    if input_data != data:
      on_edit(input_data)
  except:
    # TODO: Show error message
    dl = ig.get_window_draw_list()
    dl.add_rect(
      cursor_pos[0],
      cursor_pos[1],
      cursor_pos[0] + size[0],
      cursor_pos[1] + ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1],
      0xFF0000FF,
    )

  return VariableDisplayAction.NONE


def _display_checkbox(
  id: str,
  data: Any,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  highlight: bool,
  on_edit: Callable[[Any], None],
  on_select: Callable[[], None],
) -> VariableDisplayAction:
  cursor_pos = ig.get_cursor_pos()
  _, input = ig.checkbox('##cb-' + id, dcast(bool, formatter.output(data)))

  input_data = formatter.input(input)
  if input_data != data:
    on_edit(input_data)

  ig.set_cursor_pos(cursor_pos)
  clicked, _ = ig.selectable(
    '##cbbg-' + id,
    highlight,
    width=size[0],
    height=size[1],
  )
  if clicked:
    on_select()

  return VariableDisplayAction.NONE


# TODO: This doesn't feel right
def display_variable_data(
  id: str,
  data: Any,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  edit_state: Optional[VariableDisplayEditState] = None,
  highlight: bool = False,
  on_edit: Callable[[Any], None] = lambda _: None,
  on_select: Callable[[], None] = lambda: None,
) -> VariableDisplayAction:
  if isinstance(formatter, TextFormatter):
    if edit_state is None:
      return _display_text(
        id,
        data,
        formatter,
        size,
        highlight,
        on_select,
      )
    else:
      return _display_text_edit(
        id,
        data,
        formatter,
        size,
        edit_state,
        on_edit,
      )
  elif isinstance(formatter, CheckboxFormatter):
    return _display_checkbox(
      id,
      data,
      formatter,
      size,
      highlight,
      on_edit,
      on_select,
    )
  else:
    raise NotImplementedError(formatter)
