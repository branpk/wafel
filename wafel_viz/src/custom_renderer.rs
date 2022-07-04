use bytemuck::cast_slice;
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub v0: [f32; 3],
    pub v1: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug)]
pub struct CustomRenderer {
    lines_pipeline: wgpu::RenderPipeline,
    lines_vertex_buffer: Option<(u32, wgpu::Buffer)>,
}

impl CustomRenderer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/color_clip.wgsl"));

        let lines_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lines"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // pos
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        // color
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                }],
            }),
            multiview: None,
        });

        Self {
            lines_pipeline,
            lines_vertex_buffer: None,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, scene: &Scene) {
        let mut lines_vertex_data: Vec<f32> = Vec::new();
        let mut num_vertices = 0;
        for line in &scene.lines {
            lines_vertex_data.extend(&line.v0);
            lines_vertex_data.push(1.0);
            lines_vertex_data.extend(&line.color);
            num_vertices += 1;
            lines_vertex_data.extend(&line.v1);
            lines_vertex_data.push(1.0);
            lines_vertex_data.extend(&line.color);
            num_vertices += 1;
        }

        let lines_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&lines_vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.lines_vertex_buffer = Some((num_vertices, lines_vertex_buffer));
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        let (num_vertices, buffer) = self
            .lines_vertex_buffer
            .as_ref()
            .expect("lines vertex buffer not initialized");

        rp.set_pipeline(&self.lines_pipeline);
        rp.set_vertex_buffer(0, buffer.slice(..));
        rp.draw(0..*num_vertices, 0..1);
    }
}
