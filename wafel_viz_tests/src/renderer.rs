use std::env;

use image::{Pixel, Rgb, RgbImage};
use wafel_api::Game;
use wafel_viz::{VizConfig, VizRenderer};

#[derive(Debug)]
pub struct Renderer {
    device_info: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sized: Option<SizedRenderer>,
}

impl Renderer {
    pub fn new() -> Self {
        pollster::block_on(Self::new_async())
    }

    pub fn device_info(&self) -> &str {
        &self.device_info
    }

    async fn new_async() -> Self {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("failed to request GPU adapter");
        let backend = format!("{:?}", adapter.get_info().backend).to_lowercase();

        let device_info = format!("{}_{}_{}", env::consts::OS, env::consts::ARCH, backend);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .expect("failed to request GPU device");

        Self {
            device_info,
            device,
            queue,
            sized: None,
        }
    }

    pub fn render(&mut self, game: &Game, config: &VizConfig) -> RgbImage {
        if self.sized.as_ref().map(|r| r.output_size) != Some(config.screen_size) {
            self.sized = None;
        }
        let sized_renderer = self
            .sized
            .get_or_insert_with(|| SizedRenderer::new(&self.device, config.screen_size));
        sized_renderer.render(&self.device, &self.queue, game, config)
    }
}

#[derive(Debug)]
struct SizedRenderer {
    viz_renderer: VizRenderer,
    output_size: [u32; 2],
    output_format: wgpu::TextureFormat,
    output_texture: wgpu::Texture,
    depth_texture: wgpu::Texture,
    padded_bytes_per_row: u32,
    output_buffer: wgpu::Buffer,
}

impl SizedRenderer {
    fn new(device: &wgpu::Device, output_size: [u32; 2]) -> Self {
        assert!(output_size[0] > 0 && output_size[1] > 0);

        let output_format = wgpu::TextureFormat::Rgba8Unorm;

        let renderer = VizRenderer::new(device, output_format);

        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: output_size[0],
                height: output_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: output_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        });

        let unpadded_bytes_per_row = 4 * output_size[0];
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padding = (align - (unpadded_bytes_per_row % align)) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (padded_bytes_per_row * output_size[1]) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
        });

        Self {
            viz_renderer: renderer,
            output_size,
            output_format,
            output_texture,
            depth_texture,
            padded_bytes_per_row,
            output_buffer,
        }
    }

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        game: &Game,
        config: &VizConfig,
    ) -> RgbImage {
        let render_data = game.render(config).expect("failed to render game");
        self.viz_renderer
            .prepare(device, queue, self.output_format, &render_data);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let output_texture_view = self
                .output_texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let depth_texture_view = self
                .depth_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            {
                let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &output_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_texture_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                });
                self.viz_renderer.render(&mut rp, self.output_size);
            }

            encoder.copy_texture_to_buffer(
                self.output_texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &self.output_buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(self.padded_bytes_per_row.try_into().unwrap()),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: self.output_size[0],
                    height: self.output_size[1],
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.submit([encoder.finish()]);

        let buffer_slice = self.output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        let image = {
            let buffer_view = buffer_slice.get_mapped_range();
            let mut image = RgbImage::new(self.output_size[0], self.output_size[1]);
            for y in 0..self.output_size[1] {
                for x in 0..self.output_size[0] {
                    let i = (y * self.padded_bytes_per_row + 4 * x) as usize;
                    let rgb = *Rgb::from_slice(&buffer_view[i..i + 3]);
                    image.put_pixel(x, y, rgb);
                }
            }
            image
        };

        self.output_buffer.unmap();
        image
    }
}
