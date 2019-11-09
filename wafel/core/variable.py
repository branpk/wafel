from typing import *
from enum import Enum, auto
import json

from wafel.util import *
from wafel.core.variable_param import *
from wafel.core.data_path import DataPath
from wafel.core.game_state import ObjectId, Object
from wafel.core.game_lib import GameLib
from wafel.core.object_type import ObjectType


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
  def objects(names: Set[str]) -> 'VariableGroup':
    return VariableGroup('_object', ObjectSet(names))

  @staticmethod
  def all_objects() -> 'VariableGroup':
    return VariableGroup('_object', ObjectSet.all())

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

  def __hash__(self) -> int:
    return hash((self.name, self.object_set))

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


class VariableId:
  def __init__(
    self,
    name: str,
    object_id: Optional[ObjectId] = None,
  ) -> None:
    self.name = name
    self.object_id = object_id

  def with_name(self, name: str) -> 'VariableId':
    return VariableId(name, self.object_id)

  def with_object_id(self, object_id: ObjectId) -> 'VariableId':
    return VariableId(self.name, object_id)

  def _args(self) -> Tuple[Any, ...]:
    return (self.name, self.object_id)

  def to_bytes(self) -> bytes:
    return json.dumps(self._args()).encode('utf-8')

  @staticmethod
  def from_bytes(b: bytes) -> 'VariableId':
    return VariableId(*json.loads(b.decode('utf-8')))

  def __eq__(self, other) -> bool:
    return isinstance(other, VariableId) and self._args() == other._args()

  def __hash__(self) -> int:
    return hash(self._args())

  def __repr__(self) -> str:
    args = [self.name]
    if self.object_id is not None:
      args.append('obj=' + str(self.object_id))
    return 'Variable(' + ', '.join(args) + ')'


class Variable:
  @staticmethod
  def create_all(lib: GameLib) -> 'Variables':
    return _all_variables(lib)

  def __init__(
    self,
    id: VariableId,
    group: VariableGroup,
    label: str,
    lib: GameLib,
    params: List[VariableParam],
    semantics: VariableSemantics,
    read_only: bool,
    data_type: VariableDataType,
  ) -> None:
    self.id = id
    self.group = group
    self.label = label
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

  def get_object_id(self) -> Optional[ObjectId]:
    return self.id.object_id

  def __repr__(self) -> str:
    return repr(self.id)

  def __eq__(self, other: 'Variable') -> bool:
    return isinstance(other, Variable) and self.id == other.id

  def __hash__(self) -> int:
    return hash(self.id)


class _DataVariable(Variable):
  def __init__(
    self,
    name: str,
    group: VariableGroup,
    label: str,
    lib: GameLib,
    semantics: VariableSemantics,
    path: str,
    read_only: bool = False,
  ) -> None:
    self.path = DataPath.parse(lib, path)
    super().__init__(
      VariableId(name),
      group,
      label,
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
    name: str,
    group: VariableGroup,
    label: str,
    flags: Variable,
    flag: str,
    read_only: bool = False
  ) -> None:
    super().__init__(
      flags.id.with_name(name),
      group,
      label,
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
      variable.id.with_object_id(object_id),
      variable.group,
      variable.label,
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


class VariableSpec:
  def __init__(self) -> None:
    pass

  def label(self, label: str) -> 'VariableSpec':
    self._label = label
    return self

  def hidden(self) -> 'VariableSpec':
    self._label = None
    return self

  def _dependencies(self) -> List[str]:
    return []

class DataVariableSpec(VariableSpec):
  def __init__(self, path: str) -> None:
    self.path = path

class FlagVariableSpec(VariableSpec):
  def __init__(self, flags: str, flag: str) -> None:
    self.flags = flags
    self.flag = flag

  def _dependencies(self) -> List[str]:
    return [self.flags]


class Variables:
  def __init__(self, variables: List[Variable]) -> None:
    self.variables = variables

  def __iter__(self) -> Iterator[Variable]:
    return iter(self.variables)

  def __getitem__(self, id: Union[str, VariableId]) -> Variable:
    if isinstance(id, str):
      id = VariableId(id)
    if id.object_id is not None:
      return self[VariableId(id.name)].at_object(id.object_id)
    else:
      matches = [var for var in self.variables if var.id == id]
      if len(matches) == 0:
        raise KeyError(id)
      return matches[0]

  def group(self, group: VariableGroup) -> List[Variable]:
    return [var for var in self if var.group.contains(group)]


def _all_variables(lib: GameLib) -> Variables:
  from wafel.core.variables import VARIABLES

  info: Dict[str, Tuple[VariableGroup, VariableSpec]] = {}
  for group, group_variables in VARIABLES.items():
    for name, spec in group_variables.items():
      if name in info:
        raise Exception('Duplicate variable ' + name)
      info[name] = (group, spec)

  dependencies = {
    name: spec._dependencies() for name, (_, spec) in info.items()
  }
  ordered = topological_sort(dependencies)

  variables: Dict[str, Variable] = {}

  for name in ordered:
    group, spec = info[name]

    if not hasattr(spec, '_label'):
      raise Exception('Missing label or .hidden(): ' + name)
    label = spec._label
    if label is None:
      label = name
      group = VariableGroup.hidden()

    if isinstance(spec, DataVariableSpec):
      variable = _DataVariable(
        name,
        group,
        label,
        lib,
        VariableSemantics.RAW,
        spec.path,
      )
    elif isinstance(spec, FlagVariableSpec):
      flags = variables[spec.flags]
      variable = _FlagVariable(
        name,
        group,
        label,
        flags,
        spec.flag,
      )
    else:
      raise NotImplementedError(spec)

    variables[name] = variable

  return Variables(list(variables.values()))
