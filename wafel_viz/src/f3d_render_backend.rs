use crate::{
    f3d_render_data::*,
    render_api::{decode_shader_id, RenderBackend, ShaderId, ShaderInfo, ShaderItem},
};

#[derive(Debug, Default)]
pub struct F3DRenderBackend {
    viewport: ScreenRectangle,
    scissor: ScreenRectangle,
    state: RenderState,
    tile: usize,
    texture_index: [Option<TextureIndex>; 2],
    data: F3DRenderData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct RenderState {
    shader_id: Option<u32>,
    depth_test: bool,
    depth_mask: bool,
    zmode_decal: bool,
    use_alpha: bool,
    cull_mode: CullMode,
}

impl F3DRenderBackend {
    pub fn finish(self) -> F3DRenderData {
        self.data
    }
}

impl RenderBackend for F3DRenderBackend {
    fn z_is_from_0_to_1(&self) -> bool {
        true
    }

    fn unload_shader(&mut self, _old_prg: ShaderId) {
        self.state.shader_id = None;
    }

    fn load_shader(&mut self, new_prg: ShaderId) {
        let shader_id = new_prg.0 as u32;
        self.state.shader_id = Some(shader_id);
    }

    fn create_and_load_new_shader(&mut self, shader_id: u32) -> ShaderId {
        let id = ShaderId(shader_id as usize);
        self.load_shader(id);
        id
    }

    fn lookup_shader(&self, shader_id: u32) -> Option<ShaderId> {
        Some(ShaderId(shader_id as usize))
    }

    fn shader_get_info(&self, prg: ShaderId) -> ShaderInfo {
        let shader_id = prg.0 as u32;
        let features = decode_shader_id(shader_id);
        ShaderInfo {
            num_inputs: features.num_inputs as u8,
            used_textures: features.used_textures,
        }
    }

    fn new_texture(&mut self) -> u32 {
        let index = TextureIndex(self.data.textures.len() as u32);
        self.data.textures.insert(index, TextureState::default());
        index.0
    }

    fn select_texture(&mut self, tile: i32, texture_id: u32) {
        let texture_index = TextureIndex(texture_id);
        assert!(
            self.data.textures.contains_key(&texture_index),
            "invalid texture id"
        );
        self.tile = tile as usize;
        self.texture_index[self.tile] = Some(texture_index);
    }

    fn upload_texture(&mut self, rgba32_buf: &[u8], width: i32, height: i32) {
        assert!(4 * width * height <= rgba32_buf.len() as i32);
        let texture_index = self.texture_index[self.tile].expect("no selected texture");
        let texture = self.data.textures.entry(texture_index).or_default();
        texture.data = TextureData {
            width: width as u32,
            height: height as u32,
            rgba8: rgba32_buf.to_vec(),
        };
    }

    fn set_sampler_parameters(&mut self, tile: i32, linear_filter: bool, cms: u32, cmt: u32) {
        self.tile = tile as usize;
        let texture_index = self.texture_index[self.tile].expect("no selected texture");
        let texture = self.data.textures.entry(texture_index).or_default();

        let wrap_mode = |v| {
            if v & 0x2 != 0 {
                WrapMode::Clamp
            } else if v & 0x1 != 0 {
                WrapMode::MirrorRepeat
            } else {
                WrapMode::Repeat
            }
        };

        texture.sampler = SamplerState {
            u_wrap: wrap_mode(cms),
            v_wrap: wrap_mode(cmt),
            linear_filter,
        };
    }

    fn set_depth_test(&mut self, depth_test: bool) {
        self.state.depth_test = depth_test;
    }

    fn set_depth_mask(&mut self, z_upd: bool) {
        self.state.depth_mask = z_upd;
    }

    fn set_zmode_decal(&mut self, zmode_decal: bool) {
        self.state.zmode_decal = zmode_decal;
    }

    fn set_viewport(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.viewport = ScreenRectangle {
            x,
            y,
            w: width,
            h: height,
        };
    }

    fn set_scissor(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.scissor = ScreenRectangle {
            x,
            y,
            w: width,
            h: height,
        };
    }

    fn set_use_alpha(&mut self, use_alpha: bool) {
        self.state.use_alpha = use_alpha;
    }

    fn set_cull_mode(&mut self, cull_mode: CullMode) {
        self.state.cull_mode = cull_mode;
    }

    fn draw_triangles(&mut self, buf_vbo: &[f32], buf_vbo_num_tris: usize) {
        let cc_features = decode_shader_id(self.state.shader_id.expect("missing shader id"));

        let shader_item_to_input = |item: ShaderItem| match item {
            ShaderItem::Zero => ColorArg::Zero,
            ShaderItem::Input1 => ColorArg::Input(0),
            ShaderItem::Input2 => ColorArg::Input(1),
            ShaderItem::Input3 => ColorArg::Input(2),
            ShaderItem::Input4 => ColorArg::Input(3),
            ShaderItem::Texel0 => ColorArg::Texel0,
            ShaderItem::Texel0A => ColorArg::Texel0Alpha,
            ShaderItem::Texel1 => ColorArg::Texel1,
        };
        // TODO: Ensure that if opt_alpha is false, the color expr handles it correctly
        let output_color = ColorExpr {
            rgb: cc_features.c[0].map(shader_item_to_input),
            a: cc_features.c[1].map(shader_item_to_input),
        };

        let pipeline_state = PipelineInfo {
            cull_mode: self.state.cull_mode,
            depth_compare: self.state.depth_test,
            depth_write: self.state.depth_mask,
            blend: self.state.use_alpha,
            decal: self.state.zmode_decal,
            used_textures: cc_features.used_textures,
            texture_edge: cc_features.opt_texture_edge,
            fog: cc_features.opt_fog,
            num_inputs: cc_features.num_inputs,
            output_color,
        };
        let pipeline = PipelineId {
            state: pipeline_state,
        };

        self.data.pipelines.insert(pipeline, pipeline_state);

        self.data.commands.push(DrawCommand {
            viewport: self.viewport,
            scissor: self.scissor,
            pipeline,
            textures: self.texture_index,
            vertex_buffer: buf_vbo.to_vec(),
            num_vertices: 3 * buf_vbo_num_tris as u32,
        });
    }

    fn on_resize(&mut self) {}

    fn start_frame(&mut self) {}

    fn end_frame(&mut self) {}

    fn finish_render(&mut self) {}
}
