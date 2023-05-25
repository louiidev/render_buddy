use std::ops::Add;

use glam::{Vec2, Vec4};

use crate::{
    arena::{ArenaId, Handle},
    mesh::{Mesh, MeshBuilder, Vertex2D, QUAD_INDICES, QUAD_UVS, QUAD_VERTEX_POSITIONS},
    sprite::Anchor,
    transform::Transform,
    RenderBuddy,
};

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// The minimum corner point of the rect.
    pub(crate) min: Vec2,
    /// The maximum corner point of the rect.
    pub(crate) max: Vec2,
    pub color: Vec4,
    pub anchor: Anchor,
}
impl Rect {
    pub(crate) fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn new(size: Vec2, color: Vec4) -> Self {
        Self {
            min: Vec2::ZERO,
            max: size,
            color,
            anchor: Anchor::Center,
        }
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }
}

impl Add<Vec2> for Rect {
    type Output = Rect;
    fn add(self, other: Vec2) -> Self {
        Self {
            min: self.min + other,
            max: self.max + other,
            color: self.color,
            anchor: self.anchor,
        }
    }
}

impl MeshBuilder for Rect {
    fn build(&self, transform: Transform, _rb: &RenderBuddy) -> Mesh {
        let quad_size = self.size();

        let positions: [[f32; 3]; 4] = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
            transform
                .transform_point(((quad_pos - self.anchor.as_vec()) * quad_size).extend(0.))
                .into()
        });

        let vertices = positions
            .iter()
            .zip(QUAD_UVS)
            .map(|(position, uv)| Vertex2D {
                position: *position,
                uv: uv.into(),
                color: self.color.to_array(),
            })
            .collect();

        Mesh::new(
            Handle::new(ArenaId::first()),
            vertices,
            QUAD_INDICES.to_vec(),
            transform.position.z,
        )
    }
}
