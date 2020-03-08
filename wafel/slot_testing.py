from __future__ import annotations

from typing import *
import time
import random

import imgui as ig

from wafel.core.slot_manager import AbstractSlot, AbstractSlots, SlotManager
from wafel.local_state import use_state, use_state_with
import wafel.ui as ui


SLOWDOWN = 1


def amortized_sleep(duration: float, scale: float) -> None:
  if random.random() < 1/scale:
    time.sleep(duration * scale)


class TestState:
  def __init__(self, frame: int, slot: TestSlot) -> None:
    self.frame = frame
    self.slot = slot


class TestSlot(AbstractSlot):
  def __init__(self, based: bool) -> None:
    self.content = ('<uninit>', 0)

    self._frame: Optional[int] = None
    self._based = based
    self._owners: List[TestState] = []

  @property
  def frame(self) -> Optional[int]:
    return self._frame

  @frame.setter
  def frame(self, frame: Optional[int]) -> None:
    self._frame = frame

  @property
  def based(self) -> bool:
    return self._based

  @property
  def frozen(self) -> bool:
    return len(self._owners) > 0

  def __enter__(self) -> TestState:
    assert self.frame is not None
    owner = TestState(self.frame, self)
    self._owners.append(owner)
    return owner

  def __exit__(self, exc_type, exc_value, traceback) -> None:
    self._owners.pop()

  def permafreeze(self) -> None:
    assert self.frame is not None
    self._owners.append(TestState(self.frame, self))

  def __repr__(self) -> str:
    return f'Slot(based={self.based}, frame={self.frame}, frozen={self.frozen})'


class TestSlots(AbstractSlots[TestSlot]):
  def __init__(self, num_frames: int, capacity: int) -> None:
    self._base = TestSlot(True)
    self._base.frame = -1
    self._base.content = ('a', 0)

    self._non_base = [TestSlot(False) for _ in range(capacity - 1)]
    self._temp = self._non_base.pop()

    self._num_frames = num_frames

    self.edits: Dict[int, str] = {}

    self.copies = 0
    self.updates = 0

  @property
  def temp(self) -> TestSlot:
    return self._temp

  @property
  def base(self) -> TestSlot:
    return self._base

  @property
  def non_base(self) -> List[TestSlot]:
    return self._non_base

  def copy(self, dst: TestSlot, src: TestSlot) -> None:
    assert not dst.frozen
    if dst is not src:
      amortized_sleep(0.001 * SLOWDOWN, 10)
      dst.content = src.content
      dst.frame = src.frame
      self.copies += 1

  def num_frames(self) -> int:
    return self._num_frames

  def execute_frame(self) -> None:
    assert self.base.frame is not None
    assert not self.base.frozen

    with self.base as state:
      if self.base.frame != -1:
        amortized_sleep(0.0001 * SLOWDOWN, 20)
        label, count = self.base.content
        self.base.content = label, count + 1
        self.updates += 1

      self.base.frame += 1
      state.frame += 1

      if state.frame in self.edits:
        state.slot.content = self.edits[state.frame], 0

  def edit(self, frame: int, label: str) -> None:
    self.edits[frame] = label
    for slot in self.where():
      if slot.frame is not None and slot.frame >= frame:
        slot.frame = None

  def __repr__(self) -> str:
    return str(self.where())


def test_timeline_algorithm(id: str) -> None:
  ig.push_id(id)

  slots = use_state_with('slots', lambda: TestSlots(7000, 10)).value
  slot_manager = use_state_with('slot-manager', lambda: SlotManager(slots)).value

  slots.copies = 0
  slots.updates = 0

  cur_frame = use_state('cur-frame', 0)
  slot_manager.set_hotspot('cur-frame', cur_frame.value)

  ig.get_io().key_repeat_rate = 1/30
  if not ig.get_io().want_capture_keyboard:
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_DOWN_ARROW)) or \
        ig.is_key_pressed(ig.get_key_index(ig.KEY_RIGHT_ARROW)):
      cur_frame.value += 1
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_UP_ARROW)) or \
        ig.is_key_pressed(ig.get_key_index(ig.KEY_LEFT_ARROW)):
      cur_frame.value -= 1
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_PAGE_DOWN)):
      cur_frame.value += 5
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_PAGE_UP)):
      cur_frame.value -= 5

  with slot_manager.request_frame(cur_frame.value) as state:
    cur_value = state.slot.content
  ig.text(f'{cur_frame.value}: {cur_value}')

  new_frame = ui.render_frame_slider(
    'frame-slider',
    cur_frame.value,
    slots.num_frames(),
    slot_manager.get_loaded_frames(),
  )
  if new_frame is not None:
    cur_frame.value = new_frame.value

  _, input = ig.input_text('##edit', cur_value[0], 32)
  if cur_value[0] != input:
    slots.edit(cur_frame.value, input)

  try:
    x, y = map(int, cur_value[0].split(','))
  except:
    x, y = 0, 0
  new_xy = ui.render_joystick_control('joystick', x / 256, y / 256)
  if new_xy is not None:
    x, y = int(new_xy[0] * 256), int(new_xy[1] * 256)
    slots.edit(cur_frame.value, f'{x},{y}')

  values = []
  # for i in range(-20, 30):
  for i in range(0, 50):
    frame = cur_frame.value + i
    if frame in range(slots.num_frames()):
      with slot_manager.request_frame(frame) as state:
        values.append(state.slot.content)
  ig.text(str(values))

  slot_manager.balance_distribution(1/120)

  last_fps_time = use_state_with('last-fps-time', lambda: time.time())
  frame_count = use_state('frame-count', 0)
  fps = use_state('fps', 0)

  frame_count.value += 1
  if time.time() > last_fps_time.value + 1:
    last_fps_time.value = time.time()
    fps.value = frame_count.value
    frame_count.value = 0

  ig.text(f'fps: {fps.value}')
  ig.text(f'copies: {slots.copies}')
  ig.text(f'updates: {slots.updates}')

  ig.pop_id()
