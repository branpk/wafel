use log::LevelFilter;
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
    env_logger::builder().filter_level(LevelFilter::Info).init(); // TODO: Replace with log file
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

    let triangle_renderer = TriangleRenderer::new(&device, swap_chain_format);

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
                            triangle_renderer.render(&mut render_pass);
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
struct TriangleRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl TriangleRenderer {
    fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(&wgpu::include_wgsl!("../shaders/triangle.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle-pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[output_format.into()],
            }),
        });

        Self { pipeline }
    }

    fn render<'r>(&'r self, render_pass: &mut wgpu::RenderPass<'r>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw(0..3, 0..1);
    }
}
