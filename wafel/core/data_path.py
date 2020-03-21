from __future__ import annotations

from typing import *
from enum import Enum, auto
from dataclasses import dataclass
import ctypes as C
from itertools import takewhile

from wafel.core.game_lib import GameLib
from wafel.core.game_state import GameState, Object, AbsoluteAddr, RelativeAddr, OffsetAddr
from wafel.util import *


class RootKind(Enum):
  ABSOLUTE = auto()
  RELATIVE = auto()

@dataclass(frozen=True)
class Root:
  kind: RootKind
  value: int

  @staticmethod
  def absolute(addr: int) -> Root:
    return Root(RootKind.ABSOLUTE, addr)

  @staticmethod
  def relative(range_index: int) -> Root:
    return Root(RootKind.RELATIVE, range_index)

  def eval(self, state: GameState) -> int:
    if self.kind == RootKind.ABSOLUTE:
      return self.value
    elif self.kind == RootKind.RELATIVE:
      return state.slot.addr_ranges[self.value].start
    else:
      raise NotImplementedError(self.kind)

  def __repr__(self) -> str:
    return self.kind.name.lower() + '(' + str(self.value) + ')'


class EdgeKind(Enum):
  OFFSET = auto()
  DEREF = auto()

@dataclass(frozen=True)
class Edge:
  kind: EdgeKind
  value: int

  @staticmethod
  def offset(amount: int) -> Edge:
    return Edge(EdgeKind.OFFSET, amount)

  @staticmethod
  def deref() -> Edge:
    return Edge(EdgeKind.DEREF, 0)

  def eval(self, addr: int) -> int:
    # If a path goes through a null pointer, the end result will be 0
    if addr == 0:
      return 0
    if self.kind == EdgeKind.OFFSET:
      return addr + self.value
    elif self.kind == EdgeKind.DEREF:
      ptr = C.cast(addr, C.POINTER(C.c_void_p)) # type: ignore
      return int(ptr[0] or 0) # type: ignore
    else:
      raise NotImplementedError(self.kind)

  def __repr__(self) -> str:
    if self.kind == EdgeKind.DEREF:
      return 'deref()'
    return self.kind.name.lower() + '(' + str(self.value) + ')'


@dataclass(frozen=True)
class AddrPath:
  root: Optional[Root]
  path: Tuple[Edge, ...]

  @staticmethod
  def root_(root: Root) -> AddrPath:
    return AddrPath(root, ())

  @staticmethod
  def edge(edge: Edge) -> AddrPath:
    return AddrPath(None, (edge,))

  def eval(self, state: GameState) -> int:
    assert self.root is not None
    addr = self.root.eval(state)
    for edge in self.path:
      addr = edge.eval(addr)

      # If the pointer has an address in the base slot, relocate it
      offset = state.slot.base_slot.addr_to_offset(addr)
      if offset is not None:
        addr = state.slot.offset_to_addr(offset)
    return addr

  def __add__(self, other: AddrPath) -> AddrPath:
    assert other.root is None
    return AddrPath(self.root, self.path + other.path)


PRIMITIVE_CTYPES = {
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

PRIMITIVE_PYTYPES = {
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


# TODO: Handle null better? (E.g. return Maybe so that the variable can be greyed
# out?)


def get_data(lib: GameLib, state: GameState, type_: dict, addr: int) -> object:
  concrete_type = lib.concrete_type(type_)

  if concrete_type['kind'] == 'primitive':
    ctype = PRIMITIVE_CTYPES[concrete_type['name']]
    pytype = PRIMITIVE_PYTYPES[concrete_type['name']]
    pointer = C.cast(addr, C.POINTER(ctype)) # type: ignore
    return pytype(pointer[0] if pointer else 0)

  elif concrete_type['kind'] == 'pointer':
    pointer = C.cast(addr, C.POINTER(C.c_void_p)) # type: ignore
    value = int(pointer[0] or 0 if pointer else 0) # type: ignore
    return state.slot.base_slot.addr_to_relative(value)

  elif concrete_type['kind'] == 'array':
    length = concrete_type['length']
    assert length is not None, concrete_type
    element_type = concrete_type['base']
    stride = align_up(element_type['size'], element_type['align'])
    return [
      get_data(lib, state, element_type, addr + stride * i)
        for i in range(length)
    ]

  elif concrete_type['kind'] == 'struct':
    result = {}
    for field_name, field in concrete_type['fields'].items():
      result[field_name] = get_data(lib, state, field['type'], addr + field['offset'])
    return result

  else:
    raise NotImplementedError(concrete_type['kind'])


@dataclass(frozen=True)
class DataPath:
  lib: GameLib
  start_type: Optional[dict]
  concrete_start_type: Optional[dict]
  end_type: dict
  concrete_end_type: dict
  addr_path: AddrPath

  @staticmethod
  def compile(lib: GameLib, source: str) -> DataPath:
    expr = parse_expr(source)
    path = resolve_expr(lib, expr)
    return path

  def get_addr(self, state: GameState) -> int:
    assert self.start_type is None
    return self.addr_path.eval(state)

  # TODO: Structs and arrays

  def get(self, state: GameState) -> object:
    addr = self.get_addr(state)
    return get_data(self.lib, state, self.concrete_end_type, addr)

  def set(self, state: GameState, value: object) -> None:
    addr = self.get_addr(state)
    if addr == 0:
      return

    if self.concrete_end_type['kind'] == 'primitive':
      ctype = PRIMITIVE_CTYPES[self.concrete_end_type['name']]
      pytype = PRIMITIVE_PYTYPES[self.concrete_end_type['name']]
      assert isinstance(value, pytype)
      pointer = C.cast(addr, C.POINTER(ctype)) # type: ignore
      # TODO: Check overflow behavior
      pointer[0] = value

    elif self.concrete_end_type['kind'] == 'pointer':
      assert isinstance(value, RelativeAddr)
      value = state.slot.base_slot.relative_to_addr(value)
      pointer = C.cast(addr, C.POINTER(C.c_void_p)) # type: ignore
      pointer[0] = value

    else:
      raise NotImplementedError(self.concrete_end_type['kind'])

  def __add__(self, other: DataPath) -> DataPath:
    assert self.lib is other.lib
    assert self.concrete_end_type == other.concrete_start_type
    return DataPath(
      lib = self.lib,
      start_type = self.start_type,
      concrete_start_type = self.concrete_start_type,
      end_type = other.end_type,
      concrete_end_type = other.concrete_end_type,
      addr_path = self.addr_path + other.addr_path,
    )


# TODO: Relative data paths (starting at struct)

@dataclass(frozen=True)
class StateExpr:
  pass

@dataclass(frozen=True)
class ObjectExpr:
  pass

@dataclass(frozen=True)
class SurfaceExpr:
  pass

@dataclass(frozen=True)
class FieldExpr:
  expr: Expr
  field: str

@dataclass(frozen=True)
class DerefExpr:
  expr: Expr

@dataclass(frozen=True)
class IndexExpr:
  expr: Expr
  index: int

Expr = Union[StateExpr, ObjectExpr, SurfaceExpr, FieldExpr, DerefExpr, IndexExpr]


def parse_expr(source: str) -> Expr:
  result: Expr

  source = source.strip()
  if source.startswith('$state'):
    source = source[len('$state'):]
    result = StateExpr()
  elif source.startswith('$object'):
    source = source[len('$object'):]
    result = ObjectExpr()
  elif source.startswith('$surface'):
    source = source[len('$surface'):]
    result = SurfaceExpr()
  else:
    raise NotImplementedError(source)

  while True:
    source = source.strip()
    if len(source) == 0:
      break
    if source.startswith('.'):
      source = source[1:]
      field = ''.join(takewhile(lambda c: c.isalnum() or c == '_', source))
      source = source[len(field):]
      result = FieldExpr(result, field)
    elif source.startswith('['):
      source = source[1:]
      index_str = ''.join(takewhile(lambda c: c != ']', source))
      source = source[len(index_str):]
      if len(index_str.strip()) == 0:
        result = DerefExpr(result)
      else:
        index = int(index_str, base=0)
        result = IndexExpr(result, index)
      assert source[0] == ']'
      source = source[1:]
    else:
      raise NotImplementedError(source)

  return result


def resolve_expr(lib: GameLib, expr: Expr) -> DataPath:
  if isinstance(expr, FieldExpr):
    if isinstance(expr.expr, StateExpr):
      field_type = lib.spec['globals'][expr.field]['type']
      field_addr = lib.symbol_addr(expr.field).value
      if isinstance(field_addr, AbsoluteAddr):
        addr_path = AddrPath.root_(Root.absolute(field_addr.addr))
      else:
        addr_path = AddrPath.root_(Root.relative(field_addr.range_index)) + \
          AddrPath.edge(Edge.offset(field_addr.offset))
      return DataPath(
        lib = lib,
        start_type = None,
        concrete_start_type = None,
        end_type = field_type,
        concrete_end_type = lib.concrete_type(field_type),
        addr_path = addr_path,
      )

    if isinstance(expr.expr, ObjectExpr):
      object_struct = lib.spec['types']['struct']['Object']
      struct_path = DataPath(
        lib = lib,
        start_type = object_struct,
        concrete_start_type = lib.concrete_type(object_struct),
        end_type = object_struct,
        concrete_end_type = lib.concrete_type(object_struct),
        addr_path = AddrPath(root=None, path=()),
      )
    elif isinstance(expr.expr, SurfaceExpr):
      surface_struct = lib.spec['types']['struct']['Surface']
      struct_path = DataPath(
        lib = lib,
        start_type = surface_struct,
        concrete_start_type = lib.concrete_type(surface_struct),
        end_type = surface_struct,
        concrete_end_type = lib.concrete_type(surface_struct),
        addr_path = AddrPath(root=None, path=()),
      )
    else:
      struct_path = resolve_expr(lib, expr.expr)

    struct_type = struct_path.concrete_end_type
    assert struct_type['kind'] in ['struct', 'union'], expr.expr

    if expr.field in lib.spec['extra']['object_fields']:
      field_spec = lib.spec['extra']['object_fields'][expr.field]
      field_type = field_spec['type']
      field_offset = struct_type['fields']['rawData']['offset'] + field_spec['offset']
    else:
      field_spec = struct_type['fields'].get(expr.field)
      # TODO: Handle anonymous fields
      assert field_spec is not None, expr.field
      field_type = field_spec['type']
      field_offset = field_spec['offset']

    return DataPath(
      lib = lib,
      start_type = struct_path.start_type,
      concrete_start_type = lib.concrete_type(struct_path.start_type),
      end_type = field_type,
      concrete_end_type = lib.concrete_type(field_type),
      addr_path = struct_path.addr_path + AddrPath.edge(Edge.offset(field_offset)),
    )

  elif isinstance(expr, DerefExpr):
    pointer_path = resolve_expr(lib, expr.expr)
    pointer_type = pointer_path.concrete_end_type
    assert pointer_type['kind'] == 'pointer', expr.expr
    return DataPath(
      lib = lib,
      start_type = pointer_path.start_type,
      concrete_start_type = lib.concrete_type(pointer_path.start_type),
      end_type = pointer_type['base'],
      concrete_end_type = lib.concrete_type(pointer_type['base']),
      addr_path = pointer_path.addr_path + AddrPath.edge(Edge.deref()),
    )

  elif isinstance(expr, IndexExpr):
    array_path = resolve_expr(lib, expr.expr)
    array_type = array_path.concrete_end_type
    assert expr.index >= 0
    if array_type['kind'] == 'array':
      if array_type['length'] is not None:
        assert expr.index < array_type['length']
      element_type = array_type['base']
    elif array_type['kind'] == 'pointer':
      element_type = array_type['base']
      array_path = resolve_expr(lib, DerefExpr(expr.expr))
    else:
      assert False, expr.expr

    stride = align_up(element_type['size'], element_type['align'])
    return DataPath(
      lib = lib,
      start_type = array_path.start_type,
      concrete_start_type = lib.concrete_type(array_path.start_type),
      end_type = element_type,
      concrete_end_type = lib.concrete_type(element_type),
      addr_path = array_path.addr_path + AddrPath.edge(Edge.offset(expr.index * stride))
    )

  else:
    raise NotImplementedError(expr)


__all__ = ['DataPath']
