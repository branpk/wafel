use crate::window::WindowedApp;

#[derive(Debug)]
pub struct WafelApp {}

impl WindowedApp for WafelApp {
    fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        WafelApp {}
    }

    fn window_event(&mut self, event: &winit::event::WindowEvent<'_>) {}

    fn update(&mut self) {}

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        output_size: [u32; 2],
    ) {
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
        }

        queue.submit([encoder.finish()]);
    }
}
