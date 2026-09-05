pub mod color;
pub mod features;

use bevy::prelude::*;
use command_system::{
    Feature, FeatureContext, FeatureError, FeatureKind, FeatureOutput,
};

use crate::bezier::SketchCurves;

const SKETCH_HALF_EXTENT: f32 = 1.0;
pub(crate) const SKETCH_BACKGROUND: LinearRgba =
    LinearRgba::new(0.06, 0.06, 0.09, 1.0);

// the default planes in any timeline
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Plane(Vec3); // TODO: complete
// with top, bottom, right (+mirrored direction) vecs

impl Plane {
    pub fn top() -> Self {
        Plane(Vec3::Y)
    }

    pub fn front() -> Self {
        Plane(Vec3::X)
    }

    pub fn right() -> Self {
        Plane(Vec3::Z)
    }

    pub fn normal(&self) -> Vec3 {
        self.0
    }
}

// a sketch creates its underlying material, of which bezier curves are drawn on
#[derive(Debug, Clone)]
pub struct SketchData {
    pub plane: Plane,
    pub entity: Entity,
}

// decoupling reasons
pub type Sketch = command_system::Sketch<SketchData>;

#[derive(Debug)]
pub struct CreateSketch {
    pub plane: Plane,
}

impl Feature<World> for CreateSketch {
    fn kind(&self) -> FeatureKind {
        FeatureKind::Sketch
    }

    fn apply(
        &self,
        world: &mut World,
        _ctx: &FeatureContext<World>,
    ) -> Result<FeatureOutput<World>, FeatureError> {
        let mesh =
            world.resource_mut::<Assets<Mesh>>().add(Plane3d::new(
                self.plane.normal(),
                Vec2::splat(SKETCH_HALF_EXTENT),
            ));

        let sketch_curves = SketchCurves::new(world, SKETCH_BACKGROUND);
        let material = sketch_curves.material.clone();

        let entity = world
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::default(),
                sketch_curves,
            ))
            .id();

        Ok(FeatureOutput::with_cleanup(
            Sketch {
                plane: SketchData {
                    plane: self.plane,
                    entity,
                },
                elements: Vec::new(),
            },
            move |world: &mut World| {
                if world.get_entity(entity).is_ok() {
                    world.entity_mut(entity).despawn();
                }
            },
        ))
    }
}
