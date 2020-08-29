use super::{BirdsEyeCamera, Camera, RotateCamera, Scene, SurfaceType};
use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use nalgebra::{distance, SliceStorage};
use pyo3::prelude::*;
use std::{f32::consts::PI, iter, mem::size_of};
use wgpu::util::DeviceExt;

type Matrix4f = nalgebra::Matrix4<f32>;
type Point3f = nalgebra::Point3<f32>;
type Vector3f = nalgebra::Vector3<f32>;
type Vector4f = nalgebra::Vector4<f32>;

const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

#[derive(Debug, Clone, Copy, Default)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 4],
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}

struct SceneBundle {
    transform_bind_group: wgpu::BindGroup,
    surface_vertex_buffer: (usize, wgpu::Buffer),
}

pub struct Renderer {
    depth_texture: Option<((u32, u32), wgpu::Texture)>,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    surface_pipeline: wgpu::RenderPipeline,
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

        let surface_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&transform_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &device.create_shader_module(wgpu::include_spirv!(
                    "../../bin/shaders/surface.vert.spv"
                )),
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &device.create_shader_module(wgpu::include_spirv!(
                    "../../bin/shaders/surface.frag.spv"
                )),
                entry_point: "main",
            }),
            rasterization_state: None,
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor::from(output_format)],
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
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            depth_texture: None,
            transform_bind_group_layout,
            surface_pipeline,
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

                let mut surface_vertices: Vec<Vertex> = Vec::new();
                for surface in &scene.surfaces {
                    let color = match surface.ty {
                        SurfaceType::Floor => [0.5, 0.5, 1.0, 1.0],
                        SurfaceType::Ceiling => [1.0, 0.5, 0.5, 1.0],
                        SurfaceType::WallXProj => [0.3, 0.8, 0.3, 1.0],
                        SurfaceType::WallZProj => [0.15, 0.4, 0.15, 1.0],
                    };

                    for pos in &surface.vertices {
                        surface_vertices.push(Vertex { pos: *pos, color });
                    }
                }
                let surface_vertex_buffer = (
                    surface_vertices.len(),
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: cast_slice(&surface_vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                );

                SceneBundle {
                    transform_bind_group,
                    surface_vertex_buffer,
                }
            })
            .collect();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: output_view,
                    resolve_target: None,
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

                render_pass.set_pipeline(&self.surface_pipeline);
                render_pass.set_bind_group(0, &bundle.transform_bind_group, &[]);

                render_pass.set_vertex_buffer(0, bundle.surface_vertex_buffer.1.slice(..));

                render_pass.draw(0..bundle.surface_vertex_buffer.0 as u32, 0..1);
            }
        }

        let command_buffer = encoder.finish();
        queue.submit(iter::once(command_buffer));
    }
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
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_TEXTURE_FORMAT,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
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
