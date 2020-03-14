from typing import *
import json
import os
import sys

from dataclasses import dataclass

from wafel.util import assert_not_none, log


version = (0, 1, 1)

dev_mode: bool
assets_directory: str
lib_directory: str
cache_directory: str
log_file: str


def version_str(delim: str) -> str:
  return delim.join(map(str, version))

def _load_cache_index() -> Any:
  cache_index = os.path.join(cache_directory, 'index.json')
  if os.path.exists(cache_index):
    with open(cache_index, 'r') as f:
      return json.load(f)
  else:
    return {}

def _save_cache_index(index: object) -> None:
  os.makedirs(cache_directory, exist_ok=True)
  cache_index = os.path.join(cache_directory, 'index.json')
  with open(cache_index, 'w') as f:
    json.dump(index, f, indent=2)

def _gen_cache_filename(format: str) -> str:
  filename = format.replace('*', '')
  if not os.path.exists(os.path.join(cache_directory, filename)):
    return filename
  k = 1
  while True:
    filename = format.replace('*', '_' + str(k))
    if not os.path.exists(os.path.join(cache_directory, filename)):
      return filename
    k += 1

# TODO: Allow binary format option instead of json

def cache_get(key: str) -> Optional[Any]:
  index = _load_cache_index()

  filename = index.get(key)
  if filename is None:
    return None

  try:
    with open(os.path.join(cache_directory, filename), 'r') as f:
      return json.load(f)
  except Exception as e:
    log.warn(f'Error while reading from cache: {e}')
    return None

def cache_put(key: str, value: object, filename='object*') -> None:
  filename_format = filename
  assert '*' in filename_format

  index = _load_cache_index()

  filename = index.get(key)
  if filename is None:
    filename = _gen_cache_filename(filename_format)

  try:
    os.makedirs(cache_directory, exist_ok=True)
    with open(os.path.join(cache_directory, filename), 'w') as f:
      json.dump(value, f, indent=2)
    index[key] = filename
    _save_cache_index(index)
  except Exception as e:
    log.warn(f'Error while writing to cache: {e}')
