use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_camera);
    }
}

/// Flycam
/// Use WASDQE movement and right-click to look
#[derive(Component)]
pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    pub yaw: f32,   // degrees, around y
    pub pitch: f32, // degrees, around x
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.15,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn update_camera(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut query: Query<(&mut CameraController, &mut Transform)>,
) {
    let Ok((mut cam, mut transform)) = query.single_mut() else {
        return;
    };

    // Look
    if mouse_buttons.pressed(MouseButton::Right) {
        for event in mouse_motion.read() {
            cam.yaw -= event.delta.x * cam.sensitivity;
            cam.pitch -= event.delta.y * cam.sensitivity;
        }
    } else {
        mouse_motion.clear();
    }
    cam.pitch = cam.pitch.clamp(-89.0, 89.0);

    let rotation = Quat::from_euler(
        EulerRot::YXZ,
        cam.yaw.to_radians(),
        cam.pitch.to_radians(),
        0.0,
    );
    transform.rotation = rotation;

    // Movement
    let forward = transform.forward();
    let right = transform.right();
    let up = Vec3::Y;
    let dt = time.delta_secs();
    let speed = cam.speed * dt;

    if keyboard.pressed(KeyCode::KeyW) { transform.translation += *forward * speed; }
    if keyboard.pressed(KeyCode::KeyS) { transform.translation -= *forward * speed; }
    if keyboard.pressed(KeyCode::KeyD) { transform.translation += *right * speed; }
    if keyboard.pressed(KeyCode::KeyA) { transform.translation -= *right * speed; }
    if keyboard.pressed(KeyCode::KeyE) { transform.translation += up * speed; }
    if keyboard.pressed(KeyCode::KeyQ) { transform.translation -= up * speed; }
}
