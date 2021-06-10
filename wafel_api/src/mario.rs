use std::collections::HashMap;

use wafel_layout::DataLayout;

/// Return a mapping from Mario action values to their name (e.g. `ACT_IDLE`).
pub(crate) fn mario_action_names(layout: &DataLayout) -> HashMap<u32, String> {
    layout
        .constants
        .iter()
        .filter(|(name, _)| {
            name.starts_with("ACT_")
                && !name.starts_with("ACT_FLAG_")
                && !name.starts_with("ACT_GROUP_")
                && !name.starts_with("ACT_ID_")
        })
        .map(|(name, constant)| (constant.value as u32, name.clone()))
        .collect()
}
