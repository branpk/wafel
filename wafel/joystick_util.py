import math
from typing import *
import itertools


def raw_to_adjusted(raw_stick_x: int, raw_stick_y: int) -> Tuple[float, float]:
  stick_x = 0.0
  stick_y = 0.0

  if raw_stick_x <= -8:
    stick_x = raw_stick_x + 6
  if raw_stick_x >= 8:
    stick_x = raw_stick_x - 6
  if raw_stick_y <= -8:
    stick_y = raw_stick_y + 6
  if raw_stick_y >= 8:
    stick_y = raw_stick_y - 6

  stick_mag = math.sqrt(stick_x * stick_x + stick_y * stick_y)
  if stick_mag > 64:
    stick_x *= 64 / stick_mag
    stick_y *= 64 / stick_mag
    stick_mag = 64

  return (stick_x, stick_y)


def adjusted_to_raw(stick_x: float, stick_y: float) -> Tuple[float, int]:
  def dist(raw_stick: Tuple[int, int]) -> float:
    x, y = raw_to_adjusted(*raw_stick)
    return (stick_x - x)**2 + (stick_y - y)**2
  raw_sticks = itertools.product(range(-128, 127), range(-128, 127))
  return min(raw_sticks, key=dist)


def raw_to_intended(
  raw_stick_x: int,
  raw_stick_y: int,
  face_yaw: int,
  camera_yaw: int,
  squish_timer: int,
) -> Tuple[float, float]:
  stick_x, stick_y = raw_to_adjusted(raw_stick_x, raw_stick_y)
  stick_mag = math.sqrt(stick_x**2 + stick_y**2)
  mag = (stick_mag / 64) * (stick_mag / 64) * 64

  if squish_timer == 0:
    intended_mag = mag / 2
  else:
    intended_mag = mag / 8

  if intended_mag > 0:
    radians = math.atan2(stick_x, -stick_y) # TODO: Accurate angles
    intended_yaw = 0x8000 * radians / math.pi + camera_yaw
  else:
    intended_yaw = face_yaw

  return intended_yaw, intended_mag


def intended_to_raw(
  intended_yaw: float, # TODO: Use int and better dist
  intended_mag: float,
  face_yaw: int,
  camera_yaw: int,
  squish_timer: int,
) -> Tuple[float, float]:
  tx = intended_mag / 32 * math.sin(-intended_yaw * math.pi / 0x8000)
  ty = intended_mag / 32 * math.cos(intended_yaw * math.pi / 0x8000)
  def dist(raw_stick: Tuple[int, int]) -> float:
    int_yaw, int_mag = raw_to_intended(*raw_stick, face_yaw, camera_yaw, squish_timer)
    x = int_mag / 32 * math.sin(-int_yaw * math.pi / 0x8000)
    y = int_mag / 32 * math.cos(int_yaw * math.pi / 0x8000)
    return (tx - x)**2 + (ty - y)**2
  raw_sticks = itertools.product(range(-128, 127), range(-128, 127))
  return min(raw_sticks, key=dist)
