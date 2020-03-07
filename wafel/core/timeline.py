from typing import *
import weakref

from wafel.core.slot_manager import SlotManager, AbstractSlots
from wafel.core.game_state import StateSlot
from wafel.core.game_lib import GameLib
from wafel.core.edit import Edits
from wafel.core.variable import Variables
from wafel.core.variable_param import VariableParam


class StateSlots(AbstractSlots[StateSlot]):
  def __init__(
    self,
    lib: GameLib,
    variables: Variables,
    edits: Edits,
    capacity: int,
  ) -> None:
    self.lib = lib
    self.variables = variables
    self.edits = edits

    self._base = self.lib.base_slot()
    self._non_base = [self.lib.alloc_slot() for _ in range(capacity - 1)]
    self._temp = self._non_base.pop()

    # Prevent callback from keeping self alive
    weak_self = weakref.ref(self)
    def invalidate(frame: int) -> None:
      self_ref = weak_self()
      if self_ref is not None:
        self_ref.invalidate_frame(frame)
    self.edits.on_edit(invalidate)

  def __del__(self) -> None:
    for slot in self.non_base + [self.temp]:
      self.lib.dealloc_slot(slot)

  @property
  def temp(self) -> StateSlot:
    return self._temp

  @property
  def base(self) -> StateSlot:
    return self._base

  @property
  def non_base(self) -> List[StateSlot]:
    return self._non_base

  def copy(self, dst: StateSlot, src: StateSlot) -> None:
    assert not dst.frozen
    self.lib.raw_copy_slot(dst, src)
    dst.frame = src.frame

  def num_frames(self) -> int:
    return len(self.edits)

  def execute_frame(self) -> None:
    assert self.base.frame is not None
    assert not self.base.frozen

    with self.base as state:
      # Disallowing reads shouldn't be necessary. It's just a precaution in case
      # variable.set ever tries to read data from another state
      self.base.disallow_reads = True

      if self.base.frame != -1:
        self.lib.execute_frame()
      self.base.frame += 1
      state.frame += 1

      for edit in self.edits.get_edits(state.frame):
        variable = self.variables[edit.variable_id]
        variable.set(edit.value, { VariableParam.STATE: state })

      self.base.disallow_reads = False

  def invalidate_frame(self, frame: int) -> None:
    for slot in self.where():
      if slot.frame is not None and slot.frame >= frame:
        slot.frame = None


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
