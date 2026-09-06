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
#[derive(Component, Clone)]
pub struct PolygonComponent {
    pub vertices: Vec<Vec2>,
}

pub struct CreatePolygon {
    pub sketch: FeatureId,
    pub vertices: Vec<Vec2>,
}

impl Feature<World, Entity> for CreatePolygon {
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
            let curves = bezier::shapes::polygon::polygon(
                &self.vertices,
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
                    .spawn(PolygonComponent {
                        vertices: self.vertices.clone(),
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
