from typing import *

import wafel.imgui as ig
from wafel.local_state import use_state


TabInfo = Tuple[str, str, Callable[[str], None]]

def render_tabs(id: str, tabs: List[TabInfo]) -> None:
  ig.push_id(id)
  ig.columns(2)

  rendered = use_state('rendered', False)
  if not rendered.value:
    rendered.value = True
    ig.set_column_width(-1, 120)

  if len(tabs) == 0:
    ig.pop_id()
    return

  selected_tab_index = use_state('selected-tab-index', 0)
  selected_tab_id = use_state('selected-tab', tabs[0][0])

  # Handle deletion/insertion
  if selected_tab_index.value >= len(tabs):
    selected_tab_index.value = len(tabs) - 1
  if tabs[selected_tab_index.value][0] != selected_tab_id.value:
    matching_indices = [i for i in range(len(tabs)) if tabs[i][0] == selected_tab_id.value]
    if len(matching_indices) > 0:
      selected_tab_index.value = matching_indices[0]
    else:
      selected_tab_id.value = tabs[selected_tab_index.value][0]

  ig.begin_child('tabs')
  for i, (tab_id, label, _) in enumerate(tabs):
    _, selected = ig.selectable(
      label + '##' + tab_id,
      selected_tab_id.value == tab_id,
    )
    if selected:
      selected_tab_index.value = i
      selected_tab_id.value = tab_id
  ig.end_child()

  ig.next_column()

  ig.begin_child('content')
  tabs[selected_tab_index.value][2](selected_tab_id.value)
  ig.end_child()

  ig.columns(1)
  ig.pop_id()


__all__ = ['render_tabs']
