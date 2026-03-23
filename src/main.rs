use bevy::color::palettes::css;
use bevy::prelude::*;
use xad::XadPlugin;
use xad::camera::CameraController;
use xad::sdf::SdfCamera;
use xad::sdf::boolean::SdfGroup;
use xad::sdf::primitves::{
    SdfCapsule, SdfCuboid, SdfRoundCuboid, SdfSphere, SdfTorus,
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

    // Ungrouped primitive grid
    for z in 0..3 {
        for y in 0..3 {
            for x in 0..5 {
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
                } else if x == 3 {
                    cmd.insert(SdfCapsule {
                        point_a: Vec3::new(0.0, -0.25, 0.0),
                        point_b: Vec3::new(0.0, 0.25, 0.0),
                        radius: 0.3,
                    });
                } else {
                    cmd.insert(SdfTorus {
                        ring_radius: 0.6,
                        tube_radius: 0.3,
                    });
                }
            }
        }
    }

    // Boolean groups
    // Difference: sphere - cube
    let sphere = commands
        .spawn((SdfSphere { radius: 0.6 }, SdfTransform::default()))
        .id();
    let cube = commands
        .spawn((
            SdfCuboid {
                extents: Vec3::splat(0.4),
            },
            SdfTransform::default(),
        ))
        .id();
    commands.spawn((
        SdfGroup::new().add(sphere).subtract(cube),
        SdfTransform::default()
            .with_translation(Vec3::new(0.0, 0.0, -3.0)),
    ));

    // Intersection: capsule ∩ torus
    let capsule = commands
        .spawn((
            SdfCapsule {
                point_a: Vec3::new(0.0, -0.5, 0.0),
                point_b: Vec3::new(0.0, 0.5, 0.0),
                radius: 0.4,
            },
            SdfTransform::default(),
        ))
        .id();
    let torus = commands
        .spawn((
            SdfTorus {
                ring_radius: 0.5,
                tube_radius: 0.25,
            },
            SdfTransform::default(),
        ))
        .id();
    commands.spawn((
        SdfGroup::new().add(capsule).intersect(torus),
        SdfTransform::default()
            .with_translation(Vec3::new(2.0, 0.0, -3.0)),
    ));

    // Exclusion: sphere XOR sphere
    let sphere_a = commands
        .spawn((
            SdfSphere { radius: 0.45 },
            SdfTransform::default()
                .with_translation(Vec3::new(-0.2, 0.0, 0.0)),
        ))
        .id();
    let sphere_b = commands
        .spawn((
            SdfSphere { radius: 0.45 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.2, 0.0, 0.0)),
        ))
        .id();
    commands.spawn((
        SdfGroup::new().add(sphere_a).exclude(sphere_b),
        SdfTransform::default()
            .with_translation(Vec3::new(4.0, 0.0, -3.0)),
    ));

    // Multiple subtractions: sphere - cube_top - capsule
    let sphere = commands
        .spawn((SdfSphere { radius: 0.65 }, SdfTransform::default()))
        .id();
    let cube_top = commands
        .spawn((
            SdfCuboid {
                extents: Vec3::new(0.4, 0.3, 0.4),
            },
            SdfTransform::default()
                .with_translation(Vec3::new(0.0, 0.35, 0.0)),
        ))
        .id();
    let capsule = commands
        .spawn((
            SdfCapsule {
                point_a: Vec3::new(-0.8, 0.0, 0.0),
                point_b: Vec3::new(0.8, 0.0, 0.0),
                radius: 0.18,
            },
            SdfTransform::default(),
        ))
        .id();
    commands.spawn((
        SdfGroup::new()
            .add(sphere)
            .subtract(cube_top)
            .subtract(capsule),
        SdfTransform::default()
            .with_translation(Vec3::new(-2.0, 0.0, -3.0)),
    ));

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
