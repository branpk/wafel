from typing import *
import json

from wafel.edit import Edits
from wafel.variable import Variable
from wafel.input_buttons import INPUT_BUTTON_FLAGS
from wafel.tas_metadata import TasMetadata


INPUT_BUTTON_LABELS = {
  Variable('input-button-a'): 'A',
  Variable('input-button-b'): 'B',
  Variable('input-button-z'): 'Z',
  Variable('input-button-s'): 'S',
  Variable('input-button-l'): 'L',
  Variable('input-button-r'): 'R',
  Variable('input-button-cu'): 'C^',
  Variable('input-button-cl'): 'C<',
  Variable('input-button-cr'): 'C>',
  Variable('input-button-cd'): 'Cv',
  Variable('input-button-du'): 'D^',
  Variable('input-button-dl'): 'D<',
  Variable('input-button-dr'): 'D>',
  Variable('input-button-dd'): 'Dv',
}


def get_input_button_by_label(label: str) -> Variable:
  for variable, value in INPUT_BUTTON_LABELS.items():
    if value.lower() == label.lower():
      return variable
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

  def iterencode(self, o):
    for s in super().iterencode(o):
      yield self.subst.get(s.strip('"')) or s


def save_wafi(filename: str, metadata: TasMetadata, edits: Edits) -> None:
  input_data = []

  buttons = 0
  stick_x = 0
  stick_y = 0

  for frame in range(len(edits)):
    # Variable hacks
    for edit in edits.get_edits(frame):
      if not edit.variable.name.startswith('input-'):
        hack_data: Dict[str, Any] = { 'variable': edit.variable.name }
        if 'object' in edit.variable.args:
          hack_data['object_slot'] = edit.variable.args['object']
        hack_data['value'] = edit.value
        input_data.append(NoIndent(hack_data))

    # Inputs
    for edit in edits.get_edits(frame):
      if edit.variable.name.startswith('input-'):
        if edit.variable in INPUT_BUTTON_FLAGS:
          flag = INPUT_BUTTON_FLAGS[edit.variable]
          if edit.value:
            buttons = buttons | flag
          else:
            buttons = buttons & ~flag
        elif edit.variable == Variable('input-buttons'):
          buttons = edit.value
        elif edit.variable == Variable('input-stick-x'):
          stick_x = edit.value
        elif edit.variable == Variable('input-stick-y'):
          stick_y = edit.value
        else:
          raise NotImplementedError(edit.variable)

    stick_inputs: List[object] = [stick_x, stick_y]
    button_labels: List[object] = [
      label for id, label in INPUT_BUTTON_LABELS.items()
        if buttons & INPUT_BUTTON_FLAGS[id]
    ]
    input_data.append(NoIndent(stick_inputs + button_labels))

  data = {
    'info': {
      'title': metadata.title,
      'authors': metadata.authors,
      'description': metadata.description,
    },
    'game': {
      'name': 'Super Mario 64',
      'version': metadata.game_version,
    },
    'frame_range': [0, len(edits)],
    'inputs': input_data,
    '_version': 0,
  }

  with open(filename, 'w') as f:
    json.dump(data, f, indent=2, cls=Encoder)


def load_wafi(filename: str) -> Tuple[TasMetadata, Edits]:
  # TODO: Json validation

  with open(filename, 'r') as f:
    data = json.load(f)
  assert data['_version'] == 0

  metadata = TasMetadata(
    data['game']['version'],
    data['info']['title'],
    data['info']['authors'],
    data['info']['description'],
  )

  edits = Edits()

  prev_buttons = 0
  prev_stick_x = 0
  prev_stick_y = 0

  frame = 0
  for edit in data['inputs']:
    if isinstance(edit, dict):
      variable = Variable(edit['variable'])
      if 'object_slot' in edit:
        variable = variable.at(object=edit['object_slot'])
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
        edits.edit(frame, 'input-stick-x', stick_x)
      if stick_y != prev_stick_y:
        edits.edit(frame, 'input-stick-y', stick_y)
      for button, flag in INPUT_BUTTON_FLAGS.items():
        if (buttons & flag) != (prev_buttons & flag):
          edits.edit(frame, button, bool(buttons & flag))

      prev_buttons = buttons
      prev_stick_x = stick_x
      prev_stick_y = stick_y
      frame += 1

  return (metadata, edits)
