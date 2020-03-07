import random
from typing import *
from dataclasses import dataclass
import weakref
import time
from abc import ABC, abstractmethod

from wafel.util import *


# Abstract interfaces for slots - mainly to allow testing of the slot algorithm

class AbstractSlot(ABC):
  @property
  @abstractmethod
  def frame(self) -> Optional[int]: ...

  @property
  def based(self) -> bool: ...

  @property
  @abstractmethod
  def frozen(self) -> bool: ...

  @abstractmethod
  def permafreeze(self) -> None: ...


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

  @abstractmethod
  def copy(self, dst: SLOT, src: SLOT) -> None: ...

  # TODO: Rethink
  @abstractmethod
  def num_frames(self) -> int: ...

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

    # Keep one slot frozen at power-on
    assert self.slots.base.frame == -1
    self.power_on = self.slots.where(base=False)[0]
    self.slots.copy(self.power_on, self.slots.base)
    self.power_on.permafreeze()

    self.hotspots: Dict[str, int] = {}

  def find_latest_slot_before(self, frame: int) -> SLOT:
    slots = [
      slot for slot in self.slots.where()
        if slot.frame is not None and slot.frame <= frame
    ]
    return max(slots, key=lambda slot: slot.frame)

  def request_frame(self, frame: int, based: bool = False) -> SLOT:
    # Load a state as close to the desired frame as possible
    latest_slot: SLOT = self.find_latest_slot_before(frame)

    # Avoid copies in common case
    if latest_slot.frame == frame and based == latest_slot.based:
      return latest_slot

    # If possible, it is good to avoid throwing away the contents of the base slot
    if not latest_slot.frozen:
      self.slots.swap_contents(self.slots.base, latest_slot)
    else:
      self.slots.copy(self.slots.base, latest_slot)

    free_slots = self.slots.where(frozen=False, base=False)

    while assert_not_none(self.slots.base.frame) < frame:
      self.slots.execute_frame()

      # Leave behind some breadcrumbs. This allows smoother backward scrolling
      # if the user scrolls to a late frame before the slot distribution has
      # caught up.
      remaining = frame - assert_not_none(self.slots.base.frame)
      if remaining % 1000 == 0 or (remaining < 60 and remaining % 10 == 0):
        selected = random.choice(free_slots)
        self.slots.copy(selected, self.slots.base)

    if based:
      return self.slots.base
    else:
      slot = random.choice(free_slots)
      self.slots.swap_contents(slot, self.slots.base)
      return slot

  def set_hotspot(self, name: str, frame: int) -> None:
    self.hotspots[name] = frame

  def delete_hotspot(self, name: str) -> None:
    if name in self.hotspots:
      del self.hotspots[name]

  def get_timeline_length(self) -> int:
    return self.slots.num_frames()

  def get_slot_buckets(self) -> Dict[int, List[SLOT]]:
    """Divide the frame timeline into buckets, where each bucket ideally
    contains the same number of slots."""

    # self.get_timeline_length() // len(self.slots) would provide a uniform distribution.
    # We increase the size to give us extra slots to work with
    default_bucket_size = self.get_timeline_length() // len(self.slots.where()) * 4
    if default_bucket_size == 0:
      default_bucket_size = 1

    buckets: Dict[int, List[SLOT]] = {
      frame: [] for frame in range(-1, self.get_timeline_length(), default_bucket_size)
    }

    # Increase the number of buckets near hotspots
    for hotspot in self.hotspots.values():
      for i in range(-60, 61, 5):
        if hotspot + i in range(self.get_timeline_length()):
          buckets[max(hotspot + i, 0)] = []

    # Divide the modifiable slots into the buckets
    free_slots = self.slots.where(frozen=False)
    for slot in free_slots:
      if slot.frame is None:
        continue
      bucket = max(b for b in buckets if b <= slot.frame)
      buckets[bucket].append(slot)

    return buckets

  def move_slot_to_frame(self, slot: SLOT, target_frame: int, max_advances: int) -> None:
    # Save the base slot's contents to the selected slot, and load the base
    # slot with a good starting point
    self.slots.copy(slot, self.slots.base)
    self.slots.copy(self.slots.base, self.find_latest_slot_before(target_frame))
    assert self.slots.base.frame is not None

    # Advance by at most max_advance frames to reach the target frame
    target_frame = min(target_frame, self.slots.base.frame + max_advances)
    while assert_not_none(self.slots.base.frame) < target_frame:
      self.slots.execute_frame()

    # The base slot gets overwritten often, so swap it back to avoid immediately
    # undoing our work
    self.slots.swap_contents(self.slots.base, slot)

  def balance_slots(self) -> None:
    # Shuffle the buckets to avoid biasing toward earlier buckets
    buckets = self.get_slot_buckets()
    shuffled_buckets = list(buckets.items())
    random.shuffle(shuffled_buckets)

    # Find the buckets with the least and most number of slots
    min_bucket = min(shuffled_buckets, key=lambda e: len(e[1]))[0]
    max_bucket = max(shuffled_buckets, key=lambda e: len(e[1]))[0]

    # Select a slot from the max bucket to move, and a frame in the min bucket
    # to move it to
    unused_slots = self.slots.where(valid=False)
    if len(unused_slots) > 0:
      slot = unused_slots[0]
    else:
      slot = random.choice(buckets[max_bucket])

    min_bucket_next = min(
      [bucket for bucket in buckets if bucket > min_bucket],
      default=self.get_timeline_length(),
    )
    target_frame = random.randrange(min_bucket, max(min_bucket_next, min_bucket + 1))

    self.move_slot_to_frame(slot, target_frame, max_advances=50)

  def balance_distribution(self, max_run_time: float) -> None:
    start_time = time.monotonic()
    while time.monotonic() - start_time < max_run_time:
      self.balance_slots()

  def get_loaded_frames(self) -> List[int]:
    return [assert_not_none(slot.frame) for slot in self.slots.where(valid=True)]
