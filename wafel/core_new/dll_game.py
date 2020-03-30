from __future__ import annotations

from typing import *
import ctypes as C
import os
from dataclasses import dataclass

from wafel.core_new.data_spec import DataSpec
from wafel.core_new.dll_data_spec import load_dll_data_spec
from wafel.core_new.game import Game, GameImpl
from wafel.core_new.memory import AccessibleMemory, Slot, VirtualAddress, Address
from wafel.loading import Loading, in_progress, load_child
from wafel.util import *


@dataclass(frozen=True)
class DLLVirtual(VirtualAddress):
  section: int
  offset: int

  def __add__(self, offset: int) -> DLLVirtual:
    return DLLVirtual(self.section, self.offset + offset)


@dataclass(frozen=True, eq=False)
class DLLSlot(Slot):
  addr_ranges: List[range]


class DLLMemory(AccessibleMemory[DLLVirtual, DLLSlot]):
  def __init__(self, dll: C.CDLL, data_spec: DataSpec, base_slot: DLLSlot) -> None:
    self._dll = dll
    self._data_spec = data_spec
    self._base_slot = base_slot

  @property
  def data_spec(self) -> DataSpec:
    return self._data_spec

  def symbol(self, name: str) -> Address[DLLVirtual]:
    location = C.addressof(C.c_uint32.in_dll(self._dll, name)) # TODO: Return NULL if not found
    virtual = self.stored_location_to_virtual(location)
    if virtual is not None:
      return Address.new_virtual(virtual)
    else:
      return Address.new_absolute(location)

  def stored_location_to_virtual(self, location: int) -> Optional[DLLVirtual]:
    for i, addr_range in enumerate(self._base_slot.addr_ranges):
      if location in addr_range:
        return DLLVirtual(i, location - addr_range.start)
    return None

  def virtual_to_location(self, slot: DLLSlot, virtual: DLLVirtual) -> int:
    return slot.addr_ranges[virtual.section].start + virtual.offset

  def virtual_to_storable_location(self, virtual: DLLVirtual) -> int:
    return self.virtual_to_location(self._base_slot, virtual)


class DLLGame(GameImpl[DLLVirtual, DLLSlot]):
  def __init__(
    self,
    dll: C.CDLL,
    data_spec: DataSpec,
    init_func: str,
    update_func: str,
  ) -> None:
    self._dll = dll
    self._data_spec = data_spec
    self._buffers: Dict[int, object] = {}

    def section_range(name: str) -> range:
      section = self._data_spec['sections'][name]
      addr = self._dll._handle + section['virtual_address']
      return range(addr, addr + section['virtual_size'])
    self._base_slot = DLLSlot([section_range('.data'), section_range('.bss')])

    self._memory = DLLMemory(self._dll, self._data_spec, self._base_slot)

    getattr(self._dll, init_func)()
    self._update_func = getattr(self._dll, update_func)

  @property
  def base_slot(self) -> DLLSlot:
    return self._base_slot

  def alloc_slot(self) -> DLLSlot:
    addr_ranges = []
    for base_addr_range in self._base_slot.addr_ranges:
      buffer = C.create_string_buffer(len(base_addr_range))
      addr = C.addressof(buffer)
      self._buffers[addr] = buffer
      addr_ranges.append(range(addr, addr + len(base_addr_range)))
    return DLLSlot(addr_ranges)

  def dealloc_slot(self, slot: DLLSlot) -> None:
    if slot is not self._base_slot:
      for addr_range in slot.addr_ranges:
        del self._buffers[addr_range.start]

  def copy_slot(self, dst: DLLSlot, src: DLLSlot) -> None:
    if dst is not src:
      for dst_range, src_range in zip(dst.addr_ranges, src.addr_ranges):
        C.memmove(dst_range.start, src_range.start, len(dst_range))

  @property
  def memory(self) -> DLLMemory:
    return self._memory

  def run_frame(self) -> None:
    self._update_func()


def load_dll_game(path: str, init_func: str, update_func: str) -> Loading[Game]:
  filename = os.path.basename(path)
  log.info('Loading', filename)

  dll = C.cdll.LoadLibrary(path)

  status = f'Loading {filename}'
  yield in_progress(0.0, status)

  spec = yield from load_child(
    0.0, 0.95, status, load_dll_data_spec(path),
  )

  yield in_progress(1.0, status)
  log.info('Done loading', filename)

  return DLLGame(dll, spec, init_func, update_func).remove_type_vars()


__all__ = ['load_dll_game']
