from typing import *
from dataclasses import dataclass

import imgui as ig

from wafel.ui.local_state import use_state


@dataclass
class JoystickControlState:
  start_value: Optional[Tuple[float, float]] = None

  def get_value(self, drag: Tuple[float, float]) -> Tuple[float, float]:
    assert self.start_value is not None
    return (self.start_value[0] + drag[0], self.start_value[1] + drag[1])

  @property
  def active(self) -> bool:
    return self.start_value is not None

  def set_active(self, value: Tuple[float, float]) -> None:
    if self.start_value is None:
      self.start_value = value

  def reset(self) -> None:
    self.start_value = None


def render_joystick_control(stick_x: float, stick_y: float) -> Optional[Tuple[float, float]]:
  ig.push_id('joystick-control')
  state = use_state('', JoystickControlState()).value

  dl = ig.get_window_draw_list()

  padding = 20
  content_region = ig.get_content_region_available()
  size = min(
    content_region.x - ig.get_style().scrollbar_size - 2 * padding,
    content_region.y - 2 * padding,
    200,
  )
  size = max(size, 100)

  initial_cursor_pos = ig.get_cursor_pos()
  top_left = (
    initial_cursor_pos[0] + ig.get_window_position()[0] + padding,
    initial_cursor_pos[1] + ig.get_window_position()[1] - ig.get_scroll_y() + padding,
  )

  dl.add_rect_filled(
    top_left[0],
    top_left[1],
    top_left[0] + size,
    top_left[1] + size,
    ig.get_color_u32_rgba(0, 0, 0, 0.3),
  )

  result = None

  if state.active and ig.is_mouse_down():
    new_offset = state.get_value(ig.get_mouse_drag_delta(lock_threshold=0))

    new_stick_x = new_offset[0] / size * 255 - 128
    new_stick_y = (1 - new_offset[1] / size) * 255 - 128
    new_stick_x = min(max(int(new_stick_x), -128), 127)
    new_stick_y = min(max(int(new_stick_y), -128), 127)

    if (new_stick_x, new_stick_y) != (stick_x, stick_y):
      stick_x, stick_y = new_stick_x, new_stick_y
      result = (stick_x, stick_y)

  offset = (
    (stick_x + 128) / 255 * size,
    (1 - (stick_y + 128) / 255) * size,
  )

  dl.add_line(
    top_left[0] + size / 2,
    top_left[1] + size / 2,
    top_left[0] + offset[0],
    top_left[1] + offset[1],
    ig.get_color_u32_rgba(1, 1, 1, 0.5),
  )

  button_size = 20
  button_pos = (
    padding + initial_cursor_pos[0] + offset[0] - button_size / 2,
    padding + initial_cursor_pos[1] + offset[1] - button_size / 2,
  )
  ig.set_cursor_pos(button_pos)
  ig.button('##joystick-button', button_size, button_size)

  ig.set_cursor_pos((
    initial_cursor_pos[0] + size + 2 * padding,
    initial_cursor_pos[1] + size + 2 * padding,
  ))

  if ig.is_item_active():
    state.set_active(offset)
  else:
    state.reset()

  ig.pop_id()
  return result
