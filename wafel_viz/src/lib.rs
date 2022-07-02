#![feature(stmt_expr_attributes)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![allow(clippy::map_entry, clippy::needless_range_loop)]
#![allow(missing_docs)] // FIXME: remove

use std::{
    collections::HashSet,
    error::Error,
    mem,
    time::{Duration, Instant},
};

use custom_renderer::{CustomRenderer, Scene};
use f3d_decode::{decode_f3d_display_list, F3DCommandIter, RawF3DCommand};
use f3d_interpret::{interpret_f3d_display_list, F3DSource};
use n64_render_backend::{process_display_list, N64RenderBackend};
use n64_renderer::N64Renderer;
use wafel_api::{load_m64, Address, Emu, Game, IntType};
use wafel_memory::{DllGameMemory, DllSlot, DllSlotMemoryView, GameMemory, MemoryRead};
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    f3d_decode::{F3DCommand, SPCommand},
    render_api::{decode_shader_id, CCFeatures},
};
pub use n64_render_data::*;

pub mod custom_renderer;
pub mod f3d_decode;
mod f3d_interpret;
mod n64_render_backend;
mod n64_render_data;
mod n64_renderer;
mod render_api;

#[derive(Debug)]
pub struct DllF3DSource<'a> {
    game: &'a Game,
}

impl<'a> DllF3DSource<'a> {
    fn view(&self) -> DllSlotMemoryView<'a> {
        self.game.memory.with_slot(&self.game.base_slot)
    }

    fn read_dl_from_addr(&self, addr: Option<Address>) -> F3DCommandIter<RawDlIter<'a>> {
        decode_f3d_display_list(RawDlIter {
            view: self.view(),
            addr,
        })
    }
}

impl<'a> F3DSource for DllF3DSource<'a> {
    type Ptr = *const ();
    type DlIter = F3DCommandIter<RawDlIter<'a>>;

    fn root_dl(&self) -> Self::DlIter {
        let root_addr = self
            .game
            .read("sDisplayListTask?.task.t.data_ptr")
            .option()
            .map(|a| a.as_address());
        self.read_dl_from_addr(root_addr)
    }

    fn read_dl(&self, ptr: Self::Ptr) -> Self::DlIter {
        let addr = self.game.memory.unchecked_pointer_to_address(ptr);
        self.read_dl_from_addr(Some(addr))
    }

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize) {
        let view = self.view();
        let addr = self.game.memory.unchecked_pointer_to_address(ptr) + offset;
        for i in 0..dst.len() {
            dst[i] = view.read_int(addr + i, IntType::U8).unwrap_or_default() as u8;
        }
    }

    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize) {
        let view = self.view();
        let addr = self.game.memory.unchecked_pointer_to_address(ptr) + offset;
        for i in 0..dst.len() {
            dst[i] = view
                .read_int(addr + 2 * i, IntType::U16)
                .unwrap_or_default() as u16;
        }
    }

    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize) {
        let view = self.view();
        let addr = self.game.memory.unchecked_pointer_to_address(ptr) + offset;
        for i in 0..dst.len() {
            dst[i] = view
                .read_int(addr + 4 * i, IntType::U32)
                .unwrap_or_default() as u32;
        }
    }
}

#[derive(Debug)]
pub struct RawDlIter<'a> {
    view: DllSlotMemoryView<'a>,
    addr: Option<Address>,
}

impl<'a> Iterator for RawDlIter<'a> {
    type Item = RawF3DCommand<*const ()>;

    fn next(&mut self) -> Option<Self::Item> {
        self.addr.as_mut().map(|addr| {
            let w_type = IntType::u_ptr_native();
            let w_size = w_type.size();

            let w0 = self.view.read_int(*addr, w_type).unwrap() as usize;
            *addr += w_size;
            let w1 = self.view.read_int(*addr, w_type).unwrap() as usize;
            *addr += w_size;

            RawF3DCommand {
                w0: w0 as u32,
                w1: w1 as u32,
                w1_ptr: w1 as *const (),
            }
        })
    }
}

pub fn test_dl() -> Result<(), Box<dyn Error>> {
    // ~~~ libsm64 ~~~
    //     let mut game = unsafe { Game::new("../libsm64-build/build/us_lib/sm64_us.dll") };
    //     let (_, inputs) = load_m64("test_files/120_u.m64");
    //     while game.read("gGlobalTimer").as_int() < 927 {
    //         game.set_input(inputs[game.frame() as usize]);
    //         game.advance();
    //     }
    //
    //     let mut backend = N64RenderBackend::default();
    //     let f3d_source = DllF3DSource { game: &game };
    //     interpret_f3d_display_list(&f3d_source, &mut backend);
    //     let data0 = backend.finish();
    //
    //     let data1 = process_display_list(&game.memory, &mut game.base_slot, 320, 240).unwrap();

    // let vs0: Vec<f32> = data0
    //     .commands
    //     .iter()
    //     .flat_map(|c| &c.vertex_buffer)
    //     .cloned()
    //     .collect();
    // let vs1: Vec<f32> = data1
    //     .commands
    //     .iter()
    //     .flat_map(|c| &c.vertex_buffer)
    //     .cloned()
    //     .collect();
    // assert_eq!(vs0.len(), vs1.len());
    // eprintln!("{:?}", data0.commands[0].vertex_buffer[0..4].to_vec());
    // eprintln!("{:?}", data1.commands[0].vertex_buffer[0..4].to_vec());

    // assert!(data0.compare(&data1));
    env_logger::init();
    futures::executor::block_on(run(3468, None, true)).unwrap();
    return Ok(());

    //     let w_type = IntType::u_ptr_native();
    //     let w_size = w_type.size();
    //
    //     let view = game.memory.with_slot(&game.base_slot);
    //     let raw_dl = (0..).map(|i| {
    //         let i0 = 2 * i;
    //         let i1 = 2 * i + 1;
    //         let w0 = view.read_int(dl_addr + w_size * i0, w_type).unwrap() as usize;
    //         let w1 = view.read_int(dl_addr + w_size * i1, w_type).unwrap() as usize;
    //         [w0, w1]
    //     });

    // ~~~ emu ~~~
    //     let emu = Emu::attach(20644, 0x0050B110, wafel_api::SM64Version::US);
    //     let global_timer = emu.read("gGlobalTimer");
    //     let dl_buffer_index = (global_timer.as_int() + 1) % 2;
    //     let dl_addr = emu
    //         .address(&format!("gGfxPools[{}].buffer", dl_buffer_index))
    //         .unwrap();
    //
    //     let raw_dl = (0..).map(|i| {
    //         let i0 = 2 * i;
    //         let i1 = 2 * i + 1;
    //         let w0 = emu.memory.read_int(dl_addr + 4 * i0, IntType::U32).unwrap() as u32;
    //         let w1 = emu.memory.read_int(dl_addr + 4 * i1, IntType::U32).unwrap() as u32;
    //         [w0, w1]
    //     });

    // ~~~ shared ~~~
    //     let dl = decode_f3d_display_list(raw_dl);
    //
    //     eprintln!("Display list:");
    //     for cmd in dl {
    //         if let F3DCommand::Unknown { w0, w1 } = cmd {
    //             eprintln!("  Unknown: {:08X} {:08X}", w0, w1);
    //         } else {
    //             eprintln!("  {:?}", cmd);
    //         }
    //         if cmd == F3DCommand::Rsp(SPCommand::EndDisplayList) {
    //             break;
    //         }
    //     }

    Ok(())
}

pub fn test(frame0: u32) -> Result<(), Box<dyn Error>> {
    env_logger::init();
    futures::executor::block_on(run(frame0, None, false))
}

async fn run(
    frame0: u32,
    arg_data: Option<N64RenderData>,
    use_rust_f3d: bool,
) -> Result<(), Box<dyn Error>> {
    let mut game = unsafe { Game::new("../libsm64-build/build/us_lib/sm64_us.dll") };
    // let (_, inputs) = load_m64("../sm64-bot/bad_bot.m64");
    let (_, inputs) = load_m64("test_files/120_u.m64");
    // let (_, inputs) = load_m64("test_files/lod-test.m64");

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

                        let render_data = arg_data.clone().unwrap_or_else(|| {
                            if use_rust_f3d {
                                let mut backend = N64RenderBackend::default();
                                let f3d_source = DllF3DSource { game: &game };
                                interpret_f3d_display_list(&f3d_source, &mut backend);
                                backend.finish()
                            } else {
                                process_display_list(
                                    &game.memory,
                                    &mut game.base_slot,
                                    config.width,
                                    config.height,
                                )
                                .expect("failed to render game")
                            }
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
