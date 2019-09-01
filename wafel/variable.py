from typing import *
from enum import Enum, auto

from wafel.util import *
from wafel.variable_param import *
from wafel.data_path import DataPath
from wafel.game_state import ObjectId, Object, GameState
from wafel.game_lib import GameLib


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
  VEC3F = auto()

  @staticmethod
  def from_spec(lib: GameLib, type_: dict) -> 'VariableDataType':
    type_ = lib.concrete_type(type_)
    if type_['kind'] == 'primitive':
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
    elif type_['kind'] == 'array':
      elem_type = VariableDataType.from_spec(lib, type_['base'])
      if type_['length'] == 3 and elem_type == VariableDataType.F32:
        return VariableDataType.VEC3F
      else:
        raise NotImplementedError(type_)
    elif type_['kind'] == 'pointer': # TODO: Remove
      return VariableDataType.U32
    else:
      raise NotImplementedError(type_['kind'])


class Variable:
  @staticmethod
  def create_all(lib: GameLib) -> 'Variables':
    return _all_variables(lib)

  def __init__(
    self,
    display_name: str,
    lib: GameLib,
    params: List[VariableParam],
    semantics: VariableSemantics,
    read_only: bool,
    data_type: VariableDataType,
  ) -> None:
    self.display_name = display_name
    self.lib = lib
    self.params = params
    self.semantics = semantics
    self.read_only = read_only
    self.data_type = data_type

  def get(self, args: VariableArgs) -> Any:
    raise NotImplementedError

  def set(self, value: Any, args: VariableArgs) -> None:
    raise NotImplementedError

  def at_object(self, object_id: ObjectId) -> 'Variable':
    return _ObjectVariable(self, object_id)

  def __repr__(self) -> str:
    return f'Variable({self.display_name})'


class _DataVariable(Variable):
  def __init__(
    self,
    display_name: str,
    lib: GameLib,
    semantics: VariableSemantics,
    path: str,
    read_only: bool = False,
  ) -> None:
    self.path = DataPath.parse(lib, path)
    super().__init__(
      display_name,
      lib,
      self.path.params,
      semantics,
      read_only,
      VariableDataType.from_spec(lib, self.path.type),
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
    flags: Variable,
    flag: str,
    read_only: bool = False
  ) -> None:
    super().__init__(
      display_name,
      flags.lib,
      flags.params,
      VariableSemantics.FLAG,
      read_only or flags.read_only,
      VariableDataType.BOOL,
    )
    self.flags = flags
    self.flag = self.flags.lib.spec['constants'][flag]['value']

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


class _ObjectVariable(Variable):
  def __init__(self, variable: Variable, object_id: ObjectId):
    params = [p for p in variable.params if p != VariableParam.OBJECT]
    if VariableParam.STATE not in params:
      params.append(VariableParam.STATE)

    super().__init__(
      variable.display_name,
      variable.lib,
      params,
      variable.semantics,
      variable.read_only,
      variable.data_type,
    )

    self.variable = variable
    self.object_id = object_id

  def _get_args(self, args: VariableArgs) -> VariableArgs:
    object_path = '$state.gObjectPool[' + str(self.object_id) + ']'
    object_addr = DataPath.parse(self.variable.lib, object_path).get_addr(args)

    new_args = {
      VariableParam.OBJECT: Object(object_addr),
    }
    new_args.update(args)
    return new_args

  def get(self, args: VariableArgs) -> Any:
    return self.variable.get(self._get_args(args))

  def set(self, value: Any, args: VariableArgs) -> None:
    self.variable.set(value, self._get_args(args))


class Variables:
  def __init__(self, variables: List[Variable]) -> None:
    self.variables = variables

  def __iter__(self) -> Iterator[Variable]:
    return iter(self.variables)

  def __getitem__(self, name: str) -> Variable:
    return [var for var in self.variables if var.display_name == name][0]


def _all_variables(lib: GameLib) -> Variables:
  input_buttons = _DataVariable('buttons', lib, VariableSemantics.RAW, '$state.gControllerPads[0].button')
  return Variables([
    input_buttons,
    _DataVariable('stick x', lib, VariableSemantics.RAW, '$state.gControllerPads[0].stick_x'),
    _DataVariable('stick y', lib, VariableSemantics.RAW, '$state.gControllerPads[0].stick_y'),
    _FlagVariable('A', input_buttons, 'A_BUTTON'),
    _FlagVariable('B', input_buttons, 'B_BUTTON'),
    _FlagVariable('Z', input_buttons, 'Z_TRIG'),
    _FlagVariable('S', input_buttons, 'START_BUTTON'),
    _DataVariable('global timer', lib, VariableSemantics.RAW, '$state.gGlobalTimer'),
    _DataVariable('mario x', lib, VariableSemantics.POSITION, '$state.gMarioState[].pos[0]'),
    _DataVariable('mario y', lib, VariableSemantics.POSITION, '$state.gMarioState[].pos[1]'),
    _DataVariable('mario z', lib, VariableSemantics.POSITION, '$state.gMarioState[].pos[2]'),
    _DataVariable('mario vel f', lib, VariableSemantics.RAW, '$state.gMarioState[].forwardVel'),
    _DataVariable('mario vel x', lib, VariableSemantics.RAW, '$state.gMarioState[].vel[0]'),
    _DataVariable('mario vel y', lib, VariableSemantics.RAW, '$state.gMarioState[].vel[1]'),
    _DataVariable('mario vel z', lib, VariableSemantics.RAW, '$state.gMarioState[].vel[2]'),

    _DataVariable('hitbox radius', lib, VariableSemantics.RAW, '$object.hitboxRadius'),
    _DataVariable('behavior', lib, VariableSemantics.RAW, '$object.behaviorSeg'),
  ])
