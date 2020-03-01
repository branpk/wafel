from typing import *
from enum import Enum

import imgui as ig

from wafel.util import *


# TODO: Garbage collection?
local_state: Dict[Tuple[int, str], Any] = {}


T = TypeVar('T')


def get_state(
  name: str,
  default: Union[NoArg, T] = NoArg.marker,
  get_default: Union[NoArg, Callable[[], T]] = NoArg.marker,
) -> T:
  key = (ig.get_id(''), name)
  if key in local_state:
    return cast(T, local_state[key])

  if default is not NoArg.marker:
    assert get_default is NoArg.marker
    value = default
  else:
    assert get_default is not NoArg.marker
    value = get_default()

  local_state[key] = value
  return value


def set_state(name: str, value: T) -> None:
  local_state[(ig.get_id(''), name)] = value
