//! SM64-specific Python API for Wafel.
//!
//! The exposed API is **not** safe because of the assumptions made about DLL loading.

use crate::graphics::{
    ImguiCommand, ImguiCommandList, ImguiConfig, ImguiDrawData, IMGUI_FONT_TEXTURE_ID,
};
use bytemuck::{cast_slice, Pod, Zeroable};
pub use imgui_input::*;
pub use pipeline::*;
use pyo3::{prelude::*, wrap_pyfunction};
use std::{
    collections::{HashMap, HashSet},
    iter, slice,
    time::Instant,
};
pub use variable::*;
use wgpu::util::DeviceExt;
// pub use window::*;
pub use window::*;
use winit::{
    dpi::PhysicalSize,
    event::{
        ElementState::{Pressed, Released},
        Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod error;
mod imgui_input;
mod pipeline;
mod value;
mod variable;
mod window;

// TODO: __str__, __repr__, __eq__, __hash__ for PyObjectBehavior, PyAddress

#[pymodule]
fn core(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPipeline>()?;
    m.add_class::<PyVariable>()?;
    m.add_class::<PyObjectBehavior>()?;
    m.add_class::<PyAddress>()?;
    m.add_class::<PyRenderer>()?;
    m.add_wrapped(wrap_pyfunction!(open_window_and_run))?;
    Ok(())
}

#[pyfunction]
pub fn open_window_and_run(title: &str, update_fn: PyObject) -> PyResult<()> {
    open_window_and_run_impl(title, update_fn)
}

#[pyclass(name = Renderer)]
pub struct PyRenderer {}

#[pymethods]
impl PyRenderer {
    #[staticmethod]
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_old(&mut self, render_func: PyObject) -> PyResult<()> {
        // TODO: Error handling (and/or make sure panics show up in log)
        futures::executor::block_on(async {
            let event_loop = EventLoop::new();

            let window = WindowBuilder::new()
                .with_title("Wafel") // TODO: Version number
                .with_inner_size(PhysicalSize::new(800, 600))
                .with_maximized(true)
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

            let mut swap_chain_desc = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8Unorm,
                width: window.inner_size().width,
                height: window.inner_size().height,
                present_mode: wgpu::PresentMode::Mailbox,
            };
            let mut swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

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
                    format: swap_chain_desc.format,
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

            let key_map = vec![
                ("KEY_TAB", "KEY_TAB"),
                ("KEY_LEFT_ARROW", "KEY_LEFT"),
                ("KEY_RIGHT_ARROW", "KEY_RIGHT"),
                ("KEY_UP_ARROW", "KEY_UP"),
                ("KEY_DOWN_ARROW", "KEY_DOWN"),
                ("KEY_PAGE_UP", "KEY_PAGE_UP"),
                ("KEY_PAGE_DOWN", "KEY_PAGE_DOWN"),
                ("KEY_HOME", "KEY_HOME"),
                ("KEY_END", "KEY_END"),
                ("KEY_DELETE", "KEY_DELETE"),
                ("KEY_BACKSPACE", "KEY_BACKSPACE"),
                ("KEY_ENTER", "KEY_ENTER"),
                ("KEY_ESCAPE", "KEY_ESCAPE"),
                ("KEY_A", "KEY_A"),
                ("KEY_C", "KEY_C"),
                ("KEY_V", "KEY_V"),
                ("KEY_X", "KEY_X"),
                ("KEY_Y", "KEY_Y"),
                ("KEY_Z", "KEY_Z"),
            ];
            let glfw = PyModule::import(py, "glfw")?;
            for (imgui_name, glfw_name) in key_map {
                let imgui_key = ig.getattr(imgui_name)?;
                let glfw_key = glfw.getattr(glfw_name)?;
                io.getattr("key_map")?.set_item(imgui_key, glfw_key)?;
            }

            let mut winit_to_glfw_key: HashMap<VirtualKeyCode, u32> = HashMap::new();
            for (glfw_name, winit_key) in GLFW_WINIT_KEY_MAP {
                let glfw_key: u32 = glfw.getattr(glfw_name)?.extract()?;
                winit_to_glfw_key.insert(*winit_key, glfw_key);
            }

            let modifier_key = |imgui_name, glfw_key_name_l, glfw_key_name_r| -> PyResult<_> {
                let glfw_key_l: u32 = glfw.getattr(glfw_key_name_l)?.extract()?;
                let glfw_key_r: u32 = glfw.getattr(glfw_key_name_r)?.extract()?;
                Ok((imgui_name, glfw_key_l, glfw_key_r))
            };
            let modifier_keys = vec![
                modifier_key("key_ctrl", "KEY_LEFT_CONTROL", "KEY_RIGHT_CONTROL")?,
                modifier_key("key_alt", "KEY_LEFT_ALT", "KEY_RIGHT_ALT")?,
                modifier_key("key_shift", "KEY_LEFT_SHIFT", "KEY_RIGHT_SHIFT")?,
                modifier_key("key_super", "KEY_LEFT_SUPER", "KEY_RIGHT_SUPER")?,
            ];

            let mut last_frame_time = Instant::now();

            event_loop.run(move |event, _, control_flow| {
                let gil = Python::acquire_gil();
                let py = gil.python();

                let result: PyResult<()> = try {
                    let ig = PyModule::import(py, "imgui")?;
                    let io = ig.call_method0("get_io")?;

                    match event {
                        Event::WindowEvent { event, .. } => match event {
                            WindowEvent::Resized(size) => {
                                swap_chain_desc.width = size.width;
                                swap_chain_desc.height = size.height;
                                swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
                            }
                            WindowEvent::CloseRequested => {
                                *control_flow = ControlFlow::Exit;
                            }
                            WindowEvent::CursorMoved { position, .. } => {
                                io.setattr("mouse_pos", (position.x, position.y))?;
                            }
                            WindowEvent::MouseInput { state, button, .. } => {
                                let is_down = match state {
                                    Pressed => true,
                                    Released => false,
                                };
                                let button_index = match button {
                                    MouseButton::Left => Some(0),
                                    MouseButton::Right => Some(1),
                                    MouseButton::Middle => Some(2),
                                    MouseButton::Other(_) => None,
                                };
                                if let Some(button_index) = button_index {
                                    io.getattr("mouse_down")?.set_item(button_index, is_down)?;
                                }
                            }
                            WindowEvent::MouseWheel { delta, .. } => {
                                if let MouseScrollDelta::LineDelta(_, y) = delta {
                                    let mouse_wheel: f32 = io.getattr("mouse_wheel")?.extract()?;
                                    io.setattr("mouse_wheel", mouse_wheel + y)?;
                                }
                            }
                            WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        state,
                                        virtual_keycode,
                                        ..
                                    },
                                ..
                            } => {
                                let is_down = match state {
                                    Pressed => true,
                                    Released => false,
                                };
                                if let Some(winit_key) = virtual_keycode {
                                    if let Some(&glfw_key) = winit_to_glfw_key.get(&winit_key) {
                                        io.getattr("keys_down")?.set_item(glfw_key, is_down)?;
                                    }
                                }

                                for (imgui_prop, glfw_key_l, glfw_key_r) in &modifier_keys {
                                    let down_l: u32 =
                                        io.getattr("keys_down")?.get_item(glfw_key_l)?.extract()?;
                                    let down_r: u32 =
                                        io.getattr("keys_down")?.get_item(glfw_key_r)?.extract()?;
                                    io.setattr(imgui_prop, down_l != 0 || down_r != 0)?;
                                }
                            }
                            WindowEvent::ReceivedCharacter(c) => {
                                let c = c as u32;
                                if c < 0x10000 {
                                    io.call_method1("add_input_character", (c,))?;
                                }
                            }
                            _ => {}
                        },
                        Event::MainEventsCleared => window.request_redraw(),
                        Event::RedrawRequested(_) => {
                            if swap_chain_desc.width > 0 && swap_chain_desc.height > 0 {
                                let delta_time = last_frame_time.elapsed().as_secs_f64();
                                last_frame_time = Instant::now();
                                io.setattr("delta_time", delta_time)?;

                                let display_size = (swap_chain_desc.width, swap_chain_desc.height);
                                io.setattr("display_size", display_size)?;

                                let draw_data = render_func.as_ref(py).call1((display_size,))?;

                                let proj_matrix: [[f32; 4]; 4] = [
                                    [2.0 / swap_chain_desc.width as f32, 0.0, 0.0, 0.0],
                                    [0.0, -2.0 / swap_chain_desc.height as f32, 0.0, 0.0],
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

                                let mut command_lists: Vec<(
                                    wgpu::Buffer,
                                    wgpu::Buffer,
                                    Vec<&PyAny>,
                                )> = Vec::new();
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
                                    }
                                    .to_owned();
                                    let index_buffer = device.create_buffer_init(
                                        &wgpu::util::BufferInitDescriptor {
                                            label: None,
                                            contents: &index_slice,
                                            usage: wgpu::BufferUsage::INDEX,
                                        },
                                    );

                                    let vertex_buffer_size: usize =
                                        commands.getattr("vtx_buffer_size")?.extract()?;
                                    let vertex_buffer_pointer: usize =
                                        commands.getattr("vtx_buffer_data")?.extract()?;
                                    let vertex_slice = unsafe {
                                        slice::from_raw_parts(
                                            vertex_buffer_pointer as *const u8,
                                            vertex_buffer_size * vertex_size,
                                        )
                                    }
                                    .to_owned();
                                    let vertex_buffer = device.create_buffer_init(
                                        &wgpu::util::BufferInitDescriptor {
                                            label: None,
                                            contents: &vertex_slice,
                                            usage: wgpu::BufferUsage::VERTEX,
                                        },
                                    );

                                    let commands = commands
                                        .getattr("commands")?
                                        .iter()?
                                        .collect::<PyResult<Vec<_>>>()?;

                                    command_lists.push((index_buffer, vertex_buffer, commands));
                                }

                                let output_view =
                                    &swap_chain.get_current_frame().unwrap().output.view;

                                let mut encoder = device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor::default(),
                                );

                                {
                                    let mut render_pass =
                                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                            color_attachments: &[
                                                wgpu::RenderPassColorAttachmentDescriptor {
                                                    attachment: output_view,
                                                    resolve_target: None,
                                                    ops: wgpu::Operations {
                                                        load: wgpu::LoadOp::Clear(
                                                            wgpu::Color::BLACK,
                                                        ),
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

                                            let clip: (f32, f32, f32, f32) =
                                                command.getattr("clip_rect")?.extract()?;
                                            render_pass.set_scissor_rect(
                                                clip.0 as u32,
                                                clip.1 as u32,
                                                (clip.2 - clip.0) as u32,
                                                (clip.3 - clip.1) as u32,
                                            );

                                            let elem_count: usize =
                                                command.getattr("elem_count")?.extract()?;

                                            render_pass.draw_indexed(
                                                initial_index as u32
                                                    ..(initial_index + elem_count) as u32,
                                                0,
                                                0..1,
                                            );

                                            initial_index += elem_count;
                                        }
                                    }
                                }

                                let command_buffer = encoder.finish();
                                queue.submit(iter::once(command_buffer));
                            }
                        }
                        _ => {}
                    }
                };
                if let Err(error) = result {
                    error.print(py);
                    *control_flow = ControlFlow::Exit;
                }
            });
        })

        // Ok(())
    }
}

fn load_imgui_config() -> PyResult<ImguiConfig> {
    let gil = Python::acquire_gil();
    let py = gil.python();

    let ig = PyModule::import(py, "imgui")?;
    let io = ig.call_method0("get_io")?;

    let font_texture = io
        .getattr("fonts")?
        .call_method0("get_tex_data_as_rgba32")?;
    let (width, height, data): (u32, u32, &[u8]) = font_texture.extract()?;

    let imgui_config = ImguiConfig {
        index_size: ig.getattr("INDEX_SIZE")?.extract()?,

        vertex_size: ig.getattr("VERTEX_SIZE")?.extract()?,
        vertex_pos_offset: ig.getattr("VERTEX_BUFFER_POS_OFFSET")?.extract()?,
        vertex_tex_coord_offset: ig.getattr("VERTEX_BUFFER_UV_OFFSET")?.extract()?,
        vertex_color_offset: ig.getattr("VERTEX_BUFFER_COL_OFFSET")?.extract()?,

        font_texture_width: width,
        font_texture_height: height,
        font_texture_data: data.to_owned(),
    };

    io.getattr("fonts")?
        .setattr("texture_id", IMGUI_FONT_TEXTURE_ID)?;
    io.getattr("fonts")?.call_method0("clear_tex_data")?;

    Ok(imgui_config)
}

fn extract_imgui_draw_data(config: &ImguiConfig, draw_data: &PyAny) -> PyResult<ImguiDrawData> {
    let mut command_lists = Vec::new();
    for command_list in draw_data.getattr("commands_lists")?.iter()? {
        let command_list = command_list?;
        command_lists.push(extract_imgui_command_list(config, command_list)?);
    }
    Ok(ImguiDrawData { command_lists })
}

fn extract_imgui_command_list(
    config: &ImguiConfig,
    command_list: &PyAny,
) -> PyResult<ImguiCommandList> {
    let index_buffer_size: usize = command_list.getattr("idx_buffer_size")?.extract()?;
    let index_buffer_pointer: usize = command_list.getattr("idx_buffer_data")?.extract()?;
    let index_buffer = unsafe {
        slice::from_raw_parts(
            index_buffer_pointer as *const u8,
            index_buffer_size * config.index_size,
        )
    }
    .to_owned();

    let vertex_buffer_size: usize = command_list.getattr("vtx_buffer_size")?.extract()?;
    let vertex_buffer_pointer: usize = command_list.getattr("vtx_buffer_data")?.extract()?;
    let vertex_buffer = unsafe {
        slice::from_raw_parts(
            vertex_buffer_pointer as *const u8,
            vertex_buffer_size * config.vertex_size,
        )
    }
    .to_owned();

    let mut commands = Vec::new();
    for command in command_list.getattr("commands")?.iter()? {
        let command = command?;
        commands.push(extract_imgui_command(command)?);
    }

    Ok(ImguiCommandList {
        index_buffer,
        vertex_buffer,
        commands,
    })
}

fn extract_imgui_command(command: &PyAny) -> PyResult<ImguiCommand> {
    Ok(ImguiCommand {
        texture_id: command.getattr("texture_id")?.extract()?,
        clip_rect: command.getattr("clip_rect")?.extract()?,
        elem_count: command.getattr("elem_count")?.extract()?,
    })
}

const GLFW_WINIT_KEY_MAP: &[(&str, VirtualKeyCode)] = &[
    ("KEY_SPACE", VirtualKeyCode::Space),
    ("KEY_APOSTROPHE", VirtualKeyCode::Apostrophe),
    ("KEY_COMMA", VirtualKeyCode::Comma),
    ("KEY_MINUS", VirtualKeyCode::Minus),
    ("KEY_PERIOD", VirtualKeyCode::Period),
    ("KEY_SLASH", VirtualKeyCode::Slash),
    ("KEY_0", VirtualKeyCode::Key0),
    ("KEY_1", VirtualKeyCode::Key1),
    ("KEY_2", VirtualKeyCode::Key2),
    ("KEY_3", VirtualKeyCode::Key3),
    ("KEY_4", VirtualKeyCode::Key4),
    ("KEY_5", VirtualKeyCode::Key5),
    ("KEY_6", VirtualKeyCode::Key6),
    ("KEY_7", VirtualKeyCode::Key7),
    ("KEY_8", VirtualKeyCode::Key8),
    ("KEY_9", VirtualKeyCode::Key9),
    ("KEY_SEMICOLON", VirtualKeyCode::Semicolon),
    ("KEY_EQUAL", VirtualKeyCode::Equals),
    ("KEY_A", VirtualKeyCode::A),
    ("KEY_B", VirtualKeyCode::B),
    ("KEY_C", VirtualKeyCode::C),
    ("KEY_D", VirtualKeyCode::D),
    ("KEY_E", VirtualKeyCode::E),
    ("KEY_F", VirtualKeyCode::F),
    ("KEY_G", VirtualKeyCode::G),
    ("KEY_H", VirtualKeyCode::H),
    ("KEY_I", VirtualKeyCode::I),
    ("KEY_J", VirtualKeyCode::J),
    ("KEY_K", VirtualKeyCode::K),
    ("KEY_L", VirtualKeyCode::L),
    ("KEY_M", VirtualKeyCode::M),
    ("KEY_N", VirtualKeyCode::N),
    ("KEY_O", VirtualKeyCode::O),
    ("KEY_P", VirtualKeyCode::P),
    ("KEY_Q", VirtualKeyCode::Q),
    ("KEY_R", VirtualKeyCode::R),
    ("KEY_S", VirtualKeyCode::S),
    ("KEY_T", VirtualKeyCode::T),
    ("KEY_U", VirtualKeyCode::U),
    ("KEY_V", VirtualKeyCode::V),
    ("KEY_W", VirtualKeyCode::W),
    ("KEY_X", VirtualKeyCode::X),
    ("KEY_Y", VirtualKeyCode::Y),
    ("KEY_Z", VirtualKeyCode::Z),
    ("KEY_LEFT_BRACKET", VirtualKeyCode::LBracket),
    ("KEY_BACKSLASH", VirtualKeyCode::Backslash),
    ("KEY_RIGHT_BRACKET", VirtualKeyCode::RBracket),
    ("KEY_GRAVE_ACCENT", VirtualKeyCode::Grave),
    // ("KEY_WORLD_1", VirtualKeyCode::WORLD_1),
    // ("KEY_WORLD_2", VirtualKeyCode::WORLD_2),
    ("KEY_ESCAPE", VirtualKeyCode::Escape),
    ("KEY_ENTER", VirtualKeyCode::Return),
    ("KEY_TAB", VirtualKeyCode::Tab),
    ("KEY_BACKSPACE", VirtualKeyCode::Back),
    ("KEY_INSERT", VirtualKeyCode::Insert),
    ("KEY_DELETE", VirtualKeyCode::Delete),
    ("KEY_RIGHT", VirtualKeyCode::Right),
    ("KEY_LEFT", VirtualKeyCode::Left),
    ("KEY_DOWN", VirtualKeyCode::Down),
    ("KEY_UP", VirtualKeyCode::Up),
    ("KEY_PAGE_UP", VirtualKeyCode::PageUp),
    ("KEY_PAGE_DOWN", VirtualKeyCode::PageDown),
    ("KEY_HOME", VirtualKeyCode::Home),
    ("KEY_END", VirtualKeyCode::End),
    ("KEY_CAPS_LOCK", VirtualKeyCode::Capital),
    ("KEY_SCROLL_LOCK", VirtualKeyCode::Scroll),
    ("KEY_NUM_LOCK", VirtualKeyCode::Numlock),
    ("KEY_PRINT_SCREEN", VirtualKeyCode::Snapshot),
    ("KEY_PAUSE", VirtualKeyCode::Pause),
    ("KEY_F1", VirtualKeyCode::F1),
    ("KEY_F2", VirtualKeyCode::F2),
    ("KEY_F3", VirtualKeyCode::F3),
    ("KEY_F4", VirtualKeyCode::F4),
    ("KEY_F5", VirtualKeyCode::F5),
    ("KEY_F6", VirtualKeyCode::F6),
    ("KEY_F7", VirtualKeyCode::F7),
    ("KEY_F8", VirtualKeyCode::F8),
    ("KEY_F9", VirtualKeyCode::F9),
    ("KEY_F10", VirtualKeyCode::F10),
    ("KEY_F11", VirtualKeyCode::F11),
    ("KEY_F12", VirtualKeyCode::F12),
    ("KEY_F13", VirtualKeyCode::F13),
    ("KEY_F14", VirtualKeyCode::F14),
    ("KEY_F15", VirtualKeyCode::F15),
    ("KEY_F16", VirtualKeyCode::F16),
    ("KEY_F17", VirtualKeyCode::F17),
    ("KEY_F18", VirtualKeyCode::F18),
    ("KEY_F19", VirtualKeyCode::F19),
    ("KEY_F20", VirtualKeyCode::F20),
    ("KEY_F21", VirtualKeyCode::F21),
    ("KEY_F22", VirtualKeyCode::F22),
    ("KEY_F23", VirtualKeyCode::F23),
    ("KEY_F24", VirtualKeyCode::F24),
    // ("KEY_F25", VirtualKeyCode::F25),
    ("KEY_KP_0", VirtualKeyCode::Numpad0),
    ("KEY_KP_1", VirtualKeyCode::Numpad1),
    ("KEY_KP_2", VirtualKeyCode::Numpad2),
    ("KEY_KP_3", VirtualKeyCode::Numpad3),
    ("KEY_KP_4", VirtualKeyCode::Numpad4),
    ("KEY_KP_5", VirtualKeyCode::Numpad5),
    ("KEY_KP_6", VirtualKeyCode::Numpad6),
    ("KEY_KP_7", VirtualKeyCode::Numpad7),
    ("KEY_KP_8", VirtualKeyCode::Numpad8),
    ("KEY_KP_9", VirtualKeyCode::Numpad9),
    ("KEY_KP_DECIMAL", VirtualKeyCode::Decimal),
    ("KEY_KP_DIVIDE", VirtualKeyCode::Divide),
    ("KEY_KP_MULTIPLY", VirtualKeyCode::Multiply),
    ("KEY_KP_SUBTRACT", VirtualKeyCode::Subtract),
    ("KEY_KP_ADD", VirtualKeyCode::Add),
    ("KEY_KP_ENTER", VirtualKeyCode::NumpadEnter),
    ("KEY_KP_EQUAL", VirtualKeyCode::NumpadEquals),
    ("KEY_LEFT_SHIFT", VirtualKeyCode::LShift),
    ("KEY_LEFT_CONTROL", VirtualKeyCode::LControl),
    ("KEY_LEFT_ALT", VirtualKeyCode::LAlt),
    ("KEY_LEFT_SUPER", VirtualKeyCode::LWin),
    ("KEY_RIGHT_SHIFT", VirtualKeyCode::RShift),
    ("KEY_RIGHT_CONTROL", VirtualKeyCode::RControl),
    ("KEY_RIGHT_ALT", VirtualKeyCode::RAlt),
    ("KEY_RIGHT_SUPER", VirtualKeyCode::RWin),
    // ("KEY_MENU", VirtualKeyCode::MENU),
];
