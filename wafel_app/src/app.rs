use std::fmt;

use wafel_app_ui::Wafel;
use wafel_viz::{VizRenderData, VizRenderer};
use winit::{event::WindowEvent, window::Window};

use crate::{egui_state::EguiState, env::WafelEnv, hot_reload, window::WindowedApp};

pub struct WafelApp {
    env: WafelEnv,
    egui_state: EguiState,
    viz_renderer: VizRenderer,
    viz_render_data: Vec<VizRenderData>,
    wafel: Wafel,
    msaa_samples: u32,
}

impl WindowedApp for WafelApp {
    fn new(
        env: WafelEnv,
        window: &Window,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        msaa_samples: u32,
    ) -> Self {
        WafelApp {
            env,
            egui_state: EguiState::new(window, device, output_format, msaa_samples),
            viz_renderer: VizRenderer::new(device, output_format, msaa_samples),
            viz_render_data: Vec::new(),
            wafel: Wafel::default(),
            msaa_samples,
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
            self.viz_render_data = hot_reload::wafel_show(&mut self.wafel, &self.env, ctx);
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
        let msaa_output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: output_size[0],
                height: output_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.msaa_samples,
            dimension: wgpu::TextureDimension::D2,
            format: output_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_output_view =
            msaa_output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: output_size[0],
                height: output_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.msaa_samples,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut clear_op = Some(wgpu::LoadOp::Clear(wgpu::Color {
            r: 27.0 / 255.0,
            g: 27.0 / 255.0,
            b: 27.0 / 255.0,
            a: 1.0,
        }));

        self.render_viz(
            device,
            queue,
            &mut encoder,
            output_view,
            &msaa_output_view,
            &depth_texture_view,
            output_format,
            &mut clear_op,
        );

        self.render_egui(
            device,
            queue,
            &mut encoder,
            output_view,
            &msaa_output_view,
            output_size,
            scale_factor,
            &mut clear_op,
        );

        queue.submit([encoder.finish()]);
    }
}

impl WafelApp {
    fn render_egui(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        msaa_output_view: &wgpu::TextureView,
        output_size: [u32; 2],
        scale_factor: f32,
        clear_op: &mut Option<wgpu::LoadOp<wgpu::Color>>,
    ) {
        self.egui_state
            .prepare(device, queue, encoder, output_size, scale_factor);

        let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa_output_view,
                resolve_target: Some(output_view),
                ops: wgpu::Operations {
                    load: clear_op.take().unwrap_or(wgpu::LoadOp::Load),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        self.egui_state.render(&mut rp);
    }

    fn render_viz(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        msaa_output_view: &wgpu::TextureView,
        depth_texture_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        clear_op: &mut Option<wgpu::LoadOp<wgpu::Color>>,
    ) {
        for viz_render_data in &self.viz_render_data {
            self.viz_renderer
                .prepare(device, queue, output_format, viz_render_data);

            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa_output_view,
                    resolve_target: Some(output_view),
                    ops: wgpu::Operations {
                        load: clear_op.take().unwrap_or(wgpu::LoadOp::Load),
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
    }
}

impl fmt::Debug for WafelApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WafelApp").finish_non_exhaustive()
    }
}
