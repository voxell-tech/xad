use bevy::prelude::*;
use command_system::{
    Feature, FeatureContext, FeatureError, FeatureId, FeatureKind,
    FeatureOutput,
};
use sketch::features::circle::CircleProfile;

use crate::sdf::primitves::SdfCuboid;
use crate::sdf::transform::SdfTransform;

// super rough prototype for demo purposes
// TODO: apply for all shapes, with basic loft
#[derive(Debug)]
pub struct BlindExtrude {
    pub face: FeatureId,
    pub distance: f32,
}

impl Feature<World> for BlindExtrude {
    fn kind(&self) -> FeatureKind {
        FeatureKind::Volume
    }

    fn depends_on(&self) -> Vec<FeatureId> {
        vec![self.face]
    }

    fn apply(
        &self,
        world: &mut World,
        ctx: &FeatureContext<World>,
    ) -> Result<FeatureOutput<World>, FeatureError> {
        let profile = ctx
            .get::<CircleProfile>(self.face)
            .ok_or(FeatureError::MissingDependency(self.face))?;

        let entity =
            world
                .spawn((
                    SdfCuboid {
                        extents: Vec3::new(
                            profile.radius,
                            self.distance,
                            profile.radius,
                        ),
                    },
                    SdfTransform::default().with_translation(
                        Vec3::new(profile.pos.x, 0.0, profile.pos.y),
                    ),
                ))
                .id();

        Ok(FeatureOutput::with_cleanup(
            entity,
            move |world: &mut World| {
                if world.get_entity(entity).is_ok() {
                    world.entity_mut(entity).despawn();
                }
            },
        ))
    }
}
