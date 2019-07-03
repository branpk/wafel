from typing import *
from enum import Enum, auto
import ctypes as C

from butter.game_state import GameState
from butter.input_sequence import Input


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


class VariableDataType(Enum):
  BOOL = auto()
  S8 = auto()
  U8 = auto()
  S16 = auto()
  U16 = auto()
  S32 = auto()
  U32 = auto()
  S64 = auto()
  U64 = auto()
  F32 = auto()
  F64 = auto()

  @staticmethod
  def from_spec(type_: dict) -> 'VariableDataType':
    assert type_['kind'] == 'primitive'
    return {
      's8': VariableDataType.S8,
      'u8': VariableDataType.U8,
      's16': VariableDataType.S16,
      'u16': VariableDataType.U16,
      's32': VariableDataType.S32,
      'u32': VariableDataType.U32,
      's64': VariableDataType.S64,
      'u64': VariableDataType.U64,
      'f32': VariableDataType.F32,
      'f64': VariableDataType.F64,
    }[type_['name']]

  def pytype(self) -> Type:
    return {
      VariableDataType.BOOL: bool,
      VariableDataType.S8: int,
      VariableDataType.U8: int,
      VariableDataType.S16: int,
      VariableDataType.U16: int,
      VariableDataType.S32: int,
      VariableDataType.U32: int,
      VariableDataType.S64: int,
      VariableDataType.U64: int,
      VariableDataType.F32: float,
      VariableDataType.F64: float,
    }[self]


class Variable:
  def __init__(
    self,
    name: str,
    params: List[VariableParam],
    semantics: VariableSemantics,
    read_only: bool,
    data_type: VariableDataType,
  ) -> None:
    self.name = name
    self.params = params
    self.semantics = semantics
    self.read_only = read_only
    self.data_type = data_type

  def get(self, *args: Any) -> Any:
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


class InputButtonVariable(Variable):
  def __init__(
    self,
    spec: dict,
    button_name: str,
    flag_name: str,
  ) -> None:
    self.flag = spec['constants'][flag_name]['value']
    super().__init__(
      button_name,
      [VariableParam.INPUT],
      VariableSemantics.FLAG,
      False,
      VariableDataType.BOOL,
    )

  def get(self, input: Input) -> bool:
    return (input.buttons & self.flag) != 0

  def set(self, value: object, input: Input) -> None:
    assert isinstance(value, bool)
    if value:
      input.buttons |= self.flag
    else:
      input.buttons &= ~self.flag


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
      VariableDataType.from_spec(field['type']),
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
    InputButtonVariable(spec, 'A', 'A_BUTTON'),
    InputButtonVariable(spec, 'B', 'B_BUTTON'),
    InputButtonVariable(spec, 'Z', 'Z_TRIG'),
    InputButtonVariable(spec, 'S', 'START_BUTTON'),
    StateDataVariable(spec, 'global timer', VariableSemantics.RAW, 'gGlobalTimer'),
  ])
