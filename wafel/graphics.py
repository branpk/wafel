from __future__ import annotations

from enum import Enum
from typing import *
import math

from wafel.core import GameState, DataPath, VariableParam, GameLib
import wafel.config as config

import ext_modules.graphics as c_graphics # type: ignore


class GameStateWrapper:
  def __init__(self, lib: GameLib, state: GameState) -> None:
    self.lib = lib
    self.state = state

  @property
  def frame(self) -> int:
    return self.state.frame

  def get_data(self, path: str) -> Any:
    data_path = DataPath.parse(self.lib, path)
    return data_path.get({
      VariableParam.STATE: self.state,
    })

  def get_data_addr(self, path: str) -> int:
    data_path = DataPath.parse(self.lib, path)
    return data_path.get_addr({
      VariableParam.STATE: self.state,
    })


Vec3f = Tuple[float, float, float]


class CameraMode(Enum):
  ROTATE = 0
  BIRDS_EYE = 1


class Viewport:
  def __init__(self, x: int, y: int, width: int, height: int):
    self.x = x
    self.y = y
    self.width = width
    self.height = height


class Camera:
  def __init__(self, mode: CameraMode) -> None:
    self.mode = mode

class RotateCamera(Camera):
  def __init__(
    self,
    pos: Vec3f,
    pitch: float,
    yaw: float,
    fov_y: float,
  ) -> None:
    super().__init__(CameraMode.ROTATE)
    self.pos = pos
    self.pitch = pitch
    self.yaw = yaw
    self.fov_y = fov_y

class BirdsEyeCamera(Camera):
  def __init__(
    self,
    pos: Vec3f,
    span_y: float,
  ) -> None:
    super().__init__(CameraMode.BIRDS_EYE)
    self.pos = pos
    self.span_y = span_y


class RenderInfo:
  def __init__(
    self,
    lib: GameLib,
    viewport: Viewport,
    camera: Camera,
    current_state: GameState,
    path_states: List[GameState],
  ) -> None:
    self.viewport = viewport
    self.camera = camera
    self.current_state = GameStateWrapper(lib, current_state)
    self.path_states = [GameStateWrapper(lib, st) for st in path_states]


class Renderer:
  _instance: Optional[Renderer] = None

  @staticmethod
  def get() -> Renderer:
    if Renderer._instance is None:
      Renderer._instance = Renderer()
    return Renderer._instance

  def __init__(self):
    assert Renderer._instance is None
    self._addr = c_graphics.new_renderer(config.assets_directory)

  def __del__(self):
    if c_graphics.delete_renderer is not None:
      c_graphics.delete_renderer(self._addr)

  def render(self, info: RenderInfo):
    c_graphics.render(self._addr, info)
