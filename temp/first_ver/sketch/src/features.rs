use bevy::prelude::*;
use command_system::{
    Feature, FeatureContext, FeatureError, FeatureKind, FeatureOutput,
};

use crate::{Plane, SketchCurves, SketchData};

pub mod circle;

const SKETCH_DEFAULT_SIZE: f32 = 1.0;

// decoupling reasons
pub type SketchOutput =
    command_system::Sketch<SketchData, bezier::BezierCurve>;

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
                Vec2::splat(SKETCH_DEFAULT_SIZE),
            ));

        // no material yet
        // TODO: consider if an empty sketch even creates a material
        let entity = world
            .spawn((
                Mesh3d(mesh),
                Transform::default(),
                SketchCurves::default(),
            ))
            .id();

        Ok(FeatureOutput::with_cleanup(
            SketchOutput {
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
