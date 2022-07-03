use core::fmt;
use std::{collections::HashMap, mem, ops};

use bytemuck::cast_slice_mut;
use derivative::Derivative;

use crate::{
    f3d_decode::*,
    render_api::{
        decode_shader_id, encode_shader_id, CCFeatures, CullMode, RenderBackend, ShaderId,
        ShaderItem,
    },
};

pub trait F3DSource {
    type Ptr: fmt::Debug + Copy + PartialEq;
    type DlIter: Iterator<Item = F3DCommand<Self::Ptr>>;

    fn root_dl(&self) -> Self::DlIter;
    fn read_dl(&self, ptr: Self::Ptr) -> Self::DlIter;

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize);
    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize);
    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize);
}

pub fn interpret_f3d_display_list(source: &impl F3DSource, backend: &mut impl RenderBackend) {
    let mut state = State::default();
    state.interpret(source, backend, source.root_dl());
    state.flush(backend);
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
struct State<Ptr> {
    color_image: Option<Image<Ptr>>,
    depth_image: Option<Ptr>,

    viewport: Viewport,
    scissor: (ScissorMode, Rectangle<u16>),
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

    fill_color: FillColor,

    lights: [Light; 8],
    num_dir_lights: u32,
    fog_mul: i16,
    fog_offset: i16,

    geometry_mode: GeometryModes,

    texture_image: Option<Image<Ptr>>,
    texture_scale: [[f32; 2]; 8],
    tile_params: [TileParams; 8],
    tile_size: [TileSize; 8],
    texture_memory: HashMap<u32, TextureMemory<Ptr>>,

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

#[derive(Debug)]
struct TextureMemory<Ptr> {
    image: Image<Ptr>,
    block: TextureBlock,
    texture_ids: HashMap<(ImageFormat, ComponentSize), u32>,
}

impl<Ptr: fmt::Debug + Copy + PartialEq> State<Ptr> {
    fn vertex(&self, index: u32) -> Vertex {
        *self
            .vertices
            .get(index as usize)
            .expect("invalid vertex index")
    }

    fn flush(&mut self, backend: &mut impl RenderBackend) {
        if self.vertex_buffer_num_tris > 0 {
            self.flush_with_shader(backend, self.get_shader_id());
        }
    }

    fn flush_with_shader(&mut self, backend: &mut impl RenderBackend, shader_id: u32) {
        if self.vertex_buffer_num_tris > 0 {
            backend.load_shader(ShaderId(shader_id as usize));
            backend.draw_triangles(&self.vertex_buffer, self.vertex_buffer_num_tris);
            self.vertex_buffer.clear();
            self.vertex_buffer_num_tris = 0;
        }
    }

    fn set_geometry_mode(&mut self, backend: &mut impl RenderBackend, mode: GeometryModes) {
        self.flush(backend);
        self.geometry_mode = mode;

        backend.set_depth_test(mode.contains(GeometryModes::ZBUFFER));
        if mode.contains(GeometryModes::CULL_BACK) {
            backend.set_cull_mode(CullMode::Back);
        } else if mode.contains(GeometryModes::CULL_FRONT) {
            // CULL_BOTH handled in software
            backend.set_cull_mode(CullMode::Front);
        } else {
            backend.set_cull_mode(CullMode::None);
        }
    }

    fn set_viewport(&mut self, backend: &mut impl RenderBackend, viewport: Viewport) {
        self.flush(backend);
        self.viewport = viewport;

        let width = 2.0 * viewport.scale[0] as f32 / 4.0;
        let height = 2.0 * viewport.scale[1] as f32 / 4.0;
        let x = (viewport.trans[0] as f32 / 4.0) - width / 2.0;
        let y = 240.0 - ((viewport.trans[1] as f32 / 4.0) + height / 2.0);

        backend.set_viewport(x as i32, y as i32, width as i32, height as i32);
    }

    fn set_scissor(
        &mut self,
        backend: &mut impl RenderBackend,
        mode: ScissorMode,
        rect: Rectangle<u16>,
    ) {
        assert!(rect.lrx > rect.ulx && rect.lry > rect.uly);
        self.flush(backend);
        self.scissor = (mode, rect);

        let ulx = rect.ulx as f32 / 4.0;
        let uly = rect.uly as f32 / 4.0;
        let lrx = rect.lrx as f32 / 4.0;
        let lry = rect.lry as f32 / 4.0;

        backend.set_scissor(
            ulx as i32,
            (240.0 - lry) as i32,
            (lrx - ulx) as i32,
            (lry - uly) as i32,
        );
    }

    fn set_render_mode(&mut self, backend: &mut impl RenderBackend, rm: RenderMode) {
        self.flush(backend);
        self.render_mode = rm;

        backend.set_depth_mask(rm.flags.contains(RenderModeFlags::Z_UPDATE));
        backend.set_zmode_decal(rm.z_mode == ZMode::Decal);
        backend.set_use_alpha(
            rm.blend_cycle1.alpha2 != BlendAlpha2::Memory
                || rm.flags.contains(RenderModeFlags::CVG_X_ALPHA),
        );
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

    fn get_vertex_input_components(&self) -> [[ColorCombineComponent; 4]; 2] {
        let cm = &self.combine_mode;
        let mut components: [[ColorCombineComponent; 4]; 2] = Default::default();
        for (i, mode) in [cm.color1, cm.alpha1].into_iter().enumerate() {
            let mut num_inputs = 0;
            for cc in mode.args {
                if matches!(
                    cc,
                    ColorCombineComponent::Prim
                        | ColorCombineComponent::Shade
                        | ColorCombineComponent::Env
                        | ColorCombineComponent::LodFraction
                ) {
                    components[i][num_inputs] = cc;
                    num_inputs += 1;
                };
            }
        }
        components
    }

    fn load_texture<S: F3DSource<Ptr = Ptr>>(
        &mut self,
        source: &S,
        backend: &mut impl RenderBackend,
        tile: TileIndex,
    ) -> u32 {
        use ComponentSize::*;
        use ImageFormat::*;

        self.flush(backend);

        let tile_params = &self.tile_params[tile.0 as usize];
        let tmem = self
            .texture_memory
            .get_mut(&tile_params.tmem)
            .expect("invalid tmem offset");

        let line_size_bytes = tile_params.line * 8;
        let size_bytes = tmem.image.size.num_bits() * (tmem.block.lrs + 1) / 8;

        let fmt = (tile_params.fmt, tile_params.size);
        if let Some(&texture_id) = tmem.texture_ids.get(&fmt) {
            backend.select_texture(tile.0.into(), texture_id);
            return texture_id;
        }

        let rgba32 = match fmt {
            (Rgba, Bits16) => read_rgba16(source, tmem.image.img, size_bytes, line_size_bytes),
            (Ia, Bits16) => read_ia16(source, tmem.image.img, size_bytes, line_size_bytes),
            (Ia, Bits8) => read_ia8(source, tmem.image.img, size_bytes, line_size_bytes),
            (Ia, Bits4) => read_ia4(source, tmem.image.img, size_bytes, line_size_bytes),
            fmt => unimplemented!("texture format: {:?}", fmt),
            // _ => TextureRgba32::dbg_gradient(),
        };

        let texture_id = backend.new_texture();
        backend.select_texture(tile.0.into(), texture_id);
        backend.upload_texture(&rgba32.data, rgba32.width as i32, rgba32.height as i32);
        tmem.texture_ids.insert(fmt, texture_id);

        self.set_sampler_parameters(backend, tile);

        texture_id
    }

    fn set_sampler_parameters(&self, backend: &mut impl RenderBackend, tile: TileIndex) {
        let tile_params = &self.tile_params[tile.0 as usize];
        backend.set_sampler_parameters(
            tile.0.into(),
            self.texture_filter != TextureFilter::Point,
            u8::from(tile_params.cms).into(),
            u8::from(tile_params.cmt).into(),
        );
    }

    fn transform_pos(&self, vtx: &Vertex) -> [f32; 4] {
        let model_pos = [vtx.pos[0] as f32, vtx.pos[1] as f32, vtx.pos[2] as f32, 1.0];
        &self.proj.cur * (&self.model_view.cur * model_pos)
    }

    fn calculate_uv(&self, vtx: &Vertex) -> [f32; 2] {
        let tile_size = &self.tile_size[0];
        let texture_width = (tile_size.lrs - tile_size.uls + 4) / 4;
        let texture_height = (tile_size.lrt - tile_size.ult + 4) / 4;

        let mut lookat_x_coeffs = [0.0; 4];
        let mut lookat_y_coeffs = [0.0; 4];
        if self.geometry_mode.contains(GeometryModes::TEXTURE_GEN) {
            let lookat_x = [1.0, 0.0, 0.0, 0.0];
            lookat_x_coeffs = &self.model_view.cur.transpose() * lookat_x;
            lookat_x_coeffs[3] = 0.0;
            lookat_x_coeffs = normalize(lookat_x_coeffs);

            let lookat_y = [0.0, 1.0, 0.0, 0.0];
            lookat_y_coeffs = &self.model_view.cur.transpose() * lookat_y;
            lookat_y_coeffs[3] = 0.0;
            lookat_y_coeffs = normalize(lookat_y_coeffs);
        }

        let mut u;
        let mut v;
        if self.geometry_mode.contains(GeometryModes::TEXTURE_GEN) {
            let mut dotx = 0.0;
            let mut doty = 0.0;
            for i in 0..3 {
                dotx += vtx.cn[i] as i8 as f32 * lookat_x_coeffs[i];
                doty += vtx.cn[i] as i8 as f32 * lookat_y_coeffs[i];
            }
            u = (dotx / 127.0 + 1.0) / 4.0 * 0x10000 as f32;
            v = (doty / 127.0 + 1.0) / 4.0 * 0x10000 as f32;
        } else {
            u = vtx.uv[0] as f32;
            v = vtx.uv[1] as f32;
        }

        u *= self.texture_scale[0][0];
        v *= self.texture_scale[0][1];
        u = (u - tile_size.uls as f32 * 8.0) / 32.0;
        v = (v - tile_size.ult as f32 * 8.0) / 32.0;
        if self.texture_filter != TextureFilter::Point {
            u += 0.5;
            v += 0.5;
        }
        u /= texture_width as f32;
        v /= texture_height as f32;

        [u, v]
    }

    fn calculate_shade(&self, vtx: &Vertex) -> Rgba32 {
        let mut shade_rgb: [u8; 3];
        if self.geometry_mode.contains(GeometryModes::LIGHTING) {
            shade_rgb = self.lights[self.num_dir_lights as usize].color;
            for light in &self.lights[0..self.num_dir_lights as usize] {
                let light_dir = [
                    light.dir[0] as f32 / 127.0,
                    light.dir[1] as f32 / 127.0,
                    light.dir[2] as f32 / 127.0,
                    0.0,
                ];
                let mut light_n = &self.model_view.cur.transpose() * light_dir;
                light_n[3] = 0.0;
                let light_n = normalize(light_n);

                let n = [
                    vtx.cn[0] as i8 as f32 / 127.0,
                    vtx.cn[1] as i8 as f32 / 127.0,
                    vtx.cn[2] as i8 as f32 / 127.0,
                    0.0,
                ];
                let intensity = dot(light_n, n).max(0.0);
                for i in 0..3 {
                    shade_rgb[i] =
                        (shade_rgb[i] as f32 + intensity * light.color[i] as f32).min(255.0) as u8;
                }
            }
        } else {
            shade_rgb = [vtx.cn[0], vtx.cn[1], vtx.cn[2]];
        }

        Rgba32::from_rgb_a(shade_rgb, vtx.cn[3])
    }

    fn calculate_fog(&self, pos: [f32; 4]) -> [f32; 4] {
        let mut w = pos[3];
        if w.abs() < 0.001 {
            w = 0.001;
        }
        let mut w_inv = 1.0 / w;
        if w_inv < 0.0 {
            w_inv = 32767.0;
        }
        let fog_factor = pos[2] * w_inv * self.fog_mul as f32 + self.fog_offset as f32;
        [
            self.fog_color.r as f32 / 255.0,
            self.fog_color.g as f32 / 255.0,
            self.fog_color.b as f32 / 255.0,
            fog_factor.clamp(0.0, 255.0) / 255.0,
        ]
    }

    fn calculate_lod_fraction(&self, pos: [f32; 4]) -> u8 {
        let lod_fraction = ((pos[3] - 3000.0) / 3000.0).clamp(0.0, 1.0);
        (lod_fraction * 255.0) as u8
    }

    fn push_vertex_color_inputs(
        &mut self,
        cc_features: &CCFeatures,
        input_comps: &[[ColorCombineComponent; 4]; 2],
        vtx: &Vertex,
        pos: [f32; 4],
    ) {
        if cc_features.opt_fog {
            let fog = self.calculate_fog(pos);
            self.vertex_buffer.extend(&fog);
        }

        let shade = self.calculate_shade(vtx);
        let lod_fraction = self.calculate_lod_fraction(pos);

        for input_index in 0..cc_features.num_inputs {
            let rgb_comp = input_comps[0][input_index as usize];
            let [r, g, b] = match rgb_comp {
                ColorCombineComponent::Prim => self.prim_color.rgb(),
                ColorCombineComponent::Shade => shade.rgb(),
                ColorCombineComponent::Env => self.env_color.rgb(),
                ColorCombineComponent::LodFraction => [lod_fraction, lod_fraction, lod_fraction],
                ColorCombineComponent::Zero => [0, 0, 0],
                c => unimplemented!("{:?}", c),
            };
            self.vertex_buffer
                .extend(&[r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);

            if cc_features.opt_alpha {
                let a_comp = input_comps[1][input_index as usize];
                let a = match a_comp {
                    ColorCombineComponent::Prim => self.prim_color.a,
                    ColorCombineComponent::Shade => shade.a,
                    ColorCombineComponent::Env => self.env_color.a,
                    ColorCombineComponent::LodFraction => lod_fraction,
                    ColorCombineComponent::Zero => 0,
                    c => unimplemented!("{:?}", c),
                };
                self.vertex_buffer.push(a as f32 / 255.0);
            }
        }
    }

    fn set_modes_for_rect(&mut self, backend: &mut impl RenderBackend) {
        self.flush(backend);
        backend.set_depth_test(false);
        backend.set_depth_mask(false);
        backend.set_viewport(0, 0, 320, 240);
        backend.set_use_alpha(true);
        backend.set_cull_mode(CullMode::None);
    }

    fn reset_modes_for_rect(&mut self, backend: &mut impl RenderBackend) {
        assert!(self.vertex_buffer.is_empty());
        self.set_geometry_mode(backend, self.geometry_mode);
        self.set_render_mode(backend, self.render_mode);
        self.set_viewport(backend, self.viewport);
    }

    fn interpret<S: F3DSource<Ptr = Ptr>>(
        &mut self,
        source: &S,
        backend: &mut impl RenderBackend,
        dl: S::DlIter,
    ) {
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
                            self.set_viewport(backend, viewport);
                        }
                    }
                    SPCommand::Light { light, n } => {
                        self.flush(backend);
                        let index = (n - 1) as usize;
                        self.lights[index] = read_light(source, light);
                    }
                    SPCommand::Vertex { v, n, v0 } => {
                        let offset = v0 as usize * mem::size_of::<Vertex>();
                        self.vertices = read_vertices(source, v, offset, n as usize);
                    }
                    SPCommand::DisplayList(ptr) => {
                        let child_dl = source.read_dl(ptr);
                        self.interpret(source, backend, child_dl);
                    }
                    SPCommand::BranchList(ptr) => {
                        let child_dl = source.read_dl(ptr);
                        self.interpret(source, backend, child_dl);
                        break;
                    }
                    SPCommand::OneTriangle { v0, v1, v2, .. } => {
                        if self.geometry_mode.contains(GeometryModes::CULL_BACK)
                            && self.geometry_mode.contains(GeometryModes::CULL_FRONT)
                        {
                            continue;
                        }

                        let cc_features = decode_shader_id(self.get_shader_id());
                        let input_comps = self.get_vertex_input_components();

                        for i in 0..2 {
                            if cc_features.used_textures[i] {
                                self.load_texture(source, backend, TileIndex(i as u8));
                            }
                        }

                        for vi in [v0, v1, v2] {
                            let vtx = self.vertex(vi);

                            let pos = self.transform_pos(&vtx);
                            let mut rendered_pos = pos;
                            if backend.z_is_from_0_to_1() {
                                rendered_pos[2] = (pos[2] + pos[3]) / 2.0;
                            }
                            self.vertex_buffer.extend(&rendered_pos);

                            if cc_features.uses_textures() {
                                let uv = self.calculate_uv(&vtx);
                                self.vertex_buffer.extend(&uv);
                            }

                            self.push_vertex_color_inputs(&cc_features, &input_comps, &vtx, pos);
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
                    SPCommand::NumLights(n) => {
                        self.flush(backend);
                        self.num_dir_lights = n;
                    }
                    // SPCommand::Segment { seg, base } => todo!(),
                    SPCommand::FogFactor { mul, offset } => {
                        self.flush(backend);
                        self.fog_mul = mul;
                        self.fog_offset = offset;
                    }
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
                    _ => unimplemented!("{:?}", cmd),
                    // _ => {}
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
                            self.set_render_mode(backend, v);
                        }
                    }
                    DPCommand::PerspNormalize(v) => {
                        if self.persp_normalize != v {
                            self.flush(backend);
                            self.persp_normalize = v;
                        }
                    }
                    DPCommand::SetColorImage(image) => {
                        self.flush(backend);
                        self.color_image = Some(image);
                    }
                    DPCommand::SetDepthImage(image) => {
                        self.flush(backend);
                        self.depth_image = Some(image);
                    }
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
                    DPCommand::SetFillColor(fill_color) => {
                        assert_eq!(
                            fill_color[0], fill_color[1],
                            "multiple fill colors not implemented"
                        );
                        self.fill_color = fill_color[0];
                    }
                    DPCommand::FillRectangle(mut rect) => {
                        use ShaderItem::*;

                        if let (Some(color), Some(depth)) = (self.color_image, self.depth_image) {
                            if color.img == depth {
                                continue;
                            }
                        }

                        self.flush(backend);

                        if matches!(self.cycle_type, CycleType::Fill | CycleType::Copy) {
                            rect.lrx += 1;
                            rect.lry += 1;
                        }

                        let cc_features = CCFeatures {
                            c: [[Zero, Zero, Zero, Input1], [Zero, Zero, Zero, Input1]],
                            opt_alpha: true,
                            num_inputs: 1,
                            ..Default::default()
                        };
                        let shader_id = encode_shader_id(cc_features);

                        self.set_modes_for_rect(backend);

                        let fill_color = rgba_16_to_32(self.fill_color.0);

                        let mut add_vertex = |x, y| {
                            self.vertex_buffer.extend(&[x, y, 0.0, 1.0]);
                            self.vertex_buffer.extend(&[
                                fill_color[0] as f32 / 255.0,
                                fill_color[1] as f32 / 255.0,
                                fill_color[2] as f32 / 255.0,
                                fill_color[3] as f32 / 255.0,
                            ]);
                        };

                        let x0 = 2.0 * (rect.ulx as f32 / 320.0) - 1.0;
                        let x1 = 2.0 * (rect.lrx as f32 / 320.0) - 1.0;
                        let y0 = 1.0 - 2.0 * (rect.uly as f32 / 240.0);
                        let y1 = 1.0 - 2.0 * (rect.lry as f32 / 240.0);

                        add_vertex(x0, y1);
                        add_vertex(x1, y1);
                        add_vertex(x0, y0);

                        add_vertex(x1, y0);
                        add_vertex(x0, y0);
                        add_vertex(x1, y1);

                        self.vertex_buffer_num_tris += 2;

                        self.flush_with_shader(backend, shader_id);
                        self.reset_modes_for_rect(backend);
                    }
                    DPCommand::SetTile(tile, params) => {
                        self.flush(backend);
                        self.tile_params[tile.0 as usize] = params;
                    }
                    DPCommand::LoadBlock(tile, block) => {
                        self.flush(backend);

                        let tile_params = &self.tile_params[tile.0 as usize];
                        let image = self.texture_image.expect("missing call to SetTextureImage");

                        self.texture_memory.insert(
                            tile_params.tmem,
                            TextureMemory {
                                image,
                                block,
                                texture_ids: HashMap::new(),
                            },
                        );
                    }
                    DPCommand::SetTileSize(tile, size) => {
                        self.flush(backend);
                        self.tile_size[tile.0 as usize] = size;
                    }
                    DPCommand::SetScissor(mode, rect) => {
                        if self.scissor != (mode, rect) {
                            self.set_scissor(backend, mode, rect);
                        }
                    }
                    DPCommand::FullSync => {}
                    DPCommand::TileSync => {}
                    DPCommand::PipeSync => {}
                    DPCommand::LoadSync => {}
                    DPCommand::TextureRectangle(tex_rect) => {
                        use ShaderItem::*;

                        if let (Some(color), Some(depth)) = (self.color_image, self.depth_image) {
                            if color.img == depth {
                                continue;
                            }
                        }

                        self.flush(backend);

                        let ulx = tex_rect.rect.ulx as f32 / 4.0;
                        let uly = tex_rect.rect.uly as f32 / 4.0;
                        let mut lrx = tex_rect.rect.lrx as f32 / 4.0;
                        let mut lry = tex_rect.rect.lry as f32 / 4.0;

                        if matches!(self.cycle_type, CycleType::Fill | CycleType::Copy) {
                            lrx += 1.0;
                            lry += 1.0;
                        }

                        let tile = tex_rect.tile;
                        self.load_texture(source, backend, tile);

                        let x0 = 2.0 * (ulx / 320.0) - 1.0;
                        let x1 = 2.0 * (lrx / 320.0) - 1.0;
                        let y0 = 1.0 - 2.0 * (uly / 240.0);
                        let y1 = 1.0 - 2.0 * (lry / 240.0);

                        let tile_size = &self.tile_size[tex_rect.tile.0 as usize];
                        let texture_width = (tile_size.lrs - tile_size.uls + 4) / 4;
                        let texture_height = (tile_size.lrt - tile_size.ult + 4) / 4;

                        let s = tex_rect.s as f32 / 32.0;
                        let t = tex_rect.t as f32 / 32.0;
                        let mut dsdx = tex_rect.dsdx as f32 / 1024.0;
                        let dtdy = tex_rect.dtdy as f32 / 1024.0;

                        if self.cycle_type == CycleType::Copy {
                            dsdx /= 4.0;
                        }

                        let u0 = s as f32 / 32.0 / texture_width as f32;
                        let v0 = t as f32 / 32.0 / texture_height as f32;
                        let u1 = u0 + dsdx * (lrx - ulx) / texture_width as f32;
                        let v1 = v0 + dtdy * (lry - uly) / texture_height as f32;

                        let vertices = [
                            [x0, y1, u0, v1],
                            [x1, y1, u1, v1],
                            [x0, y0, u0, v0],
                            [x1, y0, u1, v0],
                            [x0, y0, u0, v0],
                            [x1, y1, u1, v1],
                        ];

                        if self.cycle_type == CycleType::Copy {
                            self.set_modes_for_rect(backend);

                            let tile_params = &self.tile_params[tile.0 as usize];
                            backend.set_sampler_parameters(
                                tile.0 as i32,
                                false,
                                u8::from(tile_params.cms).into(),
                                u8::from(tile_params.cmt).into(),
                            );

                            let cc_features = CCFeatures {
                                c: [[Zero, Zero, Zero, Texel0], [Zero, Zero, Zero, Texel0]],
                                opt_alpha: true,
                                ..Default::default()
                            };
                            let shader_id = encode_shader_id(cc_features);

                            for [x, y, u, v] in vertices {
                                self.vertex_buffer.extend(&[x, y, 0.0, 1.0]);
                                self.vertex_buffer.extend(&[u, v]);
                            }

                            self.vertex_buffer_num_tris += 2;
                            self.flush_with_shader(backend, shader_id);
                            self.reset_modes_for_rect(backend);
                        } else {
                            let cc_features = decode_shader_id(self.get_shader_id());
                            let input_comps = self.get_vertex_input_components();

                            for [x, y, u, v] in vertices {
                                self.vertex_buffer.extend(&[x, y, 0.0, 1.0]);
                                if cc_features.uses_textures() {
                                    self.vertex_buffer.extend(&[u, v]);
                                }
                                // Assumes that inputs like shade and fog aren't needed
                                self.push_vertex_color_inputs(
                                    &cc_features,
                                    &input_comps,
                                    &Vertex::default(),
                                    Default::default(),
                                );
                            }

                            self.vertex_buffer_num_tris += 2;
                            self.flush(backend);
                        }
                    }
                    _ => unimplemented!("{:?}", cmd),
                    // _ => {}
                },
                F3DCommand::Unknown(_) => {
                    // unimplemented!("{:?}", cmd)
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

    fn transpose(&self) -> Self {
        let mut r = Self::default();
        for i in 0..4 {
            for j in 0..4 {
                r.0[i][j] = self.0[j][i];
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

fn normalize(v: [f32; 4]) -> [f32; 4] {
    let mag = dot(v, v).sqrt();
    if mag == 0.0 {
        v
    } else {
        [v[0] / mag, v[1] / mag, v[2] / mag, v[3] / mag]
    }
}

fn dot(v: [f32; 4], w: [f32; 4]) -> f32 {
    v[0] * w[0] + v[1] * w[1] + v[2] * w[2] + v[3] * w[3]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Light {
    color: [u8; 3],
    pad1: u8,
    color_copy: [u8; 3],
    pad2: u8,
    dir: [i8; 3],
    pad3: u8,
}

fn read_light<S: F3DSource>(source: &S, ptr: S::Ptr) -> Light {
    let mut light = Light::default();
    source.read_u8(&mut light.color, ptr, 0);
    source.read_u8(&mut light.color_copy, ptr, 4);
    source.read_u8(cast_slice_mut(&mut light.dir), ptr, 8);
    light
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

fn read_ia16<S: F3DSource>(
    source: &S,
    ptr: S::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut ia16_data: Vec<u8> = vec![0; size_bytes as usize];
    source.read_u8(&mut ia16_data, ptr, 0);

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(2 * size_bytes as usize);

    for i in 0..size_bytes / 2 {
        let i0 = (2 * i) as usize;
        let intensity = ia16_data[i0] as u8;
        let alpha = ia16_data[i0 + 1] as u8;
        rgba32_data.extend(&[intensity, intensity, intensity, alpha]);
    }

    TextureRgba32::new(
        line_size_bytes / 2,
        size_bytes / line_size_bytes,
        rgba32_data,
    )
}

fn read_ia8<S: F3DSource>(
    source: &S,
    ptr: S::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut ia8_data: Vec<u8> = vec![0; size_bytes as usize];
    source.read_u8(&mut ia8_data, ptr, 0);

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(4 * size_bytes as usize);

    for i in 0..size_bytes {
        let i0 = i as usize;
        let intensity = (ia8_data[i0] >> 4) * 0x11;
        let alpha = (ia8_data[i0] & 0xF) * 0x11;
        rgba32_data.extend(&[intensity, intensity, intensity, alpha]);
    }

    TextureRgba32::new(line_size_bytes, size_bytes / line_size_bytes, rgba32_data)
}

fn read_ia4<S: F3DSource>(
    source: &S,
    ptr: S::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut ia4_data: Vec<u8> = vec![0; size_bytes as usize];
    source.read_u8(&mut ia4_data, ptr, 0);

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(8 * size_bytes as usize);

    for i in 0..2 * size_bytes {
        let v = (ia4_data[(i / 2) as usize] >> ((1 - i % 2) * 4)) & 0xF;
        let intensity = (v >> 1) * 0x24;
        let alpha = v & 0x1;
        rgba32_data.extend(&[intensity, intensity, intensity, alpha * 255]);
    }

    TextureRgba32::new(
        line_size_bytes * 2,
        size_bytes / line_size_bytes,
        rgba32_data,
    )
}
