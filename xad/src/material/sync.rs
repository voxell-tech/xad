use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use bezier::{bounding_box, detect_faces, split_intersections};
use sketch::Sketch;

use super::debug;
use super::fill::FaceMaterial;
use super::frame::PlaneFrame;
use super::stroke::BezierMaterial;

// Margin around the sketch
const BBOX_PAD: f32 = 0.05;
// TODO: fix this so we do z-index, not just offset the plane
const FACE_LAYER_OFFSET: f32 = 0.001;

// Tracks the render child-entities spawned for a sketch so they can be
// replaced when the sketch changes.
#[derive(Component)]
pub(super) struct SketchRender {
    stroke: Entity,
    fill: Option<Entity>,
    labels: Vec<Entity>,
}

// Reactively (re)builds a sketch's rendering: a stroke layer plus one filled
// layer per detected face. The mesh is sized to the curves' bounding box so
// sketches can use real-world coordinates.
pub(super) fn sync_sketch_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bezier_materials: ResMut<Assets<BezierMaterial>>,
    mut face_materials: ResMut<Assets<FaceMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    query: Query<
        (Entity, &Sketch, Option<&SketchRender>),
        Changed<Sketch>,
    >,
) {
    for (entity, sketch, prev) in &query {
        if let Some(prev) = prev {
            commands.entity(prev.stroke).despawn();
            if let Some(fill) = prev.fill {
                commands.entity(fill).despawn();
            }
            for &label in &prev.labels {
                commands.entity(label).despawn();
            }
        }

        let Some(bounds) = bounding_box(&sketch.curves) else {
            commands.entity(entity).remove::<SketchRender>();
            continue;
        };
        let bounds = bounds.padded(BBOX_PAD);

        let normal = sketch.plane.normal();
        let (tangent, bitangent) = sketch.plane_basis();
        let frame = PlaneFrame {
            origin: sketch.get_position(),
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

        // Fill layer: faces of the intersection-split curves, all drawn by
        // one material slightly in front of the strokes.
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
                frame,
                &mut storage_buffers,
            ));
            let front = center + normal * FACE_LAYER_OFFSET;
            commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(front),
                    ChildOf(entity),
                ))
                .id()
        });

        let labels = if debug::POLARITY_LABELS {
            debug::spawn_labels(
                &mut commands,
                &split,
                &detected.polarity,
                frame,
            )
        } else {
            Vec::new()
        };

        commands.entity(entity).try_insert(SketchRender {
            stroke,
            fill,
            labels,
        });
    }
}
