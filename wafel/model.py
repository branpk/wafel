from typing import *
from ctypes import cdll
import json
import gc
import os
import weakref

import ext_modules.util as c_util
from ext_modules.core import Variable, Pipeline, ObjectBehavior

import wafel.config as config
from wafel.util import *
# from wafel.script import Scripts, ScriptController


class Model:

  def __init__(self) -> None:
    self.game_version: str
    self.pipeline: Pipeline
    self.rotational_camera_yaw = 0
    self.input_up_yaw: Optional[int] = None

  def load(self, game_version: str, edits: Dict[Variable, object]) -> None:
    self.game_version = game_version

    dll_path = os.path.join(config.lib_directory, 'libsm64', 'sm64_' + game_version + '.dll')
    self.pipeline = Pipeline.load(dll_path)

    def base_pointer(path: str) -> int:
      address = self.pipeline.path_address(0, path)
      return self.pipeline.address_to_base_pointer(address)
    c_util.init(base_pointer)

    self._set_edits(edits)

  def change_version(self, game_version: str) -> None:
    raise NotImplementedError # FIXME
    self.load(game_version, self.edits)

  def _set_edits(self, edits: Dict[Variable, object]) -> None:
    for variable, value in edits.items():
      self.pipeline.write(variable, value)
    self._max_frame = max((variable.frame or 0 for variable in edits), default=0)

    self._selected_frame = 0
    if config.dev_mode:
      self._selected_frame = 1580
    self.selected_frame_callbacks: List[Callable[[int], None]] = []

    self.play_speed = 0.0
    self.playback_mode = False

    def set_hotspot(frame: int) -> None:
      self.pipeline.set_hotspot('selected-frame', frame)
    self.on_selected_frame_change(set_hotspot)
    set_hotspot(self._selected_frame)

  # FrameSequence

  @property
  def selected_frame(self) -> int:
    return self._selected_frame

  @selected_frame.setter
  def selected_frame(self, frame: int) -> None:
    if frame != self._selected_frame:
      self._selected_frame = min(max(frame, 0), self._max_frame)
      for callback in list(self.selected_frame_callbacks):
        callback(self._selected_frame)

  def set_selected_frame(self, frame: int) -> None:
    self.selected_frame = frame

  def on_selected_frame_change(self, callback: Callable[[int], None]) -> None:
    self.selected_frame_callbacks.append(callback)

  @property
  def max_frame(self) -> int:
    return self._max_frame

  def extend_to_frame(self, frame: int) -> None:
    self._max_frame = max(self._max_frame, frame)

  def insert_frame(self, frame: int) -> None:
    self.pipeline.insert_frame(frame)
    if self.selected_frame >= frame:
      self.selected_frame += 1

  def delete_frame(self, frame: int) -> None:
    self.pipeline.delete_frame(frame)
    if self.selected_frame > frame or self.selected_frame > self._max_frame:
      self.selected_frame -= 1

  def set_hotspot(self, name: str, frame: int) -> None:
    self.pipeline.set_hotspot(name, frame)

  @overload
  def get(self, frame: int, path: str) -> object:
    ...
  @overload
  def get(self, variable: Variable) -> object:
    ...
  def get(self, arg1, arg2=None):
    if isinstance(arg1, Variable):
      variable: Variable = arg1
      return self.pipeline.read(variable)
    else:
      frame: int = arg1
      path: str = arg2
      return self.pipeline.path_read(frame, path)

  def get_object_behavior(self, frame: int, object_slot: int) -> Optional[ObjectBehavior]:
    return self.pipeline.object_behavior(frame, object_slot)

  def set(self, variable: Variable, value: object) -> None:
    self.pipeline.write(variable, value)

  def reset(self, variable: Variable) -> None:
    self.pipeline.reset(variable)

  # VariableDisplayer

  def label(self, variable: Variable) -> str:
    return assert_not_none(self.pipeline.label(variable))

  def column_header(self, variable: Variable) -> str:
    if variable.object is not None:
      if variable.object_behavior is None:
        return str(variable.object) + '\n' + self.label(variable)
      else:
        behavior_name = self.pipeline.object_behavior_name(variable.object_behavior)
        return str(variable.object) + ' - ' + behavior_name + '\n' + self.label(variable)

    elif variable.surface is not None:
      return f'Surface {variable.surface}\n{self.label(variable)}'

    else:
      return self.label(variable)
