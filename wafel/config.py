from typing import *
import json
import os
import sys

from dataclasses import dataclass

from wafel.util import assert_not_none, log


# Important: be sure to re-run `python build.py lock` after changing!
version = (0, 8, 4)

dev_mode: bool
assets_directory: str
lib_directory: str
log_file: str
settings_file: str


def version_str(delim: str) -> str:
  return delim.join(map(str, version))

def init() -> None:
  global dev_mode, assets_directory, lib_directory, log_file, settings_file
  if getattr(sys, 'frozen', False):
    dev_mode = False
    root_dir = os.path.dirname(sys.executable)
    lib_directory = root_dir
  else:
    dev_mode = '--nodev' not in sys.argv
    root_dir = os.getcwd()
    lib_directory = root_dir

  assets_directory = os.path.join(root_dir, 'assets')
  log_file = os.path.join(root_dir, 'log.txt')
  settings_file = os.path.join(root_dir, 'settings.json')

  import wafel.bindings as bindings
  bindings.init()

class _Settings:
  def _read_settings(self) -> Dict[str, Any]:
    if os.path.exists(settings_file):
      with open(settings_file, 'r') as f:
        return cast(Dict[str, Any], json.load(f))
    else:
      return {}

  def _save_settings(self, settings: Dict[str, Any]) -> None:
    with open(settings_file, 'w') as f:
      json.dump(settings, f, indent=2)

  def get(self, key: str) -> Optional[Any]:
    return self._read_settings().get(key)

  def __setitem__(self, key: str, value: Any) -> None:
    settings = self._read_settings()
    settings[key] = value
    self._save_settings(settings)

settings = _Settings()
