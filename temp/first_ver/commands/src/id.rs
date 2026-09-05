#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureId(u64);

impl FeatureId {
    // TODO: use an id generator
    pub(crate) fn new(id: u64) -> Self {
        Self(id)
    }
}

pub type SketchId = FeatureId;
