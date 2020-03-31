from typing import *

import ext_modules.util as c_util

from wafel.core import Slot, Game, DataPath, Timeline
from wafel.util import *


def _get_event_variant(event_type: str) -> str:
  variant = event_type.lower()
  if variant.startswith('flt_'):
    variant = variant[len('flt_'):]
  parts = variant.split('_')
  variant = parts[0] + ''.join(map(str.capitalize, parts[1:]))
  return variant

def get_frame_log(timeline: Timeline, frame: int) -> List[Dict[str, Any]]:
  event_types: Dict[int, str] = {
    constant['value']: constant_name
      for constant_name, constant in timeline.game.memory.data_spec['constants'].items()
        if constant['source'] == 'enum' and constant['enum_name'] == 'FrameLogEventType'
  }

  events: List[Dict[str, object]] = []

  log_length = dcast(int, timeline.get(frame, 'gFrameLogLength'))
  for i in range(log_length):
    event_type_value = dcast(int, timeline.get(frame, f'gFrameLog[{i}].type'))
    event_type = event_types[event_type_value]
    variant_name = _get_event_variant(event_type)
    event_data = dcast(dict, timeline.get(frame, f'gFrameLog[{i}].__anon.{variant_name}'))

    event: Dict[str, object] = { 'type': event_type }
    event.update(event_data)
    events.append(event)

  return events


def intended_to_raw_impl(
  game: Game, slot: Slot, int_yaw: int, int_mag: float, relative_to: int
) -> Tuple[int, int]:
  # TODO: This doesn't account for rotation from platform displacement
  face_yaw = dcast(int, game.path('gMarioState[].faceAngle[1]').get(slot))
  camera_yaw = dcast(int, game.path('gMarioState[].area[].camera[].yaw').get(slot))
  squish_timer = dcast(int, game.path('gMarioState[].squishTimer').get(slot))

  stick_x, stick_y = c_util.stick_intended_to_raw(
    trunc_signed(int_yaw, 16),
    min(max(int_mag, 0.0), 32.0),
    face_yaw,
    camera_yaw,
    squish_timer != 0,
    relative_to,
  )

  return cast(Tuple[int, int], (stick_x, stick_y))


def intended_to_raw(
  timeline: Timeline, frame: int, int_yaw: int, int_mag: float, relative_to: int
) -> Tuple[int, int]:
  with timeline.request_base(frame) as slot:
    return intended_to_raw_impl(timeline.game, slot, int_yaw, int_mag, relative_to)


__all__ = [
  'get_frame_log',
  'intended_to_raw_impl',
  'intended_to_raw',
]
