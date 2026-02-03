use bevy::prelude::*;

use crate::sdf::SdfCamera;

// use crate::sdf::SdfFullscreenMaterial;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, update_camera);
    }
}

/// Trackball camera, using the UEN convention.
///
/// <https://en.wikipedia.org/wiki/Spherical_coordinate_system>
#[derive(Component)]
pub struct SdfCameraController {
    pub target: Vec3,
    pub radius: f32,
    pub phi: f32,   // Azimuth in degrees
    pub theta: f32, // Elevation in degrees
    pub sensitivity: f32,
}

impl Default for SdfCameraController {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 5.0,
            phi: 0.0,
            theta: 0.0,
            sensitivity: 0.3,
        }
    }
}

fn setup(mut commands: Commands) {
    // Camera3d with the FullscreenMaterial component
    // The SdfFullscreenMaterial runs as a post-process, overwriting the (empty) 3D scene
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
        SdfCameraController::default(),
        SdfCamera::default(),
    ));
}

fn update_camera(
    window_q: Query<&Window>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut mouse_wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut query: Query<(&mut SdfCameraController, &mut SdfCamera)>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    let Ok((mut camera, mut uniform)) = query.single_mut() else {
        return;
    };

    // Zoom
    for event in mouse_wheel.read() {
        camera.radius -= event.y * 0.5;
        camera.radius = camera.radius.clamp(1.0, 50.0);
    }

    // Rotate
    if mouse_buttons.pressed(MouseButton::Right) {
        for event in mouse_motion.read() {
            camera.phi -= event.delta.x * camera.sensitivity;
            camera.theta += event.delta.y * camera.sensitivity;
        }
    }

    // Clamp elevation
    camera.theta = camera.theta.clamp(-89.0, 89.0);

    // Convert spherical to Cartesian
    let phi_r = camera.phi.to_radians();
    let theta_r = -camera.theta.to_radians();

    let x = camera.radius * theta_r.cos() * phi_r.sin();
    let y = camera.radius * theta_r.sin();
    let z = camera.radius * theta_r.cos() * phi_r.cos();

    let camera_pos = camera.target + Vec3::new(x, y, z);
    let camera_dir = (camera.target - camera_pos).normalize();
    let camera_right = camera_dir.cross(Vec3::Y).normalize();
    let camera_up = camera_right.cross(camera_dir).normalize();

    // Update the fullscreen material uniforms
    uniform.camera_pos = camera_pos.extend(1.0);
    uniform.camera_dir = camera_dir.extend(0.0);
    uniform.camera_up = camera_up.extend(0.0);
    uniform.resolution = Vec2::new(window.width(), window.height());
    uniform.fov = std::f32::consts::FRAC_PI_4;
}
