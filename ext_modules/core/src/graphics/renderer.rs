use crate::geo::{Matrix4f, Point3f, Vector3f, Vector4f};
use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use nalgebra::distance;
use std::{f32::consts::PI, iter, mem::size_of};
use wgpu::util::DeviceExt;

use super::scene::{BirdsEyeCamera, Camera, RotateCamera, Scene, SurfaceType};

const NUM_OUTPUT_SAMPLES: u32 = 4;
const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 4],
}

impl Vertex {
    fn new(pos: Point3f, color: [f32; 4]) -> Self {
        Self {
            pos: [pos.x, pos.y, pos.z],
            color,
        }
    }
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}

struct SceneBundle {
    transform_bind_group: wgpu::BindGroup,
    surface_vertex_buffer: (usize, wgpu::Buffer),
    hidden_surface_vertex_buffer: (usize, wgpu::Buffer),
    wall_hitbox_vertex_buffer: (usize, wgpu::Buffer),
    wall_hitbox_outline_vertex_buffer: (usize, wgpu::Buffer),
    object_vertex_buffer: (usize, wgpu::Buffer),
}

pub struct Renderer {
    multisample_texture: Option<((u32, u32), wgpu::Texture)>,
    depth_texture: Option<((u32, u32), wgpu::Texture)>,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    surface_pipeline: wgpu::RenderPipeline,
    hidden_surface_pipeline: wgpu::RenderPipeline,
    wall_hitbox_pipeline: wgpu::RenderPipeline,
    wall_hitbox_depth_pass_pipeline: wgpu::RenderPipeline,
    wall_hitbox_outline_pipeline: wgpu::RenderPipeline,
    object_pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // u_Proj
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // u_View
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let surface_pipeline =
            create_surface_pipeline(device, &transform_bind_group_layout, output_format, true);
        let hidden_surface_pipeline =
            create_surface_pipeline(device, &transform_bind_group_layout, output_format, false);
        let wall_hitbox_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            true,
            wgpu::PrimitiveTopology::TriangleList,
        );
        let wall_hitbox_depth_pass_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            false,
            wgpu::PrimitiveTopology::TriangleList,
        );
        let wall_hitbox_outline_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            true,
            wgpu::PrimitiveTopology::LineList,
        );

        let object_pipeline = create_color_pipeline(
            device,
            &transform_bind_group_layout,
            output_format,
            true,
            wgpu::PrimitiveTopology::LineList,
        );

        Self {
            multisample_texture: None,
            depth_texture: None,
            transform_bind_group_layout,
            surface_pipeline,
            hidden_surface_pipeline,
            wall_hitbox_pipeline,
            wall_hitbox_depth_pass_pipeline,
            wall_hitbox_outline_pipeline,
            object_pipeline,
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        output_format: wgpu::TextureFormat,
        scenes: &[Scene],
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
                        usage: wgpu::BufferUsage::UNIFORM,
                    });
                let view_matrix_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(view_matrix.as_slice()),
                        usage: wgpu::BufferUsage::UNIFORM,
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
                let surface_vertex_buffer = (
                    surface_vertices.len(),
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(&surface_vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                );
                let hidden_surface_vertex_buffer = (
                    hidden_surface_vertices.len(),
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(&hidden_surface_vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                );

                let (wall_hitbox_vertices, wall_hitbox_outline_vertices) =
                    get_wall_hitbox_vertices(scene);
                let wall_hitbox_vertex_buffer = (
                    wall_hitbox_vertices.len(),
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(&wall_hitbox_vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                );
                let wall_hitbox_outline_vertex_buffer = (
                    wall_hitbox_outline_vertices.len(),
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(&wall_hitbox_outline_vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                );

                let object_vertices = get_object_vertices(scene);
                let object_vertex_buffer = (
                    object_vertices.len(),
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(&object_vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                );

                SceneBundle {
                    transform_bind_group,
                    surface_vertex_buffer,
                    hidden_surface_vertex_buffer,
                    wall_hitbox_vertex_buffer,
                    wall_hitbox_outline_vertex_buffer,
                    object_vertex_buffer,
                }
            })
            .collect();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &multisample_texture_view,
                    resolve_target: Some(&output_view),
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            for (scene, bundle) in scenes.iter().zip(&scene_bundles) {
                let viewport = &scene.viewport;
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

                render_pass.set_pipeline(&self.object_pipeline);
                render_pass.set_vertex_buffer(0, bundle.object_vertex_buffer.1.slice(..));
                render_pass.draw(0..bundle.object_vertex_buffer.0 as u32, 0..1);

                if scene.wall_hitbox_radius > 0.0 {
                    // Render lines first since tris write to z buffer
                    render_pass.set_pipeline(&self.wall_hitbox_outline_pipeline);
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
            }
        }

        let command_buffer = encoder.finish();
        queue.submit(iter::once(command_buffer));
    }
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
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: NUM_OUTPUT_SAMPLES,
        dimension: wgpu::TextureDimension::D2,
        format: output_format,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    })
}

fn create_depth_texture(device: &wgpu::Device, output_size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: output_size.0,
            height: output_size.1,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: NUM_OUTPUT_SAMPLES,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_TEXTURE_FORMAT,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    })
}

fn create_surface_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    depth_write_enabled: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &device
                .create_shader_module(wgpu::include_spirv!("../../bin/shaders/surface.vert.spv")),
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &device
                .create_shader_module(wgpu::include_spirv!("../../bin/shaders/surface.frag.spv")),
            entry_point: "main",
        }),
        rasterization_state: None,
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: output_format,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            color_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            format: DEPTH_TEXTURE_FORMAT,
            depth_write_enabled,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilStateDescriptor::default(),
        }),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    // a_Pos
                    wgpu::VertexAttributeDescriptor {
                        offset: offset_of!(Vertex, pos) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float3,
                        shader_location: 0,
                    },
                    // a_Color
                    wgpu::VertexAttributeDescriptor {
                        offset: offset_of!(Vertex, color) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float4,
                        shader_location: 1,
                    },
                ],
            }],
        },
        sample_count: NUM_OUTPUT_SAMPLES,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}

fn create_color_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    color_write_enabled: bool,
    primitive_topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &device
                .create_shader_module(wgpu::include_spirv!("../../bin/shaders/color.vert.spv")),
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &device
                .create_shader_module(wgpu::include_spirv!("../../bin/shaders/color.frag.spv")),
            entry_point: "main",
        }),
        rasterization_state: None,
        primitive_topology,
        color_states: &[wgpu::ColorStateDescriptor {
            format: output_format,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            color_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: if color_write_enabled {
                wgpu::ColorWrite::ALL
            } else {
                wgpu::ColorWrite::empty()
            },
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            format: DEPTH_TEXTURE_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilStateDescriptor::default(),
        }),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    // a_Pos
                    wgpu::VertexAttributeDescriptor {
                        offset: offset_of!(Vertex, pos) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float3,
                        shader_location: 0,
                    },
                    // a_Color
                    wgpu::VertexAttributeDescriptor {
                        offset: offset_of!(Vertex, color) as wgpu::BufferAddress,
                        format: wgpu::VertexFormat::Float4,
                        shader_location: 1,
                    },
                ],
            }],
        },
        sample_count: NUM_OUTPUT_SAMPLES,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
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

    let view_matrix = Matrix4f::new_rotation(PI * Vector3f::y())
        * Matrix4f::new_rotation(camera.pitch * Vector3f::x())
        * Matrix4f::new_rotation(-camera.yaw * Vector3f::y())
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

    let view_matrix = Matrix4f::new_translation(&-Vector3f::from_row_slice(&camera.pos));

    (proj_matrix, view_matrix)
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

        let proj_dist = scene.wall_hitbox_radius / surface.normal().dot(&proj_dir);

        let wall_vertices = surface.vertices();
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
            Camera::Rotate(camera) => distance(&int_vertices[0], &Point3f::from_slice(&camera.pos)),
            Camera::BirdsEye(camera) => 1000.0,
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

        let pos = object.pos();
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
