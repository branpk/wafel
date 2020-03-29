from __future__ import annotations

from typing import *
import ctypes as C
from abc import ABC, abstractmethod
from enum import Enum, auto
from dataclasses import dataclass

from wafel.core_new.data_spec import DataSpec, spec_get_concrete_type
from wafel.util import *


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
    assert addr != 0
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


PRIMITIVE_C_TYPES = {
  'u8': C.c_uint8,
  's8': C.c_int8,
  'u16': C.c_uint16,
  's16': C.c_int16,
  'u32': C.c_uint32,
  's32': C.c_int32,
  'u64': C.c_uint64,
  's64': C.c_int64,
  'f32': C.c_float,
  'f64': C.c_double,
}

PRIMITIVE_PY_TYPES = {
  'u8': int,
  's8': int,
  'u16': int,
  's16': int,
  'u32': int,
  's32': int,
  'u64': int,
  's64': int,
  'f32': float,
  'f64': float,
}


# TODO: Move?
class Slot:
  pass


SLOT = TypeVar('SLOT', bound=Slot)


class Memory(ABC, Generic[VADDR, SLOT]):
  @property
  @abstractmethod
  def data_spec(self) -> DataSpec: ...

  @abstractmethod
  def symbol(self, name: str) -> Address[VADDR]: ...

  @abstractmethod
  def get_s8_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_s16_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_s32_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_s64_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u8_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u16_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u32_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_u64_virtual(self, slot: SLOT, addr: VADDR) -> int: ...
  @abstractmethod
  def get_f32_virtual(self, slot: SLOT, addr: VADDR) -> float: ...
  @abstractmethod
  def get_f64_virtual(self, slot: SLOT, addr: VADDR) -> float: ...
  @abstractmethod
  def get_pointer_virtual(self, slot: SLOT, addr: VADDR) -> Address[VADDR]: ...

  @abstractmethod
  def set_s8_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_s16_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_s32_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_s64_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u8_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u16_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u32_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_u64_virtual(self, slot: SLOT, addr: VADDR, value: int) -> None: ...
  @abstractmethod
  def set_f32_virtual(self, slot: SLOT, addr: VADDR, value: float) -> None: ...
  @abstractmethod
  def set_f64_virtual(self, slot: SLOT, addr: VADDR, value: float) -> None: ...
  @abstractmethod
  def set_pointer_virtual(self, slot: SLOT, addr: VADDR, value: Address[VADDR]) -> None: ...

  def get_pointer_absolute(self, addr: int) -> Address[VADDR]:
    pointer = C.cast(addr, C.POINTER(C.c_void_p)) # type: ignore
    pointer_value =  int(pointer[0] or 0) # type: ignore
    if pointer_value == 0:
      return Address.new_null()
    else:
      return Address.new_absolute(pointer_value)

  def get_pointer(self, slot: SLOT, addr: Address[VADDR]) -> Optional[Address[VADDR]]:
    if addr.type == AddressType.NULL:
      return None
    elif addr.type == AddressType.ABSOLUTE:
      return self.get_pointer_absolute(addr.absolute)
    elif addr.type == AddressType.VIRTUAL:
      return self.get_pointer_virtual(slot, addr.virtual)
    else:
      raise NotImplementedError(addr.type)

  def get_absolute(self, addr: int, type_: dict) -> object:
    type_ = spec_get_concrete_type(self.data_spec, type_)

    if type_['kind'] == 'primitive':
      if type_['name'] == 'void':
        raise Exception('Dereferencing void at ' + str(addr))
      ctype = PRIMITIVE_C_TYPES[type_['name']]
      pytype = PRIMITIVE_PY_TYPES[type_['name']]
      pointer = C.cast(addr, C.POINTER(ctype)) # type: ignore
      return pytype(pointer[0])

    elif type_['kind'] == 'pointer':
      return self.get_pointer_absolute(addr)

    elif type_['kind'] == 'array':
      length = type_['length']
      if length is None:
        raise Exception('Cannot fetch an array with null length: ' + str(type_))
      element_type = type_['base']
      stride = align_up(element_type['size'], element_type['align'])
      return [
        self.get_absolute(addr + stride * i, element_type)
          for i in range(length)
      ]

    elif type_['kind'] == 'struct':
      result = {}
      for field_name, field in type_['fields'].items():
        result[field_name] = self.get_absolute(addr + field['offset'], field['type'])
      return result

    else:
      raise NotImplementedError(type_['kind'])

  def get_virtual(self, slot: SLOT, addr: VADDR, type_: dict) -> object:
    type_ = spec_get_concrete_type(self.data_spec, type_)

    if type_['kind'] == 'primitive':
      if type_['name'] == 'void':
        raise Exception('Dereferencing void at ' + str(addr))
      method = {
        's8': self.get_s8_virtual,
        's16': self.get_s16_virtual,
        's32': self.get_s32_virtual,
        's64': self.get_s64_virtual,
        'u8': self.get_u8_virtual,
        'u16': self.get_u16_virtual,
        'u32': self.get_u32_virtual,
        'u64': self.get_u64_virtual,
        'f32': self.get_f32_virtual,
        'f64': self.get_f64_virtual,
      }[type_['name']]
      return method(slot, addr)

    elif type_['kind'] == 'pointer':
      return self.get_pointer_virtual(slot, addr)

    elif type_['kind'] == 'array':
      length = type_['length']
      if length is None:
        raise Exception('Cannot fetch an array with null length: ' + str(type_))
      element_type = type_['base']
      stride = align_up(element_type['size'], element_type['align'])
      return [
        self.get_virtual(slot, cast(VADDR, addr + stride * i), element_type)
          for i in range(length)
      ]

    elif type_['kind'] == 'struct':
      result = {}
      for field_name, field in type_['fields'].items():
        result[field_name] = self.get_virtual(slot, addr + field['offset'], field['type'])
      return result

    else:
      raise NotImplementedError(type_['kind'])

  def get(self, slot: SLOT, addr: Address, type_: dict) -> object:
    if addr.type == AddressType.NULL:
      return None
    elif addr.type == AddressType.ABSOLUTE:
      return self.get_absolute(addr.absolute, type_)
    elif addr.type == AddressType.VIRTUAL:
      return self.get_virtual(slot, addr.virtual, type_)
    else:
      raise NotImplementedError(addr.type)


__all__ = [
  'VirtualAddress',
  'AddressType',
  'Address',
  'Slot',
  'Memory',
]
