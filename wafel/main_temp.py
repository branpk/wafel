from __future__ import annotations

from typing import *
from dataclasses import dataclass
import time

from ext_modules.core import Variable, Pipeline

import wafel.imgui as ig
from wafel.variable import VariableReader, VariableWriter, VariablePipeline
from wafel.variable_display import VariableDisplayer
from wafel.variable_format import Formatters, DecimalIntFormatter, VariableFormatter, DataFormatters
from wafel.local_state import use_state_with, use_state
from wafel.frame_sheet import FrameSequence, FrameSheet, CellDragHandler
from wafel.util import *
from wafel.range_edit import RangeEditWriter
from wafel.window import open_window_and_run
from wafel.format_m64 import load_m64


class NoOpDragHandler:
  def drag(self, source: Variable, source_value: object, target_frame: int) -> None:
    pass

  def release(self) -> None:
    pass

  def highlight_range(self, variable: Variable) -> Optional[Tuple[range, ig.Color4f]]:
    return None


class Model(VariableDisplayer, FrameSequence):
  def __init__(self) -> None:
    self._selected_frame = 0
    self._max_frame = 1000
    self._pipeline = Pipeline.load('lib/libsm64/sm64_us.dll')

    metadata, edits = load_m64('test_files/22stars.m64')
    for variable, value in edits.items():
      self._pipeline.write(variable, value)

  def label(self, variable: Variable) -> str:
    return assert_not_none(self._pipeline.label(variable))

  def column_header(self, variable: Variable) -> str:
    if variable.object is not None:
      if variable.object_behavior is None:
        return str(variable.object) + '\n' + self.label(variable)
      else:
        behavior_name = '???' # TODO
        return str(variable.object) + ' - ' + behavior_name + '\n' + self.label(variable)

    elif variable.surface is not None:
      return f'Surface {variable.surface}\n{self.label(variable)}'

    else:
      return self.label(variable)

  @property
  def selected_frame(self) -> int:
    return self._selected_frame

  def set_selected_frame(self, frame: int) -> None:
    self._selected_frame = frame

  @property
  def max_frame(self) -> int:
    return self._max_frame

  def extend_to_frame(self, frame: int) -> None:
    self._max_frame = max(self._max_frame, frame)

  def insert_frame(self, frame: int) -> None:
    pass

  def delete_frame(self, frame: int) -> None:
    pass

  def set_hotspot(self, name: str, frame: int) -> None:
    self._pipeline.set_hotspot(name, frame)


def test_frame_sheet(id: str) -> None:
  ig.push_id(id)
  model = use_state_with('model', lambda: Model()).value

  def make_sheet() -> FrameSheet:
    sheet = FrameSheet(model, model._pipeline, NoOpDragHandler(), model, DataFormatters(model._pipeline))
    for name in [
          'input-button-s',
          'input-button-a',
          'input-button-b',
          'input-button-z',
          'mario-action',
          'mario-vel-f',
        ]:
      sheet.append_variable(Variable(name))
    return sheet
  sheet = use_state_with('sheet', make_sheet).value

  ig.set_next_window_content_size(sheet.get_content_width(), 0)
  ig.begin_child(
    'frame-sheet',
    height = ig.get_window_height() * 0.95,
    flags = ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
  )
  sheet.render()
  ig.end_child()

  last_fps_time = use_state_with('last-fps-time', lambda: time.time())
  frame_count = use_state('frame-count', 0)
  fps = use_state('fps', 0.0)

  frame_count.value += 1
  if time.time() > last_fps_time.value + 1:
    fps.value = frame_count.value / (time.time() - last_fps_time.value)
    last_fps_time.value = time.time()
    frame_count.value = 0
    print(f'mspf: {int(1000 / fps.value * 10) / 10} ({int(fps.value)} fps)')

  ig.pop_id()

  model._pipeline.balance_distribution(1/120)


def run():
  open_window_and_run(test_frame_sheet)
