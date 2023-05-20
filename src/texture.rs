use glam::Vec2;
use std::num::NonZeroU32;
use wgpu::Extent3d;

use crate::{
    arena::{ArenaId, Handle},
    RenderBuddy,
};

#[derive(Default, PartialEq, Hash, Eq, Clone, Copy)]
pub enum TextureSamplerType {
    Linear,
    #[default]
    Nearest,
}

pub struct Image {
    pub data: Vec<u8>,
    pub dimensions: (u32, u32),
    pub sampler: TextureSamplerType,
}

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

        let id = self.textures.insert(texture);

        assert!(
            id == ArenaId::first(),
            "Blank texture needs to be first texture inserted"
        );
    }

    pub fn add_texture(&mut self, image: Image) -> Handle<Texture> {
        self.add_texture_from_bytes(&image.data, image.dimensions, image.sampler)
    }

    pub fn add_texture_from_bytes(
        &mut self,
        bytes: &[u8],
        size: (u32, u32),
        sampler: TextureSamplerType,
    ) -> Handle<Texture> {
        let texture = self.add_texture_bytes(bytes, size, sampler);
        Handle::new(self.textures.insert(texture))
    }

    fn add_texture_bytes(
        &mut self,
        bytes: &[u8],
        size: (u32, u32),
        sampler: TextureSamplerType,
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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

    pub fn replace_texture(&mut self, handle: Handle<Texture>, image: Image) {
        let texture: Texture = self.add_texture_bytes(&image.data, image.dimensions, image.sampler);
        *self
            .textures
            .get_mut(handle.id)
            .expect("No texture to replace") = texture;
    }
}
