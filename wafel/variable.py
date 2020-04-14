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


class VariableReader(ABC):
  @staticmethod
  def combine_readers(choose: Callable[[Variable], VariableReader]) -> VariableReader:
    return ChooseVariableReader(choose)

  @abstractmethod
  def read(self, variable: Variable) -> object: ...


class ChooseVariableReader(VariableReader):
  def __init__(self, choose: Callable[[Variable], VariableReader]) -> None:
    self.choose = choose

  def read(self, variable: Variable) -> object:
    return self.choose(variable).read(variable)


class VariableWriter(ABC):
  @staticmethod
  def combine_writers(choose: Callable[[Variable], VariableWriter]) -> VariableWriter:
    return ChooseVariableWriter(choose)

  @abstractmethod
  def write(self, variable: Variable, value: object) -> None: ...

  @abstractmethod
  def reset(self, variable: Variable) -> None: ...


class ChooseVariableWriter(VariableWriter):
  def __init__(self, choose: Callable[[Variable], VariableWriter]) -> None:
    self.choose = choose

  def write(self, variable: Variable, value: object) -> None:
    self.choose(variable).write(variable, value)

  def reset(self, variable: Variable) -> None:
    self.choose(variable).reset(variable)


class VariablePipeline(VariableReader, VariableWriter):
  def __init__(self, writer: VariableWriter, reader: VariableReader) -> None:
    self.writer = writer
    self.reader = reader

  def read(self, variable: Variable) -> object:
    return self.reader.read(variable)

  def write(self, variable: Variable, value: object) -> None:
    self.writer.write(variable, value)

  def reset(self, variable: Variable) -> None:
    self.writer.reset(variable)


__all__ = [
  'Variable',
  'UndefinedVariableError',
  'ReadOnlyVariableError',
  'VariableReader',
  'VariableWriter',
  'VariablePipeline',
]
