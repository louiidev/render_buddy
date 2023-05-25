use wgpu::{
    include_wgsl, BlendState, FragmentState, FrontFace, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, VertexState,
};

use crate::{
    arena::{ArenaId, Handle},
    mesh::Vertex2D,
    RenderBuddy,
};

#[derive(Debug)]
pub struct Pipeline {
    pub(crate) render_pipeline: RenderPipeline,
}

pub struct PipelineMap {
    default: ArenaId,
    lines: ArenaId,
}

impl RenderBuddy {
    pub(crate) fn create_default_pipeline(&mut self) -> Handle<Pipeline> {
        let shader = self
            .device
            .create_shader_module(include_wgsl!("./default_shaders/2d.wgsl"));

        let texture_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("2D Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &self.camera_bind_group_layout,
                        &texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let binding: [Option<wgpu::ColorTargetState>; 1] = [Some(wgpu::ColorTargetState {
            format: self.surface_config.format,
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                entry_point: "vertex",
                buffers: &[Vertex2D::desc()],
                module: &shader,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &binding,
            }),
            layout: Some(&render_pipeline_layout),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            label: Some("default_pipeline"),
            multiview: None,
        };

        self.cached_pipelines.insert(Pipeline {
            render_pipeline: self.device.create_render_pipeline(&descriptor),
        })
    }
}
