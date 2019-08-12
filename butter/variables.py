import ctypes as C

from butter.game_state import GameState
from butter.variable import *
from butter.variable_expr import VariableExpr
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


class _DataVariable(Variable):
  def __init__(
    self,
    name: str,
    spec: dict,
    semantics: VariableSemantics,
    expression: str,
    read_only: bool = False,
  ) -> None:
    self.expr = VariableExpr.parse(spec, expression)

    type_ = concrete_type(spec, self.expr.type)
    assert type_['kind'] == 'primitive'
    self.ctype = PRIMITIVE_CTYPES[type_['name']]

    super().__init__(
      name,
      self.expr.params,
      semantics,
      read_only,
      VariableDataType.from_spec(type_),
    )

    self.pytype = self.data_type.pytype

  def get(self, *args: Any) -> Any:
    addr = C.cast(self.expr.get_addr(*args), C.POINTER(self.ctype))
    return self.pytype(addr[0])

  def set(self, value: Any, *args: Any) -> None:
    assert not self.read_only
    assert isinstance(value, self.pytype)
    addr = C.cast(self.expr.get_addr(*args), C.POINTER(self.ctype))
    # TODO: Check overflow behavior
    addr[0] = value


class _FlagVariable(Variable):
  def __init__(
    self,
    name: str,
    spec: dict,
    flags: Variable,
    flag: str,
    read_only: bool = False
  ) -> None:
    super().__init__(
      name,
      flags.params,
      VariableSemantics.FLAG,
      read_only or flags.read_only,
      VariableDataType.BOOL,
    )
    self.flags = flags
    self.flag = spec['constants'][flag]['value'] # TODO: Expressions

  def get(self, *args: Any) -> bool:
    return (self.flags.get(*args) & self.flag) != 0

  def set(self, value: bool, *args: Any) -> None:
    flags = self.flags.get(*args)
    if value:
      flags |= self.flag
    else:
      flags &= ~self.flag
    self.flags.set(flags, *args)


def create_variables(spec: dict) -> Variables:
  input_buttons = _DataVariable('buttons', spec, VariableSemantics.RAW, 'state.gControllerPads[0].button')
  return Variables([
    input_buttons,
    _DataVariable('stick x', spec, VariableSemantics.RAW, 'state.gControllerPads[0].stick_x'),
    _DataVariable('stick y', spec, VariableSemantics.RAW, 'state.gControllerPads[0].stick_y'),
    _FlagVariable('A', spec, input_buttons, 'A_BUTTON'),
    _FlagVariable('B', spec, input_buttons, 'B_BUTTON'),
    _FlagVariable('Z', spec, input_buttons, 'Z_TRIG'),
    _FlagVariable('S', spec, input_buttons, 'START_BUTTON'),
    _DataVariable('global timer', spec, VariableSemantics.RAW, 'state.gGlobalTimer'),
    # TODO: Combine position into single variable
    _DataVariable('mario x', spec, VariableSemantics.RAW, 'state.gMarioState[].pos[0]'),
    _DataVariable('mario y', spec, VariableSemantics.RAW, 'state.gMarioState[].pos[1]'),
    _DataVariable('mario z', spec, VariableSemantics.RAW, 'state.gMarioState[].pos[2]'),
  ])
