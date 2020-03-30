import random
from typing import *
from dataclasses import dataclass
import weakref
import time
from abc import abstractmethod

from wafel.util import *
from wafel.core_new.memory import Slot


class SlotAllocator(Protocol):
  @property
  @abstractmethod
  def base_slot(self) -> Slot: ...

  @abstractmethod
  def alloc_slot(self) -> Slot: ...

  @abstractmethod
  def dealloc_slot(self, slot: Slot) -> None: ...

  @abstractmethod
  def copy_slot(self, dst: Slot, src: Slot) -> None: ...


@dataclass
class SlotInfo:
  slot: Slot
  based: bool
  frame: Optional[int] = None
  read_locks: int = 0
  write_locks: int = 0

  @property
  def frozen(self) -> bool:
    return self.read_locks > 0

  def __enter__(self) -> Slot:
    assert self.frame is not None
    assert self.write_locks == 0
    self.read_locks += 1
    return self.slot

  def __exit__(self, exc_type, exc_value, traceback) -> None:
    self.read_locks -= 1

  def permafreeze(self) -> None:
    assert self.frame is not None
    assert self.write_locks == 0
    self.read_locks += 1

  def __eq__(self, other: object) -> bool:
    return self is other


class SlotManager:
  def __init__(
    self,
    slot_allocator: SlotAllocator,
    run_frame: Callable[[int], None],
    capacity: int,
  ) -> None:
    self.slot_allocator = slot_allocator
    self.run_frame_impl = run_frame

    self.base_slot = SlotInfo(slot_allocator.base_slot, based=True, frame=-1)
    self.non_base_slots = [
      SlotInfo(slot_allocator.alloc_slot(), based=False)
        for _ in range(capacity)
    ]
    self.slots = self.non_base_slots + [self.base_slot]

    self.power_on_slot = self.non_base_slots[0]
    self.copy(self.power_on_slot, self.base_slot)
    self.power_on_slot.permafreeze()

    self.hotspots: Dict[str, int] = {}

  def __del__(self) -> None:
    # Restore contents of base slot since a new timeline may be created for this DLL
    self.copy(self.base_slot, self.power_on_slot)
    for slot in self.non_base_slots:
      self.slot_allocator.dealloc_slot(slot.slot)

  def copy(self, dst: SlotInfo, src: SlotInfo) -> None:
    assert not dst.frozen
    if src is not dst:
      self.slot_allocator.copy_slot(dst.slot, src.slot)
      dst.frame = src.frame
      log.timer.record_copy()

  def run_frame(self) -> None:
    assert self.base_slot.frame is not None
    assert not self.base_slot.frozen
    assert self.base_slot.write_locks == 0

    # Disallowing reads is to prevent run_frame_impl from requesting a frame
    self.base_slot.write_locks += 1

    log.timer.record_update()
    self.run_frame_impl(self.base_slot.frame)
    self.base_slot.frame += 1

    self.base_slot.write_locks -= 1

  def request_frame(self, frame: int, allow_nesting=False, require_base=False) -> ContextManager[Slot]:
    assert frame >= 0, frame

    if require_base:
      assert not allow_nesting
    assert not self.base_slot.frozen, 'Nested frame lookups require allow_nesting=True'

    log.timer.record_request()

    def work_from(slot: SlotInfo) -> Tuple[int, int]:
      slot_frame = slot.frame
      slot_based = slot.based
      assert slot_frame is not None and slot_frame <= frame
      if slot_frame == frame and not (allow_nesting and slot_based):
        return 0, 0
      copies = 0
      if not slot_based:
        copies += 1
      updates = frame - slot_frame
      if allow_nesting:
        copies += 1
      return copies, updates

    def cost_from(slot: SlotInfo) -> int:
      copies, updates = work_from(slot)
      return 10 * copies + updates

    prev_slots = [
      slot for slot in self.slots
        if slot.frame is not None and slot.frame <= frame
    ]
    latest_slot = min(prev_slots, key=cost_from)

    if latest_slot.frame == frame and \
        not (allow_nesting and latest_slot.based) and \
        not (require_base and not latest_slot.based):
      return latest_slot

    self.copy(self.base_slot, latest_slot)
    while assert_not_none(self.base_slot.frame) < frame:
      self.run_frame()

    if not allow_nesting:
      return self.base_slot

    available_slots = [slot for slot in self.non_base_slots if not slot.frozen]
    if len(available_slots) == 0:
      raise Exception('Ran out of slots')

    # The latest slot is typically easy to recreate
    selected_slot = max(
      available_slots,
      key=lambda slot: float('inf') if slot.frame is None else slot.frame,
    )
    self.copy(selected_slot, self.base_slot)
    return selected_slot

  def invalidate(self, frame: int) -> None:
    for slot in self.slots:
      if slot.frame is not None and slot.frame >= frame:
        slot.frame = None

  def invalidate_base_slot(self) -> None:
    self.base_slot.frame = None

  def set_hotspot(self, name: str, frame: int) -> None:
    self.hotspots[name] = frame

  def delete_hotspot(self, name: str) -> None:
    if name in self.hotspots:
      del self.hotspots[name]

  def balance_distribution(self, max_run_time: float) -> None:
    start_time = time.time()

    alignments = [1, 15, 40, 145, 410, 1505, 4010, 14005]
    target_frames = sorted({
      align_down(hotspot, align)
        for align in alignments
          for hotspot in self.hotspots.values()
    })

    used_slots = []
    for frame in target_frames:
      if time.time() - start_time >= max_run_time:
        return

      matching_slots = [slot for slot in self.non_base_slots if slot.frame == frame]
      if len(matching_slots) > 0:
        used_slots.append(matching_slots[0])
        continue

      slot = self.request_frame(frame)
      available_slots = [
        slot for slot in self.non_base_slots
          if not slot.frozen and slot not in used_slots
      ]
      if len(available_slots) > 0:
        selected_slot = random.choice(available_slots)
        self.copy(selected_slot, cast(SlotInfo, slot))
      else:
        log.warn('Using a suboptimal number of slots')

  def get_loaded_frames(self) -> List[int]:
    return [slot.frame for slot in self.slots if slot.frame is not None]


__all__ = ['SlotManager']
