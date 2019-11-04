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
  },
  VariableGroup('Misc'): {
    'global-timer': data('$state.gGlobalTimer').label('global timer'),
  },
  VariableGroup.all_objects(): {
    'obj-active-flags': data('$object.activeFlags').hidden(),
    'obj-active-flags-active': flag('obj-active-flags', 'ACTIVE_FLAG_ACTIVE').label('active'),
    'obj-behavior-ptr': data('$object.behaviorSeg').hidden(),
    'obj-hitbox-radius': data('$object.hitboxRadius').label('hitbox radius'),
    'obj-pos-x': data('$object.oPosX').label('pos x'),
    'obj-pos-y': data('$object.oPosY').label('pos y'),
    'obj-pos-z': data('$object.oPosZ').label('pos z'),
    'obj-vel-f': data('$object.oForwardVel').label('vel f'),
    'obj-vel-x': data('$object.oVelX').label('vel x'),
    'obj-vel-y': data('$object.oVelY').label('vel y'),
    'obj-vel-z': data('$object.oVelZ').label('vel z'),
  },
}
