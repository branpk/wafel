from typing import *
from abc import ABC, abstractmethod
import traceback

from wafel.core.game_state import GameState
from wafel.core.game_lib import GameLib
from wafel.core.data_path import DataPath
from wafel.util import *


Script = str


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
      exec(script, script_globals, script_locals)

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
    self._post_edit: Dict[int, Script] = {}
    self._invalidation_callbacks: List[Callable[[int], None]] = []

  def on_invalidation(self, callback: Callable[[int], None]) -> None:
    self._invalidation_callbacks.append(callback)

  def _invalidate(self, frame: int) -> None:
    for callback in list(self._invalidation_callbacks):
      callback(frame)

  def post_edit(self, frame: int) -> Script:
    return self._post_edit.get(frame, '')

  def set_post_edit(self, frame: int, script: Script) -> None:
    if self.post_edit(frame) != script:
      if script == '':
        del self._post_edit[frame]
      else:
        self._post_edit[frame] = script
      self._invalidate(frame)

  def run_post_edit(self, state: GameState) -> None:
    script = self._post_edit.get(state.frame)
    if script is not None:
      self._context.run_state_script(state, script)
