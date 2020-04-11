from __future__ import annotations

from typing import *
from abc import abstractmethod, ABC
from dataclasses import dataclass, field
import pickle
from wafel.util import *


@dataclass(frozen=True)
class Variable:
  name: str
  args: Dict[str, Any]
  _hash: int

  def __init__(self, name: str, args: Dict[str, Any] = {}, **kwargs) -> None:
    super().__setattr__('name', name)
    super().__setattr__('args', dict(args))
    self.args.update(kwargs)
    super().__setattr__('_hash', hash((self.name, tuple(self.args.items()))))

  def at(self, **kwargs) -> Variable:
    args = dict(self.args)
    args.update(kwargs)
    return Variable(self.name, args)

  def without(self, arg: str) -> Variable:
    args = dict(self.args)
    if arg in args:
      del args[arg]
    return Variable(self.name, args)

  def to_bytes(self) -> bytes:
    return pickle.dumps(self)

  @staticmethod
  def from_bytes(b: bytes) -> Variable:
    return dcast(Variable, pickle.loads(b))

  def __hash__(self) -> int:
    return self._hash

  def __str__(self) -> str:
    if len(self.args) > 0:
      return self.name + '[' + ', '.join([f'{k}={v}' for k, v in self.args.items()]) + ']'
    else:
      return self.name


class UndefinedVariableError(Exception):
  pass

class ReadOnlyVariableError(Exception):
  pass


class VariableAccessor(ABC):
  @staticmethod
  def combine(choose: Callable[[Variable], VariableAccessor]) -> VariableAccessor:
    return DeterminedVariableAccessor(choose)

  @abstractmethod
  def get(self, variable: Variable) -> object: ...

  @abstractmethod
  def set(self, variable: Variable, value: object) -> None: ...

  @abstractmethod
  def reset(self, variable: Variable) -> None: ...

  def edited(self, variable: Variable) -> bool:
    return False


class DeterminedVariableAccessor(VariableAccessor):
  def __init__(self, choose: Callable[[Variable], VariableAccessor]) -> None:
    self.choose = choose

  def get(self, variable: Variable) -> object:
    return self.choose(variable).get(variable)

  def set(self, variable: Variable, value: object) -> None:
    self.choose(variable).set(variable, value)

  def reset(self, variable: Variable) -> None:
    self.choose(variable).reset(variable)

  def edited(self, variable: Variable) -> bool:
    return self.choose(variable).edited(variable)


__all__ = [
  'Variable',
  'UndefinedVariableError',
  'ReadOnlyVariableError',
  'VariableAccessor',
]
