use std::ops::Add;

use glam::Vec2;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// The minimum corner point of the rect.
    pub min: Vec2,
    /// The maximum corner point of the rect.
    pub max: Vec2,
}
impl Rect {
    pub(crate) fn size(&self) -> Vec2 {
        todo!()
    }
}

impl Add<Vec2> for Rect {
    type Output = Rect;
    fn add(self, other: Vec2) -> Self {
        Self {
            min: self.min + other,
            max: self.max + other,
        }
    }
}
