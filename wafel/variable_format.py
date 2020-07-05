from typing import *
from abc import abstractmethod

from ext_modules.core import Variable, Pipeline


class VariableFormatter:
  def output(self, data: object) -> object:
    raise NotImplementedError

  def input(self, rep: object) -> object:
    raise NotImplementedError


class TextFormatter(VariableFormatter):
  pass


class EmptyFormatter(TextFormatter): # TODO: Implement better UI than empty text
  def output(self, data: object) -> object:
    assert data is None
    return ''

  def input(self, rep: object) -> object:
    assert rep == ''
    return None


# TODO: Signed, unsigned, int sizes
class DecimalIntFormatter(TextFormatter):
  def output(self, data: object) -> object:
    assert isinstance(data, int)
    return str(data)

  def input(self, rep: object) -> object:
    assert isinstance(rep, str)
    return int(rep, base=0)


# TODO: Precision
class FloatFormatter(TextFormatter):
  def output(self, data: object) -> object:
    assert isinstance(data, float)
    return str(data)

  def input(self, rep: object) -> object:
    assert isinstance(rep, str)
    return float(rep)


class CheckboxFormatter(VariableFormatter):
  def output(self, data: object) -> object:
    assert isinstance(data, int)
    return bool(data)

  def input(self, rep: object) -> object:
    assert isinstance(rep, int)
    return int(bool(rep))


class EnumFormatter(TextFormatter):
  def __init__(self, id_to_name: Dict[int, str]) -> None:
    self.id_to_name = id_to_name
    self.name_to_id = { v: k for k, v in id_to_name.items() }

  def output(self, data: object) -> object:
    assert isinstance(data, int)
    return self.id_to_name[data]

  def input(self, rep: object) -> object:
    assert isinstance(rep, str)
    try:
      return int(rep, base=0)
    except:
      return self.name_to_id[rep]


class StringFormatter(TextFormatter):
  def output(self, data: object) -> object:
    assert isinstance(data, str)
    return data

  def input(self, rep: object) -> object:
    assert isinstance(rep, str)
    return rep


class Formatters(Protocol):
  @abstractmethod
  def __getitem__(self, variable: Variable) -> VariableFormatter: ...


class DataFormatters:
  def __init__(self, pipeline: Pipeline) -> None:
    self.pipeline = pipeline
    self.overrides: Dict[str, VariableFormatter] = {}

  def _get_default(self, variable: Variable) -> VariableFormatter:
    if variable.name == 'wafel-script':
      return StringFormatter()

    if self.pipeline.is_bit_flag(variable):
      return CheckboxFormatter()
    elif self.pipeline.is_int(variable):
      return DecimalIntFormatter()
    elif self.pipeline.is_float(variable):
      return FloatFormatter()

    raise NotImplementedError(variable)

  def __getitem__(self, variable: Variable) -> VariableFormatter:
    return self.overrides.get(variable.name) or self._get_default(variable)

  def __setitem__(self, variable: Variable, formatter: VariableFormatter) -> None:
    self.overrides[variable.name] = formatter
