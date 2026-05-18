use bevy::color::palettes::css;
use bevy::prelude::*;
use xad::XadPlugin;
use xad::camera::CameraController;
use xad::sdf::SdfCamera;
use xad::sdf::boolean::{IntoSdfNode, SdfGroup};
use xad::sdf::primitves::{
    SdfSphere,
    // SdfCapsule,
    // SdfCuboid,
    // SdfTorus,
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

    // Boolean groups
    // Difference: sphere_a - sphere_b
    commands
        .spawn(
            SdfTransform::default()
                .with_translation(Vec3::new(-2.0, 0.0, 0.0)),
        )
        .queue(
            SdfGroup::new()
                .add(
                    (
                        SdfSphere { radius: 0.6 },
                        SdfTransform::default(),
                    )
                        .union(),
                )
                .add(
                    (
                        SdfSphere { radius: 0.6 },
                        SdfTransform::default().with_translation(
                            Vec3::new(0.3, 0.0, 0.0),
                        ),
                    )
                        .difference(),
                ),
        );

    // Intersection: sphere_a n sphere_b
    commands
        .spawn(
            SdfTransform::default()
                .with_translation(Vec3::new(0.0, 0.0, 0.0)),
        )
        .queue(
            SdfGroup::new()
                .add(
                    (
                        SdfSphere { radius: 0.5 },
                        SdfTransform::default().with_translation(
                            Vec3::new(-0.2, 0.0, 0.0),
                        ),
                    )
                        .union(),
                )
                .add(
                    (
                        SdfSphere { radius: 0.5 },
                        SdfTransform::default().with_translation(
                            Vec3::new(0.2, 0.0, 0.0),
                        ),
                    )
                        .intersect(),
                ),
        );

    // Exclusion: (sphere_a XOR sphere_b) - sphere_c
    commands
        .spawn(
            SdfTransform::default()
                .with_translation(Vec3::new(2.0, 0.0, 0.0)),
        )
        .queue(
            SdfGroup::new()
                .add(
                    (
                        SdfSphere { radius: 0.45 },
                        SdfTransform::default().with_translation(
                            Vec3::new(-0.2, 0.0, 0.0),
                        ),
                    )
                        .union(),
                )
                .add(
                    (
                        SdfSphere { radius: 0.45 },
                        SdfTransform::default().with_translation(
                            Vec3::new(0.2, 0.0, 0.0),
                        ),
                    )
                        .exclude(),
                )
                .add(
                    (
                        SdfSphere { radius: 0.3 },
                        SdfTransform::default().with_translation(
                            Vec3::new(0.0, 0.3, 0.0),
                        ),
                    )
                        .difference(),
                ),
        );

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::default(),
    ));
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
