use fast3d::interpret::F3DRenderData;
use wafel_data_access::MemoryLayout;
use wafel_memory::MemoryRead;
use wafel_sm64::{read_surfaces, SurfaceType};

use crate::{
    sm64_gfx_render::{sm64_gfx_render, GfxRenderOutput},
    ColorVertex, Element, SurfaceMode, VizConfig, VizError,
};

#[derive(Debug, Clone, PartialEq)]
pub struct VizRenderData {
    pub(crate) f3d_render_data: F3DRenderData,
    pub(crate) render_output: Option<GfxRenderOutput>,
    pub(crate) elements: Vec<Element>,
    pub(crate) surface_vertices: Vec<ColorVertex>,
}

impl From<F3DRenderData> for VizRenderData {
    fn from(data: F3DRenderData) -> Self {
        Self {
            f3d_render_data: data,
            render_output: None,
            elements: Vec::new(),
            surface_vertices: Vec::new(),
        }
    }
}

pub fn sm64_render(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    screen_top_left: [u32; 2],
    screen_size: [u32; 2],
) -> Result<F3DRenderData, VizError> {
    let config = VizConfig {
        screen_top_left,
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

    let surface_vertices = if config.surface_mode == SurfaceMode::Physical {
        get_surface_vertices(layout, memory)?
    } else {
        Vec::new()
    };

    Ok(VizRenderData {
        f3d_render_data,
        render_output: Some(render_output),
        elements: config.elements.clone(),
        surface_vertices,
    })
}

fn get_surface_vertices(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
) -> Result<Vec<ColorVertex>, VizError> {
    let mut vertices: Vec<ColorVertex> = Vec::new();

    let surfaces = read_surfaces(layout, memory)?;

    for surface in &surfaces {
        let color = match surface.ty() {
            SurfaceType::Floor => [0.5, 0.5, 1.0, 1.0],
            SurfaceType::Ceiling => [1.0, 0.5, 0.5, 1.0],
            SurfaceType::WallXProj => [0.3, 0.8, 0.3, 1.0],
            SurfaceType::WallZProj => [0.15, 0.4, 0.15, 1.0],
        };

        for pos in &surface.vertices {
            vertices.push(ColorVertex {
                pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
                color,
            });
        }
    }

    Ok(vertices)
}
