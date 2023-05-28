use std::fmt::Debug;

use wgpu::{
    BindGroupLayout, BlendState, FragmentState, FrontFace, PolygonMode, PrimitiveState,
    RenderPipeline, RenderPipelineDescriptor, VertexBufferLayout, VertexState, VertexStepMode,
};

use crate::{material::Material, mesh::get_attribute_layout, RenderBuddy};

pub struct Pipeline {
    pub(crate) render_pipeline: RenderPipeline,
    pub(crate) material: Box<dyn Material>,
}

impl Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("render_pipeline", &self.render_pipeline)
            .field("material", &self.material)
            .finish()
    }
}

impl RenderBuddy {
    pub(crate) fn create_pipeline_from_material(
        &mut self,
        material: &impl Material,
    ) -> RenderPipeline {
        let shader = self.device.create_shader_module(material.shader());

        let vertex_attributes = material.vertex_attributes();

        let (vertex_attribute, offset) = get_attribute_layout(vertex_attributes.iter());

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: offset as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vertex_attribute.as_slice(),
        };

        let bind_group_layouts: Vec<BindGroupLayout> =
            material.get_bind_group_layouts(&self.device);

        let mut predefined_bind_group_layouts = if material.has_texture() {
            vec![
                &self.camera_bind_group_layout,
                &self.texture_bind_group_layout,
            ]
        } else {
            vec![&self.camera_bind_group_layout]
        };
        predefined_bind_group_layouts.append(&mut bind_group_layouts.iter().map(|x| &*x).collect());

        dbg!(&predefined_bind_group_layouts);

        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Material Pipeline Layout"),
                    bind_group_layouts: predefined_bind_group_layouts.as_slice(),
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
                buffers: &[vertex_buffer_layout],
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
                topology: material.topology(),
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            label: Some(material.label()),
            multiview: None,
        };

        self.device.create_render_pipeline(&descriptor)
    }
}
