from typing import *
import sys

from wafel.core.data_path import DataPath
from wafel.util import *


class DataCache:
  def __init__(self) -> None:
    self.rows: Dict[int, Dict[object, object]] = {}
    self.max_num_rows = 100
    self.warned = False

  def path_key(self, path: DataPath) -> Optional[object]:
    type_ = path.concrete_end_type
    if type_['kind'] == 'primitive':
      return (path.addr_path, type_['name'])
    elif type_['kind'] == 'pointer':
      return (path.addr_path, 'pointer')
    else:
      if not self.warned:
        log.warn('Cache could not save:', type_)
        self.warned = True
      return None

  def get(self, frame: int, path: DataPath) -> Maybe[object]:
    row = self.rows.get(frame)
    if row is None:
      return None
    key = self.path_key(path)
    if key is None:
      return None
    value = row.get(key)
    if value is None:
      return None
    return Just(value)

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
