from typing import *
from enum import Enum

import imgui as ig

from wafel.util import *


# TODO: Garbage collection?
local_state: Dict[Tuple[int, str], Any] = {}


T = TypeVar('T')


def use_state_with(name: str, default: Callable[[], T]) -> Ref[T]:
  key = (ig.get_id(''), name)
  ref = local_state.get(key)
  if ref is None:
    ref = Ref(default())
    local_state[key] = ref
  return ref


def use_state(name: str, default: T) -> Ref[T]:
  return use_state_with(name, lambda: default)
