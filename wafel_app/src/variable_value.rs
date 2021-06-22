use std::{collections::HashMap, ops::Range, sync::Arc};

use imgui::{self as ig, im_str};
use wafel_api::{FloatValue, IntValue, Value};
use wafel_core::Variable;

// TODO: Show error message while editing?

#[derive(Debug, Clone)]
pub(crate) enum VariableFormatter {
    Empty,
    DecimalInt, // TODO: Signed, unsigned, int sizes
    Float,      // TODO: Precision
    Checkbox,
    Enum {
        value_to_name: Arc<HashMap<IntValue, String>>,
        name_to_value: Arc<HashMap<String, IntValue>>,
    },
}

impl VariableFormatter {
    fn text_output(&self, value: &Value) -> String {
        match self {
            VariableFormatter::Empty => String::new(),
            VariableFormatter::DecimalInt => value.as_int().to_string(),
            VariableFormatter::Float => value.as_float().to_string(),
            VariableFormatter::Enum { value_to_name, .. } => {
                let n = value.as_int();
                value_to_name
                    .get(&n)
                    .cloned()
                    .unwrap_or_else(|| format!("{}", n))
            }
            _ => unimplemented!("{:?}", self),
        }
    }

    fn text_input(&self, input: &str) -> Option<Value> {
        // TODO: Allow other bases for parsing ints
        match self {
            VariableFormatter::Empty => Some(Value::None),
            VariableFormatter::DecimalInt => input.parse::<IntValue>().ok().map(Value::Int),
            VariableFormatter::Float => input.parse::<FloatValue>().ok().map(Value::Float),
            VariableFormatter::Enum { name_to_value, .. } => match name_to_value.get(input) {
                Some(&value) => Some(Value::Int(value)),
                None => input.parse::<IntValue>().ok().map(Value::Int),
            },
            _ => unimplemented!("{:?}", self),
        }
    }

    fn bool_output(&self, value: &Value) -> bool {
        match self {
            VariableFormatter::Checkbox => value.as_int() != 0,
            _ => unimplemented!("{:?}", self),
        }
    }

    fn bool_input(&self, input: bool) -> Option<Value> {
        match self {
            VariableFormatter::Checkbox => Some(Value::Int(input as IntValue)),
            _ => unimplemented!("{:?}", self),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VariableValueResult {
    pub(crate) changed_value: Option<Value>,
    pub(crate) clicked: bool,
    pub(crate) pressed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct VariableCellResult {
    pub(crate) changed_value: Option<Value>,
    pub(crate) clear_edit: bool,
    pub(crate) selected: bool,
    pub(crate) pressed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LabeledVariableResult {
    pub(crate) changed_value: Option<Value>,
    pub(crate) clear_edit: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct VariableValueUi {
    editing: bool,
    initial_focus: bool,
}

impl VariableValueUi {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn render_value(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        value: &Value,
        formatter: VariableFormatter,
        size: [f32; 2],
        highlight: bool,
    ) -> VariableValueResult {
        let id_token = ui.push_id(id);

        let result = match formatter {
            VariableFormatter::Checkbox => {
                self.render_checkbox(ui, value, formatter, size, highlight)
            }
            _ => self.render_text(ui, value, formatter, size, highlight),
        };

        id_token.pop(ui);
        result
    }

    pub(crate) fn render_cell(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        value: &Value,
        formatter: VariableFormatter,
        cell_size: [f32; 2],
        is_selected: bool,
        frame: Option<u32>,
        highlight_range: Option<(Range<u32>, ig::ImColor32)>,
    ) -> VariableCellResult {
        let id_token = ui.push_id(id);

        let window_pos = ui.window_pos();
        let item_spacing = ui.clone_style().item_spacing;

        let mut cell_cursor_pos = ui.cursor_pos();
        cell_cursor_pos[0] += window_pos[0] - item_spacing[0];
        cell_cursor_pos[1] += window_pos[1] - ui.scroll_y() - item_spacing[1];

        if let (Some((highlight_frames, highlight_color)), Some(frame)) = (highlight_range, frame) {
            let margin = 5.0;
            let offset_top = if frame == highlight_frames.start {
                margin
            } else {
                0.0
            };
            let offset_bottom = if frame + 1 == highlight_frames.end {
                margin
            } else {
                0.0
            };
            let dl = ui.get_window_draw_list();
            dl.add_rect(
                [cell_cursor_pos[0] + margin, cell_cursor_pos[1] + offset_top],
                [
                    cell_cursor_pos[0] + cell_size[0] - margin,
                    cell_cursor_pos[1] + cell_size[1] - offset_bottom,
                ],
                highlight_color,
            )
            .filled(true)
            .build();
        }

        let value_result = self.render_value(
            ui,
            "value",
            value,
            formatter,
            [
                cell_size[0] - 2.0 * item_spacing[0],
                cell_size[1] - 2.0 * item_spacing[1],
            ],
            is_selected,
        );

        let clear_edit = ui.is_item_hovered() && ui.is_mouse_down(ig::MouseButton::Middle);

        id_token.pop(ui);

        VariableCellResult {
            changed_value: value_result.changed_value,
            clear_edit,
            selected: value_result.clicked,
            pressed: value_result.pressed,
        }
    }

    pub(crate) fn render_labeled(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        label: &str,
        variable: &Variable,
        value: &Value,
        formatter: VariableFormatter,
        is_edited: bool,
        label_width: f32,
        value_width: f32,
    ) -> LabeledVariableResult {
        let id_token = ui.push_id(id);

        ig::Selectable::new(&im_str!("{}##label", label))
            .size([label_width, 0.0])
            .build(ui);

        // TODO: Implement drag & drop
        // if ui.begin_drag_drop_source() {
        //   ui.text(label);
        //   ui.set_drag_drop_payload("ve-var", variable.to_bytes());
        //   ui.end_drag_drop_source();
        // }

        ui.same_line(0.0);

        let cell_size = [
            value_width,
            ui.text_line_height() + 2.0 * ui.clone_style().frame_padding[1],
        ];

        let mut cell_cursor_pos = ui.cursor_pos();
        cell_cursor_pos[0] += ui.window_pos()[0] - ui.scroll_x();
        cell_cursor_pos[1] += ui.window_pos()[1] - ui.scroll_y();

        let value_result = self.render_value(ui, "value", value, formatter, cell_size, false);

        let clear_edit =
            is_edited && ui.is_item_hovered() && ui.is_mouse_down(ig::MouseButton::Middle);

        if is_edited {
            let dl = ui.get_window_draw_list();
            let mut spacing = ui.clone_style().item_spacing;
            spacing = [spacing[0] / 2.0, spacing[1] / 2.0];
            dl.add_rect(
                [
                    cell_cursor_pos[0] - spacing[0],
                    cell_cursor_pos[1] - spacing[1],
                ],
                [
                    cell_cursor_pos[0] + cell_size[0] + spacing[0] - 1.0,
                    cell_cursor_pos[1] + cell_size[1] + spacing[1] - 1.0,
                ],
                ig::ImColor32::from_rgb_f32s(0.8, 0.6, 0.0),
            )
            .build();
        }

        id_token.pop(ui);

        LabeledVariableResult {
            changed_value: value_result.changed_value,
            clear_edit,
        }
    }

    fn render_text(
        &mut self,
        ui: &ig::Ui<'_>,
        value: &Value,
        formatter: VariableFormatter,
        size: [f32; 2],
        highlight: bool,
    ) -> VariableValueResult {
        if !self.editing {
            let clicked = ig::Selectable::new(&im_str!("{}##text", formatter.text_output(value)))
                .selected(highlight)
                .size(size)
                .allow_double_click(true)
                .build(ui);

            if clicked && ui.is_mouse_double_clicked(ig::MouseButton::Left) {
                self.editing = true;
                self.initial_focus = false;
            }

            let pressed = ui.is_item_hovered() && ui.is_mouse_clicked(ig::MouseButton::Left);

            return VariableValueResult {
                changed_value: None,
                clicked,
                pressed,
            };
        }

        let mut cursor_pos = ui.cursor_pos();
        cursor_pos[0] += ui.window_pos()[0];
        cursor_pos[1] += ui.window_pos()[1] - ui.scroll_y();

        let value_text = formatter.text_output(value);
        let mut buffer = ig::ImString::from(value_text);
        buffer.reserve(1000); // TODO: Add clipboard length

        ui.set_next_item_width(size[0]);
        ui.input_text(im_str!("##text-edit"), &mut buffer).build();

        let input = buffer.to_string();

        if !self.initial_focus {
            ui.set_keyboard_focus_here(ig::FocusedWidget::Previous);
            self.initial_focus = true;
        } else if !ui.is_item_active() {
            self.editing = false;
        }

        match formatter.text_input(&input) {
            Some(input_value) => {
                if input_value != *value {
                    return VariableValueResult {
                        changed_value: Some(input_value),
                        clicked: false,
                        pressed: false,
                    };
                }
            }
            None => {
                let dl = ui.get_window_draw_list();
                dl.add_rect(
                    [cursor_pos[0], cursor_pos[1]],
                    [
                        cursor_pos[0] + size[0],
                        cursor_pos[1]
                            + ui.text_line_height()
                            + 2.0 * ui.clone_style().frame_padding[1],
                    ],
                    ig::ImColor32::from_rgb_f32s(1.0, 0.0, 0.0),
                )
                .build();
            }
        }

        VariableValueResult {
            changed_value: None,
            clicked: false,
            pressed: false,
        }
    }

    fn render_checkbox(
        &mut self,
        ui: &ig::Ui<'_>,
        value: &Value,
        formatter: VariableFormatter,
        size: [f32; 2],
        highlight: bool,
    ) -> VariableValueResult {
        let cursor_pos = ui.cursor_pos();

        let mut input = formatter.bool_output(value);
        ui.checkbox(im_str!("##checkbox"), &mut input);

        ui.set_cursor_pos(cursor_pos);
        let clicked = ig::Selectable::new(im_str!("##checkbox-background"))
            .selected(highlight)
            .size(size)
            .build(ui);

        let pressed = ui.is_item_hovered() && ui.is_mouse_clicked(ig::MouseButton::Left);

        let input_value = formatter.bool_input(input).expect("invalid formatter");
        if input_value != *value {
            VariableValueResult {
                changed_value: Some(input_value),
                clicked,
                pressed,
            }
        } else {
            VariableValueResult {
                changed_value: None,
                clicked,
                pressed,
            }
        }
    }
}
