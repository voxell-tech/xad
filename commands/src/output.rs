use std::any::Any;
use std::collections::HashMap;

use crate::FeatureId;

// Frontend-agnostic Feature definition
pub struct FeatureOutput<W> {
    value: Box<dyn Any + Send + Sync>,
    // TODO: optimise
    // currently, we run cleanup on all entities, and regenerate the entire scene
    // there are definitely cheaper more efficient ways to do this
    // eg: culling via dependencies, etc.
    cleanup: Option<Box<dyn Fn(&mut W) + Send + Sync>>,
}

impl<W> FeatureOutput<W> {
    pub fn new<T: Send + Sync + 'static>(value: T) -> Self {
        Self {
            value: Box::new(value),
            cleanup: None,
        }
    }

    pub fn with_cleanup<T: Send + Sync + 'static>(
        value: T,
        cleanup: impl Fn(&mut W) + Send + Sync + 'static,
    ) -> Self {
        Self {
            value: Box::new(value),
            cleanup: Some(Box::new(cleanup)),
        }
    }

    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }

    pub(crate) fn cleanup(&self, world: &mut W) {
        if let Some(cleanup) = &self.cleanup {
            cleanup(world);
        }
    }
}

pub struct FeatureContext<'a, W> {
    // every FeatureId is tied to a specific type
    // eg:
    //  id_01 -> Sketch,
    //  id_02 -> Circle
    //  id_03 -> Sketch
    //
    // we definitely have the tech to make this better
    // but for now, just use this super messy solution
    pub(crate) outputs: &'a HashMap<FeatureId, FeatureOutput<W>>,
}

impl<'a, W> FeatureContext<'a, W> {
    pub fn get<T: 'static>(&self, id: FeatureId) -> Option<&T> {
        self.outputs.get(&id)?.downcast()
    }
}
