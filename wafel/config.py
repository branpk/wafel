from typing import Optional, Any, IO
import json
import os
import sys

from dataclasses import dataclass

from wafel.util import assert_not_none


@dataclass
class Config:
  _lib_directory: Optional[str] = None
  _cache_directory: Optional[str] = None

  @property
  def lib_directory(self) -> str:
    return assert_not_none(self._lib_directory)

  @property
  def cache_directory(self) -> str:
    return assert_not_none(self._cache_directory)

  def _load_cache_index(self) -> Any:
    cache_index = os.path.join(self.cache_directory, 'index.json')
    if os.path.exists(cache_index):
      with open(cache_index, 'r') as f:
        return json.load(f)
    else:
      return {}

  def _save_cache_index(self, index: Any) -> Any:
    os.makedirs(self.cache_directory, exist_ok=True)
    cache_index = os.path.join(self.cache_directory, 'index.json')
    with open(cache_index, 'w') as f:
      return json.dump(index, f, indent=2)

  def _gen_cache_filename(self, format: str) -> str:
    filename = format.replace('*', '')
    if not os.path.exists(os.path.join(self.cache_directory, filename)):
      return filename
    k = 1
    while True:
      filename = format.replace('*', '_' + str(k))
      if not os.path.exists(os.path.join(self.cache_directory, filename)):
        return filename
      k += 1

  # TODO: Allow binary format option instead of json

  def cache_get(self, key: str) -> Optional[Any]:
    index = self._load_cache_index()

    filename = index.get(key)
    if filename is None:
      return None

    try:
      with open(os.path.join(self.cache_directory, filename), 'r') as f:
        return json.load(f)
    except Exception as e:
      sys.stderr.write(f'Warning: Error while reading from cache: {e}\n')
      sys.stderr.flush()
      return None

  def cache_put(self, key: str, value: object, filename='object*') -> None:
    filename_format = filename
    assert '*' in filename_format

    index = self._load_cache_index()

    filename = index.get(key)
    if filename is None:
      filename = self._gen_cache_filename(filename_format)

    try:
      os.makedirs(self.cache_directory, exist_ok=True)
      with open(os.path.join(self.cache_directory, filename), 'w') as f:
        json.dump(value, f, indent=2)
      index[key] = filename
      self._save_cache_index(index)
    except Exception as e:
      sys.stderr.write(f'Warning: Error while writing to cache: {e}')
      sys.stderr.flush()


config = Config()
