from __future__ import annotations

from typing import *
from enum import Enum, auto
from dataclasses import dataclass
import ctypes as C
import re

from wafel.util import *
from wafel.core_new.game import Game, AddressType, Address, Slot, DataSpec
from wafel.core_new.data_spec_util import spec_get_concrete_type


class EdgeType(Enum):
  OFFSET = auto()
  DEREF = auto()


@dataclass(frozen=True)
class Edge:
  type: EdgeType
  value: int

  @staticmethod
  def offset(amount: int) -> Edge:
    return Edge(EdgeType.OFFSET, amount)

  @staticmethod
  def deref() -> Edge:
    return Edge(EdgeType.DEREF, 0)


# TODO: Move deref implementations somewhere else?
def deref_pointer(game: Game, slot: Slot, addr: Address) -> Address:
  if addr.type == AddressType.NULL:
    return addr
  elif addr.type == AddressType.ABSOLUTE:
    ptr = C.cast(addr.absolute, C.POINTER(C.c_void_p)) # type: ignore
    ptr_value = int(ptr[0] or 0) # type: ignore
    return Address.new_absolute(ptr_value)
  elif addr.type == AddressType.VIRTUAL:
    return game.memory.get_pointer(slot, addr.virtual)
  else:
    raise NotImplementedError(addr.type)


@dataclass(frozen=True)
class DataPath:
  game: Game
  start_type: Optional[dict]
  end_type: dict
  start_addr: Optional[Address]
  edges: List[Edge]

  @staticmethod
  def compile(game: Game, source: str) -> DataPath:
    return compile_data_path(game, source)

  def get_addr(self, slot: Slot) -> Address:
    assert self.start_addr is not None
    addr = self.start_addr

    for edge in self.edges:
      if edge.type == EdgeType.OFFSET:
        addr += edge.value
      elif edge.type == EdgeType.DEREF:
        addr = deref_pointer(self.game, slot, addr)
      else:
        raise NotImplementedError(edge.type)

    return addr

  def append(self, edge: Edge, end_type: dict) -> DataPath:
    return DataPath(
      game = self.game,
      start_type = self.start_type,
      end_type = spec_get_concrete_type(self.game.data_spec, end_type),
      start_addr = self.start_addr,
      edges = self.edges + [edge],
    )

  def __add__(self, other: DataPath) -> DataPath:
    assert self.game == other.game
    if self.end_type != other.start_type:
      raise Exception('Mismatched types: ' + str(self.end_type) + ' and ' + str(other.start_type))
    return DataPath(
      game = self.game,
      start_type = self.start_type,
      end_type = other.end_type,
      start_addr = self.start_addr,
      edges = self.edges + other.edges,
    )


class DataPathContext:
  def __init__(self, path: DataPath) -> None:
    self.path = path

  def __getattribute__(self, name: str) -> DataPathContext:
    # TODO: Anonymous fields
    path = cast(DataPath, super().__getattribute__('path'))

    if path.end_type['kind'] in ['struct', 'union']:
      struct_path = path
    elif path.end_type['kind'] == 'pointer':
      struct_path = path.append(Edge.deref(), path.end_type['base'])
    else:
      raise Exception(
        'Trying to access field ' + name + ' from non-struct type ' + str(path.end_type)
      )

    field = struct_path.end_type['fields'].get(name)
    if field is None:
      raise Exception('Field not defined: ' + name + ' in ' + str(struct_path.end_type))

    return DataPathContext(
      struct_path.append(Edge.offset(field['offset']), field['type'])
    )

  def __getitem__(self, index: object) -> DataPathContext:
    if not isinstance(index, int) or index < 0:
      raise Exception('Subscript must be a non-negative integer: ' + str(index))

    path = cast(DataPath, super().__getattribute__('path'))

    if path.end_type['kind'] == 'array':
      array_path = path
    elif path.end_type['kind'] == 'pointer':
      array_path = path.append(Edge.deref(), {
        'kind': 'array',
        'base': path.end_type['base'],
        'length': None,
      })
    else:
      raise Exception('Trying to subscript into non-array type ' + str(path.end_type))

    array_type = array_path.end_type
    if array_type['length'] is not None:
      if index >= array_type['length']:
        raise Exception('Index out of bounds: ' + str(index) + ' in ' + str(path.end_type))

    stride = align_up(array_type['base']['size'], array_type['base']['align'])
    return DataPathContext(
      array_path.append(Edge.offset(index * stride), array_type['base'])
    )


class NamespaceContext:
  def __init__(self, game: Game, namespace: str) -> None:
    self.game = game
    self.namespace = namespace

  def __getattribute__(self, name: str) -> DataPathContext:
    game = cast(Game, super().__getattribute__('game'))
    namespace = cast(str, super().__getattribute__('namespace'))

    type_ = game.data_spec['types'][namespace].get(name)
    if type_ is None:
      raise Exception('Undefined type: ' + namespace + ' ' + name)
    type_ = spec_get_concrete_type(game.data_spec, type_)

    return DataPathContext(DataPath(
      game = game,
      start_type = type_,
      end_type = type_,
      start_addr = None,
      edges = [],
    ))


class GlobalContext:
  def __init__(self, game: Game) -> None:
    self.game = game

  def __getattribute__(self, name: str) -> Union[NamespaceContext, DataPathContext]:
    game = cast(Game, super().__getattribute__('game'))

    if name in ['struct', 'union', 'typedef']:
      return NamespaceContext(game, name)

    global_var = game.data_spec['globals'].get(name)
    if global_var is None:
      raise Exception('Global variable not defined: ' + name)

    type_ = spec_get_concrete_type(game.data_spec, global_var['type'])

    addr = game.symbol(name)
    assert addr.type != AddressType.NULL, name

    return DataPathContext(DataPath(
      game = game,
      start_type = None,
      end_type = type_,
      start_addr = addr,
      edges = [],
    ))


def compile_data_path(game: Game, source: str) -> DataPath:
  original_source = source

  source = source.strip()
  source = re.sub('^struct ', 'struct.', source)
  source = re.sub('^union ', 'typedef.', source)
  source = re.sub('^typedef ', 'typedef.', source)
  source = source.replace('[]', '[0]')
  source = source.replace('->', '[0].')
  source = 'context.' + source

  result = eval(source, {}, { 'context': GlobalContext(game) })

  if object.__getattribute__(result, '__class__') is not DataPathContext:
    raise Exception('Invalid path: ' + original_source + ' (returned ' + str(result) + ')')
  return cast(DataPath, object.__getattribute__(result, 'path'))


__all__ = ['DataPath']
