use wgpu::{CommandEncoder, SurfaceTexture, TextureView};

pub struct RenderContext {
    pub(crate) output: SurfaceTexture,
    pub(crate) view: TextureView,
    pub(crate) command_encoder: CommandEncoder,
}
