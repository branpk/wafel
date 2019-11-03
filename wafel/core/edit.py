from typing import *
from typing import IO
import struct

from wafel.core.game_state import GameState
from wafel.core.variable import Variable, VariableParam, Variables, \
  VariableGroup, _FlagVariable


class Edit:
  def apply(self, state: GameState) -> None:
    raise NotImplementedError()


class VariableEdit(Edit):
  def __init__(self, variable: Variable, value: Any) -> None:
    self.variable = variable
    self.value = value

    # Don't edit hidden variables, e.g. buttons instead of A, B, Z, as then
    # the edits won't be visible to the user
    # TODO: Maybe implement Variable#contains(Variable) to handle this case instead?
    assert variable.group != VariableGroup.hidden()

  def apply(self, state: GameState) -> None:
    self.variable.set(self.value, { VariableParam.STATE: state })


def read_byte(f: IO[bytes]) -> Optional[int]:
  return (f.read(1) or [None])[0]


def read_big_short(f: IO[bytes]) -> Optional[int]:
  byte1 = read_byte(f)
  byte2 = read_byte(f)
  if byte1 is None or byte2 is None:
    return None
  return byte1 << 8 | byte2


class Edits:
  @staticmethod
  def from_m64(m64: IO[bytes], variables: Variables) -> 'Edits':
    input_button_vars = {}
    for variable in variables:
      if isinstance(variable, _FlagVariable) and variable.flags == variables['input-buttons']:
        input_button_vars[variable] = variable.flag

    edits = Edits()

    prev_buttons = 0
    prev_stick_x = 0
    prev_stick_y = 0

    m64.seek(0x400)
    frame = 0
    while True:
      buttons = read_big_short(m64)
      stick_x = read_byte(m64)
      stick_y = read_byte(m64)

      if buttons is None or stick_x is None or stick_y is None:
        break
      else:
        for variable, flag in input_button_vars.items():
          if (prev_buttons & flag) != (buttons & flag):
            edits.add(frame, VariableEdit(variable, bool(buttons & flag)))
        if stick_x != prev_stick_x:
          edits.add(frame, VariableEdit(variables['input-stick-x'], stick_x))
        if stick_y != prev_stick_y:
          edits.add(frame, VariableEdit(variables['input-stick-y'], stick_y))

      prev_buttons = buttons
      prev_stick_x = stick_x
      prev_stick_y = stick_y
      frame += 1

    return edits

  def save_m64(self, m64: IO[bytes], variables: Variables) -> None:
    # TODO: Remove blank frames at end
    m64.write(b'\x4d\x36\x34\x1a')
    m64.write(b'\x03\x00\x00\x00')
    m64.write(b'\x00\x00\x00\x00') # movie uid
    m64.write(b'\xff\xff\xff\xff')

    m64.write(b'\xbb\xff\xff\xff')
    m64.write(b'\x3c\x01\x00\x00')
    m64.write(struct.pack('<I', len(self._items)))
    m64.write(b'\x02\x00\x00\x00') # power-on

    m64.write(b'\x01\x00\x00\x00')
    m64.write(bytes(160))
    m64.write(b'SUPER MARIO 64'.ljust(32, b'\x00'))
    m64.write(b'\x4e\xaa\x3d\x0e') # crc
    m64.write(b'J\x00') # country code
    m64.write(bytes(56))

    m64.write(bytes(64))
    m64.write(bytes(64))
    m64.write(bytes(64))
    m64.write(bytes(64))

    m64.write(b'Authors here'.ljust(222, b'\x00')) # TODO
    m64.write(b'Description here'.ljust(256, b'\x00')) # TODO

    buttons = 0
    stick_x = 0
    stick_y = 0

    for edits in self._items:
      for edit in edits:
        if isinstance(edit, VariableEdit):
          if isinstance(edit.variable, _FlagVariable) and \
              edit.variable.flags == variables['input-buttons']:
            if edit.value:
              buttons = buttons | edit.variable.flag
            else:
              buttons = buttons & ~edit.variable.flag
          elif edit.variable == variables['input-buttons']:
            buttons = edit.value
          elif edit.variable == variables['input-stick-x']:
            stick_x = edit.value
          elif edit.variable == variables['input-stick-y']:
            stick_y = edit.value

      m64.write(struct.pack(b'>H', buttons & 0xFFFF))
      m64.write(struct.pack(b'<B', stick_x & 0xFF))
      m64.write(struct.pack(b'<B', stick_y & 0xFF))

  def __init__(self):
    self._items: List[List[Edit]] = []
    self.edit_frame_callbacks: List[Callable[[int], None]] = []

  def on_edit(self, callback: Callable[[int], None]) -> None:
    self.edit_frame_callbacks.append(callback)

  def extend(self, new_len: int) -> None:
    while len(self._items) < new_len:
      self._items.append([])

  def insert_frame(self, index: int) -> None:
    self.extend(index)
    self._items.insert(index, [])
    self._invalidate(index)

  def delete_frame(self, index: int) -> None:
    if index < len(self._items):
      del self._items[index]
      self._invalidate(index - 1)

  def _invalidate(self, frame: int) -> None:
    for callback in list(self.edit_frame_callbacks):
      callback(frame)

  def add(self, frame: int, edit: Edit) -> None:
    # TODO: Remove overwritten edits
    self._get_edits(frame).append(edit)
    self._invalidate(frame)

  def _get_edits(self, frame: int) -> List[Edit]:
    while frame >= len(self._items):
      self._items.append([])
    return self._items[frame]

  def apply(self, state: GameState) -> None:
    for edit in self._get_edits(state.frame):
      edit.apply(state)

  def is_edited(self, frame: int, variable: Variable) -> bool:
    for edit in self._get_edits(frame):
      if isinstance(edit, VariableEdit) and edit.variable == variable:
        return True
    return False

  def reset(self, frame: int, variable: Variable) -> None:
    edits = self._get_edits(frame)
    for edit in list(edits):
      if isinstance(edit, VariableEdit) and edit.variable == variable:
        edits.remove(edit)
    self._invalidate(frame)
