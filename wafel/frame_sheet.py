import sys
from typing import *

import wafel.imgui as ig
from wafel.object_type import ObjectType
from wafel.variable import Variable, VariableId
from wafel.core import DataPath
from wafel.model import Model
from wafel.variable_format import Formatters, EmptyFormatter, VariableFormatter, VariableSemantics
import wafel.ui as ui
from wafel.util import *


class FrameSheetColumn:
  def __init__(
    self,
    variable: Variable,
    object_type: Optional[ObjectType] = None,
  ) -> None:
    self.variable = variable
    # TODO: Semantic object ids should make object_type unnecessary
    self.object_type = object_type
    self.width = 100 # TODO: Remove

  def __eq__(self, other) -> bool:
    return isinstance(other, FrameSheetColumn) and \
      self.variable == other.variable and \
      self.object_type == other.object_type

  def __hash__(self) -> int:
    return hash((self.variable, self.object_type))


class FrameSheet:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    super().__init__()
    self.model = model
    self.formatters = formatters

    self.columns: List[FrameSheetColumn] = []
    self.next_columns: List[FrameSheetColumn] = []

    self.row_height = 30
    self.frame_column_width = 60

    self.scroll_to_frame: Optional[int] = None
    def selected_frame_changed(frame: int) -> None:
      self.scroll_to_frame = frame
    self.model.on_selected_frame_change(selected_frame_changed)


  def _insert_variable(self, index: int, variable: Variable) -> None:
    if self.columns != self.next_columns:
      log.error('Multiple frame sheet column mods on same frame')
      return

    object_id = variable.get_object_id()
    if object_id is None:
      column = FrameSheetColumn(variable)
    else:
      # TODO: This should use the state that the drop began
      column = FrameSheetColumn(
        variable,
        self.model.get_object_type(self.model.selected_frame, object_id),
      )
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


  def get_row_count(self) -> int:
    return len(self.model.edits)


  def get_header_label(self, column: FrameSheetColumn) -> str:
    variable = column.variable
    object_id = variable.get_object_id()
    surface = variable.id.surface

    if object_id is not None:
      if column.object_type is None:
        return str(object_id) + '\n' + variable.label
      else:
        return str(object_id) + ' - ' + column.object_type.name + '\n' + variable.label

    elif surface is not None:
      return f'Surface {surface}\n{variable.label}'

    else:
      return variable.label


  def get_data(self, frame: int, column: FrameSheetColumn) -> Maybe[object]:
    variable = column.variable

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      row_object_type = self.model.get_object_type(frame, object_id)
      if row_object_type != column.object_type:
        return None

    if variable.id.surface is not None:
      num_surfaces = dcast(int, self.model.timeline.get(frame, 'gSurfacesAllocated'))
      if variable.id.surface >= num_surfaces:
        return None

    return Just(variable.get(self.model.timeline, frame))


  def set_data(self, frame: int, column: FrameSheetColumn, data: Any) -> None:
    variable = column.variable

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      row_object_type = self.model.get_object_type(frame, object_id)
      if row_object_type != column.object_type:
        raise Exception # TODO: Error message

    if variable.id.surface is not None:
      num_surfaces = dcast(int, self.model.timeline.get(frame, 'gSurfacesAllocated'))
      if variable.id.surface >= num_surfaces:
        raise Exception

    self.model.edit(frame, variable, data)


  def get_content_width(self) -> int:
    if len(self.columns) == 0:
      return 0
    return self.frame_column_width + sum(column.width for column in self.columns)


  def render_headers(self) -> None:
    header_labels = [self.get_header_label(column) for column in self.columns]
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
          variable = self.model.variables[VariableId.from_bytes(payload)]
          self._insert_variable(index, variable)

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
    data: object
    formatter: VariableFormatter
    maybe_data = self.get_data(frame, column)
    if maybe_data is None:
      data = None
      formatter = EmptyFormatter()
    else:
      data = maybe_data.value
      formatter = EmptyFormatter() if data is None else self.formatters[column.variable]

    changed_data, clear_edit, selected = ui.render_variable_cell(
      f'cell-{frame}-{hash(column)}',
      data,
      formatter,
      (column.width, self.row_height),
      self.model.is_edited(frame, column.variable.id),
      frame == self.model.selected_frame,
    )
    if changed_data is not None:
      self.set_data(frame, column, changed_data.value)
    if clear_edit:
      self.model.reset(frame, column.variable.id)
    if selected:
      self.model.selected_frame = frame


  def render_rows(self) -> None:
    ig.columns(len(self.columns) + 1)

    min_row = int(ig.get_scroll_y()) // self.row_height - 1
    min_row = max(min_row, 0)
    max_row = int(ig.get_scroll_y() + ig.get_window_height()) // self.row_height
    # max_row = min(max_row, self.get_row_count() - 1)

    self.model.edits.extend(max_row + 100)

    timeline_operations: List[Callable[[], None]] = []

    for row in range(min_row, max_row + 1):
      initial_pos = ig.get_cursor_pos()
      ig.set_cursor_pos((initial_pos[0], row * self.row_height))

      if len(self.columns) > 0:
        ig.set_column_width(-1, self.frame_column_width)
      clicked, _ = ig.selectable(
        str(row) + '##fs-framenum-' + str(id(self)) + '-' + str(row),
        row == self.model.selected_frame,
        height=self.row_height - 8, # TODO: Compute padding
      )
      if clicked:
        self.model.selected_frame = row

      if ig.begin_popup_context_item('##fs-framenumctx-' + str(id(self)) + '-' + str(row)):
        if ig.selectable('Insert above')[0]:
          def op(row):
            return lambda: self.model.insert_frame(row)
          timeline_operations.append(op(row))
        if ig.selectable('Insert below')[0]:
          def op(row):
            return lambda: self.model.insert_frame(row + 1)
          timeline_operations.append(op(row))
        if ig.selectable('Delete')[0]:
          def op(row):
            return lambda: self.model.delete_frame(row)
          timeline_operations.append(op(row))
        ig.end_popup_context_item()

      ig.next_column()

      for column in self.columns:
        self.render_cell(row, column)

        ig.set_column_width(-1, column.width)
        ig.next_column()
      ig.separator()

    ig.set_cursor_pos((0, self.get_row_count() * self.row_height))

    ig.columns(1)

    for operation in timeline_operations:
      operation()


  def update_scolling(self) -> None:
    if self.scroll_to_frame is None:
      return

    target_y = self.scroll_to_frame * self.row_height
    current_min_y = ig.get_scroll_y()
    current_max_y = ig.get_scroll_y() + ig.get_window_height() - self.row_height

    if target_y > current_max_y:
      ig.set_scroll_y(target_y - ig.get_window_height() + self.row_height)
    elif target_y < current_min_y:
      ig.set_scroll_y(target_y)

    self.scroll_to_frame = None


  def render(self) -> None:
    self.render_headers()
    # TODO: Make the vertical scrollbar always visible?
    ig.begin_child('Frame Sheet Rows', flags=ig.WINDOW_ALWAYS_VERTICAL_SCROLLBAR)
    self.update_scolling()
    min_frame = int(ig.get_scroll_y()) // self.row_height - 1
    self.model.timeline.set_hotspot('frame-sheet-min', max(min_frame, 0))
    self.render_rows()
    ig.end_child()

    self.columns = list(self.next_columns)
