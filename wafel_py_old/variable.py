from __future__ import annotations

from wafel_core import Variable

from typing import *
from abc import abstractmethod, ABC
from dataclasses import dataclass, field
import pickle
from wafel.util import *


class VariableReader(Protocol):
  @abstractmethod
  def read(self, variable: Variable) -> object: ...


class VariableWriter(Protocol):
  @abstractmethod
  def write(self, variable: Variable, value: object) -> None: ...

  @abstractmethod
  def reset(self, variable: Variable) -> None: ...


class VariablePipeline(Protocol):
  @abstractmethod
  def read(self, variable: Variable) -> object: ...

  @abstractmethod
  def write(self, variable: Variable, value: object) -> None: ...

  @abstractmethod
  def reset(self, variable: Variable) -> None: ...


__all__ = [
  'VariableReader',
  'VariableWriter',
  'VariablePipeline',
]
