from typing import *
import struct

from wafel.core import INPUT_BUTTON_FLAGS, VariableId
from wafel.util import *
from wafel.input_file import InputFile


def save_m64(filename: str, input: InputFile) -> None:
  with open(filename, 'wb') as f:
    # TODO: Remove blank frames at end
    # TODO: crc, country code, authors, description
    f.write(b'\x4d\x36\x34\x1a')
    f.write(b'\x03\x00\x00\x00')
    f.write(b'\x00\x00\x00\x00') # movie uid
    f.write(b'\xff\xff\xff\xff')

    f.write(b'\xbb\xff\xff\xff')
    f.write(b'\x3c\x01\x00\x00')
    f.write(struct.pack('<I', len(input.edits)))
    f.write(b'\x02\x00\x00\x00') # power-on

    f.write(b'\x01\x00\x00\x00')
    f.write(bytes(160))
    f.write(bytes_to_buffer(b'SUPER MARIO 64', 32))
    f.write(b'\x4e\xaa\x3d\x0e') # crc
    f.write(b'J\x00') # country code
    f.write(bytes(56))

    f.write(bytes(64))
    f.write(bytes(64))
    f.write(bytes(64))
    f.write(bytes(64))

    f.write(bytes_to_buffer(b'Authors here', 222))
    f.write(bytes_to_buffer(b'Description here', 256))

    buttons = 0
    stick_x = 0
    stick_y = 0

    for frame in range(len(input.edits)):
      for variable, value in input.get_edits(frame):
        if variable in INPUT_BUTTON_FLAGS:
          flag = INPUT_BUTTON_FLAGS[variable]
          if value:
            buttons |= flag
          else:
            buttons &= ~flag
        elif variable == VariableId('input-buttons'):
          buttons = value
        elif variable == VariableId('input-stick-x'):
          stick_x = value
        elif variable == VariableId('input-stick-y'):
          stick_y = value

      f.write(struct.pack(b'>H', buttons & 0xFFFF))
      f.write(struct.pack(b'=B', stick_x & 0xFF))
      f.write(struct.pack(b'=B', stick_y & 0xFF))


def load_m64(filename: str) -> InputFile:
  with open(filename, 'rb') as f:
    input = InputFile(
      'jp',
      filename,
      ['TODO'],
      'TODO',
    )

    prev_buttons = 0
    prev_stick_x = 0
    prev_stick_y = 0

    f.seek(0x400)
    frame = 0
    while True:
      try:
        buttons = struct.unpack('>H', f.read(2))[0]
        stick_x = struct.unpack('=b', f.read(1))[0]
        stick_y = struct.unpack('=b', f.read(1))[0]
      except struct.error:
        break

      for variable, flag in INPUT_BUTTON_FLAGS.items():
        if (prev_buttons & flag) != (buttons & flag):
          input.get_edits(frame).append((variable, bool(buttons & flag)))
      if stick_x != prev_stick_x:
        input.get_edits(frame).append((VariableId('input-stick-x'), stick_x))
      if stick_y != prev_stick_y:
        input.get_edits(frame).append((VariableId('input-stick-y'), stick_y))

      prev_buttons = buttons
      prev_stick_x = stick_x
      prev_stick_y = stick_y
      frame += 1

    return input
