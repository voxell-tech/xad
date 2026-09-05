use bevy::prelude::*;
use bezier::gen_color;
use bezier::shapes::circle::circle;
use command_system::{
    Feature, FeatureContext, FeatureError, FeatureId, FeatureKind,
    FeatureOutput, SketchId,
};

use crate::{SketchCurves, features::SketchOutput};

const STROKE_WIDTH: f32 = 0.01;

// currently rough placeholder
// TODO: complete
#[derive(Debug, Clone, Copy)]
pub struct CircleProfile {
    pub pos: Vec2,
    pub radius: f32,
}

#[derive(Debug)]
pub struct CreateCircle {
    pub sketch: SketchId,
    pub pos: Vec2,
    pub radius: f32,
}

impl Feature<World> for CreateCircle {
    fn kind(&self) -> FeatureKind {
        FeatureKind::Sketch
    }

    fn depends_on(&self) -> Vec<FeatureId> {
        vec![self.sketch]
    }

    fn apply(
        &self,
        world: &mut World,
        ctx: &FeatureContext<World>,
    ) -> Result<FeatureOutput<World>, FeatureError> {
        let sketch = ctx
            .get::<SketchOutput>(self.sketch)
            .ok_or(FeatureError::MissingDependency(self.sketch))?;
        let entity = sketch.plane.entity;

        // this will most likely go away once we have a self-resizing bezier material
        let uv_pos = self.pos + Vec2::splat(0.5);
        let curves =
            circle(uv_pos, self.radius, gen_color(), STROKE_WIDTH);

        let mut sketch_curves = world
            .get_mut::<SketchCurves>(entity)
            .ok_or(FeatureError::MissingDependency(self.sketch))?;
        sketch_curves.0.extend(curves);

        Ok(FeatureOutput::new(CircleProfile {
            pos: self.pos,
            radius: self.radius,
        }))
    }
}
