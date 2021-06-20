use imgui::{self as ig, im_str};

// TODO: Hide loaded frames in release mode

pub(crate) fn render_frame_slider(
    ui: &ig::Ui<'_>,
    id: &str,
    current_frame: u32,
    max_frame: u32,
    loaded_frames: &[u32],
) -> Option<u32> {
    let id_token = ui.push_id(id);

    let mut pos = ui.cursor_pos();
    pos[0] += ui.window_pos()[0];
    pos[1] += ui.window_pos()[1] - ui.scroll_y();

    let width = ui.content_region_avail()[0];
    ui.set_next_item_width(width);

    let mut new_frame = current_frame;
    let changed = ig::Slider::new(im_str!("##slider"))
        .range(0..=max_frame)
        .build(ui, &mut new_frame);

    let dl = ui.get_window_draw_list();
    for &frame in loaded_frames {
        let line_pos = pos[0] + frame as f32 / (max_frame + 1) as f32 * width;
        dl.add_line(
            [line_pos, pos[1] + 13.0],
            [line_pos, pos[1] + 18.0],
            ig::ImColor32::from_rgb_f32s(1.0, 0.0, 0.0),
        )
        .build();
    }

    id_token.pop(ui);
    changed.then(|| new_frame)
}
