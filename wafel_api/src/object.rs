use wafel_data_type::{FloatType, IntType};
use wafel_layout::DataLayout;
use wafel_memory::{MemoryRead, SymbolLookup};

use crate::{data_path_cache::DataPathCache, Error};

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

pub(crate) fn read_object_hitboxes<S: SymbolLookup>(
    memory: &impl MemoryRead,
    layout: &DataLayout,
    data_path_cache: &DataPathCache<S>,
) -> Result<Vec<ObjectHitbox>, Error> {
    let object_pool_addr = data_path_cache
        .global("gObjectPool")?
        .address(memory)?
        .unwrap();

    let object_size = data_path_cache
        .global("gObjectPool")?
        .concrete_type()
        .stride()
        .ok()
        .flatten()
        .ok_or(Error::UnsizedObjectPoolArray)?;

    let offset =
        |path| -> Result<usize, Error> { Ok(data_path_cache.local(path)?.field_offset()?) };
    let o_active_flags = offset("struct Object.activeFlags")?;
    let o_pos_x = offset("struct Object.oPosX")?;
    let o_pos_y = offset("struct Object.oPosY")?;
    let o_pos_z = offset("struct Object.oPosZ")?;
    let o_hitbox_height = offset("struct Object.hitboxHeight")?;
    let o_hitbox_radius = offset("struct Object.hitboxRadius")?;

    let active_flag_active = layout.constant("ACTIVE_FLAG_ACTIVE")?.value as i16;

    let read_f32 = |address| -> Result<f32, Error> {
        let result = memory.read_float(address, FloatType::F32)? as f32;
        Ok(result)
    };

    let read_s16 = |address| -> Result<i16, Error> {
        let result = memory.read_int(address, IntType::S16)? as i16;
        Ok(result)
    };

    let mut objects = Vec::new();
    for slot in 0..240 {
        let object_addr = object_pool_addr + slot * object_size;

        let active_flags = read_s16(object_addr + o_active_flags)?;
        if (active_flags & active_flag_active) != 0 {
            objects.push(ObjectHitbox {
                pos: [
                    read_f32(object_addr + o_pos_x)?,
                    read_f32(object_addr + o_pos_y)?,
                    read_f32(object_addr + o_pos_z)?,
                ],
                hitbox_height: read_f32(object_addr + o_hitbox_height)?,
                hitbox_radius: read_f32(object_addr + o_hitbox_radius)?,
            })
        }
    }

    Ok(objects)
}
