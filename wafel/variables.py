import ctypes as C

from wafel.game_state import GameState
from wafel.variable import *
from wafel.data_path import DataPath
from wafel.util import *


class _DataVariable(Variable):
  def __init__(
    self,
    display_name: str,
    spec: dict,
    semantics: VariableSemantics,
    path: str,
    read_only: bool = False,
  ) -> None:
    self.path = DataPath.parse(spec, path)
    super().__init__(
      display_name,
      self.path.params,
      semantics,
      read_only,
      VariableDataType.from_spec(spec, self.path.type),
    )

  def get(self, args: VariableArgs) -> Any:
    return self.path.get(args)

  def set(self, value: Any, args: VariableArgs) -> None:
    assert not self.read_only
    self.path.set(value, args)


class _FlagVariable(Variable):
  def __init__(
    self,
    display_name: str,
    spec: dict,
    flags: Variable,
    flag: str,
    read_only: bool = False
  ) -> None:
    super().__init__(
      display_name,
      flags.params,
      VariableSemantics.FLAG,
      read_only or flags.read_only,
      VariableDataType.BOOL,
    )
    self.flags = flags
    self.flag = spec['constants'][flag]['value']

  def get(self, args: VariableArgs) -> bool:
    return (self.flags.get(args) & self.flag) != 0

  def set(self, value: bool, args: VariableArgs) -> None:
    assert not self.read_only
    flags = self.flags.get(args)
    if value:
      flags |= self.flag
    else:
      flags &= ~self.flag
    self.flags.set(flags, args)


def create_variables(spec: dict) -> List[Variable]:
  input_buttons = _DataVariable('buttons', spec, VariableSemantics.RAW, '$state.gControllerPads[0].button')
  return [
    input_buttons,
    _DataVariable('stick x', spec, VariableSemantics.RAW, '$state.gControllerPads[0].stick_x'),
    _DataVariable('stick y', spec, VariableSemantics.RAW, '$state.gControllerPads[0].stick_y'),
    _FlagVariable('A', spec, input_buttons, 'A_BUTTON'),
    _FlagVariable('B', spec, input_buttons, 'B_BUTTON'),
    _FlagVariable('Z', spec, input_buttons, 'Z_TRIG'),
    _FlagVariable('S', spec, input_buttons, 'START_BUTTON'),
    _DataVariable('global timer', spec, VariableSemantics.RAW, '$state.gGlobalTimer'),
    _DataVariable('mario x', spec, VariableSemantics.POSITION, '$state.gMarioState[].pos[0]'),
    _DataVariable('mario y', spec, VariableSemantics.POSITION, '$state.gMarioState[].pos[1]'),
    _DataVariable('mario z', spec, VariableSemantics.POSITION, '$state.gMarioState[].pos[2]'),
    _DataVariable('mario vel f', spec, VariableSemantics.RAW, '$state.gMarioState[].forwardVel'),
    _DataVariable('mario vel x', spec, VariableSemantics.RAW, '$state.gMarioState[].vel[0]'),
    _DataVariable('mario vel y', spec, VariableSemantics.RAW, '$state.gMarioState[].vel[1]'),
    _DataVariable('mario vel z', spec, VariableSemantics.RAW, '$state.gMarioState[].vel[2]'),

    # _DataVariable('hitbox radius', spec, VariableSemantics.RAW, '$object.hitboxRadius'),
  ]
