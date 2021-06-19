from typing import *

from wafel_core import ObjectBehavior

import wafel.imgui as ig


def render_object_slots(
  id: str,
  behaviors: List[Optional[ObjectBehavior]],
  behavior_name: Callable[[ObjectBehavior], str],
) -> Optional[int]:
  ig.push_id(id)

  button_size = 50
  window_left = ig.get_window_position()[0]
  window_right = window_left + ig.get_window_content_region_max()[0]
  prev_item_right = window_left
  style = ig.get_style()

  result = None

  for slot, behavior in enumerate(behaviors):
    item_right = prev_item_right + style.item_spacing[0] + button_size
    if item_right > window_right:
      prev_item_right = window_left
    elif slot != 0:
      ig.same_line()
    prev_item_right = prev_item_right + style.item_spacing[0] + button_size

    if behavior is None:
      label = str(slot)
    else:
      label = str(slot) + '\n' + behavior_name(behavior)

    if ig.button(label + '##slot-' + str(slot), button_size, button_size):
      result = slot

  ig.pop_id()
  return result


__all__ = ['render_object_slots']
