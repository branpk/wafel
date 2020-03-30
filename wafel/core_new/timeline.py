from typing import *
from abc import ABC, abstractmethod
import weakref

from wafel.core_new.game import Game
from wafel.core_new.memory import Slot
from wafel.core_new.data_path import DataPath
from wafel.core_new.slot_manager import SlotManager


class Controller(ABC):
  def __init__(self) -> None:
    self._on_change_callbacks: List[Callable[[int], None]] = []

  def on_change(self, callback: Callable[[int], None]) -> None:
    self._on_change_callbacks.append(callback)

  def notify(self, frame: int) -> None:
    for callback in list(self._on_change_callbacks):
      callback(frame)

  @abstractmethod
  def apply(self, game: Game, frame: int, slot: Slot) -> None: ...


class NoOpController(Controller):
  def apply(self, game: Game, frame: int, slot: Slot) -> None:
    pass


class BaseSlotInvalidator:
  def __init__(self, game: Game, slot_manager: SlotManager) -> None:
    self.game = game
    self.slot_manager = slot_manager

  def __enter__(self) -> Slot:
    return self.game.base_slot

  def __exit__(self, exc_type, exc_value, traceback) -> None:
    self.slot_manager.invalidate_base_slot()


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

  def request_then_invalidate(self, frame: int) -> ContextManager[Slot]:
    return BaseSlotInvalidator(self.game, self.slot_manager)

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


__all__ = ['Controller', 'NoOpController', 'Timeline']
