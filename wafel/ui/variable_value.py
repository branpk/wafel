from typing import *

from wafel.variable_format import VariableFormatter, TextFormatter, CheckboxFormatter
from wafel.util import *
from wafel.local_state import use_state
import wafel.imgui as ig


T = TypeVar('T')


def _render_text(
  value: T,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  highlight: bool,
) -> Tuple[Maybe[T], bool]:
  editing = use_state('editing', False)
  initial_focus = use_state('initial-focus', False)

  if not editing.value:
    clicked, _ = ig.selectable(
      dcast(str, formatter.output(value)) + '##text',
      highlight,
      width=size[0],
      height=size[1],
      flags = ig.SELECTABLE_ALLOW_DOUBLE_CLICK,
    )

    if clicked:
      if ig.is_mouse_double_clicked():
        editing.value = True
        initial_focus.value = False

    return None, clicked

  cursor_pos = ig.get_cursor_pos()
  cursor_pos = (
    ig.get_window_position()[0] + cursor_pos[0],
    ig.get_window_position()[1] + cursor_pos[1] - ig.get_scroll_y(),
  )

  ig.push_item_width(size[0])
  _, input = ig.input_text('##text-edit', dcast(str, formatter.output(value)), 32)
  ig.pop_item_width()

  if not initial_focus.value:
    ig.set_keyboard_focus_here(-1)
    initial_focus.value = True
  elif not ig.is_item_active():
    editing.value = False

  try:
    input_value = formatter.input(input)
    assert type(input_value) is type(value)
    if input_value != value:
      return Just(cast(T, input_value)), False
  except:
    # TODO: Show error message
    dl = ig.get_window_draw_list()
    dl.add_rect(
      cursor_pos[0],
      cursor_pos[1],
      cursor_pos[0] + size[0],
      cursor_pos[1] + ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1],
      ig.get_color_u32_rgba(1, 0, 0, 1),
    )

  return None, False


def _render_checkbox(
  value: T,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  highlight: bool,
) -> Tuple[Maybe[T], bool]:
  cursor_pos = ig.get_cursor_pos()
  _, input = ig.checkbox('##checkbox', dcast(bool, formatter.output(value)))

  ig.set_cursor_pos(cursor_pos)
  clicked, _ = ig.selectable(
    '##checkbox-background',
    highlight,
    width=size[0],
    height=size[1],
  )

  input_value = formatter.input(input)
  assert type(input_value) == type(value)
  if input_value != value:
    return Just(cast(T, input_value)), clicked
  else:
    return None, clicked


def render_variable_value(
  id: str,
  value: T,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  highlight: bool = False,
) -> Tuple[Maybe[T], bool]:
  ig.push_id(id)

  if isinstance(formatter, TextFormatter):
    result = _render_text(value, formatter, size, highlight)
  elif isinstance(formatter, CheckboxFormatter):
    result = _render_checkbox(value, formatter, size, highlight)
  else:
    raise NotImplementedError(formatter)

  ig.pop_id()
  return result


__all__ = ['render_variable_value']
