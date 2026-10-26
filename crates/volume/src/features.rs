use std::collections::HashMap;

use bevy::prelude::*;
use command_system::{
    Feature, FeatureId, FeatureOutput, FeatureType,
};
use sketch::Sketch;

use crate::csg::{BoolOp, Volume};
use crate::solid::Extent;

/// Sweeps one face of a sketch into a body.
pub struct Extrude {
    pub sketch: FeatureId,
    pub face: usize,
    pub extent: Extent,
}

impl Feature<World, Entity> for Extrude {
    fn kind(&self) -> FeatureType {
        FeatureType::Volume
    }

    fn apply(
        &self,
        world: &mut World,
        outputs: &HashMap<FeatureId, FeatureOutput<World, Entity>>,
    ) -> FeatureOutput<World, Entity> {
        let volume = outputs
            .get(&self.sketch)
            .and_then(|output| output.value)
            .and_then(|entity| world.get::<Sketch>(entity))
            .and_then(|sketch| {
                Volume::from_face(sketch, self.face, self.extent)
            });

        FeatureOutput {
            value: volume
                .map(|volume| world.spawn(volume.into_bundle()).id()),
            cleanup: Some(Box::new(|entity, world: &mut World| {
                if let Some(&entity) = entity {
                    world.despawn(entity);
                }
            })),
        }
    }
}

/// Folds the tool body into the target, in place.
pub struct Boolean {
    pub target: FeatureId,
    pub tool: FeatureId,
    pub op: BoolOp,
}

impl Feature<World, Entity> for Boolean {
    fn kind(&self) -> FeatureType {
        FeatureType::Volume
    }

    fn apply(
        &self,
        world: &mut World,
        outputs: &HashMap<FeatureId, FeatureOutput<World, Entity>>,
    ) -> FeatureOutput<World, Entity> {
        let target =
            outputs.get(&self.target).and_then(|output| output.value);
        let tool =
            outputs.get(&self.tool).and_then(|output| output.value);

        // Taking the tool's Volume spends it: its entity lives on under
        // its own feature's cleanup, but stops being a body of its own.
        if let Some(target) = target
            && let Some(tool) = tool
            && let Some(cut) = world.entity_mut(tool).take::<Volume>()
            && let Some(mut volume) = world.get_mut::<Volume>(target)
        {
            volume.combine(cut, self.op);
        }

        FeatureOutput {
            value: target,
            // The target belongs to the feature that spawned it; a
            // cleanup here would despawn it twice.
            cleanup: None,
        }
    }
}
