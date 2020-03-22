import math
from typing import *
import itertools

import ext_modules.util as c_util

from wafel.util import *


def raw_to_adjusted(raw_stick_x: int, raw_stick_y: int) -> Tuple[float, float]:
  stick = c_util.stick_raw_to_adjusted(raw_stick_x, raw_stick_y)
  return stick.x, stick.y


def adjusted_to_raw(stick_x: float, stick_y: float) -> Tuple[int, int]:
  return cast(Tuple[int, int], c_util.stick_adjusted_to_raw(stick_x, stick_y))


def raw_to_intended(
  raw_stick_x: int,
  raw_stick_y: int,
  face_yaw: int,
  camera_yaw: int,
  squish_timer: int,
) -> Tuple[int, float]:
  adjusted = c_util.stick_raw_to_adjusted(raw_stick_x, raw_stick_y)
  intended = c_util.stick_adjusted_to_intended(adjusted, face_yaw, camera_yaw, squish_timer != 0)
  return cast(Tuple[int, float], intended)


def intended_to_raw(
  intended_yaw: int,
  intended_mag: float,
  face_yaw: int,
  camera_yaw: int,
  squish_timer: int,
) -> Tuple[int, int]:
  raw = c_util.stick_intended_to_raw_visual(
    trunc_signed(intended_yaw, 16), intended_mag, face_yaw, camera_yaw, squish_timer != 0,
  )
  yaw, mag = raw_to_intended(raw[0], raw[1], face_yaw, camera_yaw, squish_timer)
  return cast(Tuple[int, int], raw)
