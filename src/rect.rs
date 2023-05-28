use std::{collections::BTreeMap, ops::Add};

use glam::{Vec2, Vec4};

use crate::{
    arena::{ArenaId, Handle},
    mesh::{
        AttributeValue, Mesh, MeshAttribute, MeshBuilder, Vertex, QUAD_INDICES, QUAD_UVS,
        QUAD_VERTEX_POSITIONS,
    },
    pipeline::Pipeline,
    sprite::Anchor,
    transform::Transform,
    RenderBuddy,
};

#[derive(Default, Clone, Copy, Debug)]
pub struct Rect {
    /// The minimum corner point of the rect.
    pub(crate) min: Vec2,
    /// The maximum corner point of the rect.
    pub(crate) max: Vec2,
    pub color: Vec4,
    pub anchor: Anchor,
    pub material: Option<Handle<Pipeline>>,
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
            material: None,
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
            material: self.material,
        }
    }
}

impl MeshBuilder for Rect {
    fn build(&self, transform: Transform, rb: &RenderBuddy) -> Mesh {
        let quad_size = self.size();

        let positions: [[f32; 3]; 4] = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
            transform
                .transform_point(((quad_pos - self.anchor.as_vec()) * quad_size).extend(0.))
                .into()
        });

        let vertices = positions
            .iter()
            .zip(QUAD_UVS)
            .map(|(position, uv)| {
                Vertex(BTreeMap::from([
                    (MeshAttribute::Position, AttributeValue::Position(*position)),
                    (MeshAttribute::UV, AttributeValue::UV(uv.into())),
                    (
                        MeshAttribute::Color,
                        AttributeValue::Color(self.color.into()),
                    ),
                ]))
            })
            .collect();

        let material_handle = self.material.unwrap_or(rb.material_map.default);

        Mesh::new(
            Some(Handle::new(ArenaId::first())),
            material_handle,
            vertices,
            QUAD_INDICES.to_vec(),
            transform.position.z,
        )
    }
}
