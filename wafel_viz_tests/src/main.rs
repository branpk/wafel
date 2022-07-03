use std::fs;

use image::{Pixel, Rgba, RgbaImage};
use wafel_api::{load_m64, Game};
use wafel_viz::{prepare_render_data, N64Renderer};

#[derive(Debug)]
struct TestCase {
    name: &'static str,
    frame: u32,
}

fn u120_test_cases() -> Vec<TestCase> {
    let case = |name, frame| TestCase { name, frame };
    vec![
        case("power_on", 0),
        case("unnamed_12", 12),
        case("unnamed_52", 52),
        case("unnamed_120", 120),
        case("unnamed_140", 140),
        case("unnamed_279", 279),
        case("unnamed_975", 975),
        case("unnamed_1059", 1059),
        case("unnamed_1163", 1163),
        case("unnamed_1176", 1176),
        case("unnamed_1502", 1502),
        case("unnamed_1624", 1624),
        case("unnamed_1627", 1627),
        case("unnamed_1722", 1722),
        case("unnamed_1905", 1905),
        case("unnamed_2037", 2037),
        case("unnamed_2299", 2299),
        case("unnamed_2418", 2418),
        case("unnamed_2628", 2628),
        case("unnamed_2632", 2632),
        case("unnamed_3363", 3363),
        case("unnamed_3395", 3395),
        case("unnamed_3422", 3422),
        case("unnamed_3603", 3603),
        case("unnamed_4375", 4375),
        case("unnamed_4575", 4575),
        case("unnamed_5577", 5577),
        case("unnamed_6122", 6122),
        case("unnamed_6180", 6180),
        case("unnamed_9044", 9044),
        case("unnamed_9738", 9738),
        case("unnamed_10543", 10543),
        case("unnamed_10565", 10565),
        case("unnamed_10582", 10582),
        case("unnamed_10717", 10717),
        case("unnamed_10802", 10802),
        case("unnamed_10903", 10903),
        case("unnamed_10908", 10908),
        case("unnamed_11024", 11024),
        case("unnamed_11326", 11326),
        case("unnamed_11360", 11360),
        case("unnamed_11437", 11437),
        case("unnamed_11646", 11646),
        case("unnamed_11638", 11638),
        case("unnamed_11645", 11645),
        case("unnamed_11652", 11652),
        case("unnamed_11749", 11749),
        case("unnamed_11845", 11845),
        case("unnamed_12222", 12222),
        case("unnamed_13385", 13385),
        case("unnamed_13408", 13408),
        case("unnamed_13480", 13480),
        case("unnamed_13808", 13808),
        case("unnamed_15056", 15056),
        case("unnamed_15098", 15098),
        case("unnamed_15117", 15117),
        case("unnamed_15156", 15156),
        case("unnamed_15410", 15410),
        case("unnamed_15547", 15547),
        case("unnamed_15744", 15744),
        case("unnamed_15829", 15829),
        case("unnamed_15852", 15852),
        case("unnamed_16282", 16282),
        case("unnamed_17233", 17233),
        case("unnamed_22651", 22651),
        case("unnamed_22659", 22659),
        case("unnamed_22839", 22839),
        case("unnamed_22871", 22871),
        case("unnamed_22874", 22874),
        case("unnamed_23088", 23088),
        case("unnamed_23097", 23097),
        case("unnamed_24183", 24183),
        case("unnamed_24198", 24198),
        case("unnamed_24386", 24386),
        case("unnamed_25086", 25086),
        case("unnamed_25090", 25090),
        case("unnamed_25138", 25138),
        case("unnamed_25141", 25141),
        case("unnamed_26638", 26638),
        case("unnamed_32013", 32013),
        case("unnamed_32935", 32935),
        case("unnamed_34660", 34660),
        case("unnamed_34725", 34725),
        case("unnamed_35247", 35247),
        case("unnamed_36082", 36082),
        case("unnamed_38651", 38651),
        case("unnamed_38713", 38713),
        case("unnamed_41689", 41689),
        case("unnamed_44732", 44732),
        case("unnamed_45459", 45459),
        case("unnamed_45467", 45467),
        case("unnamed_45833", 45833),
        case("unnamed_45966", 45966),
        case("unnamed_52033", 52033),
        case("unnamed_52884", 52884),
        case("unnamed_52943", 52943),
        case("unnamed_52968", 52968),
        case("unnamed_53197", 53197),
        case("unnamed_53496", 53496),
        case("unnamed_54664", 54664),
        case("unnamed_55577", 55577),
        case("unnamed_55642", 55642),
        case("unnamed_55945", 55945),
        case("unnamed_56273", 56273),
        case("unnamed_60881", 60881),
        case("unnamed_61098", 61098),
        case("unnamed_61473", 61473),
        case("unnamed_63451", 63451),
        case("unnamed_64590", 64590),
        case("unnamed_66344", 66344),
        case("unnamed_68798", 68798),
        case("unnamed_69260", 69260),
        case("unnamed_71009", 71009),
        case("unnamed_71323", 71323),
        case("unnamed_72554", 72554),
        case("unnamed_81881", 81881),
        case("unnamed_81960", 81960),
        case("unnamed_82906", 82906),
        case("unnamed_85194", 85194),
        case("unnamed_85800", 85800),
        case("unnamed_85282", 85282),
        case("unnamed_90671", 90671),
        case("unnamed_92354", 92354),
        case("unnamed_99903", 99903),
        case("unnamed_100449", 100449),
        case("unnamed_100758", 100758),
        case("unnamed_102446", 102446),
        case("unnamed_103259", 103259),
        case("unnamed_104109", 104109),
        case("unnamed_105905", 105905),
        case("unnamed_109556", 109556),
        case("unnamed_109612", 109612),
        case("unnamed_110107", 110107),
        case("unnamed_110127", 110127),
        case("unnamed_110130", 110130),
        case("unnamed_110147", 110147),
        case("unnamed_114943", 114943),
        case("unnamed_114955", 114955),
        case("unnamed_117872", 117872),
        case("unnamed_122216", 122216),
        case("unnamed_122942", 122942),
        case("unnamed_123263", 123263),
        case("unnamed_123589", 123589),
        case("unnamed_125576", 125576),
        case("unnamed_126160", 126160),
        case("unnamed_128143", 128143),
        case("unnamed_129859", 129859),
        case("unnamed_130266", 130266),
        case("unnamed_130373", 130373),
        case("unnamed_134235", 134235),
        case("unnamed_137580", 137580),
        case("unnamed_138322", 138322),
        case("unnamed_138984", 138984),
        case("unnamed_139601", 139601),
        case("unnamed_139721", 139721),
        case("unnamed_141811", 141811),
        case("unnamed_141930", 141930),
        case("unnamed_143441", 143441),
        case("unnamed_143448", 143448),
        case("unnamed_144714", 144714),
        case("unnamed_145414", 145414),
        case("unnamed_145473", 145473),
        case("unnamed_147682", 147682),
        case("unnamed_148424", 148424),
        case("unnamed_148484", 148484),
        case("unnamed_148573", 148573),
        case("unnamed_149182", 149182),
        case("unnamed_149706", 149706),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let _ = fs::remove_dir_all("wafel_viz_tests/actual");
    fs::create_dir_all("wafel_viz_tests/actual")?;

    let mut renderer = futures::executor::block_on(Renderer::new((320, 240)));

    // FIXME: Update to libsm64/sm64_us.dll
    // TODO: Other game versions
    let mut game = unsafe { Game::new("../libsm64-build/build/us_lib/sm64_us.dll") };
    let (_, inputs) = load_m64("wafel_viz_tests/input/120_u.m64");

    let mut test_cases = u120_test_cases();
    test_cases.sort_by_key(|case| case.frame);

    for (i, case) in test_cases.iter().enumerate() {
        eprintln!("[{:3}/{}] {}", i + 1, test_cases.len(), case.name);

        while game.frame() < case.frame {
            let input = inputs
                .get(game.frame() as usize)
                .copied()
                .unwrap_or_default();
            game.set_input(input);
            game.advance();
        }

        let image = renderer.render(&game);

        image.save(format!("wafel_viz_tests/actual/u120_{}.png", case.name))?;
    }

    Ok(())
}

#[derive(Debug)]
struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: N64Renderer,
    output_size: (u32, u32),
    output_format: wgpu::TextureFormat,
    output_texture: wgpu::Texture,
    depth_texture: wgpu::Texture,
    padded_bytes_per_row: u32,
    output_buffer: wgpu::Buffer,
}

impl Renderer {
    async fn new(output_size: (u32, u32)) -> Self {
        assert!(output_size.0 > 0 && output_size.1 > 0);

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("failed to request GPU adapter");

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

        let output_format = wgpu::TextureFormat::Rgba8Unorm;

        let renderer = N64Renderer::new(&device);

        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: output_size.0,
                height: output_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: output_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        });

        let unpadded_bytes_per_row = 4 * output_size.0;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padding = (align - (unpadded_bytes_per_row % align)) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (padded_bytes_per_row * output_size.1) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
        });

        Self {
            device,
            queue,
            renderer,
            output_size,
            output_format,
            output_texture,
            depth_texture,
            padded_bytes_per_row,
            output_buffer,
        }
    }

    fn render(&mut self, game: &Game) -> RgbaImage {
        let render_data = prepare_render_data(game, self.output_size);
        self.renderer
            .prepare(&self.device, &self.queue, self.output_format, &render_data);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
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
                self.renderer.render(&mut rp, self.output_size);
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
                    width: self.output_size.0,
                    height: self.output_size.1,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.queue.submit([encoder.finish()]);

        let buffer_slice = self.output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let image = {
            let buffer_view = buffer_slice.get_mapped_range();
            let mut image = RgbaImage::new(self.output_size.0, self.output_size.1);
            for y in 0..self.output_size.1 {
                for x in 0..self.output_size.0 {
                    let i = (y * self.padded_bytes_per_row + 4 * x) as usize;
                    let rgba = *Rgba::from_slice(&buffer_view[i..i + 4]);
                    image.put_pixel(x, y, rgba);
                }
            }
            image
        };

        self.output_buffer.unmap();
        image
    }
}
