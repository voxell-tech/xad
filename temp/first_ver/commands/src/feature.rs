use std::fmt::Debug;

use crate::{FeatureContext, FeatureId, FeatureOutput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureKind {
    Sketch, // for 2d features
    Volume, // for 3d features
}

#[derive(Debug)]
pub enum FeatureError {
    MissingDependency(FeatureId),
}

// Frontend agnostic feature
// apply is called during timeline regen, to construct the volume
pub trait Feature<W>: Debug + Send + Sync {
    fn kind(&self) -> FeatureKind;

    // other features this one depends on
    // important to know whether a feature is possible or not
    // also useful for multiplayer too ig
    fn depends_on(&self) -> Vec<FeatureId> {
        Vec::new()
    }

    fn apply(
        &self,
        world: &mut W,
        ctx: &FeatureContext<W>,
    ) -> Result<FeatureOutput<W>, FeatureError>;
}
