import random
from typing import *
import weakref
import time


# TODO: Could do a quick warm up pass on first load and input change


# Raw state address
ADDR = TypeVar('ADDR')

# Owned game state
ST = TypeVar('ST')


class StateSequence(Generic[ST, ADDR]):
  def base_state(self) -> ADDR:
    raise NotImplementedError

  def alloc_state_buffer(self) -> ADDR:
    raise NotImplementedError

  def dealloc_state_buffer(self, addr: ADDR) -> None:
    raise NotImplementedError

  def raw_copy_state(self, dst: ADDR, src: ADDR) -> None:
    """Copy without updating pointers."""
    raise NotImplementedError

  def execute_frame(self) -> None:
    raise NotImplementedError

  def to_owned(self, base_addr: ADDR, frame: int, addr: ADDR) -> ST:
    raise NotImplementedError

  def apply_edits(self, state: ST) -> None:
    raise NotImplementedError

  def get_num_frames(self) -> int:
    raise NotImplementedError

  def on_invalidation(self, callback: Callable[[int], None]) -> None:
    raise NotImplementedError


class _Cell(Generic[ST, ADDR]):
  """A block of memory containing an SM64State struct."""

  def __init__(self, frame: int, addr: ADDR) -> None:
    self.state: Optional[weakref.ref[ST]] = None
    self.frame = frame
    self.addr = addr

  def mark_loaded(self, state: ST) -> None:
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


class _CellManager(Generic[ST, ADDR]):
  def __init__(self, game: StateSequence[ST, ADDR], capacity: int) -> None:
    self.game = game
    self.capacity = capacity

    self.cells = [self.base_cell] + [self.new_cell() for _ in range(capacity - 1)]

    self.temp_cell = self.cells.pop()

    # The base cell is the only one that SM64 C code can be run on, since
    # its pointers refer to its own memory
    self.base_cell: _Cell[ST, ADDR] = _Cell(-1, self.game.base_state())
    self.cells.insert(0, self.base_cell)
    for cell in self.cells:
      self.copy_cell(cell, self.base_cell, unsafe=True)

    # Keep one cell fixed at frame 0
    self.power_on_cell = self.cells[1]

    self.hotspots: Dict[str, int] = {}

    # Prevent callback from keeping self alive
    weak_self = weakref.ref(self)
    def invalidate(frame: int) -> None:
      self_ref = weak_self()
      if self_ref is not None:
        self_ref.invalidate_frame(frame)
    self.game.on_invalidation(invalidate)

  def __del__(self) -> None:
    for cell in self.cells:
      self.game.dealloc_state_buffer(cell.addr)

  def can_modify_cell(self, cell: _Cell[ST, ADDR]) -> bool:
    return cell is not self.power_on_cell and not cell.loaded

  def new_cell(self) -> _Cell[ST, ADDR]:
    # Frame "-1" is a power-on state before any edits have been applied.
    # Frame 0 is after edits are applied but before the first frame advance.
    return _Cell(-1, self.game.alloc_state_buffer())

  def copy_cell(
    self,
    dst: _Cell[ST, ADDR],
    src: _Cell[ST, ADDR],
    unsafe: bool = False,
  ) -> None:
    if not unsafe:
      assert self.can_modify_cell(dst)
    if src is not dst:
      self.game.raw_copy_state(dst.addr, src.addr)
      dst.frame = src.frame

  def advance_base_cell(self) -> None:
    assert self.can_modify_cell(self.base_cell)

    if self.base_cell.frame != -1:
      self.game.execute_frame()
    self.base_cell.frame += 1

    temp_state = self.load_cell(self.base_cell)
    self.game.apply_edits(temp_state)

  def swap_cell_contents(self, cell1: _Cell[ST, ADDR], cell2: _Cell[ST, ADDR]) -> None:
    if cell1 is not cell2:
      self.copy_cell(self.temp_cell, cell1)
      self.copy_cell(cell1, cell2)
      self.copy_cell(cell2, self.temp_cell)

  def load_cell(self, cell: _Cell[ST, ADDR]) -> ST:
    state = self.game.to_owned(self.base_cell.addr, cell.frame, cell.addr)
    cell.mark_loaded(state)
    return state

  def invalidate_frame(self, frame: int) -> None:
    valid_cells = [cell for cell in self.cells if cell.frame < frame]
    invalid_cells = [cell for cell in self.cells if cell.frame >= frame]

    for invalid_cell in invalid_cells:
      valid_cell = random.choice(valid_cells)
      self.copy_cell(invalid_cell, valid_cell)

  def find_latest_cell_before(self, frame: int) -> _Cell:
    usable_cells = [cell for cell in self.cells if cell.frame <= frame]
    return max(usable_cells, key=lambda cell: cell.frame)

  def request_frame(self, frame: int, based: bool = False) -> ST:
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

  def set_hotspot(self, name: str, frame: int) -> None:
    self.hotspots[name] = frame

  def delete_hotspot(self, name: str) -> None:
    if name in self.hotspots:
      del self.hotspots[name]

  def get_timeline_length(self) -> int:
    # TODO: Compute this correctly
    return self.game.get_num_frames()

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
    for hotspot in self.hotspots.values():
      for i in range(-60, 61, 5):
        if hotspot + i in range(self.get_timeline_length()):
          buckets[max(hotspot + i, 0)] = []

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


class GenericTimeline(Generic[ST, ADDR]):
  def __init__(self, game: StateSequence[ST, ADDR]) -> None:
    self._cell_manager = _CellManager(game, capacity=200)
    self._game = game

  def __getitem__(self, frame: int) -> ST:
    return self._cell_manager.request_frame(frame)

  def __len__(self) -> int:
    # TODO: Handle length better
    return self._game.get_num_frames()

  def set_hotspot(self, name: str, frame: int) -> None:
    """Mark a certain frame as a "hotspot", which is a hint to try to ensure
    that scrolling near the frame is smooth.
    """
    self._cell_manager.set_hotspot(name, frame)

  def delete_hotspot(self, name: str) -> None:
    self._cell_manager.delete_hotspot(name)

  def balance_distribution(self, max_run_time: float) -> None:
    """Perform maintenance to maintain a nice distribution of loaded frames."""
    self._cell_manager.balance_distribution(max_run_time)

  def get_loaded_frames(self) -> List[int]:
    return self._cell_manager.get_loaded_frames()
