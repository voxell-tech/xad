use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

#[derive(Component, Reflect, ShaderType, Debug, Clone, Copy)]
pub struct Cuboid {
    pub extents: Vec3,
}

impl Cuboid {
    pub const ONE: Self = Self { extents: Vec3::ONE };
}

#[derive(Component, Reflect, ShaderType, Debug, Clone, Copy)]
pub struct Sphere {
    pub radius: f32,
}

impl Sphere {
    pub const ONE: Self = Self { radius: 1.0 };
}
