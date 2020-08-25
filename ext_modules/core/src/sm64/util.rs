use super::{ObjectBehavior, ObjectSlot, SM64ErrorCause, SurfaceSlot};
use crate::{
    data_path::GlobalDataPath,
    error::Error,
    memory::{ConstantSource, IntValue, Memory, Value},
    timeline::State,
};
use std::collections::HashMap;

/// Get the data path for an object, or None if the object is inactive.
pub fn object_path(
    state: &impl State,
    object: ObjectSlot,
) -> Result<Option<GlobalDataPath>, Error> {
    let active_flags = state
        .read(&format!("gObjectPool[{}].activeFlags", object.0))?
        .as_int()?;

    if active_flags == 0 {
        return Ok(None);
    }

    let object_path = state
        .memory()
        .global_path(&format!("gObjectPool[{}]", object.0))?;

    Ok(Some(object_path))
}

/// Get the behavior address for an object.
pub fn object_behavior(
    state: &impl State,
    object_path: &GlobalDataPath,
) -> Result<ObjectBehavior, Error> {
    let behavior_path =
        object_path.concat(&state.memory().local_path("struct Object.behavior")?)?;
    let behavior_address = state.path_read(&behavior_path)?.as_address()?;
    Ok(ObjectBehavior(behavior_address))
}

/// Get the data path for a surface, or None if the surface is inactive.
pub fn surface_path(
    state: &impl State,
    surface: SurfaceSlot,
) -> Result<Option<GlobalDataPath>, Error> {
    let num_surfaces = state.read("gSurfacesAllocated")?.as_usize()?;
    if surface.0 >= num_surfaces {
        return Ok(None);
    }
    let surface_path = state
        .memory()
        .global_path(&format!("sSurfacePool[{}]", surface))?;
    Ok(Some(surface_path))
}

/// Get the wafel frame log.
///
/// The events in the frame log occurred on the frame leading to `state`.
pub fn frame_log(state: &impl State) -> Result<Vec<HashMap<String, Value>>, Error> {
    let event_type_source = ConstantSource::Enum {
        name: Some("FrameLogEventType".to_owned()),
    };
    let event_types: HashMap<IntValue, String> = state
        .memory()
        .data_layout()
        .constants
        .iter()
        .filter(|(_, constant)| constant.source == event_type_source)
        .map(|(name, constant)| (constant.value, name.clone()))
        .collect();

    let log_length = state.read("gFrameLogLength")?.as_usize()?;

    (0..log_length)
        .map(|i| -> Result<_, Error> {
            let event_type_value = state.read(&format!("gFrameLog[{}].type", i))?.as_int()?;
            let event_type = event_types.get(&event_type_value).ok_or_else(|| {
                SM64ErrorCause::InvalidFrameLogEventType {
                    value: event_type_value,
                }
            })?;

            let variant_name = frame_log_event_variant_name(event_type);
            let mut event = state
                .read(&format!("gFrameLog[{}].__anon.{}", i, variant_name))?
                .as_struct()?
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
    name.split("_")
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
