from typing import Type, Any, TypeVar

T = TypeVar('T')

def dcast(type_: Type[T], value: Any) -> T:
  if not isinstance(value, type_):
    raise TypeError('Could not cast ' + str(value) + ' to ' + str(type_))
  return value
