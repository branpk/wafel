from __future__ import annotations

from typing import *
from abc import abstractmethod
from dataclasses import dataclass, field
import json


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
    return Variable(self.name, **args)

  def to_bytes(self) -> bytes:
    return json.dumps(self.__dict__).encode('utf-8')

  @staticmethod
  def from_bytes(b: bytes) -> Variable:
    return Variable(**json.loads(b.decode('utf-8')))

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


class VariableAccessor(Protocol):
  @abstractmethod
  def get(self, variable: Variable) -> object: ...

  @abstractmethod
  def set(self, variable: Variable, value: object) -> None: ...

  def edited(self, variable: Variable) -> bool:
    return False

  def reset(self, variable: Variable) -> None:
    pass


class VariableAccessorSequence(VariableAccessor):
  def __init__(self, accessors: Iterable[VariableAccessor]) -> None:
    self.accessors = tuple(accessors)

  def get(self, variable: Variable) -> object:
    for accessor in self.accessors:
      try:
        return accessor.get(variable)
      except UndefinedVariableError:
        pass
    raise UndefinedVariableError(variable)

  def set(self, variable: Variable, value: object) -> None:
    for accessor in self.accessors:
      try:
        accessor.set(variable, value)
      except UndefinedVariableError:
        pass
    raise UndefinedVariableError(variable)


__all__ = [
  'Variable',
  'UndefinedVariableError',
  'ReadOnlyVariableError',
  'VariableAccessor',
  'VariableAccessorSequence',
]
