use core::fmt;
use std::{collections::HashMap, mem, ops};

use bytemuck::cast_slice_mut;
use derivative::Derivative;

use crate::{f3d_decode::*, f3d_render_backend::F3DRenderBackend, f3d_render_data::*};

pub trait F3DMemory {
    type Ptr: fmt::Debug + Copy + PartialEq;
    type DlIter: Iterator<Item = F3DCommand<Self::Ptr>>;

    fn root_dl(&self) -> Self::DlIter;
    fn read_dl(&self, ptr: Self::Ptr) -> Self::DlIter;

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize);
    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize);
    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize);
}

pub fn interpret_f3d_display_list(
    memory: &impl F3DMemory,
    screen_size: (u32, u32),
) -> F3DRenderData {
    let mut backend = F3DRenderBackend::default();
    let mut state = State {
        screen_size,
        ..Default::default()
    };
    state.interpret(memory, &mut backend, memory.root_dl());
    state.flush(&mut backend);
    backend.finish()
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
struct State<Ptr> {
    screen_size: (u32, u32),

    color_image: Option<Image<Ptr>>,
    depth_image: Option<Ptr>,

    viewport: Viewport,
    scissor: (ScissorMode, Rectangle<u16>),
    model_view: MatrixState,
    proj: MatrixState,

    texture_filter: TextureFilter,
    cycle_type: CycleType,
    alpha_compare: AlphaCompare,
    render_mode: RenderMode,

    combine_mode: CombineMode,
    env_color: Rgba32,
    prim_color: Rgba32,
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
    fn aspect(&self) -> f32 {
        self.screen_size.0 as f32 / self.screen_size.1 as f32
    }

    fn screen_scale_x(&self) -> f32 {
        self.screen_size.0 as f32 / 320.0
    }

    fn screen_scale_y(&self) -> f32 {
        self.screen_size.1 as f32 / 240.0
    }

    fn vertex(&self, index: u32) -> Vertex {
        *self
            .vertices
            .get(index as usize)
            .expect("invalid vertex index")
    }

    fn flush(&mut self, backend: &mut F3DRenderBackend) {
        if self.vertex_buffer_num_tris > 0 {
            self.flush_with(
                backend,
                self.viewport_screen(),
                self.scissor_screen(),
                self.pipeline_state(),
            );
        }
    }

    fn flush_with(
        &mut self,
        backend: &mut F3DRenderBackend,
        viewport: ScreenRectangle,
        scissor: ScreenRectangle,
        pipeline: PipelineInfo,
    ) {
        if self.vertex_buffer_num_tris > 0 {
            backend.draw_triangles(
                viewport,
                scissor,
                pipeline,
                &self.vertex_buffer,
                self.vertex_buffer_num_tris,
            );
            self.vertex_buffer.clear();
            self.vertex_buffer_num_tris = 0;
        }
    }

    fn viewport_screen(&self) -> ScreenRectangle {
        let w = 2.0 * self.viewport.scale[0] as f32 / 4.0;
        let h = 2.0 * self.viewport.scale[1] as f32 / 4.0;
        let x = (self.viewport.trans[0] as f32 / 4.0) - w / 2.0;
        let y = (self.viewport.trans[1] as f32 / 4.0) - h / 2.0;

        ScreenRectangle {
            x: (x * self.screen_scale_x()) as i32,
            y: (y * self.screen_scale_y()) as i32,
            w: (w * self.screen_scale_x()) as i32,
            h: (h * self.screen_scale_y()) as i32,
        }
    }

    fn scissor_screen(&self) -> ScreenRectangle {
        let rect = self.scissor.1;

        let ulx = rect.ulx as f32 / 4.0;
        let uly = rect.uly as f32 / 4.0;
        let lrx = rect.lrx as f32 / 4.0;
        let lry = rect.lry as f32 / 4.0;

        ScreenRectangle {
            x: (ulx * self.screen_scale_x()) as i32,
            y: (uly * self.screen_scale_y()) as i32,
            w: ((lrx - ulx) * self.screen_scale_x()) as i32,
            h: ((lry - uly) * self.screen_scale_y()) as i32,
        }
    }

    fn pipeline_state(&self) -> PipelineInfo {
        let rm = &self.render_mode;
        let cm = &self.combine_mode;
        let gm = &self.geometry_mode;

        let mut used_textures = [false; 2];
        let mut num_inputs = 0;
        let mut output_color = ColorExpr::default();

        for (i, mode) in [cm.color1, cm.alpha1].into_iter().enumerate() {
            let channel = if i == 0 {
                &mut output_color.rgb
            } else {
                &mut output_color.a
            };
            let mut channel_num_inputs = 0;
            for (j, cc) in mode.args.into_iter().enumerate() {
                let arg = match cc {
                    ColorCombineComponent::Texel0 => {
                        used_textures[0] = true;
                        ColorArg::Texel0
                    }
                    ColorCombineComponent::Texel1 => {
                        used_textures[1] = true;
                        ColorArg::Texel1
                    }
                    ColorCombineComponent::Texel0Alpha => {
                        used_textures[0] = true;
                        ColorArg::Texel0Alpha
                    }
                    ColorCombineComponent::Prim
                    | ColorCombineComponent::Shade
                    | ColorCombineComponent::Env
                    | ColorCombineComponent::LodFraction => {
                        channel_num_inputs += 1;
                        ColorArg::Input(channel_num_inputs - 1)
                    }
                    _ => ColorArg::Zero,
                };
                channel[j] = arg;
            }
            num_inputs = num_inputs.max(channel_num_inputs);
        }

        PipelineInfo {
            blend: rm.blend_cycle1.alpha2 != BlendAlpha2::Memory
                || rm.flags.contains(RenderModeFlags::CVG_X_ALPHA),
            fog: rm.blend_cycle1.color1 == BlendColor::Fog,
            texture_edge: rm.flags.contains(RenderModeFlags::CVG_X_ALPHA),
            noise: self.alpha_compare == AlphaCompare::Dither,
            cull_mode: if gm.contains(GeometryModes::CULL_BACK) {
                CullMode::Back
            } else if gm.contains(GeometryModes::CULL_FRONT) {
                CullMode::Front
            } else {
                CullMode::None
            },
            depth_compare: gm.contains(GeometryModes::ZBUFFER),
            depth_write: rm.flags.contains(RenderModeFlags::Z_UPDATE),
            decal: rm.z_mode == ZMode::Decal,
            used_textures,
            num_inputs,
            output_color,
        }
    }

    fn get_color_input_components(&self) -> [[ColorCombineComponent; 4]; 2] {
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

    fn load_texture<M: F3DMemory<Ptr = Ptr>>(
        &mut self,
        memory: &M,
        backend: &mut F3DRenderBackend,
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
            (Rgba, Bits16) => read_rgba16(memory, tmem.image.img, size_bytes, line_size_bytes),
            (Ia, Bits16) => read_ia16(memory, tmem.image.img, size_bytes, line_size_bytes),
            (Ia, Bits8) => read_ia8(memory, tmem.image.img, size_bytes, line_size_bytes),
            (Ia, Bits4) => read_ia4(memory, tmem.image.img, size_bytes, line_size_bytes),
            fmt => unimplemented!("texture format: {:?}", fmt),
            // _ => TextureRgba32::dbg_gradient(),
        };

        let texture_id = backend.new_texture();
        backend.select_texture(tile.0.into(), texture_id);
        backend.upload_texture(
            tile.0.into(),
            &rgba32.data,
            rgba32.width as i32,
            rgba32.height as i32,
        );
        tmem.texture_ids.insert(fmt, texture_id);

        self.set_sampler_parameters(backend, tile);

        texture_id
    }

    fn set_sampler_parameters(&self, backend: &mut F3DRenderBackend, tile: TileIndex) {
        let tile_params = &self.tile_params[tile.0 as usize];
        backend.set_sampler_parameters(
            tile.0.into(),
            SamplerState {
                u_wrap: tile_params.cms.into(),
                v_wrap: tile_params.cmt.into(),
                linear_filter: self.texture_filter != TextureFilter::Point,
            },
        );
    }

    fn transform_pos(&self, vtx: &Vertex) -> [f32; 4] {
        let model_pos = [vtx.pos[0] as f32, vtx.pos[1] as f32, vtx.pos[2] as f32, 1.0];
        &self.proj.cur * (&self.model_view.cur * model_pos)
    }

    fn push_pos(&mut self, backend: &mut F3DRenderBackend, mut pos: [f32; 4]) {
        if backend.z_is_from_0_to_1() {
            pos[2] = (pos[2] + pos[3]) / 2.0;
        }
        pos[0] *= (320.0 / 240.0) / self.aspect();
        self.vertex_buffer.extend(&pos);
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
        pipeline: &PipelineInfo,
        input_comps: &[[ColorCombineComponent; 4]; 2],
        vtx: &Vertex,
        pos: [f32; 4],
    ) {
        if pipeline.fog {
            let fog = self.calculate_fog(pos);
            self.vertex_buffer.extend(&fog);
        }

        let shade = self.calculate_shade(vtx);
        let lod_fraction = self.calculate_lod_fraction(pos);

        for input_index in 0..pipeline.num_inputs {
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

            if pipeline.blend {
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

    fn interpret<M: F3DMemory<Ptr = Ptr>>(
        &mut self,
        memory: &M,
        backend: &mut F3DRenderBackend,
        dl: M::DlIter,
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
                        let fixed = read_matrix(memory, matrix, 0);
                        let m = Matrixf::from_fixed(&fixed);
                        match mode {
                            MatrixMode::Proj => self.proj.execute(m, op, push),
                            MatrixMode::ModelView => self.model_view.execute(m, op, push),
                        }
                    }
                    SPCommand::Viewport(ptr) => {
                        let viewport = read_viewport(memory, ptr, 0);
                        if self.viewport != viewport {
                            self.flush(backend);
                            self.viewport = viewport;
                        }
                    }
                    SPCommand::Light { light, n } => {
                        self.flush(backend);
                        let index = (n - 1) as usize;
                        self.lights[index] = read_light(memory, light);
                    }
                    SPCommand::Vertex { v, n, v0 } => {
                        let offset = v0 as usize * mem::size_of::<Vertex>();
                        self.vertices = read_vertices(memory, v, offset, n as usize);
                    }
                    SPCommand::DisplayList(ptr) => {
                        let child_dl = memory.read_dl(ptr);
                        self.interpret(memory, backend, child_dl);
                    }
                    SPCommand::BranchList(ptr) => {
                        let child_dl = memory.read_dl(ptr);
                        self.interpret(memory, backend, child_dl);
                        break;
                    }
                    SPCommand::OneTriangle { v0, v1, v2, .. } => {
                        if self.geometry_mode.contains(GeometryModes::CULL_BACK)
                            && self.geometry_mode.contains(GeometryModes::CULL_FRONT)
                        {
                            continue;
                        }

                        let pipeline = self.pipeline_state();
                        let input_comps = self.get_color_input_components();

                        for i in 0..2 {
                            if pipeline.used_textures[i] {
                                self.load_texture(memory, backend, TileIndex(i as u8));
                            }
                        }

                        for vi in [v0, v1, v2] {
                            let vtx = self.vertex(vi);

                            let pos = self.transform_pos(&vtx);
                            self.push_pos(backend, pos);

                            if pipeline.uses_textures() {
                                let uv = self.calculate_uv(&vtx);
                                self.vertex_buffer.extend(&uv);
                            }

                            self.push_vertex_color_inputs(&pipeline, &input_comps, &vtx, pos);
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
                        self.flush(backend);
                        self.geometry_mode |= mode;
                    }
                    SPCommand::ClearGeometryMode(mode) => {
                        self.flush(backend);
                        self.geometry_mode &= !mode;
                    }
                    // _ => unimplemented!("{:?}", cmd),
                    _ => {}
                },
                F3DCommand::Rdp(cmd) => match cmd {
                    DPCommand::SetTextureFilter(v) => {
                        if self.texture_filter != v {
                            self.flush(backend);
                            self.texture_filter = v;
                        }
                    }
                    DPCommand::SetCycleType(v) => {
                        if self.cycle_type != v {
                            self.flush(backend);
                            self.cycle_type = v;
                        }
                    }
                    DPCommand::SetAlphaCompare(v) => {
                        if self.alpha_compare != v {
                            self.flush(backend);
                            self.alpha_compare = v;
                        }
                    }
                    DPCommand::SetRenderMode(v) => {
                        if self.render_mode != v {
                            self.flush(backend);
                            self.render_mode = v;
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
                        use ColorArg::*;

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

                        let pipeline = PipelineInfo {
                            blend: true,
                            num_inputs: 1,
                            output_color: ColorExpr {
                                rgb: [Zero, Zero, Zero, Input(0)],
                                a: [Zero, Zero, Zero, Input(0)],
                            },
                            ..Default::default()
                        };

                        let fill_color = rgba_16_to_32(self.fill_color.0);

                        let mut add_vertex = |x, y| {
                            self.push_pos(backend, [x, y, 0.0, 1.0]);
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

                        let viewport = ScreenRectangle {
                            x: 0,
                            y: 0,
                            w: self.screen_size.0 as i32,
                            h: self.screen_size.1 as i32,
                        };
                        let scissor = self.scissor_screen();
                        self.flush_with(backend, viewport, scissor, pipeline);
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
                            self.flush(backend);
                            self.scissor = (mode, rect);
                        }
                    }
                    DPCommand::FullSync => {}
                    DPCommand::TileSync => {}
                    DPCommand::PipeSync => {}
                    DPCommand::LoadSync => {}
                    DPCommand::TextureRectangle(tex_rect) => {
                        use ColorArg::*;

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
                        self.load_texture(memory, backend, tile);

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
                            let tile_params = &self.tile_params[tile.0 as usize];
                            backend.set_sampler_parameters(
                                tile.0 as i32,
                                SamplerState {
                                    u_wrap: tile_params.cms.into(),
                                    v_wrap: tile_params.cmt.into(),
                                    linear_filter: false,
                                },
                            );

                            let pipeline = PipelineInfo {
                                blend: true,
                                output_color: ColorExpr {
                                    rgb: [Zero, Zero, Zero, Texel0],
                                    a: [Zero, Zero, Zero, Texel0],
                                },
                                used_textures: [true, false],
                                ..Default::default()
                            };

                            for [x, y, u, v] in vertices {
                                self.push_pos(backend, [x, y, 0.0, 1.0]);
                                self.vertex_buffer.extend(&[u, v]);
                            }

                            self.vertex_buffer_num_tris += 2;

                            let viewport = ScreenRectangle {
                                x: 0,
                                y: 0,
                                w: self.screen_size.0 as i32,
                                h: self.screen_size.1 as i32,
                            };
                            let scissor = self.scissor_screen();
                            self.flush_with(backend, viewport, scissor, pipeline);
                        } else {
                            let pipeline = self.pipeline_state();
                            let input_comps = self.get_color_input_components();

                            for [x, y, u, v] in vertices {
                                self.push_pos(backend, [x, y, 0.0, 1.0]);
                                if pipeline.uses_textures() {
                                    self.vertex_buffer.extend(&[u, v]);
                                }
                                // Assumes that inputs like shade and fog aren't needed
                                self.push_vertex_color_inputs(
                                    &pipeline,
                                    &input_comps,
                                    &Vertex::default(),
                                    Default::default(),
                                );
                            }

                            self.vertex_buffer_num_tris += 2;
                            self.flush(backend);
                        }
                    }
                    // _ => unimplemented!("{:?}", cmd),
                    _ => {}
                },
                F3DCommand::Unknown(_) => {
                    // unimplemented!("{:?}", cmd)
                }
            }
        }
    }
}

impl From<F3DWrapMode> for WrapMode {
    fn from(m: F3DWrapMode) -> Self {
        if m.clamp {
            WrapMode::Clamp
        } else if m.mirror {
            WrapMode::MirrorRepeat
        } else {
            WrapMode::Repeat
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

fn read_matrix<M: F3DMemory>(memory: &M, ptr: M::Ptr, offset: usize) -> Vec<i32> {
    let mut m = vec![0; 16];
    memory.read_u32(cast_slice_mut(&mut m), ptr, offset);
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

fn read_viewport<M: F3DMemory>(memory: &M, ptr: M::Ptr, offset: usize) -> Viewport {
    let mut v = Viewport::default();
    memory.read_u16(cast_slice_mut(&mut v.scale), ptr, offset);
    memory.read_u16(cast_slice_mut(&mut v.trans), ptr, offset + 8);
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Vertex {
    pos: [i16; 3],
    padding: u16,
    uv: [i16; 2],
    cn: [u8; 4],
}

fn read_vertices<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    offset: usize,
    count: usize,
) -> Vec<Vertex> {
    let stride = mem::size_of::<Vertex>();
    let mut vs = Vec::new();
    for i in 0..count {
        let mut v = Vertex::default();
        let voffset = offset + i * stride;
        memory.read_u16(cast_slice_mut(&mut v.pos), ptr, voffset);
        memory.read_u16(cast_slice_mut(&mut v.uv), ptr, voffset + 8);
        memory.read_u8(cast_slice_mut(&mut v.cn), ptr, voffset + 12);
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

fn read_light<M: F3DMemory>(memory: &M, ptr: M::Ptr) -> Light {
    let mut light = Light::default();
    memory.read_u8(&mut light.color, ptr, 0);
    memory.read_u8(&mut light.color_copy, ptr, 4);
    memory.read_u8(cast_slice_mut(&mut light.dir), ptr, 8);
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

fn read_rgba16<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut rgba16_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut rgba16_data, ptr, 0);

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

fn read_ia16<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut ia16_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut ia16_data, ptr, 0);

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

fn read_ia8<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut ia8_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut ia8_data, ptr, 0);

    let mut rgba32_data: Vec<u8> = Vec::with_capacity(4 * size_bytes as usize);

    for i in 0..size_bytes {
        let i0 = i as usize;
        let intensity = (ia8_data[i0] >> 4) * 0x11;
        let alpha = (ia8_data[i0] & 0xF) * 0x11;
        rgba32_data.extend(&[intensity, intensity, intensity, alpha]);
    }

    TextureRgba32::new(line_size_bytes, size_bytes / line_size_bytes, rgba32_data)
}

fn read_ia4<M: F3DMemory>(
    memory: &M,
    ptr: M::Ptr,
    size_bytes: u32,
    line_size_bytes: u32,
) -> TextureRgba32 {
    let mut ia4_data: Vec<u8> = vec![0; size_bytes as usize];
    memory.read_u8(&mut ia4_data, ptr, 0);

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
