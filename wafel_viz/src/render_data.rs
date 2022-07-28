use fast3d::interpret::F3DRenderData;
use wafel_data_access::MemoryLayout;
use wafel_memory::MemoryRead;

use crate::{
    sm64_gfx_render::{sm64_gfx_render, GfxRenderOutput},
    Element, VizConfig, VizError,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VizRenderData {
    pub(crate) f3d_render_data: F3DRenderData,
    pub(crate) render_output: GfxRenderOutput,
    pub(crate) elements: Vec<Element>,
}

pub fn sm64_render(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
) -> Result<F3DRenderData, VizError> {
    let (f3d_render_data, _) = sm64_gfx_render(layout, memory, &VizConfig::default(), false)?;
    Ok(f3d_render_data)
}

pub fn viz_render(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
) -> Result<VizRenderData, VizError> {
    let (f3d_render_data, render_output) = sm64_gfx_render(layout, memory, config, true)?;
    Ok(VizRenderData {
        f3d_render_data,
        render_output,
        elements: config.elements.clone(),
    })
}
