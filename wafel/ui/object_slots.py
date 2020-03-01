from typing import *

import imgui as ig

from wafel.core import ObjectType


def render_object_slots(
  object_types: List[Optional[ObjectType]],
  on_select: Callable[[int], None],
):
  button_size = 50
  window_left = ig.get_window_position()[0]
  window_right = window_left + ig.get_window_content_region_max()[0]
  prev_item_right = window_left
  style = ig.get_style()

  for slot, object_type in enumerate(object_types):
    item_right = prev_item_right + style.item_spacing[0] + button_size
    if item_right > window_right:
      prev_item_right = window_left
    elif slot != 0:
      ig.same_line()
    prev_item_right = prev_item_right + style.item_spacing[0] + button_size

    if object_type is None:
      label = str(slot)
    else:
      label = str(slot) + '\n' + object_type.name

    if ig.button(label + '##slot-' + str(slot), button_size, button_size):
      on_select(slot)
