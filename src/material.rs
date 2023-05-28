use std::{collections::BTreeSet, fmt::Debug};

use wgpu::{
    include_wgsl, BindGroup, BindGroupLayout, Device, PrimitiveTopology, ShaderModuleDescriptor,
};

use crate::{
    arena::ArenaId,
    mesh::{Mesh, MeshAttribute},
    RenderBuddy,
};

pub type MaterialHandle = ArenaId;

#[derive(Debug)]
pub struct DefaultMat {}
impl Material for DefaultMat {}

pub trait Material: Debug {
    fn shader(&self) -> ShaderModuleDescriptor {
        include_wgsl!("./default_shaders/default.wgsl")
    }

    fn vertex_attributes(&self) -> BTreeSet<MeshAttribute> {
        BTreeSet::from([
            MeshAttribute::Position,
            MeshAttribute::UV,
            MeshAttribute::Color,
        ])
    }

    fn get_bind_group_layouts(&self, device: &Device) -> Vec<BindGroupLayout> {
        Vec::default()
    }

    fn get_bind_groups(&self, mesh: &Mesh, rb: &RenderBuddy) -> Vec<BindGroup> {
        Vec::default()
    }

    fn topology(&self) -> PrimitiveTopology {
        PrimitiveTopology::TriangleList
    }

    fn has_texture(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "Default Material"
    }
}
