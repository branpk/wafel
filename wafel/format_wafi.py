from typing import *
from typing.io import *
import json

from wafel.core import Edits, Variables, INPUT_BUTTON_FLAGS


INPUT_BUTTON_LABELS = {
  'input-button-a': 'A',
  'input-button-b': 'B',
  'input-button-z': 'Z',
  'input-button-s': 'S',
  'input-button-l': 'L',
  'input-button-r': 'R',
  'input-button-cu': 'C^',
  'input-button-cl': 'C<',
  'input-button-cr': 'C>',
  'input-button-cd': 'Cv',
  'input-button-du': 'D^',
  'input-button-dl': 'D<',
  'input-button-dr': 'D>',
  'input-button-dd': 'Dv',
}


def get_input_button_by_label(label: str) -> str:
  for id, value in INPUT_BUTTON_LABELS.items():
    if value.lower() == label.lower():
      return id
  raise Exception('Invalid button: ' + label)


class NoIndent:
  def __init__(self, data: object):
    self.data = data


class Encoder(json.JSONEncoder):
  def __init__(self, *args, **kwargs):
    super().__init__(*args, **kwargs)
    self.subst = {}

  def default(self, o: Any) -> Any:
    if isinstance(o, NoIndent):
      key = '__subst_' + str(len(self.subst)) + '__'
      self.subst[key] = json.dumps(o.data)
      return key
    return super().default(o)

  def iterencode(self, o: Any) -> Iterator[str]:
    for s in super().iterencode(o):
      yield self.subst.get(s.strip('"')) or s


def save_wafi(edits: Edits, file: IO[str], variables: Variables) -> None:
  input_data = []

  buttons = 0
  stick_x = 0
  stick_y = 0

  for frame in range(len(edits)):
    # Variable hacks
    for variable, value in edits.get_edits(frame):
      if not variable.id.name.startswith('input-'):
        hack_data: Dict[str, Any] = { 'variable': variable.id.name }
        if variable.id.object_id is not None:
          hack_data['object_slot'] = variable.id.object_id  # TODO: Slot
        hack_data['value'] = value
        input_data.append(NoIndent(hack_data))

    # Inputs
    for variable, value in edits.get_edits(frame):
      if variable.id.name.startswith('input-'):
        if variable.id.name in INPUT_BUTTON_FLAGS:
          flag = INPUT_BUTTON_FLAGS[variable.id.name]
          if value:
            buttons = buttons | flag
          else:
            buttons = buttons & ~flag
        elif variable.id.name == 'input-buttons':
          buttons = value
        elif variable.id.name == 'input-stick-x':
          stick_x = value
        elif variable.id.name == 'input-stick-y':
          stick_y = value
        else:
          raise NotImplementedError(variable)

    button_labels = [
      label for id, label in INPUT_BUTTON_LABELS.items()
        if buttons & INPUT_BUTTON_FLAGS[id]
    ]
    input_data.append(NoIndent([stick_x, stick_y] + button_labels))

   # TODO: Title, author, description, game version
  data = {
    'info': {
      'title': 'Title',
      'authors': ['Author 1', 'Author 2'],
      'description': 'Description',
    },
    'game': {
      'name': 'Super Mario 64',
      'version': 'J',
    },
    'frame_range': [0, len(edits)],
    'inputs': input_data,
    '_version': 0,
  }
  json.dump(data, file, indent=2, cls=Encoder)


def load_wafi(file: IO[str], variables: Variables) -> Edits:
  data = json.load(file)
  assert data['_version'] == 0

  edits = Edits()

  prev_buttons = 0
  prev_stick_x = 0
  prev_stick_y = 0

  frame = 0
  for edit in data['inputs']:
    if isinstance(edit, dict):
      # TODO: Error if variable not supported
      variable = variables[edit['variable']]
      if 'object_slot' in edit:
        variable = variable.at_object(edit['object_slot'])
      edits.edit(frame, variable, edit['value'])

    else:
      assert isinstance(edit, list)

      stick_x = edit[0]
      stick_y = edit[1]
      buttons = 0
      for button_label in edit[2:]:
        button = get_input_button_by_label(button_label)
        buttons |= INPUT_BUTTON_FLAGS[button]

      if stick_x != prev_stick_x:
        edits.edit(frame, variables['input-stick-x'], stick_x)
      if stick_y != prev_stick_y:
        edits.edit(frame, variables['input-stick-y'], stick_y)
      for button, flag in INPUT_BUTTON_FLAGS.items():
        if (buttons & flag) != (prev_buttons & flag):
          edits.edit(frame, variables[button], bool(buttons & flag))

      prev_buttons = buttons
      prev_stick_x = stick_x
      prev_stick_y = stick_y
      frame += 1

  return edits
