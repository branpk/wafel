use std::fmt;

use egui::{ClippedPrimitive, Context, TexturesDelta};
use egui_wgpu::{renderer::ScreenDescriptor, Renderer};
use egui_winit::State;
use winit::{event::WindowEvent, window::Window};

pub struct EguiState {
    context: Context,
    state: State,
    renderer: Renderer,
    primitives: Vec<ClippedPrimitive>,
    textures_delta: TexturesDelta,
    screen_descriptor: Option<ScreenDescriptor>,
}

impl EguiState {
    pub fn new(window: &Window, device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let mut state = State::new(window);
        state.set_pixels_per_point(window.scale_factor() as f32);
        EguiState {
            context: Context::default(),
            state,
            renderer: Renderer::new(device, output_format, None, 1),
            primitives: Vec::new(),
            textures_delta: TexturesDelta::default(),
            screen_descriptor: None,
        }
    }

    pub fn window_event(&mut self, event: &WindowEvent<'_>) -> bool {
        let response = self.state.on_event(&self.context, event);
        if response.repaint {
            self.context.request_repaint();
        }
        response.consumed
    }

    pub fn run(&mut self, window: &Window, run_ui: impl FnOnce(&Context)) {
        let raw_input = self.state.take_egui_input(window);
        let egui_output = self.context.run(raw_input, |ctx| {
            let mut style = (*ctx.style()).clone();
            style.visuals.panel_fill = egui::Color32::TRANSPARENT;
            ctx.set_style(style);
            run_ui(ctx);
        });
        self.state
            .handle_platform_output(window, &self.context, egui_output.platform_output);
        self.primitives = self.context.tessellate(egui_output.shapes);
        self.textures_delta = egui_output.textures_delta;
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output_size: [u32; 2],
        scale_factor: f32,
    ) {
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: output_size,
            pixels_per_point: scale_factor,
        };

        for (id, image_delta) in &self.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &self.primitives, &screen_descriptor);

        self.screen_descriptor = Some(screen_descriptor);
    }

    pub fn render<'rp>(&'rp mut self, rp: &mut wgpu::RenderPass<'rp>) {
        let screen_descriptor = self
            .screen_descriptor
            .as_ref()
            .expect("missing call to EguiState::prepare");

        self.renderer
            .render(rp, &self.primitives, screen_descriptor);
    }
}

impl fmt::Debug for EguiState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EguiState").finish_non_exhaustive()
    }
}
