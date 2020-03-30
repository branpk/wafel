from __future__ import annotations

from typing import *
import ctypes as C
from abc import ABC, abstractmethod
from enum import Enum, auto
from dataclasses import dataclass

from wafel.core_new.data_spec import DataSpec, spec_get_concrete_type
from wafel.util import *


class Slot(ABC):
  """An abstract memory buffer that can hold all mutable game state."""
  pass


class VirtualAddress(ABC):
  """An abstract address that represents a location in slot memory.

  This should be indepedent of any single slot. For example, it could be an offset
  from the base address of a slot.
  """

  @abstractmethod
  def __add__(self, offset: int) -> VirtualAddress: ...


SLOT = TypeVar('SLOT', bound=Slot)
VADDR = TypeVar('VADDR', bound=VirtualAddress)


class AddressType(Enum):
  NULL = auto()
  ABSOLUTE = auto()
  VIRTUAL = auto()


@dataclass(frozen=True)
class Address(Generic[VADDR]):
  """Represents an address in either static memory or slot memory.

  An absolute address must always point to static/shared memory, i.e. not
  memory owned by a specific slot.
  Both absolute and virtual addresses are intended to be valid (non-NULL) pointers.
  """

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


class Memory(ABC, Generic[VADDR, SLOT]):
  @property
  @abstractmethod
  def data_spec(self) -> DataSpec: ...

  @abstractmethod
  def symbol(self, name: str) -> Address[VADDR]: ...

  @abstractmethod
  def stored_location_to_virtual(self, location: int) -> Optional[VADDR]:
    """Given an arbitrary address, return its virtual address if applicable.

    location is an address that may either be in slot memory or in static memory.
    This method is only called on addresses that it pulls out of memory. Thus if
    your memory is guaranteed to only contain base slot (or static) addresses,
    then you only need to test against the base slot memory range.
    """
    ...

  # Virtual - get

  @abstractmethod
  def get_primitive_virtual(self, slot: SLOT, addr: VADDR, type_: str) -> object: ...

  @abstractmethod
  def get_pointer_virtual(self, slot: SLOT, addr: VADDR) -> Address[VADDR]: ...

  def get_virtual(self, slot: SLOT, addr: VADDR, type_: dict) -> object:
    type_ = spec_get_concrete_type(self.data_spec, type_)

    if type_['kind'] == 'primitive':
      if type_['name'] == 'void':
        raise Exception('Dereferencing void at ' + str(addr))
      return self.get_primitive_virtual(slot, addr, type_['name'])

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

  # Virtual - set

  @abstractmethod
  def set_primitive_virtual(self, slot: SLOT, addr: VADDR, value: object, type_: str) -> None: ...

  @abstractmethod
  def set_pointer_virtual(self, slot: SLOT, addr: VADDR, value: Address[VADDR]) -> None: ...

  def set_virtual(self, slot: SLOT, addr: VADDR, value: object, type_: dict) -> None:
    type_ = spec_get_concrete_type(self.data_spec, type_)

    if type_['kind'] == 'primitive':
      if type_['name'] == 'void':
        raise Exception('Dereferencing void at ' + str(addr))
      pytype = PRIMITIVE_PY_TYPES[type_['name']]
      if not isinstance(value, pytype):
        raise Exception('Cannot set ' + type_['name'] + ' to value ' + str(value))
      self.set_primitive_virtual(slot, addr, value, type_['name'])

    elif type_['kind'] == 'pointer':
      if not isinstance(value, Address):
        raise Exception('Cannot set pointer to ' + str(value))
      self.set_pointer_virtual(slot, addr, value)

    else:
      raise NotImplementedError(type_['kind'])

  # Absolute - get

  def get_primitive_absolute(self, addr: int, type_: str) -> object:
    ctype = PRIMITIVE_C_TYPES[type_]
    pytype = PRIMITIVE_PY_TYPES[type_]
    pointer = C.cast(addr, C.POINTER(ctype)) # type: ignore
    return pytype(pointer[0])

  def get_pointer_absolute(self, addr: int) -> Address[VADDR]:
    pointer = C.cast(addr, C.POINTER(C.c_void_p)) # type: ignore
    location = int(pointer[0] or 0) # type: ignore
    if location == 0:
      return Address.new_null()
    else:
      virtual = self.stored_location_to_virtual(location)
      if virtual is None:
        return Address.new_absolute(location)
      else:
        return Address.new_virtual(virtual)

  def get_absolute(self, addr: int, type_: dict) -> object:
    type_ = spec_get_concrete_type(self.data_spec, type_)

    if type_['kind'] == 'primitive':
      if type_['name'] == 'void':
        raise Exception('Dereferencing void at ' + str(addr))
      return self.get_primitive_absolute(addr, type_['name'])

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

  # Absolute - set

  def set_primitive_absolute(self, addr: int, value: object, type_: str) -> None:
    ctype = PRIMITIVE_C_TYPES[type_]
    pytype = PRIMITIVE_PY_TYPES[type_]
    if not isinstance(value, pytype):
      raise Exception('Cannot set ' + type_ + ' to value ' + str(value))
    pointer = C.cast(addr, C.POINTER(ctype)) # type: ignore
    # TODO: Check overflow behavior
    pointer[0] = value

  def set_absolute(self, addr: int, value: object, type_: dict) -> None:
    type_ = spec_get_concrete_type(self.data_spec, type_)

    if type_['kind'] == 'primitive':
      if type_['name'] == 'void':
        raise Exception('Dereferencing void at ' + str(addr))
      self.set_primitive_absolute(addr, value, type_['name'])

    else:
      raise NotImplementedError(type_['kind'])

  # Virtual or absolute

  def get_primitive(self, slot: SLOT, addr: Address[VADDR], type_: str) -> object:
    if addr.type == AddressType.NULL:
      return None
    elif addr.type == AddressType.ABSOLUTE:
      return self.get_primitive_absolute(addr.absolute, type_)
    elif addr.type == AddressType.VIRTUAL:
      return self.get_primitive_virtual(slot, addr.virtual, type_)
    else:
      raise NotImplementedError(addr.type)

  def get_pointer(self, slot: SLOT, addr: Address[VADDR]) -> Optional[Address[VADDR]]:
    if addr.type == AddressType.NULL:
      return None
    elif addr.type == AddressType.ABSOLUTE:
      return self.get_pointer_absolute(addr.absolute)
    elif addr.type == AddressType.VIRTUAL:
      return self.get_pointer_virtual(slot, addr.virtual)
    else:
      raise NotImplementedError(addr.type)

  def get(self, slot: SLOT, addr: Address[VADDR], type_: dict) -> object:
    if addr.type == AddressType.NULL:
      return None
    elif addr.type == AddressType.ABSOLUTE:
      return self.get_absolute(addr.absolute, type_)
    elif addr.type == AddressType.VIRTUAL:
      return self.get_virtual(slot, addr.virtual, type_)
    else:
      raise NotImplementedError(addr.type)

  def set(self, slot: SLOT, addr: Address[VADDR], value: object, type_: dict) -> None:
    if addr.type == AddressType.NULL:
      pass
    elif addr.type == AddressType.ABSOLUTE:
      self.set_absolute(addr.absolute, value, type_)
    elif addr.type == AddressType.VIRTUAL:
      self.set_virtual(slot, addr.virtual, value, type_)


class AccessibleMemory(Memory[VADDR, SLOT]):
  """This can be used if every slot resides in memory.

  To avoid confusion, "location" is used to represent an integer address that
  resides in slot (virtual) memory.
  An "absolute" address is one that resides in static/shared memory.
  """

  @abstractmethod
  def virtual_to_location(self, slot: SLOT, virtual: VADDR) -> int: ...

  @abstractmethod
  def virtual_to_storable_location(self, virtual: VADDR) -> int:
    """Converts a virtual address to an address that can be saved in slot memory.

    This can be used to ensure that all stored addresses point to the base slot.
    """
    ...

  def get_primitive_virtual(self, slot: SLOT, addr: VADDR, type_: str) -> object:
    location = self.virtual_to_location(slot, addr)
    return self.get_primitive_absolute(location, type_)

  def get_pointer_virtual(self, slot: SLOT, addr: VADDR) -> Address[VADDR]:
    location = self.virtual_to_location(slot, addr)
    return self.get_pointer_absolute(location)

  def set_primitive_virtual(self, slot: SLOT, addr: VADDR, value: object, type_: str) -> None:
    location = self.virtual_to_location(slot, addr)
    self.set_primitive_absolute(location, value, type_)

  def set_pointer_virtual(self, slot: SLOT, addr: VADDR, value: Address[VADDR]) -> None:
    location = self.virtual_to_location(slot, addr)
    pointer = C.cast(location, C.POINTER(C.c_void_p)) # type: ignore
    if value.type == AddressType.NULL:
      stored_location = 0
    elif value.type == AddressType.ABSOLUTE:
      stored_location = value.absolute
    else:
      stored_location = self.virtual_to_storable_location(value.virtual)
    pointer[0] = stored_location # type: ignore


__all__ = [
  'Slot',
  'VirtualAddress',
  'AddressType',
  'Address',
  'Memory',
  'AccessibleMemory',
]
