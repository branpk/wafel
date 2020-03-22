from typing import *
from dataclasses import dataclass
import math

import wafel.imgui as ig
from wafel.model import Model
from wafel.core import ObjectId, Variable, VariableGroup, ObjectType, VariableId, DataPath, AbsoluteAddr, RelativeAddr
from wafel.variable_format import Formatters, VariableFormatter
import wafel.ui as ui
from wafel.util import *
import wafel.joystick_util as joystick_util
import wafel.config as config
from wafel.local_state import use_state


@dataclass(frozen=True)
class TabId:
  name: str
  object_id: Optional[ObjectId] = None
  surface: Optional[int] = None


FIXED_TABS = [
  TabId('Input'),
  TabId('Mario'),
  TabId('Misc'),
  TabId('Objects'),
]
if config.dev_mode:
  FIXED_TABS.insert(3, TabId('Subframe'))


class VariableExplorer:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    self.model = model
    self.formatters = formatters
    self.open_tabs: List[TabId] = []

    for tab in FIXED_TABS:
      self.open_tab(tab)

    self.current_tab = self.open_tabs[0]
    if config.dev_mode:
      self.current_tab = TabId('Subframe')


  def open_tab(self, tab: TabId) -> None:
    if tab not in self.open_tabs:
      self.open_tabs.append(tab)
    self.current_tab = tab


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


  def get_event_variant(self, event_type: str) -> str:
    variant = event_type.lower()
    if variant.startswith('flt_'):
      variant = variant[len('flt_'):]
    parts = variant.split('_')
    variant = parts[0] + ''.join(map(str.capitalize, parts[1:]))
    return variant


  def get_frame_log_events(self, frame: int) -> List[Dict[str, Any]]:
    # TODO: Move/cache event_types
    event_types: Dict[int, str] = {
      constant['value']: constant_name
        for constant_name, constant in self.model.lib.spec['constants'].items()
          if constant['source'] == 'enum' and constant['enum_name'] == 'FrameLogEventType'
    }

    with self.model.timeline[frame] as state:
      events: List[Dict[str, object]] = []

      log_length = dcast(int, DataPath.compile(self.model.lib, '$state.gFrameLogLength').get(state))
      for i in range(log_length):
        event_type_value = dcast(int, DataPath.compile(self.model.lib, f'$state.gFrameLog[{i}].type').get(state))
        event_type = event_types[event_type_value]
        variant_name = self.get_event_variant(event_type)
        event_data = dcast(dict, DataPath.compile(self.model.lib, f'$state.gFrameLog[{i}].__anon.{variant_name}').get(state))

        event: Dict[str, object] = { 'type': event_type }
        event.update(event_data)
        events.append(event)

    return events


  # TODO: Move/cache
  def mario_action_name(self, action: int) -> str:
    for constant_name, constant in self.model.lib.spec['constants'].items():
      if constant['value'] == action and constant_name.startswith('ACT_'):
        assert isinstance(constant_name, str)
        return constant_name.lower()[len('act_'):].replace('_', ' ')
    assert False, '0x%08X' % action


  def render_frame_log_tab(self) -> None:
    frame_offset = use_state('frame-offset', 1)
    round_numbers = use_state('round-numbers', True)

    ig.push_item_width(210)
    _, frame_offset.value = ig.combo(
      '##frame-offset',
      frame_offset.value,
      ['previous -> current frame', 'current -> next frame'],
    )
    ig.pop_item_width()
    _, round_numbers.value = ig.checkbox('Round##round-numbers', round_numbers.value)
    ig.dummy(1, 10)

    events = self.get_frame_log_events(self.model.selected_frame + frame_offset.value)

    def string(addr: object) -> str:
      abs_addr = dcast(AbsoluteAddr, dcast(RelativeAddr, addr).value)
      return self.model.lib.string(abs_addr)

    def round(number: object) -> str:
      assert isinstance(number, float)
      if round_numbers.value:
        return '%.3f' % number
      else:
        return str(number)

    indent = 0

    def show_text(text: str) -> None:
      ig.text('    ' * indent + text)

    for event in events:
      if event['type'] == 'FLT_CHANGE_ACTION':
        from_action = self.mario_action_name(event['from'])
        to_action = self.mario_action_name(event['to'])
        show_text(f'change action: {from_action} -> {to_action}')
      elif event['type'] == 'FLT_CHANGE_FORWARD_VEL':
        show_text(f'change f vel: {round(event["from"])} -> {round(event["to"])} ({string(event["reason"])})')
      elif event['type'] == 'FLT_WALL_PUSH':
        from_pos = ', '.join(map(round, event['from']))
        to_pos = ', '.join(map(round, event['to']))
        show_text(f'wall push: ({from_pos}) -> ({to_pos}) (surface {event["surface"]})')
      elif event['type'] == 'FLT_BEGIN_MOVEMENT_STEP':
        type_ = { 1: 'air', 2: 'ground', 3: 'water' }[event['stepType']]
        show_text(f'{type_} step {event["stepNum"]}:')
        indent += 1
      elif event['type'] == 'FLT_END_MOVEMENT_STEP':
        indent -= 1
      else:
        show_text(str(event))


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
    elif tab.name == 'Subframe':
      self.render_frame_log_tab()
    else:
      self.render_variable_tab(tab)
    ig.pop_id()


  def render(self, id: str) -> None:
    ig.push_id(id)

    def render_tab(tab: TabId) -> Callable[[str], None]:
      return lambda id: self.render_tab_contents(id, tab)

    open_tab_index = None
    if self.current_tab in self.open_tabs:
      open_tab_index = self.open_tabs.index(self.current_tab)

    open_tab, closed_tab = ui.render_tabs(
      'tabs',
      [
        ui.TabInfo(
          id = f'tab-{hash(tab)}',
          label = self.get_tab_label(tab),
          closable = tab not in FIXED_TABS,
          render = render_tab(tab),
        )
          for tab in self.open_tabs
      ],
      open_tab_index,
    )
    if open_tab is not None:
      self.current_tab = self.open_tabs[open_tab]
    if closed_tab is not None:
      del self.open_tabs[closed_tab]

    ig.pop_id()
