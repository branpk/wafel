from typing import *
from typing.io import *
import json

from wafel.core import Edits, Variables, VariableEdit, INPUT_BUTTON_FLAGS


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

  for frame_edits in edits._items:
    # Variable hacks
    for edit in frame_edits:
      if not isinstance(edit, VariableEdit):
        raise NotImplementedError(edit)

      var = edit.variable
      if not var.id.name.startswith('input-'):
        hack_data: Dict[str, Any] = { 'variable': var.id.name }
        if var.id.object_id is not None:
          hack_data['object_slot'] = var.id.object_id  # TODO: Slot
        hack_data['value'] = edit.value
        input_data.append(NoIndent(hack_data))

    # Inputs
    for edit in frame_edits:
      if not isinstance(edit, VariableEdit):
        raise NotImplementedError(edit)

      var = edit.variable
      if var.id.name.startswith('input-'):
        if var.id.name in INPUT_BUTTON_FLAGS:
          flag = INPUT_BUTTON_FLAGS[var.id.name]
          if edit.value:
            buttons = buttons | flag
          else:
            buttons = buttons & ~flag
        elif var.id.name == 'input-buttons':
          buttons = edit.value
        elif var.id.name == 'input-stick-x':
          stick_x = edit.value
        elif var.id.name == 'input-stick-y':
          stick_y = edit.value
        else:
          raise NotImplementedError(var)

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
    'frame_range': [0, len(edits._items)],
    'inputs': input_data,
    '_version': 0,
  }
  json.dump(data, file, indent=2, cls=Encoder)
