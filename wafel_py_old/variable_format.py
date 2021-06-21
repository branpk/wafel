from typing import *
from abc import abstractmethod

from wafel_core import Variable, Pipeline



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
