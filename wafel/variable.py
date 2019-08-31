from typing import *
from enum import Enum, auto

from wafel.util import *


class VariableParam(Enum):
  STATE = auto()
  OBJECT = auto()


VariableArgs = Dict[VariableParam, Any]


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
  def from_spec(spec: dict, type_: dict) -> 'VariableDataType':
    type_ = concrete_type(spec, type_)
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
      elem_type = VariableDataType.from_spec(spec, type_['base'])
      if type_['length'] == 3 and elem_type == VariableDataType.F32:
        return VariableDataType.VEC3F
      else:
        raise NotImplementedError(type_)
    else:
      raise NotImplementedError(type_['kind'])


class Variable:
  def __init__(
    self,
    display_name: str,
    params: List[VariableParam],
    semantics: VariableSemantics,
    read_only: bool,
    data_type: VariableDataType,
  ) -> None:
    self.display_name = display_name
    self.params = params
    self.semantics = semantics
    self.read_only = read_only
    self.data_type = data_type

  def get(self, args: VariableArgs) -> Any:
    raise NotImplementedError

  def set(self, value: Any, args: VariableArgs) -> None:
    raise NotImplementedError

  def __repr__(self) -> str:
    return f'Variable({self.display_name})'


class Formatter:
  @staticmethod
  def default(variable: Variable) -> 'Formatter':
    if variable.data_type == VariableDataType.BOOL:
      return CheckboxFormatter()

    elif variable.data_type in [
      VariableDataType.S8,
      VariableDataType.S16,
      VariableDataType.S32,
      VariableDataType.S64,
      VariableDataType.U8,
      VariableDataType.U16,
      VariableDataType.U32,
      VariableDataType.U64,
    ]:
      return DecimalIntFormatter()

    elif variable.data_type in [
      VariableDataType.F32,
      VariableDataType.F64,
    ]:
      return FloatFormatter()

    raise NotImplementedError(variable, variable.data_type)

  def output(self, data: Any) -> Any:
    raise NotImplementedError

  def input(self, rep: Any) -> Any:
    raise NotImplementedError


# TODO: Signed, unsigned, int sizes
class DecimalIntFormatter(Formatter):
  def output(self, data):
    assert isinstance(data, int)
    return str(data)

  def input(self, rep):
    assert isinstance(rep, str)
    return int(rep, base=0)


# TODO: Precision
class FloatFormatter(Formatter):
  def output(self, data):
    assert isinstance(data, float)
    return str(data)

  def input(self, rep):
    assert isinstance(rep, str)
    return float(rep)


class CheckboxFormatter(Formatter):
  def output(self, data):
    assert isinstance(data, bool)
    return data

  def input(self, rep):
    assert isinstance(rep, bool)
    return rep


class VariableInstance:

  # TODO: Associated object etc
  def __init__(self, variable: Variable, formatter: Formatter) -> None:
    self.variable = variable
    self.formatter = formatter

  @property
  def params(self) -> List[VariableParam]:
    return self.variable.params

  @property
  def read_only(self) -> bool:
    return self.variable.read_only

  @property
  def display_name(self) -> str:
    return self.variable.display_name

  def get_data(self, args: VariableArgs) -> Any:
    return self.variable.get(args)
