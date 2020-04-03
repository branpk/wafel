from __future__ import annotations

from typing import *
from abc import ABC, abstractmethod
import traceback
from dataclasses import dataclass

from wafel.core import Slot, Game, DataPath, Controller
from wafel.util import *
from wafel.sm64_util import *


@dataclass(frozen=True)
class Script:
  source: str


@dataclass
class ScriptedSegment:
  frame_start: int
  frame_stop: Optional[int]
  script: Script

  def __contains__(self, frame: int) -> bool:
    return frame >= self.frame_start and \
      (self.frame_stop is None or frame < self.frame_stop)

  def split(self, frame: int) -> List[ScriptedSegment]:
    if frame <= self.frame_start:
      return [self]
    if self.frame_stop is not None and frame >= self.frame_stop:
      return [self]
    return [
      ScriptedSegment(self.frame_start, frame, self.script),
      ScriptedSegment(frame, self.frame_stop, self.script),
    ]


class Scripts:
  def __init__(self) -> None:
    self._segments = [ScriptedSegment(0, None, Script(''))]
    self._on_change_callbacks: List[Callable[[int], None]] = []

  def on_change(self, callback: Callable[[int], None]) -> None:
    self._on_change_callbacks.append(callback)

  def _notify(self, frame: int) -> None:
    for callback in list(self._on_change_callbacks):
      callback(frame)

  def get(self, frame: int) -> Script:
    for segment in self._segments:
      if frame in segment:
        return segment.script
    assert False

  @property
  def segments(self) -> List[ScriptedSegment]:
    return self._segments

  def split_segment(self, frame: int) -> None:
    new_segments: List[ScriptedSegment] = []
    for segment in self._segments:
      if frame in segment:
        new_segments += segment.split(frame)
      else:
        new_segments.append(segment)
    self._segments = new_segments

  def delete_segment(self, segment: ScriptedSegment, merge_upward: bool) -> None:
    i = self._segments.index(segment)
    if merge_upward:
      assert i > 0
      self._segments[i - 1].frame_stop = segment.frame_stop
    else:
      assert i < len(self._segments) - 1
      self._segments[i + 1].frame_start = segment.frame_start
    del self._segments[i]
    self._notify(segment.frame_start)

  def set_segment_source(self, segment: ScriptedSegment, source: str) -> None:
    if source != segment.script:
      segment.script = Script(source)
      self._notify(segment.frame_start)

  def set_frame_source(self, frame: int, source: str) -> None:
    self.split_segment(frame)
    for segment in self._segments:
      if frame in segment:
        self.set_segment_source(segment, source)

  def is_edited(self, frame: int) -> bool:
    return any(seg.frame_start == frame for seg in self._segments)

  def reset_frame(self, frame: int) -> None:
    for segment in self._segments:
      if segment.frame_start == frame:
        break
    else:
      return
    self.delete_segment(segment, merge_upward=True)


def to_int(value: object) -> int:
  assert isinstance(value, int) or isinstance(value, float)
  return int(value)

def to_float(value: object) -> float:
  assert isinstance(value, int) or isinstance(value, float)
  return float(value)


class ScriptController(Controller):
  def __init__(self, scripts: Scripts) -> None:
    super().__init__()
    self.scripts = scripts
    self.scripts.on_change(self.weak_notify)

  def get_globals(self, game: Game, frame: int, slot: Slot) -> dict:
    def from_int_yaw(int_yaw: object, int_mag: object = 32.0) -> Tuple[int, int]:
      return intended_to_raw_impl(
        game, slot, to_int(int_yaw), to_float(int_mag), relative_to=0
      )

    def from_dyaw(dyaw: object, int_mag: object = 32.0) -> Tuple[int, int]:
      # TODO: How to get this accurately?
      active_face_yaw = dcast(int, game.path('gMarioState[].faceAngle[1]').get(slot))
      int_yaw = active_face_yaw + to_int(dyaw)
      return intended_to_raw_impl(
        game, slot, int_yaw, to_float(int_mag), relative_to=active_face_yaw
      )

    return {
      'from_int_yaw': from_int_yaw,
      'from_dyaw': from_dyaw,
    }

  def run_script(self, game: Game, frame: int, slot: Slot, script: Script) -> None:
    # TODO: Error handling
    # TODO: Redirect stdout/stderr

    script_globals = dict(self.get_globals(game, frame, slot))
    script_locals: dict = {}

    try:
      exec(script.source, script_globals, script_locals)

      stick = script_locals.get('stick')
      stick_x = script_locals.get('stick_x')
      stick_y = script_locals.get('stick_y')

      if stick is not None:
        assert isinstance(stick, tuple)
        assert len(stick) == 2
        if stick_x is None:
          stick_x = stick[0]
        if stick_y is None:
          stick_y = stick[1]

      if stick_x is not None:
        assert isinstance(stick_x, float) or isinstance(stick_x, int)
        stick_x = min(max(int(stick_x), -128), 127)
        game.path('gControllerPads[0].stick_x').set(slot, stick_x)

      if stick_y is not None:
        assert isinstance(stick_y, float) or isinstance(stick_y, int)
        stick_y = min(max(int(stick_y), -128), 127)
        game.path('gControllerPads[0].stick_y').set(slot, stick_y)

    except:
      log.warn('Script error:\n' + traceback.format_exc())

  def apply(self, game: Game, frame: int, slot: Slot) -> None:
    script = self.scripts.get(frame)
    self.run_script(game, frame, slot, script)
