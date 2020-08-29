use super::{Scene, SurfaceType};
use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use nalgebra::SliceStorage;
use pyo3::prelude::*;
use std::{iter, mem::size_of};
use wgpu::util::DeviceExt;

type Matrix4f = nalgebra::Matrix4<f32>;
type Vector3f = nalgebra::Vector3<f32>;
type Vector4f = nalgebra::Vector4<f32>;

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
            depth_stencil_state: None,
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
            transform_bind_group_layout,
            surface_pipeline,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        output_format: wgpu::TextureFormat,
        scenes: &[Scene],
    ) {
        let scene_bundles: Vec<SceneBundle> = scenes
            .iter()
            .map(|scene| {
                let camera = &scene.camera;

                // world x = screen up, world z = screen right
                let rotation = Matrix4f::from_columns(&[
                    Vector4f::y(),
                    -Vector4f::z(),
                    Vector4f::x(),
                    Vector4f::w(),
                ]);
                let scaling = Matrix4f::new_nonuniform_scaling(&Vector3f::new(
                    2.0 / (camera.span_y * output_size.0 as f32 / output_size.1 as f32),
                    2.0 / camera.span_y,
                    1.0 / 40_000.0,
                ));
                let proj_matrix = scaling * rotation;

                let view_matrix =
                    Matrix4f::new_translation(&-Vector3f::from_row_slice(&camera.pos));

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
                depth_stencil_attachment: None,
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
