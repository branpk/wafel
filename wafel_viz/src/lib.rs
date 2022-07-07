#![feature(stmt_expr_attributes)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![allow(
    clippy::map_entry,
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::needless_update
)]
#![allow(missing_docs)] // FIXME: remove

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    f32::consts::PI,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use custom_renderer::{CustomRenderer, Scene};
use fast3d::{interpret::F3DRenderData, render::F3DRenderer};
use sm64_gfx_tree::get_gfx_node_reader;
pub use sm64_render_mod::SM64RenderConfig;
use sm64_render_mod::{render_sm64_with_config, Camera};
use wafel_api::{load_m64, Game, SaveState};
use wafel_data_path::GlobalDataPath;
use wafel_memory::GameMemory;
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod custom_renderer;
mod sm64_gfx_tree;
mod sm64_render_mod;

pub fn prepare_render_data(game: &Game, config: &SM64RenderConfig) -> F3DRenderData {
    let memory = game.memory.with_slot(&game.base_slot);
    let get_path = |source: &str| {
        GlobalDataPath::compile(&game.layout, &game.memory, source)
            .map(Arc::new)
            .map_err(Into::into)
    };

    render_sm64_with_config(&memory, get_path, config).expect("failed to process display list")
}

pub fn test_dl() -> Result<(), Box<dyn Error>> {
    let mut game = unsafe { Game::new("libsm64/sm64_us.dll") };
    let (_, inputs) = load_m64("wafel_viz_tests/input/120_u.m64");

    while game.frame() < 1624 {
        game.set_input(inputs[game.frame() as usize]);
        game.advance();
    }

    let memory = game.memory.with_slot(&game.base_slot);
    let node_reader = get_gfx_node_reader(&memory, &game.layout)?;

    if let Some(root_addr) = game
        .read("gCurrentArea?.unk04")
        .option()
        .map(|v| v.try_as_address())
        .transpose()?
    {
        let node = node_reader.read(root_addr)?;
        eprintln!("{:?}", node);
    }

    return Ok(());

    env_logger::init();
    futures::executor::block_on(run(0, None)).unwrap();
    Ok(())
}

pub fn test(frame0: u32) -> Result<(), Box<dyn Error>> {
    env_logger::init();
    futures::executor::block_on(run(frame0, None))
}

async fn run(frame0: u32, arg_data: Option<F3DRenderData>) -> Result<(), Box<dyn Error>> {
    let mut game = unsafe { Game::new("libsm64/sm64_us.dll") };
    // let (_, inputs) = load_m64("../sm64-bot/bad_bot.m64");
    let (_, inputs) = load_m64("wafel_viz_tests/input/120_u.m64");
    // let (_, inputs) = load_m64("test_files/lod-test.m64");

    let mut save_states: HashMap<u32, Rc<SaveState>> = HashMap::new();
    let save_state_freq = 1000;
    let save_state_dur = 10_000;

    while game.frame() < frame0 {
        if let Some(&input) = inputs.get(game.frame() as usize) {
            game.set_input(input);
        }
        game.advance();

        if game.frame() % save_state_freq == 0 {
            save_states.insert(game.frame(), Rc::new(game.save_state()));
            save_states = save_states
                .clone()
                .into_iter()
                .filter(|e| e.0 + save_state_dur >= game.frame())
                .collect();
        }
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

    let mut renderer = F3DRenderer::new(&device);

    window.set_visible(true);
    let mut first_render = false;

    let mut held = HashSet::new();
    let mut last_update = Instant::now();

    let mut fixed_camera_pos = None;

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
                                if key == VirtualKeyCode::Space {
                                    if held.contains(&VirtualKeyCode::Right) {
                                        held.remove(&VirtualKeyCode::Right);
                                    } else {
                                        held.insert(VirtualKeyCode::Right);
                                    }
                                }
                                if key == VirtualKeyCode::Left {
                                    let frame = game.frame().saturating_sub(1) / save_state_freq
                                        * save_state_freq;
                                    if let Some(state) = save_states.get(&frame) {
                                        game.load_state(state);
                                    }
                                }
                                if key == VirtualKeyCode::C && !held.contains(&VirtualKeyCode::C) {
                                    fixed_camera_pos =
                                        Some(game.read("gLakituState.pos").as_f32_3());
                                }
                                held.insert(key);
                            }
                            ElementState::Released => {
                                if key == VirtualKeyCode::C && held.contains(&VirtualKeyCode::C) {
                                    fixed_camera_pos = None;
                                }
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
                                if game.frame() % save_state_freq == 0 {
                                    save_states.insert(game.frame(), Rc::new(game.save_state()));
                                    save_states = save_states
                                        .clone()
                                        .into_iter()
                                        .filter(|e| e.0 + save_state_dur >= game.frame())
                                        .collect();
                                }
                            }
                        }

                        let render_data = arg_data.clone().unwrap_or_else(|| {
                            prepare_render_data(
                                &game,
                                &SM64RenderConfig {
                                    screen_size: (config.width, config.height),
                                    camera: fixed_camera_pos
                                        .map(|pos| Camera::LookAt {
                                            pos,
                                            focus: game.read("gLakituState.focus").as_f32_3(),
                                            roll: game.read("gLakituState.roll").as_int() as f32
                                                * PI
                                                / 0x8000 as f32,
                                        })
                                        .unwrap_or_default(),
                                    ..Default::default()
                                },
                            )
                        });
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
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                        store: true,
                                    },
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
                            renderer.render(&mut rp, (config.width, config.height));
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

pub async fn custom_render_test(scene: Scene) -> Result<(), Box<dyn Error>> {
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

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: init_window_size.width,
        height: init_window_size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    surface.configure(&device, &config);

    let mut renderer = CustomRenderer::new(&device, config.format);

    window.set_visible(true);
    let mut first_render = false;

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &renderer, &scene);

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
                        renderer.prepare(&device, &scene);

                        // let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                        //     label: None,
                        //     size: wgpu::Extent3d {
                        //         width: config.width,
                        //         height: config.height,
                        //         depth_or_array_layers: 1,
                        //     },
                        //     mip_level_count: 1,
                        //     sample_count: 1,
                        //     dimension: wgpu::TextureDimension::D2,
                        //     format: wgpu::TextureFormat::Depth24Plus,
                        //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        // });
                        // let depth_texture_view =
                        //     depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                        store: true,
                                    },
                                }],
                                depth_stencil_attachment: None,
                                // depth_stencil_attachment: Some(
                                //     wgpu::RenderPassDepthStencilAttachment {
                                //         view: &depth_texture_view,
                                //         depth_ops: Some(wgpu::Operations {
                                //             load: wgpu::LoadOp::Clear(1.0),
                                //             store: true,
                                //         }),
                                //         stencil_ops: None,
                                //     },
                                // ),
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
}
