import ctypes as C

from wafel.game_state import GameState
from wafel.variable import *
from wafel.data_path import DataPath
from wafel.util import *


class _DataVariable(Variable):
  def __init__(
    self,
    name: str,
    spec: dict,
    semantics: VariableSemantics,
    path: str,
    read_only: bool = False,
  ) -> None:
    self.path = DataPath.parse(spec, path)
    super().__init__(
      name,
      self.path.params,
      semantics,
      read_only,
      VariableDataType.from_spec(spec, self.path.type),
    )

  def get(self, *args: Any) -> Any:
    return self.path.get(*args)

  def set(self, value: Any, *args: Any) -> None:
    assert not self.read_only
    self.path.set(value, *args)


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
    self.flag = spec['constants'][flag]['value']

  def get(self, *args: Any) -> bool:
    return (self.flags.get(*args) & self.flag) != 0

  def set(self, value: bool, *args: Any) -> None:
    assert not self.read_only
    flags = self.flags.get(*args)
    if value:
      flags |= self.flag
    else:
      flags &= ~self.flag
    self.flags.set(flags, *args)


def create_variables(spec: dict) -> Variables:
  input_buttons = _DataVariable('buttons', spec, VariableSemantics.RAW, '$state.gControllerPads[0].button')
  return Variables([
    input_buttons,
    _DataVariable('stick x', spec, VariableSemantics.RAW, '$state.gControllerPads[0].stick_x'),
    _DataVariable('stick y', spec, VariableSemantics.RAW, '$state.gControllerPads[0].stick_y'),
    _FlagVariable('A', spec, input_buttons, 'A_BUTTON'),
    _FlagVariable('B', spec, input_buttons, 'B_BUTTON'),
    _FlagVariable('Z', spec, input_buttons, 'Z_TRIG'),
    _FlagVariable('S', spec, input_buttons, 'START_BUTTON'),
    _DataVariable('global timer', spec, VariableSemantics.RAW, '$state.gGlobalTimer'),
    _DataVariable('mario pos', spec, VariableSemantics.POSITION, '$state.gMarioState[].pos'),
  ])
