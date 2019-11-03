import typing as _tp

_T = _tp.TypeVar('T')

def dcast(type_: _tp.Type[_T], value: _tp.Any) -> _T:
  if not isinstance(value, type_):
    raise TypeError('Could not cast ' + str(value) + ' to ' + str(type_))
  return value

def align_up(value: int, align: int) -> int:
  while value % align != 0:
    value += 1
  return value

def topological_sort(dependencies: _tp.Dict[_T, _tp.List[_T]]) -> _tp.List[_T]:
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
