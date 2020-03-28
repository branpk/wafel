from typing import *

from wafel.core import ScriptContext, GameState, DataPath
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
    self.lib = model.lib
    self.model = model

  def from_int_yaw(self, state: GameState) -> object:
    def impl(int_yaw: object, int_mag: object = 32.0) -> Tuple[int, int]:
      return intended_to_raw(
        self.lib, state, to_int(int_yaw), to_float(int_mag), relative_to=0
      )
    return impl

  def from_dyaw(self, state: GameState) -> object:
    def impl(dyaw: object, int_mag: object = 32.0) -> Tuple[int, int]:
      # TODO: How to get this accurately?
      active_face_yaw = dcast(int, DataPath.compile(self.lib, '$state.gMarioState[].faceAngle[1]').get(state))
      int_yaw = active_face_yaw + to_int(dyaw)
      return intended_to_raw(
        self.lib, state, int_yaw, to_float(int_mag), relative_to=active_face_yaw
      )
    return impl


  def get_globals(self, state: GameState) -> dict:
    return {
      'from_int_yaw': self.from_int_yaw(state),
      'from_dyaw': self.from_dyaw(state),
    }
