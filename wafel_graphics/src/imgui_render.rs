use std::{convert::TryInto, fmt, mem::size_of};

use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use wgpu::util::DeviceExt;

// TODO: High dpi

/// A wgpu renderer for Dear Imgui.
#[derive(Debug)]
pub struct ImguiRenderer {
    pipeline: wgpu::RenderPipeline,
    proj_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    font_texture_bind_group: wgpu::BindGroup,
}

/// Per-frame draw data for [ImguiRenderer].
pub struct ImguiPerFrameData {
    output_size: (u32, u32),
    proj_bind_group: wgpu::BindGroup,
    draw_lists: Vec<DrawList>,
}

impl fmt::Debug for ImguiPerFrameData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImguiPerFrameData")
            .field("output_size", &self.output_size)
            .finish_non_exhaustive()
    }
}

struct DrawList {
    commands: Vec<imgui::DrawCmd>,
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    tex_coord: [f32; 2],
    color: [u8; 4],
}

impl ImguiRenderer {
    /// Create the renderer.
    pub fn new(
        context: &mut imgui::Context,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let proj_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // r_proj
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(64),
                        },
                        count: None,
                    },
                ],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // r_sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    },
                    // r_texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let shader = device.create_shader_module(&wgpu::include_wgsl!("../shaders/imgui.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("imgui-pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&proj_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        // pos
                        wgpu::VertexAttribute {
                            offset: offset_of!(Vertex, pos) as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 0,
                        },
                        // tex_coord
                        wgpu::VertexAttribute {
                            offset: offset_of!(Vertex, tex_coord) as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 1,
                        },
                        // color
                        wgpu::VertexAttribute {
                            offset: offset_of!(Vertex, color) as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Unorm8x4,
                            shader_location: 2,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        let mut fonts = context.fonts();
        let font_texture = fonts.build_rgba32_texture();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: font_texture.width,
                height: font_texture.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            font_texture.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(
                    (4 * font_texture.width)
                        .try_into()
                        .expect("font texture has zero width"),
                ),
                rows_per_image: Some(
                    font_texture
                        .height
                        .try_into()
                        .expect("font texture has zero height"),
                ),
            },
            wgpu::Extent3d {
                width: font_texture.width,
                height: font_texture.height,
                depth_or_array_layers: 1,
            },
        );

        let font_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
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

        Self {
            pipeline,
            proj_bind_group_layout,
            texture_bind_group_layout,
            font_texture_bind_group,
        }
    }

    /// Prepare per-frame draw data.
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        output_size: (u32, u32),
        draw_data: &imgui::DrawData,
    ) -> ImguiPerFrameData {
        let [w, h] = draw_data.display_size;
        let proj_matrix: [[f32; 4]; 4] = [
            [2.0 / w, 0.0, 0.0, 0.0],
            [0.0, -2.0 / h, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];
        let proj_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&proj_matrix),
            usage: wgpu::BufferUsage::UNIFORM,
        });
        let proj_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.proj_bind_group_layout,
            entries: &[
                // r_proj
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &proj_matrix_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let draw_lists: Vec<DrawList> = draw_data
            .draw_lists()
            .map(|draw_list| {
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: cast_slice(draw_list.idx_buffer()),
                    usage: wgpu::BufferUsage::INDEX,
                });
                let vertices: &[Vertex] = unsafe { draw_list.transmute_vtx_buffer() };
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: cast_slice(vertices),
                    usage: wgpu::BufferUsage::VERTEX,
                });
                DrawList {
                    commands: draw_list.commands().collect(),
                    index_buffer,
                    vertex_buffer,
                }
            })
            .collect();

        ImguiPerFrameData {
            output_size,
            proj_bind_group,
            draw_lists,
        }
    }

    /// Render a frame.
    pub fn render<'r>(
        &'r self,
        render_pass: &mut wgpu::RenderPass<'r>,
        data: &'r ImguiPerFrameData,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &data.proj_bind_group, &[]);
        render_pass.set_bind_group(1, &self.font_texture_bind_group, &[]);

        for draw_list in &data.draw_lists {
            render_pass
                .set_index_buffer(draw_list.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, draw_list.vertex_buffer.slice(..));

            for command in &draw_list.commands {
                match command {
                    imgui::DrawCmd::Elements { count, cmd_params } => {
                        let [mut clip_x0, mut clip_y0, mut clip_x1, mut clip_y1] =
                            cmd_params.clip_rect;

                        clip_x0 = clip_x0.min(data.output_size.0 as f32);
                        clip_y0 = clip_y0.min(data.output_size.1 as f32);
                        clip_x1 = clip_x1.min(data.output_size.0 as f32);
                        clip_y1 = clip_y1.min(data.output_size.1 as f32);

                        if clip_x0 >= clip_x1 || clip_y0 >= clip_y1 {
                            continue;
                        }

                        render_pass.set_scissor_rect(
                            clip_x0 as u32,
                            clip_y0 as u32,
                            (clip_x1 - clip_x0) as u32,
                            (clip_y1 - clip_y0) as u32,
                        );

                        let index_offset = cmd_params.idx_offset as u32;
                        render_pass.draw_indexed(
                            index_offset..index_offset + *count as u32,
                            0,
                            0..1,
                        );
                    }
                    _ => unimplemented!(),
                }
            }
        }
    }
}
