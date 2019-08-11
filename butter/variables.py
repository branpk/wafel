import ctypes as C

from butter.game_state import GameState
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


class StateDataVariable(Variable):
  def __init__(
    self,
    name: str,
    spec: dict,
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
    # TODO: Check overflow behavior
    addr[0] = value


class FlagVariable(Variable):
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


# TODO: Once expressions are implemented, make these into StateDataVariables

class InputButtonsVariable(Variable):
  def __init__(self, spec: dict):
    super().__init__(
      "input buttons",
      [VariableParam.STATE],
      VariableSemantics.RAW,
      False,
      VariableDataType.U16,
    )
    globals = spec['types']['struct']['SM64State']['fields']
    controller = globals['gControllerPads']['offset']
    os_cont_pad = spec['types']['typedef']['OSContPad']['fields']
    self.offset = controller + os_cont_pad['button']['offset']

  def get(self, state: GameState) -> int:
    return C.cast(state.addr + self.offset, C.POINTER(C.c_uint16))[0]

  def set(self, value: int, state: GameState) -> None:
    C.cast(state.addr + self.offset, C.POINTER(C.c_uint16))[0] = value

class InputStickVariable(Variable):
  def __init__(self, name: str, spec: dict):
    super().__init__(
      name.replace('_', ' '),
      [VariableParam.STATE],
      VariableSemantics.RAW,
      False,
      VariableDataType.S8,
    )
    globals = spec['types']['struct']['SM64State']['fields']
    controller = globals['gControllerPads']['offset']
    os_cont_pad = spec['types']['typedef']['OSContPad']['fields']
    self.offset = controller + os_cont_pad[name]['offset']

  def get(self, state: GameState) -> int:
    return C.cast(state.addr + self.offset, C.POINTER(C.c_int8))[0]

  def set(self, value: int, state: GameState) -> None:
    C.cast(state.addr + self.offset, C.POINTER(C.c_int8))[0] = value


def create_variables(spec: dict) -> Variables:
  input_buttons = InputButtonsVariable(spec)
  return Variables([
    input_buttons,
    InputStickVariable('stick_x', spec),
    InputStickVariable('stick_y', spec),
    FlagVariable('A', spec, input_buttons, 'A_BUTTON'),
    FlagVariable('B', spec, input_buttons, 'B_BUTTON'),
    FlagVariable('Z', spec, input_buttons, 'Z_TRIG'),
    FlagVariable('S', spec, input_buttons, 'START_BUTTON'),
    StateDataVariable('global timer', spec, VariableSemantics.RAW, 'gGlobalTimer'),
  ])
