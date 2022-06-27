use std::{
    collections::HashMap,
    fmt::{self, Write},
};

use bytemuck::cast_slice;
use wgpu::util::DeviceExt;

use crate::{
    render_api::decode_shader_id,
    sm64_render_data::{RenderState, SM64RenderData},
};

#[derive(Debug)]
pub struct SM64Renderer {
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_groups: Vec<Option<wgpu::BindGroup>>,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    commands: Vec<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PipelineKey {
    shader_id: u32,
    depth_test: bool,
    depth_mask: bool,
}

impl From<RenderState> for PipelineKey {
    fn from(state: RenderState) -> Self {
        Self {
            shader_id: state.shader_id,
            depth_test: state.depth_test,
            depth_mask: state.depth_mask,
        }
    }
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
            texture_bind_group_layout,
            texture_bind_groups: Vec::new(),
            pipelines: HashMap::new(),
            commands: Vec::new(),
        }
    }

    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        key: PipelineKey,
    ) -> Result<wgpu::RenderPipeline, fmt::Error> {
        let shader_id = key.shader_id;
        let cc_features = decode_shader_id(shader_id);
        let use_texturing = cc_features.used_textures.iter().any(|&b| b);

        let mut s = String::new();
        writeln!(s, "// Shader {:#010X}", shader_id)?;
        writeln!(s)?;

        let mut bind_group_layouts: Vec<&wgpu::BindGroupLayout> = Vec::new();
        {
            if use_texturing {
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

                if use_texturing {
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
            if use_texturing {
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
            if use_texturing {
                writeln!(s, "    out.uv = in.uv;")?;
            }
            writeln!(s, "    return out;")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@fragment")?;
            #[rustfmt::skip]
            writeln!(s, "fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{")?;
            if use_texturing {
                writeln!(s, "    return textureSample(r_texture, r_sampler, in.uv);")?;
            } else if cc_features.num_inputs > 0 {
                writeln!(s, "    return in.color;")?;
            } else {
                writeln!(s, "    return vec4<f32>(1.0, 1.0, 1.0, 1.0);")?;
            }
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label("shader", shader_id)),
            source: wgpu::ShaderSource::Wgsl(s.into()),
        });

        // TODO: verify all pipeline fields
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
                depth_write_enabled: key.depth_mask,
                depth_compare: if key.depth_test {
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
        key: PipelineKey,
    ) {
        if !self.pipelines.contains_key(&key) {
            let pipeline = self.create_pipeline(device, output_format, key).unwrap();
            self.pipelines.insert(key, pipeline);
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        data: &SM64RenderData,
    ) {
        self.texture_bind_groups.clear();
        for texture_data in &data.textures {
            let bind_group = texture_data.as_ref().map(|texture_data| {
                assert!(
                    texture_data.width * texture_data.height != 0,
                    "texture of zero size"
                );

                // TODO: Sampler params + can cache sampler
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    label: None,
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    lod_min_clamp: 0.0,
                    lod_max_clamp: f32::MAX,
                    compare: None,
                    anisotropy_clamp: None,
                    border_color: None,
                });

                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: texture_data.width,
                        height: texture_data.height,
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
                    &texture_data.rgba8,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some((4 * texture_data.width).try_into().unwrap()),
                        rows_per_image: Some(texture_data.height.try_into().unwrap()),
                    },
                    wgpu::Extent3d {
                        width: texture_data.width,
                        height: texture_data.height,
                        depth_or_array_layers: 1,
                    },
                );

                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.texture_bind_group_layout,
                    entries: &[
                        // r_sampler
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        // r_texture
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                            ),
                        },
                    ],
                })
            });
            self.texture_bind_groups.push(bind_group);
        }

        self.commands.clear();
        for command in &data.commands {
            self.prepare_pipeline(device, output_format, command.state.into());

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&command.vertex_buffer),
                usage: wgpu::BufferUsages::VERTEX,
            });

            self.commands.push(Command {
                state: command.state,
                buffer,
                num_vertices: 3 * command.num_tris as u32,
            });
        }
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        let mut current_key = None;

        for command in &self.commands {
            let shader_id = command.state.shader_id;
            let cc_features = decode_shader_id(shader_id);
            let key: PipelineKey = command.state.into();

            if current_key != Some(key) {
                current_key = Some(key);
                let pipeline = self.pipelines.get(&key).expect("pipeline not prepared");
                rp.set_pipeline(pipeline);
            }

            if cc_features.used_textures.iter().any(|&b| b) {
                let texture_index = command.state.texture_index.expect("missing texture index");
                let bind_group = self
                    .texture_bind_groups
                    .get(texture_index)
                    .expect("invalid texture index")
                    .as_ref()
                    .expect("texture not uploaded");
                rp.set_bind_group(0, bind_group, &[]);
            }

            rp.set_vertex_buffer(0, command.buffer.slice(..));
            rp.draw(0..command.num_vertices, 0..1);
        }
    }
}
