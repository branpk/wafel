use bytemuck::cast_slice;
use pyo3::prelude::*;
use std::iter;
use wgpu::util::DeviceExt;

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct Scene {
    #[pyo3(set)]
    viewport: Viewport,
}

#[pymethods]
impl Scene {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct Viewport {
    #[pyo3(get, set)]
    x: f32,
    #[pyo3(get, set)]
    y: f32,
    #[pyo3(get, set)]
    width: f32,
    #[pyo3(get, set)]
    height: f32,
}

#[pymethods]
impl Viewport {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                }),
            ),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &device.create_shader_module(wgpu::include_spirv!(
                    "../../bin/shaders/simple.vert.spv"
                )),
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &device.create_shader_module(wgpu::include_spirv!(
                    "../../bin/shaders/simple.frag.spv"
                )),
                entry_point: "main",
            }),
            rasterization_state: None,
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor::from(output_format)],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: 12,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[wgpu::VertexAttributeDescriptor {
                        offset: 0,
                        format: wgpu::VertexFormat::Float3,
                        shader_location: 0,
                    }],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self { pipeline }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        output_format: wgpu::TextureFormat,
        scenes: &[Scene],
    ) {
        let vertices: Vec<[f32; 3]> = vec![[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.06,
                            g: 0.06,
                            b: 0.06,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            for scene in scenes {
                let viewport = &scene.viewport;
                render_pass.set_viewport(
                    viewport.x,
                    viewport.y,
                    viewport.width,
                    viewport.height,
                    0.0,
                    1.0,
                );
                render_pass.set_scissor_rect(
                    viewport.x as u32,
                    viewport.y as u32,
                    viewport.width as u32,
                    viewport.height as u32,
                );

                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw(0..vertices.len() as u32, 0..1);
            }
        }

        let command_buffer = encoder.finish();
        queue.submit(iter::once(command_buffer));
    }
}
