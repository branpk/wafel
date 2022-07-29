use fast3d::interpret::F3DRenderData;
use wafel_data_access::MemoryLayout;
use wafel_memory::MemoryRead;

use crate::{
    sm64_gfx_render::{sm64_gfx_render, GfxRenderOutput},
    Element, VizConfig, VizError,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VizRenderData {
    pub(crate) screen_size: [u32; 2],
    pub(crate) f3d_render_data: F3DRenderData,
    pub(crate) render_output: GfxRenderOutput,
    pub(crate) elements: Vec<Element>,
}

pub fn sm64_render(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    screen_size: [u32; 2],
) -> Result<F3DRenderData, VizError> {
    let config = VizConfig {
        screen_size,
        ..Default::default()
    };
    let (f3d_render_data, _) = sm64_gfx_render(layout, memory, &config, false)?;
    Ok(f3d_render_data)
}

pub fn viz_render(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
) -> Result<VizRenderData, VizError> {
    let (f3d_render_data, render_output) = sm64_gfx_render(layout, memory, config, true)?;
    Ok(VizRenderData {
        screen_size: config.screen_size,
        f3d_render_data,
        render_output,
        elements: config.elements.clone(),
    })
}
