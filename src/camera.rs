use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    // this is a required override from the base class :)
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, update);
    }
}

#[derive(Component)]
pub struct Camera {
    // this is a trackball camera, using the UEN convention
    // https://en.wikipedia.org/wiki/Spherical_coordinate_system
    pub target: Vec3,
    
    pub radius: f32,
    pub phi: f32, // radians
    pub theta: f32, // radians
    
    pub sensitivity: f32, 
}

impl Default for Camera {
    // these are the default values for our camera
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 10.0,
            phi: 0.0,
            theta: 0.0,
            sensitivity: 0.01,
        }
    }
}

// setup the camera
fn setup(
    mut commands: Commands,
) {
    commands.spawn((
        Camera3d::default(),
        Camera { // our camera
            // todo: these initial transforms are arbitrary;
            //  we should probably move them to a separate config file
            ..default()
        },
    ));
}

fn update(
    // mouse input i guess, i'd rather everything be handled in this class
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut mouse_wheel: EventReader<bevy::input::mouse::MouseWheel>,
    mut query: Query<(&mut Transform, &mut Camera)>,
) {
    let Ok((mut transform, mut camera)) = query.single_mut() else { return; };

    // handle input
    // scroll wheel zoom
    for event in mouse_wheel.read() {
        camera.radius -= event.y * 1.5;
        camera.radius = camera.radius.clamp(1.0, 50.0);
    }

    // we only want to rotate when the mouse is held
    if mouse_buttons.pressed(MouseButton::Right) {
        for event in mouse_motion.read() {
            camera.phi -= event.delta.x * camera.sensitivity;
            camera.theta += event.delta.y * camera.sensitivity;
        }
    }
    // clamp elevation
    camera.theta = camera.theta.clamp(-1.54, 1.54);

    let x = camera.radius * camera.theta.cos() * camera.phi.sin();
    let y = camera.radius * camera.theta.sin();
    let z = camera.radius * camera.theta.cos() * camera.phi.cos();

    transform.translation = camera.target + Vec3::new(x, y, z);
    transform.look_at(camera.target, Vec3::Y);
}