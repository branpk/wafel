#![allow(clippy::needless_update)]

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    time::{Duration, Instant},
};

use wafel_api::{try_load_m64, Error, Game, Input, SaveState};
use wafel_memory::GameMemory;
use wafel_viz::{
    viz_render, CameraControl, Element, Line, ObjectCull, SurfaceMode, VizConfig, VizRenderer,
};
use window::{open_window_and_run, App};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};

mod window;

fn main() {
    open_window_and_run::<VizApp>();
}

#[derive(Debug)]
struct VizApp {
    game: Game,
    inputs: Vec<Input>,
    save_states: HashMap<u32, Rc<SaveState>>,
    camera_control: CameraControl,
    held_keys: HashSet<VirtualKeyCode>,
    viz_renderer: VizRenderer,
    last_update: Instant,
    time_since_game_advance: Duration,
}

const SAVE_STATE_FREQ: u32 = 1000;
const SAVE_STATE_DUR: u32 = 10_000;

impl VizApp {
    fn frame_advance(&mut self) -> Result<(), Error> {
        if let Some(&input) = self.inputs.get(self.game.frame() as usize) {
            self.game.try_set_input(input)?;
        }
        self.game.advance();

        if self.game.frame() % SAVE_STATE_FREQ == 0 {
            self.save_states
                .insert(self.game.frame(), Rc::new(self.game.save_state()));
            self.save_states = self
                .save_states
                .clone()
                .into_iter()
                .filter(|e| e.0 + SAVE_STATE_DUR >= self.game.frame())
                .collect();
        }

        Ok(())
    }
}

impl App for VizApp {
    fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Result<Self, Error> {
        let game = unsafe { Game::try_new("libsm64/sm64_us")? };
        let (_, inputs) = try_load_m64("wafel_viz_tests/input/120_u.m64")?;

        let mut app = VizApp {
            game,
            inputs,
            save_states: HashMap::new(),
            camera_control: CameraControl::new(),
            held_keys: HashSet::new(),
            viz_renderer: VizRenderer::new(device, output_format),
            last_update: Instant::now(),
            time_since_game_advance: Duration::ZERO,
        };

        while app.game.frame() < 0 {
            app.frame_advance()?;
        }

        Ok(app)
    }

    fn window_event(&mut self, event: &winit::event::WindowEvent) -> Result<(), Error> {
        match event {
            WindowEvent::MouseInput { state, button, .. } => match (button, state) {
                (MouseButton::Left, ElementState::Pressed) => {
                    self.camera_control.press_mouse_left()
                }
                (MouseButton::Left, ElementState::Released) => {
                    self.camera_control.release_mouse_left()
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => self
                .camera_control
                .move_mouse([position.x as f32, position.y as f32]),
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match *delta {
                    MouseScrollDelta::LineDelta(_, dy) => dy,
                    MouseScrollDelta::PixelDelta(d) => (d.y / 30.0) as f32,
                };
                self.camera_control.scroll_wheel(amount);
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(key) = input.virtual_keycode {
                    match input.state {
                        ElementState::Pressed => {
                            if key == VirtualKeyCode::Return {
                                eprintln!("frame = {}", self.game.frame());
                            }
                            if key == VirtualKeyCode::Key1 {
                                if self.held_keys.contains(&VirtualKeyCode::Right) {
                                    self.held_keys.remove(&VirtualKeyCode::Right);
                                } else {
                                    self.held_keys.insert(VirtualKeyCode::Right);
                                }
                            }
                            if key == VirtualKeyCode::Left {
                                let frame = self.game.frame().saturating_sub(1) / SAVE_STATE_FREQ
                                    * SAVE_STATE_FREQ;
                                if let Some(state) = self.save_states.get(&frame) {
                                    self.game.try_load_state(state)?;
                                }
                            }
                            if key == VirtualKeyCode::C
                                && !self.held_keys.contains(&VirtualKeyCode::C)
                            {
                                self.camera_control.lock_to_in_game_camera();
                            }
                            if key == VirtualKeyCode::M
                                && !self.held_keys.contains(&VirtualKeyCode::M)
                            {
                                self.camera_control.lock_to_mario();
                            }
                            self.held_keys.insert(key);
                        }
                        ElementState::Released => {
                            self.held_keys.remove(&key);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn update(&mut self) -> Result<(), Error> {
        self.time_since_game_advance += self.last_update.elapsed();
        self.last_update = Instant::now();

        let speed = if self.held_keys.contains(&VirtualKeyCode::Right) {
            1
        } else if self.held_keys.contains(&VirtualKeyCode::Down) {
            10
        } else if self.held_keys.contains(&VirtualKeyCode::Up) {
            100
        } else {
            0
        };

        if speed == 0 {
            self.time_since_game_advance = Duration::ZERO;
        } else {
            let dt = Duration::from_secs_f32(1.0 / 30.0) / speed;
            while self.time_since_game_advance >= dt {
                self.time_since_game_advance -= dt;
                self.frame_advance()?;
            }
        }

        let mut camera_move = [0.0, 0.0, 0.0];
        if self.held_keys.contains(&VirtualKeyCode::S) {
            camera_move[0] += 1.0;
        }
        if self.held_keys.contains(&VirtualKeyCode::A) {
            camera_move[0] -= 1.0;
        }
        if self.held_keys.contains(&VirtualKeyCode::Space) {
            camera_move[1] += 1.0;
        }
        if self.held_keys.contains(&VirtualKeyCode::LShift) {
            camera_move[1] -= 1.0;
        }
        if self.held_keys.contains(&VirtualKeyCode::R) {
            camera_move[2] += 1.0;
        }
        if self.held_keys.contains(&VirtualKeyCode::W) {
            camera_move[2] -= 1.0;
        }
        self.camera_control.update(
            &self.game.layout,
            &self.game.memory.with_slot(&self.game.base_slot),
            camera_move,
        )?;

        Ok(())
    }

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        output_size: [u32; 2],
    ) -> Result<(), Error> {
        let mut config = VizConfig {
            screen_size: output_size,
            camera: self.camera_control.camera(),
            object_cull: ObjectCull::ShowAll,
            surface_mode: SurfaceMode::Physical,
            ..Default::default()
        };

        // let camera_pos = self.game.try_read("gLakituState.pos")?.try_as_f32_3()?;
        // let camera_focus = self.game.try_read("gLakituState.focus")?.try_as_f32_3()?;
        // config.elements.push(Element::Line(Line {
        //     vertices: [camera_pos, camera_focus],
        //     color: [1.0, 0.0, 0.0, 1.0],
        // }));
        // for y in (-8000..=8000).step_by(1000) {
        //     for t in (-8000..=8000).step_by(1000) {
        //         config.elements.push(Element::Line(Line {
        //             vertices: [[t as f32, y as f32, -8000.0], [t as f32, y as f32, 8000.0]],
        //             color: [1.0, 1.0, 1.0, 1.0],
        //         }));
        //         config.elements.push(Element::Line(Line {
        //             vertices: [[-8000.0, y as f32, t as f32], [8000.0, y as f32, t as f32]],
        //             color: [1.0, 1.0, 1.0, 1.0],
        //         }));
        //     }
        // }

        let render_data = self.game.render(&config)?;

        self.viz_renderer
            .prepare(device, queue, output_format, &render_data);

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: output_size[0],
                height: output_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        });
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            self.viz_renderer.render(&mut rp);
        }

        queue.submit([encoder.finish()]);
        Ok(())
    }
}
