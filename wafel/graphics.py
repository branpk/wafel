from enum import Enum
from typing import *
import math

from wafel.core import GameState, DataPath, VariableParam

import ext_modules.graphics as ext_graphics


class GameStateWrapper:
  def __init__(self, state: GameState) -> None:
    self.state = state

  @property
  def frame(self) -> int:
    return self.state.frame

  def get_data(self, path: str) -> Any:
    data_path = DataPath.parse(self.state.lib, path)
    return data_path.get({
      VariableParam.STATE: self.state,
    })

  def get_data_addr(self, path: str) -> int:
    data_path = DataPath.parse(self.state.lib, path)
    return data_path.get_addr({
      VariableParam.STATE: self.state,
    })


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
    pos: List[float],
    pitch: float,
    yaw: float,
    fov_y: float,
  ) -> None:
    super().__init__(CameraMode.ROTATE)
    self.pos = pos
    self.pitch = pitch
    self.yaw = yaw
    self.fov_y = fov_y

  def face_dir(self) -> List[float]:
    return [
      math.cos(self.pitch) * math.sin(self.yaw),
      math.sin(self.pitch),
      math.cos(self.pitch) * math.cos(self.yaw),
    ]

class BirdsEyeCamera(Camera):
  def __init__(
    self,
    pos: List[float],
    span_y: float,
  ) -> None:
    super().__init__(CameraMode.BIRDS_EYE)
    self.pos = pos
    self.span_y = span_y


class RenderInfo:
  def __init__(
    self,
    viewport: Viewport,
    camera: Camera,
    current_state: GameState,
    path_states: List[GameState],
  ) -> None:
    self.viewport = viewport
    self.camera = camera
    self.current_state = GameStateWrapper(current_state)
    self.path_states = [GameStateWrapper(st) for st in path_states]


class Renderer:
  def __init__(self):
    self._addr = ext_graphics.new_renderer()

  def __del__(self):
    if ext_graphics.delete_renderer is not None:
      ext_graphics.delete_renderer(self._addr)

  def render(self, info: RenderInfo):
    ext_graphics.render(self._addr, info)
