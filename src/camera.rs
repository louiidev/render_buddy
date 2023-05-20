use glam::{Mat4, Quat, Vec2, Vec3};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Device};

const DEFAULT_ORTHO_CAMERA_DEPTH: f32 = 1000.0;

pub struct CameraData {
    projection: Projection,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub(crate) bind_group_layout: BindGroupLayout,
}

#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraData {
    pub(crate) fn new(surface_size: (u32, u32), bind_group_layout: BindGroupLayout) -> Self {
        Self {
            projection: Projection::Orthographic {
                size: Vec3::new(
                    surface_size.0 as f32,
                    surface_size.1 as f32,
                    DEFAULT_ORTHO_CAMERA_DEPTH,
                ),
                origin: CameraOrigin::default(),
                target_resolution: None,
            },
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.),
            bind_group_layout,
        }
    }

    pub(crate) fn create_bind_group(&self, device: &Device) -> BindGroup {
        let projection = self.projection.compute_projection_matrix();
        let view = Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);
        let inverse_view = view.inverse();
        let view_projection = projection * inverse_view;

        let camera_uniform = CameraUniform {
            view_proj: view_projection.to_cols_array_2d(),
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("View Buffer"),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            contents: bytemuck::cast_slice(&[camera_uniform]),
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera bind group"),
            layout: &self.bind_group_layout,
        })
    }

    pub(crate) fn resize(&mut self, surface_size: (u32, u32)) {
        if let Projection::Orthographic { ref mut size, .. } = &mut self.projection {
            *size = Vec3::new(surface_size.0 as f32, surface_size.1 as f32, size.z);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum CameraOrigin {
    #[default]
    Center,
    TopLeft,
}

#[derive(Debug, Clone)]
pub enum Projection {
    Orthographic {
        size: Vec3,
        origin: CameraOrigin,
        target_resolution: Option<Vec2>,
    },
    Perspective {
        /// Vertical field of view in degrees.
        vfov: f32,
        /// Near plane distance. All projection uses a infinite far plane.
        near: f32,
    },
    Custom(Mat4),
}

impl Projection {
    pub(crate) fn compute_projection_matrix(&self) -> Mat4 {
        match self {
            Projection::Orthographic {
                size,
                origin,
                target_resolution,
            } => {
                let (width, height) = if let Some(Vec2 {
                    x: target_width,
                    y: target_height,
                }) = target_resolution.to_owned()
                {
                    let Vec3 {
                        x: width,
                        y: height,
                        ..
                    } = size;
                    if width * target_height < target_width * height {
                        (width * target_height / height, target_height)
                    } else {
                        (target_width, height * target_width / width)
                    }
                } else {
                    (size.x, size.y)
                };

                let near = size.z / 2.0;
                match origin {
                    CameraOrigin::Center => {
                        let half_width = width / 2.0;
                        let half_height = height / 2.0;

                        Mat4::orthographic_rh(
                            -half_width,
                            half_width,
                            -half_height,
                            half_height,
                            -near,
                            DEFAULT_ORTHO_CAMERA_DEPTH,
                        )
                    }
                    CameraOrigin::TopLeft => Mat4::orthographic_rh(
                        0.,
                        width,
                        height,
                        0.,
                        near,
                        DEFAULT_ORTHO_CAMERA_DEPTH,
                    ),
                }
            }
            Projection::Perspective { vfov, near } => {
                Mat4::perspective_infinite_reverse_rh(vfov.to_radians(), 1.0, *near)
            }
            Projection::Custom(custom) => *custom,
        }
    }
}
