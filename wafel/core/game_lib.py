import ctypes as C
from typing import *
import sys
import weakref

from wafel.util import *
from wafel.core.object_type import ObjectType
from wafel.core.game_state import StateSlot, RelativeAddr, AbsoluteAddr


DataSpec = Any


# TODO: Improve this abstraction - maybe copy dll and allow loading multiple times for different base states?
# Then could revert to old simpler API


class GameLib:
  def __init__(self, spec: DataSpec, dll: C.CDLL) -> None:
    self.spec = spec
    self.dll = dll
    self._buffers: Dict[int, object] = {}

    # TODO: Maybe allow .data and .bss to be stored separately to save memory
    def section_range(name):
      section = self.spec['sections'][name]
      addr = self.dll._handle + section['virtual_address']
      return range(addr, addr + section['virtual_size'])
    base_addr_ranges = [
      section_range('.data'),
      section_range('.bss'),
    ]
    self._base_slot = StateSlot(base_addr_ranges, None)
    self._base_slot.frame = -1

    self._symbols_by_offset = self._build_symbols_by_offset()

    self.dll.sm64_init()

  def _build_symbols_by_offset(self) -> Dict[RelativeAddr, str]:
    result = {}
    for symbol in self.spec['globals']:
      try:
        offset = self.symbol_addr(symbol)
      except ValueError:
        continue
      result[offset] = symbol
    return result

  def symbol_addr(self, symbol: str) -> RelativeAddr:
    addr = C.addressof(C.c_uint32.in_dll(self.dll, symbol))
    return self.base_slot().addr_to_relative(addr)

  def symbol_for_addr(self, rel_addr: RelativeAddr) -> str:
    return self._symbols_by_offset[rel_addr]

  def string(self, addr: AbsoluteAddr) -> str:
    return C.string_at(addr.addr).decode('utf-8')

  @overload
  def concrete_type(self, type_: None) -> None:
    ...
  @overload
  def concrete_type(self, type_: dict) -> dict:
    ...
  def concrete_type(self, type_):
    if type_ is None:
      return None
    while type_['kind'] == 'symbol':
      type_ = self.spec['types'][type_['namespace']][type_['name']]
    return type_


  def base_slot(self) -> StateSlot:
    return self._base_slot

  def alloc_slot(self) -> StateSlot:
    base_slot = self.base_slot()

    addr_ranges = []
    for base_addr_range in base_slot.addr_ranges:
      buffer = C.create_string_buffer(len(base_addr_range))
      addr = C.addressof(buffer)
      self._buffers[addr] = buffer
      addr_ranges.append(range(addr, addr + len(base_addr_range)))

    return StateSlot(addr_ranges, base_slot)

  def dealloc_slot(self, slot: StateSlot) -> None:
    if not slot.based:
      for addr_range in slot.addr_ranges:
        del self._buffers[addr_range.start]

  def raw_copy_slot(self, dst: StateSlot, src: StateSlot) -> None:
    if dst is not src:
      for dst_range, src_range in zip(dst.addr_ranges, src.addr_ranges):
        C.memmove(dst_range.start, src_range.start, len(dst_range))

  def execute_frame(self) -> None:
    self.dll.sm64_update()
