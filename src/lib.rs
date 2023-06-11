use std::{collections::HashMap, sync::Arc};

use arena::{Arena, ArenaId, Handle};
use batching::PreparedMeshBatch;
use bind_groups::BindGroupLayoutBuilder;
use camera::Camera;
use errors::RenderBuddyError;
use font_atlas::FontAtlas;
use fonts::{Font, FontSizeKey};
use glam::{Quat, Vec3, Vec4};
use material::DefaultMat;
use mesh::{BatchMeshCreator, Mesh, MeshCreator};
use pipeline::Pipeline;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use render_context::RenderContext;
use texture::{Texture, TextureSamplerType};
use transform::Transform;
use wgpu::{
    BindGroup, BindGroupLayout, BindingType, RenderPass, Sampler, ShaderStages,
    SurfaceConfiguration,
};

pub mod arena;
pub mod batching;
pub mod bind_groups;
pub mod camera;
pub mod dynamic_texture_atlas_builder;
pub mod errors;
mod float_ord;
mod font_atlas;
pub mod fonts;
pub mod material;
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

pub struct MaterialMap {
    default: Handle<Pipeline>,
}

pub struct RenderBuddy {
    pub(crate) fonts: Arena<Font>,
    pub(crate) font_atlases: HashMap<(FontSizeKey, ArenaId), FontAtlas>,
    pub textures: Arena<Texture>,
    pub device: wgpu::Device,
    pub(crate) meshes: Vec<Mesh>,
    pub(crate) default_texture_samplers: HashMap<TextureSamplerType, Handle<Sampler>>,
    pub samplers: Arena<Sampler>,
    camera_bind_group_layout: BindGroupLayout,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: SurfaceConfiguration,
    materials: Arena<Pipeline>,
    pub(crate) material_map: MaterialMap,
    pub(crate) depth_texture_handle: Handle<Texture>,
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

        let mut samplers = Arena::new();

        let default_sampler_linear = samplers.insert(default_sampler_linear);
        let default_sampler_nearest = samplers.insert(default_sampler_nearest);

        let camera_bind_group_layout = BindGroupLayoutBuilder::new()
            .append(
                ShaderStages::VERTEX,
                BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                None,
            )
            .build(&device, Some("camera_bind_group_layout"));

        let depth_texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None, // 5.
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        let depth_texture_sampler_handle = samplers.insert(depth_texture_sampler);

        let depth_texture = Texture::create_depth_texture(
            &device,
            &surface_config,
            depth_texture_sampler_handle.clone(),
        );

        let blank_texture = Texture::create_blank_texture(&device, &queue, default_sampler_linear);

        let mut textures = Arena::new();

        textures.insert(blank_texture);
        let depth_texture_handle = textures.insert(depth_texture);

        let mut render_buddy = Self {
            camera_bind_group_layout,
            font_atlases: HashMap::default(),
            fonts: Arena::new(),
            device,
            meshes: Vec::default(),
            textures,
            queue,
            surface,
            surface_config,
            samplers,
            default_texture_samplers: HashMap::from([
                (TextureSamplerType::Linear, default_sampler_linear),
                (TextureSamplerType::Nearest, default_sampler_nearest),
                (TextureSamplerType::Depth, depth_texture_sampler_handle),
            ]),
            materials: Arena::new(),
            material_map: MaterialMap {
                default: Handle::default(),
            },
            depth_texture_handle,
        };

        let default_mat = DefaultMat {};
        let render_pipeline = render_buddy.create_pipeline_from_material(&default_mat);
        let material_handle = render_buddy.materials.insert(Pipeline {
            render_pipeline,
            material: Box::from(default_mat),
        });
        render_buddy.material_map.default = material_handle;

        render_buddy.fonts.insert(
            Font::try_from_bytes(include_bytes!("./default_font/Roboto-Regular.ttf")).unwrap(),
        );

        Ok(render_buddy)
    }

    pub fn get_viewport_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    /// Pushes a mesh to the render queue, must implement MeshBuilder
    pub fn push(&mut self, mesh: impl MeshCreator, position: Vec3) {
        self.push_transform(mesh, Transform::from_position(position));
    }

    /// Pushes a mesh to the render queue, with a rotation
    pub fn push_rotation(&mut self, mesh: impl MeshCreator, position: Vec3, rotation: Quat) {
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
    pub fn push_scale(&mut self, mesh: impl MeshCreator, position: Vec3, scale: Vec3) {
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
    pub fn push_transform(&mut self, mesh: impl MeshCreator, transform: Transform) {
        let mesh = mesh.build(transform, &self);
        self.meshes.push(mesh);
    }

    /// Pushes a group of meshes, useful for pushing a batch of meshes, mainly used for text rendering
    /// Since each glyph is a separate mesh
    pub fn append(&mut self, meshes: impl BatchMeshCreator, position: Vec3) {
        self.append_transform(meshes, Transform::from_position(position))
    }

    /// Pushes a group of meshes, with a transform
    pub fn append_transform(&mut self, meshes: impl BatchMeshCreator, transform: Transform) {
        let mut meshes = meshes.build(transform, self);
        self.meshes.append(&mut meshes);
    }

    pub fn push_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(mesh);
    }

    pub fn append_meshes(&mut self, mut meshes: Vec<Mesh>) {
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
        use_depth_stencil_attachment: bool,
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
                        depth_stencil_attachment: if use_depth_stencil_attachment {
                            Some(wgpu::RenderPassDepthStencilAttachment {
                                view: &self.textures.get(self.depth_texture_handle).unwrap().view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            })
                        } else {
                            None
                        },
                    });

            render_prepared_meshes(
                &mesh_prepared_batch,
                &mut render_pass,
                &self.materials,
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

        self.replace_texture(
            self.depth_texture_handle,
            Texture::create_depth_texture(
                &self.device,
                &self.surface_config,
                *self
                    .default_texture_samplers
                    .get(&TextureSamplerType::Depth)
                    .unwrap(),
            ),
        );
    }
}

fn render_prepared_meshes<'a>(
    mesh_batches: &'a Vec<PreparedMeshBatch>,
    render_pass: &mut RenderPass<'a>,
    materials: &'a Arena<Pipeline>,
    camera_bind_group: &'a BindGroup,
) {
    let last_material = ArenaId::default();

    for mesh_batch in mesh_batches {
        if mesh_batch.material_handle.id != last_material {
            let pipeline: &Pipeline = materials
                .get(mesh_batch.material_handle)
                .expect("Mesh was given invalid pipeline id");
            render_pass.set_pipeline(&pipeline.render_pipeline);

            render_pass.set_bind_group(0, &camera_bind_group, &[]); // can probably do this once before the loop
        }

        for (i, bind_group) in mesh_batch.bind_groups.iter().enumerate() {
            render_pass.set_bind_group(i as u32 + 1, &bind_group, &[]);
        }

        render_pass.set_vertex_buffer(0, mesh_batch.vertex_buffer.slice(..));
        if mesh_batch.indices_len > 0 {
            render_pass
                .set_index_buffer(mesh_batch.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..mesh_batch.indices_len, 0, 0..1);
        } else {
            render_pass.draw(0..mesh_batch.vert_len, 0..1);
        }
    }
}
