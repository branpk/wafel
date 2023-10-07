use std::collections::HashMap;

use fast3d::render::F3DRenderer;
use wafel_viz::{Rect2, Vec2, Viewport, VizScene};

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

        let output_size_logical = Vec2::from(output_size_physical.map(|x| x as f32 / scale_factor));

        let vieport = match scene.viewport {
            Viewport::FullWindow => Rect2::from_min_and_size(Vec2::zero(), output_size_logical),
            Viewport::RectLogical(rect) => rect,
        };

        if let Some(f3d_render_data) = &scene.f3d_render_data {
            self.f3d_renderer.prepare(
                device,
                queue,
                output_format,
                output_size_physical,
                scale_factor,
                f3d_render_data,
            );
        }

        self.per_frame_data = Some(PerFrameData::create(
            device,
            &self.static_data,
            scene,
            output_size_logical,
            vieport,
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

        if !render_data.viewport.has_positive_area() {
            return;
        }

        // Execute F3D commands which are prior to enabling depth test (e.g. skybox).
        self.f3d_renderer
            .render_command_range(rp, render_data.f3d_pre_depth_cmd_range.clone());

        // Set the viewport and scissor rect.
        let scaled_viewport = render_data.viewport.scale(scale_factor);
        rp.set_viewport(
            scaled_viewport.min_x(),
            scaled_viewport.min_y(),
            scaled_viewport.size_x(),
            scaled_viewport.size_y(),
            0.0,
            1.0,
        );

        // Clamp scissor rect to the output window.
        let output_rect = Rect2::from_min_and_size(Vec2::zero(), render_data.output_size);
        let scissor_rect = render_data.viewport.clamp(output_rect).scale(scale_factor);
        if !scissor_rect.has_positive_area() {
            return;
        }
        rp.set_scissor_rect(
            scissor_rect.min_x() as u32,
            scissor_rect.min_y() as u32,
            scissor_rect.size_x() as u32,
            scissor_rect.size_y() as u32,
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
        self.f3d_renderer
            .render_command_range(rp, render_data.f3d_depth_cmd_range.clone());

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
        self.f3d_renderer
            .render_command_range(rp, render_data.f3d_post_depth_cmd_range.clone());
    }
}
