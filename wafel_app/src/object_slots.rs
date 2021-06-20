use imgui::{self as ig, im_str};
use wafel_core::{ObjectBehavior, ObjectSlot};

pub(crate) fn render_object_slots(
    ui: &ig::Ui<'_>,
    id: &str,
    behaviors: &[Option<ObjectBehavior>],
    behavior_name: impl Fn(&ObjectBehavior) -> String,
) -> Option<ObjectSlot> {
    let id_token = ui.push_id(id);

    let button_size = 50.0;
    let window_left = ui.window_pos()[0];
    let window_right = window_left + ui.window_content_region_max()[0];
    let mut prev_item_right = window_left;
    let style = ui.clone_style();

    let mut result = None;

    for (slot_index, behavior) in behaviors.iter().enumerate() {
        let item_right = prev_item_right + style.item_spacing[0] + button_size;
        if item_right > window_right {
            prev_item_right = window_left;
        } else if slot_index != 0 {
            ui.same_line(0.0);
        }
        prev_item_right = prev_item_right + style.item_spacing[0] + button_size;

        let label = match behavior {
            Some(behavior) => format!("{}\n{}", slot_index, behavior_name(behavior)),
            None => format!("{}", slot_index),
        };

        if ui.button(
            &im_str!("{}##slot-{}", label, slot_index),
            [button_size, button_size],
        ) {
            result = Some(ObjectSlot(slot_index));
        }
    }

    id_token.pop(ui);
    result
}
