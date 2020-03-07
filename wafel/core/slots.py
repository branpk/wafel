from typing import *
import weakref

from wafel.core.slot_manager import AbstractSlots
from wafel.core.variable import Variables
from wafel.core.variable_param import VariableParam
from wafel.core.game_lib import GameLib
from wafel.core.edit import Edits
from wafel.core.game_state import Slot


class Slots(AbstractSlots[Slot]):
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
  def temp(self) -> Slot:
    return self._temp

  @property
  def base(self) -> Slot:
    return self._base

  @property
  def non_base(self) -> List[Slot]:
    return self._non_base

  def copy(self, dst: Slot, src: Slot) -> None:
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
