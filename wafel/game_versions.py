from typing import *
from glob import glob
import os
import re
import traceback

import wafel_core

import wafel.config as config
import wafel.imgui as ig
from wafel.util import *
from wafel.log import *
from wafel.local_state import use_state


def unlock_game_version(
  version: str,
  rom_filename: str,
  locked_filename: str,
  unlocked_filename: str,
) -> None:
  log.info(f'Unlocking game version {version}')
  wafel_core.unlock_libsm64(locked_filename, unlocked_filename, rom_filename)


def lock_game_version(
  version: str,
  rom_filename: str,
  unlocked_filename: str,
  locked_filename: str,
) -> None:
  log.info(f'Locking game version {version}')
  wafel_core.lock_libsm64(unlocked_filename, locked_filename, rom_filename)


def get_game_version(filename: str) -> str:
  _, base = os.path.split(filename)
  match = assert_not_none(re.match(r'^sm64_([^.]+)', base))
  return match.group(1).upper()

def find_locked_dlls() -> Dict[str, str]:
  return {
    get_game_version(filename): filename
      for filename in glob(os.path.join(config.lib_directory, 'libsm64', 'sm64_*.dll.locked'))
  }

def find_unlocked_dlls() -> Dict[str, str]:
  return {
    get_game_version(filename): filename
      for filename in glob(os.path.join(config.lib_directory, 'libsm64', 'sm64_*.dll'))
  }

def unlocked_game_versions() -> List[str]:
  return sorted(find_unlocked_dlls())

def locked_game_versions() -> List[str]:
  return sorted(set(find_locked_dlls()).difference(unlocked_game_versions()))


def render_game_version_menu(
  id: str,
  select_rom_filename: Callable[[], Optional[str]],
) -> None:
  ig.push_id(id)

  ig.text('Wafel requires a vanilla SM64 ROM to run')

  locked_dlls = find_locked_dlls()
  unlocked_dlls = find_unlocked_dlls()
  versions = sorted(set(locked_dlls.keys()).union(unlocked_dlls.keys()))

  ig.dummy(1, 5)

  error_msgs: Dict[str, str] = use_state('error-msgs', cast(Dict[str, str], dict())).value
  for version in unlocked_dlls:
    if version in error_msgs:
      del error_msgs[version]

  for version in versions:
    ig.push_id('version-' + version)

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

    ig.pop_id()

  ig.separator()
  ig.dummy(1, 5)
  ig.pop_id()


__all__ = [
  'render_game_version_menu',
  'unlocked_game_versions',
  'locked_game_versions',
]
