use std::fmt;

use wafel_app_logic::Wafel;
use winit::{event::WindowEvent, window::Window};

use crate::{egui_state::EguiState, env::WafelEnv, window::WindowedApp};

pub struct WafelApp {
    env: WafelEnv,
    egui_state: EguiState,
    wafel: Wafel,
}

impl WindowedApp for WafelApp {
    fn new(
        env: WafelEnv,
        window: &Window,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        WafelApp {
            env,
            egui_state: EguiState::new(window, device, output_format),
            wafel: Wafel::default(),
        }
    }

    fn window_event(&mut self, event: &WindowEvent<'_>) {
        let consumed = self.egui_state.window_event(event);
        if !consumed {
            // handle event
        }
    }

    fn update(&mut self, window: &Window) {
        self.egui_state.run(window, |ctx| {
            self.wafel.show(&self.env, ctx);
        });
    }

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        output_size: [u32; 2],
        scale_factor: f32,
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.egui_state
            .prepare(device, queue, &mut encoder, output_size, scale_factor);

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
                depth_stencil_attachment: None,
                //  Some(wgpu::RenderPassDepthStencilAttachment {
                //     view: &depth_texture_view,
                //     depth_ops: Some(wgpu::Operations {
                //         load: wgpu::LoadOp::Clear(1.0),
                //         store: true,
                //     }),
                //     stencil_ops: None,
                // }),
            });

            self.egui_state.render(&mut rp);
        }

        queue.submit([encoder.finish()]);
    }
}

impl fmt::Debug for WafelApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WafelApp").finish_non_exhaustive()
    }
}
