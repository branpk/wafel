from __future__ import annotations

from typing import *
from dataclasses import dataclass

from wafel.core.slot_manager import AbstractSlot, AbstractSlots


# Note: the -1 frame represents the power-on state before the first edits are
# applied. Frame 0 is after the first edits are applied, but before the
# first frame advance.


class GameState:
  """The state of the game on a particular frame.

  The state is backed by a StateSlot object which contains the contents of the game's
  memory.

  A GameState should only be created using
    with slot as state:
      ...
  and state should not be used outside that scope.
  This guarantees that the corresponding StateSlot object is not changed while this
  GameState is in use.

  A GameState should ideally be short lived. If too many GameStates are alive
  at once, then the number of available StateSlots will be too low to perform
  frame lookups quickly.
  """

  def __init__(self, frame: int, slot: StateSlot) -> None:
    self.frame = frame
    self._slot: Optional[StateSlot] = slot

  def invalidate(self) -> None:
    self._slot = None

  @property
  def slot(self) -> StateSlot:
    assert self._slot is not None, 'Use of state after release'
    return self._slot


@dataclass(frozen=True)
class AbsoluteAddr:
  """A static address in the DLL's address space."""
  addr: int

@dataclass(frozen=True)
class OffsetAddr:
  """Slot independent offset."""
  range_index: int
  offset: int

@dataclass(frozen=True)
class RelativeAddr:
  value: Union[AbsoluteAddr, OffsetAddr]

  @staticmethod
  def absolute(addr: int) -> RelativeAddr:
    return RelativeAddr(AbsoluteAddr(addr))

  @staticmethod
  def offset(range_index: int, offset: int) -> RelativeAddr:
    return RelativeAddr(OffsetAddr(range_index, offset))


class StateSlot(AbstractSlot):
  """A memory buffer that can hold an entire game state.

  If slot.frame is not None, then the slot holds the game state for that frame.
  The data in the slot should not be used directly though, since it can be
  changed at any time.
  To access the state, use:
    with slot as state:
      ...
  This will "freeze" the slot, which tells the slot manager not to change its
  contents. The one exception is that slot.frame may be set to None during this
  time if an earlier frame is edited. This isn't a problem since it only affects
  future usages of the slot.
  """

  def __init__(self, addr_ranges: List[range], base_slot: Optional[StateSlot]) -> None:
    self.addr_ranges = addr_ranges
    self.base_slot = base_slot or self

    self._frame: Optional[int] = None
    self._owners: List[GameState] = []
    self.disallow_reads = False  # Set to True while being modified

  def addr_to_offset(self, addr: int) -> Optional[OffsetAddr]:
    for i, addr_range in enumerate(self.addr_ranges):
      if addr in addr_range:
        return OffsetAddr(i, addr - addr_range.start)
    return None

  def offset_to_addr(self, offset: OffsetAddr) -> int:
    return self.addr_ranges[offset.range_index].start + offset.offset

  def addr_to_relative(self, addr: int) -> RelativeAddr:
    return RelativeAddr(self.addr_to_offset(addr) or AbsoluteAddr(addr))

  def relative_to_addr(self, rel_addr: RelativeAddr) -> int:
    if isinstance(rel_addr.value, AbsoluteAddr):
      return rel_addr.value.addr
    else:
      return self.offset_to_addr(rel_addr.value)

  @property
  def based(self) -> bool:
    return self is self.base_slot

  @property
  def frame(self) -> Optional[int]:
    return self._frame

  @frame.setter
  def frame(self, frame: Optional[int]) -> None:
    self._frame = frame

  @property
  def frozen(self) -> bool:
    return len(self._owners) > 0

  def __enter__(self) -> GameState:
    assert self.frame is not None
    assert not self.disallow_reads
    owner = GameState(self.frame, self)
    self._owners.append(owner)
    return owner

  def __exit__(self, exc_type, exc_value, traceback) -> None:
    self._owners.pop().invalidate()

  def permafreeze(self) -> None:
    assert self.frame is not None
    assert not self.disallow_reads
    self._owners.append(GameState(self.frame, self))


ObjectId = int

class Object:
  def __init__(self, addr: int) -> None:
    self.addr = addr
