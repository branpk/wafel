use std::{
    fmt,
    sync::{Arc, Mutex},
};

use wafel_viz::VizScene;
use wafel_viz_wgpu::VizRenderer;
use winit::{event::WindowEvent, window::Window};

use crate::{
    egui_state::EguiState, fps_counter::FpsCounter, logging, wgpu_util::CachedTexture,
    window_env::WindowEnv, AppConfig,
};

#[derive(Debug)]
struct WindowEnvImpl<'a> {
    config: &'a AppConfig,
    fps: f32,
    mspf: f32,
    egui_ctx: egui::Context,
    viz_scenes: Mutex<Vec<VizScene>>,
}

static_assertions::assert_impl_all!(WindowEnvImpl<'_>: Send, Sync);

impl WindowEnv for WindowEnvImpl<'_> {
    fn config(&self) -> &AppConfig {
        self.config
    }

    fn fps(&self) -> f32 {
        self.fps
    }

    fn mspf(&self) -> f32 {
        self.mspf
    }

    fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    fn draw_viz(&self, scene: VizScene) {
        self.viz_scenes.lock().unwrap().push(scene);
    }

    fn take_recent_panic_details(&self) -> Option<String> {
        logging::take_recent_panic_details()
    }
}

#[allow(unused)]
pub struct Container<D> {
    config: AppConfig,
    draw: D,
    egui_state: Arc<Mutex<Option<EguiState>>>,
    viz_renderer: VizRenderer,
    viz_scenes: Vec<VizScene>,
    output_format: wgpu::TextureFormat,
    msaa_samples: u32,
    msaa_texture: CachedTexture,
    depth_texture: CachedTexture,
    fps_counter: FpsCounter,
}

impl<D: FnMut(&dyn WindowEnv)> Container<D> {
    pub fn new(
        config: &AppConfig,
        draw: D,
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

        if let Some(subscriber) = config.hot_reload_subscriber() {
            let egui_state = Arc::clone(&egui_state);
            let observer = subscriber();
            std::thread::spawn(move || loop {
                // Ensure that rendering/updating doesn't overlap with the dll reload
                let block_reload = observer.wait_for_about_to_reload();
                let mut egui_state_lock = egui_state.lock().unwrap();
                *egui_state_lock = None;

                drop(block_reload);
                observer.wait_for_reload();
                drop(egui_state_lock);
            });
        }

        Self {
            config: config.clone(),
            draw,
            egui_state,
            viz_renderer: VizRenderer::new(device, output_format, msaa_samples),
            viz_scenes: Vec::new(),
            output_format,
            msaa_samples,
            msaa_texture: CachedTexture::new(),
            depth_texture: CachedTexture::new(),
            fps_counter: FpsCounter::new(),
        }
    }

    pub fn window_event(&mut self, event: &WindowEvent<'_>) {
        if let Some(egui_state) = self.egui_state.lock().unwrap().as_mut() {
            let consumed = egui_state.window_event(event);
            if !consumed {
                // handle event
            }
        }
    }

    pub fn update(&mut self, window: &Window, device: &wgpu::Device) {
        // Recreate the egui state if it was dropped due to a hot reload.
        let mut guard = self.egui_state.lock().unwrap();
        let egui_state = guard.get_or_insert_with(|| {
            EguiState::new(window, device, self.output_format, self.msaa_samples)
        });

        egui_state.run(window, |ctx| {
            let env = WindowEnvImpl {
                config: &self.config,
                fps: self.fps_counter.fps(),
                mspf: self.fps_counter.mspf(),
                egui_ctx: ctx.clone(),
                viz_scenes: Mutex::new(Vec::new()),
            };

            (self.draw)(&env);

            self.viz_scenes = env.viz_scenes.into_inner().unwrap();
        });
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        output_size: [u32; 2],
        scale_factor: f32,
    ) {
        let msaa_output_view = if self.msaa_samples > 1 {
            let msaa_output_texture = self.msaa_texture.get(
                device,
                &wgpu::TextureDescriptor {
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
                },
            );
            Some(msaa_output_texture.create_view(&wgpu::TextureViewDescriptor::default()))
        } else {
            None
        };

        let depth_texture = self.depth_texture.get(
            device,
            &wgpu::TextureDescriptor {
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
            },
        );
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // The first render pass should use LoadOp::Clear and the rest should use
        // LoadOp::Load.
        let mut first_render_pass = true;
        let mut get_color_attachment = || {
            let (view, resolve_target, store) = match &msaa_output_view {
                Some(msaa_output_view) => (msaa_output_view, Some(output_view), true),
                None => (output_view, None, true),
            };

            let load = if first_render_pass {
                first_render_pass = false;
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: 27.0 / 255.0,
                    g: 27.0 / 255.0,
                    b: 27.0 / 255.0,
                    a: 1.0,
                })
            } else {
                wgpu::LoadOp::Load
            };

            wgpu::RenderPassColorAttachment {
                view,
                resolve_target,
                ops: wgpu::Operations { load, store },
            }
        };

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.render_viz(
            device,
            queue,
            &mut encoder,
            &mut get_color_attachment,
            &depth_texture_view,
            output_format,
            output_size,
            scale_factor,
        );

        self.render_egui(
            device,
            queue,
            &mut encoder,
            &mut get_color_attachment,
            output_size,
            scale_factor,
        );

        queue.submit([encoder.finish()]);
        self.fps_counter.end_frame();
    }

    fn render_egui<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        mut get_color_attachment: impl FnMut() -> wgpu::RenderPassColorAttachment<'a>,
        output_size: [u32; 2],
        scale_factor: f32,
    ) {
        if let Some(egui_state) = self.egui_state.lock().unwrap().as_mut() {
            egui_state.prepare(device, queue, encoder, output_size, scale_factor);

            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(get_color_attachment())],
                depth_stencil_attachment: None,
            });

            egui_state.render(&mut rp);
        }
    }

    fn render_viz<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        mut get_color_attachment: impl FnMut() -> wgpu::RenderPassColorAttachment<'a>,
        depth_texture_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        output_size: [u32; 2],
        scale_factor: f32,
    ) {
        for scene in &self.viz_scenes {
            self.viz_renderer.prepare(
                device,
                queue,
                output_format,
                output_size,
                scale_factor,
                scene,
            );

            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(get_color_attachment())],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_texture_view,
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

impl<D> fmt::Debug for Container<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame").finish_non_exhaustive()
    }
}
