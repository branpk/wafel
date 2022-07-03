use crate::f3d_render_data::*;

#[derive(Debug, Default)]
pub struct F3DRenderBackend {
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

    pub fn draw_triangles(
        &mut self,
        viewport: ScreenRectangle,
        scissor: ScreenRectangle,
        pipeline_info: PipelineInfo,
        buf_vbo: &[f32],
        buf_vbo_num_tris: usize,
    ) {
        let pipeline = PipelineId {
            state: pipeline_info,
        };
        self.data.pipelines.insert(pipeline, pipeline_info);

        self.data.commands.push(DrawCommand {
            viewport,
            scissor,
            pipeline,
            textures: self.texture_index,
            vertex_buffer: buf_vbo.to_vec(),
            num_vertices: 3 * buf_vbo_num_tris as u32,
        });
    }
}
