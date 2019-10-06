from typing import *

import imgui as ig

from wafel.model import Model
from wafel.core import ObjectId, Variable, VariableGroup


class ExplorerTabId:
  def __init__(self, name: str, object_id: Optional[ObjectId] = None) -> None:
    self.name = name
    self.object_id = object_id

  def __eq__(self, other: object) -> bool:
    if not isinstance(other, ExplorerTabId):
      return False
    return self.name == other.name and self.object_id == other.object_id

  def __hash__(self) -> int:
    return hash((self.name, self.object_id))


class VariableExplorer:

  def __init__(self, model: Model) -> None:
    self.model = model
    self.open_tabs: List[ExplorerTabId] = []
    self.rendered = False

    fixed_tabs = [
      ExplorerTabId('Input'),
      ExplorerTabId('Misc'),
      ExplorerTabId('Objects'),
    ]
    for tab in fixed_tabs:
      self.open_tab(tab)

    self.current_tab = self.open_tabs[0]


  def open_tab(self, tab: ExplorerTabId) -> None:
    if tab not in self.open_tabs:
      self.open_tabs.append(tab)
    self.current_tab = tab


  def close_tab(self, tab: ExplorerTabId) -> None:
    if self.current_tab == tab:
      # TODO
      pass
    if tab in self.open_tabs:
      self.open_tabs.remove(tab)


  def get_tab_label(self, tab: ExplorerTabId) -> str:
    if tab.object_id is not None:
      state = self.model.timeline.frame(self.model.selected_frame).value
      object_type = self.model.get_object_type(state, tab.object_id)
      if object_type is None:
        return str(tab.object_id)
      else:
        return str(tab.object_id) + ': ' + object_type.name

    return tab.name


  def render_objects_tab(self) -> None:
    button_size = 50
    window_left = ig.get_window_position()[0]
    window_right = window_left + ig.get_window_content_region_max()[0]
    prev_item_right = window_left
    style = ig.get_style()

    for slot in range(240):
      item_right = prev_item_right + style.item_spacing[0] + button_size
      if item_right > window_right:
        prev_item_right = window_left
      elif slot != 0:
        ig.same_line()
      prev_item_right = prev_item_right + style.item_spacing[0] + button_size

      object_id = slot
      object_type = self.model.get_object_type(
        self.model.timeline.frame(self.model.selected_frame).value,
        object_id,
      )
      if object_type is None:
        label = str(slot)
      else:
        label = str(slot) + '\n' + object_type.name

      if ig.button(label + '##slot-' + str(slot), 50, 50):
        self.open_tab(ExplorerTabId('_object', object_id))


  def get_variables_for_tab(self, tab: ExplorerTabId) -> List[Variable]:
    if tab.object_id is None:
      return self.model.variables.group(VariableGroup(tab.name))

    state = self.model.timeline.frame(self.model.selected_frame).value
    object_type = self.model.get_object_type(state, tab.object_id)
    if object_type is None:
      return []

    return [
      var.at_object(tab.object_id)
        for var in self.model.variables.group(VariableGroup.object(object_type.name))
    ]


  def render_variable_tab(self, tab: ExplorerTabId) -> None:
    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      # TODO: Variable id
      ig.selectable(variable.display_name, width=80)

      if ig.begin_drag_drop_source():
        ig.text(variable.display_name)
        ig.set_drag_drop_payload('ve-var', variable.id.to_bytes())
        ig.end_drag_drop_source()

      # TODO: Reuse display/edit code with frame sheet
      ig.same_line()
      ig.push_item_width(80)
      ig.input_text('##' + variable.display_name, 'hey', 32, False)
      ig.pop_item_width()


  def render_tab_contents(self, tab: ExplorerTabId) -> None:
    if tab.name == 'Objects':
      self.render_objects_tab()
    else:
      self.render_variable_tab(tab)


  def render(self) -> None:
    ig.columns(2)
    if not self.rendered:
      self.rendered = True
      ig.set_column_width(-1, 120)

    ig.begin_child('Variable Explorer Tabs')
    for tab in self.open_tabs:
      _, selected = ig.selectable(
        self.get_tab_label(tab) + '##' + str(id(tab)),
        self.current_tab == tab,
      )
      if selected:
        self.current_tab = tab
    ig.end_child()

    ig.next_column()

    ig.begin_child('Variable Explorer Content')
    self.render_tab_contents(self.current_tab)
    ig.end_child()

    ig.columns(1)
