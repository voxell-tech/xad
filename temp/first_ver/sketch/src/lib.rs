mod plane;
mod sketch_curves;

pub use plane::Plane;
pub use sketch_curves::SketchCurves;

pub mod features;

use bevy::prelude::*;

// a positioned bundle of curves, ready to be turned into a renderable material
pub struct Sketch {
    pub curves: Vec<bezier::BezierCurve>,
    pub position: Vec3,
    pub background_color: LinearRgba,
}

// a sketch's plane, plus the entity its mesh and curves live on
#[derive(Debug, Clone)]
pub struct SketchData {
    pub plane: Plane,
    pub entity: Entity,
}
