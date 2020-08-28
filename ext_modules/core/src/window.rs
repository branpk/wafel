use crate::graphics::{ImguiConfig, ImguiDrawData, ImguiRenderer};
use std::time::Instant;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn run(
    title: &str,
    imgui_config: &ImguiConfig,
    mut update_fn: impl FnMut((u32, u32)) -> ImguiDrawData + 'static,
) -> ! {
    // TODO: Error handling (and/or make sure panics show up in log)
    futures::executor::block_on(async {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(PhysicalSize::new(800, 600))
            .with_maximized(true)
            .with_visible(false)
            .build(&event_loop)
            .expect("failed to open window");

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

        let imgui_renderer =
            ImguiRenderer::new(&device, &queue, swap_chain_desc.format, imgui_config);

        // Get the slow first frame out of the way before making the window visible to reduce
        // the amount of time that the window shows garbage.
        update_fn((swap_chain_desc.width, swap_chain_desc.height));
        window.set_visible(true);

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    swap_chain_desc.width = size.width;
                    swap_chain_desc.height = size.height;
                    swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => {
                let output_size = (swap_chain_desc.width, swap_chain_desc.height);

                let imgui_draw_data = update_fn(output_size);

                if output_size.0 > 0 && output_size.1 > 0 {
                    let output_view = &swap_chain.get_current_frame().unwrap().output.view;

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
        })
    })
}
