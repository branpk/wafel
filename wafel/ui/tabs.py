from typing import *
from dataclasses import dataclass

import wafel.imgui as ig
from wafel.local_state import use_state, use_state_with, push_local_state_rebase, \
  pop_local_state_rebase, get_local_state_id_stack


@dataclass(frozen=True)
class TabInfo:
  id: str
  label: str
  closable: bool
  render: Callable[[str], None]


def render_tabs(
  id: str,
  tabs: List[TabInfo],
  open_tab_index: Optional[int] = None,
  allow_windowing = False,
) -> Tuple[Optional[int], Optional[int]]:
  ig.push_id(id)
  root_id = get_local_state_id_stack()
  ig.columns(2)

  closed_tab = None

  rendered = use_state('rendered', False)
  if not rendered.value:
    rendered.value = True
    ig.set_column_width(-1, 120)

  if len(tabs) == 0:
    ig.pop_id()
    return None, closed_tab

  selected_tab_index = use_state_with('selected-tab-index', lambda: open_tab_index or 0)
  selected_tab_id = use_state_with('selected-tab', lambda: tabs[selected_tab_index.value].id)

  if open_tab_index is not None:
    selected_tab_index.value = open_tab_index
    selected_tab_id.value = tabs[open_tab_index].id

  windowed_tabs = use_state('windowed-tabs', cast(Set[str], set())).value

  # TODO: Change selected tab if windowed

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
    if tab.id in windowed_tabs:
      continue

    _, selected = ig.selectable(
      tab.label + '##tab-' + tab.id,
      selected_tab_id.value == tab.id,
    )
    if selected:
      selected_tab_index.value = i
      selected_tab_id.value = tab.id

    if tab.closable and ig.is_item_hovered() and ig.is_mouse_clicked(2):
      closed_tab = i

    if allow_windowing or tab.closable:
      if ig.begin_popup_context_item(f'##ctx-{tab.id}'):
        if allow_windowing and ig.selectable('Pop out')[0]:
          windowed_tabs.add(tab.id)
        if tab.closable and ig.selectable('Close')[0]:
          closed_tab = i
        ig.end_popup_context_item()

  ig.end_child()

  ig.next_column()

  ig.begin_child('content', flags=ig.WINDOW_HORIZONTAL_SCROLLING_BAR)
  tab = tabs[selected_tab_index.value]
  if tab.id not in windowed_tabs:
    push_local_state_rebase(('rebase-tabs',) + root_id)
    tab.render(tab.id) # type: ignore
    pop_local_state_rebase()
  ig.end_child()

  ig.columns(1)

  for tab_id in set(windowed_tabs):
    matching = [tab for tab in tabs if tab.id == tab_id]
    if len(matching) == 0:
      windowed_tabs.remove(tab.id)
      continue
    tab = matching[0]

    ig.set_next_window_size(*ig.get_content_region_max(), ig.ONCE)
    ig.set_next_window_position(*ig.get_window_position(), ig.ONCE)

    ig.push_style_color(ig.COLOR_WINDOW_BACKGROUND, 0.06, 0.06, 0.06, 0.94)
    _, opened = ig.begin(
      tab.label + '##window-' + tab.id,
      closable = True,
      flags = ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
    )
    push_local_state_rebase(('rebase-tabs',) + root_id)
    tab.render(tab.id) # type: ignore
    pop_local_state_rebase()
    ig.end()
    ig.pop_style_color()

    if not opened:
      windowed_tabs.remove(tab.id)

  ig.pop_id()
  return (
    None if open_tab_index == selected_tab_index.value else selected_tab_index.value,
    closed_tab,
  )


__all__ = ['TabInfo', 'render_tabs']
