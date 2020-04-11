from typing import *
from dataclasses import dataclass
import math
import ctypes as C

import ext_modules.util as c_util

import wafel.imgui as ig
from wafel.model import Model
from wafel.variable import Variable
from wafel.object_type import ObjectType
from wafel.core import DataPath, Address
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
  TabId('Objects'),
]
if config.dev_mode:
  FIXED_TABS.insert(1, TabId('Scripting'))
  FIXED_TABS.insert(4, TabId('Subframe'))


class VariableExplorer:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    self.model = model
    self.formatters = formatters
    self.open_tabs: List[TabId] = []

    for tab in FIXED_TABS:
      self.open_tab(tab)

    self.current_tab = self.open_tabs[0]
    # if config.dev_mode:
    #   self.current_tab = TabId('Subframe')


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
    if tab.object is not None:
      object_type = self.model.get_object_type(self.model.selected_frame, tab.object)
      if object_type is None:
        return str(tab.object)
      else:
        return str(tab.object) + ': ' + object_type.name

    elif tab.surface is not None:
      return f'Surface {tab.surface}'

    return tab.name


  def render_objects_tab(self) -> None:
    object_types: List[Optional[ObjectType]] = []

    for slot in range(240):
      object_id = slot
      object_types.append(self.model.get_object_type(self.model.selected_frame, object_id))

    selected_slot = ui.render_object_slots('object-slots', object_types)
    if selected_slot is not None:
      object_id = selected_slot
      self.open_tab(TabId('_object', object_id))


  def get_variables_for_tab(self, tab: TabId) -> List[Variable]:
    if tab.object is not None:
      object_type = self.model.get_object_type(self.model.selected_frame, tab.object)
      if object_type is None:
        return []

      return [
        var.at(object=tab.object, object_type=object_type)
          for var in self.model.data_variables.group('Object')
            if self.model.data_variables[var].label is not None
      ]

    elif tab.surface is not None:
      num_surfaces = dcast(int, self.model.get(self.model.selected_frame, 'gSurfacesAllocated'))
      if tab.surface >= num_surfaces:
        return []

      return [
        var.at(surface=tab.surface)
          for var in self.model.data_variables.group('Surface')
            if self.model.data_variables[var].label is not None
      ]

    else:
      return self.model.data_variables.group(tab.name)


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
      self.model.edited(variable),
      label_width = label_width,
      value_width = value_width,
    )
    if changed_data is not None:
      self.model.set(variable, changed_data.value)
    if clear_edit:
      self.model.reset(variable)


  def render_stick_control(self, id: str, tab: TabId) -> None:
    stick_x_var = Variable('input-stick-x').at(frame=self.model.selected_frame)
    stick_y_var = Variable('input-stick-y').at(frame=self.model.selected_frame)

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

      self.model.edits.edit(stick_x_var, new_stick_x)
      self.model.edits.edit(stick_y_var, new_stick_y)


  def render_intended_stick_control(self, id: str) -> None:
    up_options = ['3d view', 'mario yaw', 'stick y', 'world x']
    up_option = use_state('up-option', 0)

    ig.text('up =')
    ig.same_line()
    ig.push_item_width(100)
    _, up_option.value = ig.combo('##up-option', up_option.value, up_options)
    ig.pop_item_width()
    ig.dummy(1, 10)

    stick_x_var = Variable('input-stick-x').at(frame=self.model.selected_frame)
    stick_y_var = Variable('input-stick-y').at(frame=self.model.selected_frame)

    face_yaw = dcast(int, self.model.get(Variable('mario-face-yaw').at(frame=self.model.selected_frame)))
    camera_yaw = dcast(int, self.model.get(Variable('camera-yaw').at(frame=self.model.selected_frame)) or 0)
    squish_timer = dcast(int, self.model.get(self.model.selected_frame, 'gMarioState[].squishTimer'))
    active_face_yaw = face_yaw

    events = get_frame_log(self.model.timeline, self.model.selected_frame + 1)

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

    raw_stick_x = self.model.get(stick_x_var)
    raw_stick_y = self.model.get(stick_y_var)

    adjusted = c_util.stick_raw_to_adjusted(raw_stick_x, raw_stick_y)
    intended_yaw, intended_mag = c_util.stick_adjusted_to_intended(
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

    target_mag = render_value('int mag', intended_mag, FloatFormatter())
    target_yaw = render_value('int yaw', intended_yaw, DecimalIntFormatter())
    dyaw = intended_yaw - active_face_yaw
    target_dyaw = render_value('dyaw', dyaw, DecimalIntFormatter())

    ig.same_line()
    if ig.button('?'):
      ig.open_popup('active-yaw-expl')
    if ig.begin_popup('active-yaw-expl'):
      ig.text(f'{intended_yaw} - {active_face_yaw} = {dyaw}')
      ig.text(f'intended yaw = {intended_yaw}')
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
        target_yaw = intended_yaw
      if target_mag is None:
        target_mag = intended_mag

      new_raw_stick_x, new_raw_stick_y = intended_to_raw(
        self.model.timeline[self.model.selected_frame], target_yaw, target_mag, relative_to
      )

      self.model.edits.edit(stick_x_var, new_raw_stick_x)
      self.model.edits.edit(stick_y_var, new_raw_stick_y)

    n_a = intended_yaw - up_angle
    n_x = intended_mag / 32 * math.sin(-n_a * math.pi / 0x8000)
    n_y = intended_mag / 32 * math.cos(n_a * math.pi / 0x8000)

    ig.set_cursor_pos((ig.get_cursor_pos().x + 155, 0))
    new_n = ui.render_joystick_control(id, n_x, n_y, 'circle')

    if new_n is not None:
      new_n_a = int(math.atan2(-new_n[0], new_n[1]) * 0x8000 / math.pi)
      new_intended_yaw = up_angle + new_n_a
      new_intended_mag = 32 * math.sqrt(new_n[0]**2 + new_n[1]**2)

      new_raw_stick_x, new_raw_stick_y = intended_to_raw(
        self.model.timeline[self.model.selected_frame], new_intended_yaw, new_intended_mag, relative_to=0
      )

      self.model.edits.edit(stick_x_var, new_raw_stick_x)
      self.model.edits.edit(stick_y_var, new_raw_stick_y)


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
        Variable('input-button-' + button).at(frame=self.model.selected_frame),
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


  def render_script_segments_tab(self, id: str) -> None:
    ig.push_id(id)

    scripts = self.model.scripts

    ig.dummy(1, 5)
    self.render_variable(
      self.current_tab,
      Variable('wafel-script').at(frame=self.model.selected_frame),
      value_width=200,
    )
    ig.dummy(1, 10)

    if ig.button('Split'):
      scripts.split_segment(self.model.selected_frame)
    ig.dummy(1, 5)

    frame_ranges = format_align(
      '%s{0}%a - %s{1}%a',
      [
        (seg.frame_start, '...' if seg.frame_stop is None else seg.frame_stop - 1)
          for seg in scripts.segments
      ]
    )
    frame_range_width = max(map(len, frame_ranges)) * 7

    segments = list(scripts.segments)
    for i, segment in enumerate(segments):
      ig.push_id('segment-' + str(py_id(segment)))

      clicked, _ = ig.selectable(
        frame_ranges[i] + '##frame-range',
        width = frame_range_width,
        selected = self.model.selected_frame in segment,
      )
      if clicked:
        self.model.selected_frame = segment.frame_start

      if ig.begin_popup_context_item('##context'):
        if i != 0:
          prev_source = segments[i - 1].script.source
          if not prev_source.strip():
            prev_source = 'none'
          if ig.selectable(f'Delete, use previous ({truncate_str(prev_source, 32, "...")})')[0]:
            scripts.delete_segment(segment, merge_upward=True)
        if i < len(segments) - 1:
          next_source = segments[i + 1].script.source
          if not next_source.strip():
            next_source = 'none'
          if ig.selectable(f'Delete, use next ({truncate_str(next_source, 32, "...")})')[0]:
            scripts.delete_segment(segment, merge_upward=False)
        ig.end_popup_context_item()

      ig.same_line()

      pending_source: Ref[Optional[str]] = use_state('pending-source', None)
      changed, new_source = ig.input_text(
        '##script',
        segment.script.source if pending_source.value is None else pending_source.value,
        len(segment.script.source) + ig.get_clipboard_length() + 1000,
      )

      if changed:
        pending_source.value = new_source
      if pending_source.value is not None and not ig.is_item_active():
        scripts.set_segment_source(segment, pending_source.value)
        pending_source.value = None

      ig.pop_id()

    ig.pop_id()


  def render_script_variables_tab(self, id: str) -> None:
    ig.push_id(id)

    scripts = self.model.scripts

    for variable in list(scripts.variables):
      ig.push_id('var_' + str(py_id(variable)))

      def validate_name(name: str) -> str:
        for var in scripts.variables:
          if var is not variable and var.name == name:
            raise Exception
        return name

      def validate_value(source: str) -> object:
        value = eval(source)
        if not isinstance(value, int) and not isinstance(value, float):
          raise Exception
        return value

      new_name = ui.render_input_text_with_error('##name', variable.name, 128, 150, validate_name)
      ig.same_line()
      new_value = ui.render_input_text_with_error('##value', str(variable.value), 128, 100, validate_value)

      if new_name is not None:
        scripts.set_variable_name(variable, new_name.value)
      if new_value is not None:
        scripts.set_variable_value(variable, new_value.value)

      ig.same_line()
      if ig.button('Delete'):
        scripts.delete_variable(variable)

      ig.pop_id()

    next_suffix = use_state('next-suffix', 1)
    if ig.button('New'):
      while any(var.name == 'V' + str(next_suffix.value) for var in scripts.variables):
        next_suffix.value += 1
      name = 'V' + str(next_suffix.value)
      next_suffix.value += 1
      scripts.create_variable(name, 0)

    ig.pop_id()


  def render_script_tab(self) -> None:
    ui.render_tabs(
      'tabs',
      [
        ui.TabInfo('segments', 'Scripts', False, self.render_script_segments_tab),
        ui.TabInfo('variables', 'Variables', False, self.render_script_variables_tab),
      ]
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

    events = get_frame_log(self.model.timeline, self.model.selected_frame + frame_offset.value)

    def string(addr: object) -> str:
      abs_addr = dcast(Address, addr).absolute
      return C.string_at(abs_addr).decode('utf-8')

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
        from_action = self.model.action_names[event['from']]
        to_action = self.model.action_names[event['to']]
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
      self.render_variable(tab, variable.at(frame=self.model.selected_frame))


  def render_tab_contents(self, id: str, tab: TabId) -> None:
    ig.push_id(id)
    if tab.name == 'Objects':
      self.render_objects_tab()
    elif tab.name == 'Input':
      self.render_input_tab(tab)
    elif tab.name == 'Scripting':
      self.render_script_tab()
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
