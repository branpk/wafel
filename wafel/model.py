from typing import *
from ctypes import cdll
import json
import gc
import os

from wafel.config import config
from wafel.core import GameLib, Variable, Edits, Timeline, GameState, ObjectId, \
  ObjectType, VariableParam


class Model:

  def load(self, game_version: str, edits: Edits) -> None:
    self._load_lib(game_version)
    self._set_edits(edits)

  def change_version(self, game_version: str) -> None:
    self.load(game_version, self.edits)

  def _load_lib(self, game_version: str) -> None:
    if hasattr(self, 'game_version') and self.game_version == game_version:
      return
    self.game_version = game_version

    dll = cdll.LoadLibrary(
      os.path.join(config.lib_directory, 'libsm64', f'sm64_{game_version}'))
    with open(os.path.join(libsm64, 'libsm64.json'), 'r') as f:
      spec = json.load(f)
    self.lib = GameLib(spec, dll)

    self.variables = Variable.create_all(self.lib)

  def _set_edits(self, edits: Edits) -> None:
    self.edits = edits
    self.timeline = Timeline(self.lib, self.variables, self.edits)

    self._selected_frame = 0
    self.selected_frame_callbacks: List[Callable[[int], None]] = []

    def set_hotspot(frame: int) -> None:
      self.timeline.set_hotspot('selected-frame', frame)
    self.on_selected_frame_change(set_hotspot)
    set_hotspot(self._selected_frame)

    gc.collect() # Force garbage collection of game state cells

  @property
  def selected_frame(self) -> int:
    return self._selected_frame

  @selected_frame.setter
  def selected_frame(self, frame: int) -> None:
    self._selected_frame = min(max(frame, 0), len(self.timeline) - 1)
    for callback in list(self.selected_frame_callbacks):
      callback(self._selected_frame)

  def on_selected_frame_change(self, callback: Callable[[int], None]) -> None:
    self.selected_frame_callbacks.append(callback)

  def insert_frame(self, index: int) -> None:
    self.edits.insert_frame(index)
    if self.selected_frame >= index:
      self.selected_frame += 1

  def delete_frame(self, index: int) -> None:
    self.edits.delete_frame(index)
    if self.selected_frame > index or self.selected_frame >= len(self.timeline):
      self.selected_frame -= 1

  def get(self, variable: Variable, frame: Optional[int] = None) -> Any:
    if frame is None:
      frame = self.selected_frame
    state = self.timeline[frame]
    return variable.get({ VariableParam.STATE: state })

  def get_object_type(self, state: GameState, object_id: ObjectId) -> Optional[ObjectType]:
    active = self.variables['obj-active-flags-active'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    if not active:
      return None

    behavior = self.variables['obj-behavior-ptr'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    return self.lib.get_object_type(behavior)
