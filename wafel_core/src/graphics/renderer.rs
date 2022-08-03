use crate::geo::{direction_to_pitch_yaw, Matrix4f, Point3f, StoredPoint3f, Vector3f, Vector4f};
use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use nalgebra::distance;
use std::{cmp::Ordering, f32::consts::PI, iter, mem::size_of};
use wgpu::{util::DeviceExt, MultisampleState};

use super::scene::{BirdsEyeCamera, Camera, ObjectPath, RotateCamera, Scene, SurfaceType};

const NUM_OUTPUT_SAMPLES: u32 = 4;
const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
struct Vertex {
    pos: StoredPoint3f,
    color: [f32; 4],
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}

impl Vertex {
    fn new(pos: impl Into<StoredPoint3f>, color: [f32; 4]) -> Self {
        Self {
            pos: pos.into(),
            color,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
struct ScreenDotInstance {
    center: StoredPoint3f,
    radius: [f32; 2],
    color: [f32; 4],
}

unsafe impl Zeroable for ScreenDotInstance {}
unsafe impl Pod for ScreenDotInstance {}

#[derive(Debug)]
struct SceneBundle {
    transform_bind_group: wgpu::BindGroup,
    surface_vertex_buffer: (usize, wgpu::Buffer),
    hidden_surface_vertex_buffer: (usize, wgpu::Buffer),
    wall_hitbox_vertex_buffer: (usize, wgpu::Buffer),
    wall_hitbox_outline_vertex_buffer: (usize, wgpu::Buffer),
    object_vertex_buffer: (usize, wgpu::Buffer),
    object_path_line_vertex_buffer: (usize, wgpu::Buffer),
    object_path_dot_instance_buffer: (usize, wgpu::Buffer),
    camera_target_line_vertex_buffer: (usize, wgpu::Buffer),
    camera_target_dot_instance_buffer: (usize, wgpu::Buffer),
    unit_square_vertex_buffer: (usize, wgpu::Buffer),
}

/// A renderer for the game views.
#[derive(Debug)]
pub struct Renderer {
    multisample_texture: Option<((u32, u32), wgpu::Texture)>,
    depth_texture: Option<((u32, u32), wgpu::Texture)>,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    color_line_pipeline: wgpu::RenderPipeline,
    screen_dot_pipeline: wgpu::RenderPipeline,
    surface_pipeline: wgpu::RenderPipeline,
    hidden_surface_pipeline: wgpu::RenderPipeline,
    wall_hitbox_pipeline: wgpu::RenderPipeline,
    wall_hitbox_depth_pass_pipeline: wgpu::RenderPipeline,
    unit_square_pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    /// Initialize the renderer.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let transform_bind_group_layout =
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
                    // u_View
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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

        let color_line_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            true,
            true,
            wgpu::PrimitiveTopology::LineList,
        );

        let screen_dot_pipeline =
            create_screen_dot_pipeline(device, &transform_bind_group_layout, output_format);

        let surface_pipeline =
            create_surface_pipeline(device, &transform_bind_group_layout, output_format, true);
        let hidden_surface_pipeline =
            create_surface_pipeline(device, &transform_bind_group_layout, output_format, false);
        let wall_hitbox_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            true,
            true,
            wgpu::PrimitiveTopology::TriangleList,
        );
        let wall_hitbox_depth_pass_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            false,
            true,
            wgpu::PrimitiveTopology::TriangleList,
        );

        let unit_square_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            true,
            false,
            wgpu::PrimitiveTopology::LineList,
        );

        Self {
            multisample_texture: None,
            depth_texture: None,
            transform_bind_group_layout,
            color_line_pipeline,
            screen_dot_pipeline,
            surface_pipeline,
            hidden_surface_pipeline,
            wall_hitbox_pipeline,
            wall_hitbox_depth_pass_pipeline,
            unit_square_pipeline,
        }
    }

    /// Render the given scenes.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        output_format: wgpu::TextureFormat,
        scenes: &[&Scene],
    ) {
        if self
            .multisample_texture
            .as_ref()
            .filter(|(size, _)| size == &output_size)
            .is_none()
        {
            self.multisample_texture = Some((
                output_size,
                create_multisample_texture(device, output_format, output_size),
            ));
        }
        let multisample_texture_view = self
            .multisample_texture
            .as_ref()
            .unwrap()
            .1
            .create_view(&wgpu::TextureViewDescriptor::default());

        if self
            .depth_texture
            .as_ref()
            .filter(|(size, _)| size == &output_size)
            .is_none()
        {
            self.depth_texture = Some((output_size, create_depth_texture(device, output_size)));
        }
        let depth_texture_view = self
            .depth_texture
            .as_ref()
            .unwrap()
            .1
            .create_view(&wgpu::TextureViewDescriptor::default());

        let scene_bundles: Vec<SceneBundle> = scenes
            .iter()
            .map(|scene| {
                let (proj_matrix, view_matrix) = match &scene.camera {
                    Camera::Rotate(camera) => rotate_transforms(camera, output_size),
                    Camera::BirdsEye(camera) => birds_eye_transforms(camera, output_size),
                };

                let proj_matrix_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(proj_matrix.as_slice()),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                let view_matrix_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(view_matrix.as_slice()),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.transform_bind_group_layout,
                    entries: &[
                        // u_Proj
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: proj_matrix_buffer.as_entire_binding(),
                        },
                        // u_View
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: view_matrix_buffer.as_entire_binding(),
                        },
                    ],
                });

                let (surface_vertices, hidden_surface_vertices) = get_surface_vertices(scene);
                let surface_vertex_buffer = upload_vertex_buffer(device, &surface_vertices);
                let hidden_surface_vertex_buffer =
                    upload_vertex_buffer(device, &hidden_surface_vertices);

                let (wall_hitbox_vertices, wall_hitbox_outline_vertices) =
                    get_wall_hitbox_vertices(scene);
                let wall_hitbox_vertex_buffer = upload_vertex_buffer(device, &wall_hitbox_vertices);
                let wall_hitbox_outline_vertex_buffer =
                    upload_vertex_buffer(device, &wall_hitbox_outline_vertices);

                let object_vertices = get_object_vertices(scene);
                let object_vertex_buffer = upload_vertex_buffer(device, &object_vertices);

                let object_path_line_vertices = get_object_path_line_vertices(scene);
                let object_path_line_vertex_buffer =
                    upload_vertex_buffer(device, &object_path_line_vertices);

                let object_path_dot_instances = get_object_path_dot_instances(scene);
                let object_path_dot_instance_buffer =
                    upload_vertex_buffer(device, &object_path_dot_instances);

                let (camera_target_line_vertices, camera_target_dot_instances) =
                    get_camera_target_vertices(scene);
                let camera_target_line_vertex_buffer =
                    upload_vertex_buffer(device, &camera_target_line_vertices);
                let camera_target_dot_instance_buffer =
                    upload_vertex_buffer(device, &camera_target_dot_instances);

                let unit_square_vertices = get_unit_square_vertices(scene);
                let unit_square_vertex_buffer = upload_vertex_buffer(device, &unit_square_vertices);

                SceneBundle {
                    transform_bind_group,
                    surface_vertex_buffer,
                    hidden_surface_vertex_buffer,
                    wall_hitbox_vertex_buffer,
                    wall_hitbox_outline_vertex_buffer,
                    object_vertex_buffer,
                    object_path_line_vertex_buffer,
                    object_path_dot_instance_buffer,
                    camera_target_line_vertex_buffer,
                    camera_target_dot_instance_buffer,
                    unit_square_vertex_buffer,
                }
            })
            .collect();

        let screen_dot_offset_vertex_buffer =
            upload_vertex_buffer(device, &get_screen_dot_offset_vertices());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &multisample_texture_view,
                    resolve_target: Some(output_view),
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            for (scene, bundle) in scenes.iter().zip(&scene_bundles) {
                let mut viewport = scene.viewport.clone();

                // Viewport size can become out of sync with output_size on the frame that the
                // window is resized
                viewport.x = viewport.x.min(output_size.0 as f32);
                viewport.y = viewport.y.min(output_size.1 as f32);
                viewport.width = viewport.width.min(output_size.0 as f32 - viewport.x);
                viewport.height = viewport.height.min(output_size.1 as f32 - viewport.y);
                if viewport.width == 0.0 || viewport.height == 0.0 {
                    continue;
                }

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

                render_pass.set_bind_group(0, &bundle.transform_bind_group, &[]);

                render_pass.set_pipeline(&self.surface_pipeline);
                render_pass.set_vertex_buffer(0, bundle.surface_vertex_buffer.1.slice(..));
                render_pass.draw(0..bundle.surface_vertex_buffer.0 as u32, 0..1);

                render_pass.set_pipeline(&self.color_line_pipeline);
                render_pass.set_vertex_buffer(0, bundle.object_vertex_buffer.1.slice(..));
                render_pass.draw(0..bundle.object_vertex_buffer.0 as u32, 0..1);

                render_pass.set_pipeline(&self.color_line_pipeline);
                render_pass.set_vertex_buffer(0, bundle.object_path_line_vertex_buffer.1.slice(..));
                render_pass.draw(0..bundle.object_path_line_vertex_buffer.0 as u32, 0..1);

                render_pass.set_pipeline(&self.screen_dot_pipeline);
                render_pass
                    .set_vertex_buffer(0, bundle.object_path_dot_instance_buffer.1.slice(..));
                render_pass.set_vertex_buffer(1, screen_dot_offset_vertex_buffer.1.slice(..));
                render_pass.draw(
                    0..screen_dot_offset_vertex_buffer.0 as u32,
                    0..bundle.object_path_dot_instance_buffer.0 as u32,
                );

                render_pass.set_pipeline(&self.color_line_pipeline);
                render_pass
                    .set_vertex_buffer(0, bundle.camera_target_line_vertex_buffer.1.slice(..));
                render_pass.draw(0..bundle.camera_target_line_vertex_buffer.0 as u32, 0..1);

                render_pass.set_pipeline(&self.screen_dot_pipeline);
                render_pass
                    .set_vertex_buffer(0, bundle.camera_target_dot_instance_buffer.1.slice(..));
                render_pass.set_vertex_buffer(1, screen_dot_offset_vertex_buffer.1.slice(..));
                render_pass.draw(
                    0..screen_dot_offset_vertex_buffer.0 as u32,
                    0..bundle.camera_target_dot_instance_buffer.0 as u32,
                );

                if scene.wall_hitbox_radius > 0.0 {
                    // Render lines first since tris write to z buffer
                    render_pass.set_pipeline(&self.color_line_pipeline);
                    render_pass
                        .set_vertex_buffer(0, bundle.wall_hitbox_outline_vertex_buffer.1.slice(..));
                    render_pass.draw(0..bundle.wall_hitbox_outline_vertex_buffer.0 as u32, 0..1);

                    // When two wall hitboxes overlap, we should not increase the opacity within
                    // their region of overlap (preference).
                    // First pass writes only to depth buffer to ensure that only the closest
                    // hitbox triangles are drawn, then second pass draws them.
                    render_pass.set_vertex_buffer(0, bundle.wall_hitbox_vertex_buffer.1.slice(..));
                    render_pass.set_pipeline(&self.wall_hitbox_depth_pass_pipeline);
                    render_pass.draw(0..bundle.wall_hitbox_vertex_buffer.0 as u32, 0..1);
                    render_pass.set_pipeline(&self.wall_hitbox_pipeline);
                    render_pass.draw(0..bundle.wall_hitbox_vertex_buffer.0 as u32, 0..1);
                }

                render_pass.set_pipeline(&self.hidden_surface_pipeline);
                render_pass.set_vertex_buffer(0, bundle.hidden_surface_vertex_buffer.1.slice(..));
                render_pass.draw(0..bundle.hidden_surface_vertex_buffer.0 as u32, 0..1);

                render_pass.set_pipeline(&self.unit_square_pipeline);
                render_pass.set_vertex_buffer(0, bundle.unit_square_vertex_buffer.1.slice(..));
                render_pass.draw(0..bundle.unit_square_vertex_buffer.0 as u32, 0..1);
            }
        }

        let command_buffer = encoder.finish();
        queue.submit(iter::once(command_buffer));
    }
}

fn upload_vertex_buffer<T: Pod>(device: &wgpu::Device, vertices: &[T]) -> (usize, wgpu::Buffer) {
    (
        vertices.len(),
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }),
    )
}

fn create_multisample_texture(
    device: &wgpu::Device,
    output_format: wgpu::TextureFormat,
    output_size: (u32, u32),
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: output_size.0,
            height: output_size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: NUM_OUTPUT_SAMPLES,
        dimension: wgpu::TextureDimension::D2,
        format: output_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    })
}

fn create_depth_texture(device: &wgpu::Device, output_size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: output_size.0,
            height: output_size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: NUM_OUTPUT_SAMPLES,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    })
}

fn create_surface_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    depth_write_enabled: bool,
) -> wgpu::RenderPipeline {
    let shader =
        device.create_shader_module(wgpu::include_wgsl!("../../assets/shaders/surface.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    // a_Pos
                    wgpu::VertexAttribute {
                        offset: offset_of!(Vertex, pos) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float32x3,
                        shader_location: 0,
                    },
                    // a_Color
                    wgpu::VertexAttribute {
                        offset: offset_of!(Vertex, color) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float32x4,
                        shader_location: 1,
                    },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_TEXTURE_FORMAT,
            depth_write_enabled,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: NUM_OUTPUT_SAMPLES,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
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
                write_mask: wgpu::ColorWrites::ALL,
            }],
        }),
        multiview: None,
    })
}

fn create_color_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    color_write_enabled: bool,
    depth_test_enabled: bool,
    primitive_topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    let shader =
        device.create_shader_module(wgpu::include_wgsl!("../../assets/shaders/color.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    // a_Pos
                    wgpu::VertexAttribute {
                        offset: offset_of!(Vertex, pos) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float32x3,
                        shader_location: 0,
                    },
                    // a_Color
                    wgpu::VertexAttribute {
                        offset: offset_of!(Vertex, color) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float32x4,
                        shader_location: 1,
                    },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: primitive_topology,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_TEXTURE_FORMAT,
            depth_write_enabled: depth_test_enabled,
            depth_compare: if depth_test_enabled {
                wgpu::CompareFunction::LessEqual
            } else {
                wgpu::CompareFunction::Always
            },
            stencil: wgpu::StencilState::default(),
            bias: Default::default(),
        }),
        multisample: MultisampleState {
            count: NUM_OUTPUT_SAMPLES,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
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
                write_mask: if color_write_enabled {
                    wgpu::ColorWrites::ALL
                } else {
                    wgpu::ColorWrites::empty()
                },
            }],
        }),
        multiview: None,
    })
}

fn create_screen_dot_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader =
        device.create_shader_module(wgpu::include_wgsl!("../../assets/shaders/screen_dot.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: size_of::<ScreenDotInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        // a_Center
                        wgpu::VertexAttribute {
                            offset: offset_of!(ScreenDotInstance, center) as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Float32x3,
                            shader_location: 0,
                        },
                        // a_Radius
                        wgpu::VertexAttribute {
                            offset: offset_of!(ScreenDotInstance, radius) as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 1,
                        },
                        // a)Color
                        wgpu::VertexAttribute {
                            offset: offset_of!(ScreenDotInstance, color) as wgpu::BufferAddress,
                            format: wgpu::VertexFormat::Float32x4,
                            shader_location: 2,
                        },
                    ],
                },
                wgpu::VertexBufferLayout {
                    array_stride: size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // a_offset
                        wgpu::VertexAttribute {
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 3,
                        },
                    ],
                },
            ],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_TEXTURE_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: NUM_OUTPUT_SAMPLES,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
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
                write_mask: wgpu::ColorWrites::ALL,
            }],
        }),
        multiview: None,
    })
}

fn rotate_transforms(camera: &RotateCamera, output_size: (u32, u32)) -> (Matrix4f, Matrix4f) {
    let camera_pos = Point3f::new(camera.pos[0], camera.pos[1], camera.pos[2]);
    let target_pos = Point3f::new(camera.target[0], camera.target[1], camera.target[2]);

    let dist_to_target = distance(&camera_pos, &target_pos);
    let dist_to_far_corner = distance(
        &Point3f::from(camera_pos.coords.abs()),
        &Point3f::new(-8191.0, -8191.0, -8191.0),
    );
    let far = dist_to_far_corner * 0.95;
    let near = (dist_to_target * 0.1).min(1000.0);
    let proj_matrix = Matrix4f::new_perspective(
        output_size.0 as f32 / output_size.1 as f32,
        camera.fov_y,
        near,
        far,
    );

    let (pitch, yaw) = direction_to_pitch_yaw(&(target_pos - camera_pos).normalize());
    let view_matrix = Matrix4f::new_rotation(PI * Vector3f::y())
        * Matrix4f::new_rotation(pitch * Vector3f::x())
        * Matrix4f::new_rotation(-yaw * Vector3f::y())
        * Matrix4f::new_translation(&-camera_pos.coords);

    (proj_matrix, view_matrix)
}

fn birds_eye_transforms(camera: &BirdsEyeCamera, output_size: (u32, u32)) -> (Matrix4f, Matrix4f) {
    // world x = screen up, world z = screen right
    let rotation =
        Matrix4f::from_columns(&[Vector4f::y(), -Vector4f::z(), Vector4f::x(), Vector4f::w()]);
    let scaling = Matrix4f::new_nonuniform_scaling(&Vector3f::new(
        2.0 / (camera.span_y * output_size.0 as f32 / output_size.1 as f32),
        2.0 / camera.span_y,
        1.0 / 40_000.0,
    ));
    let proj_matrix = scaling * rotation;

    let view_matrix = Matrix4f::new_translation(&-camera.pos.coords);

    (proj_matrix, view_matrix)
}

fn get_screen_dot_offset_vertices() -> Vec<[f32; 2]> {
    let mut vertices = Vec::new();

    let num_edges = 12;
    for i in 0..num_edges {
        let a0 = i as f32 / num_edges as f32 * 2.0 * PI;
        let a1 = (i + 1) as f32 / num_edges as f32 * 2.0 * PI;

        vertices.extend(&[[0.0, 0.0], [a0.cos(), a0.sin()], [a1.cos(), a1.sin()]]);
    }

    vertices
}

fn get_surface_vertices(scene: &Scene) -> (Vec<Vertex>, Vec<Vertex>) {
    let mut surface_vertices: Vec<Vertex> = Vec::new();
    let mut hidden_surface_vertices: Vec<Vertex> = Vec::new();

    for (i, surface) in scene.surfaces.iter().enumerate() {
        let hidden = scene.hidden_surfaces.contains(&i);
        let hovered = scene.hovered_surface == Some(i);

        let mut color = match surface.ty {
            SurfaceType::Floor => [0.5, 0.5, 1.0, 1.0],
            SurfaceType::Ceiling => [1.0, 0.5, 0.5, 1.0],
            SurfaceType::WallXProj => [0.3, 0.8, 0.3, 1.0],
            SurfaceType::WallZProj => [0.15, 0.4, 0.15, 1.0],
        };

        if hidden {
            let scale = 1.5;
            color[0] *= scale;
            color[1] *= scale;
            color[2] *= scale;
            color[3] = if hovered { 0.1 } else { 0.0 };
        }

        if hovered {
            let boost = if surface.ty == SurfaceType::Floor {
                0.08
            } else {
                0.2
            };
            color[0] += boost;
            color[1] += boost;
            color[2] += boost;
        }

        for pos in &surface.vertices {
            let vertex = Vertex { pos: *pos, color };
            if hidden {
                hidden_surface_vertices.push(vertex);
            } else {
                surface_vertices.push(vertex);
            }
        }
    }

    (surface_vertices, hidden_surface_vertices)
}

fn get_wall_hitbox_vertices(scene: &Scene) -> (Vec<Vertex>, Vec<Vertex>) {
    let mut wall_hitbox_vertices: Vec<Vertex> = Vec::new();
    let mut wall_hitbox_outline_vertices: Vec<Vertex> = Vec::new();

    for (i, surface) in scene.surfaces.iter().enumerate() {
        if scene.hidden_surfaces.contains(&i) {
            continue;
        }

        let proj_dir: Vector3f;
        let color: [f32; 4];
        match surface.ty {
            SurfaceType::Floor => continue,
            SurfaceType::Ceiling => continue,
            SurfaceType::WallXProj => {
                proj_dir = Vector3f::x();
                color = [0.3, 0.8, 0.3, 0.4];
            }
            SurfaceType::WallZProj => {
                proj_dir = Vector3f::z();
                color = [0.15, 0.4, 0.15, 0.4];
            }
        };
        let outline_color = [0.0, 0.0, 0.0, 0.5];

        let proj_dist = scene.wall_hitbox_radius / surface.normal.dot(&proj_dir);

        let wall_vertices = [
            surface.vertices[0].0,
            surface.vertices[1].0,
            surface.vertices[2].0,
        ];
        let ext_vertices = [
            wall_vertices[0] + proj_dist * proj_dir,
            wall_vertices[1] + proj_dist * proj_dir,
            wall_vertices[2] + proj_dist * proj_dir,
        ];
        let int_vertices = [
            wall_vertices[0] - proj_dist * proj_dir,
            wall_vertices[1] - proj_dist * proj_dir,
            wall_vertices[2] - proj_dist * proj_dir,
        ];

        wall_hitbox_vertices.extend(&[
            Vertex::new(ext_vertices[0], color),
            Vertex::new(ext_vertices[1], color),
            Vertex::new(ext_vertices[2], color),
        ]);
        wall_hitbox_vertices.extend(&[
            Vertex::new(int_vertices[0], color),
            Vertex::new(int_vertices[1], color),
            Vertex::new(int_vertices[2], color),
        ]);

        wall_hitbox_outline_vertices.extend(&[
            Vertex::new(ext_vertices[0], outline_color),
            Vertex::new(ext_vertices[1], outline_color),
            Vertex::new(ext_vertices[2], outline_color),
        ]);
        wall_hitbox_outline_vertices.extend(&[
            Vertex::new(int_vertices[0], outline_color),
            Vertex::new(int_vertices[1], outline_color),
            Vertex::new(int_vertices[2], outline_color),
        ]);

        // TODO: Helper for calculating bump sizes
        let camera_dist = match &scene.camera {
            Camera::Rotate(camera) => distance(&int_vertices[0], &camera.pos),
            Camera::BirdsEye(_camera) => 1000.0,
        };

        for i0 in 0..3 {
            let i1 = (i0 + 1) % 3;

            // Bump slightly inward. This prevents flickering with floors and adjacent
            // walls
            let mut bump = 0.1 * camera_dist / 1000.0;
            if surface.ty == SurfaceType::WallZProj {
                bump *= 2.0; // Avoid flickering between x and z projected wall hitboxes
            }

            let vertices = [int_vertices[i0], int_vertices[i1], ext_vertices[i0]];
            let normal = (vertices[1] - vertices[0])
                .cross(&(vertices[2] - vertices[0]))
                .normalize();
            for vertex in &vertices {
                wall_hitbox_vertices.push(Vertex::new(vertex - bump * normal, color));
            }

            let vertices = [ext_vertices[i0], int_vertices[i1], ext_vertices[i1]];
            let normal = (vertices[1] - vertices[0])
                .cross(&(vertices[2] - vertices[0]))
                .normalize();
            for vertex in &vertices {
                wall_hitbox_vertices.push(Vertex::new(vertex - bump * normal, color));
            }

            wall_hitbox_outline_vertices.extend(&[
                Vertex::new(int_vertices[i0], outline_color),
                Vertex::new(ext_vertices[i0], outline_color),
            ]);
            wall_hitbox_outline_vertices.extend(&[
                Vertex::new(int_vertices[i0], outline_color),
                Vertex::new(int_vertices[i1], outline_color),
            ]);
            wall_hitbox_outline_vertices.extend(&[
                Vertex::new(ext_vertices[i0], outline_color),
                Vertex::new(ext_vertices[i1], outline_color),
            ]);
        }
    }

    (wall_hitbox_vertices, wall_hitbox_outline_vertices)
}

fn get_object_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    for object in &scene.objects {
        let color = [1.0, 0.0, 0.0, 1.0];

        let pos = object.pos.0;
        vertices.extend(&[
            Vertex::new(pos, color),
            Vertex::new(pos + Vector3f::new(0.0, object.hitbox_height, 0.0), color),
        ]);

        if object.hitbox_radius > 0.0 {
            let num_edges = 64;
            for i in 0..num_edges {
                let a0 = i as f32 / num_edges as f32 * 2.0 * PI;
                let a1 = (i + 1) as f32 / num_edges as f32 * 2.0 * PI;

                let offset0 = object.hitbox_radius * Vector3f::new(a0.sin(), 0.0, a0.cos());
                let offset1 = object.hitbox_radius * Vector3f::new(a1.sin(), 0.0, a1.cos());

                vertices.extend(&[
                    Vertex::new(pos + offset0, color),
                    Vertex::new(pos + offset1, color),
                ]);
            }
        }
    }

    vertices
}

fn get_object_path_line_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let offset = Vector3f::new(0.0, 0.01, 0.0);

    for path in &scene.object_paths {
        for (index, node) in path.nodes.iter().enumerate() {
            let color = [0.5, 0.0, 0.0, get_path_alpha(path, index)];

            vertices.push(Vertex::new(node.pos.0 + offset, color));

            for step in &node.quarter_steps {
                vertices.push(Vertex::new(step.intended_pos.0 + offset, color));
                vertices.push(Vertex::new(step.result_pos.0 + offset, color));
            }
        }
    }

    vertices.windows(2).flatten().cloned().collect()
}

fn get_object_path_dot_instances(scene: &Scene) -> Vec<ScreenDotInstance> {
    let mut instances = Vec::new();

    for path in &scene.object_paths {
        for (index, node) in path.nodes.iter().enumerate() {
            let alpha = get_path_alpha(path, index);

            let y_radius = 0.01;
            let x_radius = y_radius * scene.viewport.height as f32 / scene.viewport.width as f32;
            instances.push(ScreenDotInstance {
                center: node.pos,
                radius: [x_radius, y_radius],
                color: [1.0, 0.0, 0.0, alpha],
            });

            for step in &node.quarter_steps {
                let y_radius = 0.008;
                let x_radius =
                    y_radius * scene.viewport.height as f32 / scene.viewport.width as f32;

                if step.intended_pos != step.result_pos {
                    instances.push(ScreenDotInstance {
                        center: step.intended_pos,
                        radius: [x_radius, y_radius],
                        color: [0.8, 0.5, 0.8, alpha],
                    });
                }

                if index == path.nodes.len() - 1 || step.result_pos != path.nodes[index + 1].pos {
                    instances.push(ScreenDotInstance {
                        center: step.result_pos,
                        radius: [x_radius, y_radius],
                        color: [1.0, 0.5, 0.0, alpha],
                    });
                }
            }
        }
    }

    instances
}

fn get_path_alpha(path: &ObjectPath, index: usize) -> f32 {
    let rel_index = index as isize - path.root_index as isize;
    let t = match rel_index.cmp(&0) {
        Ordering::Greater => rel_index as f32 / (path.nodes.len() - path.root_index - 1) as f32,
        Ordering::Less => -rel_index as f32 / path.root_index as f32,
        Ordering::Equal => 0.0,
    };
    1.0 - t
}

fn get_camera_target_vertices(scene: &Scene) -> (Vec<Vertex>, Vec<ScreenDotInstance>) {
    let mut line_vertices = Vec::new();
    let mut dot_instances = Vec::new();

    if scene.show_camera_target {
        if let Camera::Rotate(camera) = &scene.camera {
            let color = [0.2, 0.2, 0.2, 0.8];

            let y_radius = 0.01;
            let x_radius = y_radius * scene.viewport.height as f32 / scene.viewport.width as f32;
            dot_instances.push(ScreenDotInstance {
                center: camera.target,
                radius: [x_radius, y_radius],
                color,
            });

            line_vertices.extend(&[
                Vertex::new(camera.target, color),
                Vertex::new(camera.target.0 + Vector3f::new(0.0, -10_000.0, 0.0), color),
            ]);
        }
    }

    (line_vertices, dot_instances)
}

fn get_unit_square_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    if let Camera::BirdsEye(camera) = &scene.camera {
        let span_x = camera.span_y;
        let span_z = span_x * scene.viewport.width as f32 / scene.viewport.height as f32;

        let min_x = camera.pos[0] - span_x / 2.0;
        let max_x = camera.pos[0] + span_x / 2.0;
        let min_z = camera.pos[2] - span_z / 2.0;
        let max_z = camera.pos[2] + span_z / 2.0;

        let density_threshold = 0.1;
        let density = ((max_x - min_x) / scene.viewport.height as f32)
            .max((max_z - min_z) / scene.viewport.width as f32);

        if density <= density_threshold {
            let color = [0.8, 0.8, 1.0, 0.5];
            let y = camera.pos[1];

            for x in min_x as i32..=max_x as i32 {
                vertices.extend(&[
                    Vertex::new(Point3f::new(x as f32, y, min_z), color),
                    Vertex::new(Point3f::new(x as f32, y, max_z), color),
                ]);
            }

            for z in min_z as i32..=max_z as i32 {
                vertices.extend(&[
                    Vertex::new(Point3f::new(min_x, y, z as f32), color),
                    Vertex::new(Point3f::new(max_x, y, z as f32), color),
                ]);
            }
        }
    }

    vertices
}
