from typing import *
from glob import glob
import os
import re
import base64
import struct
import traceback

from cryptography.fernet import Fernet
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

import wafel.config as config
import wafel.imgui as ig
from wafel.util import *
from wafel.log import *
from wafel.local_state import use_state


def get_key(version_id: bytes, rom: bytes) -> bytes:
  kdf = PBKDF2HMAC( # type: ignore
    algorithm = hashes.SHA256(),
    length = 32,
    salt = b'',
    iterations = 10_000,
  )
  key = kdf.derive(version_id + rom)
  return base64.urlsafe_b64encode(key)

def encrypt(version_id: bytes, rom: bytes, plain_dll: bytes) -> bytes:
  key = get_key(version_id, rom)
  return Fernet(key).encrypt(plain_dll)

def decrypt(version_id: bytes, rom: bytes, cipher_dll: bytes) -> bytes:
  key = get_key(version_id, rom)
  return Fernet(key).decrypt(cipher_dll)


def swap16(b: bytes) -> bytes:
  length = len(b) // 2
  ints = struct.unpack('<' + str(length) + 'H', b)
  return struct.pack('>' + str(length) + 'H', *ints)

def swap32(b: bytes) -> bytes:
  length = len(b) // 4
  ints = struct.unpack('<' + str(length) + 'I', b)
  return struct.pack('>' + str(length) + 'I', *ints)

def rom_to_z64(rom: bytes) -> bytes:
  head = struct.unpack('>I', rom[:4])[0]
  swap_fn: Callable[[bytes], bytes] = {
    0x80371240: lambda b: b,
    0x37804012: swap16,
    0x40123780: swap32,
  }[head]
  return swap_fn(rom)


def unlock_game_version(
  version: str,
  rom_filename: str,
  locked_filename: str,
  unlocked_filename: str,
) -> None:
  log.info(f'Unlocking game version {version}')

  version_id = ('wafel-' + config.version_str('.')).encode('utf-8')
  with open(rom_filename, 'rb') as f:
    rom = rom_to_z64(f.read())
  with open(locked_filename, 'rb') as f:
    cipher_dll = f.read()

  plain_dll = decrypt(version_id, rom, cipher_dll)

  with open(unlocked_filename, 'wb') as f:
    f.write(plain_dll)


def lock_game_version(
  version: str,
  rom_filename: str,
  unlocked_filename: str,
  locked_filename: str,
) -> None:
  log.info(f'Locking game version {version}')

  version_id = ('wafel-' + config.version_str('.')).encode('utf-8')
  with open(rom_filename, 'rb') as f:
    rom = rom_to_z64(f.read())
  with open(unlocked_filename, 'rb') as f:
    plain_dll = f.read()

  cipher_dll = encrypt(version_id, rom, plain_dll)

  with open(locked_filename, 'wb') as f:
    f.write(cipher_dll)


def get_game_version(filename: str) -> str:
  _, base = os.path.split(filename)
  match = assert_not_none(re.match(r'^sm64_([^.]+)', base))
  return match.group(1).upper()


def render_game_version_menu(
  id: str,
  select_rom_filename: Callable[[], Optional[str]],
) -> None:
  ig.push_id(id)

  ig.text('Wafel requires a vanilla SM64 ROM to run')

  locked_dlls = {
    get_game_version(filename): filename
      for filename in glob(os.path.join(config.lib_directory, 'libsm64', 'sm64_*.dll.locked'))
  }
  unlocked_dlls = {
    get_game_version(filename): filename
      for filename in glob(os.path.join(config.lib_directory, 'libsm64', 'sm64_*.dll'))
  }

  versions = list(set(locked_dlls.keys()).union(unlocked_dlls.keys()))
  versions.sort()

  ig.dummy(1, 5)

  error_msgs: Dict[str, str] = use_state('error-msgs', cast(Dict[str, str], dict())).value
  for version in unlocked_dlls:
    if version in error_msgs:
      del error_msgs[version]

  for version in versions:
    ig.separator()
    locked = version not in unlocked_dlls
    ig.text('SM64 ' + version + ' - ' + ('locked' if locked else 'unlocked'))

    if locked:
      ig.same_line()
      ig.dummy(3, 1)
      ig.same_line()

      if ig.button('Select ROM'):
        rom_filename = select_rom_filename()
        if rom_filename is not None:
          locked_filename = locked_dlls[version]
          unlocked_filename = locked_filename[:-len('.locked')]
          try:
            unlock_game_version(version, rom_filename, locked_filename, unlocked_filename)
          except:
            error = traceback.format_exc()
            log.error('Failed to unlock game version ' + version + ': ' + error)
            error_msgs[version] = 'Error: ROM did not match vanilla ' + version + ' ROM'

    elif config.dev_mode:
      ig.same_line()
      ig.dummy(3, 1)
      ig.same_line()

      if ig.button('Lock'):
        rom_filename = select_rom_filename()
        if rom_filename is not None:
          unlocked_filename = unlocked_dlls[version]
          locked_filename = unlocked_filename + '.locked'
          lock_game_version(version, rom_filename, unlocked_filename, locked_filename)

    error_msg = error_msgs.get(version)
    if error_msg is not None:
      ig.text(error_msg)

  ig.separator()
  ig.dummy(1, 5)
  ig.pop_id()


__all__ = ['render_game_version_menu']
