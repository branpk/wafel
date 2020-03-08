from typing import *
from enum import Enum
from dataclasses import dataclass

T = TypeVar('T')
S = TypeVar('S')

def dcast(type_: Type[T], value: Any) -> T:
  if not isinstance(value, type_):
    raise TypeError('Could not cast ' + str(value) + ' to ' + str(type_))
  return value

def assert_not_none(value: Optional[T]) -> T:
  assert value is not None
  return value

def align_up(value: int, align: int) -> int:
  if value % align == 0:
    return value
  else:
    return value + (align - (value % align))

def align_down(value: int, align: int) -> int:
  return value - (value % align)

def topological_sort(dependencies: Dict[T, List[T]]) -> List[T]:
  deps = [(v, list(e)) for v, e in dependencies.items()]
  deps.reverse()
  result = []
  fringe = [v for v, e in deps if len(e) == 0]
  while len(fringe) > 0:
    v = fringe.pop()
    result.append(v)
    for w, e in deps:
      if v in e:
        e.remove(v)
        if len(e) == 0:
          fringe.append(w)
  if len(result) != len(deps):
    raise Exception('Graph has loop')
  return result

def bytes_to_buffer(b: bytes, n: int) -> bytes:
  return b[:n].ljust(n, b'\x00')

def dict_inverse(d: Dict[T, S]) -> Dict[S, T]:
  return {v: k for k, v in d.items()}

class NoArg(Enum):
  marker = 0

class Ref(Generic[T]):
  def __init__(self, value: T) -> None:
    self.value = value

@dataclass(frozen=True)
class Just(Generic[T]):
  value: T

Maybe = Optional[Just[T]]


__all__ = [
  'dcast',
  'assert_not_none',
  'align_up',
  'align_down',
  'topological_sort',
  'bytes_to_buffer',
  'dict_inverse',
  'NoArg',
  'Ref',
  'Just',
  'Maybe',
]
