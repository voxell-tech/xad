use bevy::prelude::*;

pub mod camera;
pub mod material;
pub mod sdf;

use camera::CameraPlugin;
use material::BezierMaterialPlugin;
use sdf::SdfPlugin;

pub struct XadPlugin;

impl Plugin for XadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SdfPlugin,
            CameraPlugin,
            BezierMaterialPlugin,
        ));
    }
}
