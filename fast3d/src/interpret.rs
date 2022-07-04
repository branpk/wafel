//! A display list interpreter that produces a self-contained [F3DRenderData] object that is
//! straightforward to render.
//!
//! After implementing [F3DMemory] for loading vertex/texture/dls/etc data from memory,
//! [interpret_f3d_display_list] can be called.
//!
//! Note: this module is not complete and may have errors.

use core::fmt;
use std::{collections::HashMap, mem};

use derivative::Derivative;

pub use crate::f3d_render_data::*;
use crate::{decode::*, util::*};

/// A trait with methods for reading from game memory.
///
/// This needs to be implemented so that [interpret_f3d_display_list] can read
/// objects from memory (viewports, textures, display lists, etc).
pub trait F3DMemory {
    /// The pointer type that can be read from a display list command.
    type Ptr: fmt::Debug + Copy + PartialEq;
    /// An iterator over a display list that is read from memory.
    type DlIter: Iterator<Item = F3DCommand<Self::Ptr>>;

    /// Returns the top level display list to be interpreted.
    fn root_dl(&self) -> Self::DlIter;
    /// Reads a child display list from memory at the given address.
    fn read_dl(&self, ptr: Self::Ptr) -> Self::DlIter;

    /// Reads dst.len() u8s from memory, starting at ptr + offset (in bytes).
    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize);
    /// Reads dst.len() u16s from memory, starting at ptr + offset (in bytes).
    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize);
    /// Reads dst.len() u32s from memory, starting at ptr + offset (in bytes).
    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize);
}

/// Processes `memory.root_dl()` and returns draw data in a simpler to render and
/// self-contained [F3DRenderData] object.
pub fn interpret_f3d_display_list(
    memory: &impl F3DMemory,
    screen_size: (u32, u32),
    z_is_from_0_to_1: bool,
) -> F3DRenderData {
    let mut interpreter = Interpreter {
        memory: Some(memory),
        screen_size,
        z_is_from_0_to_1,
        ..Default::default()
    };
    interpreter.interpret(memory.root_dl());
    interpreter.finish()
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
struct Interpreter<'m, M: F3DMemory> {
    memory: Option<&'m M>,
    screen_size: (u32, u32),
    z_is_from_0_to_1: bool,
    result: F3DRenderData,

    color_image: Option<Image<M::Ptr>>,
    depth_image: Option<M::Ptr>,

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

    texture_image: Option<Image<M::Ptr>>,
    texture_scale: [[f32; 2]; 8],
    tile_params: [TileParams; 8],
    tile_size: [TileSize; 8],
    texture_memory: HashMap<u32, TextureMemory<M::Ptr>>,

    vertices: Vec<Vertex>,
    vertex_buffer: Vec<f32>,
    num_vertices: u32,
}

#[derive(Debug)]
struct TextureMemory<Ptr> {
    image: Image<Ptr>,
    block: TextureBlock,
    loaded: HashMap<(ImageFormat, ComponentSize), TextureIndex>,
}

impl<'m, M: F3DMemory> Interpreter<'m, M> {
    fn finish(mut self) -> F3DRenderData {
        self.flush();
        self.result
    }

    fn memory(&self) -> &M {
        self.memory.unwrap()
    }

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

    fn flush(&mut self) {
        if self.num_vertices > 0 {
            let pipeline = self.pipeline_state();
            let textures = self.load_textures(&pipeline);
            self.flush_with(
                self.viewport_screen(),
                self.scissor_screen(),
                pipeline,
                textures,
            );
        }
    }

    fn flush_with(
        &mut self,
        viewport: ScreenRectangle,
        scissor: ScreenRectangle,
        pipeline_info: PipelineInfo,
        textures: [Option<TextureIndex>; 2],
    ) {
        let pipeline = PipelineId(pipeline_info);
        self.result
            .pipelines
            .entry(pipeline)
            .or_insert(pipeline_info);

        if self.num_vertices > 0 {
            self.result.commands.push(DrawCommand {
                viewport,
                scissor,
                pipeline,
                textures,
                vertex_buffer: mem::take(&mut self.vertex_buffer),
                num_vertices: mem::take(&mut self.num_vertices),
            });
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

    fn load_textures(&mut self, pipeline: &PipelineInfo) -> [Option<TextureIndex>; 2] {
        [0, 1].map(|i| {
            if pipeline.used_textures[i] {
                Some(self.load_texture(TileIndex(i as u8)))
            } else {
                None
            }
        })
    }

    fn load_texture(&mut self, tile: TileIndex) -> TextureIndex {
        use ComponentSize::*;
        use ImageFormat::*;

        let tile_params = &self.tile_params[tile.0 as usize];
        let tmem = self
            .texture_memory
            .get(&tile_params.tmem)
            .expect("invalid tmem offset");

        let line_size_bytes = tile_params.line * 8;
        let size_bytes = tmem.image.size.num_bits() * (tmem.block.lrs + 1) / 8;

        let fmt = (tile_params.fmt, tile_params.size);
        if let Some(&texture_index) = tmem.loaded.get(&fmt) {
            return texture_index;
        }

        let ptr = tmem.image.img;
        let data = match fmt {
            (Rgba, Bits16) => read_rgba16(self.memory(), ptr, size_bytes, line_size_bytes),
            (Ia, Bits16) => read_ia16(self.memory(), ptr, size_bytes, line_size_bytes),
            (Ia, Bits8) => read_ia8(self.memory(), ptr, size_bytes, line_size_bytes),
            (Ia, Bits4) => read_ia4(self.memory(), ptr, size_bytes, line_size_bytes),
            fmt => unimplemented!("texture format: {:?}", fmt),
            // _ => TextureRgba32::dbg_gradient(),
        };

        let texture = TextureState {
            data,
            sampler: SamplerState {
                u_wrap: tile_params.cms.into(),
                v_wrap: tile_params.cmt.into(),
                linear_filter: self.texture_filter != TextureFilter::Point,
            },
        };

        let texture_index = TextureIndex(self.result.textures.len() as u32);
        self.result.textures.insert(texture_index, texture);

        let tmem = self.texture_memory.get_mut(&tile_params.tmem).unwrap();
        tmem.loaded.insert(fmt, texture_index);

        texture_index
    }

    fn texture_mut(&mut self, texture_index: TextureIndex) -> &mut TextureState {
        self.result.textures.get_mut(&texture_index).unwrap()
    }

    fn transform_pos(&self, vtx: &Vertex) -> [f32; 4] {
        let model_pos = [vtx.pos[0] as f32, vtx.pos[1] as f32, vtx.pos[2] as f32, 1.0];
        &self.proj.cur * (&self.model_view.cur * model_pos)
    }

    fn push_pos(&mut self, mut pos: [f32; 4]) {
        if self.z_is_from_0_to_1 {
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
            } else {
                self.vertex_buffer.push(1.0);
            }
        }
    }

    fn draw_triangle(&mut self, v0: u32, v1: u32, v2: u32) {
        if self.geometry_mode.contains(GeometryModes::CULL_BACK)
            && self.geometry_mode.contains(GeometryModes::CULL_FRONT)
        {
            return;
        }

        let pipeline = self.pipeline_state();
        let input_comps = self.get_color_input_components();

        for vi in [v0, v1, v2] {
            let vtx = self.vertex(vi);

            let pos = self.transform_pos(&vtx);
            self.push_pos(pos);

            if pipeline.uses_textures() {
                let uv = self.calculate_uv(&vtx);
                self.vertex_buffer.extend(&uv);
            }

            self.push_vertex_color_inputs(&pipeline, &input_comps, &vtx, pos);

            self.num_vertices += 1;
        }
    }

    fn fill_rectangle(&mut self, mut rect: Rectangle<u32>) {
        use ColorArg::*;

        if let (Some(color), Some(depth)) = (self.color_image, self.depth_image) {
            if color.img == depth {
                return;
            }
        }

        self.flush();

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

        let x0 = 2.0 * (rect.ulx as f32 / 320.0) - 1.0;
        let x1 = 2.0 * (rect.lrx as f32 / 320.0) - 1.0;
        let y0 = 1.0 - 2.0 * (rect.uly as f32 / 240.0);
        let y1 = 1.0 - 2.0 * (rect.lry as f32 / 240.0);

        let vertices = [[x0, y1], [x1, y1], [x0, y0], [x1, y0], [x0, y0], [x1, y1]];

        for [x, y] in vertices {
            self.push_pos([x, y, 0.0, 1.0]);

            let fill_color = rgba_16_to_32(self.fill_color.0);
            self.vertex_buffer.extend(&[
                fill_color[0] as f32 / 255.0,
                fill_color[1] as f32 / 255.0,
                fill_color[2] as f32 / 255.0,
                fill_color[3] as f32 / 255.0,
            ]);
            self.num_vertices += 1;
        }

        let viewport = ScreenRectangle {
            x: 0,
            y: 0,
            w: self.screen_size.0 as i32,
            h: self.screen_size.1 as i32,
        };
        let scissor = self.scissor_screen();
        let textures = self.load_textures(&pipeline);

        self.flush_with(viewport, scissor, pipeline, textures);
    }

    fn texture_rectangle(&mut self, tex_rect: TextureRectangle) {
        use ColorArg::*;

        if let (Some(color), Some(depth)) = (self.color_image, self.depth_image) {
            if color.img == depth {
                return;
            }
        }

        self.flush();

        let ulx = tex_rect.rect.ulx as f32 / 4.0;
        let uly = tex_rect.rect.uly as f32 / 4.0;
        let mut lrx = tex_rect.rect.lrx as f32 / 4.0;
        let mut lry = tex_rect.rect.lry as f32 / 4.0;

        if matches!(self.cycle_type, CycleType::Fill | CycleType::Copy) {
            lrx += 1.0;
            lry += 1.0;
        }

        let tile = tex_rect.tile;
        self.load_texture(tile);

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
            let texture_index = self.load_texture(tile);

            let sampler = &mut self.texture_mut(texture_index).sampler;
            let saved_linear_filter = sampler.linear_filter;
            sampler.linear_filter = false;

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
                self.push_pos([x, y, 0.0, 1.0]);
                self.vertex_buffer.extend(&[u, v]);
                self.num_vertices += 1;
            }

            let viewport = ScreenRectangle {
                x: 0,
                y: 0,
                w: self.screen_size.0 as i32,
                h: self.screen_size.1 as i32,
            };
            let scissor = self.scissor_screen();

            self.flush_with(viewport, scissor, pipeline, [Some(texture_index), None]);

            let sampler = &mut self.texture_mut(texture_index).sampler;
            sampler.linear_filter = saved_linear_filter;
        } else {
            let pipeline = self.pipeline_state();
            let input_comps = self.get_color_input_components();

            for [x, y, u, v] in vertices {
                self.push_pos([x, y, 0.0, 1.0]);
                if pipeline.uses_textures() {
                    self.vertex_buffer.extend(&[u, v]);
                }
                // Assumes that inputs like shade and fog aren't needed (since they depend on pos)
                self.push_vertex_color_inputs(
                    &pipeline,
                    &input_comps,
                    &Vertex::default(),
                    Default::default(),
                );
                self.num_vertices += 1;
            }

            self.flush();
        }
    }

    fn interpret(&mut self, dl: M::DlIter) {
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
                        self.flush();
                        let fixed = read_matrix(self.memory(), matrix, 0);
                        let m = Matrixf::from_fixed(&fixed);
                        match mode {
                            MatrixMode::Proj => self.proj.execute(m, op, push),
                            MatrixMode::ModelView => self.model_view.execute(m, op, push),
                        }
                    }
                    SPCommand::Viewport(ptr) => {
                        let viewport = read_viewport(self.memory(), ptr, 0);
                        if self.viewport != viewport {
                            self.flush();
                            self.viewport = viewport;
                        }
                    }
                    SPCommand::Light { light, n } => {
                        self.flush();
                        let index = (n - 1) as usize;
                        self.lights[index] = read_light(self.memory(), light);
                    }
                    SPCommand::Vertex { v, n, v0 } => {
                        let offset = v0 as usize * mem::size_of::<Vertex>();
                        self.vertices = read_vertices(self.memory(), v, offset, n as usize);
                    }
                    SPCommand::DisplayList(ptr) => {
                        let child_dl = self.memory().read_dl(ptr);
                        self.interpret(child_dl);
                    }
                    SPCommand::BranchList(ptr) => {
                        let child_dl = self.memory().read_dl(ptr);
                        self.interpret(child_dl);
                        break;
                    }
                    SPCommand::OneTriangle { v0, v1, v2, .. } => {
                        self.draw_triangle(v0, v1, v2);
                    }
                    SPCommand::PopMatrix(mode) => {
                        self.flush();
                        match mode {
                            MatrixMode::Proj => self.proj.pop(),
                            MatrixMode::ModelView => self.model_view.pop(),
                        }
                    }
                    SPCommand::NumLights(n) => {
                        self.flush();
                        self.num_dir_lights = n;
                    }
                    // SPCommand::Segment { seg, base } => todo!(),
                    SPCommand::FogFactor { mul, offset } => {
                        self.flush();
                        self.fog_mul = mul;
                        self.fog_offset = offset;
                    }
                    SPCommand::Texture { sc, tc, tile, .. } => {
                        self.flush();
                        self.texture_scale[tile as usize] =
                            [sc as f32 / 0x10000 as f32, tc as f32 / 0x10000 as f32];
                    }
                    SPCommand::EndDisplayList => break,
                    SPCommand::SetGeometryMode(mode) => {
                        self.flush();
                        self.geometry_mode |= mode;
                    }
                    SPCommand::ClearGeometryMode(mode) => {
                        self.flush();
                        self.geometry_mode &= !mode;
                    }
                    // _ => unimplemented!("{:?}", cmd),
                    _ => {}
                },
                F3DCommand::Rdp(cmd) => match cmd {
                    DPCommand::SetTextureFilter(v) => {
                        if self.texture_filter != v {
                            self.flush();
                            self.texture_filter = v;
                        }
                    }
                    DPCommand::SetCycleType(v) => {
                        if self.cycle_type != v {
                            self.flush();
                            self.cycle_type = v;
                        }
                    }
                    DPCommand::SetAlphaCompare(v) => {
                        if self.alpha_compare != v {
                            self.flush();
                            self.alpha_compare = v;
                        }
                    }
                    DPCommand::SetRenderMode(v) => {
                        if self.render_mode != v {
                            self.flush();
                            self.render_mode = v;
                        }
                    }
                    DPCommand::SetColorImage(image) => {
                        self.flush();
                        self.color_image = Some(image);
                    }
                    DPCommand::SetDepthImage(image) => {
                        self.flush();
                        self.depth_image = Some(image);
                    }
                    DPCommand::SetTextureImage(image) => {
                        self.texture_image = Some(image);
                    }
                    DPCommand::SetCombineMode(mode) => {
                        if self.combine_mode != mode {
                            self.flush();
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
                    DPCommand::FillRectangle(rect) => {
                        self.fill_rectangle(rect);
                    }
                    DPCommand::SetTile(tile, params) => {
                        self.flush();
                        self.tile_params[tile.0 as usize] = params;
                    }
                    DPCommand::LoadBlock(tile, block) => {
                        self.flush();

                        let tile_params = &self.tile_params[tile.0 as usize];
                        let image = self.texture_image.expect("missing call to SetTextureImage");

                        self.texture_memory.insert(
                            tile_params.tmem,
                            TextureMemory {
                                image,
                                block,
                                loaded: HashMap::new(),
                            },
                        );
                    }
                    DPCommand::SetTileSize(tile, size) => {
                        self.flush();
                        self.tile_size[tile.0 as usize] = size;
                    }
                    DPCommand::SetScissor(mode, rect) => {
                        if self.scissor != (mode, rect) {
                            self.flush();
                            self.scissor = (mode, rect);
                        }
                    }
                    DPCommand::FullSync => {}
                    DPCommand::TileSync => {}
                    DPCommand::PipeSync => {}
                    DPCommand::LoadSync => {}
                    DPCommand::TextureRectangle(tex_rect) => {
                        self.texture_rectangle(tex_rect);
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