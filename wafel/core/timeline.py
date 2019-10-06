import random
from typing import *
import weakref
import time

from wafel.core.game_state import GameState
from wafel.core.edit import Edits
from wafel.core.game_lib import GameLib
from wafel.reactive import Reactive, ReactiveValue


# TODO: Could do a quick warm up pass on first load and input change


class _Cell:
  """A block of memory containing an SM64State struct."""

  def __init__(self, frame: int, addr: int) -> None:
    self.state = None
    self.frame = frame
    self.addr = addr

  def mark_loaded(self, state: GameState) -> None:
    self.state = weakref.ref(state)

  @property
  def loaded(self) -> bool:
    if self.state is None:
      return False
    elif self.state() is None:
      self.state = None
      return False
    else:
      return True


class _CellManager:
  def __init__(
    self,
    lib: GameLib,
    edits: Edits,
    capacity: int,
  ) -> None:
    self.lib = lib
    self.edits = edits
    self.capacity = capacity

    self.cells = [self.new_cell() for _ in range(capacity)]
    self.temp_cell = self.cells.pop()

    # The base cell is the only one that SM64 C code can be run on, since
    # its pointers refer to its own memory
    self.base_cell = self.cells[0]
    for cell in self.cells:
      self.copy_cell(cell, self.base_cell, unsafe=True)

    # Keep track of currently alive GameState to ensure we never change the base
    # cell while it is in use
    self.loaded_states: List[weakref.ref[GameState]] = []

    # Keep one cell fixed at frame 0
    self.power_on_cell = self.cells[1]

    self.hotspots: Set[Reactive[int]] = set()

  def can_modify_cell(self, cell: _Cell) -> bool:
    return cell is not self.power_on_cell and not cell.loaded

  def new_cell(self) -> _Cell:
    # Frame "-1" is a power-on state before any edits have been applied.
    # Frame 0 is after edits are applied but before the first frame advance.
    return _Cell(-1, self.lib.state_new())

  def copy_cell(self, dst: _Cell, src: _Cell, unsafe: bool = False) -> None:
    if not unsafe:
      assert self.can_modify_cell(dst)
    if src is not dst:
      self.lib.state_raw_copy(dst.addr, src.addr)
      dst.frame = src.frame

  def advance_base_cell(self) -> None:
    assert self.can_modify_cell(self.base_cell)

    if self.base_cell.frame != -1:
      self.lib.state_update(self.base_cell.addr)
    self.base_cell.frame += 1

    temp_state = self.load_cell(self.base_cell)
    self.edits.apply(temp_state)

  def swap_cell_contents(self, cell1: _Cell, cell2: _Cell) -> None:
    if cell1 is not cell2:
      self.copy_cell(self.temp_cell, cell1)
      self.copy_cell(cell1, cell2)
      self.copy_cell(cell2, self.temp_cell)

  def invalidate_frame(self, frame: int) -> None:
    valid_cells = [cell for cell in self.cells if cell.frame < frame]
    invalid_cells = [cell for cell in self.cells if cell.frame >= frame]

    for invalid_cell in invalid_cells:
      valid_cell = random.choice(valid_cells)
      self.copy_cell(invalid_cell, valid_cell)

  def find_latest_cell_before(self, frame: int) -> _Cell:
    usable_cells = [cell for cell in self.cells if cell.frame <= frame]
    return max(usable_cells, key=lambda cell: cell.frame)

  def load_cell(self, cell: _Cell) -> GameState:
    state = GameState(self.lib, self.base_cell.addr, cell.frame, cell.addr)
    cell.mark_loaded(state)
    return state

  def request_frame(self, frame: int, based: bool = False) -> Optional[GameState]:
    # Load a state as close to the desired frame as possible
    latest_cell = self.find_latest_cell_before(frame)

    # Avoid copies in common case
    if latest_cell.frame == frame and self.can_modify_cell(latest_cell) and \
        based == (latest_cell is self.base_cell):
      return self.load_cell(latest_cell)

    if self.can_modify_cell(latest_cell):
      self.swap_cell_contents(self.base_cell, latest_cell)
    else:
      self.copy_cell(self.base_cell, latest_cell)

    free_cells = [
      cell for cell in self.cells
        if self.can_modify_cell(cell) and cell is not self.base_cell
    ]

    while self.base_cell.frame < frame:
      self.advance_base_cell()

      # Leave behind some breadcrumbs. This allows smoother backward scrolling
      # if the user scrolls to a late frame before the cell distribution has
      # caught up.
      remaining = frame - self.base_cell.frame
      if remaining % 1000 == 0 or (remaining < 60 and remaining % 10 == 0):
        selected = random.choice(free_cells)
        self.copy_cell(selected, self.base_cell)

    if based:
      selected = self.base_cell
    else:
      selected = random.choice(free_cells)
      self.swap_cell_contents(selected, self.base_cell)

    return self.load_cell(selected)

  def add_hotspot(self, frame: Reactive[int]) -> None:
    self.hotspots.add(frame)

  def remove_hotspot(self, frame: Reactive[int]) -> None:
    if frame in self.hotspots:
      self.hotspots.remove(frame)

  def get_timeline_length(self) -> int:
    # TODO: Compute this correctly
    return len(self.edits._items)

  def get_cell_buckets(self) -> Dict[int, List[_Cell]]:
    """Divide the frame timeline into buckets, where each bucket ideally
    contains the same number of cells."""

    # self.get_timeline_length() // len(self.cells) would provide a uniform distribution.
    # We increase the size to give us extra cells to work with
    default_bucket_size = self.get_timeline_length() // len(self.cells) * 4
    if default_bucket_size == 0:
      default_bucket_size = 1

    buckets: Dict[int, List[_Cell]] = {
      frame: [] for frame in range(-1, self.get_timeline_length(), default_bucket_size)
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
      default=self.get_timeline_length(),
    )
    target_frame = random.randrange(min_bucket, max(min_bucket_next, min_bucket + 1))

    self.move_cell_to_frame(cell, target_frame, max_advances=50)

  def balance_distribution(self, max_run_time: float) -> None:
    start_time = time.monotonic()
    while time.monotonic() - start_time < max_run_time:
      self.balance_cells()

  def get_loaded_frames(self) -> List[int]:
    return [cell.frame for cell in self.cells]


# TODO: Watch for input changes
# TODO: GameStateTimeline events (adding/removing frames, any state changed (for frame sheet caching), etc)
# TODO: Handling case where request_frame returns None (once implemented)

class _ReactiveGameState(Reactive[GameState]):
  def __init__(self, timeline: 'Timeline', frame: Reactive[int]) -> None:
    self.timeline = timeline
    self.frame = frame

  @property
  def value(self) -> GameState:
    return self.timeline._get_state_now(self.frame.value)

  def _on_change(self, callback: Callable[[], None]) -> None:
    self.frame.on_change(callback)
    self.timeline._on_state_change(self.frame, callback)


class Timeline:
  def __init__(
    self,
    lib: GameLib,
    edits: Edits,
  ) -> None:
    self._cell_manager = _CellManager(lib, edits, capacity=200)
    self._callbacks: List[Tuple[Reactive[int], Callable[[], None]]] = []

    edits.latest_edited_frame.on_change(self._invalidate_frame)

  def _get_state_now(self, frame: int) -> GameState:
    state = self._cell_manager.request_frame(frame)
    assert state is not None
    return state

  def _on_state_change(self, frame: Reactive[int], callback: Callable[[], None]) -> None:
    self._callbacks.append((frame, callback))

  def _invalidate_frame(self, frame: int) -> None:
    self._cell_manager.invalidate_frame(frame)

    callbacks = [cb for f, cb in self._callbacks if f.value >= frame]
    for callback in callbacks:
      callback()

  def frame(self, frame: Union[int, Reactive[int]]) -> Reactive[GameState]:
    if isinstance(frame, int):
      frame = ReactiveValue(frame)
    return _ReactiveGameState(self, frame)

  def __len__(self) -> int:
    # TODO: Handle length better
    return len(self._cell_manager.edits._items)

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
