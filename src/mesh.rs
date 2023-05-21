use glam::Vec2;

use crate::{
    arena::{ArenaId, Handle},
    pipeline::Pipeline,
    texture::Texture,
    transform::Transform,
    RenderBuddy,
};

pub const QUAD_INDICES: [u16; 6] = [0, 2, 3, 0, 1, 2];

pub const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];

pub const QUAD_UVS: [Vec2; 4] = [
    Vec2::new(0., 1.),
    Vec2::new(1., 1.),
    Vec2::new(1., 0.),
    Vec2::new(0., 0.),
];

pub trait MeshBuilder {
    fn build(&self, transform: Transform, rb: &RenderBuddy) -> Mesh;
}

pub trait BatchMeshBuild {
    fn build(&self, transform: Transform, rb: &mut RenderBuddy) -> Vec<Mesh>;
}

#[derive(Debug)]
pub struct Mesh {
    pub(crate) handle: Handle<Texture>,
    pub(crate) pipeline_handle: Handle<Pipeline>,
    pub(crate) vertices: Vec<Vertex2D>,
    pub(crate) indices: Vec<u16>,
    // used for sorting
    pub(crate) z: f32,
}
impl Mesh {
    pub fn new(
        handle: Handle<Texture>,
        vertices: Vec<Vertex2D>,
        indices: Vec<u16>,
        z: f32,
    ) -> Self {
        Self {
            pipeline_handle: Handle::new(ArenaId::first()),
            handle,
            vertices,
            indices,
            z,
        }
    }

    pub fn concat(&mut self, mut vertices: Vec<Vertex2D>, mut indices: Vec<u16>) {
        self.vertices.append(&mut vertices);
        self.indices.append(&mut indices);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex2D {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex2D {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex2D>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 5]>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
