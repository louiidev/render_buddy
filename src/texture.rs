use glam::Vec2;
use std::num::NonZeroU32;
use wgpu::{Extent3d, TextureFormat};

use crate::{
    arena::{ArenaId, Handle},
    RenderBuddy,
};

#[derive(Default, Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub enum TextureSamplerType {
    Linear,
    #[default]
    Nearest,
}

#[derive(Clone)]
pub struct Image {
    pub data: Vec<u8>,
    pub dimensions: (u32, u32),
    pub sampler: TextureSamplerType,
    pub format: TextureFormat,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            data: Default::default(),
            dimensions: Default::default(),
            sampler: Default::default(),
            format: TextureFormat::Rgba8UnormSrgb,
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub dimensions: Vec2,
    pub(crate) sampler: TextureSamplerType,
}

impl RenderBuddy {
    pub(crate) fn create_blank_texture(&mut self) {
        let size = Extent3d::default();
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &[255u8; 4],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * size.width),
                rows_per_image: NonZeroU32::new(size.height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let texture = Texture {
            texture,
            view,
            dimensions: Vec2::new(size.width as f32, size.height as f32),
            sampler: TextureSamplerType::default(),
        };

        let handle: Handle<Texture> = self.textures.insert(texture);

        assert!(
            handle.id == ArenaId::first(),
            "Blank texture needs to be first texture inserted"
        );
    }

    /// Loads a texture to the GPU
    /// Returns a handle to the texture ref
    pub fn add_texture(&mut self, image: Image) -> Handle<Texture> {
        self.add_texture_from_bytes(&image.data, image.dimensions, image.sampler, image.format)
    }

    /// Loads a texture to the GPU by passing the image bytes
    /// Must be parsed by a crate like image or something similar
    pub fn add_texture_from_bytes(
        &mut self,
        bytes: &[u8],
        size: (u32, u32),
        sampler: TextureSamplerType,
        format: TextureFormat,
    ) -> Handle<Texture> {
        let texture = self.add_texture_bytes(bytes, size, sampler, format);
        self.textures.insert(texture)
    }

    pub(crate) fn add_texture_bytes(
        &mut self,
        bytes: &[u8],
        size: (u32, u32),
        sampler: TextureSamplerType,
        format: TextureFormat,
    ) -> Texture {
        let size = Extent3d {
            width: size.0 as _,
            height: size.1 as _,
            depth_or_array_layers: 1,
        };

        let texture_descriptor = wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let format_described = texture_descriptor.format.describe();
        let texture = self.device.create_texture(&texture_descriptor);

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(format_described.block_size as u32 * size.width),
                rows_per_image: NonZeroU32::new(size.height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Texture {
            texture,
            view,
            dimensions: Vec2::new(size.width as f32, size.height as f32),
            sampler,
        }
    }

    /// Replaces the given texture handle
    /// Useful for hot reloading
    pub fn replace_texture(&mut self, handle: Handle<Texture>, image: Image) {
        let texture: Texture =
            self.add_texture_bytes(&image.data, image.dimensions, image.sampler, image.format);
        *self
            .textures
            .get_mut(handle)
            .expect("No texture to replace") = texture;
    }
}
