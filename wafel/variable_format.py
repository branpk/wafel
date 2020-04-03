from typing import *

from wafel.variable import Variable, VariableDataType, VariableSemantics


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
    assert isinstance(data, bool)
    return data

  def input(self, rep: object) -> object:
    assert isinstance(rep, bool)
    return rep


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


class Formatters:
  def __init__(self) -> None:
    self.overrides: Dict[Variable, VariableFormatter] = {}

  def _get_default(self, variable: Variable) -> VariableFormatter:
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

    elif variable.semantics == VariableSemantics.SCRIPT:
      return StringFormatter()

    raise NotImplementedError(variable, variable.data_type)

  def __getitem__(self, variable: Variable) -> VariableFormatter:
    return self.overrides.get(variable) or self._get_default(variable)

  def __setitem__(self, variable: Variable, formatter: VariableFormatter) -> None:
    self.overrides[variable] = formatter
