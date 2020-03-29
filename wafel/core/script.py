from __future__ import annotations

from typing import *
from abc import ABC, abstractmethod
import traceback
from dataclasses import dataclass

from wafel.core.game_state import GameState
from wafel.core.game_lib import GameLib
from wafel.core.data_path import DataPath
from wafel.util import *


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


class ScriptContext:
  lib: GameLib

  @abstractmethod
  def get_globals(self, state: GameState) -> dict: ...

  def run_state_script(self, state: GameState, script: Script) -> None:
    # TODO: Error handling
    # TODO: Redirect stdout/stderr

    script_globals = dict(self.get_globals(state))
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
        DataPath.compile(self.lib, '$state.gControllerPads[0].stick_x').set(state, stick_x)

      if stick_y is not None:
        assert isinstance(stick_y, float) or isinstance(stick_y, int)
        stick_y = min(max(int(stick_y), -128), 127)
        DataPath.compile(self.lib, '$state.gControllerPads[0].stick_y').set(state, stick_y)

    except:
      log.warn('Script error:\n' + traceback.format_exc())


class Scripts:
  def __init__(self, context: ScriptContext) -> None:
    self._context = context
    self._segments = [ScriptedSegment(0, None, Script(''))]
    self._invalidation_callbacks: List[Callable[[int], None]] = []

  def on_invalidation(self, callback: Callable[[int], None]) -> None:
    self._invalidation_callbacks.append(callback)

  def _invalidate(self, frame: int) -> None:
    for callback in list(self._invalidation_callbacks):
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
    self._invalidate(segment.frame_start)

  def set_segment_source(self, segment: ScriptedSegment, source: str) -> None:
    if source != segment.script:
      segment.script = Script(source)
      self._invalidate(segment.frame_start)

  def run(self, state: GameState) -> None:
    script = self.get(state.frame)
    self._context.run_state_script(state, script)
