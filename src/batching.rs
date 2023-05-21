use crate::{
    arena::{ArenaId, Handle},
    mesh::Mesh,
    pipeline::Pipeline,
    RenderBuddy,
};
use wgpu::{util::DeviceExt, BindGroup, Buffer};

#[derive(Debug)]
pub(crate) struct PreparedMeshBatch {
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
    pub(crate) texture_bind_group: BindGroup,
    pub(crate) indices_len: u32,
    pub(crate) pipeline_handle: Handle<Pipeline>,
}

impl RenderBuddy {
    pub(crate) fn prepare_mesh_batch(&mut self) -> Vec<PreparedMeshBatch> {
        let mut meshes = self.meshes.drain(0..).collect::<Vec<Mesh>>();

        meshes.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));

        let mut current_batch_texture_id = ArenaId::default();
        let mut current_pipeline_id = ArenaId::default();
        let mut batches: Vec<Mesh> = Vec::new();

        for mesh in meshes {
            if current_batch_texture_id == mesh.handle.id
                && current_pipeline_id == mesh.pipeline_handle.id
            {
                let length = batches.len();

                let current_mesh = &mut batches[length - 1];
                let vert_count = current_mesh.vertices.len() as u16;
                let indices = mesh
                    .indices
                    .iter()
                    .map(|index| index + vert_count)
                    .collect::<Vec<u16>>();

                current_mesh.concat(mesh.vertices, indices);
            } else {
                current_batch_texture_id = mesh.handle.id;
                current_pipeline_id = mesh.pipeline_handle.id;
                batches.push(mesh);
            }
        }

        batches
            .iter()
            .map(|batch| {
                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&batch.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                let index_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(&batch.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                let texture = self.textures.get(batch.handle).unwrap();

                let sampler = self.texture_samplers.get(&texture.sampler).unwrap();

                let pipeline = self
                    .cached_pipelines
                    .get(batch.pipeline_handle)
                    .expect("Pipeline is missing for mesh");

                let texture_bind_group =
                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &pipeline.render_pipeline.get_bind_group_layout(1),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(sampler),
                            },
                        ],
                        label: None,
                    });

                PreparedMeshBatch {
                    vertex_buffer,
                    index_buffer,
                    texture_bind_group,
                    indices_len: batch.indices.len() as _,
                    pipeline_handle: batch.pipeline_handle,
                }
            })
            .collect()
    }
}
