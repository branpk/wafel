from typing import *
from itertools import takewhile

import ctypes as C

from wafel.core.game_lib import GameLib
from wafel.core.variable_param import VariableParam, VariableArgs
from wafel.util import *


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
  def parse(lib: GameLib, expr: str) -> 'DataPath':
    expr = expr.strip()

    if expr.startswith('$state'):
      expr = expr[len('$state'):]
      result = _State(lib)
    elif expr.startswith('$object'):
      expr = expr[len('$object'):]
      result = _Object(lib)
    else:
      raise NotImplementedError(expr)

    while True:
      expr = expr.strip()
      if len(expr) == 0:
        break
      elif expr.startswith('.'):
        expr = expr[1:]
        field = ''.join(takewhile(lambda c: c.isalnum() or c == '_', expr))
        expr = expr[len(field):]
        result = _Field(lib, result, field)
      elif expr.startswith('['):
        expr = expr[1:]
        index_str = ''.join(takewhile(lambda c: c != ']', expr))
        expr = expr[len(index_str):]
        if len(index_str.strip()) == 0:
          result = _Deref(lib, result)
        else:
          index = int(index_str, base=0)
          result = _Index(lib, result, index)
        assert expr[0] == ']'
        expr = expr[1:]
      else:
        raise NotImplementedError(expr)

    return result

  def __init__(self, lib: GameLib, params: List[VariableParam], type_: dict) -> None:
    self.lib = lib
    self.params = params
    self.type = type_
    self.concrete_type = self.lib.concrete_type(self.type)

    if self.concrete_type['kind'] == 'pointer' and VariableParam.STATE not in self.params:
      self.params = [VariableParam.STATE] + self.params

  def get_addr(self, args: VariableArgs) -> int:
    raise NotImplementedError

  def get(self, args: VariableArgs) -> Any:
    # TODO: This and set can be made more efficient

    if self.concrete_type['kind'] == 'primitive':
      ctype = PRIMITIVE_CTYPES[self.concrete_type['name']]
      pytype = PRIMITIVE_PYTYPES[self.concrete_type['name']]
      addr = C.cast(self.get_addr(args), C.POINTER(ctype))
      return pytype(addr[0])

    elif self.concrete_type['kind'] == 'pointer':
      addr = C.cast(self.get_addr(args), C.POINTER(C.c_void_p))
      value = int(addr[0] or 0)

      state = args[VariableParam.STATE]
      state_size = self.lib.spec['types']['struct']['SM64State']['size']

      if value < state.base_addr or value >= state.base_addr + state_size:
        return value
      else:
        return value - state.base_addr + state.addr

    elif self.concrete_type['kind'] == 'array':
      assert self.concrete_type['length'] is not None
      return tuple(
        _Index(self.lib, self, i).get(args)
          for i in range(self.concrete_type['length'])
      )

    else:
      raise NotImplementedError(self.concrete_type['kind'])

  def set(self, value: Any, args: VariableArgs) -> Any:
    if self.concrete_type['kind'] == 'primitive':
      ctype = PRIMITIVE_CTYPES[self.concrete_type['name']]
      pytype = PRIMITIVE_PYTYPES[self.concrete_type['name']]
      assert isinstance(value, pytype)
      addr = C.cast(self.get_addr(args), C.POINTER(ctype))
      # TODO: Check overflow behavior
      addr[0] = value

    elif self.concrete_type['kind'] == 'pointer':
      raise NotImplementedError('pointer')
      assert isinstance(value, int)
      addr = C.cast(self.get_addr(args), C.POINTER(C.c_void_p))
      addr[0] = value

    elif self.concrete_type['kind'] == 'array':
      assert self.concrete_type['length'] is not None
      assert isinstance(value, tuple)
      assert len(value) == self.concrete_type['length']
      for i, elem_value in enumerate(value):
        _Index(self.lib, self, i).set(elem_value, args)

    else:
      raise NotImplementedError(self.concrete_type['kind'])


class _State(DataPath):
  def __init__(self, lib: GameLib) -> None:
    super().__init__(lib, [VariableParam.STATE], lib.spec['types']['struct']['SM64State'])

  def get_addr(self, args: VariableArgs) -> int:
    return args[VariableParam.STATE].addr


class _Object(DataPath):
  def __init__(self, lib: GameLib) -> None:
    super().__init__(lib, [VariableParam.OBJECT], lib.spec['types']['struct']['Object'])

  def get_addr(self, args: VariableArgs) -> int:
    return args[VariableParam.OBJECT].addr


class _Field(DataPath):
  def __init__(self, lib: GameLib, struct: DataPath, field: str) -> None:
    struct_type = lib.concrete_type(struct.type)
    assert struct_type['kind'] == 'struct'
    field_spec = struct_type['fields'][field]

    super().__init__(lib, struct.params, field_spec['type'])
    self.struct = struct
    self.offset = field_spec['offset']

  def get_addr(self, args: VariableArgs) -> int:
    return self.struct.get_addr(args) + self.offset


class _Index(DataPath):
  def __init__(self, lib: GameLib, array: DataPath, index: int) -> None:
    array_type = lib.concrete_type(array.type)
    assert array_type['kind'] == 'array'
    element_type = array_type['base']
    stride = align_up(element_type['size'], element_type['align'])

    super().__init__(lib, array.params, element_type)
    self.array = array
    self.offset = stride * index

  def get_addr(self, args: VariableArgs) -> int:
    return self.array.get_addr(args) + self.offset


class _Deref(DataPath):
  def __init__(self, lib: GameLib, pointer: DataPath) -> None:
    pointer_type = lib.concrete_type(pointer.type)
    assert pointer_type['kind'] == 'pointer'

    super().__init__(lib, pointer.params, pointer_type['base'])
    self.pointer = pointer

  def get_addr(self, args: VariableArgs) -> int:
    return self.pointer.get(args)
