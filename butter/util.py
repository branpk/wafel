from typing import Type, Any, TypeVar

T = TypeVar('T')

def dcast(type_: Type[T], value: Any) -> T:
  if not isinstance(value, type_):
    raise TypeError('Could not cast ' + str(value) + ' to ' + str(type_))
  return value


def concrete_type(spec: dict, type_: dict) -> dict:
  while type_['kind'] == 'symbol':
    type_ = spec['types'][type_['namespace']][type_['name']]
  return type_

def align_up(value: int, align: int) -> int:
  while value % align != 0:
    value += 1
  return value
