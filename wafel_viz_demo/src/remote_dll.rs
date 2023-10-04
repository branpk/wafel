use std::collections::HashSet;

use wafel_api::{Error, RemoteDll};
use wafel_viz_sm64::{InGameRenderMode, ObjectCull, PerspCameraControl, SurfaceMode, VizConfig};
use wafel_viz_wgpu::VizRenderer;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};

use crate::window::App;

#[derive(Debug)]
pub struct RemoteDllApp {
    remote_dll: RemoteDll,
    camera_control: PerspCameraControl,
    held_keys: HashSet<VirtualKeyCode>,
    viz_renderer: VizRenderer,
}

impl App for RemoteDllApp {
    fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Result<Self, Error> {
        let mut remote_dll = RemoteDll::attach(6084, 0x7FF8384E0000, "libsm64/sm64_us");
        // remote_dll
        //     .memory
        //     .load_cache(remote_dll.address("gGlobalTimer").unwrap())
        //     .unwrap();
        Ok(Self {
            remote_dll,
            camera_control: PerspCameraControl::new(),
            held_keys: HashSet::new(),
            viz_renderer: VizRenderer::new(device, output_format, 1),
        })
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
            &self.remote_dll.layout,
            &self.remote_dll.memory,
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
        scale_factor: f32,
    ) -> Result<(), Error> {
        let config = VizConfig {
            screen_size: [
                (output_size[0] as f32 / scale_factor) as u32,
                (output_size[1] as f32 / scale_factor) as u32,
            ],
            in_game_render_mode: if self.held_keys.contains(&VirtualKeyCode::X) {
                InGameRenderMode::DisplayList
            } else if self.held_keys.contains(&VirtualKeyCode::Z) {
                InGameRenderMode::Disabled
            } else {
                InGameRenderMode::Rerender
            },
            camera: self.camera_control.camera(),
            object_cull: ObjectCull::ShowAll,
            ..Default::default()
        };

        let scene = self.remote_dll.render(&config);

        self.viz_renderer.prepare(
            device,
            queue,
            output_format,
            output_size,
            scale_factor,
            &scene,
        );

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
            view_formats: &[],
        });
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
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
