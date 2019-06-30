import ctypes
import random
from typing import *
import weakref
import time

from butter.util import dcast
from butter.game_state import GameState
from butter.input_sequence import InputSequence
from butter.reactive import Reactive, ReactiveValue


# TODO: Could do a quick warm up pass on first load and input change


class _Cell:
  """A block of memory containing an SM64State struct."""

  def __init__(self, frame: int, addr: int) -> None:
    self.frame = frame
    self.addr = addr


# TODO: Cell invalidation on input change

class _CellManager:
  def __init__(
    self,
    lib: ctypes.CDLL,
    spec: dict,
    inputs: InputSequence,
    capacity: int,
  ) -> None:
    self.lib = lib
    self.spec = spec
    self.inputs = inputs
    self.capacity = capacity

    self.cells = [self.new_cell() for _ in range(capacity)]
    self.temp_cell = self.cells.pop()

    # The base cell is the only valid cell, since pointers in the state refer
    # to its memory. All other cells are used to "back up" the base cell's data
    self.base_cell = self.cells[0]
    for cell in self.cells:
      self.copy_cell(cell, self.base_cell, unsafe=True)

    # Keep track of currently alive GameState to ensure we never change the base
    # cell while it is in use
    self.loaded_states: List[weakref.ref[GameState]] = []

    # Keep one cell fixed at frame 0
    self.power_on_cell = self.cells[1]

    self.hotspots: Set[Reactive[int]] = set()

  def any_states_loaded(self) -> bool:
    self.loaded_states = [st for st in self.loaded_states if st() is not None]
    return len(self.loaded_states) > 0

  def can_modify_cell(self, cell: _Cell) -> bool:
    if cell is self.base_cell:
      return not self.any_states_loaded()
    elif cell is self.power_on_cell:
      return False
    else:
      return True

  def new_cell(self) -> _Cell:
    addr = dcast(int, self.lib.sm64_state_new())
    return _Cell(0, addr)

  def copy_cell(self, dst: _Cell, src: _Cell, unsafe: bool = False) -> None:
    if not unsafe:
      assert self.can_modify_cell(dst)
    if src is not dst:
      self.lib.sm64_state_raw_copy(dst.addr, src.addr)
      dst.frame = src.frame

  def advance_base_cell(self) -> None:
    assert self.can_modify_cell(self.base_cell)

    temp_state = GameState(self.spec, self.base_cell.frame, self.base_cell.addr)
    self.inputs.apply(temp_state)

    self.lib.sm64_state_update(self.base_cell.addr)
    self.base_cell.frame += 1

  def swap_cell_contents(self, cell1: _Cell, cell2: _Cell) -> None:
    if cell1 is not cell2:
      self.copy_cell(self.temp_cell, cell1)
      self.copy_cell(cell1, cell2)
      self.copy_cell(cell2, self.temp_cell)

  def find_latest_cell_before(self, frame: int) -> _Cell:
    usable_cells = [cell for cell in self.cells if cell.frame <= frame]
    return max(usable_cells, key=lambda cell: cell.frame)

  def request_frame(self, frame: int) -> Optional[GameState]:
    # Load a state as close to the desired frame as possible
    self.copy_cell(self.base_cell, self.find_latest_cell_before(frame))

    # TODO: Max number of frame advances, return None otherwise
    while self.base_cell.frame < frame:
      self.advance_base_cell()

    state = GameState(self.spec, self.base_cell.frame, self.base_cell.addr)
    self.loaded_states.append(weakref.ref(state))
    return state

  def add_hotspot(self, frame: Reactive[int]) -> None:
    self.hotspots.add(frame)

  def remove_hotspot(self, frame: Reactive[int]) -> None:
    if frame in self.hotspots:
      self.hotspots.remove(frame)

  def get_cell_buckets(self) -> Dict[int, List[_Cell]]:
    """Divide the frame timeline into buckets, where each bucket ideally
    contains the same number of cells."""

    # len(self.inputs) // len(self.cells) would provide a uniform distribution.
    # We increase the size to give us extra cells to work with
    default_bucket_size = len(self.inputs) // len(self.cells) * 4

    buckets: Dict[int, List[_Cell]] = {
      frame: [] for frame in range(0, len(self.inputs), default_bucket_size)
    }

    # Increase the number of buckets near hotspots
    for hotspot in self.hotspots:
      for i in range(-60, 61, 5):
        buckets[max(hotspot.value + i, 0)] = []

    # Divide the modifiable cells into the buckets
    free_cells = [cell for cell in self.cells if self.can_modify_cell(cell)]
    for cell in free_cells:
      bucket = max(b for b in buckets if b <= cell.frame)
      buckets[bucket].append(cell)

    return buckets

  def move_cell_to_frame(self, cell: _Cell, target_frame: int, max_advances: int) -> None:
    # Save the base cell's contents to the selected cell, and load the base
    # cell with a good starting point
    self.copy_cell(cell, self.base_cell)
    self.copy_cell(self.base_cell, self.find_latest_cell_before(target_frame))

    # Advance by at most max_advance frames to reach the target frame
    target_frame = min(target_frame, self.base_cell.frame + max_advances)
    while self.base_cell.frame < target_frame:
      self.advance_base_cell()

    # The base cell gets overwritten often, so swap it back to avoid immediately
    # undoing our work
    self.swap_cell_contents(self.base_cell, cell)

  def balance_cells(self) -> None:
    # Shuffle the buckets to avoid biasing toward earlier buckets
    buckets = self.get_cell_buckets()
    shuffled_buckets = list(buckets.items())
    random.shuffle(shuffled_buckets)

    # Find the buckets with the least and most number of cells
    min_bucket = min(shuffled_buckets, key=lambda e: len(e[1]))[0]
    max_bucket = max(shuffled_buckets, key=lambda e: len(e[1]))[0]

    # Select a cell from the max bucket to move, and a frame in the min bucket
    # to move it to
    cell = random.choice(buckets[max_bucket])
    min_bucket_next = min(
      [bucket for bucket in buckets if bucket > min_bucket],
      default=len(self.inputs),
    )
    target_frame = random.randrange(min_bucket, max(min_bucket_next, min_bucket + 1))

    self.move_cell_to_frame(cell, target_frame, max_advances=50)

  def balance_distribution(self, max_run_time: float) -> None:
    # TODO: Could save and restore the base cell to allow loaded states to exist
    # through calls to this method
    start_time = time.monotonic()
    while time.monotonic() - start_time < max_run_time:
      self.balance_cells()

  def get_loaded_frames(self) -> List[int]:
    return [cell.frame for cell in self.cells]


# TODO: Watch for input changes
# TODO: Make GameStateTimeline itself reactive (adding/removing frames etc)
# TODO: Handling case where request_frame returns None (once implemented)

class _ReactiveGameState(Reactive[GameState]):
  def __init__(self, timeline: 'Timeline', frame: Reactive[int]) -> None:
    self.timeline = timeline
    self.frame = frame

  @property
  def value(self) -> GameState:
    return self.timeline._get_state_now(self.frame.value)

  def on_change(self, callback: Callable[[GameState], None]) -> None:
    def on_frame_change(frame: int) -> None:
      callback(self.timeline._get_state_now(frame))
    self.frame.on_change(on_frame_change)

    self.timeline._on_state_change(self.frame, callback)


class Timeline:
  def __init__(
    self,
    lib: ctypes.CDLL,
    spec: dict,
    inputs: InputSequence,
  ) -> None:
    self._cell_manager = _CellManager(lib, spec, inputs, capacity=200)
    self._callbacks: List[Tuple[Reactive[int], Callable[[GameState], None]]] = []

  def _get_state_now(self, frame: int) -> GameState:
    state = self._cell_manager.request_frame(frame)
    assert state is not None
    return state

  def _on_state_change(self, frame: Reactive[int], callback: Callable[[GameState], None]) -> None:
    self._callbacks.append((frame, callback))

  def _invalidate_state(self, frame: int) -> None:
    # TODO: Invalidate cells
    callbacks = [(f, cb) for f, cb in self._callbacks if f.value >= frame]
    for callback_frame, callback in callbacks:
      callback(self._get_state_now(callback_frame.value))

  def frame(self, frame: Union[int, Reactive[int]]) -> Reactive[GameState]:
    if isinstance(frame, int):
      frame = ReactiveValue(frame)
    return _ReactiveGameState(self, frame)

  def __len__(self) -> int:
    return len(self._cell_manager.inputs) + 1

  def add_hotspot(self, frame: Reactive[int]) -> None:
    """Mark a certain frame as a "hotspot", which is a hint to try to ensure
    that scrolling near the frame is smooth.
    """
    self._cell_manager.add_hotspot(frame)

  def delete_hotspot(self, frame: Reactive[int]) -> None:
    self._cell_manager.remove_hotspot(frame)

  def balance_distribution(self, max_run_time: float) -> None:
    """Perform maintenance to maintain a nice distribution of loaded frames."""
    self._cell_manager.balance_distribution(max_run_time)

  def get_loaded_frames(self) -> List[int]:
    return self._cell_manager.get_loaded_frames()
