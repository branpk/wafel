import sys
from typing import *
from abc import abstractmethod
from dataclasses import dataclass, field

from ext_modules.core import Variable

import wafel.imgui as ig
from wafel.variable import VariablePipeline
from wafel.variable_display import VariableDisplayer
from wafel.variable_format import Formatters, EmptyFormatter, VariableFormatter
import wafel.ui as ui
from wafel.util import *


class FrameSequence(Protocol):
  @property
  @abstractmethod
  def selected_frame(self) -> int: ...

  @abstractmethod
  def set_selected_frame(self, frame: int) -> None: ...

  @property
  @abstractmethod
  def max_frame(self) -> int: ...

  @abstractmethod
  def extend_to_frame(self, frame: int) -> None: ...

  @abstractmethod
  def insert_frame(self, frame: int) -> None: ...

  @abstractmethod
  def delete_frame(self, frame: int) -> None: ...

  @abstractmethod
  def set_hotspot(self, name: str, frame: int) -> None: ...


class CellDragHandler(Protocol):
  @abstractmethod
  def drag(self, source: Variable, source_value: object, target_frame: int) -> None: ...

  @abstractmethod
  def release(self) -> None: ...

  @abstractmethod
  def highlight_range(self, variable: Variable) -> Optional[Tuple[range, ig.Color4f]]: ...


@dataclass(unsafe_hash=True)
class FrameSheetColumn:
  variable: Variable
  width: int = field(default=100, hash=False, compare=False)


class FrameSheet:

  def __init__(
    self,
    sequence: FrameSequence,
    pipeline: VariablePipeline,
    drag_handler: CellDragHandler,
    displayer: VariableDisplayer,
    formatters: Formatters,
  ) -> None:
    super().__init__()
    self.sequence = sequence
    self.pipeline = pipeline
    self.drag_handler = drag_handler
    self.displayer = displayer
    self.formatters = formatters

    self.columns: List[FrameSheetColumn] = []
    self.next_columns: List[FrameSheetColumn] = []

    self.row_height = 30
    self.frame_column_width = 60

    self.drag_source: Optional[Variable] = None
    self.drag_target: Optional[int] = None

    self.prev_selected_frame: Optional[int] = None
    self.scroll_delta = 0.0


  def _insert_variable(self, index: int, variable: Variable) -> None:
    if self.columns != self.next_columns:
      log.error('Multiple frame sheet column mods on same frame')
      return

    object_slot = variable.object
    column = FrameSheetColumn(variable)
    if column not in self.columns:
      self.next_columns.insert(index, column)


  def append_variable(self, variable: Variable) -> None:
    self._insert_variable(len(self.columns), variable)
    self.columns = list(self.next_columns)


  def _move_column(self, source: int, dest: int) -> None:
    if self.columns != self.next_columns:
      log.error('Multiple frame sheet column mods on same frame')
      return

    column = self.next_columns[source]
    del self.next_columns[source]
    self.next_columns.insert(dest, column)


  def _remove_column(self, index: int) -> None:
    if self.columns != self.next_columns:
      log.error('Multiple frame sheet column mods on same frame')
      return
    del self.next_columns[index]


  def get_content_width(self) -> int:
    if len(self.columns) == 0:
      return 0
    return self.frame_column_width + sum(column.width for column in self.columns)


  def render_headers(self) -> None:
    header_labels = [self.displayer.column_header(column.variable) for column in self.columns]
    header_lines = max((len(label.split('\n')) for label in header_labels), default=1)

    ig.columns(len(self.columns) + 1)
    if len(self.columns) > 0:
      ig.set_column_width(-1, self.frame_column_width)
    ig.text('')
    ig.next_column()

    for index, column in enumerate(self.columns):
      initial_cursor_pos = ig.get_cursor_pos()
      ig.selectable(
        '##fs-col-' + str(id(self)) + '-' + str(id(column)),
        height = header_lines * ig.get_text_line_height(),
      )

      # TODO: Width adjusting
      ig.set_column_width(-1, column.width)

      if ig.begin_drag_drop_source():
        ig.text(header_labels[index])
        ig.set_drag_drop_payload('fs-col', str(index).encode('utf-8'))
        ig.end_drag_drop_source()

      if ig.begin_drag_drop_target():
        payload = ig.accept_drag_drop_payload('fs-col')
        if payload is not None:
          source = int(payload.decode('utf-8'))
          self._move_column(source, index)

        payload = ig.accept_drag_drop_payload('ve-var')
        if payload is not None:
          self._insert_variable(index, Variable.from_bytes(payload))

        ig.end_drag_drop_target()

      if ig.is_item_hovered() and ig.is_mouse_clicked(2):
        self._remove_column(index)

      if ig.begin_popup_context_item('##fs-colctx-' + str(id(self)) + '-' + str(id(column))):
        if ig.selectable('Close')[0]:
          self._remove_column(index)
        ig.end_popup_context_item()

      ig.set_cursor_pos(initial_cursor_pos)
      ig.text(header_labels[index])

      ig.next_column()
    ig.separator()
    ig.columns(1)


  def render_cell(self, frame: int, column: FrameSheetColumn) -> None:
    cell_variable = column.variable.with_frame(frame)

    data = self.pipeline.read(cell_variable)
    formatter = EmptyFormatter() if data is None else self.formatters[cell_variable]

    changed_data, clear_edit, selected, pressed = ui.render_variable_cell(
      f'cell-{frame}-{hash(column)}',
      data,
      formatter,
      (column.width, self.row_height),
      frame == self.sequence.selected_frame,
      frame,
      self.drag_handler.highlight_range(cell_variable),
    )
    if changed_data is not None:
      self.pipeline.write(cell_variable, changed_data.value)
    if clear_edit:
      self.pipeline.reset(cell_variable)
    if selected:
      self.sequence.set_selected_frame(frame)
    if pressed:
      self.drag_source = cell_variable

    return None


  def render_rows(self) -> None:
    ig.columns(len(self.columns) + 1)

    min_row = int(ig.get_scroll_y() + self.scroll_delta) // self.row_height - 1
    min_row = max(min_row, 0)
    max_row = int(ig.get_scroll_y() + self.scroll_delta + ig.get_window_height()) // self.row_height
    # max_row = min(max_row, self.get_row_count() - 1)

    self.sequence.extend_to_frame(max_row + 100)

    timeline_operations: List[Callable[[], None]] = []

    mouse_pos = (
      ig.get_mouse_pos().x - ig.get_window_position().x,
      ig.get_mouse_pos().y - ig.get_window_position().y + ig.get_scroll_y() + self.scroll_delta,
    )

    for row in range(min_row, max_row + 1):
      row_pos = (0.0, row * self.row_height - self.scroll_delta)
      ig.set_cursor_pos(row_pos)

      mouse_in_row = mouse_pos[1] > row_pos[1] and mouse_pos[1] <= row_pos[1] + self.row_height
      if mouse_in_row:
        self.drag_target = row

      if len(self.columns) > 0:
        ig.set_column_width(-1, self.frame_column_width)
      clicked, _ = ig.selectable(
        str(row) + '##fs-framenum-' + str(id(self)) + '-' + str(row),
        row == self.sequence.selected_frame,
        height=self.row_height - 8, # TODO: Compute padding
      )
      if clicked:
        self.sequence.set_selected_frame(row)

      if ig.begin_popup_context_item('##fs-framenumctx-' + str(id(self)) + '-' + str(row)):
        if ig.selectable('Insert above')[0]:
          def op(row):
            return lambda: self.sequence.insert_frame(row)
          timeline_operations.append(op(row))
        if ig.selectable('Insert below')[0]:
          def op(row):
            return lambda: self.sequence.insert_frame(row + 1)
          timeline_operations.append(op(row))
        if ig.selectable('Delete')[0]:
          def op(row):
            return lambda: self.sequence.delete_frame(row)
          timeline_operations.append(op(row))
        ig.end_popup_context_item()

      ig.next_column()

      for column in self.columns:
        self.render_cell(row, column)

        ig.set_column_width(-1, column.width)
        ig.next_column()
      ig.separator()

    ig.set_cursor_pos((0, (self.sequence.max_frame + 1) * self.row_height))

    ig.columns(1)

    for operation in timeline_operations:
      operation()


  def update_scolling(self) -> None:
    self.scroll_delta = 0.0

    if self.sequence.selected_frame == self.prev_selected_frame:
      return
    self.prev_selected_frame = self.sequence.selected_frame

    target_y = self.sequence.selected_frame * self.row_height
    curr_scroll_y = ig.get_scroll_y()
    current_min_y = curr_scroll_y
    current_max_y = curr_scroll_y + ig.get_window_height() - self.row_height

    if target_y > current_max_y:
      new_scroll_y = target_y - ig.get_window_height() + self.row_height
    elif target_y < current_min_y:
      new_scroll_y = target_y
    else:
      return

    ig.set_scroll_y(new_scroll_y)

    # Account for one frame set_scroll_y delay to prevent flickering
    self.scroll_delta = new_scroll_y - curr_scroll_y


  def render(self) -> None:
    self.render_headers()
    # TODO: Make the vertical scrollbar always visible?

    ig.begin_child('Frame Sheet Rows', flags=ig.WINDOW_ALWAYS_VERTICAL_SCROLLBAR)
    self.update_scolling()
    min_frame = int(ig.get_scroll_y()) // self.row_height - 1
    self.sequence.set_hotspot('frame-sheet-min', max(min_frame, 0))

    if self.drag_source is not None and not ig.is_mouse_down():
      self.drag_handler.release()
      self.drag_source = None
    self.drag_target = None
    self.render_rows()
    if self.drag_source is not None and self.drag_target is not None:
      self.drag_handler.drag(
        self.drag_source,
        self.pipeline.read(self.drag_source),
        self.drag_target,
      )

    ig.end_child()

    self.columns = list(self.next_columns)
