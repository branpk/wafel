from typing import *
from typing.io import *
import struct

from wafel.core import Variables, Edits, VariableEdit, Variable, INPUT_BUTTON_FLAGS
from wafel.util import *


def load_m64(file: IO[bytes], variables: Variables) -> Edits:
  edits = Edits()

  prev_buttons = 0
  prev_stick_x = 0
  prev_stick_y = 0

  file.seek(0x400)
  frame = 0
  while True:
    try:
      buttons = struct.unpack('>H', file.read(2))[0]
      stick_x = struct.unpack('=b', file.read(1))[0]
      stick_y = struct.unpack('=b', file.read(1))[0]
    except struct.error:
      break

    for var_name, flag in INPUT_BUTTON_FLAGS.items():
      if (prev_buttons & flag) != (buttons & flag):
        edits.add(frame, VariableEdit(variables[var_name], bool(buttons & flag)))
    if stick_x != prev_stick_x:
      edits.add(frame, VariableEdit(variables['input-stick-x'], stick_x))
    if stick_y != prev_stick_y:
      edits.add(frame, VariableEdit(variables['input-stick-y'], stick_y))

    prev_buttons = buttons
    prev_stick_x = stick_x
    prev_stick_y = stick_y
    frame += 1

  return edits


def save_m64(edits: Edits, file: IO[bytes], variables: Variables) -> None:
  # TODO: Remove blank frames at end
  # TODO: crc, country code, authors, description
  file.write(b'\x4d\x36\x34\x1a')
  file.write(b'\x03\x00\x00\x00')
  file.write(b'\x00\x00\x00\x00') # movie uid
  file.write(b'\xff\xff\xff\xff')

  file.write(b'\xbb\xff\xff\xff')
  file.write(b'\x3c\x01\x00\x00')
  file.write(struct.pack('<I', len(edits._items)))
  file.write(b'\x02\x00\x00\x00') # power-on

  file.write(b'\x01\x00\x00\x00')
  file.write(bytes(160))
  file.write(bytes_to_buffer(b'SUPER MARIO 64', 32))
  file.write(b'\x4e\xaa\x3d\x0e') # crc
  file.write(b'J\x00') # country code
  file.write(bytes(56))

  file.write(bytes(64))
  file.write(bytes(64))
  file.write(bytes(64))
  file.write(bytes(64))

  file.write(bytes_to_buffer(b'Authors here', 222))
  file.write(bytes_to_buffer(b'Description here', 256))

  buttons = 0
  stick_x = 0
  stick_y = 0

  for frame_edits in edits._items:
    for edit in frame_edits:
      if isinstance(edit, VariableEdit):
        if edit.variable.id.name in INPUT_BUTTON_FLAGS:
          flag = INPUT_BUTTON_FLAGS[edit.variable.id.name]
          if edit.value:
            buttons |= flag
          else:
            buttons &= ~flag
        elif edit.variable == variables['input-buttons']:
          buttons = edit.value
        elif edit.variable == variables['input-stick-x']:
          stick_x = edit.value
        elif edit.variable == variables['input-stick-y']:
          stick_y = edit.value

    file.write(struct.pack(b'>H', buttons & 0xFFFF))
    file.write(struct.pack(b'=B', stick_x & 0xFF))
    file.write(struct.pack(b'=B', stick_y & 0xFF))
