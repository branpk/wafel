from typing import *
import random

import imgui as ig

import wafel.ui as ui
from wafel.local_state import use_state, use_state_with, local_state
from wafel.window import open_window_and_run
from wafel.core import ObjectType, VariableId
from wafel.util import *
from wafel.variable_format import DecimalIntFormatter, CheckboxFormatter
from wafel.slot_testing import test_timeline_algorithm


# TODO: Hot reloading?


def test_object_slots(id: str) -> None:
  ig.push_id(id)

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

  selected = ui.render_object_slots('object-slots', object_types.value)
  if selected is not None:
    selected_slot.value = selected

  ig.pop_id()


def test_joystick_control(id: str) -> None:
  ig.push_id(id)

  shapes = ['square', 'circle']
  shape_index = use_state('shape-index', 0)
  _, shape_index.value = ig.combo('##shape', shape_index.value, shapes)

  stick = use_state('stick', (0.0, 0.0))

  new_stick = ui.render_joystick_control(
    'joystick-control',
    stick.value[0],
    stick.value[1],
    shapes[shape_index.value],
  )
  if new_stick is not None:
    stick.value = new_stick

  ig.text(f'stick = {stick.value}')

  ig.pop_id()


def test_variable_value(id: str) -> None:
  ig.push_id(id)

  int_variable = use_state('int-variable', 0)
  bool_variable = use_state('bool-variable', False)
  last_selected = use_state('last-selected', '')

  cell_width = 80
  cell_height = ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1]

  new_int_value, selected = ui.render_variable_value(
    'int-value',
    int_variable.value,
    DecimalIntFormatter(),
    (cell_width, cell_height),
    highlight=bool_variable.value,
  )
  if new_int_value is not None:
    int_variable.value = new_int_value.value
  if selected:
    last_selected.value = 'int'

  new_int_value, selected = ui.render_variable_value(
    'int-value-copy',
    int_variable.value,
    DecimalIntFormatter(),
    (cell_width, cell_height),
    highlight=not bool_variable.value,
  )
  if new_int_value is not None:
    int_variable.value = new_int_value.value
  if selected:
    last_selected.value = 'int2'

  new_bool_value, selected = ui.render_variable_value(
    'bool-value',
    bool_variable.value,
    CheckboxFormatter(),
    (cell_width, cell_height),
    highlight=bool_variable.value,
  )
  if new_bool_value is not None:
    bool_variable.value = new_bool_value.value
  if selected:
    last_selected.value = 'bool'

  ig.pop_id()


def test_labeled_variable(id: str) -> None:
  ig.push_id(id)

  default = 1234
  variable = use_state('value', default)
  edited = use_state('edited', False)

  changed_value, clear_edit = ui.render_labeled_variable(
    'var',
    'Variable',
    VariableId('my-var'),
    variable.value,
    DecimalIntFormatter(),
    edited.value,
  )

  if changed_value is not None:
    variable.value = changed_value.value
    edited.value = True
  if clear_edit:
    edited.value = False

  ig.pop_id()


def test_tabs(id: str) -> None:
  ig.push_id(id)

  open_tabs = use_state('open-tabs', {3}).value

  for i in range(1, 6):
    if ig.button(f'Open {i}##open-{i}'):
      open_tabs.add(i)
    if i != 5:
      ig.same_line()
  for i in range(1, 6):
    if ig.button(f'Close {i}##close-{i}'):
      if i in open_tabs:
        open_tabs.remove(i)
    if i != 5:
      ig.same_line()

  def tab_render(id: str) -> None:
    ig.text(f'Tab id = {id}')

  ui.render_tabs(
    'tabs',
    [
      (f'tab-{i}', f'Tab {i}', tab_render)
        for i in sorted(open_tabs)
    ],
  )

  ig.pop_id()


def test_frame_slider(id: str) -> None:
  ig.push_id(id)

  current = use_state('current', 0)
  length = use_state('length', 1000)

  new_length = ui.render_frame_slider(
    'length',
    length.value,
    10000,
  )
  if new_length is not None:
    length.value = new_length.value

  new_value = ui.render_frame_slider(
    'slider',
    current.value,
    length.value,
    [0, 5, 10, length.value - 11, length.value - 6, length.value - 1],
  )
  if new_value is not None:
    current.value = new_value.value

  ig.pop_id()


def test_variable_cell(id: str) -> None:
  ig.push_id(id)

  num_columns = 5
  num_rows = 5
  ig.columns(num_columns)

  for row in range(num_rows):
    for column in range(num_columns):
      ui.render_variable_cell(
        f'cell-{row}-{column}',
        1234,
        DecimalIntFormatter(),
        (ig.get_column_width(-1), 30),
        row == 1 and column == 2,
        False,
      )
      ig.next_column()
    ig.separator()

  ig.columns(1)

  ig.pop_id()


DEFAULT_TEST = test_timeline_algorithm

TESTS = [
  test_joystick_control,
  test_object_slots,
  test_variable_value,
  test_labeled_variable,
  test_tabs,
  test_frame_slider,
  test_variable_cell,
  test_timeline_algorithm,
]


def render_tests(id: str) -> None:
  ig.push_id(id)

  test_index = use_state('_current-test', TESTS.index(DEFAULT_TEST))
  ig.columns(2)

  ig.set_column_width(-1, 220)

  for i, test in enumerate(TESTS):
    test_name = test.__name__.replace('_', '-')
    if test_name.startswith('test-'):
      test_name = test_name[len('test-'):]

    _, selected = ig.selectable(f'{test_name}##{i}', test_index.value == i)
    if selected:
      test_index.value = i

  ig.next_column()
  ig.begin_child('test')

  # for k, v in local_state.items():
  #   if not k[1].startswith('_'):
  #     ig.text(f'{k} -> {v.value}')

  ig.separator()
  test = TESTS[test_index.value]
  test(test.__name__)
  ig.separator()

  ig.end_child()
  ig.columns(1)

  ig.pop_id()


open_window_and_run(render_tests)
