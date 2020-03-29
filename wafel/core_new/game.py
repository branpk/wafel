from __future__ import annotations

from typing import *
from dataclasses import dataclass
from enum import Enum, auto
from abc import ABC, abstractmethod

from wafel.core_new.memory import Memory, VirtualAddress, Slot


VADDR = TypeVar('VADDR', bound=VirtualAddress)
SLOT = TypeVar('SLOT', bound=Slot)


class GameImpl(ABC, Generic[VADDR, SLOT]):
  def remove_type_vars(self) -> Game:
    return cast(Game, self)

  @property
  @abstractmethod
  def base_slot(self) -> SLOT: ...

  @abstractmethod
  def alloc_slot(self) -> SLOT: ...

  @abstractmethod
  def dealloc_slot(self, slot: SLOT) -> None: ...

  @abstractmethod
  def copy_slot(self, dst: SLOT, src: SLOT) -> None: ...

  @property
  @abstractmethod
  def memory(self) -> Memory[VADDR, SLOT]: ...

  @abstractmethod
  def run_frame(self) -> None: ...


Game = GameImpl[VirtualAddress, Slot]


__all__ = ['GameImpl', 'Game']
