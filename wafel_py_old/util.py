from typing import *
from enum import Enum
from dataclasses import dataclass

import wafel.log as log

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

def trunc_signed(n: int, bits: int) -> int:
  d = 1 << bits
  m = n % d
  if m >= 1 << (bits - 1):
    return m - d
  else:
    return m

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
  def __repr__(self) -> str:
    return 'Ref(' + repr(self.value) + ')'
  def __str__(self) -> str:
    return 'Ref(' + str(self.value) + ')'

@dataclass(frozen=True)
class Just(Generic[T]):
  value: T

Maybe = Optional[Just[T]]

def format_align(fmt: str, line_args: Iterable[Iterable[object]]) -> List[str]:
  results = ['' for _ in line_args]

  part_fmts = fmt.split('%a')
  for k, part_fmt in enumerate(part_fmts):
    if k != len(part_fmts) - 1:
      assert part_fmt.count('%s') == 1, part_fmt
    part_fmt = part_fmt.replace('%s', '{padding}')

    lengths = [len(part_fmt.format(*args, padding='')) for args in line_args]
    max_length = max(lengths, default=0)

    for i, (args, length) in enumerate(zip(line_args, lengths)):
      padding = ' ' * (max_length - length)
      results[i] += part_fmt.format(*args, padding=padding)

  return results

def truncate_str(s: str, max_length: int, end: str = '') -> str:
  if len(s) > max_length:
    return s[:max_length - len(end)] + end
  else:
    return s

# Since id is a common variable name
py_id = id


__all__ = [
  'log',
  'dcast',
  'assert_not_none',
  'align_up',
  'align_down',
  'trunc_signed',
  'topological_sort',
  'bytes_to_buffer',
  'dict_inverse',
  'NoArg',
  'Ref',
  'Just',
  'Maybe',
  'format_align',
  'truncate_str',
  'py_id',
]
