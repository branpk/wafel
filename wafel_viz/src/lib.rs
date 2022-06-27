#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

use std::error::Error;

use render_api::{update_and_render_with_backend, RenderBackend, ShaderId, ShaderInfo};
use sm64_render_data::sm64_update_and_render;
use wafel_memory::DllGameMemory;

mod render_api;
mod sm64_render_data;

pub fn test() -> Result<(), Box<dyn Error>> {
    let (memory, mut base_slot) = unsafe {
        DllGameMemory::load(
            "../libsm64-build/build/us_lib/sm64_us.dll",
            "sm64_init",
            "sm64_update",
        )?
    };

    let render_data = sm64_update_and_render(&memory, &mut base_slot, 640, 480)?;

    Ok(())
}
