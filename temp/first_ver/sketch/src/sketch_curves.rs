use bevy::prelude::*;
use bezier::BezierCurve;

// used to create rendered curves, etc.
#[derive(Component, Debug, Clone, Default)]
pub struct SketchCurves(pub Vec<BezierCurve>);
