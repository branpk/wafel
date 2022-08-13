use std::{f32::consts::PI, mem::size_of, ops::Range};

use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use fast3d::{render::F3DRenderer, util::Matrixf};
use wgpu::util::DeviceExt;

use crate::{Element, VizRenderData};

// TODO: Specify frag_depth as uniform / push constant, combine color_decal.wgsl and
// color.wgsl, use for wall hitboxes instead of calculating by hand

#[derive(Debug, Clone, Copy, PartialEq, Default, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct ColorVertex {
    pub(crate) pos: [f32; 4],
    pub(crate) color: [f32; 4],
}

impl ColorVertex {
    pub(crate) fn new(pos: [f32; 4], color: [f32; 4]) -> Self {
        Self { pos, color }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
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
struct PointInstance {
    center: [f32; 4],
    radius: [f32; 2],
    color: [f32; 4],
}

impl PointInstance {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
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
struct PointVertex {
    offset: [f32; 2],
}

impl PointVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
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

fn point4(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 1.0]
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

fn create_line_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader_module =
        device.create_shader_module(wgpu::include_wgsl!("../shaders/color_decal.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-line"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[ColorVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_point_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader_module = device.create_shader_module(wgpu::include_wgsl!("../shaders/point.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-point"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[PointInstance::layout(), PointVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_surface_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    depth_write_enabled: bool,
) -> wgpu::RenderPipeline {
    let shader_module = device.create_shader_module(wgpu::include_wgsl!("../shaders/surface.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-surface"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[ColorVertex::layout()],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_color_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    color_write_enabled: bool,
    depth_write_enabled: bool,
    depth_compare_enabled: bool,
    topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    let shader_module = device.create_shader_module(wgpu::include_wgsl!("../shaders/color.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-color"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[ColorVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled,
            depth_compare: if depth_compare_enabled {
                wgpu::CompareFunction::LessEqual
            } else {
                wgpu::CompareFunction::Always
            },
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: if color_write_enabled {
                    wgpu::ColorWrites::ALL
                } else {
                    wgpu::ColorWrites::empty()
                },
            })],
        }),
        multiview: None,
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
pub struct VizRenderer {
    f3d_renderer: F3DRenderer,
    transform_bind_group_layout: wgpu::BindGroupLayout,

    line_pipeline: wgpu::RenderPipeline,
    point_pipeline: wgpu::RenderPipeline,
    surface_pipeline: wgpu::RenderPipeline,
    transparent_surface_pipeline: wgpu::RenderPipeline,
    wall_hitbox_pipeline: wgpu::RenderPipeline,
    wall_hitbox_depth_pass_pipeline: wgpu::RenderPipeline,

    point_vertex_buffer: (u32, wgpu::Buffer),
    render_data: Option<RenderData>,
}

#[derive(Debug)]
struct RenderData {
    screen_top_left: [u32; 2],
    screen_size: [u32; 2],
    f3d_pre_depth_cmd_range: Range<usize>,
    f3d_depth_cmd_range: Range<usize>,
    f3d_post_depth_cmd_range: Range<usize>,
    transform_bind_group: wgpu::BindGroup,
    line_vertex_buffer: (u32, wgpu::Buffer),
    point_instance_buffer: (u32, wgpu::Buffer),
    surface_vertex_buffer: (u32, wgpu::Buffer),
    transparent_surface_vertex_buffer: (u32, wgpu::Buffer),
    wall_hitbox_vertex_buffer: (u32, wgpu::Buffer),
    wall_hitbox_outline_vertex_buffer: (u32, wgpu::Buffer),
}

impl VizRenderer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let transform_bind_group_layout = create_transform_bind_group_layout(device);

        let line_pipeline =
            create_line_pipeline(device, &transform_bind_group_layout, output_format);
        let point_pipeline =
            create_point_pipeline(device, &transform_bind_group_layout, output_format);
        let surface_pipeline =
            create_surface_pipeline(device, &transform_bind_group_layout, output_format, true);
        let transparent_surface_pipeline =
            create_surface_pipeline(device, &transform_bind_group_layout, output_format, false);
        let wall_hitbox_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            true,
            true,
            true,
            wgpu::PrimitiveTopology::TriangleList,
        );
        let wall_hitbox_depth_pass_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            false,
            true,
            true,
            wgpu::PrimitiveTopology::TriangleList,
        );

        let point_vertex_buffer = create_point_vertex_buffer(device);

        Self {
            f3d_renderer: F3DRenderer::new(device),
            transform_bind_group_layout,

            line_pipeline,
            point_pipeline,
            surface_pipeline,
            transparent_surface_pipeline,
            wall_hitbox_pipeline,
            wall_hitbox_depth_pass_pipeline,

            point_vertex_buffer,
            render_data: None,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        data: &VizRenderData,
    ) {
        self.render_data = None;

        if data.render_output.is_some() {
            self.prepare_viz(device, queue, output_format, data);
        } else {
            self.f3d_renderer
                .prepare(device, queue, output_format, &data.f3d_render_data);
        }
    }

    fn prepare_viz(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        data: &VizRenderData,
    ) {
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

        self.f3d_renderer
            .prepare(device, queue, output_format, &data.f3d_render_data);

        let transform_bind_group = self.create_transform_bind_group(device, data);

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

        self.render_data = Some(RenderData {
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
        });
    }

    fn create_transform_bind_group(
        &self,
        device: &wgpu::Device,
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
            layout: &self.transform_bind_group_layout,
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

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        if let Some(render_data) = &self.render_data {
            self.f3d_renderer
                .render_command_range(rp, render_data.f3d_pre_depth_cmd_range.clone());

            let vx = render_data.screen_top_left[0];
            let vy = render_data.screen_top_left[1];
            let vw = render_data.screen_size[0];
            let vh = render_data.screen_size[1];
            rp.set_viewport(vx as f32, vy as f32, vw as f32, vh as f32, 0.0, 1.0);
            rp.set_scissor_rect(vx, vy, vw, vh);

            rp.set_pipeline(&self.surface_pipeline);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.surface_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.surface_vertex_buffer.0, 0..1);

            rp.set_pipeline(&self.line_pipeline);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.line_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.line_vertex_buffer.0, 0..1);

            rp.set_pipeline(&self.point_pipeline);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.point_instance_buffer.1.slice(..));
            rp.set_vertex_buffer(1, self.point_vertex_buffer.1.slice(..));
            rp.draw(
                0..self.point_vertex_buffer.0,
                0..render_data.point_instance_buffer.0,
            );

            self.f3d_renderer
                .render_command_range(rp, render_data.f3d_depth_cmd_range.clone());

            {
                // Render wall hitbox outline first since tris write to z buffer
                rp.set_pipeline(&self.line_pipeline);
                rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
                rp.set_vertex_buffer(0, render_data.wall_hitbox_outline_vertex_buffer.1.slice(..));
                rp.draw(
                    0..render_data.wall_hitbox_outline_vertex_buffer.0 as u32,
                    0..1,
                );

                // When two wall hitboxes overlap, we should not increase the opacity within
                // their region of overlap (preference).
                // First pass writes only to depth buffer to ensure that only the closest
                // hitbox triangles are drawn, then second pass draws them.
                rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
                rp.set_vertex_buffer(0, render_data.wall_hitbox_vertex_buffer.1.slice(..));

                rp.set_pipeline(&self.wall_hitbox_depth_pass_pipeline);
                rp.draw(0..render_data.wall_hitbox_vertex_buffer.0 as u32, 0..1);
                rp.set_pipeline(&self.wall_hitbox_pipeline);
                rp.draw(0..render_data.wall_hitbox_vertex_buffer.0 as u32, 0..1);
            }

            rp.set_pipeline(&self.transparent_surface_pipeline);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.transparent_surface_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.transparent_surface_vertex_buffer.0, 0..1);

            self.f3d_renderer
                .render_command_range(rp, render_data.f3d_post_depth_cmd_range.clone());
        } else {
            self.f3d_renderer.render(rp);
        }
    }
}
