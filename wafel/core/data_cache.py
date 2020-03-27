from typing import *
import sys

from wafel.core.data_path import DataPath
from wafel.util import *


class DataCache:
  def __init__(self) -> None:
    self.rows: Dict[int, Dict[object, object]] = {}
    self.path_ids: Dict[object, int] = {}
    self.paths: Dict[object, DataPath] = {}
    self.max_num_rows = 200
    self.warned = False

  def path_key(self, path: DataPath) -> Optional[object]:
    key = path._cache_key
    if key is not None:
      return key

    type_ = path.concrete_end_type
    if type_['kind'] == 'primitive':
      info = (path.addr_path, type_['name'])
    elif type_['kind'] == 'pointer':
      info = (path.addr_path, 'pointer')
    else:
      if not self.warned:
        log.warn('Cache could not save:', type_)
        self.warned = True
      return None

    key = self.path_ids.setdefault(info, max(self.path_ids.values(), default=0) + 1)

    path._cache_key = key
    self.paths[key] = path
    return key

  def get(self, frame: int, path: DataPath) -> Optional[object]:
    try:
      row = self.rows[frame]
      key = self.path_key(path)
      return row[key]
    except KeyError:
      return None

  def put(self, frame: int, path: DataPath, value: object) -> None:
    key = self.path_key(path)
    if key is None:
      return
    row = self.rows.get(frame)
    if row is None:
      row = {}
    else:
      del self.rows[frame]
    self.rows[frame] = row
    row[key] = value
    self.shrink_if_necessary()

  def __contains__(self, frame: int) -> bool:
    return frame in self.rows

  def get_paths_to_prime(self) -> Iterable[DataPath]:
    return self.paths.values()

  def get_size(self) -> int:
    return sys.getsizeof(self.rows)

  def shrink_if_necessary(self) -> None:
    if len(self.rows) <= self.max_num_rows:
      return
    frames = list(self.rows)
    for frame in frames[:len(frames) - self.max_num_rows]:
      del self.rows[frame]

  def invalidate(self, frame: int) -> None:
    for cached_frame in list(self.rows):
      if cached_frame >= frame:
        del self.rows[cached_frame]
