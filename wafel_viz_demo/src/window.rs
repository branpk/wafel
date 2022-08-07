use wafel_api::Error;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub trait App: Sized + 'static {
    fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Result<Self, Error>;

    fn window_event(&mut self, event: &WindowEvent) -> Result<(), Error>;

    fn update(&mut self) -> Result<(), Error>;

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        output_size: [u32; 2],
    ) -> Result<(), Error>;
}

pub fn open_window_and_run<A: App>() {
    env_logger::init();
    pollster::block_on(open_window_and_run_impl::<A>());
}

async fn open_window_and_run_impl<A: App>() {
    let event_loop = EventLoop::new();
    let max_screen_dim = event_loop
        .available_monitors()
        .flat_map(|m| [m.size().width, m.size().height])
        .max()
        .unwrap_or_default();

    let window = WindowBuilder::new()
        .with_title("Wafel Viz")
        .with_visible(false)
        .with_max_inner_size(winit::dpi::PhysicalSize::new(
            max_screen_dim,
            max_screen_dim,
        ))
        .build(&event_loop)
        .expect("failed to create window");
    let init_window_size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("failed to request GPU adapter");

    let surface = unsafe { instance.create_surface(&window) };

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits {
                    max_texture_dimension_2d: max_screen_dim,
                    ..wgpu::Limits::downlevel_defaults()
                },
            },
            None,
        )
        .await
        .expect("failed to request GPU device");

    let output_format = wgpu::TextureFormat::Bgra8Unorm;

    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: output_format,
        width: init_window_size.width,
        height: init_window_size.height,
        present_mode: wgpu::PresentMode::AutoNoVsync,
    };
    surface.configure(&device, &surface_config);

    let mut app = handle_err(A::new(&device, output_format));

    window.set_visible(true);
    let mut first_render = false;

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter);

        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => {
                handle_err(app.window_event(&event));
                match event {
                    WindowEvent::Resized(size) => {
                        surface_config.width = size.width;
                        surface_config.height = size.height;
                        if surface_config.width != 0 && surface_config.height != 0 {
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
                    handle_err(app.update());
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
                        handle_err(app.render(
                            &device,
                            &queue,
                            &output_view,
                            output_format,
                            [surface_config.width, surface_config.height],
                        ));
                    }

                    frame.present();
                }
            }
            _ => {}
        }
    });
}

#[track_caller]
fn handle_err<T>(r: Result<T, Error>) -> T {
    match r {
        Ok(v) => v,
        Err(error) => panic!("Error:\n  {}\n", error),
    }
}
