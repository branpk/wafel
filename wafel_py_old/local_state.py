from typing import *
from enum import Enum

import wafel.imgui as ig
from wafel.util import *


# TODO: Garbage collection? Probably safe to delete after one frame w/o usage
local_state: Dict[Tuple[str, ...], Any] = {}

local_state_rebases: List[Tuple[Tuple[str, ...], Tuple[str, ...]]] = []


T = TypeVar('T')


def get_local_state_id_stack() -> Tuple[str, ...]:
  id_stack = ig.get_id_stack()
  for prev, new in local_state_rebases:
    assert id_stack[:len(prev)] == prev
    id_stack = new + id_stack[len(prev):]
  return id_stack


def push_local_state_rebase(new_stack: Tuple[str, ...]) -> None:
  local_state_rebases.append((get_local_state_id_stack(), new_stack))


def pop_local_state_rebase() -> None:
  local_state_rebases.pop()


def use_state_with(name: str, default: Callable[[], T]) -> Ref[T]:
  key = get_local_state_id_stack() + (name,)
  ref = local_state.get(key)
  if ref is None:
    ref = Ref(default())
    local_state[key] = ref
  return ref


def use_state(name: str, default: T) -> Ref[T]:
  return use_state_with(name, lambda: default)
