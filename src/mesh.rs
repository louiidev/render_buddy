use std::{collections::BTreeMap, mem};

use bytemuck::cast_slice;
use glam::{Vec2, Vec3};
use wgpu::{VertexAttribute, VertexFormat};

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

pub const CUBE_VERTEX_UVS: [Vec2; 36] = [
    Vec2::new(0.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(0.0, 0.0),
    Vec2::new(0.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(0.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(0.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(0.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, 0.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, 0.0),
    Vec2::new(0.0, 1.0),
];

pub const CUBE_VERTEX_POSITIONS: [Vec3; 36] = [
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(0.5, -0.5, -0.5),
    Vec3::new(0.5, 0.5, -0.5),
    Vec3::new(0.5, 0.5, -0.5),
    Vec3::new(-0.5, 0.5, -0.5),
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(-0.5, -0.5, 0.5),
    Vec3::new(0.5, -0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(-0.5, 0.5, 0.5),
    Vec3::new(-0.5, -0.5, 0.5),
    Vec3::new(-0.5, 0.5, 0.5),
    Vec3::new(-0.5, 0.5, -0.5),
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(-0.5, -0.5, 0.5),
    Vec3::new(-0.5, 0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(0.5, 0.5, -0.5),
    Vec3::new(0.5, -0.5, -0.5),
    Vec3::new(0.5, -0.5, -0.5),
    Vec3::new(0.5, -0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(0.5, -0.5, -0.5),
    Vec3::new(0.5, -0.5, 0.5),
    Vec3::new(0.5, -0.5, 0.5),
    Vec3::new(-0.5, -0.5, 0.5),
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(-0.5, 0.5, -0.5),
    Vec3::new(0.5, 0.5, -0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(-0.5, 0.5, 0.5),
    Vec3::new(-0.5, 0.5, -0.5),
];

pub trait MeshCreator {
    fn build(&self, transform: Transform, rb: &RenderBuddy) -> Mesh;
}

pub trait BatchMeshCreator {
    fn build(&self, transform: Transform, rb: &mut RenderBuddy) -> Vec<Mesh>;
}

pub struct MeshBuilder {
    pub(crate) texture_handle: Option<Handle<Texture>>,
    pub(crate) material_handle: Handle<Pipeline>,
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u16>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self {
            texture_handle: None,
            material_handle: Handle::new(ArenaId::first()), // TODO(loui): we should really ref the default material but its hard without a ref to rb
            vertices: Vec::default(),
            indices: Vec::default(),
        }
    }

    pub fn quad(size: Vec2, transform: Transform) -> Self {
        MeshBuilder::new()
            .with_attributes(
                MeshAttribute::Position,
                QUAD_VERTEX_POSITIONS
                    .iter()
                    .map(|v| {
                        AttributeValue::Position(
                            transform.transform_point((*v * size).extend(0.)).into(),
                        )
                    })
                    .collect(),
            )
            .with_indices(&QUAD_INDICES)
    }

    pub fn cube(size: Vec3, transform: Transform) -> Self {
        MeshBuilder::new()
            .with_attributes(
                MeshAttribute::Position,
                CUBE_VERTEX_POSITIONS
                    .iter()
                    .map(|v| AttributeValue::Position(transform.transform_point(*v * size).into()))
                    .collect(),
            )
            .with_attributes(
                MeshAttribute::UV,
                CUBE_VERTEX_UVS
                    .iter()
                    .map(|uv| AttributeValue::UV(uv.to_array()))
                    .collect(),
            )
    }

    pub fn with_vertices(mut self, vertices: Vec<Vertex>) -> Self {
        self.vertices = vertices;

        self
    }

    pub fn with_attributes(
        mut self,
        attribute: MeshAttribute,
        values: Vec<AttributeValue>,
    ) -> Self {
        if self.vertices.is_empty() {
            self.vertices = values
                .iter()
                .map(|v| Vertex::new().with_attribute(attribute, *v))
                .collect();
        } else {
            for i in 0..values.len() {
                let vert = &mut self.vertices[i];
                vert.set_attribute(attribute, values[i]);
            }
        }

        self
    }

    pub fn with_attribute(mut self, attribute: MeshAttribute, value: AttributeValue) -> Self {
        for i in 0..self.vertices.len() {
            let vert = &mut self.vertices[i];
            vert.set_attribute(attribute, value);
        }

        self
    }

    pub fn with_indices(mut self, indices: &[u16]) -> Self {
        self.indices = indices.into();

        self
    }

    pub fn with_material(mut self, material_handle: Handle<Pipeline>) -> Self {
        self.material_handle = material_handle;

        self
    }

    pub fn with_texture(mut self, texture_handle: Handle<Texture>) -> Self {
        self.texture_handle = Some(texture_handle);

        self
    }

    pub fn build(self) -> Mesh {
        Mesh {
            texture_handle: self.texture_handle,
            material_handle: self.material_handle,
            vertices: self.vertices,
            indices: self.indices,
            z: 0.,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub(crate) texture_handle: Option<Handle<Texture>>,
    pub(crate) material_handle: Handle<Pipeline>,
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u16>,
    // used for sorting
    pub(crate) z: f32,
}
impl Mesh {
    pub fn new(
        texture_handle: Option<Handle<Texture>>,
        material_handle: Handle<Pipeline>,
        vertices: Vec<Vertex>,
        indices: Vec<u16>,
        z: f32,
    ) -> Self {
        Self {
            material_handle,
            texture_handle,
            vertices,
            indices,
            z,
        }
    }

    pub fn concat(&mut self, mut vertices: Vec<Vertex>, mut indices: Vec<u16>) {
        self.vertices.append(&mut vertices);
        self.indices.append(&mut indices);
    }
}

#[derive(Hash, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MeshAttribute {
    Position,
    UV,
    Color,
    Normal,
}

impl MeshAttribute {
    pub fn size(&self) -> usize {
        match self {
            MeshAttribute::Position => mem::size_of::<[f32; 3]>(),
            MeshAttribute::UV => mem::size_of::<[f32; 2]>(),
            MeshAttribute::Color => mem::size_of::<[f32; 4]>(),
            MeshAttribute::Normal => mem::size_of::<[f32; 2]>(),
        }
    }

    fn format(&self) -> wgpu::VertexFormat {
        match self {
            MeshAttribute::Position => VertexFormat::Float32x3,
            MeshAttribute::UV => VertexFormat::Float32x2,
            MeshAttribute::Color => VertexFormat::Float32x4,
            MeshAttribute::Normal => VertexFormat::Float32x2,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum AttributeValue {
    Position([f32; 3]),
    UV([f32; 2]),
    Color([f32; 4]),
    Normal([f32; 2]),
}

impl AttributeValue {
    fn into_mesh_attr(&self) -> MeshAttribute {
        match self {
            AttributeValue::Position(_) => MeshAttribute::Position,
            AttributeValue::UV(_) => MeshAttribute::UV,
            AttributeValue::Color(_) => MeshAttribute::Color,
            AttributeValue::Normal(_) => MeshAttribute::Normal,
        }
    }

    fn get_bytes(&self) -> &[u8] {
        match self {
            AttributeValue::Position(values) => cast_slice(values),
            AttributeValue::UV(values) => cast_slice(values),
            AttributeValue::Color(values) => cast_slice(values),
            AttributeValue::Normal(values) => cast_slice(values),
        }
    }
}

pub fn get_attribute_layout<'a>(
    attributes: impl Iterator<Item = &'a MeshAttribute>,
) -> (Vec<VertexAttribute>, u64) {
    let mut vertex_attribute: Vec<VertexAttribute> = Vec::new();
    let mut offset: usize = 0;
    for (index, data) in attributes.enumerate() {
        vertex_attribute.push(VertexAttribute {
            format: data.format(),
            offset: offset as u64,
            shader_location: index as u32,
        });

        offset += data.size();
    }

    (vertex_attribute, offset as u64)
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex(pub BTreeMap<MeshAttribute, AttributeValue>);

impl Vertex {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn with_attribute(mut self, attribute: MeshAttribute, value: AttributeValue) -> Self {
        self.0.insert(attribute, value);

        self
    }

    pub fn set_attribute(&mut self, attribute: MeshAttribute, value: AttributeValue) {
        self.0.insert(attribute, value);
    }

    // TODO: This seems a little slow,
    // Maybe it could be better
    pub fn get_bytes(&self) -> Vec<u8> {
        let mut base = Vec::new();

        for attr in self.0.values() {
            base.append(&mut attr.get_bytes().to_vec());
        }

        base
    }
}
