use fast3d::interpret::F3DRenderData;
use ultraviolet::Vec3;
use wafel_data_access::MemoryLayout;
use wafel_memory::MemoryRead;
use wafel_sm64::{read_surfaces, Surface, SurfaceType};

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
    pub(crate) wall_hitbox_vertices: Vec<ColorVertex>,
    pub(crate) wall_hitbox_outline_vertices: Vec<ColorVertex>,
}

impl VizRenderData {
    pub fn new(screen_top_left: [u32; 2], screen_size: [u32; 2]) -> Self {
        F3DRenderData::new(screen_top_left, screen_size).into()
    }
}

impl From<F3DRenderData> for VizRenderData {
    fn from(f3d_render_data: F3DRenderData) -> Self {
        Self {
            f3d_render_data,
            render_output: None,
            elements: Vec::new(),
            surface_vertices: Vec::new(),
            transparent_surface_vertices: Vec::new(),
            wall_hitbox_vertices: Vec::new(),
            wall_hitbox_outline_vertices: Vec::new(),
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

    let mut render_data = VizRenderData::from(f3d_render_data);
    render_data.elements = config.elements.clone();
    draw_extras(&mut render_data, layout, memory, config, &render_output)?;
    render_data.render_output = Some(render_output);

    Ok(render_data)
}

fn draw_extras(
    r: &mut VizRenderData,
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
    render_output: &GfxRenderOutput,
) -> Result<(), VizError> {
    draw_surfaces(r, layout, memory, config, render_output)?;
    draw_camera_focus(r, layout, memory, config)?;
    Ok(())
}

fn draw_surfaces(
    r: &mut VizRenderData,
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
    render_output: &GfxRenderOutput,
) -> Result<(), VizError> {
    if config.surface_mode != SurfaceMode::Physical {
        return Ok(());
    }

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
                r.transparent_surface_vertices.push(vertex);
            } else {
                r.surface_vertices.push(vertex);
            }
        }
    }

    draw_wall_hitboxes(r, config, &surfaces, render_output)?;

    Ok(())
}

fn draw_wall_hitboxes(
    r: &mut VizRenderData,
    config: &VizConfig,
    surfaces: &[Surface],
    render_output: &GfxRenderOutput,
) -> Result<(), VizError> {
    if config.wall_hitbox_radius <= 0.0 {
        return Ok(());
    }

    for (i, surface) in surfaces.iter().enumerate() {
        if config.transparent_surfaces.contains(&i) {
            continue;
        }

        let proj_dir: Vec3;
        let color: [f32; 4];
        match surface.ty() {
            SurfaceType::Floor => continue,
            SurfaceType::Ceiling => continue,
            SurfaceType::WallXProj => {
                proj_dir = Vec3::unit_x();
                color = [0.3, 0.8, 0.3, 0.4];
            }
            SurfaceType::WallZProj => {
                proj_dir = Vec3::unit_z();
                color = [0.15, 0.4, 0.15, 0.4];
            }
        };
        let outline_color = [0.0, 0.0, 0.0, 0.5];

        let surface_normal = Vec3::from(surface.normal);
        let proj_dist = config.wall_hitbox_radius / surface_normal.dot(proj_dir);

        let wall_vertices = surface.vertices.map(|v| Vec3::from(v.map(|t| t as f32)));
        let ext_vertices = [
            wall_vertices[0] + proj_dist * proj_dir,
            wall_vertices[1] + proj_dist * proj_dir,
            wall_vertices[2] + proj_dist * proj_dir,
        ];
        let int_vertices = [
            wall_vertices[0] - proj_dist * proj_dir,
            wall_vertices[1] - proj_dist * proj_dir,
            wall_vertices[2] - proj_dist * proj_dir,
        ];

        r.wall_hitbox_vertices.extend(triangle(ext_vertices, color));
        r.wall_hitbox_vertices.extend(triangle(int_vertices, color));

        r.wall_hitbox_outline_vertices
            .extend(triangle(ext_vertices, outline_color));
        r.wall_hitbox_outline_vertices
            .extend(triangle(int_vertices, outline_color));

        let camera_dist = match &render_output.used_camera {
            Some(Camera::LookAt(camera)) => {
                let camera_pos = Vec3::from(camera.pos);
                (int_vertices[0] - camera_pos).mag()
            }
            _ => 1000.0,
        };

        for i0 in 0..3 {
            let i1 = (i0 + 1) % 3;

            // Bump slightly inward. This prevents flickering with floors and adjacent
            // walls
            let mut bump = 0.1 * camera_dist / 1000.0;
            if surface.ty() == SurfaceType::WallZProj {
                bump *= 2.0; // Avoid flickering between x and z projected wall hitboxes
            }

            let vertices = [int_vertices[i0], int_vertices[i1], ext_vertices[i0]];
            let normal = (vertices[1] - vertices[0])
                .cross(vertices[2] - vertices[0])
                .normalized();
            for vertex in vertices {
                r.wall_hitbox_vertices
                    .push(ColorVertex::new(point(vertex - bump * normal), color));
            }

            let vertices = [ext_vertices[i0], int_vertices[i1], ext_vertices[i1]];
            let normal = (vertices[1] - vertices[0])
                .cross(vertices[2] - vertices[0])
                .normalized();
            for vertex in vertices {
                r.wall_hitbox_vertices
                    .push(ColorVertex::new(point(vertex - bump * normal), color));
            }

            r.wall_hitbox_outline_vertices.extend(&[
                ColorVertex::new(point(int_vertices[i0]), outline_color),
                ColorVertex::new(point(ext_vertices[i0]), outline_color),
            ]);
            r.wall_hitbox_outline_vertices.extend(&[
                ColorVertex::new(point(int_vertices[i0]), outline_color),
                ColorVertex::new(point(int_vertices[i1]), outline_color),
            ]);
            r.wall_hitbox_outline_vertices.extend(&[
                ColorVertex::new(point(ext_vertices[i0]), outline_color),
                ColorVertex::new(point(ext_vertices[i1]), outline_color),
            ]);
        }
    }

    Ok(())
}

fn triangle(vertices: [Vec3; 3], color: [f32; 4]) -> [ColorVertex; 3] {
    vertices.map(|v| ColorVertex::new(point(v), color))
}

fn point(v: Vec3) -> [f32; 4] {
    *v.into_homogeneous_point().as_array()
}

fn draw_camera_focus(
    r: &mut VizRenderData,
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    config: &VizConfig,
) -> Result<(), VizError> {
    if !config.show_camera_focus {
        return Ok(());
    }

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
    r.elements.push(Element::Point(Point {
        pos: focus,
        size: 4.0,
        color,
    }));
    if show_line {
        r.elements.push(Element::Line(Line {
            vertices: [focus, [focus[0], focus[1] - 10_000.0, focus[2]]],
            color,
        }));
    }

    Ok(())
}
