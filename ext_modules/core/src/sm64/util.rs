use super::{ObjectBehavior, ObjectSlot, SM64ErrorCause, SurfaceSlot};
use crate::{data_path::GlobalDataPath, error::Error, memory::Memory, timeline::State};

/// Get the data path for an object.
pub fn object_path(state: &impl State, object: ObjectSlot) -> Result<GlobalDataPath, Error> {
    try_object_path(state, object)?.ok_or_else(|| SM64ErrorCause::InactiveObject { object }.into())
}

/// Get the data path for an object, or None if the object is inactive.
///
/// This is faster than `object_path` if inactive objects are common.
pub fn try_object_path(
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

/// Get the data path for a surface.
pub fn surface_path(state: &impl State, surface: SurfaceSlot) -> Result<GlobalDataPath, Error> {
    let num_surfaces = state.read("gSurfacesAllocated")?.as_int()? as usize;
    if surface.0 >= num_surfaces {
        Err(SM64ErrorCause::InactiveSurface { surface })?;
    }
    let surface_path = state
        .memory()
        .global_path(&format!("sSurfacePool[{}]", surface))?;
    Ok(surface_path)
}
