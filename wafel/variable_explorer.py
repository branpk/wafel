from typing import *
from dataclasses import dataclass

import imgui as ig

from wafel.model import Model
from wafel.core import ObjectId, Variable, VariableGroup, VariableParam, ObjectType
from wafel.variable_display import VariableDisplayAction, display_variable_data
from wafel.variable_format import Formatters
from wafel.frame_sheet import CellEditState
import wafel.ui as ui


@dataclass(frozen=True)
class TabId:
  name: str
  object_id: Optional[ObjectId] = None


@dataclass(frozen=True)
class VariableCell:
  tab: TabId
  variable: Variable
  frame: int


class VariableExplorer:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    self.model = model
    self.formatters = formatters
    self.open_tabs: List[TabId] = []
    self.rendered = False
    self.cell_edit_state: CellEditState[VariableCell] = CellEditState()

    fixed_tabs = [
      TabId('Input'),
      TabId('Mario'),
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
    object_types: List[Optional[ObjectType]] = []

    state = self.model.timeline[self.model.selected_frame]
    for slot in range(240):
      object_id = slot
      object_types.append(self.model.get_object_type(state, object_id))

    def on_select(slot: int) -> None:
      object_id = slot
      self.open_tab(TabId('_object', object_id))

    ui.render_object_slots(object_types, on_select)


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

    ig.selectable(variable.label + '##ve-label-' + str(hash(cell)), width=80)

    if ig.begin_drag_drop_source():
      ig.text(variable.label)
      ig.set_drag_drop_payload('ve-var', variable.id.to_bytes())
      ig.end_drag_drop_source()

    ig.same_line()

    cell_width = 80
    cell_height = ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1]

    cell_cursor_pos = ig.get_cursor_pos()
    cell_cursor_pos = (
      cell_cursor_pos[0] + ig.get_window_position()[0],
      cell_cursor_pos[1] + ig.get_window_position()[1] - ig.get_scroll_y(),
    )

    def on_edit(data: Any) -> None:
      self.model.edits.edit(frame, variable, data)

    action = display_variable_data(
      've-var-' + str(hash(cell)),
      data,
      self.formatters[variable],
      (cell_width, cell_height),
      self.cell_edit_state.get(cell),
      on_edit = on_edit,
    )

    if action == VariableDisplayAction.BEGIN_EDIT:
      self.cell_edit_state.begin_edit(cell)
    elif action == VariableDisplayAction.END_EDIT:
      self.cell_edit_state.end_edit()

    if ig.is_item_hovered() and ig.is_mouse_down(2):
      self.model.edits.reset(frame, variable.id)

    if self.model.edits.is_edited(frame, variable.id):
      dl = ig.get_window_draw_list()
      spacing = ig.get_style().item_spacing
      spacing = (spacing[0] / 2, spacing[1] / 2)
      dl.add_rect(
        cell_cursor_pos[0] - spacing[0],
        cell_cursor_pos[1] - spacing[1],
        cell_cursor_pos[0] + cell_width + spacing[0] - 1,
        cell_cursor_pos[1] + cell_height + spacing[1] - 1,
        ig.get_color_u32_rgba(0.8, 0.6, 0, 1),
      )


  def render_stick_control(self, stick_x_var: Variable, stick_y_var: Variable) -> None:
    stick_x = self.model.get(stick_x_var)
    stick_y = self.model.get(stick_y_var)

    new_values = ui.render_joystick_control(stick_x, stick_y)

    if new_values is not None:
      new_stick_x, new_stick_y = new_values

      self.model.edits.edit(self.model.selected_frame, stick_x_var, new_stick_x)
      self.model.edits.edit(self.model.selected_frame, stick_y_var, new_stick_y)


  def render_input_tab(self, tab: TabId) -> None:
    ig.columns(2)
    ig.set_column_width(-1, 160)

    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      self.render_variable(tab, variable)

    ig.next_column()

    self.render_stick_control(
      self.model.variables['input-stick-x'],
      self.model.variables['input-stick-y'],
    )

    ig.columns(1)


  def render_variable_tab(self, tab: TabId) -> None:
    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      self.render_variable(tab, variable)


  def render_tab_contents(self, tab: TabId) -> None:
    if tab.name == 'Objects':
      self.render_objects_tab()
    elif tab.name == 'Input':
      self.render_input_tab(tab)
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
