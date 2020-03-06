from typing import *

from wafel.core.cell_manager import CellManager, OwnedBuffer, Buffer
from wafel.core.game_state import GameState
from wafel.core.game_lib import GameLib
from wafel.core.edit import Edits
from wafel.core.variable import Variables
from wafel.core.variable_param import VariableParam


# TODO: These classes are solely for moving lib around. Get rid of them


class _EditsWrapper:
  def __init__(
    self,
    lib: GameLib,
    variables: Variables,
    edits: Edits,
  ) -> None:
    self.lib = lib
    self.variables = variables
    self.edits = edits

  def apply_edits(self, frame: int, buffer: OwnedBuffer) -> None:
    state = GameState(self.lib, frame, buffer, self.lib.base_buffer())
    for edit in self.edits.get_edits(state.frame):
      variable = self.variables[edit.variable_id]
      variable.set(edit.value, { VariableParam.STATE: state })

  def __len__(self) -> int:
    return len(self.edits)

  def on_edit(self, callback: Callable[[int], None]) -> None:
    self.edits.on_edit(callback)


class Timeline:
  def __init__(
    self,
    lib: GameLib,
    variables: Variables,
    edits: Edits,
  ) -> None:
    self.lib = lib
    self.cell_manager = CellManager(lib, _EditsWrapper(lib, variables, edits), capacity=200)

  def __getitem__(self, frame: int) -> GameState:
    buffer = self.cell_manager.request_frame(frame)
    return GameState(self.lib, frame, buffer, self.lib.base_buffer())

  def __len__(self) -> int:
    # TODO: Handle length better
    return self.cell_manager.get_timeline_length()

  def set_hotspot(self, name: str, frame: int) -> None:
    """Mark a certain frame as a "hotspot", which is a hint to try to ensure
    that scrolling near the frame is smooth.
    """
    self.cell_manager.set_hotspot(name, frame)

  def delete_hotspot(self, name: str) -> None:
    self.cell_manager.delete_hotspot(name)

  def balance_distribution(self, max_run_time: float) -> None:
    """Perform maintenance to maintain a nice distribution of loaded frames."""
    self.cell_manager.balance_distribution(max_run_time)

  def get_loaded_frames(self) -> List[int]:
    return self.cell_manager.get_loaded_frames()
