use std::time::Instant;

use image::ImageFormat;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};

use crate::ImguiRenderer;

/// Open a window and run the main Wafel application.
pub fn run_wafel_app(render_app: Box<dyn FnMut(&imgui::Ui<'_>)>) {
    pollster::block_on(run(render_app));
}

async fn run(mut render_app: Box<dyn FnMut(&imgui::Ui<'_>)>) {
    let instance = wgpu::Instance::new(wgpu::Backends::all());

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
            force_fallback_adapter: false,
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

    let swapchain_format = wgpu::TextureFormat::Bgra8Unorm; // surface.get_preferred_format(&adapter).unwrap();

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    surface.configure(&device, &config);

    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut imgui_winit_platform = WinitPlatform::init(&mut imgui_context);
    imgui_winit_platform.attach_window(imgui_context.io_mut(), &window, HiDpiMode::Default);

    let imgui_renderer = ImguiRenderer::new(&mut imgui_context, &device, &queue, swapchain_format);

    let mut first_render = true;
    let mut prev_frame_time = Instant::now();

    window.set_visible(true);
    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter);

        *control_flow = ControlFlow::Poll;

        imgui_winit_platform.handle_event(imgui_context.io_mut(), &window, &event);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    if size.width != 0 && size.height != 0 {
                        config.width = size.width;
                        config.height = size.height;
                        surface.configure(&device, &config);
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::MainEventsCleared => {
                if config.width > 0 && config.height > 0 {
                    let frame = surface.get_current_texture().unwrap();
                    let output_view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

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

                        render_app(&ui);

                        imgui_winit_platform.prepare_render(&ui, &window);
                        let imgui_draw_data = ui.render();

                        let imgui_per_frame_data = imgui_renderer.prepare(
                            &device,
                            [config.width, config.height],
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
                                        view: &output_view,
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

                    frame.present();
                }
            }
            _ => {}
        }
    });
}

fn load_window_icon() -> Icon {
    let image =
        image::load_from_memory_with_format(include_bytes!("../../wafel.ico"), ImageFormat::Ico)
            .unwrap()
            .to_rgba8();
    let width = image.width();
    let height = image.height();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}
