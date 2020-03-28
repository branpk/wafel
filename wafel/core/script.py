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
  frame: int
  source: str


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

      stick_x = script_locals.get('stick_x')
      stick_y = script_locals.get('stick_y')

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
    self._post_edit: Dict[int, Script] = { 0: Script(0, '') }
    self._invalidation_callbacks: List[Callable[[int], None]] = []

  def on_invalidation(self, callback: Callable[[int], None]) -> None:
    self._invalidation_callbacks.append(callback)

  def _invalidate(self, frame: int) -> None:
    for callback in list(self._invalidation_callbacks):
      callback(frame)

  def post_edit(self, frame: int) -> Script:
    for prior_frame in range(frame, -1, -1):
      script = self._post_edit.get(prior_frame)
      if script is not None:
        return script
    assert False

  def set_post_edit_source(self, frame: int, source: str) -> None:
    current = self.post_edit(frame)
    self._post_edit[frame] = Script(frame, source)
    if current.source != source:
      self._invalidate(frame)

  def run_post_edit(self, state: GameState) -> None:
    script = self.post_edit(state.frame)
    if script is not None:
      self._context.run_state_script(state, script)
