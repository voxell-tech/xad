use bevy::prelude::*;

pub mod camera;
pub mod sdf;

// custom plugins here
use camera::CameraPlugin;
use sdf::SdfPlugin;

fn main() {
    println!("Hello World!!");

    let mut app = App::new();

    // add plugins here
    app.add_plugins(DefaultPlugins)
        .add_plugins(CameraPlugin)
        .add_plugins(SdfPlugin);

    // add systems here
    // each system should have it's own related subsystems, no need to bundle them in here
    app.add_systems(Update, exit_system);

    // add resources here

    app.run();
}

fn exit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
