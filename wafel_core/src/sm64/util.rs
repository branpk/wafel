use graphics::scene::{self, Scene};
use wafel_api::Timeline;

use super::{ObjectBehavior, ObjectSlot, SM64ErrorCause, SurfaceSlot};
use crate::{
    error::Error,
    geo::Point3f,
    geo::Vector3f,
    graphics,
    memory::Memory,
    timeline::{SlotState, State},
};
use std::collections::HashMap;
use wafel_data_type::{FloatType, IntType, IntValue, Value};
use wafel_layout::ConstantSource;

/// Get the data path for an object, or None if the object is inactive.
pub fn object_path(
    timeline: &Timeline,
    frame: u32,
    object: ObjectSlot,
) -> Result<Option<String>, Error> {
    let active_flags = timeline
        .try_read(frame, &format!("gObjectPool[{}].activeFlags", object.0))?
        .try_as_int()?;

    if active_flags == 0 {
        return Ok(None);
    }

    Ok(Some(format!("gObjectPool[{}]", object.0)))
}

pub fn concat_object_path(object_path: &str, field_path: &str) -> String {
    let path_suffix = field_path
        .strip_prefix("struct Object")
        .expect("invalid object field path");
    format!("{}{}", object_path, path_suffix)
}

/// Get the behavior address for an object.
pub fn object_behavior(
    timeline: &Timeline,
    frame: u32,
    object_path: &str,
) -> Result<ObjectBehavior, Error> {
    let behavior_path = concat_object_path(object_path, "struct Object.behavior");
    let behavior_address = timeline.try_read(frame, &behavior_path)?.try_as_address()?;
    Ok(ObjectBehavior(behavior_address))
}

/// Get the data path for a surface, or None if the surface is inactive.
pub fn surface_path(
    timeline: &Timeline,
    frame: u32,
    surface: SurfaceSlot,
) -> Result<Option<String>, Error> {
    let num_surfaces = timeline
        .try_read(frame, "gSurfacesAllocated")?
        .try_as_usize()?;
    if surface.0 >= num_surfaces {
        return Ok(None);
    }
    Ok(Some(format!("sSurfacePool[{}]", surface)))
}

pub fn concat_surface_path(surface_path: &str, field_path: &str) -> String {
    let path_suffix = field_path
        .strip_prefix("struct Surface")
        .expect("invalid surface field path");
    format!("{}{}", surface_path, path_suffix)
}

#[derive(Debug, Clone)]
struct Surface {
    normal: [f32; 3],
    vertices: [[i16; 3]; 3],
}

fn read_surfaces(state: &impl SlotState) -> Result<Vec<Surface>, Error> {
    let memory = state.memory();

    let surface_pool_addr = state.read("sSurfacePool?")?;
    if surface_pool_addr.is_none() {
        return Ok(Vec::new());
    }
    let surface_pool_addr = surface_pool_addr.try_as_address()?;

    let surfaces_allocated = state.read("gSurfacesAllocated")?.try_as_int()? as usize;

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

    let active_flag_active = memory.data_layout().constant("ACTIVE_FLAG_ACTIVE")?.value as i16;

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
