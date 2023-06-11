use std::collections::BTreeMap;

use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
};
use glam::{Vec2, Vec4};
use wgpu::TextureFormat;

use crate::{
    arena::{ArenaId, Handle},
    float_ord::FloatOrd,
    font_atlas::FontAtlas,
    fonts::{Font, GlyphAtlasInfo, PositionedGlyph},
    mesh::{
        AttributeValue, BatchMeshCreator, Mesh, MeshAttribute, Vertex, QUAD_INDICES,
        QUAD_VERTEX_POSITIONS,
    },
    pipeline::Pipeline,
    texture::{Image, Texture},
    transform::Transform,
    RenderBuddy,
};

pub struct Text {
    handle: Handle<Font>,
    material: Option<Handle<Pipeline>>,
    value: String,
    font_size: f32,
    color: Vec4,
    vertical_alignment: VerticalAlign,
    horizontal_alignment: HorizontalAlign,
    y_axis_orientation: CoordinateSystem,
}

impl Text {
    pub fn new(value: &str, font_size: f32) -> Self {
        Self {
            value: value.to_owned(),
            font_size,
            ..Default::default()
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }
}

impl RenderBuddy {
    pub fn measure_text(&mut self, text: &Text) -> Vec2 {
        let positioned_glyphs = self.get_positioned_glyphs(text, None);

        let size: Vec2 = positioned_glyphs.iter().fold(
            Vec2::default(),
            |mut size: Vec2, text_glyph: &PositionedGlyph| {
                let rect = text_glyph.rect;
                let glyph_position = text_glyph.position;

                let x_distance = glyph_position.x - size.x;
                let y_distance = glyph_position.y + size.y;
                dbg!(y_distance);
                let actual_glyph_size = rect.size();
                size.y = size.y.max(actual_glyph_size.y + y_distance.abs() / 2.);
                size.x += actual_glyph_size.x + x_distance;

                size
            },
        );
        // panic!();
        size
    }
}

impl BatchMeshCreator for Text {
    fn build(&self, mut transform: Transform, rb: &mut RenderBuddy) -> Vec<crate::mesh::Mesh> {
        let positioned_glyphs = rb.get_positioned_glyphs(self, None);

        let size = positioned_glyphs.iter().fold(
            Vec2::default(),
            |mut size: Vec2, text_glyph: &PositionedGlyph| {
                let rect = text_glyph.rect;
                let glyph_position = text_glyph.position;

                let x_distance = glyph_position.x - size.x;
                let actual_glyph_size = rect.size();
                size.y = size.y.max(actual_glyph_size.y);
                size.x += actual_glyph_size.x + x_distance;

                size
            },
        );

        let offset = Vec2::new(size.x / 2. * transform.scale.x, -size.y * transform.scale.y);
        transform.position -= offset.extend(0.);

        positioned_glyphs
            .iter()
            .map(|text_glyph| {
                // let transform = Transform::from_translation(position + text_glyph.position.extend(0.));
                let mut transform = transform.clone();
                transform.position = transform.transform_point(text_glyph.position.extend(0.));
                let current_image_size = text_glyph.atlas_info.atlas_size;
                // let scale_factor = 1f32;
                let rect = text_glyph.rect;

                let mut vertices = Vec::new();

                let uvs = [
                    Vec2::new(rect.min.x, rect.max.y),
                    Vec2::new(rect.max.x, rect.max.y),
                    Vec2::new(rect.max.x, rect.min.y),
                    Vec2::new(rect.min.x, rect.min.y),
                ]
                .map(|pos| pos / current_image_size);

                let positions: [[f32; 3]; 4] = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
                    transform
                        .transform_point(
                            ((quad_pos - Vec2::new(-0.5, -0.5)) * rect.size()).extend(0.),
                        )
                        .into()
                });

                for i in 0..QUAD_VERTEX_POSITIONS.len() {
                    vertices.push(Vertex(BTreeMap::from([
                        (
                            MeshAttribute::Position,
                            AttributeValue::Position(positions[i]),
                        ),
                        (MeshAttribute::UV, AttributeValue::UV(uvs[i].into())),
                        (
                            MeshAttribute::Color,
                            AttributeValue::Color(self.color.into()),
                        ),
                    ])));
                }

                let material_handle = self.material.unwrap_or(rb.material_map.default);

                Mesh {
                    texture_handle: Some(text_glyph.atlas_info.texture_handle),
                    material_handle,
                    vertices,
                    indices: QUAD_INDICES.to_vec(),
                    z: transform.position.z,
                }
            })
            .collect()
    }
}

impl Default for Text {
    fn default() -> Self {
        Self {
            handle: Handle::new(ArenaId::first()),
            material: None,
            value: Default::default(),
            font_size: Default::default(),
            vertical_alignment: VerticalAlign::Top,
            horizontal_alignment: HorizontalAlign::Center,
            y_axis_orientation: CoordinateSystem::PositiveYUp,
            color: Vec4::new(1., 1., 1., 1.), // White
        }
    }
}

impl RenderBuddy {
    pub(crate) fn get_positioned_glyphs(
        &mut self,
        text: &Text,
        container_size: Option<Vec2>,
    ) -> Vec<PositionedGlyph> {
        let texture = self.add_glyphs_to_atlas(text.handle, &text.value, text.font_size);

        if let Some(temp_texture_data) = texture {
            let texture = self.add_texture_bytes(
                &temp_texture_data.data,
                temp_texture_data.dimensions,
                crate::texture::TextureSamplerType::Linear,
                TextureFormat::Rgba8UnormSrgb,
            );

            // Update texture or insert new texture
            if let Some(handle) = self
                .fonts
                .get(text.handle)
                .unwrap()
                .texture_ids
                .get(&(FloatOrd(text.font_size)))
            {
                *self.textures.get_mut(*handle).unwrap() = texture;
            } else {
                let texture_handle = self.textures.insert(texture);

                self.fonts
                    .get_mut(text.handle)
                    .unwrap()
                    .texture_ids
                    .insert(FloatOrd(text.font_size), texture_handle);
            }
        }

        let texture_handle = *self
            .fonts
            .get(text.handle)
            .unwrap()
            .texture_ids
            .get(&(FloatOrd(text.font_size)))
            .expect("Error, missing texture id for font");

        let mut positioned_glyphs = Vec::new();

        let mut layout = Layout::new(text.y_axis_orientation);

        layout.reset(&LayoutSettings {
            x: 0.0,
            y: 0.0,
            max_width: container_size.map(|size| size.x),
            max_height: container_size.map(|size| size.y),
            horizontal_align: text.horizontal_alignment,
            vertical_align: text.vertical_alignment,
            ..Default::default()
        });
        let font = &self.fonts.get(text.handle).unwrap().font;
        layout.append(&[font], &TextStyle::new(&text.value, text.font_size, 0));

        for glyph in layout.glyphs() {
            let atlas_info = self
                .get_glyph_atlas_info(text.font_size, text.handle.id, glyph.parent, texture_handle)
                .unwrap();

            positioned_glyphs.push(PositionedGlyph {
                position: Vec2::new(glyph.x, glyph.y),
                rect: atlas_info.texture_rect,
                atlas_info,
            });
        }

        positioned_glyphs
    }

    pub(crate) fn add_glyphs_to_atlas(
        &mut self,
        font_handle: Handle<Font>,
        text: &str,
        font_size: f32,
    ) -> Option<Image> {
        let font_atlas = self
            .font_atlases
            .entry((FloatOrd(font_size), font_handle.id))
            .or_insert_with(|| FontAtlas::new(Vec2::splat(512.0)));
        let font = self.fonts.get(font_handle).unwrap();
        let mut update_texture_data = None;
        for character in text.chars() {
            if !font_atlas.has_glyph(character) {
                let (metrics, bitmap) = font.rasterize(character, font_size);
                update_texture_data = font_atlas.add_glyph(character, &bitmap, metrics);
            }
        }

        update_texture_data
    }

    pub fn get_glyph_atlas_info(
        &mut self,
        font_size: f32,
        font_id: ArenaId,
        glyph: char,
        texture_handle: Handle<Texture>,
    ) -> Option<GlyphAtlasInfo> {
        let key = (FloatOrd(font_size), font_id);
        let atlas = self.font_atlases.get(&key);

        if let Some(font_atlas) = atlas {
            let texture_atlas = &font_atlas.dynamic_texture_atlas_builder.texture_atlas;

            return font_atlas
                .get_glyph_index(glyph)
                .map(|(glyph_index, metrics)| GlyphAtlasInfo {
                    texture_rect: texture_atlas.textures.get(glyph_index).copied().unwrap(),
                    metrics,
                    texture_handle,
                    atlas_size: texture_atlas.size,
                });
        }

        None
    }
}
