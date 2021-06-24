use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    time::{Duration, Instant},
};

use imgui::{self as ig, im_str};
use wafel_core::{EditRangeId, Pipeline, Variable};

use crate::variable_value::{VariableFormatter, VariableValueUi};

const DEFAULT_COLUMN_WIDTH: f32 = 100.0;
const ROW_HEIGHT: f32 = 30.0;
const FRAME_COLUMN_WIDTH: f32 = 60.0;

#[derive(Debug, Default)]
pub(crate) struct FrameSheet {
    columns: Vec<Variable>,
    next_columns: Vec<Variable>,
    column_widths: HashMap<Variable, f32>,
    dragging: bool,
    time_started_dragging: Option<Instant>,
    prev_selected_frame: Option<u32>,
    scroll_delta: f32,
    cell_ui: HashMap<(Variable, u32), VariableValueUi>, // TODO: Remove unused each frame
}

impl FrameSheet {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn check_for_edits(&self) -> bool {
        let edits_pending = self.columns != self.next_columns;
        debug_assert!(!edits_pending, "multiple edits on same frame");
        edits_pending
    }

    fn insert_variable(&mut self, index: usize, variable: Variable) {
        if self.check_for_edits() {
            return;
        }
        let variable = variable.without_frame();
        if !self.columns.contains(&variable) {
            self.next_columns.insert(index, variable);
        }
    }

    pub(crate) fn append_variable(&mut self, variable: Variable) {
        self.insert_variable(self.columns.len(), variable);
        self.columns.clone_from(&self.next_columns);
    }

    fn move_column(&mut self, source: usize, dest: usize) {
        if self.check_for_edits() {
            return;
        }
        let column = self.next_columns.remove(source);
        self.next_columns.insert(dest, column);
    }

    fn remove_column(&mut self, index: usize) {
        if self.check_for_edits() {
            return;
        }
        self.next_columns.remove(index);
    }

    fn column_width(&self, column: &Variable) -> f32 {
        self.column_widths
            .get(column)
            .copied()
            .unwrap_or(DEFAULT_COLUMN_WIDTH)
    }

    fn content_width(&self) -> f32 {
        if self.columns.is_empty() {
            0.0
        } else {
            FRAME_COLUMN_WIDTH
                + self
                    .columns
                    .iter()
                    .map(|c| self.column_width(c))
                    .sum::<f32>()
        }
    }

    fn column_header(&self, variable: &Variable) -> String {
        // TODO: column header
        format!("{}", variable)
    }

    fn render_headers(&mut self, ui: &ig::Ui<'_>) {
        let header_labels: Vec<String> =
            self.columns.iter().map(|c| self.column_header(c)).collect();
        let header_lines = header_labels
            .iter()
            .map(|s| s.split('\n').count())
            .max()
            .unwrap_or(1);

        ui.columns((self.columns.len() + 1) as i32, im_str!("headers"), true);
        if !self.columns.is_empty() {
            ui.set_column_width(-1, FRAME_COLUMN_WIDTH);
        }
        ui.text("");
        ui.next_column();

        let columns_copy = self.columns.clone();
        for (index, column) in columns_copy.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            column.hash(&mut hasher);
            let column_hash = hasher.finish();

            let initial_cursor_pos = ui.cursor_pos();
            ig::Selectable::new(&im_str!("##fs-col-{}", column_hash))
                .size([0.0, header_lines as f32 * ui.text_line_height()])
                .build(ui);

            // TODO: Width adjusting
            ui.set_column_width(-1, self.column_width(column));

            // TODO: Drag & drop
            //       if ig.begin_drag_drop_source():
            //ig.text(header_labels[index])
            //ig.set_drag_drop_payload('fs-col', str(index).encode('utf-8'))
            //ig.end_drag_drop_source()
            //
            //       if ig.begin_drag_drop_target():
            //payload = ig.accept_drag_drop_payload('fs-col')
            //if payload is not None:
            //  source = int(payload.decode('utf-8'))
            //  self._move_column(source, index)
            //
            //payload = ig.accept_drag_drop_payload('ve-var')
            //if payload is not None:
            //  self._insert_variable(index, Variable.from_bytes(payload))
            //
            //ig.end_drag_drop_target()

            if ui.is_item_hovered() && ui.is_mouse_clicked(ig::MouseButton::Middle) {
                self.remove_column(index);
            }

            // TODO: Context menu
            //       if ig.begin_popup_context_item('##fs-colctx-' + str(id(self)) + '-' + str(id(column))):
            //if ig.selectable('Close')[0]:
            //  self._remove_column(index)
            //ig.end_popup_context_item()

            ui.set_cursor_pos(initial_cursor_pos);
            ui.text(&header_labels[index]);

            ui.next_column();
        }

        ui.separator();
        ui.columns(1, im_str!("end-headers"), false);
    }

    fn range_color(&self, id: EditRangeId) -> ig::ImColor32 {
        static RANGE_COLORS: &[[f32; 4]] = &[
            [0.4, 0.9, 0.0, 0.3],
            [0.6, 0.4, 0.0, 0.3],
            [0.4, 0.9, 0.5, 0.3],
            [0.5, 0.5, 0.5, 0.3],
            [0.2, 0.6, 0.0, 0.3],
            [0.7, 0.7, 0.3, 0.3],
            [0.3, 0.3, 0.7, 0.3],
        ];
        let index = id.0 % RANGE_COLORS.len();
        let [r, g, b, a] = RANGE_COLORS[index];
        ig::ImColor32::from_rgba_f32s(r, g, b, a)
    }

    fn render_cell(
        &mut self,
        ui: &ig::Ui<'_>,
        pipeline: &mut Pipeline,
        selected_frame: &mut u32,
        frame: u32,
        column: &Variable,
    ) {
        let mut hasher = DefaultHasher::new();
        column.hash(&mut hasher);
        let column_hash = hasher.finish();

        let cell_variable = column.with_frame(frame);

        let value = pipeline.read(&cell_variable);
        let formatter = if value.is_none() {
            VariableFormatter::Empty
        } else if pipeline.is_bit_flag(&cell_variable) {
            VariableFormatter::Checkbox
        } else if pipeline.is_int(&cell_variable) {
            VariableFormatter::DecimalInt
        } else if pipeline.is_float(&cell_variable) {
            VariableFormatter::Float
        } else {
            unimplemented!("{}", &cell_variable)
        };

        let edit_range = pipeline.find_edit_range(&cell_variable);
        let highlight_range = edit_range.map(|edit_range| {
            let color = self.range_color(edit_range.id);
            (edit_range.frames.clone(), color)
        });

        let cell_size = [self.column_width(column), ROW_HEIGHT];

        let cell_ui = self.cell_ui.entry((column.clone(), frame)).or_default();
        let cell_result = cell_ui.render_cell(
            ui,
            &format!("cell-{}-{}", frame, column_hash),
            &value,
            formatter,
            cell_size,
            frame == *selected_frame,
            Some(frame),
            highlight_range,
        );

        if let Some(changed_value) = cell_result.changed_value {
            pipeline.write(&cell_variable, changed_value);
        }
        if cell_result.clear_edit {
            pipeline.reset(&cell_variable);
        }
        if cell_result.selected {
            *selected_frame = frame;
        }
        if cell_result.pressed {
            pipeline.begin_drag(&cell_variable, pipeline.read(&cell_variable));
            self.dragging = true;
            self.time_started_dragging = Some(Instant::now());
        }
    }

    fn render_rows(
        &mut self,
        ui: &ig::Ui<'_>,
        pipeline: &mut Pipeline,
        max_frame: &mut u32,
        selected_frame: &mut u32,
    ) {
        ui.columns((self.columns.len() + 1) as i32, im_str!("rows"), true);

        let min_row =
            ((ui.scroll_y() + self.scroll_delta) as i32 / ROW_HEIGHT as i32 - 1).max(0) as u32;
        let max_row = ((ui.scroll_y() + self.scroll_delta + ui.window_size()[1]) as i32
            / ROW_HEIGHT as i32) as u32;

        *max_frame = (*max_frame).max(max_row + 100);

        // TODO:
        //timeline_operations: List[Callable[[], None]] = []

        // TODO: python uses get_mouse_pos()
        let mouse_pos = [
            ui.cursor_pos()[0] - ui.window_pos()[0],
            ui.cursor_pos()[1] - ui.window_pos()[1] + ui.scroll_y() + self.scroll_delta,
        ];

        for row in min_row..=max_row {
            let row_pos = [0.0, row as f32 * ROW_HEIGHT - self.scroll_delta];
            ui.set_cursor_pos(row_pos);

            let mouse_in_row = mouse_pos[1] > row_pos[1] && mouse_pos[1] <= row_pos[1] + ROW_HEIGHT;
            if mouse_in_row
                && self.dragging
                && self.time_started_dragging.unwrap().elapsed() > Duration::from_secs_f32(0.1)
            {
                pipeline.update_drag(row);
            }

            if !self.columns.is_empty() {
                ui.set_column_width(-1, FRAME_COLUMN_WIDTH);
            }

            // TODO: Width doesn't match header width
            let clicked = ig::Selectable::new(&im_str!("{}##fs-framenum-{}", row, row))
                .selected(row == *selected_frame)
                .size([0.0, ROW_HEIGHT - 8.0]) // TODO: Compute padding
                .build(ui);
            if clicked {
                *selected_frame = row;
            }

            // TODO: Context menu
            //  if ig.begin_popup_context_item('##fs-framenumctx-' + str(id(self)) + '-' + str(row)):
            //    if ig.selectable('Insert above')[0]:
            //      def op(row):
            //        return lambda: self.sequence.insert_frame(row)
            //      timeline_operations.append(op(row))
            //    if ig.selectable('Insert below')[0]:
            //      def op(row):
            //        return lambda: self.sequence.insert_frame(row + 1)
            //      timeline_operations.append(op(row))
            //    if ig.selectable('Delete')[0]:
            //      def op(row):
            //        return lambda: self.sequence.delete_frame(row)
            //      timeline_operations.append(op(row))
            //    ig.end_popup_context_item()

            ui.next_column();

            for column in &self.columns.clone() {
                self.render_cell(ui, pipeline, selected_frame, row, column);

                ui.set_column_width(-1, self.column_width(column));
                ui.next_column();
            }

            ui.separator();
        }

        ui.set_cursor_pos([0.0, (*max_frame + 1) as f32 * ROW_HEIGHT]);

        ui.columns(1, im_str!("end-rows"), true);

        // TODO:
        //for operation in timeline_operations:
        //  operation()
    }

    fn update_scrolling(&mut self, ui: &ig::Ui<'_>, selected_frame: &mut u32) {
        self.scroll_delta = 0.0;

        if self.prev_selected_frame == Some(*selected_frame) {
            return;
        }
        self.prev_selected_frame = Some(*selected_frame);

        let target_y = *selected_frame as f32 * ROW_HEIGHT;
        let curr_scroll_y = ui.scroll_y();
        let current_min_y = curr_scroll_y;
        let current_max_y = curr_scroll_y + ui.window_size()[1] - ROW_HEIGHT;

        let new_scroll_y;
        if target_y > current_max_y {
            new_scroll_y = target_y - ui.window_size()[1] + ROW_HEIGHT;
        } else if target_y < current_min_y {
            new_scroll_y = target_y;
        } else {
            return;
        }

        ui.set_scroll_y(new_scroll_y);

        // Account for one frame set_scroll_y delay to prevent flickering
        self.scroll_delta = new_scroll_y - curr_scroll_y;
    }

    pub(crate) fn render(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        pipeline: &mut Pipeline,
        max_frame: &mut u32,
        selected_frame: &mut u32,
    ) {
        let id_token = ui.push_id(id);

        self.render_headers(ui);
        // TODO: Make the vertical scrollbar always visible?

        ig::ChildWindow::new("Frame Sheet Rows")
            .flags(ig::WindowFlags::ALWAYS_VERTICAL_SCROLLBAR)
            .build(ui, || {
                self.update_scrolling(ui, selected_frame);
                let min_frame = (ui.scroll_y() as i32 / ROW_HEIGHT as i32 - 1).max(0) as u32;
                pipeline.set_hotspot("frame-sheet-min", min_frame);

                if self.dragging && !ui.is_mouse_down(ig::MouseButton::Left) {
                    pipeline.release_drag();
                    self.dragging = false;
                }

                self.render_rows(ui, pipeline, max_frame, selected_frame);
            });

        self.columns = self.next_columns.clone();

        id_token.pop(ui);
    }
}
