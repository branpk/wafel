from __future__ import annotations

from typing import *
from dataclasses import dataclass
from enum import Enum, auto

from abc import ABC, abstractmethod


class VirtualAddress(ABC):
  @abstractmethod
  def __add__(self, offset: int) -> VirtualAddress: ...


VADDR = TypeVar('VADDR', bound=VirtualAddress)


class AddressType(Enum):
  NULL = auto()
  ABSOLUTE = auto()
  VIRTUAL = auto()


@dataclass(frozen=True)
class Address(Generic[VADDR]):
  type: AddressType
  _absolute: Optional[int] = None
  _virtual: Optional[VADDR] = None

  @staticmethod
  def new_null() -> Address[VADDR]:
    return Address(type=AddressType.NULL)

  @staticmethod
  def new_absolute(addr: int) -> Address[VADDR]:
    return Address(type=AddressType.ABSOLUTE, _absolute=addr)

  @staticmethod
  def new_virtual(addr: VADDR) -> Address[VADDR]:
    return Address(type=AddressType.VIRTUAL, _virtual=addr)

  @property
  def absolute(self) -> int:
    assert self._absolute is not None
    return self._absolute

  @property
  def virtual(self) -> VADDR:
    assert self._virtual is not None
    return self._virtual

  def __add__(self, offset: int) -> Address[VADDR]:
    if self.type == AddressType.NULL:
      return self
    elif self.type == AddressType.ABSOLUTE:
      return Address.new_absolute(self.absolute + offset)
    elif self.type == AddressType.VIRTUAL:
      return Address.new_virtual(cast(VADDR, self.virtual + offset))
    else:
      raise NotImplementedError(self.type)

  def __str__(self) -> str:
    if self.type == AddressType.NULL:
      return 'null'
    elif self.type == AddressType.ABSOLUTE:
      return 'absolute(0x%X)' % self.absolute
    elif self.type == AddressType.VIRTUAL:
      return f'virtual({self.virtual})'
    else:
      raise NotImplementedError(self.type)


class Slot:
  pass


SLOT = TypeVar('SLOT', bound=Slot)


DataSpec = dict


class MemoryAccess(ABC, Generic[VADDR, SLOT]):
  @abstractmethod
  def get_s8(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_s16(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_s32(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_s64(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u8(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u16(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u32(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u64(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_f32(self, slot: SLOT, addr: VADDR) -> float: ...
  @abstractmethod
  def get_f64(self, slot: SLOT, addr: VADDR) -> float: ...
  @abstractmethod
  def get_pointer(self, slot: SLOT, addr: VADDR) -> Address[VADDR]: ...

  @abstractmethod
  def set_s8(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_s16(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_s32(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_s64(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u8(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u16(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u32(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u64(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_f32(self, slot: SLOT, addr: VADDR, value: float) -> None: ...
  @abstractmethod
  def set_f64(self, slot: SLOT, addr: VADDR, value: float) -> None: ...
  @abstractmethod
  def set_pointer(self, slot: SLOT, addr: VADDR, value: Address[VADDR]) -> None: ...


class GameImpl(ABC, Generic[VADDR, SLOT]):
  def build(self) -> Game:
    return cast(Game, self)

  # Slot management

  @property
  @abstractmethod
  def base_slot(self) -> SLOT: ...

  @abstractmethod
  def alloc_slot(self) -> SLOT: ...

  @abstractmethod
  def dealloc_slot(self, slot: SLOT) -> None: ...

  @abstractmethod
  def copy_slot(self, dst: SLOT, src: SLOT) -> None: ...

  # Data access

  @property
  @abstractmethod
  def data_spec(self) -> DataSpec: ...

  @abstractmethod
  def symbol(self, name: str) -> Address[VADDR]: ...

  @property
  @abstractmethod
  def memory(self) -> MemoryAccess[VADDR, SLOT]: ...

  # Execution

  @abstractmethod
  def run_frame(self) -> None: ...


Game = GameImpl[VirtualAddress, Slot]


__all__ = [
  'VirtualAddress',
  'AddressType',
  'Address',
  'Slot',
  'DataSpec',
  'MemoryAccess',
  'GameImpl',
  'Game',
]
