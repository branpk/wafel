from typing import *
from abc import abstractmethod

from ext_modules.core import Variable


class VariableDisplayer(Protocol):
  @abstractmethod
  def label(self, variable: Variable) -> str: ...

  @abstractmethod
  def column_header(self, variable: Variable) -> str: ...
