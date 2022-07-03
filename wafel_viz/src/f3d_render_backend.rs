use crate::{
    f3d_render_data::*,
    render_api::{decode_shader_id, ShaderId, ShaderItem},
};

#[derive(Debug, Default)]
pub struct F3DRenderBackend {
    viewport: ScreenRectangle,
    scissor: ScreenRectangle,
    shader_id: Option<u32>,
    depth_test: bool,
    depth_mask: bool,
    zmode_decal: bool,
    use_alpha: bool,
    cull_mode: CullMode,
    texture_index: [Option<TextureIndex>; 2],
    data: F3DRenderData,
}

impl F3DRenderBackend {
    pub fn finish(self) -> F3DRenderData {
        self.data
    }

    pub fn z_is_from_0_to_1(&self) -> bool {
        true
    }

    pub fn load_shader(&mut self, new_prg: ShaderId) {
        let shader_id = new_prg.0 as u32;
        self.shader_id = Some(shader_id);
    }

    pub fn new_texture(&mut self) -> u32 {
        let index = TextureIndex(self.data.textures.len() as u32);
        self.data.textures.insert(index, TextureState::default());
        index.0
    }

    pub fn select_texture(&mut self, tile: i32, texture_id: u32) {
        let texture_index = TextureIndex(texture_id);
        assert!(
            self.data.textures.contains_key(&texture_index),
            "invalid texture id"
        );
        self.texture_index[tile as usize] = Some(texture_index);
    }

    pub fn upload_texture(&mut self, tile: i32, rgba32_buf: &[u8], width: i32, height: i32) {
        assert!(4 * width * height <= rgba32_buf.len() as i32);
        let texture_index = self.texture_index[tile as usize].expect("no selected texture");
        let texture = self.data.textures.entry(texture_index).or_default();
        texture.data = TextureData {
            width: width as u32,
            height: height as u32,
            rgba8: rgba32_buf.to_vec(),
        };
    }

    pub fn set_sampler_parameters(&mut self, tile: i32, sampler: SamplerState) {
        let texture_index = self.texture_index[tile as usize].expect("no selected texture");
        let texture = self.data.textures.entry(texture_index).or_default();
        texture.sampler = sampler;
    }

    pub fn set_depth_test(&mut self, depth_test: bool) {
        self.depth_test = depth_test;
    }

    pub fn set_depth_mask(&mut self, z_upd: bool) {
        self.depth_mask = z_upd;
    }

    pub fn set_zmode_decal(&mut self, zmode_decal: bool) {
        self.zmode_decal = zmode_decal;
    }

    pub fn set_viewport(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.viewport = ScreenRectangle {
            x,
            y,
            w: width,
            h: height,
        };
    }

    pub fn set_scissor(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.scissor = ScreenRectangle {
            x,
            y,
            w: width,
            h: height,
        };
    }

    pub fn set_use_alpha(&mut self, use_alpha: bool) {
        self.use_alpha = use_alpha;
    }

    pub fn set_cull_mode(&mut self, cull_mode: CullMode) {
        self.cull_mode = cull_mode;
    }

    pub fn pipeline_state(&self) -> PipelineInfo {
        let cc_features = decode_shader_id(self.shader_id.expect("missing shader id"));

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

        PipelineInfo {
            cull_mode: self.cull_mode,
            depth_compare: self.depth_test,
            depth_write: self.depth_mask,
            blend: self.use_alpha,
            decal: self.zmode_decal,
            used_textures: cc_features.used_textures,
            texture_edge: cc_features.opt_texture_edge,
            fog: cc_features.opt_fog,
            num_inputs: cc_features.num_inputs,
            output_color,
        }
    }

    pub fn draw_triangles(&mut self, buf_vbo: &[f32], buf_vbo_num_tris: usize) {
        let pipeline_state = self.pipeline_state();
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
}
