from typing import *
from dataclasses import dataclass
import math
import ctypes as C

from wafel_core import Variable, ObjectBehavior, Address, stick_raw_to_adjusted, \
  stick_adjusted_to_intended

import wafel.imgui as ig
from wafel.model import Model
from wafel.variable_format import Formatters, VariableFormatter, DecimalIntFormatter, FloatFormatter, EmptyFormatter
import wafel.ui as ui
from wafel.util import *
import wafel.config as config
from wafel.local_state import use_state
from wafel.sm64_util import *


@dataclass(frozen=True)
class TabId:
  name: str
  object: Optional[int] = None
  surface: Optional[int] = None

  @property
  def id(self) -> str:
    return 'tab-' + '-'.join(map(str, self.__dict__.values()))


FIXED_TABS = [
  TabId('Input'),
  TabId('Mario'),
  TabId('Misc'),
  TabId('Viz (WIP)'),
  TabId('Subframe'),
  TabId('Objects'),
]


class VariableExplorer:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    self.model = model
    self.formatters = formatters
    self.open_tabs: List[TabId] = []

    for tab in FIXED_TABS:
      self.open_tab(tab)

    self.current_tab = self.open_tabs[0]

  def open_tab(self, tab: TabId) -> None:
    if tab not in self.open_tabs:
      self.open_tabs.append(tab)
    self.current_tab = tab

  def open_object_tab(self, object: int) -> None:
    self.open_tab(TabId('_object', object=object))

  def open_surface_tab(self, surface: int) -> None:
    self.open_tab(TabId('_surface', surface=surface))

  def close_tab(self, tab: TabId) -> None:
    if tab in self.open_tabs:
      self.open_tabs.remove(tab)

  def get_tab_label(self, tab: TabId) -> str:
    if tab.object is not None:
      behavior = self.model.get_object_behavior(self.model.selected_frame, tab.object)
      if behavior is None:
        return str(tab.object)
      else:
        return str(tab.object) + ': ' + self.model.pipeline.object_behavior_name(behavior)

    elif tab.surface is not None:
      return f'Surface {tab.surface}'

    return tab.name

  def render_objects_tab(self) -> None:
    behaviors: List[Optional[ObjectBehavior]] = []

    for slot in range(240):
      behaviors.append(self.model.get_object_behavior(self.model.selected_frame, slot))

    selected_slot = ui.render_object_slots(
      'object-slots',
      behaviors,
      self.model.pipeline.object_behavior_name,
    )
    if selected_slot is not None:
      self.open_object_tab(selected_slot)

  def get_variables_for_tab(self, tab: TabId) -> List[Variable]:
    if tab.object is not None:
      behavior = self.model.get_object_behavior(self.model.selected_frame, tab.object)
      if behavior is None:
        return []

      return [
        var.with_object(tab.object).with_object_behavior(behavior)
          for var in self.model.pipeline.variable_group('Object')
            if self.model.pipeline.label(var) is not None
      ]

    elif tab.surface is not None:
      num_surfaces = dcast(int, self.model.get(self.model.selected_frame, 'gSurfacesAllocated'))
      if tab.surface >= num_surfaces:
        return []

      return [
        var.with_surface(tab.surface)
          for var in self.model.pipeline.variable_group('Surface')
            if self.model.pipeline.label(var) is not None
      ]

    else:
      return self.model.pipeline.variable_group(tab.name)

  def render_variable(
    self,
    tab: TabId,
    variable: Variable,
    label_width = 80,
    value_width = 80,
  ) -> None:
    value = self.model.get(variable)

    changed_data, clear_edit = ui.render_labeled_variable(
      f'var-{hash((tab, variable))}',
      self.model.label(variable),
      variable,
      value,
      EmptyFormatter() if value is None else self.formatters[variable],
      False,
      label_width = label_width,
      value_width = value_width,
    )
    if changed_data is not None:
      self.model.set(variable, changed_data.value)
    if clear_edit:
      self.model.reset(variable)

  def render_stick_control(self, id: str, tab: TabId) -> None:
    stick_x_var = Variable('input-stick-x').with_frame(self.model.selected_frame)
    stick_y_var = Variable('input-stick-y').with_frame(self.model.selected_frame)

    self.render_variable(tab, stick_x_var, 60, 50)
    self.render_variable(tab, stick_y_var, 60, 50)

    stick_x = dcast(int, self.model.get(stick_x_var))
    stick_y = dcast(int, self.model.get(stick_y_var))

    n_x = 2 * ((stick_x + 128) / 255) - 1
    n_y = 2 * ((stick_y + 128) / 255) - 1
    new_n = ui.render_joystick_control(id, n_x, n_y)

    if new_n is not None:
      new_stick_x = int(0.5 * (new_n[0] + 1) * 255 - 128)
      new_stick_y = int(0.5 * (new_n[1] + 1) * 255 - 128)

      self.model.set(stick_x_var, new_stick_x)
      self.model.set(stick_y_var, new_stick_y)

  def render_intended_stick_control(self, id: str) -> None:
    up_options = ['3d view', 'mario yaw', 'stick y', 'world x']
    up_option = use_state('up-option', 0)

    ig.text('up =')
    ig.same_line()
    ig.push_item_width(100)
    _, up_option.value = ig.combo('##up-option', up_option.value, up_options)
    ig.pop_item_width()
    ig.dummy(1, 10)

    stick_x_var = Variable('input-stick-x').with_frame(self.model.selected_frame)
    stick_y_var = Variable('input-stick-y').with_frame(self.model.selected_frame)

    face_yaw = dcast(int, self.model.get(Variable('mario-face-yaw').with_frame(self.model.selected_frame)))
    camera_yaw = dcast(int, self.model.get(Variable('camera-yaw').with_frame(self.model.selected_frame)) or 0)
    squish_timer = dcast(int, self.model.get(self.model.selected_frame, 'gMarioState->squishTimer'))
    active_face_yaw = face_yaw

    events = self.model.pipeline.frame_log(self.model.selected_frame + 1)

    active_face_yaw_action = None
    for event in events:
      if event['type'] == 'FLT_EXECUTE_ACTION':
        action_name = self.model.action_names[event['action']]
        active_face_yaw = event['faceAngle'][1]
        active_face_yaw_action = action_name
        if action_name == 'idle':
          break

    up_angle = {
      'mario yaw': active_face_yaw,
      'stick y': camera_yaw + 0x8000,
      'world x': 0x4000,
      '3d view': self.model.rotational_camera_yaw,
    }[up_options[up_option.value]]
    self.model.input_up_yaw = up_angle

    raw_stick_x = dcast(int, self.model.get(stick_x_var))
    raw_stick_y = dcast(int, self.model.get(stick_y_var))

    adjusted = stick_raw_to_adjusted(raw_stick_x, raw_stick_y)
    intended = stick_adjusted_to_intended(
      adjusted,
      face_yaw,
      camera_yaw,
      squish_timer != 0,
    )

    def render_value(label: str, value: object, formatter: VariableFormatter) -> Optional[Any]:
      label_width = 60
      value_size = (
        60 if label == 'dyaw' else 80,
        ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1],
      )
      ig.push_item_width(label_width)
      ig.selectable(label, width=label_width)
      ig.pop_item_width()
      ig.same_line()
      new_value, _, _ = ui.render_variable_value(
        'value-' + label, value, formatter, value_size
      )
      return None if new_value is None else new_value.value

    target_yaw: Optional[int] = None
    target_dyaw: Optional[int] = None
    target_mag: Optional[float] = None

    target_mag = render_value('int mag', intended.mag, FloatFormatter())
    target_yaw = render_value('int yaw', intended.yaw, DecimalIntFormatter())
    dyaw = intended.yaw - active_face_yaw
    target_dyaw = render_value('dyaw', dyaw, DecimalIntFormatter())

    ig.same_line()
    if ig.button('?'):
      ig.open_popup('active-yaw-expl')
    if ig.begin_popup('active-yaw-expl'):
      ig.text(f'{intended.yaw} - {active_face_yaw} = {dyaw}')
      ig.text(f'intended yaw = {intended.yaw}')
      if active_face_yaw == face_yaw:
        ig.text(f'face yaw = {face_yaw}')
      if active_face_yaw != face_yaw:
        ig.text(f'face yaw = {active_face_yaw} at start of {active_face_yaw_action} action')
        ig.text(f'(face yaw = {face_yaw} at start of frame)')
      ig.end_popup()

    if dyaw not in range(0, 16):
      if ig.button('dyaw = 0'):
        target_dyaw = 0

    if target_yaw is not None or target_dyaw is not None or target_mag is not None:
      relative_to = 0 if target_yaw is not None else active_face_yaw
      if target_dyaw is not None:
        target_yaw = active_face_yaw + target_dyaw
      if target_yaw is None:
        target_yaw = intended.yaw
      if target_mag is None:
        target_mag = intended.mag

      new_raw_stick_x, new_raw_stick_y = intended_to_raw(
        face_yaw, camera_yaw, squish_timer, target_yaw, target_mag, relative_to
      )

      self.model.set(stick_x_var, new_raw_stick_x)
      self.model.set(stick_y_var, new_raw_stick_y)

    n_a = intended.yaw - up_angle
    n_x = intended.mag / 32 * math.sin(-n_a * math.pi / 0x8000)
    n_y = intended.mag / 32 * math.cos(n_a * math.pi / 0x8000)

    ig.set_cursor_pos((ig.get_cursor_pos().x + 155, 0))
    new_n = ui.render_joystick_control(id, n_x, n_y, 'circle')

    if new_n is not None:
      new_n_a = int(math.atan2(-new_n[0], new_n[1]) * 0x8000 / math.pi)
      new_intended_yaw = up_angle + new_n_a
      new_intended_mag = 32 * math.sqrt(new_n[0]**2 + new_n[1]**2)

      new_raw_stick_x, new_raw_stick_y = intended_to_raw(
        face_yaw, camera_yaw, squish_timer, new_intended_yaw, new_intended_mag, relative_to=0
      )

      self.model.set(stick_x_var, new_raw_stick_x)
      self.model.set(stick_y_var, new_raw_stick_y)

  def render_input_tab(self, tab: TabId) -> None:
    column_sizes = [170, 370, 200]

    ig.set_next_window_content_size(sum(column_sizes), 0)
    ig.begin_child('##input', flags=ig.WINDOW_HORIZONTAL_SCROLLING_BAR)
    ig.columns(3)

    for i, w in enumerate(column_sizes):
      ig.set_column_width(i, w)

    def render_button(button: str) -> None:
      self.render_variable(
        tab,
        Variable('input-button-' + button).with_frame(self.model.selected_frame),
        10,
        25,
      )
    ig.dummy(1, 3)
    render_button('a'); ig.same_line(); render_button('b'); ig.same_line(); render_button('z')
    ig.dummy(1, 5)
    render_button('s'); ig.same_line(); ig.dummy(43, 1); ig.same_line(); render_button('r')
    ig.dummy(1, 5)
    ig.dummy(43, 1); ig.same_line(); render_button('cu')
    ig.dummy(17, 1); ig.same_line(); render_button('cl'); ig.same_line(); render_button('cr')
    ig.dummy(43, 1); ig.same_line(); render_button('cd')

    ig.next_column()
    self.render_intended_stick_control('intended')
    ig.next_column()
    self.render_stick_control('joystick', tab)

    ig.columns(1)
    ig.end_child()

  def render_viz_tab(self) -> None:
    _, self.model.viz_enabled = ig.checkbox('Enabled', self.model.viz_enabled)
    ig.separator()
    if self.model.viz_enabled:
      config = self.model.viz_config
      label_width = 80
      value_width = 100

      def dropdown(label, options, option_labels, value):
        ig.push_item_width(label_width)
        ig.selectable(label, width=label_width)
        ig.pop_item_width()
        ig.same_line()

        ig.push_item_width(value_width)
        index = options.index(value)
        _, index = ig.combo(f'##{label}', index, option_labels)
        ig.pop_item_width()

        return options[index]

      config['object_cull'] = dropdown(
        'Object cull',
        ['Normal', 'ShowAll'],
        ['normal', 'show all'],
        config['object_cull'],
      )
      config['surface_mode'] = dropdown(
        'Surfaces',
        ['Visual', 'Physical', 'None'],
        ['visual', 'physical', 'none'],
        config['surface_mode'],
      )

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

    events = self.model.pipeline.frame_log(self.model.selected_frame + frame_offset.value)

    def string(addr: object) -> str:
      return self.model.pipeline.read_string(0, dcast(Address, addr)).decode('utf-8')

    def f32(number: object) -> str:
      assert isinstance(number, float)
      if round_numbers.value:
        return '%.3f' % number
      else:
        return str(number)

    def vec3f(vector: object) -> str:
      return '(' + ', '.join(map(f32, dcast(list, vector))) + ')'

    def action(action: object) -> str:
      return self.model.action_names[dcast(int, action)]

    indent = 0
    action_indent = 0

    def show_text(text: str) -> None:
      ig.text('    ' * indent + text)

    for event in events:
      if event['type'] == 'FLT_CHANGE_ACTION':
        show_text(f'change action: {action(event["from"])} -> {action(event["to"])}')
      elif event['type'] == 'FLT_CHANGE_FORWARD_VEL':
        show_text(f'change f vel: {f32(event["from"])} -> {f32(event["to"])} ({string(event["reason"])})')
      elif event['type'] == 'FLT_WALL_PUSH':
        show_text(f'wall push: {vec3f(event["from"])} -> {vec3f(event["to"])} (surface {event["surface"]})')
      elif event['type'] == 'FLT_BEGIN_MOVEMENT_STEP':
        type_ = { 1: 'air', 2: 'ground', 3: 'water' }[event['stepType']]
        show_text(f'{type_} step {event["stepNum"]}:')
        indent += 1
      elif event['type'] == 'FLT_END_MOVEMENT_STEP':
        indent -= 1
      elif event['type'] == 'FLT_EXECUTE_ACTION':
        indent -= action_indent
        action_indent = 0
        show_text(f'execute action: {action(event["action"])}')
        indent += 1
        action_indent += 1
      else:
        sorted_event = { 'type': event['type'] }
        sorted_event.update(sorted(event.items()))
        show_text(str(sorted_event))

  def render_variable_tab(self, tab: TabId) -> None:
    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      self.render_variable(tab, variable.with_frame(self.model.selected_frame))

  def render_tab_contents(self, id: str, tab: TabId) -> None:
    ig.push_id(id)
    if tab.name == 'Objects':
      self.render_objects_tab()
    elif tab.name == 'Input':
      self.render_input_tab(tab)
    elif tab.name == 'Viz (WIP)':
      self.render_viz_tab()
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
          id = tab.id,
          label = self.get_tab_label(tab),
          closable = tab not in FIXED_TABS,
          render = render_tab(tab),
        )
          for tab in self.open_tabs
      ],
      open_tab_index,
      allow_windowing = True,
    )
    if open_tab is not None:
      self.current_tab = self.open_tabs[open_tab]
    if closed_tab is not None:
      del self.open_tabs[closed_tab]

    ig.pop_id()
