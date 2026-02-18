use bevy::prelude::*;
use bevy::render::extract_component::{
    ExtractComponent, ExtractComponentPlugin,
};
use bevy::render::render_resource::ShaderType;

pub struct SdfPrimitivePlugin;

impl Plugin for SdfPrimitivePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // TODO: Change to extract visible only?
            ExtractComponentPlugin::<SdfSphere>::default(),
            ExtractComponentPlugin::<SdfCuboid>::default(),
            ExtractComponentPlugin::<SdfRoundCuboid>::default(),
        ));
    }
}

#[derive(
    ExtractComponent,
    Component,
    Reflect,
    ShaderType,
    Debug,
    Clone,
    Copy,
)]
pub struct SdfSphere {
    pub radius: f32,
}

impl SdfSphere {
    pub const ONE: Self = Self { radius: 1.0 };
}

impl Default for SdfSphere {
    fn default() -> Self {
        Self::ONE
    }
}

#[derive(
    ExtractComponent,
    Component,
    Reflect,
    ShaderType,
    Debug,
    Clone,
    Copy,
)]
pub struct SdfCuboid {
    pub extents: Vec3,
}

impl SdfCuboid {
    pub const ONE: Self = Self { extents: Vec3::ONE };
}

impl Default for SdfCuboid {
    fn default() -> Self {
        Self::ONE
    }
}

#[derive(
    ExtractComponent,
    Component,
    Reflect,
    ShaderType,
    Debug,
    Clone,
    Copy,
)]
pub struct SdfRoundCuboid {
    pub extents: Vec3,
    pub radius: f32,
}

impl SdfRoundCuboid {
    pub const ONE: Self = Self {
        extents: Vec3::ONE,
        radius: 1.0,
    };
}

impl Default for SdfRoundCuboid {
    fn default() -> Self {
        Self::ONE
    }
}
