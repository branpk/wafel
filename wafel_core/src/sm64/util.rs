use graphics::scene::{self, Scene};

use super::{ObjectBehavior, ObjectSlot, SM64ErrorCause, SurfaceSlot};
use crate::{
    data_path::GlobalDataPath,
    error::Error,
    geo::Point3f,
    geo::Vector3f,
    graphics,
    memory::{ConstantSource, IntValue, Memory, Value},
    timeline::{SlotState, State},
};
use std::collections::HashMap;
use wafel_types::{FloatType, IntType};

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
            let event_type = event_types.get(&event_type_value).ok_or({
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

#[derive(Debug, Clone)]
struct Surface {
    normal: [f32; 3],
    vertices: [[i16; 3]; 3],
}

fn read_surfaces(state: &impl SlotState) -> Result<Vec<Surface>, Error> {
    let memory = state.memory();

    let surface_pool_addr = state.read("sSurfacePool?")?;
    if surface_pool_addr.is_null() {
        return Ok(Vec::new());
    }
    let surface_pool_addr = surface_pool_addr.as_address()?;

    let surfaces_allocated = state.read("gSurfacesAllocated")?.as_int()? as usize;

    let surface_size = memory
        .global_path("sSurfacePool")?
        .concrete_type()
        .stride()
        .ok()
        .flatten()
        .ok_or(SM64ErrorCause::UnsizedSurfacePoolPointer)?;

    let offset = |path| -> Result<usize, Error> { memory.local_path(path)?.field_offset() };
    let o_normal = offset("struct Surface.normal")?;
    let o_vertex1 = offset("struct Surface.vertex1")?;
    let o_vertex2 = offset("struct Surface.vertex2")?;
    let o_vertex3 = offset("struct Surface.vertex3")?;

    let read_f32 = |address| -> Result<f32, Error> {
        let classified_address = memory.classify_address(&address);
        let result = memory.read_float(state.slot(), &classified_address, FloatType::F32)? as f32;
        Ok(result)
    };
    let read_f32_3 = |address| -> Result<[f32; 3], Error> {
        Ok([
            read_f32(address)?,
            read_f32(address + 4)?,
            read_f32(address + 8)?,
        ])
    };

    let read_s16 = |address| -> Result<i16, Error> {
        let classified_address = memory.classify_address(&address);
        let result = memory.read_int(state.slot(), &classified_address, IntType::S16)? as i16;
        Ok(result)
    };
    let read_s16_3 = |address| -> Result<[i16; 3], Error> {
        Ok([
            read_s16(address)?,
            read_s16(address + 2)?,
            read_s16(address + 4)?,
        ])
    };

    let mut surfaces = Vec::new();
    for index in 0..surfaces_allocated {
        let surface_addr = surface_pool_addr + index * surface_size;

        let normal = read_f32_3(surface_addr + o_normal)?;
        let vertex1 = read_s16_3(surface_addr + o_vertex1)?;
        let vertex2 = read_s16_3(surface_addr + o_vertex2)?;
        let vertex3 = read_s16_3(surface_addr + o_vertex3)?;

        surfaces.push(Surface {
            normal,
            vertices: [vertex1, vertex2, vertex3],
        });
    }

    Ok(surfaces)
}

/// Load the SM64 surfaces from the game state and add them to the scene.
pub fn read_surfaces_to_scene(scene: &mut Scene, state: &impl SlotState) -> Result<(), Error> {
    scene.surfaces = read_surfaces(state)?
        .iter()
        .map(|surface| {
            let ty = if surface.normal[1] > 0.01 {
                scene::SurfaceType::Floor
            } else if surface.normal[1] < -0.01 {
                scene::SurfaceType::Ceiling
            } else if surface.normal[0] < -0.707 || surface.normal[0] > 0.707 {
                scene::SurfaceType::WallXProj
            } else {
                scene::SurfaceType::WallZProj
            };

            let as_point = |p: [i16; 3]| Point3f::new(p[0] as f32, p[1] as f32, p[2] as f32);
            scene::Surface {
                ty,
                vertices: [
                    as_point(surface.vertices[0]).into(),
                    as_point(surface.vertices[1]).into(),
                    as_point(surface.vertices[2]).into(),
                ],
                normal: Vector3f::from_row_slice(&surface.normal).into(),
            }
        })
        .collect();

    Ok(())
}

/// Load the SM64 objects from the game state and add them to the scene.
pub fn read_objects_to_scene(scene: &mut Scene, state: &impl SlotState) -> Result<(), Error> {
    let memory = state.memory();
    let object_pool_addr = state.address("gObjectPool")?.unwrap();

    let object_size = memory
        .global_path("gObjectPool")?
        .concrete_type()
        .stride()
        .ok()
        .flatten()
        .ok_or(SM64ErrorCause::UnsizedObjectPoolArray)?;

    let offset = |path| -> Result<usize, Error> { memory.local_path(path)?.field_offset() };
    let o_active_flags = offset("struct Object.activeFlags")?;
    let o_pos_x = offset("struct Object.oPosX")?;
    let o_pos_y = offset("struct Object.oPosY")?;
    let o_pos_z = offset("struct Object.oPosZ")?;
    let o_hitbox_height = offset("struct Object.hitboxHeight")?;
    let o_hitbox_radius = offset("struct Object.hitboxRadius")?;

    let active_flag_active = memory
        .data_layout()
        .get_constant("ACTIVE_FLAG_ACTIVE")?
        .value as i16;

    let read_f32 = |address| -> Result<f32, Error> {
        let classified_address = memory.classify_address(&address);
        let result = memory.read_float(state.slot(), &classified_address, FloatType::F32)? as f32;
        Ok(result)
    };

    let read_s16 = |address| -> Result<i16, Error> {
        let classified_address = memory.classify_address(&address);
        let result = memory.read_int(state.slot(), &classified_address, IntType::S16)? as i16;
        Ok(result)
    };

    for slot in 0..240 {
        let object_addr = object_pool_addr + slot * object_size;

        let active_flags = read_s16(object_addr + o_active_flags)?;
        if (active_flags & active_flag_active) != 0 {
            scene.objects.push(scene::Object {
                pos: Point3f::new(
                    read_f32(object_addr + o_pos_x)?,
                    read_f32(object_addr + o_pos_y)?,
                    read_f32(object_addr + o_pos_z)?,
                )
                .into(),
                hitbox_height: read_f32(object_addr + o_hitbox_height)?,
                hitbox_radius: read_f32(object_addr + o_hitbox_radius)?,
            })
        }
    }

    Ok(())
}

/// Trace a ray until it hits a surface, and return the surface's index in the surface pool.
pub fn trace_ray_to_surface(
    state: &impl SlotState,
    ray: (Point3f, Vector3f),
) -> Result<Option<(usize, Point3f)>, Error> {
    let mut nearest: Option<(f32, (usize, Point3f))> = None;

    let vertex_to_vec = |p: [i16; 3]| Point3f::new(p[0] as f32, p[1] as f32, p[2] as f32);

    let surfaces = read_surfaces(state)?;
    for (i, surface) in surfaces.iter().enumerate() {
        let normal = Vector3f::from_row_slice(&surface.normal);
        let vertices = [
            vertex_to_vec(surface.vertices[0]),
            vertex_to_vec(surface.vertices[1]),
            vertex_to_vec(surface.vertices[2]),
        ];

        let t = -normal.dot(&(ray.0 - vertices[0])) / normal.dot(&ray.1);
        if t <= 0.0 {
            continue;
        }

        let p = ray.0 + t * ray.1;

        let mut interior = true;
        for k in 0..3 {
            let edge = vertices[(k + 1) % 3] - vertices[k];
            if normal.dot(&edge.cross(&(p - vertices[k]))) < 0.0 {
                interior = false;
                break;
            }
        }
        if !interior {
            continue;
        }

        if nearest.is_none() || t < nearest.unwrap().0 {
            nearest = Some((t, (i, p)));
        }
    }

    Ok(nearest.map(|(_, result)| result))
}
