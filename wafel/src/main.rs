use log::LevelFilter;
use wafel_graphics::{ImguiPerFrameData, ImguiRenderer};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// TODO
// #![warn(
//     missing_docs,
//     missing_debug_implementations,
//     rust_2018_idioms,
//     unreachable_pub
// )]

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module("wgpu_core::device", LevelFilter::Warn)
        .init(); // TODO: Replace with log file
    pollster::block_on(run());
}

async fn run() {
    let instance = wgpu::Instance::new(wgpu::BackendBit::all());

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Wafel") // TODO
        .with_window_icon(None) // TODO
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

    let mut app = App::new(&device, &queue, swap_chain_format);

    let mut first_render = true;

    window.set_visible(true);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
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
                        let draw_data =
                            app.run_frame(&device, (swap_chain_desc.width, swap_chain_desc.height));

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
                                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                            store: true,
                                        },
                                    }],
                                    depth_stencil_attachment: None,
                                });

                            app.render(&mut render_pass, &draw_data);
                        }
                        queue.submit([encoder.finish()]);
                    }
                }
            }
            _ => {}
        }
    });
}

#[derive(Debug)]
struct App {
    imgui_context: imgui::Context,
    imgui_renderer: ImguiRenderer,
}

struct AppDrawData {
    imgui_per_frame_data: ImguiPerFrameData,
}

impl App {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, output_format: wgpu::TextureFormat) -> Self {
        let mut imgui_context = imgui::Context::create();

        let imgui_renderer = ImguiRenderer::new(&mut imgui_context, device, queue, output_format);

        Self {
            imgui_context,
            imgui_renderer,
        }
    }

    fn run_frame(&mut self, device: &wgpu::Device, output_size: (u32, u32)) -> AppDrawData {
        self.imgui_context.io_mut().display_size = [output_size.0 as f32, output_size.1 as f32];

        let ui = self.imgui_context.frame();

        // application logic

        imgui::Window::new(imgui::im_str!("test window"))
            .size([300.0, 100.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text("Hello world");
            });

        // end application logic

        let imgui_draw_data = ui.render();

        // TODO: Ideally have two methods:
        // - run_frame(&mut self, output_size: (u32, u32)) -> AppFrameOutput
        // - prepare(&self, output: &AppFrameOutput) -> AppDrawData
        // Need to copy imgui draw data into buffers though
        let imgui_per_frame_data =
            self.imgui_renderer
                .prepare(device, output_size, imgui_draw_data);

        AppDrawData {
            imgui_per_frame_data,
        }
    }

    fn render<'r>(&'r self, render_pass: &mut wgpu::RenderPass<'r>, data: &'r AppDrawData) {
        self.imgui_renderer
            .render(render_pass, &data.imgui_per_frame_data);
    }
}
