use glam::Vec2;
use std::{num::NonZeroU32, sync::Arc};
use wgpu::{
    BindGroup, BindGroupLayout, BindingResource, Device, Extent3d, Queue, Sampler,
    SurfaceConfiguration, TextureFormat,
};

use crate::{
    arena::{Arena, ArenaId, Handle},
    bind_groups::BindGroupBuilder,
    RenderBuddy,
};

#[derive(Default, Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub enum TextureSamplerType {
    Linear,
    #[default]
    Nearest,
    Depth,
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
    pub sampler: Handle<Sampler>,
}

impl Texture {
    pub(crate) fn create_blank_texture(
        device: &Device,
        queue: &Queue,
        sampler: Handle<Sampler>,
    ) -> Self {
        let size = Extent3d::default();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
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
            sampler,
        };

        texture
    }

    pub fn create_bind_group(
        &self,
        device: &Device,
        bgl: &BindGroupLayout,
        sampler: &Sampler,
    ) -> BindGroup {
        BindGroupBuilder::new()
            .append_texture_view(&self.view)
            .append(BindingResource::Sampler(&sampler))
            .build(device, None, &bgl)
    }

    pub(crate) fn create_depth_texture(
        device: &Device,
        surface_config: &SurfaceConfiguration,
        sampler: Handle<Sampler>,
    ) -> Self {
        let size = wgpu::Extent3d {
            // 2.
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[TextureFormat::Depth32Float],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // TODO: we should probably store this and have a ref in the texture

        Texture {
            texture,
            view,
            dimensions: Vec2::new(surface_config.width as f32, surface_config.height as f32),
            sampler,
        }
    }
}

impl RenderBuddy {
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
        texture_sampler_type: TextureSamplerType,
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
            sampler: *self
                .default_texture_samplers
                .get(&texture_sampler_type)
                .unwrap(),
        }
    }

    pub(crate) fn replace_texture(&mut self, handle: Handle<Texture>, texture: Texture) {
        *self
            .textures
            .get_mut(handle)
            .expect("No texture to replace") = texture;
    }

    /// Replaces the given texture handle
    /// Useful for hot reloading
    pub fn replace_image(&mut self, handle: Handle<Texture>, image: Image) {
        let texture: Texture =
            self.add_texture_bytes(&image.data, image.dimensions, image.sampler, image.format);
        self.replace_texture(handle, texture)
    }
}
