import ctypes as C
from typing import *
import sys

from wafel.util import *
from wafel.core.object_type import ObjectType


DataSpec = Any


# TODO: Improve this abstraction - maybe copy dll and allow loading multiple times for different base states?
# Then could revert to old simpler API


class GameLib:
  def __init__(self, spec: DataSpec, dll: C.CDLL) -> None:
    self.spec = spec
    self.dll = dll
    self._buffers: Dict[int, object] = {}
    self._symbols_by_offset: Dict[int, str] = self._build_symbols_by_offset()

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

    self.dll.sm64_init()

  def _build_symbols_by_offset(self) -> Dict[int, str]:
    result: Dict[int, str] = {}
    for symbol in self.spec['globals']:
      try:
        offset = self.symbol_offset(symbol)
      except ValueError:
        continue
      assert offset not in result, symbol
      result[offset] = symbol
    return result

  def symbol_offset(self, symbol: str) -> int:
    return C.addressof(C.c_uint32.in_dll(self.dll, symbol)) - self.base_state()

  def symbol_for_offset(self, offset: int) -> str:
    return self._symbols_by_offset[offset]

  def base_state(self) -> int:
    return self.dll._handle

  def base_state_range(self) -> range:
    return range(self.base_state(), self.base_state() + max(r.stop for r in self.state_ranges))

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
    if addr != self.base_state():
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
