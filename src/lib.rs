use std::{collections::HashMap, sync::Arc};

use arena::{Arena, ArenaId};
use batching::PreparedMeshBatch;
use camera::CameraData;
use glam::Vec4;
use mesh::{Mesh, MeshBuilder, Vertex2D};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use texture::{Texture, TextureSamplerType};
use transform::Transform;
use wgpu::{
    include_wgsl, util::DeviceExt, BindGroup, BlendState, CommandEncoder, FragmentState, FrontFace,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SurfaceConfiguration, SurfaceTexture, TextureView,
    VertexState,
};

pub mod arena;
pub mod batching;
pub mod camera;
pub mod mesh;
pub mod rect;
pub mod texture;
pub mod textured_rect;
pub mod transform;

pub struct RenderBuddy {
    pub(crate) cached_pipelines: Arena<RenderPipeline>,
    pub(crate) textures: Arena<Texture>,
    pub(crate) device: wgpu::Device,
    pub(crate) meshes: Vec<Mesh>,
    pub(crate) texture_samplers: HashMap<TextureSamplerType, Arc<Sampler>>,
    pub(crate) camera_data: CameraData,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: SurfaceConfiguration,
}

impl RenderBuddy {
    pub async fn new<W>(window: &W, surface_size: (u32, u32)) -> Self
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window).unwrap() };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

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
            width: surface_size.0,
            height: surface_size.1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::default(),
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

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

        let camera_bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            camera_data: CameraData::new(surface_size, camera_bind_group),
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

        render_buddy.create_blank_texture();
        render_buddy.create_default_pipeline();

        render_buddy
    }

    pub fn push(&mut self, mesh: impl MeshBuilder, transform: Transform) {
        let mesh = mesh.build(transform, &self);
        self.meshes.push(mesh);
    }

    fn prepare_mesh_batch(&mut self) -> Vec<PreparedMeshBatch> {
        let mut meshes = self.meshes.drain(0..).collect::<Vec<Mesh>>();

        meshes.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));

        let mut current_batch_texture_id = ArenaId::default();
        let mut current_pipeline_id = ArenaId::default();
        let mut batches: Vec<Mesh> = Vec::new();

        for mesh in meshes {
            if current_batch_texture_id == mesh.texture_id
                && current_pipeline_id == mesh.pipeline_id
            {
                let length = batches.len();

                let current_mesh = &mut batches[length - 1];
                let vert_count = current_mesh.vertices.len() as u16;
                let indices = mesh
                    .indices
                    .iter()
                    .map(|index| index + vert_count)
                    .collect::<Vec<u16>>();

                current_mesh.concat(mesh.vertices, indices);
            } else {
                current_batch_texture_id = mesh.texture_id;
                current_pipeline_id = mesh.pipeline_id;
                batches.push(mesh);
            }
        }

        batches
            .iter()
            .map(|batch| {
                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&batch.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                let index_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(&batch.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                let texture = self.textures.get(batch.texture_id).unwrap();

                let sampler = self.texture_samplers.get(&texture.sampler).unwrap();

                let pipeline = self
                    .cached_pipelines
                    .get(batch.pipeline_id)
                    .expect("Pipeline is missing for mesh");

                let texture_bind_group =
                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &pipeline.get_bind_group_layout(1),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(sampler),
                            },
                        ],
                        label: None,
                    });

                PreparedMeshBatch {
                    vertex_buffer,
                    index_buffer,
                    texture_bind_group,
                    indices_len: batch.indices.len() as _,
                    pipeline_id: batch.pipeline_id, // TODO: UPDATE THIS
                }
            })
            .collect()
    }

    pub fn begin(&self) -> (SurfaceTexture, TextureView, CommandEncoder) {
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

        (output, view, command_encoder)
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        command_encoder: &mut CommandEncoder,
        clear_color: Option<Vec4>,
    ) {
        let mesh_prepared_batch = self.prepare_mesh_batch();
        let camera_bind_group = self.camera_data.create_bind_group(&self.device);

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
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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

    pub fn end_frame(&mut self, command_encoder: CommandEncoder, output: SurfaceTexture) {
        self.queue.submit(std::iter::once(command_encoder.finish()));
        output.present();
    }

    pub fn resize_surface(&mut self, new_surface_size: (u32, u32)) {
        self.surface_config.width = new_surface_size.0;
        self.surface_config.height = new_surface_size.1;
        self.surface.configure(&self.device, &self.surface_config);
        self.camera_data.resize(new_surface_size);
    }

    fn create_default_pipeline(&mut self) {
        let shader = self
            .device
            .create_shader_module(include_wgsl!("./default_shaders/2d.wgsl"));

        let texture_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("2D Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &self.camera_data.bind_group_layout,
                        &texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let binding: [Option<wgpu::ColorTargetState>; 1] = [Some(wgpu::ColorTargetState {
            format: self.surface_config.format,
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                entry_point: "vertex",
                buffers: &[Vertex2D::desc()],
                module: &shader,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &binding,
            }),
            layout: Some(&render_pipeline_layout),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            label: Some("default_pipeline"),
            multiview: None,
        };

        let id = self
            .cached_pipelines
            .insert(self.device.create_render_pipeline(&descriptor));

        assert!(
            id == ArenaId::first(),
            "Default pipeline is not set to first"
        )
    }
}

fn render_prepared_meshes<'a>(
    mesh_batches: &'a Vec<PreparedMeshBatch>,
    render_pass: &mut RenderPass<'a>,
    cached_pipelines: &'a Arena<RenderPipeline>,
    camera_bind_group: &'a BindGroup,
) {
    let last_pipeline_id = ArenaId::default();

    for mesh_batch in mesh_batches {
        if mesh_batch.pipeline_id != last_pipeline_id {
            let pipeline = cached_pipelines
                .get(mesh_batch.pipeline_id)
                .expect("Mesh was given invalid pipeline id");
            render_pass.set_pipeline(&pipeline);
        }

        render_pass.set_bind_group(0, &camera_bind_group, &[]);
        render_pass.set_bind_group(1, &mesh_batch.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh_batch.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh_batch.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..mesh_batch.indices_len, 0, 0..1);
    }
}
