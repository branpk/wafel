use std::{f32::consts::PI, mem::size_of, ops::Range};

use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use fast3d::util::Matrixf;
use wgpu::util::DeviceExt;

use crate::{Element, VizRenderData};

#[derive(Debug, Clone, Copy, PartialEq, Default, Zeroable, Pod)]
#[repr(C)]
pub struct ColorVertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

impl ColorVertex {
    pub fn new(pos: [f32; 4], color: [f32; 4]) -> Self {
        Self { pos, color }
    }

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: vec![
                // pos
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: offset_of!(Self, pos) as u64,
                    shader_location: 0,
                },
                // color
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: offset_of!(Self, color) as u64,
                    shader_location: 1,
                },
            ]
            .leak(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Zeroable, Pod)]
#[repr(C)]
pub struct PointInstance {
    pub center: [f32; 4],
    pub radius: [f32; 2],
    pub color: [f32; 4],
}

impl PointInstance {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: vec![
                // center
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: offset_of!(Self, center) as u64,
                    shader_location: 0,
                },
                // radius
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: offset_of!(Self, radius) as u64,
                    shader_location: 1,
                },
                // color
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: offset_of!(Self, color) as u64,
                    shader_location: 2,
                },
            ]
            .leak(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Zeroable, Pod)]
#[repr(C)]
pub struct PointVertex {
    pub offset: [f32; 2],
}

impl PointVertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: vec![
                // offset
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: offset_of!(Self, offset) as u64,
                    shader_location: 3,
                },
            ]
            .leak(),
        }
    }
}

pub fn point4(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 1.0]
}

#[derive(Debug)]
pub struct StaticData {
    pub transform_bind_group_layout: wgpu::BindGroupLayout,
    pub point_vertex_buffer: (u32, wgpu::Buffer),
}

impl StaticData {
    pub fn create(device: &wgpu::Device) -> Self {
        let transform_bind_group_layout = create_transform_bind_group_layout(device);
        let point_vertex_buffer = create_point_vertex_buffer(device);
        Self {
            transform_bind_group_layout,
            point_vertex_buffer,
        }
    }
}

fn create_transform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            // r_proj
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // r_view
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn create_point_vertex_buffer(device: &wgpu::Device) -> (u32, wgpu::Buffer) {
    let mut vertices: Vec<PointVertex> = Vec::new();

    let num_edges = 12;
    for i in 0..num_edges {
        let a0 = i as f32 / num_edges as f32 * 2.0 * PI;
        let a1 = (i + 1) as f32 / num_edges as f32 * 2.0 * PI;

        vertices.extend(&[
            PointVertex { offset: [0.0, 0.0] },
            PointVertex {
                offset: [a0.cos(), a0.sin()],
            },
            PointVertex {
                offset: [a1.cos(), a1.sin()],
            },
        ]);
    }

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    (vertices.len() as u32, buffer)
}

#[derive(Debug)]
pub struct PerFrameData {
    pub screen_top_left: [u32; 2],
    pub screen_size: [u32; 2],
    pub f3d_pre_depth_cmd_range: Range<usize>,
    pub f3d_depth_cmd_range: Range<usize>,
    pub f3d_post_depth_cmd_range: Range<usize>,
    pub transform_bind_group: wgpu::BindGroup,
    pub line_vertex_buffer: (u32, wgpu::Buffer),
    pub point_instance_buffer: (u32, wgpu::Buffer),
    pub surface_vertex_buffer: (u32, wgpu::Buffer),
    pub transparent_surface_vertex_buffer: (u32, wgpu::Buffer),
    pub wall_hitbox_vertex_buffer: (u32, wgpu::Buffer),
    pub wall_hitbox_outline_vertex_buffer: (u32, wgpu::Buffer),
}

impl PerFrameData {
    pub fn create(device: &wgpu::Device, static_data: &StaticData, data: &VizRenderData) -> Self {
        let mut first_depth_cmd = None;
        let mut last_depth_cmd = None;
        for (i, cmd) in data.f3d_render_data.commands.iter().enumerate() {
            let pipeline = &data.f3d_render_data.pipelines[&cmd.pipeline];
            if pipeline.depth_compare {
                if first_depth_cmd.is_none() {
                    first_depth_cmd = Some(i);
                }
                last_depth_cmd = Some(i);
            }
        }
        let first_depth_cmd = first_depth_cmd.unwrap_or(0);
        let post_depth_cmd = last_depth_cmd.map(|i| i + 1).unwrap_or(0);

        let transform_bind_group = create_transform_bind_group(device, static_data, data);

        let mut line_vertex_data: Vec<ColorVertex> = Vec::new();
        let mut point_instance_data: Vec<PointInstance> = Vec::new();

        for element in &data.elements {
            match element {
                Element::Line(line) => {
                    line_vertex_data.extend(&[
                        ColorVertex {
                            pos: point4(line.vertices[0]),
                            color: line.color,
                        },
                        ColorVertex {
                            pos: point4(line.vertices[1]),
                            color: line.color,
                        },
                    ]);
                }
                Element::Point(point) => {
                    let x_radius = point.size * 2.0 / data.f3d_render_data.screen_size[0] as f32;
                    let y_radius = point.size * 2.0 / data.f3d_render_data.screen_size[1] as f32;
                    point_instance_data.push(PointInstance {
                        center: [point.pos[0], point.pos[1], point.pos[2], 1.0],
                        radius: [x_radius, y_radius],
                        color: point.color,
                    });
                }
            }
        }

        let line_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&line_vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let point_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&point_instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let surface_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&data.surface_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let transparent_surface_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&data.transparent_surface_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let wall_hitbox_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&data.wall_hitbox_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let wall_hitbox_outline_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&data.wall_hitbox_outline_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Self {
            screen_top_left: data.f3d_render_data.screen_top_left,
            screen_size: data.f3d_render_data.screen_size,
            f3d_pre_depth_cmd_range: 0..first_depth_cmd,
            f3d_depth_cmd_range: first_depth_cmd..post_depth_cmd,
            f3d_post_depth_cmd_range: post_depth_cmd..data.f3d_render_data.commands.len(),
            transform_bind_group,
            line_vertex_buffer: (line_vertex_data.len() as u32, line_vertex_buffer),
            point_instance_buffer: (point_instance_data.len() as u32, point_instance_buffer),
            surface_vertex_buffer: (data.surface_vertices.len() as u32, surface_vertex_buffer),
            transparent_surface_vertex_buffer: (
                data.transparent_surface_vertices.len() as u32,
                transparent_surface_vertex_buffer,
            ),
            wall_hitbox_vertex_buffer: (
                data.wall_hitbox_vertices.len() as u32,
                wall_hitbox_vertex_buffer,
            ),
            wall_hitbox_outline_vertex_buffer: (
                data.wall_hitbox_outline_vertices.len() as u32,
                wall_hitbox_outline_vertex_buffer,
            ),
        }
    }
}

fn create_transform_bind_group(
    device: &wgpu::Device,
    static_data: &StaticData,
    data: &VizRenderData,
) -> wgpu::BindGroup {
    let screen_size = data.f3d_render_data.screen_size;
    let render_output = data.render_output.as_ref().expect("missing gfx output");

    let aspect = screen_size[0] as f32 / screen_size[1] as f32;
    let x_scale = (320.0 / 240.0) / aspect;
    let proj_modify = Matrixf::from_rows([
        [x_scale, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 0.5, 0.5],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let proj_mtx = &proj_modify * &render_output.proj_mtx;

    let proj_mtx_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: cast_slice(&proj_mtx.cols),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let view_mtx_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: cast_slice(&render_output.view_mtx.cols),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &static_data.transform_bind_group_layout,
        entries: &[
            // r_proj
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &proj_mtx_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            // r_view
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &view_mtx_buffer,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    })
}
