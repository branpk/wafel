use std::collections::HashMap;

use wafel_data_type::{IntValue, Value};
use wafel_layout::{ConstantSource, DataLayout};
use wafel_memory::MemoryRead;

use crate::{data_path_cache::DataPathCache, Error};

// TODO: Parse frame log into data structure

pub(crate) fn read_frame_log(
    memory: &impl MemoryRead,
    layout: &DataLayout,
    data_path_cache: &DataPathCache,
) -> Result<Vec<HashMap<String, Value>>, Error> {
    let event_type_source = ConstantSource::Enum {
        name: Some("FrameLogEventType".to_owned()),
    };
    let event_types: HashMap<IntValue, String> = layout
        .constants
        .iter()
        .filter(|(_, constant)| constant.source == event_type_source)
        .map(|(name, constant)| (constant.value, name.clone()))
        .collect();

    let log_length = data_path_cache
        .get("gFrameLogLength")?
        .read(memory)?
        .try_as_usize()?;

    (0..log_length)
        .map(|i| -> Result<_, Error> {
            let event_type_value = data_path_cache
                .get(&format!("gFrameLog[{}].type", i))?
                .read(memory)?
                .try_as_int()?;
            let event_type = event_types
                .get(&event_type_value)
                .ok_or(Error::InvalidFrameLogEventType(event_type_value))?;

            let variant_name = frame_log_event_variant_name(event_type);
            let mut event = data_path_cache
                .get(&format!("gFrameLog[{}].__anon.{}", i, variant_name))?
                .read(memory)?
                .try_as_struct()?
                .clone();

            event.insert("type".to_owned(), Value::String(event_type.clone()));
            Ok(event)
        })
        .collect()
}

/// Convert a frame log event type to the variant name corresponding to its data.
///
/// For example, `FLT_BEGIN_MOVEMENT_STEP` maps to `beginMovementStep`.
fn frame_log_event_variant_name(event_type: &str) -> String {
    let mut name = event_type.to_ascii_lowercase();
    name = name.strip_prefix("flt_").unwrap_or(&name).to_owned();
    name.split('_')
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_owned()
            } else {
                // Capitalize the segment
                let mut chars = part.chars();
                match chars.next() {
                    Some(c) => format!("{}{}", c.to_ascii_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            }
        })
        .collect()
}
