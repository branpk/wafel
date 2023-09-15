use crate::ColorVertex;

use super::data::{PointInstance, PointVertex};

#[derive(Debug)]
pub struct Pipelines {
    pub line: wgpu::RenderPipeline,
    pub point: wgpu::RenderPipeline,
    pub surface: wgpu::RenderPipeline,
    pub transparent_surface: wgpu::RenderPipeline,
    pub wall_hitbox: wgpu::RenderPipeline,
    pub wall_hitbox_depth_pass: wgpu::RenderPipeline,
}

impl Pipelines {
    pub fn create(
        device: &wgpu::Device,
        transform_bind_group_layout: &wgpu::BindGroupLayout,
        output_format: wgpu::TextureFormat,
        msaa_samples: u32,
    ) -> Self {
        let line = create_line_pipeline(
            device,
            transform_bind_group_layout,
            output_format,
            msaa_samples,
        );
        let point = create_point_pipeline(
            device,
            transform_bind_group_layout,
            output_format,
            msaa_samples,
        );

        let surface = create_surface_pipeline(
            device,
            transform_bind_group_layout,
            output_format,
            true,
            msaa_samples,
        );
        let transparent_surface = create_surface_pipeline(
            device,
            transform_bind_group_layout,
            output_format,
            false,
            msaa_samples,
        );

        let wall_hitbox = create_color_pipeline(
            device,
            transform_bind_group_layout,
            output_format,
            true,
            true,
            true,
            wgpu::PrimitiveTopology::TriangleList,
            msaa_samples,
        );
        let wall_hitbox_depth_pass = create_color_pipeline(
            device,
            transform_bind_group_layout,
            output_format,
            false,
            true,
            true,
            wgpu::PrimitiveTopology::TriangleList,
            msaa_samples,
        );

        Self {
            line,
            point,
            surface,
            transparent_surface,
            wall_hitbox,
            wall_hitbox_depth_pass,
        }
    }
}

fn create_line_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    msaa_samples: u32,
) -> wgpu::RenderPipeline {
    let shader_module =
        device.create_shader_module(wgpu::include_wgsl!("../../shaders/color_decal.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-line"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[ColorVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: msaa_samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_point_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    msaa_samples: u32,
) -> wgpu::RenderPipeline {
    let shader_module =
        device.create_shader_module(wgpu::include_wgsl!("../../shaders/point.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-point"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[PointInstance::layout(), PointVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: msaa_samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_surface_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    depth_write_enabled: bool,
    msaa_samples: u32,
) -> wgpu::RenderPipeline {
    let shader_module =
        device.create_shader_module(wgpu::include_wgsl!("../../shaders/surface.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-surface"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[ColorVertex::layout()],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: msaa_samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_color_pipeline(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    color_write_enabled: bool,
    depth_write_enabled: bool,
    depth_compare_enabled: bool,
    topology: wgpu::PrimitiveTopology,
    msaa_samples: u32,
) -> wgpu::RenderPipeline {
    let shader_module =
        device.create_shader_module(wgpu::include_wgsl!("../../shaders/color.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("viz-color"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[transform_bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[ColorVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled,
            depth_compare: if depth_compare_enabled {
                wgpu::CompareFunction::LessEqual
            } else {
                wgpu::CompareFunction::Always
            },
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: msaa_samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: if color_write_enabled {
                    wgpu::ColorWrites::ALL
                } else {
                    wgpu::ColorWrites::empty()
                },
            })],
        }),
        multiview: None,
    })
}
