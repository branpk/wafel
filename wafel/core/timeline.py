from __future__ import annotations

from typing import *
from abc import ABC, abstractmethod
import weakref

from wafel.core.game import Game
from wafel.core.memory import Slot
from wafel.core.data_path import DataPath
from wafel.core.slot_manager import SlotManager


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

  @abstractmethod
  def apply(self, game: Game, frame: int, slot: Slot) -> None: ...


class ControllerSequence(Controller):
  def __init__(self, controllers: Iterable[Controller]) -> None:
    self.controllers = controllers

  def on_change(self, callback: Callable[[int], None]) -> None:
    for controller in self.controllers:
      controller.on_change(callback)

  def apply(self, game: Game, frame: int, slot: Slot) -> None:
    for controller in self.controllers:
      controller.apply(game, frame, slot)


class BaseSlotContextManager:
  def __init__(
    self,
    slot_manager: SlotManager,
    slot: ContextManager[Slot],
    invalidate: bool,
  ) -> None:
    self.slot_manager = slot_manager
    self.slot = slot
    self.invalidate = invalidate

  def __enter__(self) -> Slot:
    return self.slot.__enter__()

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

    weak_self_ref = weakref.ref(self)
    def invalidate(frame: int) -> None:
      weak_self = weak_self_ref()
      if weak_self is not None:
        weak_self.invalidate(frame)
    self.controller.on_change(invalidate)

  def invalidate(self, frame: int) -> None:
    self.slot_manager.invalidate(frame)

  def run_frame(self, frame: int) -> None:
    if frame != -1:
      self.game.run_frame()
    self.controller.apply(self.game, frame + 1, self.game.base_slot)

  def get(self, frame: int, path: Union[DataPath, str]) -> object:
    if isinstance(path, str):
      path = self.game.path(path)
    with self.slot_manager.request_frame(frame) as slot:
      return path.get(slot)

  def request_base(self, frame: int, invalidate=False) -> ContextManager[Slot]:
    return BaseSlotContextManager(
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


__all__ = ['Controller', 'Timeline']
