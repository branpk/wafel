from typing import *
from enum import Enum, auto
import ctypes as C

from butter.game_state import GameState


class VariableParam(Enum):
  STATE = auto()
  INPUT = auto()


class VariableSemantics(Enum):
  RAW = auto()
  POSITION = auto()
  VELOCITY = auto()
  ANGLE = auto()
  FLAG = auto()
  MARIO_ACTION = auto()


class Variable:
  def __init__(
    self,
    name: str,
    params: List[VariableParam],
    semantics: VariableSemantics,
    read_only: bool,
    data_type: dict,
  ) -> None:
    self.name = name
    self.params = params
    self.semantics = semantics
    self.read_only = read_only
    self.data_type = data_type

  def get(self, *args: Any) -> object:
    raise NotImplementedError

  def set(self, value: Any, *args: Any) -> None:
    raise NotImplementedError


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


class StateDataVariable(Variable):
  def __init__(
    self,
    spec: dict,
    name: str,
    semantics: VariableSemantics,
    expression: str,
    read_only: bool = False,
  ) -> None:
    # TODO: Expressions
    field = spec['types']['struct']['SM64State']['fields'][expression]
    self.offset = field['offset']

    assert field['type']['kind'] == 'primitive'
    self.ctype = PRIMITIVE_CTYPES[field['type']['name']]

    super().__init__(
      name,
      [VariableParam.STATE],
      semantics,
      read_only,
      field['type'],
    )

  def get(self, state: GameState) -> object:
    addr = C.cast(state.addr + self.offset, C.POINTER(self.ctype))
    return int(addr[0])

  def set(self, value: object, state: GameState) -> None:
    assert not self.read_only
    assert isinstance(value, int)
    addr = C.cast(state.addr + self.offset, C.POINTER(self.ctype))
    addr[0] = value


class Variables:
  def __init__(self, variables: List[Variable]) -> None:
    # TODO: Formatting settings
    self.variables = variables


def create_variables(spec: dict) -> Variables:
  return Variables([
    StateDataVariable(spec, 'global timer', VariableSemantics.RAW, 'gGlobalTimer'),
  ])
