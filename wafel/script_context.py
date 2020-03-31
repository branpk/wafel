from typing import *

from wafel.script import ScriptContext
from wafel.core import Game, Slot, DataPath
from wafel.model import Model
from wafel.util import *
from wafel.sm64_util import *


def to_int(value: object) -> int:
  assert isinstance(value, int) or isinstance(value, float)
  return int(value)

def to_float(value: object) -> float:
  assert isinstance(value, int) or isinstance(value, float)
  return float(value)


class SM64ScriptContext(ScriptContext):
  def __init__(self, model: Model) -> None:
    self.model = model

  @property
  def game(self) -> Game:
    return self.model.game

  def from_int_yaw(self, slot: Slot) -> object:
    def impl(int_yaw: object, int_mag: object = 32.0) -> Tuple[int, int]:
      return intended_to_raw_impl(
        self.game, slot, to_int(int_yaw), to_float(int_mag), relative_to=0
      )
    return impl

  def from_dyaw(self, slot: Slot) -> object:
    def impl(dyaw: object, int_mag: object = 32.0) -> Tuple[int, int]:
      # TODO: How to get this accurately?
      active_face_yaw = dcast(int, self.game.path('gMarioState[].faceAngle[1]').get(slot))
      int_yaw = active_face_yaw + to_int(dyaw)
      return intended_to_raw_impl(
        self.game, slot, int_yaw, to_float(int_mag), relative_to=active_face_yaw
      )
    return impl

  def get_globals(self, frame: int, slot: Slot) -> dict:
    return {
      'from_int_yaw': self.from_int_yaw(slot),
      'from_dyaw': self.from_dyaw(slot),
    }
