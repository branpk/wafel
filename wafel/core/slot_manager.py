import random
from typing import *
from dataclasses import dataclass
import weakref
import time
from abc import ABC, abstractmethod

from wafel.util import *


class AbstractSlot(ABC):
  @property
  @abstractmethod
  def frame(self) -> Optional[int]: ...

  @property
  def based(self) -> bool: ...

  @property
  @abstractmethod
  def frozen(self) -> bool: ...


SLOT = TypeVar('SLOT', bound=AbstractSlot)


class AbstractSlots(ABC, Generic[SLOT]):
  @property
  @abstractmethod
  def temp(self) -> SLOT: ...

  @property
  @abstractmethod
  def base(self) -> SLOT: ...

  @property
  @abstractmethod
  def non_base(self) -> List[SLOT]: ...

  @property
  @abstractmethod
  def power_on(self) -> SLOT: ...

  @abstractmethod
  def copy(self, dst: SLOT, src: SLOT) -> None: ...

  @abstractmethod
  def execute_frame(self) -> None: ...

  def where(
    self,
    base: Optional[bool] = None,
    frozen: Optional[bool] = None,
    valid: Optional[bool] = None,
  ) -> List[SLOT]:
    if base == True:
      results = [self.base]
    elif base == False:
      results = self.non_base
    else:
      results = self.non_base + [self.base]
    if frozen is not None:
      results = [slot for slot in results if slot.frozen == frozen]
    if valid is not None:
      results = [slot for slot in results if (slot.frame is not None) == valid]
    return results

  def swap_contents(self, slot1: SLOT, slot2: SLOT) -> None:
    if slot1 is not slot2:
      self.copy(self.temp, slot1)
      self.copy(slot1, slot2)
      self.copy(slot2, self.temp)


class SlotManager(Generic[SLOT]):
  def __init__(self, slots: AbstractSlots[SLOT]) -> None:
    self.slots = slots
    self.hotspots: Dict[str, int] = {}

  def latest_non_base_slot_before(
    self,
    frame: int,
    base: Optional[bool] = None,
  ) -> Optional[SLOT]:
    prev_slots = [
      (slot.frame, slot) for slot in self.slots.where(base=base)
        if slot.frame is not None and slot.frame <= frame
    ]
    return max(prev_slots, key=lambda s: s[:-1], default=(None,))[-1]

  def request_frame(self, frame: int, allow_nesting=False) -> SLOT:
    assert frame >= 0, frame

    assert not self.slots.base.frozen, 'Nested frame lookups require allow_nesting=True'

    # TODO: Could compute this automatically by running the computation
    def work_from(slot: SLOT) -> Tuple[int, int]:
      assert slot.frame is not None and slot.frame <= frame
      if slot.frame == frame and not (allow_nesting and slot.based):
        return 0, 0
      copies = 0
      if not slot.based:
        copies += 1
      updates = frame - slot.frame
      if allow_nesting:
        copies += 1
      return copies, updates

    def cost_from(slot: SLOT) -> int:
      copies, updates = work_from(slot)
      return 10 * copies + updates

    prev_slots = [
      slot for slot in self.slots.where()
        if slot.frame is not None and slot.frame <= frame
    ]
    latest_slot = min(prev_slots, key=cost_from)

    if latest_slot.frame == frame and not (allow_nesting and latest_slot.based):
      return latest_slot

    self.slots.copy(self.slots.base, latest_slot)
    while assert_not_none(self.slots.base.frame) < frame:
      self.slots.execute_frame()

    if allow_nesting:
      slot = random.choice(self.slots.where(base=False, frozen=False))
      self.slots.copy(slot, self.slots.base)
      return slot
    else:
      return self.slots.base

  def set_hotspot(self, name: str, frame: int) -> None:
    self.hotspots[name] = frame

  def delete_hotspot(self, name: str) -> None:
    if name in self.hotspots:
      del self.hotspots[name]

  def balance_distribution(self, max_run_time: float) -> None:
    start_time = time.time()
    iters = 0
    while time.time() - start_time < max_run_time:
      if len(self.hotspots) == 0:
        continue

      hotspot = random.choice(list(self.hotspots.values()))
      alignments = [1, 15, 40, 145, 410, 1505, 4010, 14005]
      target_frames = list(sorted(align_down(hotspot, align) for align in alignments))

      used_slots = []
      for frame in target_frames:
        matching_slots = [
          slot for slot in self.slots.where(base=False)
            if slot.frame == frame
        ]
        if len(matching_slots) > 0:
          used_slots.append(matching_slots[0])
          continue

        slot = self.request_frame(frame)
        available_slots = [
          slot for slot in self.slots.where(base=False, frozen=False)
            if slot not in used_slots
        ]
        if len(available_slots) > 0:
          selected_slot = random.choice(available_slots)
          self.slots.copy(selected_slot, slot)

        break

      iters += 1
    # print(iters)

  def get_loaded_frames(self) -> List[int]:
    return [assert_not_none(slot.frame) for slot in self.slots.where(valid=True)]
