from typing import *
import struct

from wafel.core import INPUT_BUTTON_FLAGS, VariableId, Edits
from wafel.util import *
from wafel.tas_metadata import TasMetadata


CRC_CODES = {
  'jp': b'\x4e\xaa\x3d\x0e',
  'us': b'\x63\x5a\x2b\xff',
}

COUNTRY_CODES = {
  'jp': b'J',
  'us': b'E',
}


def save_m64(filename: str, metadata: TasMetadata, edits: Edits) -> None:
  with open(filename, 'wb') as f:
    # TODO: Remove blank frames at end
    f.write(b'\x4d\x36\x34\x1a')
    f.write(b'\x03\x00\x00\x00')
    f.write(b'\x00\x00\x00\x00') # movie uid
    f.write(b'\xff\xff\xff\xff')

    f.write(struct.pack('<I', (metadata.rerecords or 0) & 0xFFFFFFFF))
    f.write(b'\x3c\x01\x00\x00')
    f.write(struct.pack('<I', len(edits)))
    f.write(b'\x02\x00\x00\x00') # power-on

    f.write(b'\x01\x00\x00\x00')
    f.write(bytes(160))
    f.write(bytes_to_buffer(b'SUPER MARIO 64', 32))
    f.write(CRC_CODES[metadata.game_version])
    f.write(COUNTRY_CODES[metadata.game_version])
    f.write(bytes(57))

    f.write(bytes(64))
    f.write(bytes(64))
    f.write(bytes(64))
    f.write(bytes(64))

    f.write(bytes_to_buffer(metadata.authors.encode('utf-8'), 222))
    f.write(bytes_to_buffer(metadata.description.encode('utf-8'), 256))

    buttons = 0
    stick_x = 0
    stick_y = 0

    for frame in range(len(edits)):
      for edit in edits.get_edits(frame):
        if edit.variable_id in INPUT_BUTTON_FLAGS:
          flag = INPUT_BUTTON_FLAGS[edit.variable_id]
          if edit.value:
            buttons |= flag
          else:
            buttons &= ~flag
        elif edit.variable_id == VariableId('input-buttons'):
          buttons = edit.value
        elif edit.variable_id == VariableId('input-stick-x'):
          stick_x = edit.value
        elif edit.variable_id == VariableId('input-stick-y'):
          stick_y = edit.value

      f.write(struct.pack(b'>H', buttons & 0xFFFF))
      f.write(struct.pack(b'=B', stick_x & 0xFF))
      f.write(struct.pack(b'=B', stick_y & 0xFF))


def load_m64(filename: str) -> Tuple[TasMetadata, Edits]:
  with open(filename, 'rb') as f:
    f.seek(0x10)
    rerecords = struct.unpack('>H', f.read(2))[0]

    f.seek(0xE4)
    crc = f.read(4)
    game_version = dict_inverse(CRC_CODES).get(crc) or 'us'
    # TODO: Fall back to country code?

    f.seek(0x222)
    authors = f.read(222).partition(b'\x00')[0].decode('utf-8')

    f.seek(0x300)
    description = f.read(256).partition(b'\x00')[0].decode('utf-8')

    metadata = TasMetadata(
      game_version,
      filename,
      authors,
      description,
      rerecords,
    )
    edits = Edits()

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
          edits.edit(frame, variable, bool(buttons & flag))
      if stick_x != prev_stick_x:
        edits.edit(frame, 'input-stick-x', stick_x)
      if stick_y != prev_stick_y:
        edits.edit(frame, 'input-stick-y', stick_y)

      prev_buttons = buttons
      prev_stick_x = stick_x
      prev_stick_y = stick_y
      frame += 1

    return (metadata, edits)
