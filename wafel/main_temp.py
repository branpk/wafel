from __future__ import annotations

from typing import *
from dataclasses import dataclass
import time

from ext_modules.core import Variable, Pipeline

import wafel.imgui as ig
from wafel.variable import VariableReader, VariableWriter, VariablePipeline
from wafel.variable_display import VariableDisplayer
from wafel.variable_format import Formatters, DecimalIntFormatter, VariableFormatter, \
  DataFormatters, EnumFormatter
from wafel.local_state import use_state_with, use_state
from wafel.frame_sheet import FrameSequence, FrameSheet, CellDragHandler
from wafel.util import *
from wafel.range_edit import RangeEditWriter
from wafel.window import open_window_and_run
from wafel.format_m64 import load_m64
from wafel.model import Model


class NoOpDragHandler:
  def drag(self, source: Variable, source_value: object, target_frame: int) -> None:
    pass

  def release(self) -> None:
    pass

  def highlight_range(self, variable: Variable) -> Optional[Tuple[range, ig.Color4f]]:
    return None


def test_frame_sheet(id: str) -> None:
  ig.push_id(id)

  def create_model() -> Model:
    metadata, edits = load_m64('test_files/22stars.m64')
    model = Model()
    model.load('jp', edits)
    return model

  def create_formatters(model: Model) -> DataFormatters:
    formatters = DataFormatters(model.pipeline)
    formatters[Variable('mario-action')] = EnumFormatter(model.pipeline.action_names())
    return formatters

  model = use_state_with('model', create_model).value
  formatters = use_state_with('formatters', lambda: create_formatters(model)).value

  def make_sheet() -> FrameSheet:
    sheet = FrameSheet(model, model.pipeline, NoOpDragHandler(), model, formatters)
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

  model.pipeline.balance_distribution(1/120)


def run():
  open_window_and_run(test_frame_sheet)

  # pipeline = Pipeline.load('lib/libsm64/sm64_jp.dll')
  # print(pipeline.field_offset('struct Surface.normal'))
  # print(pipeline.dump_layout())

  # model = Model()
  # metadata, edits = load_m64('test_files/22stars.m64')
  # model.load('jp', edits)

  # print(model.get(Variable('global-timer').with_frame(100)))
