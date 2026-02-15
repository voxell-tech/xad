use bevy::prelude::*;

pub mod camera;
pub mod sdf;

use camera::CameraPlugin;
use sdf::SdfPlugin;

pub struct XadPlugin;

impl Plugin for XadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SdfPlugin, CameraPlugin));
    }
}
