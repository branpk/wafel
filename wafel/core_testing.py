from __future__ import annotations

from typing import *
from dataclasses import dataclass, field
import json

from wafel.core_new import *
from wafel.util import *


@dataclass(frozen=True)
class TestVirtual(VirtualAddress):
  section: str
  offset: int

  def __add__(self, offset: int) -> TestVirtual:
    return TestVirtual(self.section, self.offset + offset)

  def __str__(self) -> str:
    return '%s+0x%X' % (self.section, self.offset)


@dataclass
class TestSlot(Slot):
  name: str
  memory: Dict[Tuple[str, int], object] = field(default_factory=dict)


class TestMemory(Memory[TestVirtual, TestSlot]):
  @property
  def data_spec(self) -> DataSpec:
    return DATA_SPEC

  def symbol(self, name: str) -> Address[TestVirtual]:
    var = self.data_spec['globals'].get(name)
    if var is None:
      return Address.new_null()
    addr = var['addr']
    if isinstance(addr, int):
      return Address.new_absolute(addr)
    else:
      return Address.new_virtual(TestVirtual(addr[0], addr[1]))

  def _location_to_virtual(self, location: int) -> Optional[TestVirtual]:
    return None

  def get_object(self, slot: TestSlot, addr: TestVirtual) -> Optional[Any]:
    return slot.memory.get((addr.section, addr.offset))

  def get_primitive_virtual(self, slot: TestSlot, addr: TestVirtual, type_: str) -> object:
    return self.get_object(slot, addr)

  def get_pointer_virtual(self, slot: TestSlot, addr: TestVirtual) -> Address[TestVirtual]:
    pointer = self.get_object(slot, addr) or 0
    if isinstance(pointer, int):
      if pointer == 0:
        return Address.new_null()
      else:
        return Address.new_absolute(pointer)
    else:
      return Address.new_virtual(TestVirtual(pointer[0], pointer[1]))

  def set_object(self, slot: TestSlot, addr: TestVirtual, value: object) -> None:
    slot.memory[(addr.section, addr.offset)] = value

  def set_primitive_virtual(self, slot: TestSlot, addr: TestVirtual, value: object, type_: str) -> None:
    return self.set_object(slot, addr, value)

  def set_pointer_virtual(self, slot: TestSlot, addr: TestVirtual, value: Address[TestVirtual]) -> None:
    pointer: object
    if value.type == AddressType.NULL:
      pointer = 0
    elif value.type == AddressType.ABSOLUTE:
      pointer = value.absolute
    elif value.type == AddressType.VIRTUAL:
      pointer = (value.virtual.section, value.virtual.offset)
    self.set_object(slot, addr, pointer)


def prim(name: str) -> dict:
  if name == 'void':
    return { 'kind': 'primitive', 'name': 'void', 'size': 0, 'align': 1 }
  else:
    size = int(name[1:]) // 8
    return { 'kind': 'primitive', 'name': name, 'size': size, 'align': size }

def struct(fields: dict) -> dict:
  fields_spec = {
    name: { 'type': type_ }
      for name, type_ in fields.items()
  }
  return { 'kind': 'struct', 'fields': fields_spec }

def union(fields: dict) -> dict:
  fields_spec = {
    name: { 'type': type_ }
      for name, type_ in fields.items()
  }
  return { 'kind': 'union', 'fields': fields_spec }

def array(base: dict, length: Optional[int] = None) -> dict:
  return { 'kind': 'array', 'base': base, 'length': length }

def pointer(base: dict) -> dict:
  return { 'kind': 'pointer', 'base': base, 'size': 8, 'align': 8 }

def symbol(namespace: str, name: str) -> dict:
  return { 'kind': 'symbol', 'namespace': namespace, 'name': name }

def global_vars(sections: Dict[Optional[str], Any]) -> dict:
  result = {}
  for section, vars in sections.items():
    offset = 0x1000
    for name, type_ in vars.items():
      result[name] = {
        'type': type_,
        'addr': offset if section is None else [section, offset],
      }
      offset += 0x1000
  return result

DATA_SPEC: DataSpec = {
  'types': {
    'struct': {
      'Player': struct({
        'pos': symbol('typedef', 'Vec2f'),
        'vel': symbol('typedef', 'Vec2f'),
      }),
      'Controller': struct({
        'stick_x': prim('s8'),
        'stick_y': prim('s8'),
      }),
      'Object': struct({
        'pos': symbol('typedef', 'Vec2f'),
        'raw_data': array(prim('u8'), 100),
      })
    },
    'union': {
    },
    'typedef': {
      'Vec2f': array(prim('f32'), 2),
    },
  },
  'globals': global_vars({
    '.bss': {
      'player_pool': array(symbol('struct', 'Player')),
      'controller': symbol('struct', 'Controller'),
    },
    '.data': {
      'player': pointer(symbol('struct', 'Player')),
    },
  }),
  'constants': {

  },
  'extra': {
    'object_fields': {
      'o_health': { 'type': prim('f32'), 'offset': 10 },
    },
  },
}

spec_populate_sizes_and_alignment(DATA_SPEC, populate_offsets=True)

DATA_SPEC['types']['struct']['Object']['fields'].update(DATA_SPEC['extra']['object_fields'])


class TestGame(GameImpl[TestVirtual, TestSlot]):
  def __init__(self) -> None:
    self._memory = TestMemory()
    self._slots: Dict[str, TestSlot] = {
      'base': TestSlot('base'),
    }

    # Initialize game
    player_pool = self.path('player_pool').get_addr(self.base_slot)
    self.path('player').set(self.base_slot, player_pool)

  @property
  def base_slot(self) -> TestSlot:
    return self._slots['base']

  def alloc_slot(self) -> TestSlot:
    name = 'S' + str(len(self._slots))
    slot = TestSlot(name)
    self._slots[name] = slot
    return slot

  def dealloc_slot(self, slot: TestSlot) -> None:
    del self._slots[slot.name]

  def copy_slot(self, dst: TestSlot, src: TestSlot) -> None:
    dst.memory = dict(src.memory)

  @property
  def memory(self) -> Memory[TestVirtual, TestSlot]:
    return self._memory

  def run_frame(self) -> None:
    pass


game = TestGame().remove_type_vars()

game.path('player.pos[1]').set(game.base_slot, 32.0)
print(game.path('player.pos[1]').get(game.base_slot))
