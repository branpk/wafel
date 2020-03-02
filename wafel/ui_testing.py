from typing import *
import random

import imgui as ig

import wafel.ui as ui
from wafel.ui import use_state, use_state_with
from wafel.window import open_window_and_run
from wafel.core import ObjectType
from wafel.util import *


def test_object_slots():
  ig.push_id('test-object-slots')

  def initial_object_types() -> List[Optional[ObjectType]]:
    object_types = [
      ObjectType(1, 'bhvMario'),
      ObjectType(2, 'bhvGoomba'),
      ObjectType(3, 'bhvPokeyBodyPart'),
      ObjectType(4, 'bhvButterflyTriplet'),
      None,
      None,
    ] * 40
    random.shuffle(object_types)
    return object_types

  object_types = use_state_with('_object-types', initial_object_types)
  selected_slot: Ref[Optional[int]] = use_state('selected-slot', None)

  selected = ui.render_object_slots(object_types.value)
  if selected is not None:
    selected_slot.value = selected

  ig.pop_id()


def test_joystick_control():
  ig.push_id('test-joystick-control')

  stick = use_state('stick', (0.0, 0.0))

  new_stick = ui.render_joystick_control(stick.value[0], stick.value[1])
  if new_stick is not None:
    stick.value = new_stick

  ig.pop_id()


TESTS = [
  test_object_slots,
  test_joystick_control,
]


def render_tests() -> None:
  ig.push_id('tests')
  test_index = use_state('_current-test', 0)
  ig.columns(2)

  ig.set_column_width(-1, 300)

  for i, test in enumerate(TESTS):
    test_name = test.__name__
    if test_name.startswith('test_'):
      test_name = test_name[len('test_'):]

    _, selected = ig.selectable(f'{test_name}##{id(i)}', test_index.value == i)
    if selected:
      test_index.value = i

  ig.next_column()
  ig.begin_child('test')

  for k, v in ui.local_state.items():
    if not k[1].startswith('_'):
      ig.text(f'{k} -> {v.value}')

  ig.separator()
  TESTS[test_index.value]()
  ig.separator()

  ig.end_child()
  ig.columns(1)
  ig.pop_id()


open_window_and_run(render_tests)
