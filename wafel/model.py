from typing import *
from ctypes import cdll
import json

from wafel.core import GameLib, Variable, Edits, Timeline, GameState, ObjectId, \
  ObjectType, VariableParam


class Model:
  def __init__(self):
    dll = cdll.LoadLibrary('lib/libsm64/jp/sm64')
    with open('lib/libsm64/jp/libsm64.json', 'r') as f:
      spec: dict = json.load(f)
    self.lib = GameLib(spec, dll)

    self.variables = Variable.create_all(self.lib)

    with open('test_files/1key_j.m64', 'rb') as m64:
      self.edits = Edits.from_m64(m64, self.variables)

    self.timeline = Timeline(self.lib, self.edits)

    self._selected_frame = 0
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
