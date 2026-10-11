use std::collections::HashMap;

use bevy::prelude::*;
use command_system::{
    Feature, FeatureId, FeatureOutput, FeatureType,
};

use crate::Sketch;

const CURVE_WIDTH: f32 = 0.01;

// Store this for later (extrusion, lofting, etc.)
#[allow(dead_code)]
#[derive(Component, Clone, Copy)]
pub struct CurveComponent {
    pub points: [Vec2; 4],
}

// Appends a cubic Bezier to a sketch
pub struct CreateCurve {
    pub sketch: FeatureId,
    pub points: [Vec2; 4],
}

impl Feature<World, Entity> for CreateCurve {
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
            let [p0, p1, p2, p3] = self.points;
            let curves = bezier::BezierCurve::cubic(p0, p1, p2, p3)
                .map(|c| {
                    c.with_color(bezier::gen_color())
                        .with_width(CURVE_WIDTH)
                });

            if let Some(mut sketch) =
                world.get_mut::<Sketch>(sketch_entity)
            {
                sketch.curves.extend(curves);
            }
        }

        // TODO: do we still spawn the curve component if the sketch doesnt exist?
        FeatureOutput {
            value: Some(
                world
                    .spawn(CurveComponent {
                        points: self.points,
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
