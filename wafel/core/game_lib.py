import ctypes as C
from typing import *
import sys

from wafel.util import *
from wafel.core.object_type import ObjectType, OBJECT_TYPES


DataSpec = Any


class GameLib:
  def __init__(self, spec: DataSpec, dll: C.CDLL) -> None:
    self.spec = spec
    self.dll = dll
    self._buffers: Dict[int, object] = {}
    self._cache_object_types()

    # TODO: Maybe allow .data and .bss to be stored separately to save memory
    def section_range(name):
      section = self.spec['sections'][name]
      return range(
        section['virtual_address'],
        section['virtual_address'] + section['virtual_size'],
      )
    self.state_ranges = [
      section_range('.data'),
      section_range('.bss'),
    ]

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

  def base_state(self) -> int:
    return self.dll._handle

  def alloc_state_buffer(self) -> int:
    contiguous_state_range = range(
      min(r.start for r in self.state_ranges),
      max(r.stop for r in self.state_ranges),
    )
    buffer = C.create_string_buffer(len(contiguous_state_range))
    addr = C.addressof(buffer) - contiguous_state_range.start
    self._buffers[addr] = buffer
    return addr

  def dealloc_state_buffer(self, addr: int) -> None:
    del self._buffers[addr]

  def raw_copy_state(self, dst: int, src: int) -> None:
    for state_range in self.state_ranges:
      C.memmove(dst + state_range.start, src + state_range.start, len(state_range))

  def execute_frame(self) -> None:
    self.dll.sm64_update()

  def concrete_type(self, type_: dict) -> dict:
    while type_['kind'] == 'symbol':
      type_ = self.spec['types'][type_['namespace']][type_['name']]
    return type_

  def get_object_type(self, behavior: int) -> ObjectType:
    return self.object_types[behavior]
