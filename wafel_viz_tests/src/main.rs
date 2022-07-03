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
    // TODO:
    // 105905 has extra snow particle on mupen
    let case = |name, frame| TestCase { name, frame };
    vec![
        case("u120_000000_power_on", 0),
        case("u120_000052_logo", 52),
        case("u120_000120_mario_head", 120),
        case("u120_000140_file_select", 140),
        case("u120_000279_peach_letter", 279),
        case("u120_000975_castle", 975),
        case("u120_001059_water_shadow", 1059),
        case("u120_001176_tree_shadows", 1176),
        case("u120_001502_mario", 1502),
        case("u120_001624_tilted_text", 1624),
        case("u120_001627_scrolling_text", 1627),
        case("u120_001722_dust", 1722),
        case("u120_001905_key_door", 1905),
        case("u120_002037_pause", 2037),
        case("u120_002299_wkww", 2299),
        case("u120_002418_star_grab", 2418),
        case("u120_002628_continue1", 2628),
        case("u120_002632_continue2", 2632),
        case("u120_003363_exit_slide", 3363),
        case("u120_003422_slide_star", 3422),
        case("u120_003603_slide_star_grab", 3603), // TODO: Mupen shows an extra snow particle
        case("u120_004375_snow_run", 4375),
        case("u120_004575_star_fade", 4575),
        case("u120_005577_slide_ice", 5577),
        case("u120_006122_dust_butt", 6122),
        case("u120_006180_star_head", 6180),
        case("u120_009044_star_select", 9044),
        case("u120_009738_ice", 9738),
        case("u120_010543_caught_in_the_undertoad", 10543),
        case("u120_010565_pss_room", 10565),
        case("u120_010717_pss_fog", 10717),
        case("u120_010802_pss_cull", 10802),
        case("u120_010903_pss_wall", 10903),
        case("u120_010908_pss_fog2", 10908),
        case("u120_011024_pss_fog3", 11024),
        case("u120_011326_mario_mario", 11326),
        case("u120_011360_lobby_decal", 11360),
        case("u120_011638_lod_peach1", 11638),
        case("u120_011645_lod_peach2", 11645),
        case("u120_011652_lod_peach3", 11652),
        case("u120_011749_transparent_box", 11749),
        case("u120_011845_flames", 11845),
        case("u120_012222_amps", 12222),
        case("u120_013385_bowser_key_1", 13385),
        case("u120_013408_bowser_key_2", 13408),
        case("u120_013480_bowser_key_3", 13480),
        case("u120_013808_key_cutscene", 13808),
        case("u120_015056_totwc_light", 15056),
        case("u120_015098_totwc_light_2", 15098),
        case("u120_015117_totwc_light_3", 15117),
        case("u120_015156_totwc_1", 15156),
        case("u120_015410_totwc_2", 15410),
        case("u120_015547_totwc_3_blocky_clouds", 15547),
        case("u120_015744_totwc_4", 15744),
        case("u120_015829_totwc_5", 15829),
        case("u120_015852_totwc_6", 15852),
        case("u120_016282_wf_entry", 16282),
        case("u120_017233_cage_cull", 17233),
        case("u120_022651_box_squish", 22651),
        case("u120_022659_cap_grab", 22659),
        case("u120_022839_winged_goomba", 22839),
        case("u120_022871_tree_hold_1", 22871),
        case("u120_022874_tree_hold_2", 22874),
        case("u120_023097_bob_fog", 23097),
        case("u120_024183_bob_fog_2", 24183),
        case("u120_024386_bob_oob", 24386),
        case("u120_025086_fading_shadow", 25086),
        case("u120_025090_tilted_shadow", 25090),
        case("u120_025141_limbo", 25141),
        case("u120_026638_box_kick", 26638),
        case("u120_032013_whirlpool", 32013),
        case("u120_032935_bowser_door", 32935),
        case("u120_034660_ddd_entrance", 34660),
        case("u120_034725_ddd_entrance_2", 34725),
        case("u120_035247_inside_chest", 35247),
        case("u120_036082_ddd_entry", 36082),
        case("u120_038713_ddd_rings", 38713),
        case("u120_041689_lava", 41689),
        case("u120_044732_mips_shadow", 44732),
        case("u120_045459_hmc_entrance_1", 45459),
        case("u120_045467_hmc_entrance_2", 45467),
        case("u120_045833_hmc_fog", 45833),
        case("u120_045966_hmc_limbo", 45966),
        case("u120_052884_dorrie_1", 52884),
        case("u120_052943_dorrie_2", 52943),
        case("u120_052968_dorrie_3", 52968),
        case("u120_053197_metal_cap", 53197),
        case("u120_053496_metal_cap_2", 53496),
        case("u120_054664_hmc_star_grab", 54664),
        case("u120_055577_pokey_face", 55577),
        case("u120_055642_quicksand", 55642),
        case("u120_055945_ssl_flame", 55945),
        case("u120_056273_pyramid_fog", 56273),
        case("u120_060881_star_grab_in_sand", 60881), // TODO: Mupen has black bars on top/bottom, which is noticeable here
        case("u120_061098_ssl_ripple", 61098),
        case("u120_061473_bomb_clip", 61473),
        case("u120_063451_lll_ripple", 63451),
        case("u120_064590_smoke_texture", 64590),
        case("u120_066344_lll_puzzle", 66344),
        case("u120_068798_lava_shell", 68798),
        case("u120_069260_lavafall", 69260), // TODO: Texture is different than on mupen
        case("u120_071009_vanish", 71009),   // TODO: Dithering not implemented
        case("u120_071323_vanish_2", 71323),
        case("u120_072554_jrb_fog_and_metal", 72554),
        case("u120_081881_bbh_entry", 81881),
        case("u120_081960_bbh_entry_2", 81960),
        case("u120_082906_killing_a_ghost", 82906),
        case("u120_085194_vc_box_top", 85194),
        case("u120_085282_bbh_door", 85282),
        case("u120_085800_bbh_star_grab", 85800),
        case("u120_090671_bbh_window", 90671),
        case("u120_092354_koopa_underwear", 92354),
        case("u120_099903_wiggler_1", 99903),
        case("u120_100449_wiggler_2", 100449),
        case("u120_100758_mirror_room", 100758),
        case("u120_102446_ice_bully", 102446),
        case("u120_103259_snowmans_head", 103259),
        case("u120_104109_igloo_star", 104109), // TODO: Extra snow particle on mupen
        case("u120_105905_moneybags", 105905),
        case("u120_109556_ttc_slide_ripples", 109556),
        case("u120_109612_ttc_slide_fade", 109612),
        case("u120_110107_slide_trap", 110107),
        case("u120_110127_slide_smile", 110127),
        case("u120_110130_slide_smile_gone", 110130),
        case("u120_110147_slide_smile_back", 110147),
        case("u120_114943_wdw_water_shadow_1", 114943),
        case("u120_114955_wdw_water_shadow_2", 114955),
        case("u120_117872_crystal_tap", 117872),
        case("u120_122216_cloud_entry", 122216),
        case("u120_122942_cannon_shot_1", 122942),
        case("u120_123263_cannon_shot_2", 123263),
        case("u120_123589_rain_cloud", 123589),
        case("u120_125576_ttc_fog", 125576),
        case("u120_126160_ttc_fog_2", 126160),
        case("u120_128143_clock", 128143),
        case("u120_129859_toad_star", 129859),
        case("u120_130266_rr_rainbow", 130266),
        case("u120_130373_rr_blue_flame", 130373),
        case("u120_134235_cannon_shot_3", 134235),
        case("u120_137580_staircase_fog", 137580),
        case("u120_138322_bits_seam", 138322),
        case("u120_138984_bits_blj", 138984),
        case("u120_139601_bowser3", 139601),
        case("u120_139721_boomer3", 139721),
        case("u120_141811_peach_cutscene_1", 141811),
        case("u120_141930_peach_cutscene_2", 141930),
        case("u120_144714_credits_wf", 144714),
        case("u120_145414_credits_bbh_1", 145414),
        case("u120_145473_credits_bbh_2", 145473),
        case("u120_147682_credits_ttc", 147682),
        case("u120_148424_credits_cotmc", 148424),
        case("u120_148484_credits_ddd_1", 148484),
        case("u120_148573_credits_ddd_2", 148573),
        case("u120_149182_credits_zoom", 149182),
        case("u120_149706_thank_you", 149706),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let _ = fs::remove_dir_all("wafel_viz_tests/output");
    fs::create_dir_all("wafel_viz_tests/output/all")?;
    fs::create_dir_all("wafel_viz_tests/output/mismatches")?;

    let mut renderer = futures::executor::block_on(Renderer::new((320, 240)));

    // FIXME: Update to libsm64/sm64_us.dll
    // TODO: Other game versions
    let mut game = unsafe { Game::new("../libsm64-build/build/us_lib/sm64_us.dll") };
    let (_, inputs) = load_m64("wafel_viz_tests/input/120_u.m64");

    let mut test_cases = u120_test_cases();
    test_cases.sort_by_key(|case| case.frame);

    let mut mismatches = Vec::new();

    for (i, case) in test_cases.iter().enumerate() {
        while game.frame() < case.frame {
            let input = inputs
                .get(game.frame() as usize)
                .copied()
                .unwrap_or_default();
            game.set_input(input);
            game.advance();
        }

        let actual = renderer.render(&mut game);

        let expected = image::open(format!(
            "wafel_viz_tests/{}/{}.png",
            renderer.device_info, case.name
        ))
        .ok()
        .map(|img| img.to_rgba8());

        actual.save(format!("wafel_viz_tests/output/all/{}.png", case.name))?;

        let matches = Some(&actual) == expected.as_ref();
        if !matches {
            actual.save(format!(
                "wafel_viz_tests/output/mismatches/{}.png",
                case.name
            ))?;
            mismatches.push(case.name);
        };

        eprintln!(
            "[{:3}/{}] \x1b[{}m{}\x1b[0m",
            i + 1,
            test_cases.len(),
            if matches { 32 } else { 31 },
            case.name
        );
    }

    eprintln!();
    if mismatches.is_empty() {
        eprintln!("\x1b[32mAll cases match!\x1b[0m");
    } else {
        eprintln!("\x1b[31m{} mismatches:\x1b[0m", mismatches.len());
        for name in mismatches {
            eprintln!("  {}", name);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Renderer {
    device_info: String,
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
        let backend = format!("{:?}", adapter.get_info().backend).to_lowercase();
        let device_info = format!("win_x64_{}", backend);

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
            device_info,
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

    fn render(&mut self, game: &mut Game) -> RgbaImage {
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
