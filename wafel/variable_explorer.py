from typing import *
from dataclasses import dataclass
import math

import wafel.imgui as ig
from wafel.model import Model
from wafel.core import ObjectId, Variable, VariableGroup, ObjectType, VariableId, DataPath
from wafel.variable_format import Formatters, VariableFormatter
import wafel.ui as ui
from wafel.util import *
import wafel.joystick_util as joystick_util


@dataclass(frozen=True)
class TabId:
  name: str
  object_id: Optional[ObjectId] = None
  surface: Optional[int] = None


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
    self.current_tab = tab # TODO: This does nothing


  def open_surface_tab(self, surface: int) -> None:
    self.open_tab(TabId('_surface', surface=surface))


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

    elif tab.surface is not None:
      return f'Surface {tab.surface}'

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
    if tab.object_id is not None:
      with self.model.timeline[self.model.selected_frame] as state:
        object_type = self.model.get_object_type(state, tab.object_id)
      if object_type is None:
        return []

      return [
        var.at_object(tab.object_id)
          for var in self.model.variables.group(VariableGroup.object(object_type.name))
      ]

    elif tab.surface is not None:
      with self.model.timeline[self.model.selected_frame] as state:
        num_surfaces = DataPath.compile(self.model.lib, '$state.gSurfacesAllocated').get(state)
        if tab.surface >= dcast(int, num_surfaces):
          return []

        return [
          var.at_surface(tab.surface)
            for var in self.model.variables.group(VariableGroup('Surface'))
        ]

    else:
      return self.model.variables.group(VariableGroup(tab.name))


  def render_variable(self, tab: TabId, variable: Variable) -> None:
    frame = self.model.selected_frame
    with self.model.timeline[frame] as state:
      value = variable.get(state)

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


  def render_stick_control(self, id: str) -> None:
    stick_x_var = self.model.variables['input-stick-x']
    stick_y_var = self.model.variables['input-stick-y']

    stick_x = self.model.get(stick_x_var)
    stick_y = self.model.get(stick_y_var)

    n_x = 2 * ((stick_x + 128) / 255) - 1
    n_y = 2 * ((stick_y + 128) / 255) - 1
    new_n = ui.render_joystick_control(id, n_x, n_y)

    if new_n is not None:
      new_stick_x = int(0.5 * (new_n[0] + 1) * 255 - 128)
      new_stick_y = int(0.5 * (new_n[1] + 1) * 255 - 128)

      self.model.edits.edit(self.model.selected_frame, stick_x_var, new_stick_x)
      self.model.edits.edit(self.model.selected_frame, stick_y_var, new_stick_y)


  def render_adjusted_stick_control(self, id: str) -> None:
    stick_x_var = self.model.variables['input-stick-x']
    stick_y_var = self.model.variables['input-stick-y']

    raw_stick_x = self.model.get(stick_x_var)
    raw_stick_y = self.model.get(stick_y_var)

    stick_x, stick_y = joystick_util.raw_to_adjusted(raw_stick_x, raw_stick_y)
    new_n = ui.render_joystick_control(id, stick_x / 64, stick_y / 64, 'circle')

    if new_n is not None:
      new_stick_x = new_n[0] * 64
      new_stick_y = new_n[1] * 64

      new_raw_stick_x, new_raw_stick_y = \
        joystick_util.adjusted_to_raw(new_stick_x, new_stick_y)

      self.model.edits.edit(self.model.selected_frame, stick_x_var, new_raw_stick_x)
      self.model.edits.edit(self.model.selected_frame, stick_y_var, new_raw_stick_y)


  def render_dyaw_stick_control(self, id: str) -> None:
    stick_x_var = self.model.variables['input-stick-x']
    stick_y_var = self.model.variables['input-stick-y']
    face_yaw = self.model.get(self.model.variables['mario-face-yaw'])
    camera_yaw = self.model.get(self.model.variables['camera-yaw'])
    squish_timer = 0 # TODO

    raw_stick_x = self.model.get(stick_x_var)
    raw_stick_y = self.model.get(stick_y_var)

    int_yaw, int_mag = joystick_util.raw_to_intended(
      raw_stick_x,
      raw_stick_y,
      face_yaw,
      camera_yaw,
      squish_timer,
    )
    int_dyaw = int_yaw - face_yaw
    n_x = int_mag / 32 * math.sin(-int_dyaw * math.pi / 0x8000)
    n_y = int_mag / 32 * math.cos(int_dyaw * math.pi / 0x8000)
    new_n = ui.render_joystick_control(id, n_x, n_y, 'circle')

    if new_n is not None:
      new_int_dyaw = math.atan2(-new_n[0], new_n[1]) * 0x8000 / math.pi
      new_int_mag = 32 * math.sqrt(new_n[0]**2 + new_n[1]**2)
      new_int_yaw = face_yaw + new_int_dyaw

      new_raw_stick_x, new_raw_stick_y = joystick_util.intended_to_raw(
        new_int_yaw,
        new_int_mag,
        face_yaw,
        camera_yaw,
        squish_timer,
      )

      self.model.edits.edit(self.model.selected_frame, stick_x_var, new_raw_stick_x)
      self.model.edits.edit(self.model.selected_frame, stick_y_var, new_raw_stick_y)


  def render_input_tab(self, tab: TabId) -> None:
    ig.columns(3)
    ig.set_column_width(-1, 160)
    ig.set_column_width(-2, 160)

    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      self.render_variable(tab, variable)

    ig.next_column()
    self.render_stick_control('joystick')
    # ig.next_column()
    # self.render_adjusted_stick_control('adjusted')
    ig.next_column()
    self.render_dyaw_stick_control('intended')

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
