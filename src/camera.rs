use bevy::prelude::*;

use crate::sdf::SdfMaterial;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    // this is a required override from the base class :)
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, (update_camera, update_canvas));
    }
}

#[derive(Component)]
pub struct Camera {
    // this is a trackball camera, using the UEN convention
    // https://en.wikipedia.org/wiki/Spherical_coordinate_system
    pub target: Vec3,
    
    pub radius: f32,
    pub phi: f32, // degrees
    pub theta: f32, // degrees
    
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
            sensitivity: 0.3,
        }
    }
}

// setup the camera
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    let camera_entity = commands.spawn((
        Camera3d::default(),
        Camera::default(),
    )).id();

    commands.entity(camera_entity).with_children(|parent| {
        // this is the raymarching "canvas"
        // the thing about modern rendering pipelines is that the fragment shader only runs on rasterized pixels (faces on vertices)
        // as such, if we want every pixel to be calculated, we render a face that covers the entire viewport... 
        parent.spawn((
            Mesh3d(meshes.add(Rectangle::from_size(Vec2::splat(10.0)))),
            MeshMaterial3d(materials.add(SdfMaterial { camera_pos: Vec3::ZERO.extend(1.0) })),
            Transform {
                translation: Vec3::new(0.0, 0.0, -1.0),
                ..default()
            },
        ));
    });
}


// this function updates the camera transforms based on input from the mouse
// converts to spherical and then into cartesian
fn update_camera(
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
        camera.radius -= event.y;
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
    camera.theta = camera.theta.clamp(-89.0, 89.0);

    // convert to cartesian
    let phi_r = camera.phi.to_radians();
    let theta_r = camera.theta.to_radians();

    let x = camera.radius * theta_r.cos() * phi_r.sin();
    let y = camera.radius * theta_r.sin();
    let z = camera.radius * theta_r.cos() * phi_r.cos();

    // apply transformation
    transform.translation = camera.target + Vec3::new(x, y, z);
    transform.look_at(camera.target, Vec3::Y);
}


// this function updates the screen-space canvas plane thing
fn update_canvas(
    window_q: Query<&Window>,
    camera_q: Query<(&Transform, &Projection), With<Camera>>,
    mut canvas_q: Query<&mut Transform, (With<MeshMaterial3d<SdfMaterial>>, Without<Camera>)>, // canvas
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    // todo: THIS CONVENTION IS BAD PRACTICE, NEED TO CHANGE, CAUSES SILENT FAILURES!!!
    let Ok(window) = window_q.single() else { return; };
    let Ok((camera_transform, projection)) = camera_q.single() else { return; };
    let Ok(mut canvas_transform) = canvas_q.single_mut() else { return; };

    // update shader camera uniform 
    for material in materials.iter_mut() {
        // we only need the camera position for shader calculations
        // direction is not needed; you can ask Marcel why
        material.1.camera_pos = camera_transform.translation.extend(1.0);
    }

    // scale canvas to viewport
    if let Projection::Perspective(persp) = projection {
        let distance = canvas_transform.translation.z.abs();
        let height = 2.0 * distance * (persp.fov / 2.0).tan();
        let width = height * (window.width() / window.height());
        
        canvas_transform.scale = Vec3::new(width, height, 1.0);
    }
}