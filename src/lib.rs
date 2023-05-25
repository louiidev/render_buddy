use std::{collections::HashMap, sync::Arc};

use arena::{Arena, ArenaId};
use batching::PreparedMeshBatch;
use camera::Camera;
use errors::RenderBuddyError;
use font_atlas::FontAtlas;
use fonts::{Font, FontSizeKey};
use glam::{Quat, Vec3, Vec4};
use mesh::{BatchMeshBuild, Mesh, MeshBuilder};
use pipeline::Pipeline;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use render_context::RenderContext;
use texture::{Texture, TextureSamplerType};
use transform::Transform;
use wgpu::{BindGroup, BindGroupLayout, RenderPass, Sampler, SurfaceConfiguration};

pub mod arena;
pub mod batching;
pub mod camera;
pub mod dynamic_texture_atlas_builder;
pub mod errors;
mod float_ord;
mod font_atlas;
pub mod fonts;
pub mod mesh;
pub mod pipeline;
pub mod rect;
mod render_context;
pub mod sprite;
pub mod text;
pub mod texture;
pub mod texture_atlas;
pub mod transform;

pub use glam;
pub use wgpu;

pub struct RenderBuddy {
    pub(crate) fonts: Arena<Font>,
    pub(crate) font_atlases: HashMap<(FontSizeKey, ArenaId), FontAtlas>,
    pub(crate) cached_pipelines: Arena<Pipeline>,
    pub(crate) textures: Arena<Texture>,
    pub(crate) device: wgpu::Device,
    pub(crate) meshes: Vec<Mesh>,
    pub(crate) texture_samplers: HashMap<TextureSamplerType, Arc<Sampler>>,
    camera_bind_group_layout: BindGroupLayout,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: SurfaceConfiguration,
}

impl RenderBuddy {
    /// Creates a  [`RenderBuddy`] Instance
    /// Creates a WGPU Surface, Instance, Device and Queue
    /// Requires async to request instance adapter
    pub async fn new<W>(window: &W, viewport_size: (u32, u32)) -> Result<Self, RenderBuddyError>
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window)? };
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
        {
            Some(it) => it,
            None => return Err(RenderBuddyError::new("Unable to request adapter from wgpu")),
        };

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.describe().srgb)
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: viewport_size.0,
            height: viewport_size.1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::default(),
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // Webgl 2 for web until WGPU is fully supported
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await?;

        surface.configure(&device, &surface_config);

        let default_sampler_nearest = {
            device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            })
        };

        let default_sampler_linear = {
            device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
        };

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let mut render_buddy = Self {
            camera_bind_group_layout,
            font_atlases: HashMap::default(),
            fonts: Arena::new(),
            cached_pipelines: Arena::new(),
            device,
            meshes: Vec::default(),
            textures: Arena::new(),
            queue,
            surface,
            surface_config,
            texture_samplers: HashMap::from([
                (TextureSamplerType::Linear, Arc::new(default_sampler_linear)),
                (
                    TextureSamplerType::Nearest,
                    Arc::new(default_sampler_nearest),
                ),
            ]),
        };

        render_buddy.fonts.insert(
            Font::try_from_bytes(include_bytes!("./default_font/Roboto-Regular.ttf")).unwrap(),
        );

        render_buddy.create_blank_texture();
        let _default_handle = render_buddy.create_default_pipeline();

        Ok(render_buddy)
    }

    /// Pushes a mesh to the render queue, must implement MeshBuilder
    pub fn push(&mut self, mesh: impl MeshBuilder, position: Vec3) {
        self.push_transform(mesh, Transform::from_position(position));
    }

    /// Pushes a mesh to the render queue, with a rotation
    pub fn push_rotation(&mut self, mesh: impl MeshBuilder, position: Vec3, rotation: Quat) {
        self.push_transform(
            mesh,
            Transform {
                position,
                rotation,
                ..Transform::IDENTITY
            },
        );
    }
    /// Pushes a mesh to the render queue, with a scale
    pub fn push_scale(&mut self, mesh: impl MeshBuilder, position: Vec3, scale: Vec3) {
        self.push_transform(
            mesh,
            Transform {
                position,
                scale,
                ..Transform::IDENTITY
            },
        );
    }
    /// Pushes a mesh to the render queue, with a full transform
    pub fn push_transform(&mut self, mesh: impl MeshBuilder, transform: Transform) {
        let mesh = mesh.build(transform, &self);
        self.meshes.push(mesh);
    }

    /// Pushes a group of meshes, useful for pushing a batch of meshes, mainly used for text rendering
    /// Since each glyph is a separate mesh
    pub fn append(&mut self, meshes: impl BatchMeshBuild, position: Vec3) {
        self.append_transform(meshes, Transform::from_position(position))
    }

    /// Pushes a group of meshes, with a transform
    pub fn append_transform(&mut self, meshes: impl BatchMeshBuild, transform: Transform) {
        let mut meshes = meshes.build(transform, self);
        self.meshes.append(&mut meshes);
    }

    /// Begin the render process by prepping the [`RenderContext`]
    pub fn begin(&self) -> RenderContext {
        let output = self
            .surface
            .get_current_texture()
            .expect("Missing current texture in surface");

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        RenderContext {
            output,
            view,
            command_encoder,
        }
    }

    /// Render the queue
    /// takes in an optional clear color, potentially useful if you want to call render twice in one frame
    pub fn render(
        &mut self,
        render_context: &mut RenderContext,
        clear_color: Option<Vec4>,
        camera: &Camera,
    ) {
        let mesh_prepared_batch = self.prepare_mesh_batch();
        let camera_bind_group = camera.create_bind_group(
            &self.device,
            (self.surface_config.width, self.surface_config.height),
            &self.camera_bind_group_layout,
        );

        let load = if let Some(clear_color) = clear_color {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: clear_color.x as f64,
                g: clear_color.y as f64,
                b: clear_color.z as f64,
                a: clear_color.w as f64,
            })
        } else {
            wgpu::LoadOp::Load
        };

        {
            let mut render_pass =
                render_context
                    .command_encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &render_context.view,
                            resolve_target: None,
                            ops: wgpu::Operations { load, store: true },
                        })],
                        depth_stencil_attachment: None,
                    });

            render_prepared_meshes(
                &mesh_prepared_batch,
                &mut render_pass,
                &self.cached_pipelines,
                &camera_bind_group,
            );
        }
    }

    /// Presents the frame to WGPU for rendering
    /// Drops the [`RenderContext`]
    pub fn end_frame(&mut self, render_context: RenderContext) {
        self.queue
            .submit(std::iter::once(render_context.command_encoder.finish()));
        render_context.output.present();
    }

    /// Should be called when the window has been resized
    pub fn resize(&mut self, new_surface_size: (u32, u32)) {
        self.surface_config.width = new_surface_size.0;
        self.surface_config.height = new_surface_size.1;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

fn render_prepared_meshes<'a>(
    mesh_batches: &'a Vec<PreparedMeshBatch>,
    render_pass: &mut RenderPass<'a>,
    cached_pipelines: &'a Arena<Pipeline>,
    camera_bind_group: &'a BindGroup,
) {
    let last_pipeline_id = ArenaId::default();

    for mesh_batch in mesh_batches {
        if mesh_batch.pipeline_handle.id != last_pipeline_id {
            let pipeline = cached_pipelines
                .get(mesh_batch.pipeline_handle)
                .expect("Mesh was given invalid pipeline id");
            render_pass.set_pipeline(&pipeline.render_pipeline);
        }

        render_pass.set_bind_group(0, &camera_bind_group, &[]);
        render_pass.set_bind_group(1, &mesh_batch.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh_batch.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh_batch.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..mesh_batch.indices_len, 0, 0..1);
    }
}
