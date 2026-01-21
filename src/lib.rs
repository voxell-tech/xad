use bevy::prelude::*;

pub mod camera;
pub mod sdf;

use camera::CameraPlugin;
use sdf::SdfPlugin;

pub struct XadPlugin;

impl Plugin for XadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins)
            .add_plugins(CameraPlugin)
            .add_plugins(SdfPlugin);

        app.add_systems(Update, exit_system);
    }
}

fn exit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
