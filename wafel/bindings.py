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
      .replace('RIGHT', 'R')
      .replace('LEFT', 'L')
      .replace('MULTIPLY', 'MUL')
      .replace('SUBTRACT', 'SUB')
      .replace('GRAVE_ACCENT', 'ACCENT')
      .replace('SCROLL_LOCK', 'SCR_LOCK')
      .replace('APOSTROPHE', 'APOS')
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


def get_joysticks() -> List[Tuple[int, str]]:
  joysticks = [glfw.JOYSTICK_1 + i for i in range(16)]
  joysticks = [j for j in joysticks if glfw.joystick_present(j)]
  return [(j, glfw.get_joystick_guid(j).decode('utf-8')) for j in joysticks]


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
      axes = tuple(axes_ptr[i] for i in range(length))
      if input.index < len(axes):
        axis_value = float(axes[input.index])
        axis_value = min(max(axis_value * input.direction, 0.0), 1.0)
        if axis_value < 0.001: # TODO: Allow dead zone configuration
          axis_value = 0.0
        if axis_value > 0.0:
          return axis_value
    return 0.0

  elif isinstance(input, JoystickButton):
    for joystick in get_joysticks_by_id(input.joystick):
      buttons_ptr, length = glfw.get_joystick_buttons(joystick)
      buttons = tuple(buttons_ptr[i] for i in range(length))
      if input.index < len(buttons) and buttons[input.index]:
        return 1.0
    return 0.0

  elif isinstance(input, KeyboardKey):
    return 1.0 if ig.is_key_down(input.key) else 0.0


bindings = bindings_from_json(config.settings.get('bindings') or {})


def render_binding_settings(id: str) -> None:
  ig.push_id(id)

  n64_controls = [
    'n64-' + label for label in [
      'A', 'B', 'S',
      'Z', 'L', 'R',
      '^', '<', '>', 'v',
      'C^', 'C<', 'C>', 'Cv',
      'D^', 'D<', 'D>', 'Dv',
    ]
  ]

  listening_for: Ref[Optional[str]] = use_state('listening-for', None)
  waiting_for_release: Ref[bool] = use_state('waiting-for-release', False)

  if listening_for.value is not None:
    input = detect_input()

    if input is not None and not waiting_for_release.value:
      bindings[listening_for.value] = input
      config.settings['bindings'] = bindings_to_json(bindings)

      index = n64_controls.index(listening_for.value) + 1
      listening_for.value = n64_controls[index] if index < len(n64_controls) else None
      waiting_for_release.value = True

    elif ig.is_key_pressed(ig.get_key_index(ig.KEY_ESCAPE)):
      listening_for.value = None

    if input is None:
      waiting_for_release.value = False

  def button(label: str) -> None:
    name = 'n64-' + label

    value = check_input(bindings.get(name))
    color = (0.26 + value * 0.7, 0.59 + value * 0.41, 0.98, 0.4)
    if listening_for.value == name:
      color = (0.86, 0.59, 0.98, 0.4)

    ig.push_style_color(ig.COLOR_BUTTON, *color)
    if ig.button(label):
      listening_for.value = name
    ig.pop_style_color()

    ig.same_line()
    if name in bindings:
      text = str(bindings[name])
    else:
      text = 'Unbound'
    if listening_for.value == name:
      text = '(' + text + ')'
    ig.text(text)

  def cardinal(prefix: str) -> None:
    ig.dummy(45, 1)
    ig.same_line()
    button(prefix + '^')
    button(prefix + '<')
    ig.same_line()
    ig.set_cursor_pos((110, ig.get_cursor_pos().y))
    button(prefix + '>')
    ig.dummy(45, 1)
    ig.same_line()
    button(prefix + 'v')

  button('A')
  button('B')
  button('S')

  ig.dummy(1, 5)
  button('Z')
  button('L')
  button('R')

  ig.dummy(1, 5)
  cardinal('')

  ig.dummy(1, 15)
  cardinal('C')

  ig.dummy(1, 15)
  cardinal('D')

  ig.dummy(200, 1)

  ig.pop_id()


def input_float(name: str) -> float:
  return check_input(bindings.get(name))

def input_bool(name: str) -> bool:
  return bool(round(input_float(name)))


__all__ = ['render_binding_settings', 'input_float', 'input_bool']
