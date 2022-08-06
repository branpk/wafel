use fast3d::interpret::F3DRenderData;
use wafel_data_access::MemoryLayout;
use wafel_memory::MemoryRead;
use wafel_sm64::{read_surfaces, SurfaceType};

use crate::{
    sm64_gfx_render::{sm64_gfx_render, GfxRenderOutput},
    Camera, ColorVertex, Element, Line, Point, SurfaceMode, VizConfig, VizError,
};

#[derive(Debug, Clone, PartialEq)]
pub struct VizRenderData {
    pub(crate) f3d_render_data: F3DRenderData,
    pub(crate) render_output: Option<GfxRenderOutput>,
    pub(crate) elements: Vec<Element>,
    pub(crate) surface_vertices: Vec<ColorVertex>,
    pub(crate) transparent_surface_vertices: Vec<ColorVertex>,
}

impl From<F3DRenderData> for VizRenderData {
    fn from(data: F3DRenderData) -> Self {
        Self {
            f3d_render_data: data,
            render_output: None,
            elements: Vec::new(),
            surface_vertices: Vec::new(),
            transparent_surface_vertices: Vec::new(),
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

    let (surface_vertices, transparent_surface_vertices) =
        if config.surface_mode == SurfaceMode::Physical {
            get_surface_vertices(layout, memory, config)?
        } else {
            (Vec::new(), Vec::new())
        };

    let mut elements = config.elements.clone();

    if config.show_camera_focus {
        draw_camera_focus(&mut elements, layout, memory, config)?;
    }

    Ok(VizRenderData {
        f3d_render_data,
        render_output: Some(render_output),
        elements,
        surface_vertices,
        transparent_surface_vertices,
    })
}

fn get_surface_vertices(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
) -> Result<(Vec<ColorVertex>, Vec<ColorVertex>), VizError> {
    let mut vertices: Vec<ColorVertex> = Vec::new();
    let mut transparent_vertices: Vec<ColorVertex> = Vec::new();

    let surfaces = read_surfaces(layout, memory)?;

    for (i, surface) in surfaces.iter().enumerate() {
        let transparent = config.transparent_surfaces.contains(&i);
        let highlighted = config.highlighted_surfaces.contains(&i);

        let mut color = match surface.ty() {
            SurfaceType::Floor => [0.5, 0.5, 1.0, 1.0],
            SurfaceType::Ceiling => [1.0, 0.5, 0.5, 1.0],
            SurfaceType::WallXProj => [0.3, 0.8, 0.3, 1.0],
            SurfaceType::WallZProj => [0.15, 0.4, 0.15, 1.0],
        };

        if transparent {
            let scale = 1.5;
            color[0] *= scale;
            color[1] *= scale;
            color[2] *= scale;
            color[3] = if highlighted { 0.1 } else { 0.0 };
        }

        if highlighted {
            let boost = if surface.ty() == SurfaceType::Floor {
                0.08
            } else {
                0.2
            };
            color[0] += boost;
            color[1] += boost;
            color[2] += boost;
        }

        for pos in &surface.vertices {
            let vertex = ColorVertex {
                pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
                color,
            };
            if transparent {
                transparent_vertices.push(vertex);
            } else {
                vertices.push(vertex);
            }
        }
    }

    Ok((vertices, transparent_vertices))
}

fn draw_camera_focus(
    elements: &mut Vec<Element>,
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
) -> Result<(), VizError> {
    let focus;
    let show_line;
    match &config.camera {
        Camera::InGame => {
            focus = layout
                .global_path("gLakituState.focus")?
                .read(memory)?
                .try_as_f32_3()?;
            show_line = true;
        }
        Camera::LookAt(camera) => {
            focus = camera.focus;
            show_line = true;
        }
        Camera::Ortho(camera) => {
            let dist = 1.0;
            focus = [
                camera.pos[0] + camera.forward[0] * dist,
                camera.pos[1] + camera.forward[1] * dist,
                camera.pos[2] + camera.forward[2] * dist,
            ];
            show_line = false;
        }
    };

    let color = [0.2, 0.2, 0.2, 0.8];
    elements.push(Element::Point(Point {
        pos: focus,
        size: 4.0,
        color,
    }));
    if show_line {
        elements.push(Element::Line(Line {
            vertices: [focus, [focus[0], focus[1] - 10_000.0, focus[2]]],
            color,
        }));
    }

    Ok(())
}
