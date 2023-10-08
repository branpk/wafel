//! Sets up the main application window and event loop.

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, WindowLevel},
};

use crate::{container::Container, AppConfig, AppEnv};

/// Opens a maximized window and runs the application.
///
/// This function does not return.
pub fn open_window_and_run(config: &AppConfig, draw: impl FnMut(&dyn AppEnv) + 'static) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title(config.title())
            .with_window_icon(config.icon().cloned())
            .with_visible(false)
            .build(&event_loop)
            .expect("failed to create window");
        window.set_maximized(config.maximized());

        if config.always_on_top() {
            window.set_window_level(WindowLevel::AlwaysOnTop);
        }

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
        tracing::info!(
            "GPU: {}, {:?}, {:?}",
            adapter_info.name,
            adapter_info.device_type,
            adapter_info.backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits {
                        ..wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits())
                    },
                },
                None,
            )
            .await
            .expect("failed to request GPU device");
        device.on_uncaptured_error(Box::new(|error| {
            panic!("wgpu error: {}", error);
        }));

        let output_format = wgpu::TextureFormat::Bgra8Unorm;

        let present_mode = if surface
            .get_capabilities(&adapter)
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::AutoNoVsync
        };

        let mut surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: output_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surface_config);

        let texture_format_features = adapter.get_texture_format_features(output_format).flags;
        let msaa_samples = [4, 2]
            .into_iter()
            .find(|&count| texture_format_features.sample_count_supported(count))
            .unwrap_or(1);
        tracing::info!("MSAA samples: {msaa_samples}");

        let mut container =
            Container::new(config, draw, &window, &device, output_format, msaa_samples);

        window.set_visible(true);
        let mut first_render = false;

        event_loop.run(move |event, _, control_flow| {
            // Since event_loop.run never returns, we should move all Drop objects
            // into this closure. These ones aren't referenced elsewhere in the
            // closure, so we reference them explicitly here.
            let _ = (&instance, &adapter);

            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event, .. } => {
                    container.window_event(&window, &event);
                    match event {
                        WindowEvent::Resized(size) => {
                            surface_config.width = size.width;
                            surface_config.height = size.height;
                            if surface_config.width > 0 && surface_config.height > 0 {
                                surface.configure(&device, &surface_config);
                            }
                        }
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => {}
                    }
                }
                Event::RedrawEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    if !first_render {
                        container.update(&window, &device);
                    }

                    if surface_config.width > 0 && surface_config.height > 0 {
                        let surface_texture = surface
                            .get_current_texture()
                            .expect("failed to acquire next swap chain texture");
                        let output_view = surface_texture
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        if first_render {
                            // Draw a black screen as quickly as possible
                            first_render = false;
                        } else {
                            container.render(
                                &device,
                                &queue,
                                &output_view,
                                output_format,
                                [surface_config.width, surface_config.height],
                                window.scale_factor() as f32,
                            );
                        }

                        surface_texture.present();
                    }
                }
                _ => {}
            }
        });
    });
}
