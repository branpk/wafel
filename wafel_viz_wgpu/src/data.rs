use std::{f32::consts::PI, mem::size_of, ops::Range};

use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use enum_map::{Enum, EnumMap};
use wafel_viz::{Element, Rect2, TransparencyHint, Vec2, Vec4, VizScene};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Default, Zeroable, Pod)]
#[repr(C)]
pub struct ColorVertex {
    pub pos: Vec4,
    pub color: Vec4,
}

impl ColorVertex {
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
    pub center: Vec4,
    pub radius: Vec2,
    pub color: Vec4,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Enum)]
pub enum TriangleTransparency {
    Opaque,
    Transparent,
    TransparentWallHitbox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Enum)]
pub enum BufferId {
    Point {
        transparent: bool,
    },
    Line {
        transparent: bool,
    },
    Triangle {
        transparency: TriangleTransparency,
        surface_gradient: bool,
    },
}

#[derive(Debug)]
pub struct PerFrameData {
    pub output_size: Vec2,
    pub viewport: Rect2,
    pub scale_factor: f32,

    pub f3d_pre_depth_cmd_range: Range<usize>,
    pub f3d_depth_cmd_range: Range<usize>,
    pub f3d_post_depth_cmd_range: Range<usize>,

    pub transform_bind_group: wgpu::BindGroup,
    pub buffers: EnumMap<BufferId, Option<(u32, wgpu::Buffer)>>,
}

impl PerFrameData {
    pub fn create(
        device: &wgpu::Device,
        static_data: &StaticData,
        scene: &VizScene,
        output_size: Vec2,
        viewport: Rect2,
        scale_factor: f32,
    ) -> Self {
        let mut first_depth_cmd = None;
        let mut last_depth_cmd = None;
        if let Some(f3d_render_data) = &scene.f3d_render_data {
            for (i, cmd) in f3d_render_data.commands.iter().enumerate() {
                let pipeline = &f3d_render_data.pipelines[&cmd.pipeline];
                if pipeline.depth_compare {
                    if first_depth_cmd.is_none() {
                        first_depth_cmd = Some(i);
                    }
                    last_depth_cmd = Some(i);
                }
            }
        }
        let first_depth_cmd = first_depth_cmd.unwrap_or(0);
        let post_depth_cmd = last_depth_cmd.map(|i| i + 1).unwrap_or(0);
        let num_cmds = scene
            .f3d_render_data
            .as_ref()
            .map_or(0, |data| data.commands.len());

        let transform_bind_group = create_transform_bind_group(device, static_data, scene);

        let mut counts: EnumMap<BufferId, u32> = EnumMap::default();
        let mut buffer_data: EnumMap<BufferId, Vec<u8>> = EnumMap::default();

        for element in &scene.elements {
            match element {
                Element::Point(point) => {
                    let buffer_id = BufferId::Point {
                        transparent: point.color[3] < 1.0,
                    };
                    counts[buffer_id] += 1;
                    buffer_data[buffer_id].extend(cast_slice(&[PointInstance {
                        center: point.pos.into_homogeneous_point(),
                        radius: Vec2::broadcast(point.size * 2.0) / viewport.size(),
                        color: point.color,
                    }]));
                }
                Element::Line(line) => {
                    let buffer_id = BufferId::Line {
                        transparent: line.color[3] < 1.0,
                    };
                    counts[buffer_id] += 2;
                    buffer_data[buffer_id].extend(cast_slice(&[
                        ColorVertex {
                            pos: line.vertices[0].into_homogeneous_point(),
                            color: line.color,
                        },
                        ColorVertex {
                            pos: line.vertices[1].into_homogeneous_point(),
                            color: line.color,
                        },
                    ]));
                }
                Element::Triangle(triangle) => {
                    let transparency = if triangle.color[3] >= 1.0 {
                        TriangleTransparency::Opaque
                    } else {
                        match triangle.transparency_hint {
                            TransparencyHint::None => TriangleTransparency::Transparent,
                            TransparencyHint::WallHitbox => {
                                TriangleTransparency::TransparentWallHitbox
                            }
                        }
                    };
                    let buffer_id = BufferId::Triangle {
                        transparency,
                        surface_gradient: triangle.surface_gradient,
                    };
                    counts[buffer_id] += 3;
                    buffer_data[buffer_id].extend(cast_slice(&[
                        ColorVertex {
                            pos: triangle.vertices[0].into_homogeneous_point(),
                            color: triangle.color,
                        },
                        ColorVertex {
                            pos: triangle.vertices[1].into_homogeneous_point(),
                            color: triangle.color,
                        },
                        ColorVertex {
                            pos: triangle.vertices[2].into_homogeneous_point(),
                            color: triangle.color,
                        },
                    ]));
                }
            }
        }

        let mut buffers = EnumMap::default();
        for (buffer_id, data) in buffer_data {
            let count = counts[buffer_id];
            if count > 0 {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: cast_slice(&data),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                buffers[buffer_id] = Some((count, buffer));
            }
        }

        Self {
            output_size,
            viewport,
            scale_factor,
            f3d_pre_depth_cmd_range: 0..first_depth_cmd,
            f3d_depth_cmd_range: first_depth_cmd..post_depth_cmd,
            f3d_post_depth_cmd_range: post_depth_cmd..num_cmds,
            transform_bind_group,
            buffers,
        }
    }
}

fn create_transform_bind_group(
    device: &wgpu::Device,
    static_data: &StaticData,
    scene: &VizScene,
) -> wgpu::BindGroup {
    let proj_mtx_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: cast_slice(&scene.camera.proj_mtx.cols),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let view_mtx_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: cast_slice(&scene.camera.view_mtx.cols),
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
