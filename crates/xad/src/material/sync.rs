use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use bezier::{Bvh, bounding_box, detect_faces, split_intersections};
use sketch::Sketch;

use super::debug;
use super::fill::{FaceMaterial, FaceMaterialPlugin};
use super::frame::PlaneFrame;
use super::stroke::{BezierMaterial, BezierMaterialPlugin};

// Margin around the sketch
const BBOX_PAD: f32 = 0.05;

/// Everything a `Sketch` needs to render: the stroke and fill materials,
/// the reactive rebuild, and the debug toggles.
pub struct SketchRenderPlugin;

impl Plugin for SketchRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BezierMaterialPlugin, FaceMaterialPlugin))
            .init_resource::<debug::ShowPolarityLabels>()
            .init_resource::<debug::ShowEdgeTint>()
            .init_resource::<debug::ShowBvh>()
            .add_systems(
                Update,
                (
                    debug::toggle_labels,
                    debug::toggle_edge_tint,
                    debug::toggle_bvh,
                    sync_sketch_render,
                    debug::position_labels,
                    debug::draw_bvh,
                )
                    .chain(),
            );
    }
}

// Tracks the render entities spawned for a sketch. The on_replace hook frees
// them whenever the component is overwritten by the next rebuild, removed, or
// its sketch entity despawns — the labels are not children, so they would
// otherwise outlive the sketch. (on_remove alone misses the overwrite path.)
#[derive(Component)]
#[component(on_discard = despawn_render)]
pub(super) struct SketchRender {
    stroke: Entity,
    fill: Option<Entity>,
    labels: Vec<Entity>,
}

fn despawn_render(mut world: DeferredWorld, ctx: HookContext) {
    let render = world.get::<SketchRender>(ctx.entity).unwrap();
    let entities: Vec<Entity> = std::iter::once(render.stroke)
        .chain(render.fill)
        .chain(render.labels.iter().copied())
        .collect();

    let mut commands = world.commands();
    for entity in entities {
        commands.entity(entity).try_despawn();
    }
}

// Reactively (re)builds a sketch's rendering: a stroke layer plus one filled
// layer per detected face. The mesh is sized to the curves' bounding box so
// sketches can use real-world coordinates.
pub(super) fn sync_sketch_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bezier_materials: ResMut<Assets<BezierMaterial>>,
    mut face_materials: ResMut<Assets<FaceMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderBuffer>>,
    edge_tint: Res<debug::ShowEdgeTint>,
    query: Query<
        (Entity, &Sketch, Option<&SketchRender>),
        Changed<Sketch>,
    >,
) {
    for (entity, sketch, _) in &query {
        // Replacing (or removing) SketchRender triggers its on_remove hook,
        // which despawns the previous stroke/fill/labels.
        let Some(bounds) = bounding_box(&sketch.curves) else {
            commands
                .entity(entity)
                .remove::<(SketchRender, debug::SketchBvh)>();
            continue;
        };
        let bounds = bounds.padded(BBOX_PAD);

        let normal = sketch.plane.normal();
        let (tangent, bitangent) = sketch.plane_basis();
        let frame = PlaneFrame {
            origin: Vec3::ZERO,
            tangent,
            bitangent,
        };
        let center = sketch.plane_offset(bounds.center());
        let mesh =
            meshes.add(Plane3d::new(normal, bounds.size() * 0.5));

        // Stroke layer.
        let stroke_material =
            bezier_materials.add(BezierMaterial::from_curves(
                &sketch.curves,
                frame,
                &mut storage_buffers,
            ));
        let stroke = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(stroke_material),
                Transform::from_translation(center),
                ChildOf(entity),
            ))
            .id();

        // Fill layer: faces of the intersection-split curves, coplanar
        // with the strokes; FaceMaterial's depth bias draws it on top.
        let split = split_intersections(sketch.curves.clone());
        let detected = detect_faces(&split);
        let fill = (!detected.faces.is_empty()).then(|| {
            let colors: Vec<LinearRgba> = detected
                .faces
                .iter()
                .map(|_| bezier::gen_color().with_alpha(0.55))
                .collect();
            let material = face_materials.add(FaceMaterial::new(
                &split,
                &detected.polarity,
                &colors,
                bounds,
                frame,
                edge_tint.0,
                &mut storage_buffers,
            ));
            commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(center),
                    ChildOf(entity),
                ))
                .id()
        });

        let labels = debug::spawn_labels(
            &mut commands,
            &split,
            &detected.polarity,
            frame,
        );

        commands.entity(entity).try_insert((
            SketchRender {
                stroke,
                fill,
                labels,
            },
            debug::SketchBvh(Bvh::build(&split)),
        ));
    }
}
