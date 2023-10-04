use std::collections::HashMap;

use fast3d::render::F3DRenderer;
use wafel_viz::VizScene;

use crate::{
    data::{BufferId, PerFrameData, StaticData, TriangleTransparency},
    pipelines::{create_pipelines, PipelineId},
};

// TODO: Specify frag_depth as uniform / push constant, combine color_decal.wgsl and
// color.wgsl, use for wall hitboxes instead of calculating by hand

/// A wgpu renderer for [VizScene].
#[derive(Debug)]
pub struct VizRenderer {
    f3d_renderer: F3DRenderer,
    static_data: StaticData,
    pipelines: HashMap<PipelineId, wgpu::RenderPipeline>,
    per_frame_data: Option<PerFrameData>,
}

impl VizRenderer {
    /// Constructs a new [VizRenderer].
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        msaa_samples: u32,
    ) -> Self {
        let static_data = StaticData::create(device);
        let pipelines = create_pipelines(
            device,
            &static_data.transform_bind_group_layout,
            output_format,
            msaa_samples,
        );

        Self {
            f3d_renderer: F3DRenderer::new(device, msaa_samples),
            static_data,
            pipelines,
            per_frame_data: None,
        }
    }

    /// This should be called with a [VizScene] before [Self::render] is called.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        output_size_physical: [u32; 2],
        scale_factor: f32,
        scene: &VizScene,
    ) {
        self.per_frame_data = None;

        let viewport_top_left_logical = scene.viewport_top_left_logical.unwrap_or([0, 0]);
        let viewport_size_logical = scene.viewport_size_logical.unwrap_or([
            (output_size_physical[0] as f32 / scale_factor) as u32 - viewport_top_left_logical[0],
            (output_size_physical[1] as f32 / scale_factor) as u32 - viewport_top_left_logical[1],
        ]);

        if let Some(f3d_render_data) = &scene.f3d_render_data {
            assert_eq!(
                [f3d_render_data.screen_top_left, f3d_render_data.screen_size],
                [viewport_top_left_logical, viewport_size_logical],
                "F3D screen size must match VizScene viewport size"
            );

            self.f3d_renderer
                .prepare(device, queue, output_format, f3d_render_data);
        }

        self.per_frame_data = Some(PerFrameData::create(
            device,
            &self.static_data,
            scene,
            viewport_top_left_logical,
            viewport_size_logical,
            scale_factor,
        ));
    }

    fn draw_buffer<'r>(
        &'r self,
        rp: &mut wgpu::RenderPass<'r>,
        render_data: &'r PerFrameData,
        pipeline_id: PipelineId,
        buffer_id: BufferId,
    ) {
        if let Some((count, buffer)) = &render_data.buffers[buffer_id] {
            let pipeline = self.pipelines.get(&pipeline_id).expect("missing pipeline");

            if matches!(buffer_id, BufferId::Point { .. }) {
                // Points use instanced rendering.
                rp.set_pipeline(pipeline);
                rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
                rp.set_vertex_buffer(0, buffer.slice(..));
                rp.set_vertex_buffer(1, self.static_data.point_vertex_buffer.1.slice(..));
                rp.draw(0..self.static_data.point_vertex_buffer.0, 0..*count);
            } else {
                rp.set_pipeline(pipeline);
                rp.set_bind_group(0, &render_data.transform_bind_group, &[]);
                rp.set_vertex_buffer(0, buffer.slice(..));
                rp.draw(0..*count, 0..1);
            }
        }
    }

    /// Renders a [VizScene] that was provided to [Self::prepare].
    pub fn render<'r>(&'r self, rp: &mut wgpu::RenderPass<'r>) {
        let render_data = self
            .per_frame_data
            .as_ref()
            .expect("missing call to VizRenderer::prepare");
        let scale_factor = render_data.scale_factor;

        // Execute F3D commands which are prior to enabling depth test (e.g. skybox).
        self.f3d_renderer.render_command_range(
            rp,
            render_data.f3d_pre_depth_cmd_range.clone(),
            scale_factor,
        );

        // Set the viewport and scissor rect.
        let vx = render_data.viewport_top_left[0];
        let vy = render_data.viewport_top_left[1];
        let vw = render_data.viewport_size[0];
        let vh = render_data.viewport_size[1];
        rp.set_viewport(
            (vx as f32) * scale_factor,
            (vy as f32) * scale_factor,
            (vw as f32) * scale_factor,
            (vh as f32) * scale_factor,
            0.0,
            1.0,
        );
        rp.set_scissor_rect(
            ((vx as f32) * scale_factor) as u32,
            ((vy as f32) * scale_factor) as u32,
            ((vw as f32) * scale_factor) as u32,
            ((vh as f32) * scale_factor) as u32,
        );

        // Draw opaque triangles, lines, then points.
        for surface_gradient in [true, false] {
            self.draw_buffer(
                rp,
                render_data,
                PipelineId::Triangle {
                    surface_gradient,
                    depth_write: true,
                    color_write: true,
                },
                BufferId::Triangle {
                    transparency: TriangleTransparency::Opaque,
                    surface_gradient,
                },
            );
        }
        self.draw_buffer(
            rp,
            render_data,
            PipelineId::Line,
            BufferId::Line { transparent: false },
        );
        self.draw_buffer(
            rp,
            render_data,
            PipelineId::Point,
            BufferId::Point { transparent: false },
        );

        // Execute F3D commands which have depth test enabled.
        self.f3d_renderer.render_command_range(
            rp,
            render_data.f3d_depth_cmd_range.clone(),
            scale_factor,
        );

        // Draw transparent points and lines with depth test and write enabled.
        self.draw_buffer(
            rp,
            render_data,
            PipelineId::Line,
            BufferId::Line { transparent: true },
        );
        self.draw_buffer(
            rp,
            render_data,
            PipelineId::Point,
            BufferId::Point { transparent: true },
        );

        // Render wall hitboxes before other transparent triangles.
        // When two wall hitboxes overlap, we should not increase the opacity
        // within their region of overlap (preference).
        // The first pass writes only to the depth buffer to ensure that only
        // the closest hitbox triangles are drawn, then the second pass draws
        // them.
        for color_write in [false, true] {
            for surface_gradient in [false, true] {
                self.draw_buffer(
                    rp,
                    render_data,
                    PipelineId::Triangle {
                        surface_gradient,
                        depth_write: true,
                        color_write,
                    },
                    BufferId::Triangle {
                        transparency: TriangleTransparency::TransparentWallHitbox,
                        surface_gradient,
                    },
                );
            }
        }

        // Render remaining transparent triangles.
        // These will not be visible through wall hitboxes (which is fine
        // because wall hitboxes are small), but wall hitboxes will be visible
        // through them which is more important.
        // These are rendered in the order they were added to the scene.
        for surface_gradient in [false, true] {
            self.draw_buffer(
                rp,
                render_data,
                PipelineId::Triangle {
                    surface_gradient,
                    depth_write: false,
                    color_write: true,
                },
                BufferId::Triangle {
                    transparency: TriangleTransparency::Transparent,
                    surface_gradient,
                },
            );
        }

        // Render post depth F3D commands (e.g. the HUD).
        self.f3d_renderer.render_command_range(
            rp,
            render_data.f3d_post_depth_cmd_range.clone(),
            scale_factor,
        );
    }
}
