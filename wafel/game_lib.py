import ctypes as C
from typing import *
import sys

from wafel.util import *
from wafel.object_type import ObjectType, OBJECT_TYPES


class GameLib:
  def __init__(self, spec: dict, dll: C.CDLL) -> None:
    self.spec = spec
    self.dll = dll
    self._cache_object_types()

  def _cache_object_types(self) -> None:
    self.object_types: Dict[int, ObjectType] = {}
    for type_ in OBJECT_TYPES:
      # TODO: Backup behaviors
      try:
        behavior_addr = C.addressof(C.c_uint32.in_dll(self.dll, type_.behavior[0]))
        assert behavior_addr not in self.object_types, type_.name
        self.object_types[behavior_addr] = type_
      except ValueError:
        sys.stderr.write('Warning: Could not load object type ' + type_.name + '\n')
        sys.stderr.flush()

  def state_new(self) -> int:
    return dcast(int, self.dll.sm64_state_new())

  def state_raw_copy(self, dst: int, src: int) -> None:
    self.dll.sm64_state_raw_copy(dst, src)

  def state_update(self, addr: int) -> None:
    self.dll.sm64_state_update(addr)

  def concrete_type(self, type_: dict) -> dict:
    while type_['kind'] == 'symbol':
      type_ = self.spec['types'][type_['namespace']][type_['name']]
    return type_

  def get_object_type(self, behavior: int) -> ObjectType:
    return self.object_types[behavior]
