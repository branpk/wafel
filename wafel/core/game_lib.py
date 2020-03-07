import ctypes as C
from typing import *
import sys
import weakref

from wafel.util import *
from wafel.core.object_type import ObjectType
from wafel.core.game_state import Slot


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
    return C.addressof(C.c_uint32.in_dll(self.dll, symbol)) - self.dll._handle

  def symbol_for_offset(self, offset: int) -> str:
    return self._symbols_by_offset[offset]

  def concrete_type(self, type_: dict) -> dict:
    while type_['kind'] == 'symbol':
      type_ = self.spec['types'][type_['namespace']][type_['name']]
    return type_


  def slot_size(self) -> int:
    return cast(int, max(r.stop for r in self.state_ranges))

  def base_slot(self) -> Slot:
    # TODO: Rename to create_base_slot() and only allow calling once
    base_slot = Slot(addr=self.dll._handle, size=self.slot_size(), base_slot=None)
    base_slot.frame = -1
    return base_slot

  def alloc_slot(self) -> Slot:
    contiguous_state_range = range(
      min(r.start for r in self.state_ranges),
      max(r.stop for r in self.state_ranges),
    )
    buffer = C.create_string_buffer(len(contiguous_state_range))
    addr = C.addressof(buffer) - contiguous_state_range.start
    self._buffers[addr] = buffer
    return Slot(addr, self.slot_size(), self.base_slot())

  def dealloc_slot(self, slot: Slot) -> None:
    if not slot.based:
      del self._buffers[slot.addr]

  def raw_copy_slot(self, dst: Slot, src: Slot) -> None:
    for state_range in self.state_ranges:
      C.memmove(dst.addr + state_range.start, src.addr + state_range.start, len(state_range))

  def execute_frame(self) -> None:
    self.dll.sm64_update()
