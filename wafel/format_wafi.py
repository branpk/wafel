from typing import *
from typing.io import *
import json

from wafel.core import Edits, VariableId, INPUT_BUTTON_FLAGS
from wafel.tas_metadata import TasMetadata


INPUT_BUTTON_LABELS = {
  VariableId('input-button-a'): 'A',
  VariableId('input-button-b'): 'B',
  VariableId('input-button-z'): 'Z',
  VariableId('input-button-s'): 'S',
  VariableId('input-button-l'): 'L',
  VariableId('input-button-r'): 'R',
  VariableId('input-button-cu'): 'C^',
  VariableId('input-button-cl'): 'C<',
  VariableId('input-button-cr'): 'C>',
  VariableId('input-button-cd'): 'Cv',
  VariableId('input-button-du'): 'D^',
  VariableId('input-button-dl'): 'D<',
  VariableId('input-button-dr'): 'D>',
  VariableId('input-button-dd'): 'Dv',
}


def get_input_button_by_label(label: str) -> VariableId:
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

  def iterencode(self, o: Any) -> Iterator[str]:
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
      if not edit.variable_id.name.startswith('input-'):
        hack_data: Dict[str, Any] = { 'variable': edit.variable_id.name }
        if edit.variable_id.object_id is not None:
          hack_data['object_slot'] = edit.variable_id.object_id  # TODO: Slot
        hack_data['value'] = edit.value
        input_data.append(NoIndent(hack_data))

    # Inputs
    for edit in edits.get_edits(frame):
      if edit.variable_id.name.startswith('input-'):
        if edit.variable_id in INPUT_BUTTON_FLAGS:
          flag = INPUT_BUTTON_FLAGS[edit.variable_id]
          if edit.value:
            buttons = buttons | flag
          else:
            buttons = buttons & ~flag
        elif edit.variable_id == VariableId('input-buttons'):
          buttons = edit.value
        elif edit.variable_id == VariableId('input-stick-x'):
          stick_x = edit.value
        elif edit.variable_id == VariableId('input-stick-y'):
          stick_y = edit.value
        else:
          raise NotImplementedError(edit.variable_id)

    button_labels = [
      label for id, label in INPUT_BUTTON_LABELS.items()
        if buttons & INPUT_BUTTON_FLAGS[id]
    ]
    input_data.append(NoIndent([stick_x, stick_y] + button_labels))

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
      variable = VariableId(edit['variable'])
      if 'object_slot' in edit:
        variable = variable.with_object_id(edit['object_slot'])
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
