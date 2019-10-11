import sys
from typing import *
import traceback

import imgui as ig

from wafel.core import Variable, ObjectType, VariableParam, VariableEdit, \
  VariableId
from wafel.model import Model
from wafel.variable_format import Formatters, EmptyFormatter
from wafel.variable_display import VariableDisplayAction, \
  VariableDisplayEditState, display_variable_data


class FrameSheetColumn:
  def __init__(
    self,
    variable: Variable,
    object_type: Optional[ObjectType] = None,
  ) -> None:
    self.variable = variable
    # TODO: Semantic object ids should make object_type unnecessary
    self.object_type = object_type
    self.width = 100

  def __eq__(self, other) -> bool:
    return isinstance(other, FrameSheetColumn) and \
      self.variable == other.variable and \
      self.object_type == other.object_type


T = TypeVar('T')

class CellEditState(Generic[T]):
  def __init__(self) -> None:
    self.current_cell: Optional[Tuple[T, VariableDisplayEditState]] = None

  def begin_edit(self, cell: T) -> None:
    self.current_cell = (cell, VariableDisplayEditState())

  def end_edit(self) -> None:
    self.current_cell = None

  def get(self, cell: T) -> Optional[VariableDisplayEditState]:
    if self.current_cell is not None and self.current_cell[0] == cell:
      return self.current_cell[1]
    else:
      return None


class FrameSheet:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    super().__init__()
    self.model = model
    self.formatters = formatters
    self.columns: List[FrameSheetColumn] = []
    self.row_height = 30
    self.frame_column_width = 60
    self.cell_edit_state: CellEditState[Tuple[int, FrameSheetColumn]] = CellEditState()

    self.scroll_to_frame: Optional[int] = None
    def selected_frame_changed(frame: int) -> None:
      self.scroll_to_frame = frame
    self.model.on_selected_frame_change(selected_frame_changed)


  def insert_variable(self, index: int, variable: Variable) -> None:
    object_id = variable.get_object_id()
    if object_id is None:
      column = FrameSheetColumn(variable)
    else:
      # TODO: This should use the state that the drop began
      state = self.model.timeline[self.model.selected_frame]
      column = FrameSheetColumn(variable, self.model.get_object_type(state, object_id))
    if column not in self.columns:
      self.columns.insert(index, column)


  def append_variable(self, variable: Variable) -> None:
    self.insert_variable(len(self.columns), variable)


  def move_column(self, source: int, dest: int) -> None:
    column = self.columns[source]
    del self.columns[source]
    self.columns.insert(dest, column)


  def get_row_count(self) -> int:
    return len(self.model.timeline)


  def get_header_label(self, column: FrameSheetColumn) -> str:
    variable = column.variable
    object_id = variable.get_object_id()

    if object_id is None:
      return variable.display_name

    if column.object_type is None:
      return str(object_id) + '\n' + variable.display_name

    return str(object_id) + ' - ' + column.object_type.name + '\n' + variable.display_name


  def get_data(self, frame: int, column: FrameSheetColumn) -> Any:
    variable = column.variable
    state = self.model.timeline[frame]

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      row_object_type = self.model.get_object_type(state, object_id)
      if row_object_type != column.object_type:
        raise Exception # TODO: Error msg

    args = { VariableParam.STATE: state }
    return variable.get(args)


  def set_data(self, frame: int, column: FrameSheetColumn, data: Any) -> None:
    variable = column.variable

    object_id = variable.get_object_id()
    if column.object_type is not None and object_id is not None:
      state = self.model.timeline[frame]
      row_object_type = self.model.get_object_type(state, object_id)
      if row_object_type != column.object_type:
        raise Exception # TODO: Error message
      del state

    self.model.edits.add(frame, VariableEdit(variable, data))


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

    for index, column in list(enumerate(self.columns)):
      initial_cursor_pos = ig.get_cursor_pos()
      ig.selectable(
        '##fs-col-' + str(id(column)),
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
          self.move_column(source, index)

        payload = ig.accept_drag_drop_payload('ve-var')
        if payload is not None:
          variable = self.model.variables[VariableId.from_bytes(payload)]
          self.insert_variable(index, variable)

        ig.end_drag_drop_target()

      ig.set_cursor_pos(initial_cursor_pos)
      ig.text(header_labels[index])

      ig.next_column()
    ig.separator()
    ig.columns(1)


  def render_cell(self, frame: int, column: FrameSheetColumn) -> None:
    cell_cursor_pos = ig.get_cursor_pos()
    cell_cursor_pos = (
      cell_cursor_pos[0] + ig.get_window_position()[0] - ig.get_style().item_spacing[0],
      cell_cursor_pos[1] + ig.get_window_position()[1] - ig.get_scroll_y() - ig.get_style().item_spacing[1],
    )

    try:
      data = self.get_data(frame, column)
      formatter = self.formatters[column.variable]
    except: # TODO: Only catch object mismatch exception
      data = None
      formatter = EmptyFormatter()

    def on_select() -> None:
      self.model.selected_frame = frame

    def on_edit(data: Any) -> None:
      self.set_data(frame, column, data)

    action = display_variable_data(
      'fs-cell-' + str(frame) + '-' + str(id(column)),
      data,
      formatter,
      (
        column.width - 2 * ig.get_style().item_spacing[0],
        self.row_height - 2 * ig.get_style().item_spacing[1],
      ),
      self.cell_edit_state.get((frame, column)),
      highlight = frame == self.model.selected_frame,
      on_edit = on_edit,
      on_select = on_select,
    )
    if action == VariableDisplayAction.BEGIN_EDIT:
      self.cell_edit_state.begin_edit((frame, column))
    elif action == VariableDisplayAction.END_EDIT:
      self.cell_edit_state.end_edit()

    if self.model.edits.is_edited(frame, column.variable):
      dl = ig.get_window_draw_list()
      dl.add_rect(
        cell_cursor_pos[0],
        cell_cursor_pos[1],
        cell_cursor_pos[0] + column.width - 1,
        cell_cursor_pos[1] + self.row_height - 1,
        ig.get_color_u32_rgba(0.8, 0.6, 0, 1),
      )


  def render_rows(self) -> None:
    ig.columns(len(self.columns) + 1)

    min_row = int(ig.get_scroll_y()) // self.row_height - 1
    min_row = max(min_row, 0)
    max_row = int(ig.get_scroll_y() + ig.get_window_height()) // self.row_height
    max_row = min(max_row, self.get_row_count() - 1)

    for row in range(min_row, max_row + 1):
      initial_pos = ig.get_cursor_pos()
      ig.set_cursor_pos((initial_pos[0], row * self.row_height))

      if len(self.columns) > 0:
        ig.set_column_width(-1, self.frame_column_width)
      clicked, _ = ig.selectable(
        str(row) + '##fs-framenum-' + str(row),
        row == self.model.selected_frame,
        height=self.row_height - 8, # TODO: Compute padding
      )
      if clicked:
        self.model.selected_frame = row
      ig.next_column()

      for column in self.columns:
        self.render_cell(row, column)

        ig.set_column_width(-1, column.width)
        ig.next_column()
      ig.separator()

    ig.set_cursor_pos((0, self.get_row_count() * self.row_height))

    ig.columns(1)


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
    self.render_rows()
    ig.end_child()
