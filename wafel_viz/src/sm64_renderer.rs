use std::{
    collections::HashMap,
    fmt::{self, Write},
};

use bytemuck::cast_slice;
use wgpu::util::DeviceExt;

use crate::{
    render_api::decode_shader_id,
    sm64_render_data::{RenderState, SM64RenderData, SamplerState, Texture},
};

#[derive(Debug)]
pub struct SM64Renderer {
    samplers: HashMap<SamplerState, wgpu::Sampler>,
    textures: HashMap<usize, wgpu::Texture>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_groups: HashMap<(SamplerState, usize), wgpu::BindGroup>,
    pipelines: HashMap<RenderState, wgpu::RenderPipeline>,
    commands: Vec<Command>,
}

#[derive(Debug)]
struct Command {
    state: RenderState,
    sampler: SamplerState,
    texture_index: Option<usize>,
    buffer: wgpu::Buffer,
    num_vertices: u32,
}

fn label(prefix: &str, shader_id: u32) -> &'static str {
    let label = format!("{}-{:#010X}", prefix, shader_id);
    Box::leak(label.into_boxed_str())
}

impl SM64Renderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // r_sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // r_texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        Self {
            samplers: HashMap::new(),
            textures: HashMap::new(),
            texture_bind_group_layout,
            texture_bind_groups: HashMap::new(),
            pipelines: HashMap::new(),
            commands: Vec::new(),
        }
    }

    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        state: RenderState,
    ) -> Result<wgpu::RenderPipeline, fmt::Error> {
        let shader_id = state.shader_id.expect("missing shader id");
        let cc_features = decode_shader_id(shader_id);

        let mut s = String::new();
        writeln!(s, "// Shader {:#010X}", shader_id)?;
        writeln!(s)?;

        let mut bind_group_layouts: Vec<&wgpu::BindGroupLayout> = Vec::new();
        {
            if cc_features.uses_textures() {
                writeln!(s, "@group(0) @binding(0) var r_sampler: sampler;")?;
                writeln!(s, "@group(0) @binding(1) var r_texture: texture_2d<f32>;")?;
                writeln!(s)?;
                bind_group_layouts.push(&self.texture_bind_group_layout);
            }
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let mut vertex_attributes: Vec<wgpu::VertexAttribute> = Vec::new();
        let vertex_buffer_layout = {
            let mut current_attribute_offset = 0;
            let mut current_location = 0;

            writeln!(s, "struct VertexData {{")?;
            {
                writeln!(s, "    @location({}) pos: vec4<f32>,", current_location)?;
                vertex_attributes.push(wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: current_attribute_offset,
                    shader_location: current_location,
                });
                current_attribute_offset += 16;
                current_location += 1;

                if cc_features.uses_textures() {
                    writeln!(s, "    @location({}) uv: vec2<f32>,", current_location)?;
                    vertex_attributes.push(wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: current_attribute_offset,
                        shader_location: current_location,
                    });
                    current_attribute_offset += 8;
                    current_location += 1;
                }

                if cc_features.opt_fog {
                    writeln!(s, "    @location({}) fog: vec4<f32>,", current_location)?;
                    vertex_attributes.push(wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: current_attribute_offset,
                        shader_location: current_location,
                    });
                    current_attribute_offset += 16;
                    current_location += 1;
                }

                for input_index in 0..cc_features.num_inputs {
                    if cc_features.opt_alpha {
                        writeln!(
                            s,
                            "    @location({}) input{}: vec4<f32>,",
                            current_location, input_index
                        )?;
                        vertex_attributes.push(wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: current_attribute_offset,
                            shader_location: current_location,
                        });
                        current_attribute_offset += 16;
                        current_location += 1;
                    } else {
                        writeln!(
                            s,
                            "    @location({}) input{}: vec3<f32>,",
                            current_location, input_index
                        )?;
                        vertex_attributes.push(wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: current_attribute_offset,
                            shader_location: current_location,
                        });
                        current_attribute_offset += 12;
                        current_location += 1;
                    }
                }
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
            if cc_features.num_inputs > 0 {
                writeln!(s, "    @location(0) color: vec4<f32>,")?;
            }
            if cc_features.uses_textures() {
                writeln!(s, "    @location(1) uv: vec2<f32>,")?;
            }
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@vertex")?;
            writeln!(s, "fn vs_main(in: VertexData) -> VertexOutput {{")?;
            writeln!(s, "    var out = VertexOutput();")?;
            writeln!(s, "    out.position = in.pos;")?;
            if cc_features.num_inputs > 0 {
                if cc_features.opt_alpha {
                    writeln!(s, "    out.color = in.input0;")?;
                } else {
                    writeln!(s, "    out.color = vec4<f32>(in.input0, 0.0);")?;
                }
            }
            if cc_features.uses_textures() {
                writeln!(s, "    out.uv = in.uv;")?;
            }
            writeln!(s, "    return out;")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "struct FragmentOutput {{")?;
            writeln!(s, "    @builtin(frag_depth) frag_depth: f32,")?;
            writeln!(s, "    @location(0) color: vec4<f32>,")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@fragment")?;
            writeln!(s, "fn fs_main(in: VertexOutput) -> FragmentOutput {{")?;
            writeln!(s, "    var out = FragmentOutput();")?;
            if state.zmode_decal {
                writeln!(s, "    out.frag_depth = in.position.z - 0.001;")?;
            } else {
                writeln!(s, "    out.frag_depth = in.position.z;")?;
            }
            #[rustfmt::skip]
            if cc_features.uses_textures() {
                writeln!(s, "    out.color = textureSample(r_texture, r_sampler, in.uv);")?;
            } else if cc_features.num_inputs > 0 {
                writeln!(s, "    out.color = in.color;")?;
            } else {
                writeln!(s, "    out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);")?;
            }
            writeln!(s, "    return out;")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label("shader", shader_id)),
            source: wgpu::ShaderSource::Wgsl(s.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label("pipeline", shader_id)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: state.depth_mask,
                depth_compare: if state.depth_test {
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
                targets: &[wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING).filter(|_| state.use_alpha),
                    write_mask: wgpu::ColorWrites::all(),
                }],
            }),
            multiview: None,
        });

        Ok(pipeline)
    }

    fn prepare_pipeline(
        &mut self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        state: RenderState,
    ) {
        if !self.pipelines.contains_key(&state) {
            let pipeline = self.create_pipeline(device, output_format, state).unwrap();
            self.pipelines.insert(state, pipeline);
        }
    }

    fn prepare_sampler(&mut self, device: &wgpu::Device, state: SamplerState) {
        if !self.samplers.contains_key(&state) {
            let filter = if state.linear_filter {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            };

            let address_mode = |v| {
                if v & 0x2 != 0 {
                    wgpu::AddressMode::ClampToEdge
                } else if v & 0x1 != 0 {
                    wgpu::AddressMode::MirrorRepeat
                } else {
                    wgpu::AddressMode::Repeat
                }
            };

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: address_mode(state.cms),
                address_mode_v: address_mode(state.cmt),
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: f32::MAX,
                compare: None,
                anisotropy_clamp: None,
                border_color: None,
            });
            self.samplers.insert(state, sampler);
        }
    }

    fn prepare_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: usize,
        data: &Texture,
    ) {
        if !self.textures.contains_key(&index) {
            assert!(data.width * data.height != 0, "texture of zero size");

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: data.width,
                    height: data.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            });
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data.rgba8,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((4 * data.width).try_into().unwrap()),
                    rows_per_image: Some(data.height.try_into().unwrap()),
                },
                wgpu::Extent3d {
                    width: data.width,
                    height: data.height,
                    depth_or_array_layers: 1,
                },
            );

            self.textures.insert(index, texture);
        }
    }

    fn prepare_texture_bind_group(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sampler_state: SamplerState,
        texture_index: usize,
        texture: &Texture,
    ) {
        let key = (sampler_state, texture_index);
        if !self.texture_bind_groups.contains_key(&key) {
            self.prepare_sampler(device, sampler_state);
            self.prepare_texture(device, queue, texture_index, texture);

            let sampler = &self.samplers[&sampler_state];
            let texture = &self.textures[&texture_index];

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.texture_bind_group_layout,
                entries: &[
                    // r_sampler
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    // r_texture
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });

            self.texture_bind_groups.insert(key, bind_group);
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        data: &SM64RenderData,
    ) {
        // Textures may change index across frames
        self.textures.clear();
        self.texture_bind_groups.clear();

        self.commands.clear();
        for command in &data.commands {
            let shader_id = command.state.shader_id.expect("missing shader id");
            let cc_features = decode_shader_id(shader_id);

            self.prepare_pipeline(device, output_format, command.state);

            if cc_features.uses_textures() {
                let sampler = command.sampler;
                let texture_index = command.texture_index.expect("missing texture index");
                let texture = data
                    .textures
                    .get(texture_index)
                    .expect("invalid texture id")
                    .as_ref()
                    .expect("texture not uploaded");
                self.prepare_texture_bind_group(device, queue, sampler, texture_index, texture);
            }

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&command.vertex_buffer),
                usage: wgpu::BufferUsages::VERTEX,
            });

            self.commands.push(Command {
                state: command.state,
                sampler: command.sampler,
                texture_index: command.texture_index,
                buffer,
                num_vertices: 3 * command.num_tris as u32,
            });
        }
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        let mut current_state = None;

        for command in &self.commands {
            let shader_id = command.state.shader_id.expect("missing shader id");
            let cc_features = decode_shader_id(shader_id);

            if current_state != Some(command.state) {
                current_state = Some(command.state);
                let pipeline = self
                    .pipelines
                    .get(&command.state)
                    .expect("pipeline not prepared");
                rp.set_pipeline(pipeline);
            }

            if cc_features.uses_textures() {
                let sampler = command.sampler;
                let texture_index = command.texture_index.expect("missing texture index");

                let bind_group = self
                    .texture_bind_groups
                    .get(&(sampler, texture_index))
                    .expect("bind group not prepared");

                rp.set_bind_group(0, bind_group, &[]);
            }

            rp.set_vertex_buffer(0, command.buffer.slice(..));
            rp.draw(0..command.num_vertices, 0..1);
        }
    }
}
