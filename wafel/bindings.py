from typing import *
from dataclasses import dataclass
import time

import glfw

import wafel.imgui as ig
from wafel.local_state import use_state
from wafel.util import *


# TODO: Keyboard (along w/ program hotkeys)

@dataclass(frozen=True)
class JoystickButton:
  index: int

  def __str__(self) -> str:
    return f'Button {self.index + 1}'

@dataclass(frozen=True)
class JoystickAxis:
  index: int
  direction: int

  def __str__(self) -> str:
    sign = '+' if self.direction > 0 else '-'
    return f'Axis {self.index + 1}{sign}'

JoystickControl = Union[JoystickButton, JoystickAxis]


@dataclass
class Controller:
  bound: Dict[str, JoystickControl]


def detect_control(joystick: int) -> Optional[JoystickControl]:
  axes_ptr, length = glfw.get_joystick_axes(glfw.JOYSTICK_1)
  axes = tuple(axes_ptr[i] for i in range(length))

  if len(axes) > 0:
    max_axis = max(range(len(axes)), key=lambda i: abs(axes[i]))
    if abs(axes[max_axis]) > 0.5:
      return JoystickAxis(max_axis, 1 if axes[max_axis] > 0 else -1)

  buttons_ptr, length = glfw.get_joystick_buttons(glfw.JOYSTICK_1)
  buttons = tuple(buttons_ptr[i] for i in range(length))

  for i, pressed in enumerate(buttons):
    if pressed:
      return JoystickButton(i)

  return None


def check_control(joystick: int, control: Optional[JoystickControl]) -> float:
  if control is None:
    return 0.0

  elif isinstance(control, JoystickAxis):
    axes_ptr, length = glfw.get_joystick_axes(glfw.JOYSTICK_1)
    axes = tuple(axes_ptr[i] for i in range(length))
    if control.index < len(axes):
      axis_value = float(axes[control.index])
      return max(axis_value * control.direction, 0.0)
    else:
      return 0.0

  elif isinstance(control, JoystickButton):
    buttons_ptr, length = glfw.get_joystick_buttons(glfw.JOYSTICK_1)
    buttons = tuple(buttons_ptr[i] for i in range(length))
    if control.index < len(buttons) and buttons[control.index]:
      return 1.0
    else:
      return 0.0


def render_binding_settings(id: str) -> None:
  ig.push_id(id)

  # TODO: Controller mappings should be saved between sessions and should try
  # to pick the correct mapping based on joystick index and name

  used_joystick: Ref[Optional[int]] = use_state('used-joystick', None)
  controller = use_state('controller', Controller(bound={})).value

  joysticks = [glfw.JOYSTICK_1 + i for i in range(16)]
  joysticks = [j for j in joysticks if glfw.joystick_present(j)]
  joystick_labels = [
    str(i + 1) + ': ' + glfw.get_joystick_name(j).decode('utf-8')
      for i, j in enumerate(joysticks)
  ]

  # TODO: Check if joysticks is empty

  if used_joystick.value in joysticks:
    joystick_index = joysticks.index(used_joystick.value)
  else:
    joystick_index = 0

  used_joystick.value = joysticks[ig.combo('##used-joystick', joystick_index, joystick_labels)[1]]

  if used_joystick.value is None:
    ig.pop_id()
    return

  n64_controls = [
    'A', 'B', 'S',
    'Z', 'L', 'R',
    '^', '<', '>', 'v',
    'C^', 'C<', 'C>', 'Cv',
    'D^', 'D<', 'D>', 'Dv',
  ]

  listening_for: Ref[Optional[str]] = use_state('listening-for', None)
  waiting_for_release: Ref[bool] = use_state('waiting-for-release', False)

  if listening_for.value is not None:
    control = detect_control(used_joystick.value)
    if control is not None and not waiting_for_release.value:
      controller.bound[listening_for.value] = control
      index = n64_controls.index(listening_for.value) + 1
      listening_for.value = n64_controls[index] if index < len(n64_controls) else None
      waiting_for_release.value = True
    elif ig.is_key_pressed(ig.get_key_index(ig.KEY_ESCAPE)):
      # if listening_for.value in controller.bound:
      #   del controller.bound[listening_for.value]
      listening_for.value = None
    if control is None:
      waiting_for_release.value = False

  def button(name: str) -> None:
    value = check_control(assert_not_none(used_joystick.value), controller.bound.get(name))
    color = (0.26 + value * 0.7, 0.59 + value * 0.41, 0.98, 0.4)
    if listening_for.value == name:
      color = (0.86, 0.59, 0.98, 0.4)
    ig.push_style_color(ig.COLOR_BUTTON, *color)
    if ig.button(name):
      listening_for.value = name
    ig.pop_style_color()
    ig.same_line()
    if name in controller.bound:
      text = str(controller.bound[name])
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

  ig.dummy(1, 5)
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
