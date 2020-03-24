from typing import *
from ctypes import cdll
import json
import gc
import os

import ext_modules.util as c_util

import wafel.config as config
from wafel.core import GameLib, Variable, Edits, Timeline, GameState, ObjectId, \
  ObjectType, load_libsm64, RelativeAddr
from wafel.loading import Loading
from wafel.util import *


# TODO: Should create a new Model on every load

class Model:

  def __init__(self) -> None:
    self.game_version: str
    self.timeline: Timeline
    self.rotational_camera_yaw = 0

  def load(self, game_version: str, edits: Edits) -> Loading[None]:
    yield from self._load_lib(game_version)
    self._set_edits(edits)

  def change_version(self, game_version: str) -> Loading[None]:
    yield from self.load(game_version, self.edits)

  def _load_lib(self, game_version: str) -> Loading[None]:
    if hasattr(self, 'game_version') and self.game_version == game_version:
      return
    self.game_version = game_version

    self.lib = yield from load_libsm64(game_version)
    c_util.init(self.lib.static_addr)

    self.variables = Variable.create_all(self.lib)

    self.action_names: Dict[int, str] = {}
    for constant_name, constant in self.lib.spec['constants'].items():
      if constant_name.startswith('ACT_') and \
          not any(constant_name.startswith(s) for s in ['ACT_FLAG_', 'ACT_GROUP_', 'ACT_ID_']):
        action_name = constant_name.lower()[len('act_'):].replace('_', ' ')
        self.action_names[constant['value']] = action_name

  def _set_edits(self, edits: Edits) -> None:
    if hasattr(self, 'timeline'):
      del self.timeline
    gc.collect() # Force garbage collection of game state slots

    self.edits = edits
    self.timeline = Timeline(self.lib, self.variables, self.edits)

    self._selected_frame = 0
    if config.dev_mode:
      self._selected_frame = 3299
    self.selected_frame_callbacks: List[Callable[[int], None]] = []

    def set_hotspot(frame: int) -> None:
      self.timeline.set_hotspot('selected-frame', frame)
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
      return variable.get(state)

  def get_object_type(self, state: GameState, object_id: ObjectId) -> Optional[ObjectType]:
    active = self.variables['obj-active-flags-active'].at_object(object_id).get(state)
    if not active:
      return None

    behavior_addr = self.variables['obj-behavior-ptr'].at_object(object_id).get(state)
    assert isinstance(behavior_addr, RelativeAddr)
    return ObjectType(
      behavior_addr,
      self.lib.symbol_for_addr(behavior_addr),
    )

  def get_object_type_cached(self, frame: int, object_id: ObjectId) -> Optional[ObjectType]:
    active = self.timeline.get_cached(
      frame,
      self.variables['obj-active-flags-active'].at_object(object_id),
    )
    if not active:
      return None

    behavior_addr = self.timeline.get_cached(
      frame,
      self.variables['obj-behavior-ptr'].at_object(object_id),
    )
    assert isinstance(behavior_addr, RelativeAddr)
    return ObjectType(
      behavior_addr,
      self.lib.symbol_for_addr(behavior_addr),
    )
