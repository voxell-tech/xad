//! Example: one plane whose Bezier control points wander randomly.
//!
//! Showcases updating a `BezierMaterial`'s GPU buffer every frame to animate
//! curves in realtime, rather than baking them once at spawn time.

use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use bezier::BezierCurve;
use sketch::Sketch;
use xad::material::{
    BezierMaterial, BezierMaterialPlugin, construct_bezier_material,
};

const CURVE_COLORS: [LinearRgba; 3] = [
    LinearRgba::new(1.00, 0.45, 0.10, 1.0),
    LinearRgba::new(0.20, 0.80, 1.00, 1.0),
    LinearRgba::new(0.90, 0.40, 1.00, 1.0),
];
const CURVE_WIDTH: f32 = 0.008;
const POINT_BOUNDS_MIN: f32 = 0.05;
const POINT_BOUNDS_MAX: f32 = 0.95;
const MAX_SPEED: f32 = 0.35;
const ACCEL: f32 = 0.6;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BezierMaterialPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, wander_control_points)
        .run();
}

// PCG32 PRNG (https://www.pcg-random.org/)
struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (seed << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.state = old_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted =
            (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

#[derive(Resource)]
struct WanderingCurves {
    material: Handle<BezierMaterial>,
    background: LinearRgba,
    points: Vec<[Vec2; 4]>,
    velocities: Vec<[Vec2; 4]>,
    rng: Rng,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BezierMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 3.5, 3.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let background = LinearRgba::new(0.05, 0.05, 0.08, 0.0);

    let mut rng = Rng::new(67676767); // lol
    let points: Vec<[Vec2; 4]> = (0..CURVE_COLORS.len())
        .map(|_| {
            [
                Vec2::new(
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                ),
                Vec2::new(
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                ),
                Vec2::new(
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                ),
                Vec2::new(
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                    rng.range(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX),
                ),
            ]
        })
        .collect();
    let velocities = vec![[Vec2::ZERO; 4]; points.len()];

    let curves: Vec<BezierCurve> = points
        .iter()
        .enumerate()
        .flat_map(|(i, p)| {
            BezierCurve::cubic(p[0], p[1], p[2], p[3]).map(|c| {
                c.with_color(CURVE_COLORS[i]).with_width(CURVE_WIDTH)
            })
        })
        .collect();

    let sketch = Sketch {
        curves,
        position: Vec3::ZERO,
        background_color: background,
    };
    let material = materials.add(construct_bezier_material(
        &sketch,
        &mut storage_buffers,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(2.5)))),
        MeshMaterial3d(material.clone()),
    ));

    commands.insert_resource(WanderingCurves {
        material,
        background,
        points,
        velocities,
        rng,
    });
}

fn wander_control_points(
    time: Res<Time>,
    mut wandering: ResMut<WanderingCurves>,
    mut materials: ResMut<Assets<BezierMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let dt = time.delta_secs();
    let WanderingCurves {
        points,
        velocities,
        rng,
        ..
    } = &mut *wandering;

    for (point, velocity) in
        points.iter_mut().zip(velocities.iter_mut())
    {
        for (p, v) in point.iter_mut().zip(velocity.iter_mut()) {
            // random accel values
            *v +=
                Vec2::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0))
                    * ACCEL
                    * dt;
            *v = v.clamp_length_max(MAX_SPEED);
            *p += *v * dt;

            if p.x < POINT_BOUNDS_MIN || p.x > POINT_BOUNDS_MAX {
                p.x = p.x.clamp(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX);
                v.x = -v.x;
            }
            if p.y < POINT_BOUNDS_MIN || p.y > POINT_BOUNDS_MAX {
                p.y = p.y.clamp(POINT_BOUNDS_MIN, POINT_BOUNDS_MAX);
                v.y = -v.y;
            }
        }
    }

    let curves: Vec<BezierCurve> = points
        .iter()
        .enumerate()
        .flat_map(|(i, p)| {
            BezierCurve::cubic(p[0], p[1], p[2], p[3]).map(|c| {
                c.with_color(CURVE_COLORS[i]).with_width(CURVE_WIDTH)
            })
            // .with_debug(true)
        })
        .collect();

    if let Some(material) = materials.get(&wandering.material) {
        material.update_curves(
            wandering.background,
            &curves,
            &mut storage_buffers,
        );
    }

    // doesnt automatically reload the material, so mark it to force a rebuild
    // NOTE: might be expensive
    materials.get_mut(&wandering.material);
}
