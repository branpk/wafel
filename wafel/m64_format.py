from typing import *
from typing.io import *
import struct

from wafel.core import Variables, Edits, VariableEdit, Variable
from wafel.util import *


 # FIXME: Hack until editing overlapping vars works
def get_input_button_flags(variables: Variables) -> Dict[Variable, int]:
  input_button_vars = {}
  for variable in variables:
    if variable.id.name.startswith('input-button-'):
      input_button_vars[variable] = variable.flag
  return input_button_vars


def load_m64(m64: IO[bytes], variables: Variables) -> Edits:
  input_button_flags = get_input_button_flags(variables)
  edits = Edits()

  prev_buttons = 0
  prev_stick_x = 0
  prev_stick_y = 0

  m64.seek(0x400)
  frame = 0
  while True:
    try:
      buttons = struct.unpack('>H', m64.read(2))[0]
      stick_x = struct.unpack('=b', m64.read(1))[0]
      stick_y = struct.unpack('=b', m64.read(1))[0]
    except struct.error:
      break

    for variable, flag in input_button_flags.items():
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


def save_m64(edits: Edits, m64: IO[bytes], variables: Variables) -> None:
  input_button_flags = get_input_button_flags(variables)

  # TODO: Remove blank frames at end
  m64.write(b'\x4d\x36\x34\x1a')
  m64.write(b'\x03\x00\x00\x00')
  m64.write(b'\x00\x00\x00\x00') # movie uid
  m64.write(b'\xff\xff\xff\xff')

  m64.write(b'\xbb\xff\xff\xff')
  m64.write(b'\x3c\x01\x00\x00')
  m64.write(struct.pack('<I', len(edits._items)))
  m64.write(b'\x02\x00\x00\x00') # power-on

  m64.write(b'\x01\x00\x00\x00')
  m64.write(bytes(160))
  m64.write(bytes_to_buffer(b'SUPER MARIO 64', 32))
  m64.write(b'\x4e\xaa\x3d\x0e') # crc
  m64.write(b'J\x00') # country code
  m64.write(bytes(56))

  m64.write(bytes(64))
  m64.write(bytes(64))
  m64.write(bytes(64))
  m64.write(bytes(64))

  m64.write(bytes_to_buffer(b'Authors here', 222)) # TODO
  m64.write(bytes_to_buffer(b'Description here', 256)) # TODO

  buttons = 0
  stick_x = 0
  stick_y = 0

  for frame_edits in edits._items:
    for edit in frame_edits:
      if isinstance(edit, VariableEdit):
        if edit.variable in input_button_flags:
          flag = input_button_flags[edit.variable]
          if edit.value:
            buttons = buttons | flag
          else:
            buttons = buttons & ~flag
        elif edit.variable == variables['input-buttons']:
          buttons = edit.value
        elif edit.variable == variables['input-stick-x']:
          stick_x = edit.value
        elif edit.variable == variables['input-stick-y']:
          stick_y = edit.value

    m64.write(struct.pack(b'>H', buttons & 0xFFFF))
    m64.write(struct.pack(b'=B', stick_x & 0xFF))
    m64.write(struct.pack(b'=B', stick_y & 0xFF))
