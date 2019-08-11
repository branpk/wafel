from enum import Enum
from typing import List
import math

from butter.game_state import GameState


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


class RenderInfo:
  def __init__(
    self,
    camera: Camera,
    current_state: GameState,
  ) -> None:
    self.camera = camera
    self.current_state = current_state

  # TODO: Make expression evaluation available to ext module to compute offsets
