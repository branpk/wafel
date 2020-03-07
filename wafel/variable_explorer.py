from typing import *
from dataclasses import dataclass

import imgui as ig

from wafel.model import Model
from wafel.core import ObjectId, Variable, VariableGroup, VariableParam, \
  ObjectType, VariableId
from wafel.variable_format import Formatters, VariableFormatter
import wafel.ui as ui
from wafel.util import *


@dataclass(frozen=True)
class TabId:
  name: str
  object_id: Optional[ObjectId] = None


class VariableExplorer:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    self.model = model
    self.formatters = formatters
    self.open_tabs: List[TabId] = []

    fixed_tabs = [
      TabId('Input'),
      TabId('Mario'),
      TabId('Misc'),
      TabId('Objects'),
    ]
    for tab in fixed_tabs:
      self.open_tab(tab)


  def open_tab(self, tab: TabId) -> None:
    if tab not in self.open_tabs:
      self.open_tabs.append(tab)
    self.current_tab = tab


  def close_tab(self, tab: TabId) -> None:
    if tab in self.open_tabs:
      self.open_tabs.remove(tab)


  def get_tab_label(self, tab: TabId) -> str:
    if tab.object_id is not None:
      with self.model.timeline[self.model.selected_frame] as state:
        object_type = self.model.get_object_type(state, tab.object_id)
      if object_type is None:
        return str(tab.object_id)
      else:
        return str(tab.object_id) + ': ' + object_type.name

    return tab.name


  def render_objects_tab(self) -> None:
    object_types: List[Optional[ObjectType]] = []

    with self.model.timeline[self.model.selected_frame] as state:
      for slot in range(240):
        object_id = slot
        object_types.append(self.model.get_object_type(state, object_id))

    selected_slot = ui.render_object_slots('object-slots', object_types)
    if selected_slot is not None:
      object_id = selected_slot
      self.open_tab(TabId('_object', object_id))


  def get_variables_for_tab(self, tab: TabId) -> List[Variable]:
    if tab.object_id is None:
      return self.model.variables.group(VariableGroup(tab.name))

    with self.model.timeline[self.model.selected_frame] as state:
      object_type = self.model.get_object_type(state, tab.object_id)
    if object_type is None:
      return []

    return [
      var.at_object(tab.object_id)
        for var in self.model.variables.group(VariableGroup.object(object_type.name))
    ]


  def render_variable(self, tab: TabId, variable: Variable) -> None:
    frame = self.model.selected_frame
    with self.model.timeline[frame] as state:
      value = variable.get({ VariableParam.STATE: state })

    changed_data, clear_edit = ui.render_labeled_variable(
      f'var-{hash((tab, variable))}',
      variable.label,
      variable.id,
      value,
      self.formatters[variable],
      self.model.edits.is_edited(frame, variable.id),
    )

    if changed_data is not None:
      self.model.edits.edit(frame, variable, changed_data.value)

    if clear_edit:
      self.model.edits.reset(frame, variable.id)


  def render_stick_control(self, stick_x_var: Variable, stick_y_var: Variable) -> None:
    stick_x = self.model.get(stick_x_var)
    stick_y = self.model.get(stick_y_var)

    new_values = ui.render_joystick_control('joystick-control', stick_x, stick_y)

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


  def render_tab_contents(self, id: str, tab: TabId) -> None:
    ig.push_id(id)
    if tab.name == 'Objects':
      self.render_objects_tab()
    elif tab.name == 'Input':
      self.render_input_tab(tab)
    else:
      self.render_variable_tab(tab)
    ig.pop_id()


  def render(self, id: str) -> None:
    ig.push_id(id)

    def render_tab(tab: TabId) -> Callable[[str], None]:
      return lambda id: self.render_tab_contents(id, tab)

    ui.render_tabs(
      'tabs',
      [
        (f'tab-{hash(tab)}', self.get_tab_label(tab), render_tab(tab))
          for tab in self.open_tabs
      ]
    )

    ig.pop_id()
