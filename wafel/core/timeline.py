from typing import *

from wafel.core.slot_manager import SlotManager
from wafel.core.game_state import StateSlot
from wafel.core.game_lib import GameLib
from wafel.core.edit import Edits
from wafel.core.variable import Variables
from wafel.core.variable_param import VariableParam
from wafel.core.slots import StateSlots


class Timeline:
  def __init__(
    self,
    lib: GameLib,
    variables: Variables,
    edits: Edits,
  ) -> None:
    self.edits = edits
    self.slots = StateSlots(lib, variables, edits, capacity=200)
    self.slot_manager = SlotManager(self.slots)

  def __getitem__(self, frame: int) -> StateSlot:
    return self.slot_manager.request_frame(frame)

  def __len__(self) -> int:
    # TODO: Handle length better
    return len(self.edits)

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

  def get_loaded_frames(self) -> List[int]:
    return self.slot_manager.get_loaded_frames()
