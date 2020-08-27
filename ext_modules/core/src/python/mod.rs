//! SM64-specific Python API for Wafel.
//!
//! The exposed API is **not** safe because of the assumptions made about DLL loading.

use bytemuck::{cast_slice, Pod, Zeroable};
pub use pipeline::*;
use pyo3::prelude::*;
use std::{iter, slice};
pub use variable::*;
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod error;
mod pipeline;
mod value;
mod variable;

// TODO: __str__, __repr__, __eq__, __hash__ for PyObjectBehavior, PyAddress

#[pymodule]
fn core(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPipeline>()?;
    m.add_class::<PyVariable>()?;
    m.add_class::<PyObjectBehavior>()?;
    m.add_class::<PyAddress>()?;
    m.add_class::<PyRenderer>()?;
    Ok(())
}

#[pyclass(name = Renderer)]
pub struct PyRenderer {}

#[pymethods]
impl PyRenderer {
    #[staticmethod]
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(&mut self, render_func: PyObject) -> PyResult<()> {
        // TODO: Error handling (and/or make sure panics show up in log)
        futures::executor::block_on(async {
            let event_loop = EventLoop::new();

            let window = WindowBuilder::new()
                .with_title("Wafel") // TODO: Version number
                // .with_maximized(true)
                .with_inner_size(PhysicalSize::new(800, 600))
                .build(&event_loop)
                .expect("failed to open window");

            let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
            let surface = unsafe { instance.create_surface(&window) };
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&surface),
                })
                .await
                .expect("no compatible device");
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        features: wgpu::Features::empty(),
                        limits: wgpu::Limits::default(),
                        shader_validation: true,
                    },
                    None,
                )
                .await
                .unwrap();

            let swap_chain_descriptor = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8Unorm,
                width: window.inner_size().width,
                height: window.inner_size().height,
                present_mode: wgpu::PresentMode::Mailbox,
            };
            let mut swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

            let gil = Python::acquire_gil();
            let py = gil.python();

            let ig = PyModule::import(py, "imgui")?;
            let io = ig.call_method0("get_io")?;

            let index_size: usize = ig.getattr("INDEX_SIZE")?.extract()?;
            let vertex_size: usize = ig.getattr("VERTEX_SIZE")?.extract()?;
            let vertex_pos_offset: usize = ig.getattr("VERTEX_BUFFER_POS_OFFSET")?.extract()?;
            let vertex_uv_offset: usize = ig.getattr("VERTEX_BUFFER_UV_OFFSET")?.extract()?;
            let vertex_col_offset: usize = ig.getattr("VERTEX_BUFFER_COL_OFFSET")?.extract()?;

            let proj_bind_group_layout =
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
                    ],
                });

            let texture_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        // u_Sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Sampler { comparison: false },
                            count: None,
                        },
                        // u_Texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::SampledTexture {
                                dimension: wgpu::TextureViewDimension::D2,
                                component_type: wgpu::TextureComponentType::Float,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[&proj_bind_group_layout, &texture_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &device.create_shader_module(wgpu::include_spirv!(
                        "../../bin/shaders/imgui.vert.spv"
                    )),
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &device.create_shader_module(wgpu::include_spirv!(
                        "../../bin/shaders/imgui.frag.spv"
                    )),
                    entry_point: "main",
                }),
                rasterization_state: None,
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: swap_chain_descriptor.format,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    color_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                depth_stencil_state: None,
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: match index_size {
                        2 => wgpu::IndexFormat::Uint16,
                        4 => wgpu::IndexFormat::Uint32,
                        n => unimplemented!("{}", n),
                    },
                    vertex_buffers: &[wgpu::VertexBufferDescriptor {
                        stride: vertex_size as wgpu::BufferAddress,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &[
                            // a_Pos
                            wgpu::VertexAttributeDescriptor {
                                offset: vertex_pos_offset as wgpu::BufferAddress,
                                format: wgpu::VertexFormat::Float2,
                                shader_location: 0,
                            },
                            // a_TexCoord
                            wgpu::VertexAttributeDescriptor {
                                offset: vertex_uv_offset as wgpu::BufferAddress,
                                format: wgpu::VertexFormat::Float2,
                                shader_location: 1,
                            },
                            // a_Color
                            wgpu::VertexAttributeDescriptor {
                                offset: vertex_col_offset as wgpu::BufferAddress,
                                format: wgpu::VertexFormat::Uchar4Norm,
                                shader_location: 2,
                            },
                        ],
                    }],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

            // io.setattr("delta_time", 1.0 / 60.0)?;
            io.setattr("display_size", (800, 600))?;

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

            let font_texture = io
                .getattr("fonts")?
                .call_method0("get_tex_data_as_rgba32")?;
            let (width, height, data): (usize, usize, &[u8]) = font_texture.extract()?;

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: width as u32,
                    height: height as u32,
                    depth: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
            });
            queue.write_texture(
                wgpu::TextureCopyView {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                data,
                wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: (4 * width) as u32,
                    rows_per_image: height as u32,
                },
                wgpu::Extent3d {
                    width: width as u32,
                    height: height as u32,
                    depth: 1,
                },
            );

            let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

            io.getattr("fonts")?.setattr("texture_id", 0)?;
            io.getattr("fonts")?.call_method0("clear_tex_data")?;

            event_loop.run(move |event, _, control_flow| {
                let gil = Python::acquire_gil();
                let py = gil.python();

                match event {
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(_) => {}
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => {}
                    },
                    Event::MainEventsCleared => window.request_redraw(),
                    Event::RedrawRequested(_) => {
                        let result: PyResult<()> = try {
                            let draw_data = render_func.as_ref(py).call0()?;

                            let proj_matrix: [[f32; 4]; 4] = [
                                [2.0 / 800.0, 0.0, 0.0, 0.0],
                                [0.0, -2.0 / 600.0, 0.0, 0.0],
                                [0.0, 0.0, -1.0, 0.0],
                                [-1.0, 1.0, 0.0, 1.0],
                            ];
                            let proj_matrix_buffer =
                                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: None,
                                    contents: cast_slice(&proj_matrix),
                                    usage: wgpu::BufferUsage::UNIFORM,
                                });
                            let proj_bind_group =
                                device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: None,
                                    layout: &proj_bind_group_layout,
                                    entries: &[
                                        // u_Proj
                                        wgpu::BindGroupEntry {
                                            binding: 0,
                                            resource: wgpu::BindingResource::Buffer {
                                                buffer: &proj_matrix_buffer,
                                                offset: 0,
                                                size: None,
                                            },
                                        },
                                    ],
                                });

                            // fn vtx(pos: [f32; 2], tex_coord: [f32; 2], color: [u8; 4]) -> Vertex {
                            //     Vertex {
                            //         pos,
                            //         tex_coord,
                            //         color,
                            //     }
                            // }

                            // let indices: Vec<u16> = vec![
                            //     0, 1, 2, 0, 2, 3, 7, 4, 6, 6, 9, 7, 8, 5, 4, 4, 7, 8, 10, 7, 9, 9,
                            //     12, 10, 11, 8, 7, 7, 10, 11, 13, 10, 12, 12, 15, 13, 14, 11, 10,
                            //     10, 13, 14, 4, 13, 15, 15, 6, 4, 5, 14, 13, 13, 4, 5, 16, 17, 18,
                            //     16, 18, 19, 20, 21, 22, 20, 22, 23, 24, 25, 26, 24, 26, 27, 28, 29,
                            //     30, 28, 30, 31, 32, 33, 34, 32, 34, 35, 36, 37, 38, 36, 38, 39, 40,
                            //     41, 42, 40, 42, 43, 44, 45, 46, 44, 46, 47, 48, 49, 50, 48, 50, 51,
                            //     52, 53, 54, 52, 54, 55,
                            // ];
                            // let index_buffer =
                            //     device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            //         label: None,
                            //         contents: cast_slice(&indices),
                            //         usage: wgpu::BufferUsage::INDEX,
                            //     });

                            // let vertices = vec![
                            //     Vertex {
                            //         pos: [0.0, 0.0],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [36, 36, 36, 255],
                            //     },
                            //     Vertex {
                            //         pos: [800.0, 0.0],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [36, 36, 36, 255],
                            //     },
                            //     Vertex {
                            //         pos: [800.0, 19.0],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [36, 36, 36, 255],
                            //     },
                            //     Vertex {
                            //         pos: [0.0, 19.0],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [36, 36, 36, 255],
                            //     },
                            //     Vertex {
                            //         pos: [0.5, 0.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 128],
                            //     },
                            //     Vertex {
                            //         pos: [-0.5, -0.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [1.5, 1.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [799.5, 0.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 128],
                            //     },
                            //     Vertex {
                            //         pos: [800.5, -0.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [798.5, 1.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [799.5, 599.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 128],
                            //     },
                            //     Vertex {
                            //         pos: [800.5, 600.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [798.5, 598.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [0.5, 599.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 128],
                            //     },
                            //     Vertex {
                            //         pos: [-0.5, 600.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [1.5, 598.5],
                            //         tex_coord: [0.0009765625, 0.0078125],
                            //         color: [110, 110, 128, 0],
                            //     },
                            //     Vertex {
                            //         pos: [9.0, 29.0],
                            //         tex_coord: [0.76953125, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [14.0, 29.0],
                            //         tex_coord: [0.7792969, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [14.0, 38.0],
                            //         tex_coord: [0.7792969, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [9.0, 38.0],
                            //         tex_coord: [0.76953125, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [16.0, 32.0],
                            //         tex_coord: [0.234375, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [21.0, 32.0],
                            //         tex_coord: [0.24414063, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [21.0, 38.0],
                            //         tex_coord: [0.24414063, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [16.0, 38.0],
                            //         tex_coord: [0.234375, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [24.0, 29.0],
                            //         tex_coord: [0.91015625, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [26.0, 29.0],
                            //         tex_coord: [0.9140625, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [26.0, 38.0],
                            //         tex_coord: [0.9140625, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [24.0, 38.0],
                            //         tex_coord: [0.91015625, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [31.0, 29.0],
                            //         tex_coord: [0.91015625, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [33.0, 29.0],
                            //         tex_coord: [0.9140625, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [33.0, 38.0],
                            //         tex_coord: [0.9140625, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [31.0, 38.0],
                            //         tex_coord: [0.91015625, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [37.0, 32.0],
                            //         tex_coord: [0.19921875, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [42.0, 32.0],
                            //         tex_coord: [0.20898438, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [42.0, 38.0],
                            //         tex_coord: [0.20898438, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [37.0, 38.0],
                            //         tex_coord: [0.19921875, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [50.0, 32.0],
                            //         tex_coord: [0.15234375, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [57.0, 32.0],
                            //         tex_coord: [0.16601563, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [57.0, 38.0],
                            //         tex_coord: [0.16601563, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [50.0, 38.0],
                            //         tex_coord: [0.15234375, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [58.0, 32.0],
                            //         tex_coord: [0.19921875, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [63.0, 32.0],
                            //         tex_coord: [0.20898438, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [63.0, 38.0],
                            //         tex_coord: [0.20898438, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [58.0, 38.0],
                            //         tex_coord: [0.19921875, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [65.0, 32.0],
                            //         tex_coord: [0.31640625, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [70.0, 32.0],
                            //         tex_coord: [0.32617188, 0.4375],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [70.0, 38.0],
                            //         tex_coord: [0.32617188, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [65.0, 38.0],
                            //         tex_coord: [0.31640625, 0.53125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [73.0, 29.0],
                            //         tex_coord: [0.91015625, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [75.0, 29.0],
                            //         tex_coord: [0.9140625, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [75.0, 38.0],
                            //         tex_coord: [0.9140625, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [73.0, 38.0],
                            //         tex_coord: [0.91015625, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [79.0, 29.0],
                            //         tex_coord: [0.72265625, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [84.0, 29.0],
                            //         tex_coord: [0.7324219, 0.1875],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [84.0, 38.0],
                            //         tex_coord: [0.7324219, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            //     Vertex {
                            //         pos: [79.0, 38.0],
                            //         tex_coord: [0.72265625, 0.328125],
                            //         color: [255, 255, 255, 255],
                            //     },
                            // ];
                            // let vertex_buffer =
                            //     device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            //         label: None,
                            //         contents: cast_slice(&vertices),
                            //         usage: wgpu::BufferUsage::VERTEX,
                            //     });

                            // let output_view = &swap_chain.get_current_frame().unwrap().output.view;

                            // let mut encoder = device
                            //     .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                            // {
                            //     let mut render_pass =
                            //         encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            //             color_attachments: &[
                            //                 wgpu::RenderPassColorAttachmentDescriptor {
                            //                     attachment: output_view,
                            //                     resolve_target: None,
                            //                     ops: wgpu::Operations {
                            //                         load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            //                         store: true,
                            //                     },
                            //                 },
                            //             ],
                            //             depth_stencil_attachment: None,
                            //         });
                            //     render_pass.set_pipeline(&pipeline);
                            //     render_pass.set_bind_group(0, &proj_bind_group, &[]);
                            //     render_pass.set_bind_group(1, &texture_bind_group, &[]);
                            //     render_pass.set_index_buffer(index_buffer.slice(..));
                            //     render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                            //     render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
                            // }

                            let mut command_lists: Vec<(wgpu::Buffer, wgpu::Buffer, Vec<&PyAny>)> =
                                Vec::new();
                            for commands in draw_data.getattr("commands_lists")?.iter()? {
                                let commands = commands?;

                                let index_buffer_size: usize =
                                    commands.getattr("idx_buffer_size")?.extract()?;
                                let index_buffer_pointer: usize =
                                    commands.getattr("idx_buffer_data")?.extract()?;
                                let index_slice = unsafe {
                                    slice::from_raw_parts(
                                        index_buffer_pointer as *const u8,
                                        index_buffer_size * index_size,
                                    )
                                };
                                let index_buffer =
                                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                        label: None,
                                        contents: index_slice,
                                        usage: wgpu::BufferUsage::INDEX,
                                    });

                                let vertex_buffer_size: usize =
                                    commands.getattr("vtx_buffer_size")?.extract()?;
                                let vertex_buffer_pointer: usize =
                                    commands.getattr("vtx_buffer_data")?.extract()?;
                                let vertex_slice = unsafe {
                                    slice::from_raw_parts(
                                        vertex_buffer_pointer as *const u8,
                                        vertex_buffer_size * vertex_size,
                                    )
                                };
                                let vertex_buffer =
                                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                        label: None,
                                        contents: vertex_slice,
                                        usage: wgpu::BufferUsage::VERTEX,
                                    });

                                let commands = commands
                                    .getattr("commands")?
                                    .iter()?
                                    .collect::<PyResult<Vec<_>>>()?;

                                command_lists.push((index_buffer, vertex_buffer, commands));
                            }

                            let output_view = &swap_chain.get_current_frame().unwrap().output.view;

                            let mut encoder = device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                            {
                                let mut render_pass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        color_attachments: &[
                                            wgpu::RenderPassColorAttachmentDescriptor {
                                                attachment: output_view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                                    store: true,
                                                },
                                            },
                                        ],
                                        depth_stencil_attachment: None,
                                    });
                                render_pass.set_pipeline(&pipeline);
                                render_pass.set_bind_group(0, &proj_bind_group, &[]);
                                render_pass.set_bind_group(1, &texture_bind_group, &[]);

                                for (index_buffer, vertex_buffer, commands) in &command_lists {
                                    render_pass.set_index_buffer(index_buffer.slice(..));
                                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

                                    let mut initial_index = 0;

                                    for command in commands {
                                        let texture_id: usize =
                                            command.getattr("texture_id")?.extract()?;
                                        assert_eq!(texture_id, 0);

                                        let elem_count: usize =
                                            command.getattr("elem_count")?.extract()?;

                                        render_pass.draw_indexed(
                                            initial_index as u32
                                                ..(initial_index + elem_count) as u32,
                                            0,
                                            0..1,
                                        );

                                        initial_index += elem_count;

                                        // println!("  cmd:");
                                        // println!(
                                        //     "    clip_rect = {}",
                                        //     command.getattr("clip_rect")?
                                        // );
                                        // println!(
                                        //     "    texture_id = {}",
                                        //     command.getattr("texture_id")?
                                        // );
                                        // println!(
                                        //     "    elem_count = {}",
                                        //     command.getattr("elem_count")?
                                        // );
                                    }
                                }
                            }

                            let command_buffer = encoder.finish();
                            queue.submit(iter::once(command_buffer));
                        };
                        result.unwrap();
                        // if let Err(error) = result {
                        //     error.restore(py);
                        // }
                        // *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            });
        })

        // Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    tex_coord: [f32; 2],
    color: [u8; 4],
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}
