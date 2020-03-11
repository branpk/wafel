from typing import *
from ctypes import cdll
import json
import gc
import os

from wafel.config import config
from wafel.core import GameLib, Variable, Edits, Timeline, GameState, ObjectId, \
  ObjectType, VariableParam, load_libsm64


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

    # TODO: Loading bar?
    self.lib = load_libsm64(game_version)

    self.variables = Variable.create_all(self.lib)

  def _set_edits(self, edits: Edits) -> None:
    self.timeline = None
    gc.collect() # Force garbage collection of game state slots

    self.edits = edits
    self.timeline = Timeline(self.lib, self.variables, self.edits)

    self._selected_frame = 0
    self.selected_frame_callbacks: List[Callable[[int], None]] = []

    def set_hotspot(frame: int) -> None:
      self.timeline.set_hotspot('selected-frame', max(frame - 5, 0))
    self.on_selected_frame_change(set_hotspot)
    set_hotspot(self._selected_frame)

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
    with self.timeline[frame] as state:
      return variable.get({ VariableParam.STATE: state })

  def get_object_type(self, state: GameState, object_id: ObjectId) -> Optional[ObjectType]:
    active = self.variables['obj-active-flags-active'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    if not active:
      return None

    behavior_addr = self.variables['obj-behavior-ptr'].at_object(object_id).get({
      VariableParam.STATE: state,
    })
    relative_addr = state.slot.addr_to_relative(behavior_addr)
    return ObjectType(
      relative_addr,
      self.lib.symbol_for_addr(relative_addr),
    )
