from __future__ import annotations

from typing import *
import time
import random

import wafel.imgui as ig
from wafel.core.slot_manager import SlotManager, SlotAllocator, Slot
from wafel.local_state import use_state, use_state_with
import wafel.ui as ui


SLOWDOWN = 1


def amortized_sleep(duration: float, scale: float) -> None:
  if random.random() < 1/scale:
    time.sleep(duration * scale)


class TestSlot(Slot):
  def __init__(self, based: bool) -> None:
    self.based = based
    self.content = ('<uninit>', 0)


class TestSlotAllocator(SlotAllocator):
  def __init__(self) -> None:
    self._base = TestSlot(True)
    self._base.content = ('a', 0)

    self.edits: Dict[int, str] = {}

    self.copies = 0
    self.updates = 0

  @property
  def base_slot(self) -> TestSlot:
    return self._base

  def alloc_slot(self) -> TestSlot:
    return TestSlot(False)

  def dealloc_slot(self, slot: Slot) -> None:
    pass

  def copy_slot(self, dst: Slot, src: Slot) -> None:
    assert isinstance(dst, TestSlot)
    assert isinstance(src, TestSlot)
    if dst is not src:
      amortized_sleep(0.001 * SLOWDOWN, 10)
      dst.content = src.content
      self.copies += 1

  def run_frame(self, frame: int) -> None:
    if frame != -1:
      amortized_sleep(0.0001 * SLOWDOWN, 20)
      label, count = self._base.content
      self._base.content = label, count + 1
      self.updates += 1
    frame += 1
    if frame in self.edits:
      self._base.content = self.edits[frame], 0

  def edit(self, frame: int, label: str) -> None:
    self.edits[frame] = label


def test_timeline_algorithm(id: str) -> None:
  ig.push_id(id)

  slots = use_state_with('slots', lambda: TestSlotAllocator()).value
  slot_manager = use_state_with('slot-manager', lambda: SlotManager(slots, slots.run_frame, 10)).value

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

  with slot_manager.request_frame(cur_frame.value) as slot:
    cur_value = cast(TestSlot, slot).content
  ig.text(f'{cur_frame.value}: {cur_value}')

  new_frame = ui.render_frame_slider(
    'frame-slider',
    cur_frame.value,
    7000,
    slot_manager.get_loaded_frames(),
  )
  if new_frame is not None:
    cur_frame.value = new_frame.value

  _, input = ig.input_text('##edit', cur_value[0], 32)
  if cur_value[0] != input:
    slots.edit(cur_frame.value, input)
    slot_manager.invalidate(cur_frame.value)

  try:
    x, y = map(int, cur_value[0].split(','))
  except:
    x, y = 0, 0
  new_xy = ui.render_joystick_control('joystick', x / 256, y / 256)
  if new_xy is not None:
    x, y = int(new_xy[0] * 256), int(new_xy[1] * 256)
    slots.edit(cur_frame.value, f'{x},{y}')
    slot_manager.invalidate(cur_frame.value)

  values = []
  # for i in range(-20, 30):
  for i in range(0, 50):
    frame = cur_frame.value + i
    if frame >= 0:
      with slot_manager.request_frame(frame) as slot:
        values.append(cast(TestSlot, slot).content)
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
