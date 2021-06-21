use std::{collections::HashMap, sync::Arc};

use imgui::{self as ig, im_str};
use wafel_api::{FloatValue, IntValue, Value};

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

#[derive(Debug, Clone, Default)]
pub(crate) struct VariableValueUi {
    editing: bool,
    initial_focus: bool,
}

impl VariableValueUi {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn render(
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
