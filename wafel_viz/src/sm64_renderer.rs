use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Write},
    mem::size_of,
};

use bytemuck::cast_slice;
use wgpu::util::DeviceExt;

use crate::{
    render_api::{decode_shader_id, CCFeatures},
    sm64_render_data::{RenderState, SM64RenderData},
};

#[derive(Debug)]
pub struct SM64Renderer {
    shader_id_to_pipeline: HashMap<u32, wgpu::RenderPipeline>,
    commands: Vec<Command>,
}

#[derive(Debug)]
struct Command {
    state: RenderState,
    buffer: wgpu::Buffer,
    num_vertices: u32,
}

fn label(prefix: &str, shader_id: u32) -> &'static str {
    let label = format!("{}-{:#010X}", prefix, shader_id);
    Box::leak(label.into_boxed_str())
}

impl SM64Renderer {
    pub fn new() -> Self {
        Self {
            shader_id_to_pipeline: HashMap::new(),
            commands: Vec::new(),
        }
    }

    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        shader_id: u32,
    ) -> Result<wgpu::RenderPipeline, fmt::Error> {
        let cc_features = decode_shader_id(shader_id);

        let mut s = String::new();
        writeln!(s, "// Shader {:#010X}", shader_id)?;
        writeln!(s)?;

        let mut vertex_attributes: Vec<wgpu::VertexAttribute> = Vec::new();
        let vertex_buffer_layout = {
            let mut current_attribute_offset = 0;

            writeln!(s, "struct VertexData {{")?;
            {
                writeln!(s, "    @location(0) pos: vec4<f32>,")?;
                vertex_attributes.push(wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: current_attribute_offset,
                    shader_location: 0,
                });
                current_attribute_offset += 16;
            }
            writeln!(s, "}}")?;
            writeln!(s)?;

            wgpu::VertexBufferLayout {
                array_stride: current_attribute_offset,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attributes,
            }
        };

        {
            writeln!(s, "struct VertexOutput {{")?;
            writeln!(s, "    @builtin(position) position: vec4<f32>,")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@vertex")?;
            writeln!(s, "fn vs_main(vertex: VertexData) -> VertexOutput {{")?;
            writeln!(s, "    var output = VertexOutput();")?;
            writeln!(s, "    output.position = vertex.pos;")?;
            writeln!(s, "    return output;")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@fragment")?;
            #[rustfmt::skip]
            writeln!(s, "fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {{")?;
            writeln!(s, "    return vec4<f32>(1.0, 1.0, 1.0, 1.0);")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label("shader", shader_id)),
            source: wgpu::ShaderSource::Wgsl(s.into()),
        });

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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[output_format.into()],
            }),
            multiview: None,
        });

        Ok(pipeline)
    }

    fn prepare_pipeline(
        &mut self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        shader_id: u32,
    ) {
        if !self.shader_id_to_pipeline.contains_key(&shader_id) {
            let pipeline = self
                .create_pipeline(device, output_format, shader_id)
                .unwrap();
            self.shader_id_to_pipeline.insert(shader_id, pipeline);
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        data: &SM64RenderData,
    ) {
        self.commands.clear();
        for vertex_buffer in &data.vertex_buffers {
            self.prepare_pipeline(device, output_format, vertex_buffer.state.shader_id);

            let vertex_stride = vertex_buffer.buffer.len() / (3 * vertex_buffer.num_tris);
            let mut used_buffer: Vec<f32> = Vec::new();
            for i in 0..3 * vertex_buffer.num_tris {
                let i0 = i * vertex_stride;
                used_buffer.extend(&vertex_buffer.buffer[i0..i0 + 4]);
            }

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&used_buffer),
                usage: wgpu::BufferUsages::VERTEX,
            });

            self.commands.push(Command {
                state: vertex_buffer.state,
                buffer,
                num_vertices: 3 * vertex_buffer.num_tris as u32,
            });
        }
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        let mut current_shader_id = None;

        for command in &self.commands {
            if current_shader_id != Some(command.state.shader_id) {
                current_shader_id = Some(command.state.shader_id);

                let pipeline = self
                    .shader_id_to_pipeline
                    .get(&command.state.shader_id)
                    .expect("pipeline not prepared");

                rp.set_pipeline(pipeline);
            }

            rp.set_vertex_buffer(0, command.buffer.slice(..));
            rp.draw(0..command.num_vertices, 0..1);
        }
    }
}
