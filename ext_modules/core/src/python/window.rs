use crate::{
    graphics::scene::Scene,
    graphics::{
        ImguiCommand, ImguiCommandList, ImguiConfig, ImguiDrawData, ImguiRenderer, Renderer,
        IMGUI_FONT_TEXTURE_ID,
    },
    python::ImguiInput,
};
use pyo3::prelude::*;
use std::{slice, time::Instant};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn open_window_and_run_impl(title: &str, update_fn: PyObject) -> PyResult<()> {
    // TODO: Error handling (and/or make sure panics show up in log)
    futures::executor::block_on(async {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(title)
            .with_visible(false)
            .build(&event_loop)
            .expect("failed to open window");
        window.set_maximized(true);

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

        let mut imgui_input = ImguiInput::new(py)?;
        imgui_input.set_key_map(py)?;

        let imgui_config = load_imgui_config()?;
        let imgui_renderer =
            ImguiRenderer::new(&device, &queue, swap_chain_desc.format, &imgui_config);
        let mut renderer = Renderer::new(&device, swap_chain_desc.format);

        window.set_visible(true);

        let mut last_frame_time = Instant::now();

        event_loop.run(move |event, _, control_flow| {
            let gil = Python::acquire_gil();
            let py = gil.python();

            let result: PyResult<()> = try {
                match event {
                    Event::WindowEvent { event, .. } => {
                        imgui_input.handle_event(py, &event)?;
                        match event {
                            WindowEvent::Resized(size) => {
                                swap_chain_desc.width = size.width;
                                swap_chain_desc.height = size.height;
                                swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
                            }
                            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                            _ => {}
                        }
                    }
                    Event::MainEventsCleared => window.request_redraw(),
                    Event::RedrawRequested(_) => {
                        let delta_time = last_frame_time.elapsed().as_secs_f64();
                        last_frame_time = Instant::now();
                        imgui_input.set_delta_time(py, delta_time)?;

                        let output_size = (swap_chain_desc.width, swap_chain_desc.height);
                        imgui_input.set_display_size(py, output_size)?;

                        let (py_imgui_draw_data, scenes): (&PyAny, Vec<Scene>) =
                            update_fn.as_ref(py).call0()?.extract()?;
                        let imgui_draw_data =
                            extract_imgui_draw_data(&imgui_config, py_imgui_draw_data)?;

                        if output_size.0 > 0 && output_size.1 > 0 {
                            let output_view = &swap_chain.get_current_frame().unwrap().output.view;

                            renderer.render(
                                &device,
                                &queue,
                                output_view,
                                output_size,
                                swap_chain_desc.format,
                                &scenes,
                            );

                            imgui_renderer.render(
                                &device,
                                &queue,
                                output_view,
                                output_size,
                                swap_chain_desc.format,
                                &imgui_draw_data,
                            );
                        }
                    }
                    _ => {}
                }
            };

            if let Err(error) = result {
                error.print(py);
                todo!()
            }
        })
    })
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
