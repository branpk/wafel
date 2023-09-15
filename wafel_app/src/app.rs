use std::{
    fmt,
    sync::{Arc, Mutex},
};

use wafel_app_ui::Wafel;
use wafel_viz::{VizRenderData, VizRenderer};
use winit::{event::WindowEvent, window::Window};

use crate::{egui_state::EguiState, env::WafelEnv, hot_reload, window::WindowedApp};

#[allow(unused)]
pub struct WafelApp {
    env: WafelEnv,
    egui_state: Arc<Mutex<Option<EguiState>>>,
    is_reloading: Arc<Mutex<bool>>,
    viz_renderer: VizRenderer,
    viz_render_data: Vec<VizRenderData>,
    wafel: Wafel,
    output_format: wgpu::TextureFormat,
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
        let egui_state = Arc::new(Mutex::new(Some(EguiState::new(
            window,
            device,
            output_format,
            msaa_samples,
        ))));

        // To avoid crashes when hot reloading, we need to drop EguiState before the reload happens,
        // and recreate it afterward.
        let is_reloading = Arc::new(Mutex::new(false));
        #[cfg(feature = "reload")]
        {
            let egui_state = Arc::clone(&egui_state);
            let is_reloading = Arc::clone(&is_reloading);

            let observer = hot_reload::subscribe();
            std::thread::spawn(move || loop {
                observer.wait_for_about_to_reload();
                *is_reloading.lock().unwrap() = true;
                *egui_state.lock().unwrap() = None;

                observer.wait_for_reload();
                *is_reloading.lock().unwrap() = false;
            });
        }

        WafelApp {
            env,
            egui_state,
            is_reloading,
            viz_renderer: VizRenderer::new(device, output_format, msaa_samples),
            viz_render_data: Vec::new(),
            wafel: Wafel::default(),
            output_format,
            msaa_samples,
        }
    }

    fn window_event(&mut self, event: &WindowEvent<'_>) {
        if let Some(egui_state) = self.egui_state.lock().unwrap().as_mut() {
            let consumed = egui_state.window_event(event);
            if !consumed {
                // handle event
            }
        }
    }

    fn update(&mut self, window: &Window, _device: &wgpu::Device) {
        // Recreate EguiState if necessary after hot reloading.
        #[cfg(feature = "reload")]
        if !*self.is_reloading.lock().unwrap() {
            let mut egui_state = self.egui_state.lock().unwrap();
            egui_state.get_or_insert_with(|| {
                EguiState::new(window, _device, self.output_format, self.msaa_samples)
            });
        }

        if let Some(egui_state) = self.egui_state.lock().unwrap().as_mut() {
            egui_state.run(window, |ctx| {
                ctx.input(|input| input.key_down(egui::Key::A));
                self.viz_render_data = hot_reload::wafel_show(&mut self.wafel, &self.env, ctx);
            });
        }
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
        self.viz_render_data
            .insert(0, VizRenderData::new([0, 0], output_size));

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
        if let Some(egui_state) = self.egui_state.lock().unwrap().as_mut() {
            egui_state.prepare(device, queue, encoder, output_size, scale_factor);

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

            egui_state.render(&mut rp);
        }
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
