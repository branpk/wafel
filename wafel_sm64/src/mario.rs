use std::collections::HashMap;

use wafel_data_access::MemoryLayout;

/// Return a mapping from Mario action values to their name (e.g. `ACT_IDLE`).
pub fn mario_action_names(layout: &impl MemoryLayout) -> HashMap<u32, String> {
    layout
        .data_layout()
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
