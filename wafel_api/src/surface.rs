use wafel_data_access::{DataReadable, MemoryLayout};
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

#[derive(Debug, Clone, DataReadable)]
#[struct_name("Surface")]
struct SM64Surface {
    normal: [f32; 3],
    vertex1: [i16; 3],
    vertex2: [i16; 3],
    vertex3: [i16; 3],
}

pub(crate) fn read_surfaces(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
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

    let reader = SM64Surface::reader(layout)?;

    let mut surfaces = Vec::new();
    for index in 0..surfaces_allocated {
        let surface_addr = surface_pool_addr + index * surface_size;
        let surface = reader.read(memory, surface_addr)?;

        surfaces.push(Surface {
            normal: surface.normal,
            vertices: [surface.vertex1, surface.vertex2, surface.vertex3],
        });
    }

    Ok(surfaces)
}
