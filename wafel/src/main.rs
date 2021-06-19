//! Executable for the Wafel application.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

use std::time::Instant;

use image::ImageFormat;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use log::LevelFilter;
use wafel_app::App;
use wafel_graphics::ImguiRenderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module("wgpu_core::device", LevelFilter::Warn)
        .init(); // TODO: Replace with log file
    pollster::block_on(run());
}

async fn run() {
    let mut app = App::open();

    let instance = wgpu::Instance::new(wgpu::BackendBit::all());

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Wafel") // TODO: version number
        .with_window_icon(Some(load_window_icon()))
        .with_visible(false)
        .build(&event_loop)
        .expect("failed to open window");
    window.set_maximized(true);

    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("no compatible device");
    let adapter_info = adapter.get_info();
    log::info!(
        "Selected GPU: {}, {:?}, {:?}",
        adapter_info.name,
        adapter_info.device_type,
        adapter_info.backend
    );

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("failed to create device");

    device.on_uncaptured_error(move |error| {
        log::error!("wgpu: {}", error);
        log::info!("Aborting due to previous error");
        panic!("aborting due to wgpu error");
    });

    let swap_chain_format = adapter
        .get_swap_chain_preferred_format(&surface)
        .expect("incompatible surface");
    let mut swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: swap_chain_format,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    let mut swap_chain = Some(device.create_swap_chain(&surface, &swap_chain_desc));

    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut imgui_winit_platform = WinitPlatform::init(&mut imgui_context);
    imgui_winit_platform.attach_window(imgui_context.io_mut(), &window, HiDpiMode::Default);

    let imgui_renderer = ImguiRenderer::new(&mut imgui_context, &device, &queue, swap_chain_format);

    let mut first_render = true;
    let mut prev_frame_time = Instant::now();

    window.set_visible(true);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        imgui_winit_platform.handle_event(imgui_context.io_mut(), &window, &event);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    swap_chain_desc.width = size.width;
                    swap_chain_desc.height = size.height;
                    if size.width == 0 || size.height == 0 {
                        swap_chain = None;
                    } else {
                        swap_chain = Some(device.create_swap_chain(&surface, &swap_chain_desc));
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::MainEventsCleared => {
                if let Some(swap_chain) = &swap_chain {
                    let output_view = &swap_chain
                        .get_current_frame()
                        .expect("failed to acquire swap chain texture")
                        .output
                        .view;

                    if first_render {
                        // Draw a black screen as quickly as possible
                        first_render = false;
                    } else {
                        imgui_context
                            .io_mut()
                            .update_delta_time(prev_frame_time.elapsed());
                        prev_frame_time = Instant::now();

                        imgui_winit_platform
                            .prepare_frame(imgui_context.io_mut(), &window)
                            .expect("failed to prepare frame");
                        let ui = imgui_context.frame();

                        app.render(&ui);

                        imgui_winit_platform.prepare_render(&ui, &window);
                        let imgui_draw_data = ui.render();

                        let imgui_per_frame_data = imgui_renderer.prepare(
                            &device,
                            (swap_chain_desc.width, swap_chain_desc.height),
                            imgui_draw_data,
                        );

                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: None,
                            });
                        {
                            let mut render_pass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: None,
                                    color_attachments: &[wgpu::RenderPassColorAttachment {
                                        view: output_view,
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

                            imgui_renderer.render(&mut render_pass, &imgui_per_frame_data)
                        }
                        queue.submit([encoder.finish()]);
                    }
                }
            }
            _ => {}
        }
    });
}

fn load_window_icon() -> Icon {
    let image =
        image::load_from_memory_with_format(include_bytes!("../wafel.ico"), ImageFormat::Ico)
            .unwrap()
            .to_rgba8();
    let width = image.width();
    let height = image.height();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}
