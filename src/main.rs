use bevy::prelude::*;

pub mod camera;

// custom plugins here
use camera::CameraPlugin;

fn main() {
    println!("Hello World!!");

    let mut app = App::new();

    // add plugins here
    app
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraPlugin);

    // add systems here
    // each system should have it's own related subsystems, no need to bundle them in here 
    app
        .add_systems(Startup, setup)
        .add_systems(Update ,exit_system);

    // add resources here

    app.run();
}

// THIS IS A PLACEHOLDER, FOR TESTING STUFF OUT
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())), 
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));

    commands.spawn((
        PointLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}

fn exit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}