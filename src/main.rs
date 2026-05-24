use bevy::color::palettes::css;
use bevy::prelude::*;
use xad::XadPlugin;
use xad::camera::CameraController;
use xad::sdf::SdfCamera;
use xad::sdf::boolean::{SdfGroup, SdfOperandOf, SdfOperands};
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
    let sphere_a = commands
        .spawn((SdfSphere { radius: 0.6 }, SdfTransform::default()))
        .id();
    let sphere_b = commands
        .spawn((
            SdfSphere { radius: 0.6 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.3, 0.0, 0.0)),
        ))
        .id();
    commands.spawn((
        SdfGroup::new(sphere_a).difference(sphere_b),
        SdfTransform::default()
            .with_translation(Vec3::new(-2.0, 0.0, 0.0)),
    ));

    // Intersection: sphere_a n sphere_b
    let sphere_a = commands
        .spawn((
            SdfSphere { radius: 0.5 },
            SdfTransform::default()
                .with_translation(Vec3::new(-0.2, 0.0, 0.0)),
        ))
        .id();
    let sphere_b = commands
        .spawn((
            SdfSphere { radius: 0.5 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.2, 0.0, 0.0)),
        ))
        .id();
    commands.spawn((
        SdfGroup::new(sphere_a).intersect(sphere_b),
        SdfTransform::default(),
    ));

    // Exclusion: (sphere_a XOR sphere_b) - sphere_c
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
    let sphere_c = commands
        .spawn((
            SdfSphere { radius: 0.3 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.0, 0.3, 0.0)),
        ))
        .id();
    commands.spawn((
        SdfGroup::new(sphere_a)
            .exclude(sphere_b)
            .difference(sphere_c),
        SdfTransform::default()
            .with_translation(Vec3::new(2.0, 0.0, 0.0)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::default(),
    ));

    // Nested boolean test (2 levels): big_sphere - (medium_sphere - small_sphere)
    let nested_big_sphere = commands
        .spawn((SdfSphere { radius: 0.6 }, SdfTransform::default()))
        .id();

    let nested_medium_sphere = commands
        .spawn((
            SdfSphere { radius: 0.45 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.25, 0.0, 0.0)),
        ))
        .id();

    let nested_small_sphere = commands
        .spawn((
            SdfSphere { radius: 0.25 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.25, 0.0, 0.0)),
        ))
        .id();

    let nested_inner_group = commands
        .spawn((
            SdfGroup::new(nested_medium_sphere)
                .difference(nested_small_sphere),
            SdfTransform::default(),
        ))
        .id();

    commands.spawn((
        SdfGroup::new(nested_big_sphere)
            .difference(nested_inner_group),
        SdfTransform::default()
            .with_translation(Vec3::new(0.0, 1.5, 0.0)),
    ));

    // Nested boolean test (3 levels): big_sphere - (small_sphere - (tiny_sphere_a - tiny_sphere_b))
    let nested3_big_sphere = commands
        .spawn((SdfSphere { radius: 0.6 }, SdfTransform::default()))
        .id();

    let nested3_small_sphere = commands
        .spawn((
            SdfSphere { radius: 0.4 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.4, 0.0, 0.0)),
        ))
        .id();

    let nested3_tiny_sphere_a = commands
        .spawn((
            SdfSphere { radius: 0.25 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.7, 0.0, 0.0)),
        ))
        .id();

    let nested3_tiny_sphere_b = commands
        .spawn((
            SdfSphere { radius: 0.12 },
            SdfTransform::default()
                .with_translation(Vec3::new(0.7, 0.0, 0.0)),
        ))
        .id();

    // Level 3: nested3_tiny_sphere_a - nested3_tiny_sphere_b
    let nested3_level3_group = commands
        .spawn((
            SdfGroup::new(nested3_tiny_sphere_a)
                .difference(nested3_tiny_sphere_b),
            SdfTransform::default(),
        ))
        .id();

    // Level 2: nested3_small_sphere - nested3_level3_group
    let nested3_level2_group = commands
        .spawn((
            SdfGroup::new(nested3_small_sphere)
                .difference(nested3_level3_group),
            SdfTransform::default(),
        ))
        .id();

    // Level 1: nested3_big_sphere - nested3_level2_group
    commands.spawn((
        SdfGroup::new(nested3_big_sphere)
            .difference(nested3_level2_group),
        SdfTransform::default()
            .with_translation(Vec3::new(0.0, -1.5, 0.0)),
    ));
}

fn rotate_sdf(
    mut q_transforms: Query<
        &mut SdfTransform,
        (With<SdfOperands>, Without<SdfOperandOf>),
    >,
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
