use fast3d::render::F3DRenderer;

use crate::VizRenderData;

#[derive(Debug)]
pub struct VizRenderer {
    f3d_renderer: F3DRenderer,
}

impl VizRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            f3d_renderer: F3DRenderer::new(device),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        data: &VizRenderData,
    ) {
        self.f3d_renderer
            .prepare(device, queue, output_format, &data.f3d_render_data);
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>, output_size: [u32; 2]) {
        self.f3d_renderer.render(rp, output_size);
    }
}
