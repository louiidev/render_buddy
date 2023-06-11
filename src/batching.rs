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
    pub(crate) vert_len: u32,
    pub(crate) indices_len: u32,
    pub(crate) material_handle: Handle<Pipeline>,
    pub(crate) bind_groups: Vec<BindGroup>,
}

impl RenderBuddy {
    pub(crate) fn prepare_mesh_batch(&mut self) -> Vec<PreparedMeshBatch> {
        let mut meshes = self.meshes.drain(0..).collect::<Vec<Mesh>>();

        meshes.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));

        let mut current_batch_texture_handle = Handle::new(ArenaId::default());
        let mut current_material_handle_id = ArenaId::default();
        let mut batches: Vec<Mesh> = Vec::new();

        for mesh in meshes {
            if current_batch_texture_handle == mesh.texture_handle.unwrap_or(Handle::default())
                && current_material_handle_id == mesh.material_handle.id
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
                current_batch_texture_handle = mesh.texture_handle.unwrap_or(Handle::default());
                current_material_handle_id = mesh.material_handle.id;
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
                            contents: &batch
                                .vertices
                                .iter()
                                .map(|v| v.get_bytes())
                                .flatten()
                                .collect::<Vec<u8>>(),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                let index_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(&batch.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                let mut bind_groups = Vec::default();

                let material = self
                    .materials
                    .get(batch.material_handle)
                    .expect("Cant find material for batch");

                if let Some(texture_handle) = batch.texture_handle {
                    if material.material.has_texture() {
                        let texture = self.textures.get(texture_handle).unwrap();
                        let sampler = self.samplers.get(texture.sampler).unwrap();
                        let texture_bind_group = texture.create_bind_group(
                            &self.device,
                            &material.render_pipeline.get_bind_group_layout(1),
                            &sampler,
                        );

                        bind_groups.push(texture_bind_group);
                    }
                }

                let mut mat_bind_groups =
                    material
                        .material
                        .get_bind_groups(&batch, &self, &material.render_pipeline);

                bind_groups.append(&mut mat_bind_groups);

                PreparedMeshBatch {
                    vertex_buffer,
                    index_buffer,
                    bind_groups,
                    vert_len: batch.vertices.len() as _,
                    indices_len: batch.indices.len() as _,
                    material_handle: batch.material_handle,
                }
            })
            .collect()
    }
}
