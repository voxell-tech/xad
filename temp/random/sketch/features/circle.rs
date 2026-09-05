use std::f32::consts::TAU;

use bevy::prelude::*;
use command_system::{
    Feature, FeatureContext, FeatureError, FeatureId, FeatureKind,
    FeatureOutput, SketchId,
};

use crate::bezier::{BezierCurve, SketchCurves};
use crate::sketch::color::gen_color;
use crate::sketch::features::arc::arc;
use crate::sketch::{SKETCH_BACKGROUND, Sketch};

const STROKE_WIDTH: f32 = 0.01;

pub fn circle(
    center: Vec2,
    radius: f32,
    color: LinearRgba,
    stroke_width: f32,
) -> Vec<BezierCurve> {
    arc(center, radius, 0.0, TAU, color, stroke_width)
}

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
            .get::<Sketch>(self.sketch)
            .ok_or(FeatureError::MissingDependency(self.sketch))?;
        let entity = sketch.plane.entity;

        // this will most likely go away once we have a self-resizing bezier material
        let uv_pos = self.pos + Vec2::splat(0.5);
        let curves =
            circle(uv_pos, self.radius, gen_color(), STROKE_WIDTH);

        // TODO: optimise
        // this is expensive frfr
        SketchCurves::add_curves(world, entity, SKETCH_BACKGROUND, curves)
            .ok_or(FeatureError::MissingDependency(self.sketch))?;

        Ok(FeatureOutput::new(CircleProfile {
            pos: self.pos,
            radius: self.radius,
        }))
    }
}
