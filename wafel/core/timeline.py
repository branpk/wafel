from __future__ import annotations

from typing import *
from abc import ABC, abstractmethod
from dataclasses import dataclass
import weakref

from wafel.core.game import Game
from wafel.core.memory import Slot, Address, VirtualAddress
from wafel.core.data_path import DataPath
from wafel.core.slot_manager import SlotManager
from wafel.core.data_cache import DataCache


class Controller(ABC):
  @staticmethod
  def no_op() -> Controller:
    return ControllerSequence([])

  @staticmethod
  def sequence(*controllers: Controller) -> Controller:
    return ControllerSequence(controllers)

  def __init__(self) -> None:
    self._on_change_callbacks: List[Callable[[int], None]] = []

  def on_change(self, callback: Callable[[int], None]) -> None:
    self._on_change_callbacks.append(callback)

  def notify(self, frame: int) -> None:
    for callback in list(self._on_change_callbacks):
      callback(frame)

  @property
  def weak_notify(self) -> Callable[[int], None]:
    weak_self_ref = weakref.ref(self)
    def notify(frame: int) -> None:
      weak_self = weak_self_ref()
      if weak_self is not None:
        weak_self.notify(frame)
    return notify

  @abstractmethod
  def apply(self, state: SlotState) -> None: ...


class ControllerSequence(Controller):
  def __init__(self, controllers: Iterable[Controller]) -> None:
    self.controllers = controllers

  def on_change(self, callback: Callable[[int], None]) -> None:
    for controller in self.controllers:
      controller.on_change(callback)

  def apply(self, state: SlotState) -> None:
    for controller in self.controllers:
      controller.apply(state)


class BaseSlotContextManager:
  def __init__(
    self,
    game: Game,
    frame: int,
    slot_manager: SlotManager,
    slot: ContextManager[Slot],
    invalidate: bool,
  ) -> None:
    self.game = game
    self.frame = frame
    self.slot_manager = slot_manager
    self.slot = slot
    self.invalidate = invalidate

  def __enter__(self) -> SlotState:
    return SlotState(self.game, self.frame, self.slot.__enter__())

  def __exit__(self, exc_type, exc_value, traceback) -> None:
    if self.invalidate:
      self.slot_manager.invalidate_base_slot()
    self.slot.__exit__(exc_type, exc_value, traceback)


class Timeline:
  def __init__(
    self,
    game: Game,
    controller: Controller,
    slot_capacity: int,
  ) -> None:
    self.game = game
    self.controller = controller
    self.slot_manager = SlotManager(game, self.run_frame, slot_capacity)
    self.data_cache = DataCache()

    weak_self_ref = weakref.ref(self)
    def invalidate(frame: int) -> None:
      weak_self = weak_self_ref()
      if weak_self is not None:
        weak_self.invalidate(frame)
    self.controller.on_change(invalidate)

  def invalidate(self, frame: int) -> None:
    self.slot_manager.invalidate(frame)
    self.data_cache.invalidate(frame)

  def run_frame(self, frame: int) -> None:
    if frame != -1:
      self.game.run_frame()
    state = SlotState(self.game, frame + 1, self.game.base_slot)
    self.controller.apply(state)

  def get(self, frame: int, path: Union[DataPath, str]) -> object:
    if isinstance(path, str):
      path = self.game.path(path)

    value = self.data_cache.get(frame, path)
    if value is not None:
      return value

    with self.slot_manager.request_frame(frame) as slot:
      value = path.get(slot)

    if value is not None:
      self.data_cache.put(frame, path, value)
    return value

  def __getitem__(self, frame: int) -> State:
    return TimelineFrameState(self, frame)

  def request_base(self, frame: int, invalidate=False) -> ContextManager[SlotState]:
    return BaseSlotContextManager(
      self.game,
      frame,
      self.slot_manager,
      self.slot_manager.request_frame(frame, require_base=True),
      invalidate,
    )

  def set_hotspot(self, name: str, frame: int) -> None:
    """Mark a certain frame as a "hotspot", which is a hint to try to ensure
    that scrolling near the frame is smooth.
    """
    self.slot_manager.set_hotspot(name, frame)

  def delete_hotspot(self, name: str) -> None:
    self.slot_manager.delete_hotspot(name)

  def balance_distribution(self, max_run_time: float) -> None:
    """Perform maintenance to maintain a nice distribution of loaded frames."""
    self.slot_manager.balance_distribution(max_run_time)

  def on_invalidation(self, callback: Callable[[int], None]) -> None:
    self.controller.on_change(callback)


class State(ABC):
  @property
  @abstractmethod
  def game(self) -> Game: ...

  @property
  @abstractmethod
  def frame(self) -> int: ...

  @abstractmethod
  def get(self, path: Union[DataPath, str]) -> object: ...


class SlotState(State):
  def __init__(self, game: Game, frame: int, slot: Slot) -> None:
    self._game = game
    self._frame = frame
    self._slot = slot

  @property
  def game(self) -> Game:
    return self._game

  @property
  def frame(self) -> int:
    return self._frame

  @property
  def slot(self) -> Slot:
    return self._slot

  def get_addr(self, path: Union[DataPath, str]) -> Address[VirtualAddress]:
    if isinstance(path, str):
      path = self.game.path(path)
    return path.get_addr(self.slot)

  def get(self, path: Union[DataPath, str]) -> object:
    if isinstance(path, str):
      path = self.game.path(path)
    return path.get(self.slot)


class TimelineFrameState(State):
  def __init__(self, timeline: Timeline, frame: int) -> None:
    self._timeline = timeline
    self._frame = frame

  @property
  def game(self) -> Game:
    return self._timeline.game

  @property
  def frame(self) -> int:
    return self._frame

  @property
  def timeline(self) -> Timeline:
    return self._timeline

  def get(self, path: Union[DataPath, str]) -> object:
    return self.timeline.get(self.frame, path)


__all__ = ['Controller', 'Timeline', 'State', 'SlotState']
