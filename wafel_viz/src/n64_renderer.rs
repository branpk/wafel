use std::{
    collections::HashMap,
    fmt::{self, Write},
};

use bytemuck::cast_slice;
use wgpu::util::DeviceExt;

use crate::{
    n64_render_data::{
        N64RenderData, RenderState, SamplerState, ScreenRectangle, TextureData, TextureState,
    },
    render_api::{decode_shader_id, CCFeatures, ShaderItem},
};

#[derive(Debug)]
pub struct N64Renderer {
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_groups: HashMap<usize, wgpu::BindGroup>,
    pipelines: HashMap<RenderState, wgpu::RenderPipeline>,
    commands: Vec<Command>,
}

#[derive(Debug)]
struct Command {
    viewport: ScreenRectangle,
    scissor: ScreenRectangle,
    state: RenderState,
    texture_index: [Option<usize>; 2],
    buffer: wgpu::Buffer,
    num_vertices: u32,
}

fn label(prefix: &str, shader_id: u32) -> &'static str {
    let label = format!("{}-{:#010X}", prefix, shader_id);
    Box::leak(label.into_boxed_str())
}

#[rustfmt::skip]
fn write_fragment_shader_body(s: &mut String, cc_features: CCFeatures) -> Result<(), fmt::Error> {
    for i in 0..2 {
        if cc_features.used_textures[i] {
            writeln!(s, "    let tex{} = textureSample(r_texture{}, r_sampler{}, in.uv);", i, i, i)?;
        }
    }

    if cc_features.opt_alpha && cc_features.color_alpha_same {
        writeln!(s, "    out.color = {};", component_expr(cc_features, 0))?;
    } else {
        writeln!(s, "    let rgb = ({}).rgb;", component_expr(cc_features, 0))?;
        if cc_features.opt_alpha {
            writeln!(s, "    let a = ({}).a;", component_expr(cc_features, 1))?;
            writeln!(s, "    out.color = vec4<f32>(rgb, a);")?;
        }
        else {
            writeln!(s, "    out.color = vec4<f32>(rgb, 1.0);")?;
        }
    }

    if cc_features.opt_texture_edge && cc_features.opt_alpha {
        writeln!(s, "    if out.color.a > 0.3 {{")?;
        writeln!(s, "        out.color = vec4<f32>(out.color.rgb, 1.0);")?;
        writeln!(s, "    }} else {{")?;
        writeln!(s, "        discard;")?;
        writeln!(s, "    }}")?;
    }

    if cc_features.opt_fog {
        writeln!(s, "    let fog_mixed = mix(out.color.rgb, in.fog.rgb, in.fog.a);")?;
        writeln!(s, "    out.color = vec4<f32>(fog_mixed, out.color.a);")?;
    }

    // TODO: Noise
    // if (cc_features.opt_alpha && cc_features.opt_noise) {
    //     append_line(fs_buf, &fs_len, "texel.a *= floor(random(vec3(floor(gl_FragCoord.xy * (240.0 / float(window_height))), float(frame_count))) + 0.5);");
    // }

    Ok(())
}

fn component_expr(cc_features: CCFeatures, i: usize) -> String {
    let items = cc_features.c[i];
    if cc_features.do_single[i] {
        single_expr(items)
    } else if cc_features.do_multiply[i] {
        multiply_expr(items)
    } else if cc_features.do_mix[i] {
        mix_expr(items)
    } else {
        linear_expr(items)
    }
}

fn single_expr(items: [ShaderItem; 4]) -> String {
    item_expr(items[3]).to_string()
}

fn multiply_expr(items: [ShaderItem; 4]) -> String {
    format!("{} * {}", item_expr(items[0]), item_expr(items[2]))
}

fn mix_expr(items: [ShaderItem; 4]) -> String {
    format!(
        "mix({}, {}, {})",
        item_expr(items[1]),
        item_expr(items[0]),
        item_expr(items[2])
    )
}

fn linear_expr(items: [ShaderItem; 4]) -> String {
    format!(
        "({} - {}) * {} + {}",
        item_expr(items[0]),
        item_expr(items[1]),
        item_expr(items[2]),
        item_expr(items[3])
    )
}

fn item_expr(item: ShaderItem) -> &'static str {
    match item {
        ShaderItem::Zero => "vec4<f32>()",
        ShaderItem::Input1 => "in.input1",
        ShaderItem::Input2 => "in.input2",
        ShaderItem::Input3 => "in.input3",
        ShaderItem::Input4 => "in.input4",
        ShaderItem::Texel0 => "tex0",
        ShaderItem::Texel0A => "vec4<f32>(tex0.a)",
        ShaderItem::Texel1 => "tex1",
    }
}

impl N64Renderer {
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
        #[rustfmt::skip]
        for i in 0..2 {
            if cc_features.used_textures[i] {
                writeln!(s, "@group({}) @binding(0) var r_sampler{}: sampler;", i, i)?;
                writeln!(s, "@group({}) @binding(1) var r_texture{}: texture_2d<f32>;", i, i)?;
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

                for input_index in 1..=cc_features.num_inputs {
                    let length = if cc_features.opt_alpha { 4 } else { 3 };
                    writeln!(
                        s,
                        "    @location({}) input{}: vec{}<f32>,",
                        current_location, input_index, length,
                    )?;
                    vertex_attributes.push(wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: current_attribute_offset,
                        shader_location: current_location,
                    });
                    current_attribute_offset += 4 * length;
                    current_location += 1;
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
            let mut location = 0;
            if cc_features.uses_textures() {
                writeln!(s, "    @location({}) uv: vec2<f32>,", location)?;
                location += 1;
            }
            if cc_features.opt_fog {
                writeln!(s, "    @location({}) fog: vec4<f32>,", location)?;
                location += 1;
            }
            for input_index in 1..=cc_features.num_inputs {
                writeln!(
                    s,
                    "    @location({}) input{}: vec4<f32>,",
                    location, input_index
                )?;
                location += 1;
            }
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@vertex")?;
            writeln!(s, "fn vs_main(in: VertexData) -> VertexOutput {{")?;
            writeln!(s, "    var out = VertexOutput();")?;
            writeln!(s, "    out.position = in.pos;")?;
            if cc_features.uses_textures() {
                writeln!(s, "    out.uv = in.uv;")?;
            }
            if cc_features.opt_fog {
                writeln!(s, "    out.fog = in.fog;")?;
            }
            for input_index in 1..=cc_features.num_inputs {
                if cc_features.opt_alpha {
                    writeln!(s, "    out.input{} = in.input{};", input_index, input_index)?
                } else {
                    writeln!(
                        s,
                        "    out.input{} = vec4<f32>(in.input{}, 1.0);",
                        input_index, input_index
                    )?
                }
            }
            writeln!(s, "    return out;")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "struct FragmentOutput {{")?;
            if state.zmode_decal {
                writeln!(s, "    @builtin(frag_depth) frag_depth: f32,")?;
            }
            writeln!(s, "    @location(0) color: vec4<f32>,")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        #[rustfmt::skip]
        if cc_features.opt_noise {
            writeln!(s, "fn random(v: vec3<f32>) -> f32 {{")?;
            writeln!(s, "    let r = dot(sin(v), vec3<f32>(12.9898, 78.233, 37.719));")?;
            writeln!(s, "    return fract(sin(r) * 143758.5453);")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@fragment")?;
            writeln!(s, "fn fs_main(in: VertexOutput) -> FragmentOutput {{")?;
            writeln!(s, "    var out = FragmentOutput();")?;
            if state.zmode_decal {
                writeln!(s, "    out.frag_depth = in.position.z - 0.001;")?;
            }
            write_fragment_shader_body(&mut s, cc_features)?;
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
            // primitive: wgpu::PrimitiveState {
            //     polygon_mode: wgpu::PolygonMode::Line,
            //     ..Default::default()
            // },
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

    fn create_sampler(&mut self, device: &wgpu::Device, state: SamplerState) -> wgpu::Sampler {
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

        device.create_sampler(&wgpu::SamplerDescriptor {
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
        })
    }

    fn create_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &TextureData,
    ) -> wgpu::Texture {
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

        texture
    }

    fn prepare_texture_bind_group(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_index: usize,
        texture: &TextureState,
    ) {
        if !self.texture_bind_groups.contains_key(&texture_index) {
            let sampler =
                self.create_sampler(device, texture.sampler.expect("sampler parameters not set"));
            let texture = self.create_texture(
                device,
                queue,
                texture.data.as_ref().expect("texture not uploaded"),
            );

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            });

            self.texture_bind_groups.insert(texture_index, bind_group);
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        data: &N64RenderData,
    ) {
        // Textures may change index across frames
        self.texture_bind_groups.clear();
        for (texture_index, texture) in data.textures.iter().enumerate() {
            self.prepare_texture_bind_group(device, queue, texture_index, texture);
        }

        self.commands.clear();
        for command in &data.commands {
            self.prepare_pipeline(device, output_format, command.state);

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&command.vertex_buffer),
                usage: wgpu::BufferUsages::VERTEX,
            });

            self.commands.push(Command {
                viewport: command.viewport,
                scissor: command.scissor,
                state: command.state,
                texture_index: command.texture_index,
                buffer,
                num_vertices: 3 * command.num_tris as u32,
            });
        }
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>, output_size: (u32, u32)) {
        let mut current_state = None;

        for command in &self.commands {
            let shader_id = command.state.shader_id.expect("missing shader id");
            let cc_features = decode_shader_id(shader_id);

            let ScreenRectangle {
                x,
                y,
                width,
                height,
            } = command.viewport;
            if width == 0 || height == 0 {
                continue;
            }
            let viewport_height = height;
            rp.set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);

            let ScreenRectangle {
                x,
                y,
                width,
                height,
            } = command.scissor;
            let y = viewport_height - y - height;
            let x0 = x.clamp(0, output_size.0 as i32);
            let y0 = y.clamp(0, output_size.1 as i32);
            let x1 = (x + width).clamp(0, output_size.0 as i32);
            let y1 = (y + height).clamp(0, output_size.1 as i32);
            let w = x1 - x0;
            let h = y1 - y0;
            if w <= 0 || h <= 0 {
                continue;
            }
            rp.set_scissor_rect(x as u32, y as u32, w as u32, h as u32);

            if current_state != Some(command.state) {
                current_state = Some(command.state);
                let pipeline = self
                    .pipelines
                    .get(&command.state)
                    .expect("pipeline not prepared");
                rp.set_pipeline(pipeline);
            }

            for i in 0..2 {
                if cc_features.used_textures[i] {
                    let texture_index = command.texture_index[i].expect("missing texture index");

                    let bind_group = self
                        .texture_bind_groups
                        .get(&texture_index)
                        .expect("texture bind group not prepared");

                    rp.set_bind_group(i as u32, bind_group, &[]);
                }
            }

            rp.set_vertex_buffer(0, command.buffer.slice(..));
            rp.draw(0..command.num_vertices, 0..1);
        }
    }
}
