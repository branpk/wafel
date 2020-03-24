from typing import *

from wafel.core.variable import VariableGroup, DataVariableSpec as data, \
  FlagVariableSpec as flag


VARIABLES = {
  VariableGroup('Input'): {
    'input-stick-x': data('$state.gControllerPads[0].stick_x').label('stick x'),
    'input-stick-y': data('$state.gControllerPads[0].stick_y').label('stick y'),
    'input-buttons': data('$state.gControllerPads[0].button').hidden(),
    'input-button-a': flag('input-buttons', 'A_BUTTON').label('A'),
    'input-button-b': flag('input-buttons', 'B_BUTTON').label('B'),
    'input-button-z': flag('input-buttons', 'Z_TRIG').label('Z'),
    'input-button-s': flag('input-buttons', 'START_BUTTON').label('S'),
    'input-button-l': flag('input-buttons', 'L_TRIG').label('L'),
    'input-button-r': flag('input-buttons', 'R_TRIG').label('R'),
    'input-button-cu': flag('input-buttons', 'U_CBUTTONS').label('C^'),
    'input-button-cl': flag('input-buttons', 'L_CBUTTONS').label('C<'),
    'input-button-cr': flag('input-buttons', 'R_CBUTTONS').label('C>'),
    'input-button-cd': flag('input-buttons', 'D_CBUTTONS').label('Cv'),
    'input-button-du': flag('input-buttons', 'U_JPAD').label('D^'),
    'input-button-dl': flag('input-buttons', 'L_JPAD').label('D<'),
    'input-button-dr': flag('input-buttons', 'R_JPAD').label('D>'),
    'input-button-dd': flag('input-buttons', 'D_JPAD').label('Dv'),
  },
  VariableGroup('Mario'): {
    'mario-pos-x': data('$state.gMarioState[].pos[0]').label('pos x'),
    'mario-pos-y': data('$state.gMarioState[].pos[1]').label('pos y'),
    'mario-pos-z': data('$state.gMarioState[].pos[2]').label('pos z'),
    'mario-vel-f': data('$state.gMarioState[].forwardVel').label('vel f'),
    'mario-vel-x': data('$state.gMarioState[].vel[0]').label('vel x'),
    'mario-vel-y': data('$state.gMarioState[].vel[1]').label('vel y'),
    'mario-vel-z': data('$state.gMarioState[].vel[2]').label('vel z'),
    'mario-face-pitch': data('$state.gMarioState[].faceAngle[0]').label('face pitch'),
    'mario-face-yaw': data('$state.gMarioState[].faceAngle[1]').label('face yaw'),
    'mario-face-roll': data('$state.gMarioState[].faceAngle[2]').label('face roll'),
    'mario-action': data('$state.gMarioState[].action').label('action'),
  },
  VariableGroup('Misc'): {
    'global-timer': data('$state.gGlobalTimer').label('global timer'),
    'camera-yaw': data('$state.gMarioState[].area[].camera[].yaw').label('camera yaw'),
    'level-num': data('$state.gCurrLevelNum').label('level'),
    'area-index': data('$state.gCurrAreaIndex').label('area'),
  },
  VariableGroup.all_objects(): {
    'obj-active-flags': data('$object.activeFlags').hidden(),
    'obj-active-flags-active': flag('obj-active-flags', 'ACTIVE_FLAG_ACTIVE').label('active'),
    'obj-behavior-ptr': data('$object.behavior').hidden(),
    'obj-hitbox-radius': data('$object.hitboxRadius').label('hitbox radius'),
    'obj-hitbox-height': data('$object.hitboxHeight').label('hitbox height'),
    'obj-pos-x': data('$object.oPosX').label('pos x'),
    'obj-pos-y': data('$object.oPosY').label('pos y'),
    'obj-pos-z': data('$object.oPosZ').label('pos z'),
    'obj-vel-f': data('$object.oForwardVel').label('vel f'),
    'obj-vel-x': data('$object.oVelX').label('vel x'),
    'obj-vel-y': data('$object.oVelY').label('vel y'),
    'obj-vel-z': data('$object.oVelZ').label('vel z'),
  },
  VariableGroup('Surface'): {
    'surface-normal-x': data('$surface.normal.x').label('normal x'),
    'surface-normal-y': data('$surface.normal.y').label('normal y'),
    'surface-normal-z': data('$surface.normal.z').label('normal z'),
    'surface-vertex1-x': data('$surface.vertex1[0]').label('x1'),
    'surface-vertex1-y': data('$surface.vertex1[1]').label('y1'),
    'surface-vertex1-z': data('$surface.vertex1[2]').label('z1'),
    'surface-vertex2-x': data('$surface.vertex2[0]').label('x2'),
    'surface-vertex2-y': data('$surface.vertex2[1]').label('y2'),
    'surface-vertex2-z': data('$surface.vertex2[2]').label('z2'),
    'surface-vertex3-x': data('$surface.vertex3[0]').label('x3'),
    'surface-vertex3-y': data('$surface.vertex3[1]').label('y3'),
    'surface-vertex3-z': data('$surface.vertex3[2]').label('z3'),
  }
}
