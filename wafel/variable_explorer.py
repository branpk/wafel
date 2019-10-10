from typing import *

import imgui as ig

from wafel.model import Model
from wafel.core import ObjectId, Variable, VariableGroup, VariableParam, VariableEdit
from wafel.variable_display import VariableDisplayAction, display_variable_data
from wafel.variable_format import Formatters
from wafel.frame_sheet import CellEditState


class TabId:
  def __init__(self, name: str, object_id: Optional[ObjectId] = None) -> None:
    self.name = name
    self.object_id = object_id

  def __eq__(self, other: object) -> bool:
    if not isinstance(other, TabId):
      return False
    return self.name == other.name and self.object_id == other.object_id

  def __hash__(self) -> int:
    return hash((self.name, self.object_id))


class VariableCell:
  def __init__(self, tab: TabId, variable: Variable, frame: int) -> None:
    self.tab = tab
    self.variable = variable
    self.frame = frame

  def __eq__(self, other: object) -> bool:
    if not isinstance(other, VariableCell):
      return False
    return self.tab == other.tab and \
      self.variable == other.variable and \
      self.frame == other.frame

  def __hash__(self) -> int:
    return hash((self.tab, self.variable, self.frame))


class VariableExplorer:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    self.model = model
    self.formatters = formatters
    self.open_tabs: List[TabId] = []
    self.rendered = False
    self.cell_edit_state: CellEditState[VariableCell] = CellEditState()

    fixed_tabs = [
      TabId('Input'),
      TabId('Misc'),
      TabId('Objects'),
    ]
    for tab in fixed_tabs:
      self.open_tab(tab)

    self.current_tab = self.open_tabs[0]


  def open_tab(self, tab: TabId) -> None:
    if tab not in self.open_tabs:
      self.open_tabs.append(tab)
    self.current_tab = tab


  def close_tab(self, tab: TabId) -> None:
    if self.current_tab == tab:
      # TODO
      pass
    if tab in self.open_tabs:
      self.open_tabs.remove(tab)


  def get_tab_label(self, tab: TabId) -> str:
    if tab.object_id is not None:
      state = self.model.timeline[self.model.selected_frame]
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
        self.model.timeline[self.model.selected_frame],
        object_id,
      )
      if object_type is None:
        label = str(slot)
      else:
        label = str(slot) + '\n' + object_type.name

      if ig.button(label + '##slot-' + str(slot), 50, 50):
        self.open_tab(TabId('_object', object_id))


  def get_variables_for_tab(self, tab: TabId) -> List[Variable]:
    if tab.object_id is None:
      return self.model.variables.group(VariableGroup(tab.name))

    state = self.model.timeline[self.model.selected_frame]
    object_type = self.model.get_object_type(state, tab.object_id)
    if object_type is None:
      return []

    return [
      var.at_object(tab.object_id)
        for var in self.model.variables.group(VariableGroup.object(object_type.name))
    ]


  def render_variable(self, tab: TabId, variable: Variable) -> None:
    frame = self.model.selected_frame
    state = self.model.timeline[frame]
    cell = VariableCell(tab, variable, state.frame)
    data = variable.get({ VariableParam.STATE: state })
    del state

    ig.selectable(variable.display_name + '##ve-label-' + str(hash(cell)), width=80)

    if ig.begin_drag_drop_source():
      ig.text(variable.display_name)
      ig.set_drag_drop_payload('ve-var', variable.id.to_bytes())
      ig.end_drag_drop_source()

    ig.same_line()

    def on_edit(data: Any) -> None:
      self.model.edits.add(frame, VariableEdit(variable, data))

    action = display_variable_data(
      've-var-' + str(hash(cell)),
      data,
      self.formatters[variable],
      (80, ig.get_text_line_height() + 2*ig.get_style().frame_padding[1]),
      self.cell_edit_state.get(cell),
      on_edit = on_edit,
    )

    if action == VariableDisplayAction.BEGIN_EDIT:
      self.cell_edit_state.begin_edit(cell)
    elif action == VariableDisplayAction.END_EDIT:
      self.cell_edit_state.end_edit()


  def render_variable_tab(self, tab: TabId) -> None:
    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      self.render_variable(tab, variable)


  def render_tab_contents(self, tab: TabId) -> None:
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
