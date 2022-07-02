use core::fmt;
use std::{collections::HashMap, mem, ops};

use bytemuck::cast_slice_mut;
use derivative::Derivative;
use ordered_float::NotNan;

use crate::{
    f3d_decode::*,
    render_api::{
        decode_shader_id, encode_shader_id, CCFeatures, RenderBackend, ShaderId, ShaderItem,
    },
};

pub trait F3DSource {
    type Ptr: fmt::Debug + Copy;
    type DlIter: Iterator<Item = F3DCommand<Self::Ptr>>;

    fn root_dl(&self) -> Self::DlIter;
    fn read_dl(&self, ptr: Self::Ptr) -> Self::DlIter;

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize);
    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize);
    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize);
}

pub fn interpret_f3d_display_list(source: &impl F3DSource, backend: &mut impl RenderBackend) {
    let mut state = State::default();
    state.interpret(source, backend, source.root_dl(), 0);
    state.flush(backend);
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
struct State<Ptr> {
    viewport: Viewport,
    scissor: (ScissorMode, Rectangle<NotNan<f32>>),
    model_view: MatrixState,
    proj: MatrixState,

    alpha_dither: AlphaDither,
    color_dither: ColorDither,
    combine_key: bool,
    texture_convert: TextureConvert,
    texture_filter: TextureFilter,
    texture_lut: TextureLUT,
    texture_lod: bool,
    texture_detail: TextureDetail,
    texture_persp: bool,
    cycle_type: CycleType,
    pipeline_mode: PipelineMode,
    alpha_compare: AlphaCompare,
    depth_source: DepthSource,
    render_mode: RenderMode,
    persp_normalize: u16,

    combine_mode: CombineMode,
    env_color: Rgba32,
    prim_color: Rgba32,
    blend_color: Rgba32,
    fog_color: Rgba32,

    geometry_mode: GeometryModes,

    texture_image: Option<Image<Ptr>>,
    texture_scale: [[f32; 2]; 8],
    tile_params: [TileParams; 8],
    tile_size: [TileSize; 8],
    tmem_to_texture_id: HashMap<u32, u32>,

    vertices: Vec<Vertex>,
    vertex_buffer: Vec<f32>,
    vertex_buffer_num_tris: usize,
}

#[derive(Debug)]
struct MatrixState {
    stack: Vec<Matrixf>,
    cur: Matrixf,
}

impl Default for MatrixState {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            cur: Matrixf::identity(),
        }
    }
}

impl MatrixState {
    fn execute(&mut self, m: Matrixf, op: MatrixOp, push: bool) {
        if push {
            self.stack.push(self.cur.clone());
        }
        match op {
            MatrixOp::Load => self.cur = m,
            MatrixOp::Mul => self.cur = &self.cur * &m,
        }
    }

    fn pop(&mut self) {
        self.cur = self.stack.pop().expect("popMatrix without push");
    }
}

impl<Ptr: fmt::Debug + Copy> State<Ptr> {
    fn vertex(&self, index: u32) -> Vertex {
        *self
            .vertices
            .get(index as usize)
            .expect("invalid vertex index")
    }

    fn flush(&mut self, backend: &mut impl RenderBackend) {
        if self.vertex_buffer_num_tris > 0 {
            let shader_id = self.get_shader_id();
            backend.load_shader(ShaderId(shader_id as usize));
            backend.draw_triangles(&self.vertex_buffer, self.vertex_buffer_num_tris);
            self.vertex_buffer.clear();
            self.vertex_buffer_num_tris = 0;
            // std::process::exit(0);
        }
    }

    fn set_geometry_mode(&mut self, backend: &mut impl RenderBackend, mode: GeometryModes) {
        self.flush(backend);
        self.geometry_mode = mode;
        backend.set_depth_test(mode.contains(GeometryModes::ZBUFFER));
    }

    fn get_shader_id(&self) -> u32 {
        let rm = &self.render_mode;
        let cm = &self.combine_mode;

        let mut cc_features = CCFeatures {
            opt_alpha: rm.blend_cycle1.alpha2 != BlendAlpha2::Memory
                || rm.flags.contains(RenderModeFlags::CVG_X_ALPHA),
            opt_fog: rm.blend_cycle1.color1 == BlendColor::Fog,
            opt_texture_edge: rm.flags.contains(RenderModeFlags::CVG_X_ALPHA),
            opt_noise: self.alpha_compare == AlphaCompare::Dither,
            ..Default::default()
        };

        for (i, mode) in [cm.color1, cm.alpha1].into_iter().enumerate() {
            let mut num_inputs = 0;
            for (j, cc) in mode.args.into_iter().enumerate() {
                let item = match cc {
                    ColorCombineComponent::Texel0 => ShaderItem::Texel0,
                    ColorCombineComponent::Texel1 => ShaderItem::Texel1,
                    ColorCombineComponent::Texel0Alpha => ShaderItem::Texel0A,
                    ColorCombineComponent::Prim
                    | ColorCombineComponent::Shade
                    | ColorCombineComponent::Env
                    | ColorCombineComponent::LodFraction => {
                        num_inputs += 1;
                        ShaderItem::from_index(num_inputs)
                    }
                    _ => ShaderItem::Zero,
                };
                cc_features.c[i][j] = item;
            }
        }

        encode_shader_id(cc_features)
    }

    fn interpret<S: F3DSource<Ptr = Ptr>>(
        &mut self,
        source: &S,
        backend: &mut impl RenderBackend,
        dl: S::DlIter,
        indent: usize,
    ) {
        let indent_str = "  ".repeat(indent);
        for cmd in dl {
            // if !matches!(cmd, F3DCommand::Unknown { .. }) {
            //     eprintln!("{}{:?}", indent_str, cmd);
            // }
            match cmd {
                F3DCommand::NoOp => {}
                F3DCommand::Rsp(cmd) => match cmd {
                    SPCommand::Matrix {
                        matrix,
                        mode,
                        op,
                        push,
                    } => {
                        self.flush(backend);
                        let fixed = read_matrix(source, matrix, 0);
                        let m = Matrixf::from_fixed(&fixed);
                        match mode {
                            MatrixMode::Proj => self.proj.execute(m, op, push),
                            MatrixMode::ModelView => self.model_view.execute(m, op, push),
                        }
                    }
                    SPCommand::Viewport(ptr) => {
                        let viewport = read_viewport(source, ptr, 0);
                        if self.viewport != viewport {
                            self.flush(backend);
                            self.viewport = viewport;

                            let width = 2.0 * viewport.scale[0] as f32 / 4.0;
                            let height = 2.0 * viewport.scale[1] as f32 / 4.0;
                            let x = (viewport.trans[0] as f32 / 4.0) - width / 2.0;
                            let y = 240.0 - ((viewport.trans[1] as f32 / 4.0) + height / 2.0);

                            backend.set_viewport(x as i32, y as i32, width as i32, height as i32);
                        }
                    }
                    // SPCommand::Light { light, n } => todo!(),
                    SPCommand::Vertex { v, n, v0 } => {
                        let offset = v0 as usize * mem::size_of::<Vertex>();
                        self.vertices = read_vertices(source, v, offset, n as usize);
                    }
                    SPCommand::DisplayList(ptr) => {
                        let child_dl = source.read_dl(ptr);
                        self.interpret(source, backend, child_dl, indent + 1);
                    }
                    SPCommand::BranchList(ptr) => {
                        let child_dl = source.read_dl(ptr);
                        self.interpret(source, backend, child_dl, indent + 1);
                        break;
                    }
                    SPCommand::OneTriangle { v0, v1, v2, flag } => {
                        let cc_features = decode_shader_id(self.get_shader_id());

                        // TODO: Use flag for flat shading
                        let vertices = [self.vertex(v0), self.vertex(v1), self.vertex(v2)];

                        let tile_size = &self.tile_size[0];
                        let texture_width = (tile_size.lrs - tile_size.uls + 4) / 4;
                        let texture_height = (tile_size.lrt - tile_size.ult + 4) / 4;

                        for vtx in vertices {
                            let model_pos =
                                [vtx.pos[0] as f32, vtx.pos[1] as f32, vtx.pos[2] as f32, 1.0];
                            let mut pos = &self.proj.cur * (&self.model_view.cur * model_pos);
                            if backend.z_is_from_0_to_1() {
                                pos[2] = (pos[2] + pos[3]) / 2.0;
                            }
                            self.vertex_buffer.extend(&pos);

                            if cc_features.uses_textures() {
                                let mut u = vtx.uv[0] as f32 * self.texture_scale[0][0];
                                let mut v = vtx.uv[1] as f32 * self.texture_scale[0][1];
                                u = (u - tile_size.uls as f32 * 8.0) / 32.0;
                                v = (v - tile_size.ult as f32 * 8.0) / 32.0;
                                if self.texture_filter != TextureFilter::Point {
                                    u += 0.5;
                                    v += 0.5;
                                }
                                u /= texture_width as f32;
                                v /= texture_height as f32;
                                self.vertex_buffer.extend(&[u, v]);
                            }

                            if cc_features.opt_fog {
                                self.vertex_buffer.extend(&[1.0, 1.0, 1.0, 1.0]);
                            }

                            for input_index in 0..cc_features.num_inputs {
                                if cc_features.opt_alpha {
                                    self.vertex_buffer.extend(&[1.0, 0.0, 0.0, 1.0]);
                                } else {
                                    self.vertex_buffer.extend(&[0.0, 1.0, 0.0]);
                                }
                            }
                        }
                        self.vertex_buffer_num_tris += 1;
                    }
                    SPCommand::PopMatrix(mode) => {
                        self.flush(backend);
                        match mode {
                            MatrixMode::Proj => self.proj.pop(),
                            MatrixMode::ModelView => self.model_view.pop(),
                        }
                    }
                    // SPCommand::NumLights(_) => todo!(),
                    // SPCommand::Segment { seg, base } => todo!(),
                    // SPCommand::FogFactor { mul, offset } => todo!(),
                    SPCommand::Texture { sc, tc, tile, .. } => {
                        self.flush(backend);
                        self.texture_scale[tile as usize] =
                            [sc as f32 / 0x10000 as f32, tc as f32 / 0x10000 as f32];
                    }
                    SPCommand::EndDisplayList => break,
                    SPCommand::SetGeometryMode(mode) => {
                        self.set_geometry_mode(backend, self.geometry_mode | mode);
                    }
                    SPCommand::ClearGeometryMode(mode) => {
                        self.set_geometry_mode(backend, self.geometry_mode & !mode);
                    }
                    _ => {} //unimplemented!("{:?}", cmd),
                },
                F3DCommand::Rdp(cmd) => match cmd {
                    DPCommand::SetAlphaDither(v) => {
                        if self.alpha_dither != v {
                            self.flush(backend);
                            self.alpha_dither = v;
                        }
                    }
                    DPCommand::SetColorDither(v) => {
                        if self.color_dither != v {
                            self.flush(backend);
                            self.color_dither = v;
                        }
                    }
                    DPCommand::SetCombineKey(v) => {
                        if self.combine_key != v {
                            self.flush(backend);
                            self.combine_key = v;
                        }
                    }
                    DPCommand::SetTextureConvert(v) => {
                        if self.texture_convert != v {
                            self.flush(backend);
                            self.texture_convert = v;
                        }
                    }
                    DPCommand::SetTextureFilter(v) => {
                        if self.texture_filter != v {
                            self.flush(backend);
                            self.texture_filter = v;
                        }
                    }
                    DPCommand::SetTextureLUT(v) => {
                        if self.texture_lut != v {
                            self.flush(backend);
                            self.texture_lut = v;
                        }
                    }
                    DPCommand::SetTextureLOD(v) => {
                        if self.texture_lod != v {
                            self.flush(backend);
                            self.texture_lod = v;
                        }
                    }
                    DPCommand::SetTextureDetail(v) => {
                        if self.texture_detail != v {
                            self.flush(backend);
                            self.texture_detail = v;
                        }
                    }
                    DPCommand::SetTexturePersp(v) => {
                        if self.texture_persp != v {
                            self.flush(backend);
                            self.texture_persp = v;
                        }
                    }
                    DPCommand::SetCycleType(v) => {
                        if v == CycleType::TwoCycle {
                            unimplemented!("cycle type: {:?}", v);
                        }
                        if self.cycle_type != v {
                            self.flush(backend);
                            self.cycle_type = v;
                        }
                    }
                    DPCommand::PipelineMode(v) => {
                        if self.pipeline_mode != v {
                            self.flush(backend);
                            self.pipeline_mode = v;
                        }
                    }
                    DPCommand::SetAlphaCompare(v) => {
                        if self.alpha_compare != v {
                            self.flush(backend);
                            self.alpha_compare = v;
                        }
                    }
                    DPCommand::SetDepthSource(v) => {
                        if self.depth_source != v {
                            self.flush(backend);
                            self.depth_source = v;
                        }
                    }
                    DPCommand::SetRenderMode(v) => {
                        if self.render_mode != v {
                            self.flush(backend);
                            self.render_mode = v;

                            backend.set_depth_mask(v.flags.contains(RenderModeFlags::Z_UPDATE));
                            backend.set_zmode_decal(v.z_mode == ZMode::Decal);
                        }
                    }
                    DPCommand::PerspNormalize(v) => {
                        if self.persp_normalize != v {
                            self.flush(backend);
                            self.persp_normalize = v;
                        }
                    }
                    // DPCommand::SetColorImage(_) => todo!(),
                    // DPCommand::SetDepthImage(_) => todo!(),
                    DPCommand::SetTextureImage(image) => {
                        self.texture_image = Some(image);
                    }
                    DPCommand::SetCombineMode(mode) => {
                        if self.combine_mode != mode {
                            self.flush(backend);
                            self.combine_mode = mode;
                        }
                    }
                    DPCommand::SetEnvColor(color) => {
                        self.env_color = color;
                    }
                    DPCommand::SetPrimColor(color) => {
                        self.prim_color = color;
                    }
                    DPCommand::SetBlendColor(color) => {
                        self.blend_color = color;
                    }
                    DPCommand::SetFogColor(color) => {
                        self.fog_color = color;
                    }
                    // DPCommand::SetFillColor(_) => todo!(),
                    // DPCommand::FillRectangle(_) => todo!(),
                    DPCommand::SetTile(tile, params) => {
                        self.flush(backend);
                        self.tile_params[tile.0 as usize] = params;
                    }
                    // DPCommand::LoadTile(_, _) => todo!(),
                    DPCommand::LoadBlock(tile, params) => {
                        use ComponentSize::*;
                        use ImageFormat::*;

                        self.flush(backend);

                        let load_tile_params = &self.tile_params[tile.0 as usize];
                        let render_tile_params = &self.tile_params[0];
                        let image = self
                            .texture_image
                            .as_ref()
                            .expect("missing call to SetTextureImage");

                        // eprintln!("Load block:");
                        // eprintln!("{:?}", params);
                        // eprintln!("{:?}", load_tile_params);
                        // eprintln!("{:?}", render_tile_params);
                        // eprintln!("{:?}", image);

                        let line_size_bytes = render_tile_params.line * 8;
                        let size_bytes = render_tile_params.size.num_bits() * (params.lrs + 1) / 8;

                        let rgba32 = match (render_tile_params.fmt, render_tile_params.size) {
                            (Rgba, Bits16) => {
                                read_rgba16(source, image.img, size_bytes, line_size_bytes)
                            }
                            // TODO: fmt => unimplemented!("texture format: {:?}", fmt),
                            _ => TextureRgba32::dbg_gradient(),
                        };

                        // dbg!(line_size_bytes);
                        // dbg!(size_bytes);
                        // dbg!((rgba32.width, rgba32.height));

                        let texture_id = backend.new_texture();
                        backend.select_texture(0, texture_id);
                        backend.upload_texture(
                            &rgba32.data,
                            rgba32.width as i32,
                            rgba32.height as i32,
                        );
                        backend.set_sampler_parameters(0, true, 0, 0);

                        self.tmem_to_texture_id
                            .insert(load_tile_params.tmem, texture_id);
                    }
                    DPCommand::SetTileSize(tile, size) => {
                        self.flush(backend);
                        self.tile_size[tile.0 as usize] = size;
                    }
                    // DPCommand::LoadTLUTCmd(_, _) => todo!(),
                    // DPCommand::SetOtherMode(_) => todo!(),
                    // DPCommand::SetPrimDepth(_) => todo!(),
                    DPCommand::SetScissor(mode, rect) => {
                        assert!(rect.lrx > rect.ulx && rect.lry > rect.uly);
                        let new_scissor = (mode, rect);
                        if self.scissor != new_scissor {
                            self.flush(backend);
                            self.scissor = new_scissor;
                            backend.set_scissor(
                                rect.ulx.into_inner() as i32,
                                rect.uly.into_inner() as i32,
                                (rect.lrx - rect.ulx).into_inner() as i32,
                                (rect.lry - rect.uly).into_inner() as i32,
                            );
                        }
                    }
                    // DPCommand::SetConvert(_) => todo!(),
                    // DPCommand::SetKeyR(_) => todo!(),
                    // DPCommand::SetKeyGB(_) => todo!(),
                    DPCommand::FullSync => {}
                    DPCommand::TileSync => {}
                    DPCommand::PipeSync => {}
                    DPCommand::LoadSync => {}
                    // DPCommand::TextureRectangleFlip(_) => todo!(),
                    // DPCommand::TextureRectangle(_) => todo!(),
                    _ => {} //unimplemented!("{:?}", cmd),
                },
                F3DCommand::Unknown(_) => {
                    // TODO
                    // eprintln!("{}{:?}", indent_str, cmd);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Matrixf([[f32; 4]; 4]);

impl Matrixf {
    fn identity() -> Self {
        Self([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn from_fixed(m: &[i32]) -> Self {
        let mut r = Self::default();
        for i in [0, 2] {
            for j in 0..4 {
                let int_part = m[j * 2 + i / 2] as u32;
                let frac_part = m[8 + j * 2 + i / 2] as u32;
                r.0[i][j] = ((int_part & 0xFFFF0000) | (frac_part >> 16)) as i32 as f32 / 65536.0;
                r.0[i + 1][j] = ((int_part << 16) | (frac_part & 0xFFFF)) as i32 as f32 / 65536.0;
            }
        }
        r
    }
}

impl ops::Mul<&Matrixf> for &Matrixf {
    type Output = Matrixf;

    fn mul(self, rhs: &Matrixf) -> Self::Output {
        let mut out = Matrixf::default();
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    out.0[i][j] += self.0[i][k] * rhs.0[k][j];
                }
            }
        }
        out
    }
}

impl ops::Mul<[f32; 4]> for &Matrixf {
    type Output = [f32; 4];

    fn mul(self, rhs: [f32; 4]) -> Self::Output {
        let mut out = [0.0; 4];
        for i in 0..4 {
            for k in 0..4 {
                out[i] += self.0[i][k] * rhs[k];
            }
        }
        out
    }
}

fn read_matrix<S: F3DSource>(source: &S, ptr: S::Ptr, offset: usize) -> Vec<i32> {
    let mut m = vec![0; 16];
    source.read_u32(cast_slice_mut(&mut m), ptr, offset);
    m
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Viewport {
    scale: [i16; 4],
    trans: [i16; 4],
}

fn read_viewport<S: F3DSource>(source: &S, ptr: S::Ptr, offset: usize) -> Viewport {
    let mut v = Viewport::default();
    source.read_u16(cast_slice_mut(&mut v.scale), ptr, offset);
    source.read_u16(cast_slice_mut(&mut v.trans), ptr, offset + 8);
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Vertex {
    pos: [i16; 3],
    padding: u16,
    uv: [i16; 2],
    cn: [u8; 4],
}

fn read_vertices<S: F3DSource>(
    source: &S,
    ptr: S::Ptr,
    offset: usize,
    count: usize,
) -> Vec<Vertex> {
    let stride = mem::size_of::<Vertex>();
    let mut vs = Vec::new();
    for i in 0..count {
        let mut v = Vertex::default();
        let voffset = offset + i * stride;
        source.read_u16(cast_slice_mut(&mut v.pos), ptr, voffset);
        source.read_u16(cast_slice_mut(&mut v.uv), ptr, voffset + 8);
        source.read_u8(cast_slice_mut(&mut v.cn), ptr, voffset + 12);
        vs.push(v);
    }
    vs
}

#[derive(Debug, Clone)]
struct TextureRgba32 {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl TextureRgba32 {
    #[track_caller]
    fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        assert!(4 * width * height <= data.len() as u32);
        Self {
            width,
            height,
            data,
        }
    }

    #[allow(dead_code)]
    fn dbg_constant(r: u8, g: u8, b: u8, a: u8) -> Self {
        let width = 32;
        let height = 32;
        let mut data = Vec::new();
        for _ in 0..width * height {
            data.extend(&[r, g, b, a]);
        }
        Self::new(width, height, data)
    }

    #[allow(dead_code)]
    fn dbg_gradient() -> Self {
        let width = 32;
        let height = 32;
        let mut data = Vec::new();
        for i in 0..height {
            for j in 0..width {
                let u = i as f32 / height as f32;
                let v = j as f32 / width as f32;
                let r = 0.0;
                let g = u;
                let b = v;
                data.extend(&[(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]);
            }
        }
        Self::new(width, height, data)
    }
}

fn read_rgba16<S: F3DSource>(
    source: &S,
    ptr: S::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut rgba16_data: Vec<u8> = vec![0; size_bytes as usize];
    source.read_u8(&mut rgba16_data, ptr, 0);

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(2 * size_bytes as usize);

    for i in 0..size_bytes / 2 {
        let i0 = (2 * i) as usize;
        let rgba16 = ((rgba16_data[i0] as u16) << 8) | rgba16_data[i0 + 1] as u16;
        let rgba32 = rgba_16_to_32(rgba16);
        rgba32_data.extend(&rgba32);
    }

    TextureRgba32::new(
        line_size_bytes / 2,
        size_bytes / line_size_bytes,
        rgba32_data,
    )
}

fn rgba_16_to_32(rgba16: u16) -> [u8; 4] {
    [
        (((rgba16 >> 8) & 0xF8) as u32 * 255 / 0xF8) as u8,
        (((rgba16 >> 3) & 0xF8) as u32 * 255 / 0xF8) as u8,
        (((rgba16 << 2) & 0xF8) as u32 * 255 / 0xF8) as u8,
        (rgba16 & 0x1) as u8 * 255,
    ]
}
