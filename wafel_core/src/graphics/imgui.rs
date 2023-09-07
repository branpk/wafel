//! Renderer for imgui.

use bytemuck::cast_slice;
use std::{convert::TryInto, iter};
use wgpu::util::DeviceExt;

/// Initial configuration for imgui draw lists.
#[derive(Debug, Clone)]
pub struct ImguiConfig {
    /// The size of an index element (2 for ushort or 4 for uint).
    pub index_size: usize,

    /// The size of a vertex element.
    pub vertex_size: usize,
    /// The offset of position in the vertex struct.
    pub vertex_pos_offset: usize,
    /// The offset of tex coord in the vertex struct.
    pub vertex_tex_coord_offset: usize,
    /// The offset of color in the vertex struct.
    pub vertex_color_offset: usize,

    /// The width in pixels of the font texture.
    pub font_texture_width: u32,
    /// The height in pixels of the font texture.
    pub font_texture_height: u32,
    /// The RGBA32 data for the font texture.
    pub font_texture_data: Vec<u8>,
}

/// The draw data for one frame.
#[derive(Debug, Clone)]
pub struct ImguiDrawData {
    /// The command lists to draw.
    pub command_lists: Vec<ImguiCommandList>,
}

/// An imgui command list, which consists of commands that use the same index and vertex buffers.
#[derive(Debug, Clone)]
pub struct ImguiCommandList {
    /// The bytes of the index buffer.
    pub index_buffer: Vec<u8>,
    /// The bytes of the vertex buffer.
    pub vertex_buffer: Vec<u8>,
    /// The commands in this list.
    pub commands: Vec<ImguiCommand>,
}

/// A single draw command.
#[derive(Debug, Clone)]
pub struct ImguiCommand {
    /// The texture to use (must equal `IMGUI_FONT_TEXTURE_ID`).
    pub texture_id: u32,
    /// The clip rectangle in screen coordinates.
    pub clip_rect: (f32, f32, f32, f32),
    /// The number of indices.
    pub elem_count: u32,
}

/// The main font texture.
pub const IMGUI_FONT_TEXTURE_ID: u32 = 1;

/// A renderer for imgui frame data.
#[derive(Debug)]
pub struct ImguiRenderer {
    pipeline: wgpu::RenderPipeline,
    proj_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    font_texture_bind_group: wgpu::BindGroup,
    index_format: wgpu::IndexFormat,
}

impl ImguiRenderer {
    /// Create a renderer with the given config.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        config: &ImguiConfig,
    ) -> Self {
        let proj_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // u_Proj
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
                ],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // u_Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // u_Texture
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

        let index_format = match config.index_size {
            2 => wgpu::IndexFormat::Uint16,
            4 => wgpu::IndexFormat::Uint32,
            n => unimplemented!("{}", n),
        };

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../../assets/shaders/imgui.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&proj_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: config.vertex_size as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // a_Pos
                        wgpu::VertexAttribute {
                            offset: config.vertex_pos_offset as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 0,
                        },
                        // a_TexCoord
                        wgpu::VertexAttribute {
                            offset: config.vertex_tex_coord_offset as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 1,
                        },
                        // a_Color
                        wgpu::VertexAttribute {
                            offset: config.vertex_color_offset as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Unorm8x4,
                            shader_location: 2,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: config.font_texture_width,
                height: config.font_texture_height,
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
            &config.font_texture_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((4 * config.font_texture_width).try_into().unwrap()),
                rows_per_image: Some(config.font_texture_height.try_into().unwrap()),
            },
            wgpu::Extent3d {
                width: config.font_texture_width,
                height: config.font_texture_height,
                depth_or_array_layers: 1,
            },
        );

        let font_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
            entries: &[
                // u_Sampler
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                // u_Texture
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
            index_format,
        }
    }

    /// Render the given draw data.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        draw_data: &ImguiDrawData,
    ) {
        let proj_matrix: [[f32; 4]; 4] = [
            [2.0 / output_size.0 as f32, 0.0, 0.0, 0.0],
            [0.0, -2.0 / output_size.1 as f32, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];
        let proj_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(&proj_matrix),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let proj_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.proj_bind_group_layout,
            entries: &[
                // u_Proj
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

        let buffers: Vec<(wgpu::Buffer, wgpu::Buffer)> = draw_data
            .command_lists
            .iter()
            .map(|command_list| {
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &command_list.index_buffer,
                    usage: wgpu::BufferUsages::INDEX,
                });
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &command_list.vertex_buffer,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                (index_buffer, vertex_buffer)
            })
            .collect();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &proj_bind_group, &[]);
            render_pass.set_bind_group(1, &self.font_texture_bind_group, &[]);

            for (command_list, (index_buffer, vertex_buffer)) in
                draw_data.command_lists.iter().zip(buffers.iter())
            {
                render_pass.set_index_buffer(index_buffer.slice(..), self.index_format);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

                let mut initial_index = 0;

                for command in &command_list.commands {
                    assert_eq!(command.texture_id, IMGUI_FONT_TEXTURE_ID);

                    let (mut clip_x0, mut clip_y0, mut clip_x1, mut clip_y1) = command.clip_rect;
                    clip_x0 = clip_x0.min(output_size.0 as f32);
                    clip_y0 = clip_y0.min(output_size.1 as f32);
                    clip_x1 = clip_x1.min(output_size.0 as f32);
                    clip_y1 = clip_y1.min(output_size.1 as f32);

                    #[allow(clippy::float_cmp)]
                    if clip_x0 == clip_x1 || clip_y0 == clip_y1 {
                        continue;
                    }

                    render_pass.set_scissor_rect(
                        clip_x0 as u32,
                        clip_y0 as u32,
                        (clip_x1 - clip_x0) as u32,
                        (clip_y1 - clip_y0) as u32,
                    );

                    render_pass.draw_indexed(
                        initial_index..initial_index + command.elem_count,
                        0,
                        0..1,
                    );

                    initial_index += command.elem_count;
                }
            }
        }

        let command_buffer = encoder.finish();
        queue.submit(iter::once(command_buffer));
    }
}
