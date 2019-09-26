from typing import *
from enum import Enum, auto

from wafel.util import *
from wafel.variable_param import *
from wafel.data_path import DataPath
from wafel.game_state import ObjectId, Object, GameState
from wafel.game_lib import GameLib
from wafel.object_type import ObjectType


class ObjectSet:
  @staticmethod
  def all() -> 'ObjectSet':
    return ObjectSet(None)

  @staticmethod
  def of(name: str) -> 'ObjectSet':
    return ObjectSet({name})

  def __init__(self, names: Optional[Set[str]]) -> None:
    self.names = names

  def __contains__(self, object_type: ObjectType) -> bool:
    if self.names is None:
      return True
    else:
      return object_type.name in self.names

  def contains(self, other: 'ObjectSet') -> bool:
    if self.names is None:
      return True
    elif other.names is None:
      return False
    return other.names.issubset(self.names)


class VariableGroup:
  @staticmethod
  def hidden() -> 'VariableGroup':
    return VariableGroup('_hidden')

  @staticmethod
  def objects(names: Optional[Set[str]] = None) -> 'VariableGroup':
    if names is None:
      return VariableGroup('_object', ObjectSet.all())
    else:
      return VariableGroup('_object', ObjectSet(names))

  @staticmethod
  def object(name: str) -> 'VariableGroup':
    return VariableGroup('_object', ObjectSet.of(name))

  def __init__(self, name: str, object_set: Optional[ObjectSet] = None) -> None:
    self.name = name
    self.object_set = object_set
    assert (self.object_set is not None) == (self.name == '_object')

  def __eq__(self, other) -> bool:
    if not isinstance(other, VariableGroup):
      return False
    return self.name == other.name and self.object_set == other.object_set

  def contains(self, other: 'VariableGroup') -> bool:
    if self.name != other.name:
      return False

    if self.object_set is not None:
      assert other.object_set is not None
      return self.object_set.contains(other.object_set)
    else:
      assert other.object_set is None
      return True


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
    group: VariableGroup,
    display_name: str,
    lib: GameLib,
    params: List[VariableParam],
    semantics: VariableSemantics,
    read_only: bool,
    data_type: VariableDataType,
  ) -> None:
    self.group = group
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
    group: VariableGroup,
    display_name: str,
    lib: GameLib,
    semantics: VariableSemantics,
    path: str,
    read_only: bool = False,
  ) -> None:
    self.path = DataPath.parse(lib, path)
    super().__init__(
      group,
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
    group: VariableGroup,
    display_name: str,
    flags: Variable,
    flag: str,
    read_only: bool = False
  ) -> None:
    super().__init__(
      group,
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
      variable.group,
      variable.display_name,
      variable.lib,
      params,
      variable.semantics,
      variable.read_only,
      variable.data_type,
    )

    self.variable = variable
    self.object_id = object_id

    self.object_pool_path = DataPath.parse(variable.lib, '$state.gObjectPool')
    self.object_struct_size = self.lib.spec['types']['struct']['Object']['size']

  def _get_args(self, args: VariableArgs) -> VariableArgs:
    object_pool_index = self.object_id
    object_addr = self.object_pool_path.get_addr(args) + \
      object_pool_index * self.object_struct_size

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

  def group(self, group: VariableGroup) -> List[Variable]:
    return [var for var in self if var.group.contains(group)]


def _all_variables(lib: GameLib) -> Variables:
  input_buttons = _DataVariable(VariableGroup.hidden(), 'buttons', lib, VariableSemantics.RAW, '$state.gControllerPads[0].button')
  active_flags = _DataVariable(VariableGroup.hidden(), 'active flags', lib, VariableSemantics.RAW, '$object.activeFlags')
  return Variables([
    input_buttons,
    _DataVariable(VariableGroup('Input'), 'stick x', lib, VariableSemantics.RAW, '$state.gControllerPads[0].stick_x'),
    _DataVariable(VariableGroup('Input'), 'stick y', lib, VariableSemantics.RAW, '$state.gControllerPads[0].stick_y'),
    _FlagVariable(VariableGroup('Input'), 'A', input_buttons, 'A_BUTTON'),
    _FlagVariable(VariableGroup('Input'), 'B', input_buttons, 'B_BUTTON'),
    _FlagVariable(VariableGroup('Input'), 'Z', input_buttons, 'Z_TRIG'),
    _FlagVariable(VariableGroup('Input'), 'S', input_buttons, 'START_BUTTON'),
    _DataVariable(VariableGroup('Misc'), 'global timer', lib, VariableSemantics.RAW, '$state.gGlobalTimer'),
    _DataVariable(VariableGroup.object('Mario'), 'mario x', lib, VariableSemantics.POSITION, '$state.gMarioState[].pos[0]'),
    _DataVariable(VariableGroup.object('Mario'), 'mario y', lib, VariableSemantics.POSITION, '$state.gMarioState[].pos[1]'),
    _DataVariable(VariableGroup.object('Mario'), 'mario z', lib, VariableSemantics.POSITION, '$state.gMarioState[].pos[2]'),
    _DataVariable(VariableGroup.object('Mario'), 'mario vel f', lib, VariableSemantics.RAW, '$state.gMarioState[].forwardVel'),
    _DataVariable(VariableGroup.object('Mario'), 'mario vel x', lib, VariableSemantics.RAW, '$state.gMarioState[].vel[0]'),
    _DataVariable(VariableGroup.object('Mario'), 'mario vel y', lib, VariableSemantics.RAW, '$state.gMarioState[].vel[1]'),
    _DataVariable(VariableGroup.object('Mario'), 'mario vel z', lib, VariableSemantics.RAW, '$state.gMarioState[].vel[2]'),

    _DataVariable(VariableGroup.objects(), 'hitbox radius', lib, VariableSemantics.RAW, '$object.hitboxRadius'),
    _DataVariable(VariableGroup.objects(), 'behavior', lib, VariableSemantics.RAW, '$object.behaviorSeg'),
    _FlagVariable(VariableGroup.objects(), 'active', active_flags, 'ACTIVE_FLAG_ACTIVE'),
  ])
