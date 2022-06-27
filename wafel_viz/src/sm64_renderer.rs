use std::mem::size_of;

use bytemuck::cast_slice;
use wgpu::util::DeviceExt;

use crate::sm64_render_data::SM64RenderData;

#[derive(Debug)]
pub struct SM64Renderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffers: Vec<(wgpu::Buffer, u32)>,
}

impl SM64Renderer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let shader_module =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/test.wgsl"));

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 4]>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // pos
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
            ],
        };

        // TODO: verify all pipeline fields
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            primitive: wgpu::PrimitiveState {
                polygon_mode: wgpu::PolygonMode::Line,
                ..wgpu::PrimitiveState::default()
            },
            // primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[output_format.into()],
            }),
            multiview: None,
        });

        Self {
            pipeline,
            vertex_buffers: Vec::new(),
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, data: &SM64RenderData) {
        self.vertex_buffers.clear();
        for vertex_buffer in &data.vertex_buffers {
            let vertex_stride = vertex_buffer.buffer.len() / (3 * vertex_buffer.num_tris);
            let mut used_buffer: Vec<f32> = Vec::new();
            for i in 0..3 * vertex_buffer.num_tris {
                used_buffer.extend(&vertex_buffer.buffer[i * vertex_stride..i * vertex_stride + 4]);
            }

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&used_buffer),
                usage: wgpu::BufferUsages::VERTEX,
            });
            self.vertex_buffers
                .push((buffer, 3 * vertex_buffer.num_tris as u32));
        }
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        rp.set_pipeline(&self.pipeline);
        for (buffer, num_vertices) in &self.vertex_buffers {
            rp.set_vertex_buffer(0, buffer.slice(..));
            rp.draw(0..*num_vertices, 0..1);
        }
    }
}
