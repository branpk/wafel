from typing import *

from ext_modules.core import IntendedStick, stick_intended_to_raw_heuristic

from wafel.util import *


def intended_to_raw(
  face_yaw: int,
  camera_yaw: int,
  squish_timer: int,
  int_yaw: int,
  int_mag: float,
  relative_to: int,
) -> Tuple[int, int]:
  # TODO: This doesn't account for rotation from platform displacement (if face yaw is at start of frame)

  stick_x, stick_y = stick_intended_to_raw_heuristic(
    IntendedStick(
      trunc_signed(int_yaw, 16),
      min(max(int_mag, 0.0), 32.0),
    ),
    face_yaw,
    camera_yaw,
    squish_timer != 0,
    relative_to,
  )

  return cast(Tuple[int, int], (stick_x, stick_y))


__all__ = [
  'intended_to_raw',
]
