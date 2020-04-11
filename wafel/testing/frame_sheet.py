from typing import *
from dataclasses import dataclass

import wafel.imgui as ig
from wafel.variable import VariableAccessor, Variable
from wafel.variable_display import VariableDisplayer
from wafel.variable_format import Formatters, DecimalIntFormatter, VariableFormatter
from wafel.local_state import use_state_with, use_state
from wafel.frame_sheet import FrameSequence, FrameSheet
from wafel.util import *


@dataclass
class EditRange:
  frame_start: int
  frame_stop: int
  variable: Variable
  value: object

  def __contains__(self, frame: int) -> bool:
    return frame in range(self.frame_start, self.frame_stop)


class Edits:
  def __init__(self) -> None:
    self.ranges: List[EditRange] = []

  def get_range(self, variable: Variable) -> Optional[EditRange]:
    frame = dcast(int, variable.args['frame'])
    for range in self.ranges:
      if frame in range and range.variable.at(frame=frame) == variable:
        return range
    return None

  def get_edit(self, variable: Variable) -> Maybe[object]:
    range = self.get_range(variable)
    if range is None:
      return None
    else:
      return Just(range.value)

  def set_edit(self, variable: Variable, value: object) -> None:
    range = self.get_range(variable)
    if range is None:
      frame = dcast(int, variable.args['frame'])
      self.ranges.append(EditRange(frame, frame + 1, variable.without('frame'), value))
    else:
      range.value = value


class Model(VariableAccessor, VariableDisplayer, Formatters, FrameSequence):
  def __init__(self) -> None:
    self._selected_frame = 0
    self._max_frame = 1000
    self._edits = Edits()

  def get(self, variable: Variable) -> object:
    edit = self._edits.get_edit(variable)
    if edit is not None:
      return edit.value
    frame = dcast(int, variable.args['frame'])
    if variable.name == 'vel':
      if frame == 0:
        return 0
      else:
        prev_vel = dcast(int, self.get(Variable('vel', frame=frame - 1)))
        stick = dcast(int, self.get(Variable('stick', frame=frame)))
        return prev_vel + stick
    else:
      return 0

  def set(self, variable: Variable, value: object) -> None:
    self._edits.set_edit(variable, value)

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
  model = use_state_with('model', Model).value

  def make_sheet() -> FrameSheet:
    sheet = FrameSheet(*([model] * 4))
    sheet.append_variable(Variable('stick'))
    sheet.append_variable(Variable('vel'))
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
