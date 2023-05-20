use glam::Vec2;

use crate::{
    arena::{ArenaId, Handle},
    mesh::{Mesh, MeshBuilder, Vertex2D, QUAD_INDICES, QUAD_UVS, QUAD_VERTEX_POSITIONS},
    rect::Rect,
    texture::Texture,
    RenderBuddy,
};

pub struct TexturedRect {
    pub texture_id: ArenaId,
    pub anchor: Anchor,
    pub color: [f32; 4],
    pub texture_rect: Option<Rect>,
    pub custom_size: Option<Vec2>,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Default for TexturedRect {
    fn default() -> Self {
        TexturedRect {
            texture_id: ArenaId::first(),
            anchor: Anchor::default(),
            color: [1., 1., 1., 1.],
            texture_rect: None,
            custom_size: None,
            flip_x: false,
            flip_y: false,
        }
    }
}
impl TexturedRect {
    pub fn new(handle: Handle<Texture>) -> Self {
        TexturedRect {
            texture_id: handle.id,
            ..Default::default()
        }
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }
}

impl MeshBuilder for TexturedRect {
    fn build(&self, transform: crate::transform::Transform, rb: &RenderBuddy) -> Mesh {
        let texture = rb
            .textures
            .get(self.texture_id)
            .expect("Mesh is missing texture");

        let mut uvs = QUAD_UVS;

        if self.flip_x {
            uvs = [uvs[1], uvs[0], uvs[3], uvs[2]];
        }
        if self.flip_y {
            uvs = [uvs[3], uvs[2], uvs[1], uvs[0]];
        }

        let current_image_size = texture.dimensions;

        // By default, the size of the quad is the size of the texture
        let mut quad_size = current_image_size;

        // If a rect is specified, adjust UVs and the size of the quad
        if let Some(rect) = self.texture_rect {
            let rect_size = rect.size();
            for uv in &mut uvs {
                *uv = (rect.min + *uv * rect_size) / current_image_size;
            }
            quad_size = rect_size;
        }

        // Override the size if a custom one is specified
        if let Some(custom_size) = self.custom_size {
            quad_size = custom_size;
        }

        let positions: [[f32; 3]; 4] = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
            transform
                .transform_point(((quad_pos - self.anchor.as_vec()) * quad_size).extend(0.))
                .into()
        });

        let vertices = positions
            .iter()
            .zip(uvs)
            .map(|(position, uv)| Vertex2D {
                position: *position,
                uv: uv.into(),
                color: self.color,
            })
            .collect();

        Mesh::new(
            self.texture_id,
            vertices,
            QUAD_INDICES.to_vec(),
            transform.position.z,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Anchor {
    #[default]
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
    CenterLeft,
    CenterRight,
    TopLeft,
    TopCenter,
    TopRight,
    /// Custom anchor point. Top left is `(-0.5, 0.5)`, center is `(0.0, 0.0)`. The value will
    /// be scaled with the sprite size.
    Custom(Vec2),
}

impl Anchor {
    pub fn as_vec(&self) -> Vec2 {
        match self {
            Anchor::Center => Vec2::ZERO,
            Anchor::BottomLeft => Vec2::new(-0.5, -0.5),
            Anchor::BottomCenter => Vec2::new(0.0, -0.5),
            Anchor::BottomRight => Vec2::new(0.5, -0.5),
            Anchor::CenterLeft => Vec2::new(-0.5, 0.0),
            Anchor::CenterRight => Vec2::new(0.5, 0.0),
            Anchor::TopLeft => Vec2::new(-0.5, 0.5),
            Anchor::TopCenter => Vec2::new(0.0, 0.5),
            Anchor::TopRight => Vec2::new(0.5, 0.5),
            Anchor::Custom(point) => *point,
        }
    }
}
