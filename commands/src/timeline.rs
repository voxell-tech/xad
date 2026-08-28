use std::collections::HashMap;

use crate::{
    Feature, FeatureContext, FeatureError, FeatureId, FeatureOutput,
    Sketch,
};

// ordered list of features, executed in order.
//
// before regenerating the volumes, we call cleanup
// on the timeline and all its features to despawn
// the old entities.
//
// in the future implementation, we will only regenerate
// necessary entities
pub struct FeatureTimeline<W> {
    entries: Vec<(FeatureId, Box<dyn Feature<W>>)>,
    outputs: HashMap<FeatureId, FeatureOutput<W>>,
    next_id: u64,
}

impl<W> Default for FeatureTimeline<W> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            outputs: HashMap::new(),
            next_id: 0,
        }
    }
}

impl<W> FeatureTimeline<W> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(
        &mut self,
        feature: impl Feature<W> + 'static,
    ) -> FeatureId {
        let id = FeatureId::new(self.next_id);
        self.next_id += 1;
        self.entries.push((id, Box::new(feature)));
        id
    }

    pub fn outputs_of<T: 'static>(
        &self,
    ) -> impl Iterator<Item = (FeatureId, &T)> {
        self.outputs.iter().filter_map(|(&id, output)| {
            output.downcast::<T>().map(|v| (id, v))
        })
    }

    pub fn regen(
        &mut self,
        world: &mut W,
    ) -> Result<(), FeatureError> {
        for output in self.outputs.values() {
            output.cleanup(world);
        }
        self.outputs.clear();

        for (id, feature) in &self.entries {
            let ctx = FeatureContext {
                outputs: &self.outputs,
            };
            let output = feature.apply(world, &ctx)?;
            self.outputs.insert(*id, output);
        }

        Ok(())
    }

    pub fn sketch_registry<P: 'static>(
        &self,
    ) -> Vec<(FeatureId, &Sketch<P>)> {
        self.outputs_of::<Sketch<P>>().collect()
    }
}
