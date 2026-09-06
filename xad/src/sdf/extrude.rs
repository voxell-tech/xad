use std::collections::HashMap;

use bevy::prelude::*;
use command_system::{
    Feature, FeatureId, FeatureOutput, FeatureType,
};
use sketch::features::Circle;

use crate::sdf::primitves::SdfCuboid;
use crate::sdf::transform::SdfTransform;

// super rough prototype for demo purposes
// TODO: apply for all shapes, with basic loft
#[derive(Debug)]
pub struct BlindExtrude {
    pub face: FeatureId,
    pub distance: f32,
}

impl Feature<World, Entity> for BlindExtrude {
    fn kind(&self) -> FeatureType {
        FeatureType::Volume
    }

    fn apply(
        &self,
        world: &mut World,
        outputs: &HashMap<FeatureId, FeatureOutput<World, Entity>>,
    ) -> FeatureOutput<World, Entity> {
        let face_entity =
            outputs.get(&self.face).and_then(|o| o.value);
        let circle = face_entity
            .and_then(|entity| world.get::<Circle>(entity))
            .copied();

        let value = circle.map(|circle| {
            world
                .spawn((
                    SdfCuboid {
                        extents: Vec3::new(
                            circle.radius,
                            self.distance,
                            circle.radius,
                        ),
                    },
                    SdfTransform::default().with_translation(
                        Vec3::new(
                            circle.position.x,
                            0.0,
                            circle.position.y,
                        ),
                    ),
                ))
                .id()
        });

        FeatureOutput {
            value,
            cleanup: Some(Box::new(|entity, world: &mut World| {
                if let Some(&entity) = entity {
                    world.despawn(entity);
                }
            })),
        }
    }
}
