from typing import *
from dataclasses import dataclass
from copy import copy

import wafel.imgui as ig
from wafel.variable import VariableAccessor, Variable
from wafel.variable_display import VariableDisplayer
from wafel.variable_format import Formatters, DecimalIntFormatter, VariableFormatter
from wafel.local_state import use_state_with, use_state
from wafel.frame_sheet import FrameSequence, FrameSheet, CellDragHandler
from wafel.util import *


@dataclass
class EditRange:
  frame_start: int
  frame_stop: int
  variable: Variable
  value: object

  def __contains__(self, variable: Variable) -> bool:
    frame = dcast(int, variable.args['frame'])
    return frame in self.frame_range and self.variable.at(frame=frame) == variable

  @property
  def frame_range(self) -> range:
    return range(self.frame_start, self.frame_stop)


class Edits:
  def __init__(self) -> None:
    self.ranges: List[EditRange] = []

  def get_range(self, variable: Variable) -> Optional[EditRange]:
    frame = dcast(int, variable.args['frame'])
    for range in self.ranges:
      if variable in range:
        return range
    return None

  def get_or_create_range(self, variable: Variable) -> EditRange:
    range = self.get_range(variable)
    if range is None:
      frame = dcast(int, variable.args['frame'])
      range = EditRange(frame, frame + 1, variable.without('frame'), None)
      self.ranges.append(range)
    return range

  def get_edit(self, variable: Variable) -> Maybe[object]:
    range = self.get_range(variable)
    if range is None:
      return None
    else:
      return Just(range.value)

  def set_edit(self, variable: Variable, value: object) -> None:
    self.get_or_create_range(variable).value = value

  def add_range(self, range: EditRange) -> None:
    for other in self.ranges:
      if other.variable == range.variable:
        if other.frame_stop in range.frame_range:
          other.frame_stop = range.frame_start
        if other.frame_start in range.frame_range:
          other.frame_start = range.frame_stop
    self.ranges = [r for r in self.ranges if r.frame_start < r.frame_stop]
    self.ranges.append(range)


class Model(VariableAccessor, VariableDisplayer, Formatters, FrameSequence, CellDragHandler):
  def __init__(self) -> None:
    self._selected_frame = 0
    self._max_frame = 1000
    self._edits = Edits()
    self._edits.set_edit(Variable('stick', frame=5), 10)
    self._drag_range: Optional[EditRange] = None

  def get(self, variable: Variable) -> object:
    frame = dcast(int, variable.args['frame'])
    edit: Maybe[object]
    if self._drag_range is not None and variable in self._drag_range:
      edit = Just(self._drag_range.value)
    else:
      edit = self._edits.get_edit(variable)
    if edit is not None:
      return edit.value
    if variable.name == 'vel':
      if frame == -1:
        return 0
      else:
        prev_vel = dcast(int, self.get(Variable('vel', frame=frame - 1)))
        stick = dcast(int, self.get(Variable('stick', frame=frame)))
        return prev_vel + stick
    elif variable.name == 'pos':
      if frame == -1:
        return 0
      else:
        prev_pos = dcast(int, self.get(Variable('pos', frame=frame - 1)))
        vel = dcast(int, self.get(Variable('vel', frame=frame)))
        return prev_pos + vel
    else:
      return 0

  def set(self, variable: Variable, value: object) -> None:
    self._edits.set_edit(variable, value)

  def edited(self, variable: Variable) -> bool:
    if self._drag_range is not None and variable in self._drag_range:
      return True
    return self._edits.get_range(variable) is not None

  def drag(self, source: Variable, target_frame: int) -> None:
    source_frame = dcast(int, source.args['frame'])
    if self._drag_range is None:
      range = self._edits.get_range(source)
      if range is None:
        range = EditRange(source_frame, source_frame + 1, source.without('frame'), self.get(source))
      else:
        range = copy(range)
      self._drag_range = range

    self._drag_range.frame_start = min(source_frame, target_frame)
    self._drag_range.frame_stop = max(source_frame, target_frame) + 1

  def release(self) -> None:
    if self._drag_range is not None:
      self._edits.add_range(self._drag_range)
    self._drag_range = None

  def highlight_range(self, variable: Variable) -> Optional[range]:
    if self._drag_range is not None and variable in self._drag_range:
      return self._drag_range.frame_range
    range = self._edits.get_range(variable)
    if range is not None:
      return range.frame_range
    else:
      return None

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
    sheet = FrameSheet(*([model] * 5))
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

  ig.pop_id()
