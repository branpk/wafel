use wafel_api::VizRenderData;
use wafel_viz::VizRenderer;

use super::scene::Scene;

#[derive(Debug)]
pub struct VizContainer {
    renderer: VizRenderer,
}

impl VizContainer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        Self {
            renderer: VizRenderer::new(device, output_format),
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        output_format: wgpu::TextureFormat,
        scenes: &[VizRenderData],
    ) {
        // for scene in scenes {}
    }
}
