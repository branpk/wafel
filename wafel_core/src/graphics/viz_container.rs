use wafel_api::VizRenderData;
use wafel_viz::VizRenderer;

/// Wrapper for VizRenderer that renders multiple scenes.
#[derive(Debug)]
pub struct VizContainer {
    renderer: VizRenderer,
}

impl VizContainer {
    /// Create a VizContainer.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        Self {
            renderer: VizRenderer::new(device, output_format),
        }
    }

    /// Render the given viz scenes.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        output_format: wgpu::TextureFormat,
        scenes: &[VizRenderData],
    ) {
        for scene in scenes {
            self.render_scene(
                device,
                queue,
                output_view,
                output_size,
                output_format,
                scene,
            );
        }
    }

    fn render_scene(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_size: (u32, u32),
        output_format: wgpu::TextureFormat,
        render_data: &VizRenderData,
    ) {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: output_size.0,
                height: output_size.1,
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

        self.renderer
            .prepare(device, queue, output_format, render_data);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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

            self.renderer.render(&mut rp);
        }

        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
}
