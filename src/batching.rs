use crate::arena::ArenaId;
use wgpu::{BindGroup, Buffer};

#[derive(Debug)]
pub(crate) struct PreparedMeshBatch {
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
    pub(crate) texture_bind_group: BindGroup,
    pub(crate) indices_len: u32,
    pub(crate) pipeline_id: ArenaId,
}
