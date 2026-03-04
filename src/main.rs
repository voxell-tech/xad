use bevy::color::palettes::css;
use bevy::prelude::*;
use xad::XadPlugin;
use xad::camera::CameraController;
use xad::sdf::SdfCamera;
use xad::sdf::primitves::{
    SdfCapsule, SdfCuboid, SdfRoundCuboid, SdfSphere,
};
use xad::sdf::transform::SdfTransform;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, XadPlugin));
    app.add_systems(Startup, test_setup)
        .add_systems(Update, (rotate_sdf, xyz_gizmos))
        .add_systems(Last, exit);

    app.run();
}

// TODO(nixon): This setup is just for testing purposes.
fn test_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn SDF camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
        CameraController::default(),
        SdfCamera::default(),
    ));

    for z in 0..3 {
        for y in 0..3 {
            for x in 0..4 {
                let (xf, yf, zf) = (x as f32, y as f32, z as f32);
                let mut cmd = commands.spawn(SdfTransform {
                    translation: Vec3::new(xf, yf, zf),
                    scale: 1.0 - xf * 0.1 - yf * 0.1 - zf * 0.1,
                    ..default()
                });

                if x == 0 {
                    cmd.insert(SdfSphere { radius: 0.4 });
                } else if x == 1 {
                    cmd.insert(SdfCuboid {
                        extents: Vec3::new(0.4, 0.3, 0.4),
                    });
                } else if x == 2 {
                    cmd.insert(SdfRoundCuboid {
                        extents: Vec3::new(0.4, 0.3, 0.4),
                        radius: 0.3 - yf * 0.1,
                    });
                } else {
                    cmd.insert(SdfCapsule {
                        point_a: Vec3::new(0.0, -0.25, 0.0),
                        point_b: Vec3::new(0.0, 0.25, 0.0),
                        radius: 0.3,
                    });
                }
            }
        }
    }

    let mesh =
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0))));
    let mat =
        MeshMaterial3d(materials.add(StandardMaterial::default()));
    commands.spawn((mesh, mat, Transform::default()));
}

fn rotate_sdf(
    mut q_transforms: Query<&mut SdfTransform>,
    time: Res<Time>,
) {
    for (i, mut transform) in q_transforms.iter_mut().enumerate() {
        transform.rotation = Quat::from_rotation_x(
            time.elapsed_secs() + (i as f32) * 0.2,
        );
    }
}

fn xyz_gizmos(mut gizmos: Gizmos) {
    gizmos.arrow(Vec3::ZERO, Vec3::X, css::RED);
    gizmos.arrow(Vec3::ZERO, Vec3::Y, css::GREEN);
    gizmos.arrow(Vec3::ZERO, Vec3::Z, css::BLUE);
}

fn exit(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
