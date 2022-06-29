#![feature(stmt_expr_attributes)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![allow(clippy::map_entry)]

use std::{
    collections::HashSet,
    error::Error,
    time::{Duration, Instant},
};

use n64_display_list::parse_display_list;
use n64_render_backend::process_display_list;
use n64_renderer::N64Renderer;
use wafel_api::{load_m64, Emu, Game, IntType};
use wafel_memory::{DllGameMemory, GameMemory, MemoryRead};
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    n64_display_list::{DLCommand, SPCommand},
    render_api::{decode_shader_id, CCFeatures},
};
pub use n64_render_data::*;

mod n64_display_list;
mod n64_render_backend;
mod n64_render_data;
mod n64_renderer;
mod render_api;

pub fn test_dl() -> Result<(), Box<dyn Error>> {
    let mut game = unsafe { Game::new("../libsm64-build/build/us_lib/sm64_us.dll") };
    let (_, inputs) = load_m64("test_files/120_u.m64");
    while game.read("gGlobalTimer").as_int() < 927 {
        game.set_input(inputs[game.frame() as usize]);
        game.advance();
    }
    // let dl_addr = game.read("sDisplayListTask->task.t.data_ptr").as_address();
    let global_timer = game.read("gGlobalTimer");
    let dl_buffer_index = (global_timer.as_int() + 1) % 2;
    let dl_addr = game
        .address(&format!("gGfxPools[{}].buffer", dl_buffer_index))
        .unwrap();

    let view = game.memory.with_slot(&game.base_slot);
    let raw_dl = (0..).map(|i| {
        let i0 = 2 * i;
        let i1 = 2 * i + 1;
        let w0 = view.read_int(dl_addr + 8 * i0, IntType::U64).unwrap() as u64;
        let w1 = view.read_int(dl_addr + 8 * i1, IntType::U64).unwrap() as u64;
        [w0, w1]
    });

    let dl = parse_display_list(raw_dl);

    eprintln!("Display list:");
    for cmd in dl {
        eprintln!("  {:?}", cmd);
        if cmd == DLCommand::Rsp(SPCommand::EndDisplayList) {
            break;
        }
    }

    // eprintln!();
    // let emu = Emu::attach(14768, 0x0050B110, wafel_api::SM64Version::US);
    // let global_timer = emu.read("gGlobalTimer");
    // let dl_buffer_index = (global_timer.as_int() + 1) % 2;
    // let dl_addr = emu
    //     .address(&format!("gGfxPools[{}].buffer", dl_buffer_index))
    //     .unwrap();

    Ok(())
}

pub fn test(frame0: u32) -> Result<(), Box<dyn Error>> {
    env_logger::init();
    futures::executor::block_on(run(frame0))
}

async fn run(frame0: u32) -> Result<(), Box<dyn Error>> {
    let mut game = unsafe { Game::new("../libsm64-build/build/us_lib/sm64_us.dll") };
    let (_, inputs) = load_m64("../sm64-bot/bad_bot.m64");

    while game.frame() < frame0 {
        if let Some(&input) = inputs.get(game.frame() as usize) {
            game.set_input(input);
        }
        game.advance();
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

    let mut renderer = N64Renderer::new(&device);

    window.set_visible(true);
    let mut first_render = false;

    let mut held = HashSet::new();
    let mut last_update = Instant::now();

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
                    if let Some(key) = input.virtual_keycode {
                        match input.state {
                            ElementState::Pressed => {
                                if key == VirtualKeyCode::Return {
                                    eprintln!("frame = {}", game.frame());
                                }
                                held.insert(key);
                            }
                            ElementState::Released => {
                                held.remove(&key);
                            }
                        }
                    }
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

                        if last_update.elapsed() >= Duration::from_secs_f32(1.0 / 30.0) {
                            last_update = Instant::now();

                            let num_frames = if held.contains(&VirtualKeyCode::Right) {
                                1
                            } else if held.contains(&VirtualKeyCode::PageDown) {
                                10
                            } else if held.contains(&VirtualKeyCode::PageUp) {
                                100
                            } else {
                                0
                            };
                            for _ in 0..num_frames {
                                if let Some(input) = inputs.get(game.frame() as usize) {
                                    game.write("gControllerPads[0].button", input.buttons.into());
                                    game.write("gControllerPads[0].stick_x", input.stick_x.into());
                                    game.write("gControllerPads[0].stick_y", input.stick_y.into());
                                }
                                game.advance();
                            }
                        }

                        let render_data = process_display_list(
                            &game.memory,
                            &mut game.base_slot,
                            config.width,
                            config.height,
                        )
                        .expect("failed to render game");
                        renderer.prepare(&device, &queue, output_format, &render_data);

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
