//! Sets up the main application window and event loop.

use image::ImageFormat;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, Window, WindowBuilder},
};

pub trait WindowedApp: Sized + 'static {
    fn new(window: &Window, device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self;

    fn window_event(&mut self, event: &WindowEvent<'_>);

    fn update(&mut self, window: &Window);

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        output_size: [u32; 2],
        scale_factor: f32,
    );
}

/// Opens a maximized window and runs the application.
///
/// This function does not return.
pub fn run_app<A: WindowedApp>(title: &str) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let event_loop = EventLoop::new();
        let max_screen_dim = event_loop
            .available_monitors()
            .flat_map(|m| [m.size().width, m.size().height])
            .max()
            .unwrap_or_default();

        let window = WindowBuilder::new()
            .with_title(title)
            .with_window_icon(Some(load_window_icon()))
            .with_visible(false)
            .with_max_inner_size(winit::dpi::PhysicalSize::new(
                max_screen_dim,
                max_screen_dim,
            ))
            .build(&event_loop)
            .expect("failed to create window");
        // window.set_maximized(true);

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

        let max_texture_dimension_2d = max_screen_dim.max(2048);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits {
                        max_texture_dimension_2d,
                        ..wgpu::Limits::downlevel_webgl2_defaults()
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

        let mut app = A::new(&window, &device, output_format);

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
                    app.window_event(&event);
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
                Event::MainEventsCleared => {
                    if !first_render {
                        app.update(&window);
                    }

                    if surface_config.width != 0 && surface_config.height != 0 {
                        let frame = surface
                            .get_current_texture()
                            .expect("failed to acquire next swap chain texture");
                        let output_view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        if first_render {
                            // Draw a black screen as quickly as possible
                            first_render = false;
                        } else {
                            app.render(
                                &device,
                                &queue,
                                &output_view,
                                output_format,
                                [surface_config.width, surface_config.height],
                                window.scale_factor() as f32,
                            );
                        }

                        frame.present();
                    }
                }
                _ => {}
            }
        });
    });
}

fn load_window_icon() -> Icon {
    let image = image::load_from_memory_with_format(
        include_bytes!("../assets/wafel.ico"),
        ImageFormat::Ico,
    )
    .unwrap()
    .to_rgba8();
    let width = image.width();
    let height = image.height();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_window_icon() {
        load_window_icon();
    }
}
