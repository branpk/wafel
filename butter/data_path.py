from typing import *
from itertools import takewhile

import ctypes as C

from butter.variable import VariableParam
from butter.game_state import GameState
from butter.util import *


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


class DataPath:
  @staticmethod
  def parse(spec: dict, expr: str) -> 'DataPath':
    expr = expr.strip()
    assert expr.startswith('$state')
    expr = expr[len('$state'):]

    result = _State(spec)

    while True:
      expr = expr.strip()
      if len(expr) == 0:
        break
      elif expr.startswith('.'):
        expr = expr[1:]
        field = ''.join(takewhile(lambda c: c.isalnum() or c == '_', expr))
        expr = expr[len(field):]
        result = _Field(spec, result, field)
      elif expr.startswith('['):
        expr = expr[1:]
        index_str = ''.join(takewhile(lambda c: c != ']', expr))
        expr = expr[len(index_str):]
        if len(index_str.strip()) == 0:
          result = _Deref(spec, result)
        else:
          index = int(index_str, base=0)
          result = _Index(spec, result, index)
        assert expr[0] == ']'
        expr = expr[1:]
      else:
        raise NotImplementedError(expr)

    return result

  def __init__(self, spec: dict, params: List[VariableParam], type_: dict) -> None:
    self.spec = spec
    self.params = params
    self.type = type_
    self.concrete_type = concrete_type(spec, self.type)

  def get_addr(self, *args: Any) -> int:
    raise NotImplementedError

  def get(self, *args: Any) -> Any:
    # TODO: This and set can be made more efficient

    if self.concrete_type['kind'] == 'primitive':
      ctype = PRIMITIVE_CTYPES[self.concrete_type['name']]
      pytype = PRIMITIVE_PYTYPES[self.concrete_type['name']]
      addr = C.cast(self.get_addr(*args), C.POINTER(ctype))
      return pytype(addr[0])

    elif self.concrete_type['kind'] == 'pointer':
      addr = C.cast(self.get_addr(*args), C.POINTER(C.c_void_p))
      return int(addr[0])

    elif self.concrete_type['kind'] == 'array':
      assert self.concrete_type['length'] is not None
      return tuple(
        _Index(self.spec, self, i).get(*args)
          for i in range(self.concrete_type['length'])
      )

    else:
      raise NotImplementedError(self.concrete_type['kind'])

  def set(self, value: Any, *args: Any) -> Any:
    if self.concrete_type['kind'] == 'primitive':
      ctype = PRIMITIVE_CTYPES[self.concrete_type['name']]
      pytype = PRIMITIVE_PYTYPES[self.concrete_type['name']]
      assert isinstance(value, pytype)
      addr = C.cast(self.get_addr(*args), C.POINTER(ctype))
      # TODO: Check overflow behavior
      addr[0] = value

    elif self.concrete_type['kind'] == 'pointer':
      assert isinstance(value, int)
      addr = C.cast(self.get_addr(*args), C.POINTER(C.c_void_p))
      addr[0] = value

    elif self.concrete_type['kind'] == 'array':
      assert self.concrete_type['length'] is not None
      assert isinstance(value, tuple)
      assert len(value) == self.concrete_type['length']
      for i, elem_value in enumerate(value):
        _Index(self.spec, self, i).set(elem_value, *args)

    else:
      raise NotImplementedError(self.concrete_type['kind'])


class _State(DataPath):
  def __init__(self, spec: dict) -> None:
    super().__init__(spec, [VariableParam.STATE], spec['types']['struct']['SM64State'])

  def get_addr(self, state: GameState) -> int:
    return state.addr


class _Field(DataPath):
  def __init__(self, spec: dict, struct: DataPath, field: str) -> None:
    struct_type = concrete_type(spec, struct.type)
    assert struct_type['kind'] == 'struct'
    field_spec = struct_type['fields'][field]

    super().__init__(spec, struct.params, field_spec['type'])
    self.struct = struct
    self.offset = field_spec['offset']

  def get_addr(self, *args: Any) -> int:
    return self.struct.get_addr(*args) + self.offset


class _Index(DataPath):
  def __init__(self, spec: dict, array: DataPath, index: int) -> None:
    array_type = concrete_type(spec, array.type)
    assert array_type['kind'] == 'array'
    element_type = array_type['base']
    stride = align_up(element_type['size'], element_type['align'])

    super().__init__(spec, array.params, element_type)
    self.array = array
    self.offset = stride * index

  def get_addr(self, *args: Any) -> int:
    return self.array.get_addr(*args) + self.offset


class _Deref(DataPath):
  def __init__(self, spec: dict, pointer: DataPath) -> None:
    pointer_type = concrete_type(spec, pointer.type)
    assert pointer_type['kind'] == 'pointer'

    super().__init__(spec, pointer.params, pointer_type['base'])
    self.pointer = pointer

  def get_addr(self, *args: Any) -> int:
    return self.pointer.get(*args)
