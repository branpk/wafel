//! A wgpu renderer for the data produced by [crate::interpret] (requires feature `wgpu`).

use std::{
    collections::HashMap,
    fmt::{self, Write},
    ops::Range,
};

use bytemuck::cast_slice;
use wgpu::util::DeviceExt;

use crate::f3d_render_data::*;

#[allow(missing_docs)]
#[derive(Debug)]
pub struct F3DRenderer {
    msaa_samples: u32,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_groups: HashMap<TextureIndex, wgpu::BindGroup>,
    pipelines: HashMap<PipelineId, wgpu::RenderPipeline>,
    output_size_physical: Option<[u32; 2]>,
    scale_factor: f32,
    screen_top_left: [i32; 2],
    screen_size: [i32; 2],
    commands: Vec<DrawCommand<wgpu::Buffer>>,
}

fn label(prefix: &str, pipeline_id: PipelineId) -> &'static str {
    let label = format!("{}-{:?}", prefix, pipeline_id);
    Box::leak(label.into_boxed_str())
}

#[rustfmt::skip]
fn write_fragment_shader_body(s: &mut String, p: &PipelineInfo) -> Result<(), fmt::Error> {
    for i in 0..2 {
        if p.used_textures[i] {
            writeln!(s, "    let tex{} = textureSample(r_texture{}, r_sampler{}, in.uv);", i, i, i)?;
        }
    }

    if p.blend && p.output_color.rgb == p.output_color.a {
        writeln!(s, "    out.color = {};", color_expr(p.output_color.rgb))?;
    } else {
        writeln!(s, "    let rgb = ({}).rgb;", color_expr(p.output_color.rgb))?;
        if p.blend {
            writeln!(s, "    let a = ({}).a;", color_expr(p.output_color.a))?;
            writeln!(s, "    out.color = vec4<f32>(rgb, a);")?;
        }
        else {
            writeln!(s, "    out.color = vec4<f32>(rgb, 1.0);")?;
        }
    }

    if p.texture_edge && p.blend {
        writeln!(s, "    if out.color.a > 0.3 {{")?;
        writeln!(s, "        out.color = vec4<f32>(out.color.rgb, 1.0);")?;
        writeln!(s, "    }} else {{")?;
        writeln!(s, "        discard;")?;
        writeln!(s, "    }}")?;
    }

    if p.fog {
        writeln!(s, "    let fog_mixed = mix(out.color.rgb, in.fog.rgb, in.fog.a);")?;
        writeln!(s, "    out.color = vec4<f32>(fog_mixed, out.color.a);")?;
    }

    // TODO: Noise

    Ok(())
}

fn color_expr(args: [ColorArg; 4]) -> String {
    format!(
        "({} - {}) * {} + {}",
        arg_expr(args[0]),
        arg_expr(args[1]),
        arg_expr(args[2]),
        arg_expr(args[3])
    )
}

fn arg_expr(arg: ColorArg) -> String {
    match arg {
        ColorArg::Zero => "vec4<f32>()".to_string(),
        ColorArg::Input(i) => format!("in.input{}", i),
        ColorArg::Texel0 => "tex0".to_string(),
        ColorArg::Texel0Alpha => "vec4<f32>(tex0.a)".to_string(),
        ColorArg::Texel1 => "tex1".to_string(),
    }
}

#[allow(missing_docs)]
impl F3DRenderer {
    pub fn new(device: &wgpu::Device, msaa_samples: u32) -> Self {
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
            msaa_samples,
            texture_bind_group_layout,
            texture_bind_groups: HashMap::new(),
            pipelines: HashMap::new(),
            output_size_physical: None,
            scale_factor: 1.0,
            screen_top_left: [0, 0],
            screen_size: [320, 240],
            commands: Vec::new(),
        }
    }

    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        pipeline_id: PipelineId,
        p: &PipelineInfo,
    ) -> Result<wgpu::RenderPipeline, fmt::Error> {
        let mut s = String::new();
        writeln!(s, "// Pipeline {:?}:", pipeline_id)?;
        writeln!(s, "/*\n{:#?}\n*/", p)?;
        writeln!(s)?;

        let mut bind_group_layouts: Vec<&wgpu::BindGroupLayout> = Vec::new();
        for i in 0..2 {
            if p.used_textures[i] {
                writeln!(s, "@group({}) @binding(0) var r_sampler{}: sampler;", i, i)?;
                writeln!(
                    s,
                    "@group({}) @binding(1) var r_texture{}: texture_2d<f32>;",
                    i, i
                )?;
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
            let mut offset = 0;
            let mut loc = 0;

            writeln!(s, "struct VertexData {{")?;
            {
                writeln!(s, "    @location({}) pos: vec4<f32>,", loc)?;
                vertex_attributes.push(wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset,
                    shader_location: loc,
                });
                offset += 16;
                loc += 1;

                if p.uses_textures() {
                    writeln!(s, "    @location({}) uv: vec2<f32>,", loc)?;
                    vertex_attributes.push(wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset,
                        shader_location: loc,
                    });
                    offset += 8;
                    loc += 1;
                }

                if p.fog {
                    writeln!(s, "    @location({}) fog: vec4<f32>,", loc)?;
                    vertex_attributes.push(wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset,
                        shader_location: loc,
                    });
                    offset += 16;
                    loc += 1;
                }

                for i in 0..p.num_inputs {
                    writeln!(s, "    @location({}) input{}: vec4<f32>,", loc, i)?;
                    vertex_attributes.push(wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset,
                        shader_location: loc,
                    });
                    offset += 16;
                    loc += 1;
                }
            }
            writeln!(s, "}}")?;
            writeln!(s)?;

            wgpu::VertexBufferLayout {
                array_stride: offset,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attributes,
            }
        };

        {
            writeln!(s, "struct VertexOutput {{")?;
            writeln!(s, "    @builtin(position) position: vec4<f32>,")?;
            let mut location = 0;
            if p.uses_textures() {
                writeln!(s, "    @location({}) uv: vec2<f32>,", location)?;
                location += 1;
            }
            if p.fog {
                writeln!(s, "    @location({}) fog: vec4<f32>,", location)?;
                location += 1;
            }
            for i in 0..p.num_inputs {
                writeln!(s, "    @location({}) input{}: vec4<f32>,", location, i)?;
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
            if p.uses_textures() {
                writeln!(s, "    out.uv = in.uv;")?;
            }
            if p.fog {
                writeln!(s, "    out.fog = in.fog;")?;
            }
            for i in 0..p.num_inputs {
                writeln!(s, "    out.input{} = in.input{};", i, i)?
            }
            writeln!(s, "    return out;")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "struct FragmentOutput {{")?;
            if p.decal {
                writeln!(s, "    @builtin(frag_depth) frag_depth: f32,")?;
            }
            writeln!(s, "    @location(0) color: vec4<f32>,")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        {
            writeln!(s, "@fragment")?;
            writeln!(s, "fn fs_main(in: VertexOutput) -> FragmentOutput {{")?;
            writeln!(s, "    var out = FragmentOutput();")?;
            if p.decal {
                writeln!(s, "    out.frag_depth = in.position.z - 0.001;")?;
            }
            write_fragment_shader_body(&mut s, p)?;
            writeln!(s, "    return out;")?;
            writeln!(s, "}}")?;
            writeln!(s)?;
        }

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label("shader", pipeline_id)),
            source: wgpu::ShaderSource::Wgsl(s.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label("pipeline", pipeline_id)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: match p.cull_mode {
                    CullMode::None => None,
                    CullMode::Front => Some(wgpu::Face::Front),
                    CullMode::Back => Some(wgpu::Face::Back),
                },
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: p.depth_write,
                depth_compare: if p.depth_compare {
                    wgpu::CompareFunction::LessEqual
                } else {
                    wgpu::CompareFunction::Always
                },
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: self.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING).filter(|_| p.blend),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
        });

        Ok(pipeline)
    }

    fn prepare_pipeline(
        &mut self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        pipeline_id: PipelineId,
        pipeline_info: &PipelineInfo,
    ) {
        if !self.pipelines.contains_key(&pipeline_id) {
            let pipeline = self
                .create_pipeline(device, output_format, pipeline_id, pipeline_info)
                .unwrap();
            self.pipelines.insert(pipeline_id, pipeline);
        }
    }

    fn create_sampler(
        &mut self,
        device: &wgpu::Device,
        sampler_state: &SamplerState,
    ) -> wgpu::Sampler {
        let filter = if sampler_state.linear_filter {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        };

        let address_mode = |m| match m {
            WrapMode::Clamp => wgpu::AddressMode::ClampToEdge,
            WrapMode::Repeat => wgpu::AddressMode::Repeat,
            WrapMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        };

        device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: address_mode(sampler_state.u_wrap),
            address_mode_v: address_mode(sampler_state.v_wrap),
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
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
            view_formats: &[],
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
                bytes_per_row: Some(4 * data.width),
                rows_per_image: Some(data.height),
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
        texture_index: TextureIndex,
        texture_state: &TextureState,
    ) {
        if !self.texture_bind_groups.contains_key(&texture_index) {
            let sampler = self.create_sampler(device, &texture_state.sampler);
            let texture = self.create_texture(device, queue, &texture_state.data);

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
        output_size_physical: [u32; 2],
        scale_factor: f32,
        data: &F3DRenderData,
    ) {
        for (&pipeline_id, pipeline_info) in &data.pipelines {
            self.prepare_pipeline(device, output_format, pipeline_id, pipeline_info);
        }

        // Textures may change index across frames
        self.texture_bind_groups.clear();
        for (&texture_index, texture) in &data.textures {
            self.prepare_texture_bind_group(device, queue, texture_index, texture);
        }

        self.output_size_physical = Some(output_size_physical);
        self.scale_factor = scale_factor;
        self.screen_top_left = data.screen_top_left;
        self.screen_size = data.screen_size;

        self.commands.clear();
        for command in &data.commands {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&command.vertex_buffer),
                usage: wgpu::BufferUsages::VERTEX,
            });
            self.commands.push(command.with_buffer(buffer));
        }
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        self.render_command_range(rp, 0..self.commands.len());
    }

    pub fn render_command_range<'r>(
        &'r self,
        rp: &mut wgpu::RenderPass<'r>,
        cmd_indices: Range<usize>,
    ) {
        if self.screen_size[0] <= 0 || self.screen_size[1] <= 0 || cmd_indices.is_empty() {
            return;
        }

        let output_size_physical = self
            .output_size_physical
            .expect("missing call to F3DRenderer::render");
        let scale_factor = self.scale_factor;
        let output_size_logical = output_size_physical.map(|n| (n as f32 / scale_factor) as i32);

        let mut current_pipeline = None;

        for command in &self.commands[cmd_indices] {
            let ScreenRectangle { x, y, w, h } = command.viewport;
            rp.set_viewport(
                (x as f32 + self.screen_top_left[0] as f32) * scale_factor,
                (y as f32 + self.screen_top_left[1] as f32) * scale_factor,
                (w as f32) * scale_factor,
                (h as f32) * scale_factor,
                0.0,
                1.0,
            );

            // Clamp scissor rect to both the screen and the output window.
            let screen_rect = ScreenRectangle {
                x: self.screen_top_left[0],
                y: self.screen_top_left[1],
                w: self.screen_size[0],
                h: self.screen_size[1],
            };
            let output_rect = ScreenRectangle {
                x: 0,
                y: 0,
                w: output_size_logical[0],
                h: output_size_logical[1],
            };
            let clamped_scissor = command
                .scissor
                .translate(self.screen_top_left)
                .clamp(screen_rect)
                .clamp(output_rect);

            if clamped_scissor.w <= 0 || clamped_scissor.h <= 0 {
                continue;
            }

            rp.set_scissor_rect(
                (clamped_scissor.x as f32 * scale_factor) as u32,
                (clamped_scissor.y as f32 * scale_factor) as u32,
                (clamped_scissor.w as f32 * scale_factor) as u32,
                (clamped_scissor.h as f32 * scale_factor) as u32,
            );

            if current_pipeline != Some(command.pipeline) {
                current_pipeline = Some(command.pipeline);
                let pipeline = self
                    .pipelines
                    .get(&command.pipeline)
                    .expect("pipeline not prepared");
                rp.set_pipeline(pipeline);
            }

            for i in 0..2 {
                if let Some(texture_index) = command.textures[i] {
                    let bind_group = self
                        .texture_bind_groups
                        .get(&texture_index)
                        .expect("texture bind group not prepared");
                    rp.set_bind_group(i as u32, bind_group, &[]);
                }
            }

            rp.set_vertex_buffer(0, command.vertex_buffer.slice(..));
            rp.draw(0..command.num_vertices, 0..1);
        }
    }
}
