use fast3d::render::F3DRenderer;

use crate::VizRenderData;

use self::{
    data::{PerFrameData, StaticData},
    pipelines::Pipelines,
};

pub(crate) mod data;
mod pipelines;

// TODO: Specify frag_depth as uniform / push constant, combine color_decal.wgsl and
// color.wgsl, use for wall hitboxes instead of calculating by hand

#[derive(Debug)]
pub struct VizRenderer {
    f3d_renderer: F3DRenderer,
    static_data: StaticData,
    pipelines: Pipelines,
    per_frame_data: Option<PerFrameData>,
}

impl VizRenderer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let static_data = StaticData::create(device);
        let pipelines = Pipelines::create(
            device,
            &static_data.transform_bind_group_layout,
            output_format,
        );

        Self {
            f3d_renderer: F3DRenderer::new(device),
            static_data,
            pipelines,
            per_frame_data: None,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        data: &VizRenderData,
    ) {
        self.per_frame_data = None;

        self.f3d_renderer
            .prepare(device, queue, output_format, &data.f3d_render_data);

        if data.render_output.is_some() {
            self.per_frame_data = Some(PerFrameData::create(device, &self.static_data, data));
        }
    }

    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        if let Some(render_data) = &self.per_frame_data {
            self.f3d_renderer
                .render_command_range(rp, render_data.f3d_pre_depth_cmd_range.clone());

            let vx = render_data.screen_top_left[0];
            let vy = render_data.screen_top_left[1];
            let vw = render_data.screen_size[0];
            let vh = render_data.screen_size[1];
            rp.set_viewport(vx as f32, vy as f32, vw as f32, vh as f32, 0.0, 1.0);
            rp.set_scissor_rect(vx, vy, vw, vh);

            rp.set_pipeline(&self.pipelines.surface);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.surface_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.surface_vertex_buffer.0, 0..1);

            rp.set_pipeline(&self.pipelines.line);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.line_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.line_vertex_buffer.0, 0..1);

            rp.set_pipeline(&self.pipelines.point);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.point_instance_buffer.1.slice(..));
            rp.set_vertex_buffer(1, self.static_data.point_vertex_buffer.1.slice(..));
            rp.draw(
                0..self.static_data.point_vertex_buffer.0,
                0..render_data.point_instance_buffer.0,
            );

            self.f3d_renderer
                .render_command_range(rp, render_data.f3d_depth_cmd_range.clone());

            {
                // Render wall hitbox outline first since tris write to z buffer
                rp.set_pipeline(&self.pipelines.line);
                rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
                rp.set_vertex_buffer(0, render_data.wall_hitbox_outline_vertex_buffer.1.slice(..));
                rp.draw(
                    0..render_data.wall_hitbox_outline_vertex_buffer.0 as u32,
                    0..1,
                );

                // When two wall hitboxes overlap, we should not increase the opacity within
                // their region of overlap (preference).
                // First pass writes only to depth buffer to ensure that only the closest
                // hitbox triangles are drawn, then second pass draws them.
                rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
                rp.set_vertex_buffer(0, render_data.wall_hitbox_vertex_buffer.1.slice(..));

                rp.set_pipeline(&self.pipelines.wall_hitbox_depth_pass);
                rp.draw(0..render_data.wall_hitbox_vertex_buffer.0 as u32, 0..1);
                rp.set_pipeline(&self.pipelines.wall_hitbox);
                rp.draw(0..render_data.wall_hitbox_vertex_buffer.0 as u32, 0..1);
            }

            rp.set_pipeline(&self.pipelines.transparent_surface);
            rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
            rp.set_vertex_buffer(0, render_data.transparent_surface_vertex_buffer.1.slice(..));
            rp.draw(0..render_data.transparent_surface_vertex_buffer.0, 0..1);

            self.f3d_renderer
                .render_command_range(rp, render_data.f3d_post_depth_cmd_range.clone());
        } else {
            self.f3d_renderer.render(rp);
        }
    }
}
