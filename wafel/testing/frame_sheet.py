from __future__ import annotations

from typing import *
from dataclasses import dataclass
import time

import wafel.imgui as ig
from wafel.variable import VariableReader, VariableWriter, Variable, VariablePipeline
from wafel.variable_display import VariableDisplayer
from wafel.variable_format import Formatters, DecimalIntFormatter, VariableFormatter
from wafel.local_state import use_state_with, use_state
from wafel.frame_sheet import FrameSequence, FrameSheet, CellDragHandler
from wafel.util import *
from wafel.range_edit import RangeEditWriter


class Accessor(VariableReader, VariableWriter):
  def __init__(self) -> None:
    self.edits: Dict[Variable, object] = {}

  def read(self, variable: Variable) -> object:
    return self.edits.get(variable, 0)

  def write(self, variable: Variable, value: object) -> None:
    self.edits[variable] = value

  def reset(self, variable: Variable) -> None:
    if variable in self.edits:
      del self.edits[variable]


class Model(VariableDisplayer, Formatters, FrameSequence):
  def __init__(self) -> None:
    self._selected_frame = 0
    self._max_frame = 1000
    accessor = Accessor()
    self._range_edit_writer = RangeEditWriter(accessor)
    self._pipeline = VariablePipeline(self._range_edit_writer, accessor)
    for var in ['stick', 'vel', 'pos']:
      for f in range(100):
        self._pipeline.write(Variable(var, frame=f), 10 + f)
    for i in range(10):
      for f in range(100):
        self._pipeline.write(Variable('junk ' + str(i), frame=f), 10 + f)

  def label(self, variable: Variable) -> str:
    return variable.name

  def column_header(self, variable: Variable) -> str:
    return variable.name

  def __getitem__(self, variable: Variable) -> VariableFormatter:
    return DecimalIntFormatter()

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
    pass


def test_frame_sheet(id: str) -> None:
  ig.push_id(id)
  model = use_state_with('model', lambda: Model()).value

  def make_sheet() -> FrameSheet:
    sheet = FrameSheet(model, model._pipeline, model._range_edit_writer, model, model)
    sheet.append_variable(Variable('stick'))
    sheet.append_variable(Variable('vel'))
    sheet.append_variable(Variable('pos'))
    for i in range(20):
      sheet.append_variable(Variable('junk ' + str(i)))
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
