use bytemuck::cast_slice;
use fast3d::{
    cmd::{
        ComponentSize, F3DCommand, Image, ImageFormat, MatrixMode, MatrixOp, TextureBlock,
        TileIndex, TileParams,
    },
    util::{atan2s, Matrixf, Vertex},
};
use wafel_data_access::MemoryLayout;
use wafel_data_type::{Address, Angle};
use wafel_memory::MemoryRead;
use wafel_sm64::gfx::GraphNodeBackground;

use crate::{
    f3d_builder::{F3DBuilder, Pointer, Segmented},
    LookAtCamera, VizError,
};

#[allow(dead_code)]
pub fn skybox_main<M: MemoryRead>(
    builder: &mut F3DBuilder<'_, M>,
    layout: &impl MemoryLayout,
    memory: &M,
    node: &GraphNodeBackground,
    lakitu_state: &LookAtCamera,
) -> Result<Pointer, VizError> {
    create_skybox_facing_camera(builder, layout, memory, node.background as i8, lakitu_state)
}

#[derive(Debug, Clone)]
struct Skybox {
    scaled_x: i32,
    scaled_y: i32,
    upper_left_tile: i32,
}

const SKYBOX_COLORS: [[u8; 3]; 2] = [[0x50, 0x64, 0x5A], [0xFF, 0xFF, 0xFF]];

const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 240;
const SKYBOX_WIDTH: i32 = 4 * SCREEN_WIDTH;
const SKYBOX_HEIGHT: i32 = 4 * SCREEN_HEIGHT;
const SKYBOX_TILE_WIDTH: i32 = SCREEN_WIDTH / 2;
const SKYBOX_TILE_HEIGHT: i32 = SCREEN_HEIGHT / 2;
const SKYBOX_COLS: i32 = 10;

fn round_float(num: f32) -> i16 {
    if num >= 0.0 {
        (num + 0.5) as i16
    } else {
        (num - 0.5) as i16
    }
}

fn read_skybox_texture(layout: &impl MemoryLayout, background: i8) -> Result<Address, VizError> {
    let symbol = [
        "water_skybox_ptrlist",
        "bitfs_skybox_ptrlist",
        "wdw_skybox_ptrlist",
        "cloud_floor_skybox_ptrlist",
        "ccm_skybox_ptrlist",
        "ssl_skybox_ptrlist",
        "bbh_skybox_ptrlist",
        "bidw_skybox_ptrlist",
        "clouds_skybox_ptrlist",
        "bits_skybox_ptrlist",
    ][background as usize];
    Ok(layout.symbol_address(symbol)?)
}

fn calculate_skybox_scaled_x(yaw: Angle, fov: f32) -> i32 {
    let yaw = yaw.0 as u16 as f32;
    let yaw_scaled = (SCREEN_WIDTH as f64 * 360.0 * yaw as f64 / (fov as f64 * 65536.0)) as f32;
    let mut scaled_x = (yaw_scaled + 0.5) as i32;

    if scaled_x > SKYBOX_WIDTH {
        scaled_x -= scaled_x / SKYBOX_WIDTH * SKYBOX_WIDTH;
    }
    SKYBOX_WIDTH - scaled_x
}

fn calculate_skybox_scaled_y(pitch: Angle) -> i32 {
    let pitch_in_degrees = pitch.0 as f32 * 360.0 / 65535.0;

    let degrees_to_scale = 360.0 * pitch_in_degrees / 90.0;
    let rounded_y = round_float(degrees_to_scale) as i32;

    let mut scaled_y = rounded_y + 5 * SKYBOX_TILE_HEIGHT;

    if scaled_y > SKYBOX_HEIGHT {
        scaled_y = SKYBOX_HEIGHT;
    }
    if scaled_y < SCREEN_HEIGHT {
        scaled_y = SCREEN_HEIGHT;
    }
    scaled_y
}

fn get_top_left_tile_idx(scaled_x: i32, scaled_y: i32) -> i32 {
    let tile_col = scaled_x / SKYBOX_TILE_WIDTH;
    let tile_row = (SKYBOX_HEIGHT - scaled_y) / SKYBOX_TILE_HEIGHT;

    tile_row * SKYBOX_COLS + tile_col
}

fn make_skybox_rect<M: MemoryRead>(
    builder: &mut F3DBuilder<'_, M>,
    tile_index: i32,
    color_index: i8,
) -> Pointer {
    let x = (tile_index % SKYBOX_COLS * SKYBOX_TILE_WIDTH) as i16;
    let y = (SKYBOX_HEIGHT - tile_index / SKYBOX_COLS * SKYBOX_TILE_HEIGHT) as i16;

    let color_index = color_index as usize;
    let color = [
        SKYBOX_COLORS[color_index][0],
        SKYBOX_COLORS[color_index][1],
        SKYBOX_COLORS[color_index][2],
        255,
    ];

    builder.alloc_vertices(&[
        Vertex {
            pos: [x, y, -1],
            uv: [0, 0],
            cn: color,
        },
        Vertex {
            pos: [x, y - SKYBOX_TILE_HEIGHT as i16, -1],
            uv: [0, 31 << 5],
            cn: color,
        },
        Vertex {
            pos: [
                x + SKYBOX_TILE_WIDTH as i16,
                y - SKYBOX_TILE_HEIGHT as i16,
                -1,
            ],
            uv: [31 << 5, 31 << 5],
            cn: color,
        },
        Vertex {
            pos: [x + SKYBOX_TILE_WIDTH as i16, y, -1],
            uv: [31 << 5, 0],
            cn: color,
        },
    ])
}

fn calc_dxt(width: u32, b_txl: u32) -> u32 {
    let words = u32::max(1, width * b_txl / 8);
    ((1 << 11) + words - 1) / words
}

fn load_block_texture(
    dl: &mut Vec<F3DCommand<Pointer>>,
    width: u32,
    height: u32,
    format: ImageFormat,
    image: Pointer,
) {
    use F3DCommand::*;

    dl.push(DPSetTextureImage(Image {
        fmt: format,
        size: ComponentSize::Bits16,
        width: 1,
        img: image,
    }));
    dl.push(DPTileSync);
    dl.push(DPSetTile(
        TileIndex::LOAD,
        TileParams {
            fmt: format,
            size: ComponentSize::Bits16,
            ..Default::default()
        },
    ));
    dl.push(DPLoadSync);
    dl.push(DPLoadBlock(
        TileIndex::LOAD,
        TextureBlock {
            uls: 0,
            ult: 0,
            lrs: width * height - 1,
            dxt: calc_dxt(width, 2),
        },
    ));
}

fn draw_skybox_tile_grid<M: MemoryRead>(
    builder: &mut F3DBuilder<'_, M>,
    dl: &mut Vec<F3DCommand<Pointer>>,
    layout: &impl MemoryLayout,
    memory: &M,
    background: i8,
    skybox: &Skybox,
    color_index: i8,
) -> Result<(), VizError> {
    use F3DCommand::*;

    for row in 0..3 {
        for col in 0..3 {
            let tile_index = skybox.upper_left_tile + row * SKYBOX_COLS + col;
            let texture_list =
                builder.seg_to_virt(Segmented(read_skybox_texture(layout, background)?));
            let texture = Segmented(
                memory.read_addr(texture_list + tile_index as usize * layout.pointer_size())?,
            );
            let vertices = make_skybox_rect(builder, tile_index, color_index);

            load_block_texture(dl, 32, 32, ImageFormat::Rgba, Pointer::Segmented(texture));
            dl.push(SPVertex {
                v: vertices,
                n: 4,
                v0: 0,
            });
            dl.push(SPDisplayList(Pointer::Segmented(Segmented(
                layout.symbol_address("dl_draw_quad_verts_0123")?,
            ))));
        }
    }
    Ok(())
}

fn create_skybox_ortho_matrix(skybox: &Skybox) -> Matrixf {
    let left = skybox.scaled_x as f32;
    let right = (skybox.scaled_x + SCREEN_WIDTH) as f32;
    let bottom = (skybox.scaled_y - SCREEN_HEIGHT) as f32;
    let top = skybox.scaled_y as f32;

    Matrixf::ortho(left, right, bottom, top, 0.0, 3.0, 1.0)
}

fn init_skybox_display_list<M: MemoryRead>(
    builder: &mut F3DBuilder<'_, M>,
    layout: &impl MemoryLayout,
    memory: &M,
    skybox: &Skybox,
    background: i8,
    color_index: i8,
) -> Result<Pointer, VizError> {
    use F3DCommand::*;

    let mut dl = Vec::new();

    dl.push(SPDisplayList(Pointer::Segmented(Segmented(
        layout.symbol_address("dl_skybox_begin")?,
    ))));

    let ortho = create_skybox_ortho_matrix(skybox).to_fixed();
    let matrix = builder.alloc_u32(cast_slice(&ortho));
    dl.push(SPMatrix {
        matrix,
        mode: MatrixMode::Proj,
        op: MatrixOp::Mul,
        push: false,
    });

    dl.push(SPDisplayList(Pointer::Segmented(Segmented(
        layout.symbol_address("dl_skybox_tile_tex_settings")?,
    ))));

    draw_skybox_tile_grid(
        builder,
        &mut dl,
        layout,
        memory,
        background,
        skybox,
        color_index,
    )?;

    dl.push(SPDisplayList(Pointer::Segmented(Segmented(
        layout.symbol_address("dl_skybox_end")?,
    ))));
    dl.push(SPEndDisplayList);

    Ok(builder.alloc_dl(&dl))
}

fn create_skybox_facing_camera<M: MemoryRead>(
    builder: &mut F3DBuilder<'_, M>,
    layout: &impl MemoryLayout,
    memory: &M,
    background: i8,
    lakitu_state: &LookAtCamera,
) -> Result<Pointer, VizError> {
    let LookAtCamera { pos, focus, .. } = lakitu_state;

    let camera_face_x = focus[0] - pos[0];
    let camera_face_y = focus[1] - pos[1];
    let camera_face_z = focus[2] - pos[2];
    let mut color_index = 1;

    if background == 8 {
        let file_num = layout
            .global_path("gCurrSaveFileNum")?
            .read(memory)?
            .try_as_int()?;
        let jrb_course = 3;
        let jrb_stars = layout
            .global_path(&format!(
                "gSaveBuffer.files[{}][0].courseStars[{}]",
                file_num - 1,
                jrb_course - 1,
            ))?
            .read(memory)?
            .try_as_int()?;

        if (jrb_stars & (1 << 0)) == 0 {
            color_index = 0;
        }
    }

    let fov = 90.0;
    let yaw = atan2s(camera_face_z, camera_face_x);
    let pitch = atan2s(
        (camera_face_x * camera_face_x + camera_face_z * camera_face_z).sqrt(),
        camera_face_y,
    );
    let scaled_x = calculate_skybox_scaled_x(yaw, fov);
    let scaled_y = calculate_skybox_scaled_y(pitch);
    let upper_left_tile = get_top_left_tile_idx(scaled_x, scaled_y);

    let skybox = Skybox {
        scaled_x,
        scaled_y,
        upper_left_tile,
    };

    init_skybox_display_list(builder, layout, memory, &skybox, background, color_index)
}
