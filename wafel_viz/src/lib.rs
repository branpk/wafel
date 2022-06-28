#![feature(stmt_expr_attributes)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![allow(clippy::map_entry)]

use std::error::Error;

use sm64_render_data::sm64_update_and_render;
use sm64_renderer::SM64Renderer;
use wafel_memory::{DllGameMemory, GameMemory};
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::render_api::{decode_shader_id, CCFeatures};

mod render_api;
mod sm64_render_data;
mod sm64_renderer;

pub fn test() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    futures::executor::block_on(run())
}

async fn run() -> Result<(), Box<dyn Error>> {
    let (memory, mut base_slot) = unsafe {
        DllGameMemory::load(
            "../libsm64-build/build/us_lib/sm64_us.dll",
            "sm64_init",
            "sm64_update",
        )?
    };
    for _ in 0..2500 {
        memory.advance_base_slot(&mut base_slot);
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Wafel Viz")
        .with_visible(false)
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
                // features: wgpu::Features::empty(),
                features: wgpu::Features::POLYGON_MODE_LINE,
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .expect("failed to request GPU device");

    let output_format = wgpu::TextureFormat::Bgra8Unorm;

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: output_format,
        width: init_window_size.width,
        height: init_window_size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    surface.configure(&device, &config);

    let mut renderer = SM64Renderer::new(&device);

    window.set_visible(true);
    let mut first_render = false;

    let mut second_render = true;
    let mut held = false;

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &renderer);

        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    config.width = size.width;
                    config.height = size.height;
                    if config.width != 0 && config.height != 0 {
                        surface.configure(&device, &config);
                    }
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    held = input.state == ElementState::Pressed;
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                if config.width != 0 && config.height != 0 {
                    let frame = surface
                        .get_current_texture()
                        .expect("failed to acquire next swap chain texture");
                    let output_view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    if first_render {
                        // Draw a black screen as quickly as possileb
                        first_render = false;
                    } else {
                        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                            label: None,
                            size: wgpu::Extent3d {
                                width: config.width,
                                height: config.height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Depth24Plus,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        });
                        let depth_texture_view =
                            depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

                        if second_render || held {
                            second_render = false;
                            let render_data =
                                sm64_update_and_render(&memory, &mut base_slot, 640, 480)
                                    .expect("failed to render game");
                            renderer.prepare(&device, &queue, output_format, &render_data);
                        }

                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: None,
                            });

                        {
                            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: None,
                                color_attachments: &[wgpu::RenderPassColorAttachment {
                                    view: &output_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations::default(),
                                }],
                                depth_stencil_attachment: Some(
                                    wgpu::RenderPassDepthStencilAttachment {
                                        view: &depth_texture_view,
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(1.0),
                                            store: true,
                                        }),
                                        stencil_ops: None,
                                    },
                                ),
                            });
                            renderer.render(&mut rp);
                        }

                        queue.submit([encoder.finish()]);
                    }

                    frame.present();
                }
            }
            _ => {}
        }
    });

    Ok(())
}
