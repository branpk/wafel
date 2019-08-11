import ctypes as C

from butter.game_state import GameState
from butter.edit import Input
from butter.variable import *


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
