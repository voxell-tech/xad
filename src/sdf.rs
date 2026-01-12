use bevy::prelude::*;
use bevy::render::render_resource::*;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(MaterialPlugin::<SdfMaterial>::default());
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct SdfMaterial {
    // shader uniforms are variables that get passed to the GPU
    // we handle these in the shader
    
    // #[uniform(0)]
    // pub camera_pos: Vec3,

    // this is so stupid; we need to use vec4 to match 16-byte required alignment??
    // todo: need to look into this more, surely there is a better way?
    #[uniform(0)]
    pub camera_pos: Vec4, 

    // when we need to pass more uniforms into the SDF shader,
    //  add them here
}

impl Material for SdfMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sdf_test.wgsl".into()
    }
}