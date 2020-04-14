from __future__ import annotations

from typing import *
from dataclasses import dataclass
import textwrap
import time

import wafel.imgui as ig
from wafel.variable import Variable, VariableWriter, VariableReader
from wafel.util import *


RANGE_COLORS = [
  (0.4, 0.9, 0.0, 0.3),
  (0.6, 0.4, 0.0, 0.3),
  (0.4, 0.9, 0.5, 0.3),
  (0.5, 0.5, 0.5, 0.3),
  (0.2, 0.6, 0.0, 0.3),
  (0.7, 0.7, 0.3, 0.3),
  (0.3, 0.3, 0.7, 0.3),
]


@dataclass(frozen=True)
class EditRange:
  variable: Variable
  frames: range
  value: object

  def __contains__(self, variable: Variable) -> bool:
    frame = dcast(int, variable.args['frame'])
    return frame in self.frames and self.variable.at(frame=frame) == variable

  def intersects(self, other: EditRange) -> bool:
    if self.variable != other.variable:
      return False
    return self.frames.stop > other.frames.start and other.frames.stop > self.frames.start

  def __str__(self) -> str:
    return f'{self.variable}[{self.frames.start}, {self.frames.stop})={self.value}'


@dataclass(frozen=True)
class OpAtom:
  targets: Tuple[EditRange, ...]
  results: Tuple[EditRange, ...]

  def inverse(self) -> EditRangeOp:
    return OpAtom(self.results, self.targets)

  def __str__(self) -> str:
    targets = ', '.join(map(str, self.targets))
    results = ', '.join(map(str, self.results))
    return targets + ' -> ' + results

@dataclass(frozen=True)
class OpSequence:
  ops: Tuple[EditRangeOp, ...]

  def inverse(self) -> OpSequence:
    return OpSequence(tuple(op.inverse() for op in reversed(self.ops)))

  def __str__(self) -> str:
    children = '\n'.join(map(str, self.ops))
    return 'seq:\n' + textwrap.indent(children, '  ')

EditRangeOp = Union[OpAtom, OpSequence]


class EditRangesImpl:
  def __init__(self, writer: VariableWriter) -> None:
    self._writer = writer
    self._by_variable: Dict[Variable, EditRange] = {}
    self._colors: Dict[EditRange, ig.Color4f] = {}
    self._color_index: int = 0

  def get(self, variable: Variable) -> Optional[EditRange]:
    return self._by_variable.get(variable)

  def get_color(self, edit_range: EditRange) -> ig.Color4f:
    return self._colors[edit_range]

  def intersecting(self, variable: Variable, frames: range) -> Iterable[EditRange]:
    result = set()
    for frame in frames:
      other = self._by_variable.get(variable.at(frame=frame))
      if other is not None:
        result.add(other)
    return result

  def _remove(self, edit_range: EditRange) -> None:
    for frame in edit_range.frames:
      variable = edit_range.variable.at(frame=frame)
      del self._by_variable[variable]
      self._writer.reset(variable)

  def _add(self, edit_range: EditRange) -> None:
    for frame in edit_range.frames:
      variable = edit_range.variable.at(frame=frame)
      assert variable not in self._by_variable
      self._by_variable[variable] = edit_range
      self._writer.write(variable, edit_range.value)

  def apply(self, op: EditRangeOp) -> None:
    if isinstance(op, OpAtom):
      color_queue = []

      for target in op.targets:
        color_queue.append(self._colors.pop(target))
        self._remove(target)

      for result in op.results:
        if len(color_queue) == 0:
          color = RANGE_COLORS[self._color_index]
          self._color_index = (self._color_index + 1) % len(RANGE_COLORS)
        else:
          color = color_queue.pop(0)
        self._colors[result] = color
        self._add(result)

      self._color_index = (self._color_index - len(color_queue)) % len(RANGE_COLORS)

    elif isinstance(op, OpSequence):
      for child in op.ops:
        self.apply(child)

    else:
      raise NotImplementedError(op)

  def _insert_frame_into_range(self, edit_range: EditRange, frame: int) -> EditRange:
    frames = edit_range.frames
    if frames.start >= frame:
      frames = range(frames.start + 1, frames.stop + 1)
    elif frames.stop - 1 >= frame:
      frames = range(frames.start, frames.stop + 1)
    return EditRange(edit_range.variable, frames, edit_range.value)

  def insert_frame(self, frame: int) -> None:
    targets = tuple(set(self._by_variable.values()))
    results = tuple(map(lambda r: self._insert_frame_into_range(r, frame), targets))
    op = OpAtom(targets, results)
    self.apply(op)

  def _delete_frame_from_range(self, edit_range: EditRange, frame: int) -> EditRange:
    frames = edit_range.frames
    if frames.start > frame:
      frames = range(frames.start - 1, frames.stop - 1)
    elif frames.stop - 1 >= frame:
      frames = range(frames.start, frames.stop - 1)
    return EditRange(edit_range.variable, frames, edit_range.value)

  def delete_frame(self, frame: int) -> None:
    delete_ranges = []
    for edit_range in self._by_variable.values():
      if edit_range.frames == range(frame, frame + 1):
        delete_ranges.append(edit_range)
    for edit_range in delete_ranges:
      self.apply(OpAtom((edit_range,), ()))

    targets = tuple(set(self._by_variable.values()))
    results = tuple(map(lambda r: self._delete_frame_from_range(r, frame), targets))
    op = OpAtom(targets, results)
    self.apply(op)


class EditRanges:
  def __init__(self, writer: VariableWriter) -> None:
    self._ranges = EditRangesImpl(writer)
    self._tentative_op: Optional[EditRangeOp] = None

  def apply(self, op: EditRangeOp) -> None:
    self.revert_tentative()
    self._ranges.apply(op)

  def get_range(self, variable: Variable) -> Optional[EditRange]:
    return self._ranges.get(variable)

  def get_color(self, edit_range: EditRange) -> ig.Color4f:
    return self._ranges.get_color(edit_range)

  def set_value(self, variable: Variable, value: object) -> None:
    self.revert_tentative()
    edit_range = self.get_range(variable)
    if edit_range is None:
      frame = dcast(int, variable.args['frame'])
      op = self.op_insert(variable.without('frame'), range(frame, frame + 1), value)
    else:
      op = self.op_set_value(edit_range, value)
    self.apply(op)

  def apply_tentative(self, op: EditRangeOp) -> None:
    assert self._tentative_op is None
    self.apply(op)
    self._tentative_op = op

  def revert_tentative(self) -> None:
    op = self._tentative_op
    if op is not None:
      self._tentative_op = None
      self.apply(op.inverse())

  def commit_tentative(self) -> None:
    self._tentative_op = None

  def insert_frame(self, frame: int) -> None:
    self.revert_tentative()
    self._ranges.insert_frame(frame)

  def delete_frame(self, frame: int) -> None:
    self.revert_tentative()
    self._ranges.delete_frame(frame)

  def op_no_op(self) -> EditRangeOp:
    return OpSequence(())

  def op_set_value(self, edit_range: EditRange, value: object) -> EditRangeOp:
    return OpAtom((edit_range,), (EditRange(edit_range.variable, edit_range.frames, value),))

  def op_set_frames(self, edit_range: EditRange, to_frames: range) -> EditRangeOp:
    assert len(to_frames) > 0
    return OpAtom((edit_range,), (EditRange(edit_range.variable, to_frames, edit_range.value),))

  def op_delete(self, edit_range: EditRange) -> EditRangeOp:
    return OpAtom((edit_range,), ())

  def op_insert_range(self, edit_range: EditRange) -> EditRangeOp:
    assert len(edit_range.frames) > 0
    return OpAtom((), (edit_range,))

  def op_insert(self, variable: Variable, frames: range, value: object) -> EditRangeOp:
    return self.op_insert_range(EditRange(variable, frames, value))

  def op_split(self, edit_range: EditRange, *frame_ranges: range) -> EditRangeOp:
    result = []
    for frames in frame_ranges:
      if len(frames) > 0:
        result.append(EditRange(edit_range.variable, frames, edit_range.value))
    return OpAtom((edit_range,), tuple(result))

  # There is no op_sequence since op_grow depends on self.ranges, so sequencing
  # it isn't guaranteed to result in a valid operation

  def op_shrink(self, edit_range: EditRange, to_frames: range) -> EditRangeOp:
    if len(to_frames) > 0:
      assert to_frames.start >= edit_range.frames.start
      assert to_frames.stop <= edit_range.frames.stop
      return self.op_set_frames(edit_range, to_frames)
    else:
      return self.op_delete(edit_range)

  def op_resize(self, edit_range: EditRange, frames: range) -> EditRangeOp:
    assert self._tentative_op is None
    ops = []
    for other in self._ranges.intersecting(edit_range.variable, frames):
      if other != edit_range:
        new_frames = other.frames
        if new_frames.stop in frames:
          new_frames = range(new_frames.start, frames.start)
        if new_frames.start in frames:
          new_frames = range(frames.stop, new_frames.stop)
        if new_frames != other.frames:
          ops.append(self.op_shrink(other, new_frames))
    if len(frames) > 0:
      ops.append(self.op_set_frames(edit_range, frames))
    else:
      ops.append(self.op_delete(edit_range))
    return OpSequence(tuple(ops))

  def op_include(self, edit_range: EditRange, frame: int) -> EditRangeOp:
    if frame < edit_range.frames.start:
      return self.op_resize(edit_range, range(frame, edit_range.frames.stop))
    if frame >= edit_range.frames.stop:
      return self.op_resize(edit_range, range(edit_range.frames.start, frame + 1))
    return self.op_no_op()

  def op_split_upward(self, edit_range: EditRange, gap: range) -> EditRangeOp:
    # assert gap.stop - 1 in edit_range.frames
    return self.op_split(
      edit_range,
      range(edit_range.frames.start, gap.start),
      range(gap.stop, edit_range.frames.stop),
    )

  def op_split_downward(self, edit_range: EditRange, gap: range) -> EditRangeOp:
    # assert gap.start in edit_range.frames
    return self.op_split(
      edit_range,
      range(gap.stop, edit_range.frames.stop),
      range(edit_range.frames.start, gap.start),
    )

  def op_insert_then_resize(
    self, variable: Variable, value: object, to_frames: range
  ) -> EditRangeOp:
    assert self._tentative_op is None
    assert self._ranges.get(variable) is None
    frame = variable.args['frame']
    edit_range = EditRange(variable.without('frame'), range(frame, frame + 1), value)
    return OpSequence((
      self.op_insert_range(edit_range),
      self.op_resize(edit_range, to_frames),
    ))


class RangeEditWriter(VariableWriter):
  def __init__(
    self,
    writer: VariableWriter,
    highlight_single: Callable[[Variable], bool] = lambda _: True,
  ) -> None:
    self._ranges = EditRanges(writer)
    self._highlight_single = highlight_single
    self._drag_start_time: Optional[float] = None

  def write(self, variable: Variable, value: object) -> None:
    self._ranges.set_value(variable, value)

  def edited(self, variable: Variable) -> bool:
    return self._ranges.get_range(variable) is not None

  def reset(self, variable: Variable) -> None:
    self._ranges.revert_tentative()
    edit_range = self._ranges.get_range(variable)
    if edit_range is not None:
      frame = dcast(int, variable.args['frame'])
      self._ranges.apply(self._ranges.op_split_upward(edit_range, range(frame, frame + 1)))

  def drag(self, source: Variable, source_value: object, target_frame: int) -> None:
    if self._drag_start_time is None:
      self._drag_start_time = time.time()
    self._ranges.revert_tentative()

    source_frame = dcast(int, source.args['frame'])
    edit_range = self._ranges.get_range(source)
    op: Optional[EditRangeOp] = None

    if edit_range is None:
      if target_frame > source_frame:
        op = self._ranges.op_insert_then_resize(
          source, source_value, range(source_frame, target_frame + 1)
        )
      elif target_frame < source_frame:
        op = self._ranges.op_insert_then_resize(
          source, source_value, range(target_frame, source_frame + 1)
        )
    elif edit_range.frames == range(source_frame, source_frame + 1):
      op = self._ranges.op_include(edit_range, target_frame)
    elif edit_range.frames.start == source_frame:
      op = self._ranges.op_resize(
        edit_range, range(target_frame, edit_range.frames.stop)
      )
    elif edit_range.frames.stop - 1 == source_frame:
      op = self._ranges.op_resize(
        edit_range, range(edit_range.frames.start, target_frame + 1)
      )
    else:
      if target_frame > source_frame:
        op = self._ranges.op_split_upward(
          edit_range, range(source_frame, target_frame)
        )
      elif target_frame < source_frame:
        op = self._ranges.op_split_downward(
          edit_range, range(target_frame + 1, source_frame + 1)
        )

    if op is not None:
      self._ranges.apply_tentative(op)

  def release(self) -> None:
    # Prevent accidental drags
    if self._drag_start_time is not None and time.time() - self._drag_start_time > 0.2:
      self._ranges.commit_tentative()
    else:
      self._ranges.revert_tentative()
    self._drag_start_time = None

  def highlight_range(self, variable: Variable) -> Optional[Tuple[range, ig.Color4f]]:
    edit_range = self._ranges.get_range(variable)
    if edit_range is not None:
      if len(edit_range.frames) == 1 and not self._highlight_single(variable):
        return None
      return edit_range.frames, self._ranges.get_color(edit_range)
    else:
      return None

  def insert_frame(self, frame: int) -> None:
    self._ranges.insert_frame(frame)

  def delete_frame(self, frame: int) -> None:
    self._ranges.delete_frame(frame)


__all__ = [
  'RangeEditWriter',
]
