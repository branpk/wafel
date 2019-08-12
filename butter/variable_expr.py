from typing import *
from itertools import takewhile

import ctypes

from butter.variable import VariableParam
from butter.game_state import GameState
from butter.util import *


class VariableExpr:
  @staticmethod
  def parse(spec: dict, expr: str) -> 'VariableExpr':
    expr = expr.strip()
    assert expr.startswith('state')
    expr = expr[len('state'):]

    result = _StateExpr(spec)

    while True:
      expr = expr.strip()
      if len(expr) == 0:
        break
      elif expr.startswith('.'):
        expr = expr[1:]
        field = ''.join(takewhile(lambda c: c.isalnum() or c == '_', expr))
        expr = expr[len(field):]
        result = _FieldExpr(spec, result, field)
      elif expr.startswith('['):
        expr = expr[1:]
        index_str = ''.join(takewhile(lambda c: c != ']', expr))
        expr = expr[len(index_str):]
        if len(index_str.strip()) == 0:
          result = _DerefExpr(spec, result)
        else:
          index = int(index_str, base=0)
          result = _IndexExpr(spec, result, index)
        assert expr[0] == ']'
        expr = expr[1:]
      else:
        raise NotImplementedError(expr)

    return result

  def __init__(self, params: List[VariableParam], type_: dict) -> None:
    self.params = params
    self.type = type_

  def get_addr(self, *args: Any) -> int:
    raise NotImplementedError


class _StateExpr(VariableExpr):
  def __init__(self, spec: dict) -> None:
    super().__init__([VariableParam.STATE], spec['types']['struct']['SM64State'])

  def get_addr(self, state: GameState) -> int:
    return state.addr


class _FieldExpr(VariableExpr):
  def __init__(self, spec: dict, struct: VariableExpr, field: str) -> None:
    struct_type = concrete_type(spec, struct.type)
    assert struct_type['kind'] == 'struct'
    field_spec = struct_type['fields'][field]

    super().__init__(struct.params, field_spec['type'])
    self.struct = struct
    self.offset = field_spec['offset']

  def get_addr(self, *args: Any) -> int:
    return self.struct.get_addr(*args) + self.offset


class _IndexExpr(VariableExpr):
  def __init__(self, spec: dict, array: VariableExpr, index: int) -> None:
    array_type = concrete_type(spec, array.type)
    assert array_type['kind'] == 'array'
    element_type = array_type['base']
    stride = align_up(element_type['size'], element_type['align'])

    super().__init__(array.params, element_type)
    self.array = array
    self.offset = stride * index

  def get_addr(self, *args: Any) -> int:
    return self.array.get_addr(*args) + self.offset


class _DerefExpr(VariableExpr):
  def __init__(self, spec: dict, pointer: VariableExpr) -> None:
    pointer_type = concrete_type(spec, pointer.type)
    assert pointer_type['kind'] == 'pointer'

    super().__init__(pointer.params, pointer_type['base'])
    self.pointer = pointer

  def get_addr(self, *args: Any) -> int:
    pointer_addr = self.pointer.get_addr(*args)
    return ctypes.cast(pointer_addr, ctypes.POINTER(ctypes.c_void_p))[0]
