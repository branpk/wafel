from typing import *

from wafel.variable import Variable, VariableDataType


class VariableFormatter:
  def output(self, data: Any) -> Any:
    raise NotImplementedError

  def input(self, rep: Any) -> Any:
    raise NotImplementedError


# TODO: Signed, unsigned, int sizes
class DecimalIntFormatter(VariableFormatter):
  def output(self, data):
    assert isinstance(data, int)
    return str(data)

  def input(self, rep):
    assert isinstance(rep, str)
    return int(rep, base=0)


# TODO: Precision
class FloatFormatter(VariableFormatter):
  def output(self, data):
    assert isinstance(data, float)
    return str(data)

  def input(self, rep):
    assert isinstance(rep, str)
    return float(rep)


class CheckboxFormatter(VariableFormatter):
  def output(self, data):
    assert isinstance(data, bool)
    return data

  def input(self, rep):
    assert isinstance(rep, bool)
    return rep


class Formatters:
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

    raise NotImplementedError(variable, variable.data_type)

  def __getitem__(self, variable: Variable) -> VariableFormatter:
    return self._get_default(variable)
