use bevy::core_pipeline::core_3d::graph::Node3d;
use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_graph::{InternedRenderLabel, RenderLabel};
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FullscreenMaterialPlugin::<SdfFullscreenMaterial>::default());
    }
}

#[derive(Component, ExtractComponent, Clone, Copy, Debug, ShaderType, Default)]
pub struct SdfFullscreenMaterial {
    pub camera_pos: Vec4,
    pub camera_dir: Vec4,
    pub camera_up: Vec4,
    pub resolution: Vec2,
    pub fov: f32,
    pub _padding: f32,
}

impl FullscreenMaterial for SdfFullscreenMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sdf_fullscreen.wgsl".into()
    }

    fn node_edges() -> Vec<InternedRenderLabel> {
        vec![
            // This runs after tonemapping
            //  even though we ignore the screen texture
            Node3d::Tonemapping.intern(),
            Self::node_label().intern(),
            Node3d::EndMainPassPostProcessing.intern(),
        ]
    }
}
