use imgui::{self as ig, im_str};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum JoystickControlShape {
    Square,
    Circle,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct JoystickControl {
    start_value: Option<[f32; 2]>,
}

impl JoystickControl {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn value(&self, drag: [f32; 2]) -> [f32; 2] {
        let start_value = self.start_value.expect("missing start value");
        [start_value[0] + drag[0], start_value[1] + drag[1]]
    }

    fn is_active(&self) -> bool {
        self.start_value.is_some()
    }

    fn set_active(&mut self, value: [f32; 2]) {
        if self.start_value.is_none() {
            self.start_value = Some(value);
        }
    }

    fn reset(&mut self) {
        self.start_value = None;
    }

    pub(crate) fn render(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        mut stick: [f32; 2],
        shape: JoystickControlShape,
    ) -> Option<[f32; 2]> {
        let id_token = ui.push_id(id);

        let dl = ui.get_window_draw_list();

        let padding = 10.0;
        let content_region = ui.content_region_avail();
        let size = (content_region[0] - ui.clone_style().scrollbar_size - 2.0 * padding)
            .min(content_region[1] - 2.0 * padding)
            .min(200.0)
            .max(100.0);

        let initial_cursor_pos = ui.cursor_pos();
        let top_left = [
            initial_cursor_pos[0] + ui.window_pos()[0] - ui.scroll_x() + padding,
            initial_cursor_pos[1] + ui.window_pos()[1] - ui.scroll_y() + padding,
        ];

        let background_color = ig::ImColor32::from_rgba_f32s(0.0, 0.0, 0.0, 0.3);
        match shape {
            JoystickControlShape::Square => dl
                .add_rect(
                    top_left,
                    [top_left[0] + size, top_left[1] + size],
                    background_color,
                )
                .filled(true)
                .build(),
            JoystickControlShape::Circle => dl
                .add_circle(
                    [top_left[0] + size * 0.5, top_left[1] + size * 0.5],
                    size * 0.5,
                    background_color,
                )
                .num_segments(32)
                .filled(true)
                .build(),
        }

        let mut result = None;

        if self.is_active() && ui.is_mouse_down(ig::MouseButton::Left) {
            let new_offset =
                self.value(ui.mouse_drag_delta_with_threshold(ig::MouseButton::Left, 0.0));

            let mut new_stick = [
                new_offset[0] / size * 2.0 - 1.0,
                (1.0 - new_offset[1] / size) * 2.0 - 1.0,
            ];

            match shape {
                JoystickControlShape::Square => {
                    new_stick[0] = new_stick[0].clamp(-1.0, 1.0);
                    new_stick[1] = new_stick[1].clamp(-1.0, 1.0);
                }
                JoystickControlShape::Circle => {
                    let mag = (new_stick[0].powi(2) + new_stick[1].powi(2)).sqrt();
                    if mag > 1.0 {
                        new_stick[0] /= mag;
                        new_stick[1] /= mag;
                    }
                }
            }

            #[allow(clippy::float_cmp)]
            if new_stick != stick {
                stick = new_stick;
                result = Some(stick);
            }
        }

        let offset = [
            (stick[0] + 1.0) / 2.0 * size,
            (1.0 - (stick[1] + 1.0) / 2.0) * size,
        ];

        dl.add_line(
            [top_left[0] + size / 2.0, top_left[1] + size / 2.0],
            [top_left[0] + offset[0], top_left[1] + offset[1]],
            ig::ImColor32::from_rgba_f32s(1.0, 1.0, 1.0, 0.5),
        )
        .build();

        let button_size = 20.0;
        let button_pos = [
            padding + initial_cursor_pos[0] + offset[0] - button_size / 2.0,
            padding + initial_cursor_pos[1] + offset[1] - button_size / 2.0,
        ];
        ui.set_cursor_pos(button_pos);
        ui.button(im_str!("##joystick-button"), [button_size, button_size]);

        ui.set_cursor_pos([
            initial_cursor_pos[0],
            initial_cursor_pos[1] + size + 2.0 * padding,
        ]);

        if ui.is_item_active() {
            self.set_active(offset)
        } else {
            self.reset()
        }

        id_token.pop(ui);
        result
    }
}
