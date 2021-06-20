use std::fmt::Write;

use imgui::{self as ig, im_str};

// TODO: Show error message (on hover?)

pub(crate) fn render_input_text_with_error<T>(
    ui: &ig::Ui<'_>,
    id: &str,
    value: &str,
    buffer_size: usize,
    width: f32,
    validate: impl Fn(&str) -> Option<T>,
) -> Option<T> {
    let id_token = ui.push_id(id);

    let mut cursor_pos = ui.cursor_pos();
    cursor_pos[0] += ui.window_pos()[0];
    cursor_pos[1] += ui.window_pos()[1] - ui.scroll_y();

    ui.set_next_item_width(width);

    let mut buffer = ig::ImString::with_capacity(buffer_size);
    buffer.write_str(&value[..buffer_size]).unwrap();
    let changed = ui.input_text(im_str!("##input"), &mut buffer).build();

    let mut result_value = None;
    if changed {
        match validate(buffer.to_str()) {
            Some(new_value) => result_value = Some(new_value),
            None => {
                let dl = ui.get_window_draw_list();
                dl.add_rect(
                    cursor_pos,
                    [
                        cursor_pos[0] + width,
                        cursor_pos[1]
                            + ui.text_line_height()
                            + 2.0 * ui.clone_style().frame_padding[1],
                    ],
                    ig::ImColor32::from_rgb_f32s(1.0, 0.0, 0.0),
                )
                .build();
            }
        }
    }

    id_token.pop(ui);

    result_value
}
