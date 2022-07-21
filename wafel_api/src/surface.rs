use wafel_data_access::MemoryLayout;
use wafel_data_type::{FloatType, IntType};
use wafel_memory::MemoryRead;

use crate::Error;

// TODO: Returns garbage after level is unloaded

/// An SM64 surface (currently missing several fields).
#[derive(Debug, Clone)]
pub struct Surface {
    /// The surface's normal vector.
    pub normal: [f32; 3],
    /// The surface's vertex coordinates.
    pub vertices: [[i16; 3]; 3],
}

pub(crate) fn read_surfaces(
    memory: &impl MemoryRead,
    layout: &impl MemoryLayout,
) -> Result<Vec<Surface>, Error> {
    let surface_pool_addr = layout.global_path("sSurfacePool?")?.read(memory)?;
    if surface_pool_addr.is_none() {
        return Ok(Vec::new());
    }
    let surface_pool_addr = surface_pool_addr.try_as_address()?;

    let surfaces_allocated = layout
        .global_path("gSurfacesAllocated")?
        .read(memory)?
        .try_as_usize()?;

    let surface_size = layout
        .global_path("sSurfacePool")?
        .concrete_type()
        .stride()
        .ok()
        .flatten()
        .ok_or(Error::UnsizedSurfacePoolPointer)?;

    let offset = |path| -> Result<usize, Error> { Ok(layout.local_path(path)?.field_offset()?) };
    let o_normal = offset("struct Surface.normal")?;
    let o_vertex1 = offset("struct Surface.vertex1")?;
    let o_vertex2 = offset("struct Surface.vertex2")?;
    let o_vertex3 = offset("struct Surface.vertex3")?;

    let read_f32 = |address| -> Result<f32, Error> {
        let result = memory.read_float(address, FloatType::F32)? as f32;
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
        let result = memory.read_int(address, IntType::S16)? as i16;
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
