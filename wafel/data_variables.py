from typing import *
from dataclasses import dataclass

from wafel.variable import Variable, UndefinedVariableError
from wafel.core import Timeline, DataPath, Slot, Game


def data(*args, **kwargs):
  return (args, kwargs)


DATA_VARIABLES = {
  'Input': {
    'input-stick-x': data('gControllerPads[0].stick_x', label='stick x'),
    'input-stick-y': data('gControllerPads[0].stick_y', label='stick y'),
    'input-buttons': data('gControllerPads[0].button'),
    'input-button-a': data('gControllerPads[0].button', flag='A_BUTTON', label='A'),
    'input-button-b': data('gControllerPads[0].button', flag='B_BUTTON', label='B'),
    'input-button-z': data('gControllerPads[0].button', flag='Z_TRIG', label='Z'),
    'input-button-s': data('gControllerPads[0].button', flag='START_BUTTON', label='S'),
    'input-button-l': data('gControllerPads[0].button', flag='L_TRIG', label='L'),
    'input-button-r': data('gControllerPads[0].button', flag='R_TRIG', label='R'),
    'input-button-cu': data('gControllerPads[0].button', flag='U_CBUTTONS', label='C^'),
    'input-button-cl': data('gControllerPads[0].button', flag='L_CBUTTONS', label='C<'),
    'input-button-cr': data('gControllerPads[0].button', flag='R_CBUTTONS', label='C>'),
    'input-button-cd': data('gControllerPads[0].button', flag='D_CBUTTONS', label='Cv'),
    'input-button-du': data('gControllerPads[0].button', flag='U_JPAD', label='D^'),
    'input-button-dl': data('gControllerPads[0].button', flag='L_JPAD', label='D<'),
    'input-button-dr': data('gControllerPads[0].button', flag='R_JPAD', label='D>'),
    'input-button-dd': data('gControllerPads[0].button', flag='D_JPAD', label='Dv'),
  },
  'Mario': {
    'mario-pos-x': data('gMarioState[].pos[0]', label='pos x'),
    'mario-pos-y': data('gMarioState[].pos[1]', label='pos y'),
    'mario-pos-z': data('gMarioState[].pos[2]', label='pos z'),
    'mario-vel-f': data('gMarioState[].forwardVel', label='vel f'),
    'mario-vel-x': data('gMarioState[].vel[0]', label='vel x'),
    'mario-vel-y': data('gMarioState[].vel[1]', label='vel y'),
    'mario-vel-z': data('gMarioState[].vel[2]', label='vel z'),
    'mario-face-pitch': data('gMarioState[].faceAngle[0]', label='face pitch'),
    'mario-face-yaw': data('gMarioState[].faceAngle[1]', label='face yaw'),
    'mario-face-roll': data('gMarioState[].faceAngle[2]', label='face roll'),
    'mario-action': data('gMarioState[].action', label='action'),
  },
  'Misc': {
    'global-timer': data('gGlobalTimer', label='global timer'),
    'camera-yaw': data('gMarioState[].area[].camera[].yaw', label='camera yaw'),
    'level-num': data('gCurrLevelNum', label='level'),
    'area-index': data('gCurrAreaIndex', label='area'),
  },
  'Object': {
    'obj-active-flags-active': data('struct Object.activeFlags', flag='ACTIVE_FLAG_ACTIVE', label='active'),
    'obj-behavior-ptr': data('struct Object.behavior'),
    'obj-hitbox-radius': data('struct Object.hitboxRadius', label='hitbox radius'),
    'obj-hitbox-height': data('struct Object.hitboxHeight', label='hitbox height'),
    'obj-pos-x': data('struct Object.oPosX', label='pos x'),
    'obj-pos-y': data('struct Object.oPosY', label='pos y'),
    'obj-pos-z': data('struct Object.oPosZ', label='pos z'),
    'obj-vel-f': data('struct Object.oForwardVel', label='vel f'),
    'obj-vel-x': data('struct Object.oVelX', label='vel x'),
    'obj-vel-y': data('struct Object.oVelY', label='vel y'),
    'obj-vel-z': data('struct Object.oVelZ', label='vel z'),
  },
  'Surface': {
    'surface-normal-x': data('struct Surface.normal.x', label='normal x'),
    'surface-normal-y': data('struct Surface.normal.y', label='normal y'),
    'surface-normal-z': data('struct Surface.normal.z', label='normal z'),
    'surface-vertex1-x': data('struct Surface.vertex1[0]', label='x1'),
    'surface-vertex1-y': data('struct Surface.vertex1[1]', label='y1'),
    'surface-vertex1-z': data('struct Surface.vertex1[2]', label='z1'),
    'surface-vertex2-x': data('struct Surface.vertex2[0]', label='x2'),
    'surface-vertex2-y': data('struct Surface.vertex2[1]', label='y2'),
    'surface-vertex2-z': data('struct Surface.vertex2[2]', label='z2'),
    'surface-vertex3-x': data('struct Surface.vertex3[0]', label='x3'),
    'surface-vertex3-y': data('struct Surface.vertex3[1]', label='y3'),
    'surface-vertex3-z': data('struct Surface.vertex3[2]', label='z3'),
  }
}


@dataclass(frozen=True)
class DataVariableSpec:
  group: str
  path: DataPath
  flag: Optional[int]
  label: Optional[str]


class DataVariables:
  def __init__(self, game: Game) -> None:
    self.game = game

    self.specs: Dict[str, DataVariableSpec] = {}
    for group, variables in DATA_VARIABLES.items():
      for name, (args, kwargs) in variables.items():
        flag = kwargs.get('flag')
        if flag is not None:
          flag = self.game.memory.data_spec['constants'][flag]['value']
        self.specs[name] = DataVariableSpec(
          group = group,
          path = self.game.path(args[0]),
          flag = flag,
          label = kwargs.get('label'),
        )

  def __getitem__(self, variable: Variable) -> DataVariableSpec:
    try:
      return self.specs[variable.name]
    except KeyError:
      raise UndefinedVariableError(variable)

  def group(self, group: str) -> List[Variable]:
    return [Variable(var) for var, spec in self.specs.items() if spec.group == group]

  def get_path(self, variable: Variable) -> DataPath:
    path = self[variable].path

    object_slot = variable.args.get('object')
    if object_slot is not None:
      path = self.game.path(f'gObjectPool[{object_slot}]') + path

    surface_index = variable.args.get('surface')
    if surface_index is not None:
      path = self.game.path(f'sSurfacePool[{surface_index}]') + path

    return path

  def get(self, timeline: Timeline, variable: Variable) -> object:
    spec = self[variable]
    path = self.get_path(variable)

    value = timeline.get(variable.args['frame'], path)
    if spec.flag is None or value is None:
      return value

    assert isinstance(value, int)
    return (value & spec.flag) != 0

  def get_raw(self, slot: Slot, variable: Variable) -> object:
    spec = self[variable]
    path = self.get_path(variable)

    value = path.get(slot)
    if spec.flag is None or value is None:
      return value

    assert isinstance(value, int)
    return (value & spec.flag) != 0

  def set_raw(self, slot: Slot, variable: Variable, value: object) -> None:
    spec = self[variable]
    path = self.get_path(variable)

    if spec.flag is None:
      path.set(slot, value)
      return

    flags = path.get(slot)
    if flags is not None:
      assert isinstance(flags, int)
      assert isinstance(value, bool)
      if value:
        flags |= spec.flag
      else:
        flags &= ~spec.flag
      path.set(slot, flags)
