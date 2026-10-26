use std::collections::HashMap;

use bevy::prelude::*;
use bezier::BezierCurve;
use command_system::{
    Feature, FeatureId, FeatureOutput, FeatureType,
};

use crate::Sketch;

/// The curves a [`CreateProfile`] appended, kept for later features.
#[allow(dead_code)]
#[derive(Component, Clone)]
pub struct ProfileComponent {
    pub curves: Vec<BezierCurve>,
}

/// Appends an arbitrary closed or open run of curves to a sketch, for
/// profiles the parametric shape features do not cover.
pub struct CreateProfile {
    pub sketch: FeatureId,
    pub curves: Vec<BezierCurve>,
}

impl Feature<World, Entity> for CreateProfile {
    fn kind(&self) -> FeatureType {
        FeatureType::Sketch
    }

    fn apply(
        &self,
        world: &mut World,
        outputs: &HashMap<FeatureId, FeatureOutput<World, Entity>>,
    ) -> FeatureOutput<World, Entity> {
        let sketch_entity =
            outputs.get(&self.sketch).and_then(|o| o.value);

        if let Some(sketch_entity) = sketch_entity
            && let Some(mut sketch) =
                world.get_mut::<Sketch>(sketch_entity)
        {
            sketch.curves.extend(self.curves.iter().copied());
        }

        FeatureOutput {
            value: Some(
                world
                    .spawn(ProfileComponent {
                        curves: self.curves.clone(),
                    })
                    .id(),
            ),
            cleanup: Some(Box::new(|entity, world: &mut World| {
                if let Some(&entity) = entity {
                    world.despawn(entity);
                }
            })),
        }
    }
}
