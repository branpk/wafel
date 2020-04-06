from typing import *
from dataclasses import dataclass
import time

import glfw

import wafel.imgui as ig
from wafel.local_state import use_state
from wafel.util import *
import wafel.config as config


KEY_NAMES: Dict[int, str] = {
  getattr(glfw, var):
    var[len('KEY_'):]
      .replace('RIGHT_', 'R_')
      .replace('LEFT_', 'L_')
      .replace('MULTIPLY', 'MUL')
      .replace('SUBTRACT', 'SUB')
      .replace('GRAVE_ACCENT', 'ACCENT')
      .replace('SCROLL_LOCK', 'SCR_LOCK')
      .replace('APOSTROPHE', 'QUOTE')
      .replace('DECIMAL', 'DEC')
      .replace('PRINT_SCREEN', 'PRINT_SCR')
    for var in dir(glfw)
      if var.startswith('KEY_')
}


@dataclass(frozen=True)
class JoystickButton:
  joystick: str
  index: int

  def __str__(self) -> str:
    return f'Button {self.index + 1}'

@dataclass(frozen=True)
class JoystickAxis:
  joystick: str
  index: int
  direction: int

  def __str__(self) -> str:
    sign = '+' if self.direction > 0 else '-'
    return f'Axis {self.index + 1}{sign}'

@dataclass(frozen=True)
class KeyboardKey:
  key: int

  def __str__(self) -> str:
    return KEY_NAMES[self.key]

Input = Union[JoystickButton, JoystickAxis, KeyboardKey]

def input_to_json(input: Input) -> object:
  type_ = {
    JoystickButton: 'button',
    JoystickAxis: 'axis',
    KeyboardKey: 'key',
  }[type(input)]
  return dict(type=type_, **input.__dict__)

def input_from_json(json: object) -> Input:
  assert isinstance(json, dict)
  type_ = {
    'button': JoystickButton,
    'axis': JoystickAxis,
    'key': KeyboardKey,
  }[json['type']]
  args = dict(**json)
  del args['type']
  return cast(Input, type_(**args))


Bindings = Dict[str, Input]

def bindings_to_json(bindings: Bindings) -> object:
  return {
    control: input_to_json(input)
      for control, input in bindings.items()
  }

def bindings_from_json(json: object) -> Bindings:
  assert isinstance(json, dict)
  return {
    control: input_from_json(input)
      for control, input in json.items()
  }


def get_joysticks_impl() -> List[Tuple[int, str]]:
  joysticks = [glfw.JOYSTICK_1 + i for i in range(16)]
  joysticks = [j for j in joysticks if glfw.joystick_present(j)]
  return [(j, glfw.get_joystick_guid(j).decode('utf-8')) for j in joysticks]


def log_joystick_info(joysticks: List[Tuple[int, str]]) -> None:
  message = 'Detected joysticks:\n'
  for joystick, id in joysticks:
    message += f'  {joystick}: {glfw.get_joystick_name(joystick).decode("utf-8")} ({id})\n'
  log.info(message.strip())


_prev_joysticks: Optional[List[Tuple[int, str]]] = None

def get_joysticks() -> List[Tuple[int, str]]:
  global _prev_joysticks
  joysticks = get_joysticks_impl()
  if joysticks != _prev_joysticks:
    log_joystick_info(joysticks)
  _prev_joysticks = joysticks
  return joysticks


def get_joysticks_by_id(joystick_id: str) -> List[int]:
  return [index for index, id in get_joysticks() if id == joystick_id]


def detect_input() -> Optional[Input]:
  for key in KEY_NAMES:
    if ig.is_key_down(key):
      return KeyboardKey(key)

  for joystick_index, joystick_id in get_joysticks():
    axes_ptr, length = glfw.get_joystick_axes(joystick_index)
    axes = tuple(axes_ptr[i] for i in range(length))

    if len(axes) > 0:
      max_axis = max(range(len(axes)), key=lambda i: abs(axes[i]))
      if abs(axes[max_axis]) > 0.5:
        return JoystickAxis(joystick_id, max_axis, 1 if axes[max_axis] > 0 else -1)

    buttons_ptr, length = glfw.get_joystick_buttons(joystick_index)
    buttons = tuple(buttons_ptr[i] for i in range(length))

    for i, pressed in enumerate(buttons):
      if pressed:
        return JoystickButton(joystick_id, i)

  return None


def check_input(input: Optional[Input]) -> float:
  if input is None:
    return 0.0

  elif isinstance(input, JoystickAxis):
    for joystick in get_joysticks_by_id(input.joystick):
      axes_ptr, length = glfw.get_joystick_axes(joystick)
      if input.index < length:
        axis_value = float(axes_ptr[input.index])
        axis_value = min(max(axis_value * input.direction, 0.0), 1.0)
        if axis_value < 0.001: # TODO: Allow dead zone configuration
          axis_value = 0.0
        if axis_value > 0.0:
          return axis_value
    return 0.0

  elif isinstance(input, JoystickButton):
    for joystick in get_joysticks_by_id(input.joystick):
      buttons_ptr, length = glfw.get_joystick_buttons(joystick)
      if input.index < length and buttons_ptr[input.index]:
        return 1.0
    return 0.0

  elif isinstance(input, KeyboardKey):
    return 1.0 if ig.is_key_down(input.key) else 0.0


DEFAULT_BINDINGS: Bindings = {
  'n64-^': KeyboardKey(glfw.KEY_I),
  'n64-<': KeyboardKey(glfw.KEY_J),
  'n64->': KeyboardKey(glfw.KEY_L),
  'n64-v': KeyboardKey(glfw.KEY_K),
  'n64-A': KeyboardKey(glfw.KEY_Z),
  'n64-B': KeyboardKey(glfw.KEY_X),
  'n64-Z': KeyboardKey(glfw.KEY_C),
  'frame-prev-fast': KeyboardKey(glfw.KEY_PAGE_UP),
  'frame-prev': KeyboardKey(glfw.KEY_UP),
  'frame-prev-alt': KeyboardKey(glfw.KEY_LEFT),
  'frame-next': KeyboardKey(glfw.KEY_DOWN),
  'frame-next-alt': KeyboardKey(glfw.KEY_RIGHT),
  'frame-next-fast': KeyboardKey(glfw.KEY_PAGE_DOWN),
  '3d-camera-move-f': KeyboardKey(glfw.KEY_W),
  '3d-camera-move-b': KeyboardKey(glfw.KEY_S),
  '3d-camera-move-l': KeyboardKey(glfw.KEY_A),
  '3d-camera-move-r': KeyboardKey(glfw.KEY_D),
  '3d-camera-move-u': KeyboardKey(glfw.KEY_SPACE),
  '3d-camera-move-d': KeyboardKey(glfw.KEY_LEFT_SHIFT),
  'playback-play': KeyboardKey(glfw.KEY_ENTER),
  'playback-rewind': KeyboardKey(glfw.KEY_APOSTROPHE),
  'playback-speed-up': KeyboardKey(glfw.KEY_EQUAL),
  'playback-slow-down': KeyboardKey(glfw.KEY_MINUS),
}

bindings: Bindings


def init() -> None:
  global bindings
  bindings = bindings_from_json(
    config.settings.get('bindings') or bindings_to_json(DEFAULT_BINDINGS)
  )


def find_overlapping_bindings() -> Dict[Input, List[str]]:
  input_to_controls: Dict[Input, List[str]] = {}
  for control, input in bindings.items():
    input_to_controls.setdefault(input, []).append(control)
  return {
    input: controls
      for input, controls in input_to_controls.items()
        if len(controls) > 1
  }


def begin_binding_form() -> None:
  controls: Ref[List[str]] = use_state('controls', [])
  listening_for: Ref[Optional[str]] = use_state('listening-for', None)
  waiting_for_release: Ref[bool] = use_state('waiting-for-release', False)

  if ig.is_mouse_clicked():
    listening_for.value = None

  if listening_for.value is not None:
    input = detect_input()

    if input is not None and not waiting_for_release.value:
      bindings[listening_for.value] = input
      config.settings['bindings'] = bindings_to_json(bindings)

      if listening_for.value in controls.value:
        index = controls.value.index(listening_for.value) + 1
        listening_for.value = controls.value[index] if index < len(controls.value) else None
        waiting_for_release.value = True
      else:
        listening_for.value = None

    if input is None:
      waiting_for_release.value = False

  controls.value = []


def binding_button(name: str, label: str, width=0) -> None:
  listening_for: Ref[Optional[str]] = use_state('listening-for', None)

  controls: Ref[List[str]] = use_state('controls', [])
  controls.value.append(name)

  value = check_input(bindings.get(name))
  color = (0.26 + value * 0.7, 0.59 + value * 0.41, 0.98, 0.4)
  if listening_for.value == name:
    color = (0.86, 0.59, 0.98, 0.4)

  ig.push_style_color(ig.COLOR_BUTTON, *color)
  if ig.button(label, width=width):
    listening_for.value = name
  ig.pop_style_color()

  if ig.begin_popup_context_item('##btn-ctx-' + name):
    if ig.menu_item('Clear')[0] and name in bindings:
      del bindings[name]
    if ig.menu_item('Default')[0] and name in DEFAULT_BINDINGS:
      bindings[name] = DEFAULT_BINDINGS[name]
    ig.end_popup_context_item()

  ig.same_line()
  if name in bindings:
    text = str(bindings[name])
  else:
    text = 'Unbound'
  if listening_for.value == name:
    text = '(' + text + ')'

  if name in bindings:
    overlapping = find_overlapping_bindings().get(bindings[name])
  else:
    overlapping = None

  if overlapping is None:
    ig.text(text)
  else:
    ig.text_colored(text, 1.0, 0.8, 0.0, 1.0)
    if ig.is_item_hovered():
      def control_name(name: str) -> str:
        return name.replace('-', ' ').replace('n64', 'N64')
      overlapping = [c for c in overlapping if c != name]
      ig.set_tooltip('Also bound to ' + ', '.join(map(control_name, overlapping)))


def render_controller_settings(id: str) -> None:
  ig.push_id(id)
  begin_binding_form()

  def cardinal(prefix: str) -> None:
    ig.dummy(45, 1)
    ig.same_line()
    binding_button('n64-' + prefix + '^', prefix + '^')
    binding_button('n64-' + prefix + '<', prefix + '<')
    ig.same_line()
    ig.set_cursor_pos((110, ig.get_cursor_pos().y))
    binding_button('n64-' + prefix + '>', prefix + '>')
    ig.dummy(45, 1)
    ig.same_line()
    binding_button('n64-' + prefix + 'v', prefix + 'v')

  binding_button('n64-A', 'A')
  binding_button('n64-B', 'B')
  binding_button('n64-Z', 'Z')

  ig.dummy(1, 5)
  binding_button('n64-L', 'L')
  binding_button('n64-R', 'R')
  binding_button('n64-S', 'S')

  ig.dummy(1, 5)
  cardinal('')

  ig.dummy(1, 15)
  cardinal('C')

  ig.dummy(1, 15)
  cardinal('D')

  ig.dummy(200, 1)
  ig.pop_id()


def render_key_binding_settings(id: str) -> None:
  ig.push_id(id)
  begin_binding_form()

  ig.text('Frame advance:')
  button_width = 0
  binding_button('frame-prev-fast', '-5', button_width)
  binding_button('frame-prev', '-1', button_width)
  binding_button('frame-prev-alt', '-1', button_width)
  binding_button('frame-next', '+1', button_width)
  binding_button('frame-next-alt', '+1', button_width)
  binding_button('frame-next-fast', '+5', button_width)

  ig.dummy(1, 5)

  ig.text('3D movement:')
  button_width = 70
  binding_button('3d-camera-move-f', 'Forward', button_width)
  binding_button('3d-camera-move-b', 'Back', button_width)
  binding_button('3d-camera-move-l', 'Left', button_width)
  binding_button('3d-camera-move-r', 'Right', button_width)
  binding_button('3d-camera-move-u', 'Up', button_width)
  binding_button('3d-camera-move-d', 'Down', button_width)

  ig.dummy(1, 5)

  ig.text('Playback:')
  button_width = 90
  binding_button('playback-play', 'Play', button_width)
  binding_button('playback-rewind', 'Rewind', button_width)
  binding_button('playback-speed-up', 'Speed up', button_width)
  binding_button('playback-slow-down', 'Slow down', button_width)

  ig.dummy(200, 1)
  ig.pop_id()


def input_float(name: str) -> float:
  if not ig.global_keyboard_capture():
    return 0.0
  return check_input(bindings.get(name))

def input_down(name: str) -> bool:
  return bool(round(input_float(name)))

def input_pressed(name: str) -> bool:
  ig.push_id('ctrl-pressed-' + name)
  prev_down = use_state('prev-down', False)
  down = input_down(name)
  pressed = not prev_down.value and down
  prev_down.value = down
  ig.pop_id()
  return pressed

def input_pressed_repeat(name: str, delay: float, per_second: float) -> int:
  ig.push_id('ctrl-pressed-repeat-' + name)
  down_start: Ref[Optional[float]] = use_state('down-start', None)
  counted = use_state('counted', 0)

  down = input_down(name)
  if not down:
    down_start.value = None
    count = 0
  elif down_start.value is None:
    down_start.value = time.time()
    counted.value = 0
    count = 1
  else:
    held = time.time() - down_start.value
    total_count = int(per_second * max(held - delay, 0))
    count = total_count - counted.value
    counted.value = total_count

  ig.pop_id()
  return count


__all__ = [
  'render_controller_settings',
  'render_key_binding_settings',
  'input_float',
  'input_down',
  'input_pressed',
  'input_pressed_repeat',
]
