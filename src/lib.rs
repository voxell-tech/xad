use bevy::prelude::*;

pub mod camera;
pub mod sdf;
pub mod ui;

use camera::CameraPlugin;
use sdf::SdfPlugin;
use ui::UiPlugin;

pub struct XadPlugin;

impl Plugin for XadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SdfPlugin, CameraPlugin, UiPlugin));
    }
}
