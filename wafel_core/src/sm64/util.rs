use graphics::scene::{self, Scene};
use wafel_api::Timeline;

use super::{ObjectBehavior, ObjectSlot, SurfaceSlot};
use crate::{error::Error, geo::Point3f, geo::Vector3f, graphics};

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

/// Load the SM64 surfaces from the game state and add them to the scene.
pub fn read_surfaces_to_scene(
    scene: &mut Scene,
    timeline: &Timeline,
    frame: u32,
) -> Result<(), Error> {
    scene.surfaces = timeline
        .try_surfaces(frame)?
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
pub fn read_objects_to_scene(
    scene: &mut Scene,
    timeline: &Timeline,
    frame: u32,
) -> Result<(), Error> {
    for object in timeline.try_object_hitboxes(frame)? {
        scene.objects.push(scene::Object {
            pos: Point3f::new(object.pos[0], object.pos[1], object.pos[2]).into(),
            hitbox_height: object.hitbox_height,
            hitbox_radius: object.hitbox_radius,
        });
    }

    Ok(())
}

/// Trace a ray until it hits a surface, and return the surface's index in the surface pool.
pub fn trace_ray_to_surface(
    timeline: &Timeline,
    frame: u32,
    ray: (Point3f, Vector3f),
) -> Result<Option<(usize, Point3f)>, Error> {
    let mut nearest: Option<(f32, (usize, Point3f))> = None;

    let vertex_to_vec = |p: [i16; 3]| Point3f::new(p[0] as f32, p[1] as f32, p[2] as f32);

    let surfaces = timeline.try_surfaces(frame)?;
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
