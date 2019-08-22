from enum import Enum
from typing import List
import math

from butter.game_state import GameState

import _ext_modules.graphics as ext_graphics


class CameraMode(Enum):
  ROTATE = 0
  BIRDS_EYE = 1


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
    camera: Camera,
    current_state: GameState,
    path_states: List[GameState],
  ) -> None:
    self.camera = camera
    self.current_state = current_state
    self.path_states = path_states

  # TODO: Make expression evaluation available to ext module to compute offsets?


class Renderer:
  def __init__(self):
    """Only call in initializeGL."""
    self._addr = ext_graphics.new_renderer()

  def __del__(self):
    ext_graphics.delete_renderer(self._addr)

  def render(self, info: RenderInfo):
    """Only call in paintGL after makeCurrent is called."""
    ext_graphics.render(self._addr, info)
