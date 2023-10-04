use crate::{
    graphics::scene::Scene,
    graphics::{
        scene::Camera, ImguiCommand, ImguiCommandList, ImguiConfig, ImguiDrawData, ImguiRenderer,
        Renderer, VizContainer, IMGUI_FONT_TEXTURE_ID,
    },
    python::ImguiInput,
};
use image::ImageFormat;
use pyo3::prelude::*;
use std::{slice, time::Instant};
use wafel_api::VizScene;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Icon,
    window::WindowBuilder,
};

use super::{log, PyVizScene};

/// Open a window, call `update_fn` on each frame, and render the UI and scene(s).
pub fn open_window_and_run_impl(title: &str, update_fn: PyObject) -> PyResult<()> {
    futures::executor::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(title)
            .with_window_icon(Some(load_window_icon()))
            .with_visible(false)
            .build(&event_loop)
            .expect("failed to open window");
        window.set_maximized(true);

        let surface =
            unsafe { instance.create_surface(&window) }.expect("failed to create surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no compatible device");
        let adapter_info = adapter.get_info();
        log::info_acquire(format!(
            "GPU: {}, {:?}, {:?}",
            adapter_info.name, adapter_info.device_type, adapter_info.backend
        ));

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .unwrap();
        device.on_uncaptured_error(Box::new(|error| {
            panic!("wgpu error: {}", error);
        }));

        let swapchain_format = wgpu::TextureFormat::Bgra8Unorm; // surface.get_preferred_format(&adapter).unwrap();

        let mut config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &config);

        let gil = Python::acquire_gil();
        let py = gil.python();

        let mut imgui_input = ImguiInput::new(py)?;
        imgui_input.set_key_map(py)?;

        let imgui_config = load_imgui_config()?;
        let imgui_renderer = ImguiRenderer::new(&device, &queue, config.format, &imgui_config);
        let mut renderer = Renderer::new(&device, config.format);
        let mut viz_container = VizContainer::new(&device, config.format);

        window.set_visible(true);

        let mut last_frame_time = Instant::now();

        drop(gil);

        event_loop.run(move |event, _, control_flow| {
            let gil = Python::acquire_gil();
            let py = gil.python();
            let _gil_pool = unsafe { py.new_pool() }; // prevent memory leak

            let result: PyResult<()> = try {
                match event {
                    Event::WindowEvent { event, .. } => {
                        imgui_input.handle_event(py, &event)?;
                        match event {
                            WindowEvent::Resized(size) => {
                                config.width = size.width;
                                config.height = size.height;
                                if size.width > 0 && size.height > 0 {
                                    surface.configure(&device, &config);
                                }
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

                        let output_size = (config.width, config.height);
                        imgui_input.set_display_size(py, output_size)?;

                        let (py_imgui_draw_data, scenes, viz_scenes): (
                            &PyAny,
                            Vec<Scene>,
                            Vec<PyVizScene>,
                        ) = update_fn.as_ref(py).call0()?.extract()?;
                        let imgui_draw_data =
                            extract_imgui_draw_data(&imgui_config, py_imgui_draw_data)?;

                        if output_size.0 > 0 && output_size.1 > 0 {
                            let frame = surface.get_current_texture().unwrap();
                            let output_view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            renderer.render(
                                &device,
                                &queue,
                                &output_view,
                                output_size,
                                config.format,
                                &scenes,
                            );

                            let viz_scenes: Vec<VizScene> =
                                viz_scenes.into_iter().map(|s| s.inner).collect();
                            viz_container.render(
                                &device,
                                &queue,
                                &output_view,
                                output_size,
                                config.format,
                                &viz_scenes,
                            );

                            imgui_renderer.render(
                                &device,
                                &queue,
                                &output_view,
                                output_size,
                                &imgui_draw_data,
                            );

                            frame.present();
                        }
                    }
                    _ => {}
                }
            };

            if let Err(error) = result {
                // Most errors are caught within `update_fn` and are displayed in the UI. If
                // an error occurs outside of that scope, then just print the error to the log
                // and crash.
                panic!("(Fatal) {}", error);
            }
        })
    })
}

fn load_window_icon() -> Icon {
    let image = image::load_from_memory_with_format(
        include_bytes!("../../../wafel_app/assets/wafel.ico"),
        ImageFormat::Ico,
    )
    .unwrap()
    .to_rgba8();
    let width = image.width();
    let height = image.height();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
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
