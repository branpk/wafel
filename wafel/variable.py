from __future__ import annotations

from typing import *
from enum import Enum, auto
import json
from dataclasses import dataclass
import functools

from wafel.util import *
from wafel.core import DataPath, Timeline, Game, spec_get_concrete_type, Slot
from wafel.object_type import ObjectType
from wafel.script import Scripts


ObjectId = int

class Object:
  def __init__(self, addr: int) -> None:
    self.addr = addr


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
  SCRIPT = auto()


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
  OTHER = auto()

  @staticmethod
  def from_spec(game: Game, type_: dict) -> 'VariableDataType':
    type_ = spec_get_concrete_type(game.memory.data_spec, type_)
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
      elem_type = VariableDataType.from_spec(game, type_['base'])
      if type_['length'] == 3 and elem_type == VariableDataType.F32:
        return VariableDataType.VEC3F
      else:
        raise NotImplementedError(type_)
    elif type_['kind'] == 'pointer': # TODO: Remove
      return VariableDataType.U32
    else:
      raise NotImplementedError(type_['kind'])


@dataclass(frozen=True)
class VariableId:
  name: str
  object_id: Optional[ObjectId] = None
  surface: Optional[int] = None

  def with_name(self, name: str) -> VariableId:
    return VariableId(name, self.object_id, self.surface)

  def with_object_id(self, object_id: ObjectId) -> VariableId:
    return VariableId(self.name, object_id=object_id)

  def with_surface(self, surface: int) -> VariableId:
    return VariableId(self.name, surface=surface)

  def to_bytes(self) -> bytes:
    return json.dumps(self.__dict__).encode('utf-8')

  @staticmethod
  def from_bytes(b: bytes) -> 'VariableId':
    return VariableId(**json.loads(b.decode('utf-8')))

  def __repr__(self) -> str:
    args = [self.name]
    if self.object_id is not None:
      args.append('obj=' + str(self.object_id))
    if self.surface is not None:
      args.append('surf=' + str(self.surface))
    return 'Variable(' + ', '.join(args) + ')'


class Variable:
  @staticmethod
  def create_all(game: Game) -> 'Variables':
    return _all_variables(game)

  def __init__(
    self,
    id: VariableId,
    group: VariableGroup,
    label: str,
    game: Game,
    semantics: VariableSemantics,
    read_only: bool,
    data_type: VariableDataType,
  ) -> None:
    self.id = id
    self.group = group
    self.label = label
    self.game = game
    self.semantics = semantics
    self.read_only = read_only
    self.data_type = data_type

  def get(self, timeline: Timeline, frame: int) -> object:
    raise NotImplementedError

  def set(self, slot: Slot, value: object) -> None:
    raise NotImplementedError

  def at_object_slot(self, object_id: ObjectId, slot: int) -> Variable:
    raise NotImplementedError

  def at_surface(self, surface: int) -> Variable:
    raise NotImplementedError

  def at_object(self, object_id: ObjectId) -> Variable:
    return self.at_object_slot(object_id, object_id)

  def get_object_id(self) -> Optional[ObjectId]:
    return self.id.object_id

  def __repr__(self) -> str:
    return repr(self.id)

  def __eq__(self, other: object) -> bool:
    return isinstance(other, Variable) and self.id == other.id

  def __hash__(self) -> int:
    return hash(self.id)


class _DataVariable(Variable):
  def __init__(
    self,
    id: VariableId,
    group: VariableGroup,
    label: str,
    game: Game,
    semantics: VariableSemantics,
    path: DataPath,
    read_only: bool = False,
  ) -> None:
    self.path = path
    super().__init__(
      id,
      group,
      label,
      game,
      semantics,
      read_only,
      VariableDataType.from_spec(game, self.path.end_type),
    )

  def get(self, timeline: Timeline, frame: int) -> object:
    return timeline.get(frame, self.path)

  def set(self, slot: Slot, value: object) -> None:
    assert not self.read_only
    self.path.set(slot, value)

  def at_object_slot(self, object_id: ObjectId, slot: int) -> Variable:
    object_path = self.game.path(f'gObjectPool[{slot}]')
    return _DataVariable(
      self.id.with_object_id(object_id),
      self.group,
      self.label,
      self.game,
      self.semantics,
      object_path + self.path,
      self.read_only,
    )

  def at_surface(self, surface: int) -> Variable:
    surface_path = self.game.path(f'sSurfacePool[{surface}]')
    return _DataVariable(
      self.id.with_surface(surface),
      self.group,
      self.label,
      self.game,
      self.semantics,
      surface_path + self.path,
      self.read_only,
    )


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
      flags.game,
      VariableSemantics.FLAG,
      read_only or flags.read_only,
      VariableDataType.BOOL,
    )
    self.flags = flags
    self.flag_str = flag
    self.flag = dcast(int, self.game.memory.data_spec['constants'][flag]['value'])

  def get(self, timeline: Timeline, frame: int) -> object:
    flags = dcast(int, self.flags.get(timeline, frame))
    return (flags & self.flag) != 0

  def set(self, slot: Slot, value: object) -> None:
    assert not self.read_only
    assert isinstance(value, bool)
    assert isinstance(self.flags, _DataVariable)
    flags = dcast(int, self.flags.path.get(slot))
    if value:
      flags |= self.flag
    else:
      flags &= ~self.flag
    self.flags.set(slot, flags)

  def at_object_slot(self, object_id: ObjectId, slot: int) -> Variable:
    return _FlagVariable(
      self.id.name,
      self.group,
      self.label,
      self.flags.at_object_slot(object_id, slot),
      self.flag_str,
      self.read_only,
    )

  def at_surface(self, surface: int) -> Variable:
    return _FlagVariable(
      self.id.name,
      self.group,
      self.label,
      self.flags.at_surface(surface),
      self.flag_str,
      self.read_only,
    )


class ScriptVariable(Variable):
  def __init__(self, game: Game, scripts: Scripts) -> None:
    super().__init__(
      VariableId('wafel-script'),
      VariableGroup('Scripting'),
      'script',
      game,
      VariableSemantics.SCRIPT,
      False,
      VariableDataType.OTHER,
    )
    self.scripts = scripts

  def get(self, timeline: Timeline, frame: int) -> object:
    return self.scripts.get(frame).source


class VariableSpec:
  def __init__(self) -> None:
    pass

  def label(self, label: str) -> 'VariableSpec':
    self._label: Optional[str] = label
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
    self.variables_by_id = {
      var.id: var for var in self.variables
    }

  def __iter__(self) -> Iterator[Variable]:
    return iter(self.variables)

  @functools.lru_cache(maxsize=None)
  def __getitem__(self, id: Union[str, VariableId]) -> Variable:
    if isinstance(id, str):
      id = VariableId(id)
    if id.object_id is not None:
      return self[VariableId(id.name)].at_object(id.object_id)
    elif id.surface is not None:
      return self[VariableId(id.name)].at_surface(id.surface)
    else:
      return self.variables_by_id[id]

  def group(self, group: VariableGroup) -> List[Variable]:
    return [var for var in self if var.group.contains(group)]

  def add(self, variable: Variable) -> None:
    if variable.id in self.variables_by_id:
      self.variables.remove(self.variables_by_id[variable.id])
    self.variables.append(variable)
    self.variables_by_id[variable.id] = variable


def _all_variables(game: Game) -> Variables:
  from wafel.variables import VARIABLES

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

    variable: Variable
    if isinstance(spec, DataVariableSpec):
      variable = _DataVariable(
        VariableId(name),
        group,
        label,
        game,
        VariableSemantics.RAW,
        game.path(spec.path),
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
