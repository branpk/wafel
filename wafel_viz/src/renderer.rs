use std::{mem::size_of, ops::Range};

use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use fast3d::{render::F3DRenderer, util::Matrixf};
use wgpu::util::DeviceExt;

use crate::{Element, VizRenderData};

#[derive(Debug, Clone, Copy, PartialEq, Default, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct ColorVertex {
    pub(crate) pos: [f32; 4],
    pub(crate) color: [f32; 4],
}

impl ColorVertex {
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

fn create_color_line_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader_module =
        device.create_shader_module(wgpu::include_wgsl!("../shaders/color_decal.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-color-line"),
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
            targets: &[wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            }],
        }),
        multiview: None,
    })
}

fn create_surface_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
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
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            }],
        }),
        multiview: None,
    })
}

#[derive(Debug)]
pub struct VizRenderer {
    f3d_renderer: F3DRenderer,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    color_line_pipeline: wgpu::RenderPipeline,
    surface_pipeline: wgpu::RenderPipeline,
    render_data: Option<RenderData>,
}

#[derive(Debug)]
struct RenderData {
    f3d_pre_cmds: Range<usize>,
    f3d_post_cmds: Range<usize>,
    transform_bind_group: wgpu::BindGroup,
    color_line_vertex_buffer: (u32, wgpu::Buffer),
    surface_vertex_buffer: (u32, wgpu::Buffer),
}

impl VizRenderer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let transform_bind_group_layout = create_transform_bind_group_layout(device);

        let color_line_pipeline =
            create_color_line_pipeline(device, &transform_bind_group_layout, output_format);
        let surface_pipeline =
            create_surface_pipeline(device, &transform_bind_group_layout, output_format);

        Self {
            f3d_renderer: F3DRenderer::new(device),
            transform_bind_group_layout,
            color_line_pipeline,
            surface_pipeline,
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
        let first_z_buf_index = data
            .f3d_render_data
            .commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| {
                let pipeline = &data.f3d_render_data.pipelines[&cmd.pipeline];
                pipeline.depth_compare
            })
            .map(|(i, _)| i + 1)
            .next()
            .unwrap_or(0);

        self.f3d_renderer
            .prepare(device, queue, output_format, &data.f3d_render_data);

        let transform_bind_group = self.create_transform_bind_group(device, data);

        let mut color_line_vertex_data: Vec<ColorVertex> = Vec::new();

        for element in &data.elements {
            match element {
                Element::Line(line) => {
                    color_line_vertex_data.extend(&[
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
            }
        }

        let color_line_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&color_line_vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let surface_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&data.surface_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.render_data = Some(RenderData {
            f3d_pre_cmds: 0..first_z_buf_index,
            f3d_post_cmds: first_z_buf_index..data.f3d_render_data.commands.len(),
            transform_bind_group,
            color_line_vertex_buffer: (
                color_line_vertex_data.len() as u32,
                color_line_vertex_buffer,
            ),
            surface_vertex_buffer: (data.surface_vertices.len() as u32, surface_vertex_buffer),
        });
    }

    fn create_transform_bind_group(
        &self,
        device: &wgpu::Device,
        data: &VizRenderData,
    ) -> wgpu::BindGroup {
        let render_output = data.render_output.as_ref().expect("missing gfx output");

        let aspect = data.screen_size[0] as f32 / data.screen_size[1] as f32;
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

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>, output_size: [u32; 2]) {
        if let Some(render_data) = &self.render_data {
            self.f3d_renderer.render_command_range(
                rp,
                output_size,
                render_data.f3d_pre_cmds.clone(),
            );

            rp.set_pipeline(&self.surface_pipeline);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.surface_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.surface_vertex_buffer.0, 0..1);

            rp.set_pipeline(&self.color_line_pipeline);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.color_line_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.color_line_vertex_buffer.0, 0..1);

            self.f3d_renderer.render_command_range(
                rp,
                output_size,
                render_data.f3d_post_cmds.clone(),
            );
        } else {
            self.f3d_renderer.render(rp, output_size);
        }
    }
}
