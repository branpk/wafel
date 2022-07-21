use wafel_data_access::{DataReadable, MemoryLayout};
use wafel_memory::MemoryRead;

use crate::Error;

/// Hitbox information for an SM64 object.
#[derive(Debug, Clone)]
pub struct ObjectHitbox {
    /// The object's position (oPosX, oPosY, oPosZ).
    pub pos: [f32; 3],
    /// The object's hitbox height (hitboxHeight).
    pub hitbox_height: f32,
    /// The object's hitbox radius (hitboxRadius).
    pub hitbox_radius: f32,
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("Object")]
struct SM64ObjectFields {
    active_flags: i16,
    o_pos_x: f32,
    o_pos_y: f32,
    o_pos_z: f32,
    hitbox_height: f32,
    hitbox_radius: f32,
}

pub(crate) fn read_object_hitboxes(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
) -> Result<Vec<ObjectHitbox>, Error> {
    let object_pool_addr = layout.global_path("gObjectPool")?.address(memory)?.unwrap();

    let object_size = layout
        .global_path("gObjectPool")?
        .concrete_type()
        .stride()
        .ok()
        .flatten()
        .ok_or(Error::UnsizedObjectPoolArray)?;

    let reader = SM64ObjectFields::reader(layout)?;
    let active_flag_active = layout.data_layout().constant("ACTIVE_FLAG_ACTIVE")?.value as i16;

    let mut objects = Vec::new();
    for slot in 0..240 {
        let object_addr = object_pool_addr + slot * object_size;
        let fields = reader.read(memory, object_addr)?;

        if (fields.active_flags & active_flag_active) != 0 {
            objects.push(ObjectHitbox {
                pos: [fields.o_pos_x, fields.o_pos_y, fields.o_pos_z],
                hitbox_height: fields.hitbox_height,
                hitbox_radius: fields.hitbox_radius,
            })
        }
    }

    Ok(objects)
}
