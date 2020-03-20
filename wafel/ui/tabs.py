from typing import *
from dataclasses import dataclass

import wafel.imgui as ig
from wafel.local_state import use_state


@dataclass(frozen=True)
class TabInfo:
  id: str
  label: str
  closable: bool
  render: Callable[[str], None]

def render_tabs(id: str, tabs: List[TabInfo]) -> Optional[int]:
  ig.push_id(id)
  ig.columns(2)

  closed_tab = None

  rendered = use_state('rendered', False)
  if not rendered.value:
    rendered.value = True
    ig.set_column_width(-1, 120)

  if len(tabs) == 0:
    ig.pop_id()
    return closed_tab

  selected_tab_index = use_state('selected-tab-index', 0)
  selected_tab_id = use_state('selected-tab', tabs[0].id)

  # Handle deletion/insertion
  if selected_tab_index.value >= len(tabs):
    selected_tab_index.value = len(tabs) - 1
  if tabs[selected_tab_index.value].id != selected_tab_id.value:
    matching_indices = [i for i in range(len(tabs)) if tabs[i].id == selected_tab_id.value]
    if len(matching_indices) > 0:
      selected_tab_index.value = matching_indices[0]
    else:
      selected_tab_id.value = tabs[selected_tab_index.value].id

  ig.begin_child('tabs')
  for i, tab in enumerate(tabs):
    _, selected = ig.selectable(
      tab.label + '##tab-' + tab.id,
      selected_tab_id.value == tab.id,
    )
    if selected:
      selected_tab_index.value = i
      selected_tab_id.value = tab.id

    if tab.closable:
      if ig.is_item_hovered() and ig.is_mouse_clicked(2):
        closed_tab = i
      if ig.begin_popup_context_item(f'##ctx-{tab.id}'):
        if ig.selectable('Close')[0]:
          closed_tab = i
        ig.end_popup_context_item()
  ig.end_child()

  ig.next_column()

  ig.begin_child('content')
  tabs[selected_tab_index.value].render(selected_tab_id.value) # type: ignore
  ig.end_child()

  ig.columns(1)
  ig.pop_id()

  return closed_tab


__all__ = ['TabInfo', 'render_tabs']
