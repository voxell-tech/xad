use std::collections::HashMap;

use bevy::prelude::*;
use command_system::{
    Feature, FeatureId, FeatureOutput, FeatureType,
};

use crate::Sketch;

// TODO: figure out a better data-driven approach
//
// we store these for extrusion use later, but
// preferably want these to be indenpendent of bevy
#[allow(dead_code)]
#[derive(Component, Clone, Copy)]
pub struct RectangleComponent {
    pub center: Vec2,
    pub size: Vec2,
}

pub struct CreateRectangle {
    pub sketch: FeatureId,
    pub center: Vec2,
    pub size: Vec2,
}

impl Feature<World, Entity> for CreateRectangle {
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

        if let Some(sketch_entity) = sketch_entity {
            let curves = bezier::shapes::rectangle::rectangle(
                self.center,
                self.size,
                bezier::gen_color(),
                0.01,
            );
            if let Some(mut sketch) =
                world.get_mut::<Sketch>(sketch_entity)
            {
                sketch.curves.extend(curves);
            }
        }

        FeatureOutput {
            value: Some(
                world
                    .spawn(RectangleComponent {
                        center: self.center,
                        size: self.size,
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
