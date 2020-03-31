from typing import *

from wafel.variable import VariableGroup, DataVariableSpec as data, \
  FlagVariableSpec as flag


VARIABLES = {
  VariableGroup('Input'): {
    'input-stick-x': data('gControllerPads[0].stick_x').label('stick x'),
    'input-stick-y': data('gControllerPads[0].stick_y').label('stick y'),
    'input-buttons': data('gControllerPads[0].button').hidden(),
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
    'mario-pos-x': data('gMarioState[].pos[0]').label('pos x'),
    'mario-pos-y': data('gMarioState[].pos[1]').label('pos y'),
    'mario-pos-z': data('gMarioState[].pos[2]').label('pos z'),
    'mario-vel-f': data('gMarioState[].forwardVel').label('vel f'),
    'mario-vel-x': data('gMarioState[].vel[0]').label('vel x'),
    'mario-vel-y': data('gMarioState[].vel[1]').label('vel y'),
    'mario-vel-z': data('gMarioState[].vel[2]').label('vel z'),
    'mario-face-pitch': data('gMarioState[].faceAngle[0]').label('face pitch'),
    'mario-face-yaw': data('gMarioState[].faceAngle[1]').label('face yaw'),
    'mario-face-roll': data('gMarioState[].faceAngle[2]').label('face roll'),
    'mario-action': data('gMarioState[].action').label('action'),
  },
  VariableGroup('Misc'): {
    'global-timer': data('gGlobalTimer').label('global timer'),
    'camera-yaw': data('gMarioState[].area[].camera[].yaw').label('camera yaw'),
    'level-num': data('gCurrLevelNum').label('level'),
    'area-index': data('gCurrAreaIndex').label('area'),
  },
  VariableGroup.all_objects(): {
    'obj-active-flags': data('struct Object.activeFlags').hidden(),
    'obj-active-flags-active': flag('obj-active-flags', 'ACTIVE_FLAG_ACTIVE').label('active'),
    'obj-behavior-ptr': data('struct Object.behavior').hidden(),
    'obj-hitbox-radius': data('struct Object.hitboxRadius').label('hitbox radius'),
    'obj-hitbox-height': data('struct Object.hitboxHeight').label('hitbox height'),
    'obj-pos-x': data('struct Object.oPosX').label('pos x'),
    'obj-pos-y': data('struct Object.oPosY').label('pos y'),
    'obj-pos-z': data('struct Object.oPosZ').label('pos z'),
    'obj-vel-f': data('struct Object.oForwardVel').label('vel f'),
    'obj-vel-x': data('struct Object.oVelX').label('vel x'),
    'obj-vel-y': data('struct Object.oVelY').label('vel y'),
    'obj-vel-z': data('struct Object.oVelZ').label('vel z'),
  },
  VariableGroup('Surface'): {
    'surface-normal-x': data('struct Surface.normal.x').label('normal x'),
    'surface-normal-y': data('struct Surface.normal.y').label('normal y'),
    'surface-normal-z': data('struct Surface.normal.z').label('normal z'),
    'surface-vertex1-x': data('struct Surface.vertex1[0]').label('x1'),
    'surface-vertex1-y': data('struct Surface.vertex1[1]').label('y1'),
    'surface-vertex1-z': data('struct Surface.vertex1[2]').label('z1'),
    'surface-vertex2-x': data('struct Surface.vertex2[0]').label('x2'),
    'surface-vertex2-y': data('struct Surface.vertex2[1]').label('y2'),
    'surface-vertex2-z': data('struct Surface.vertex2[2]').label('z2'),
    'surface-vertex3-x': data('struct Surface.vertex3[0]').label('x3'),
    'surface-vertex3-y': data('struct Surface.vertex3[1]').label('y3'),
    'surface-vertex3-z': data('struct Surface.vertex3[2]').label('z3'),
  }
}
