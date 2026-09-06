use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureType {
    Sketch,
    Volume,
}

pub trait Feature<W, E> {
    fn kind(&self) -> FeatureType;

    // outputs: previously computed features, to resolve dependencies from
    fn apply(
        &self,
        world: &mut W,
        outputs: &HashMap<FeatureId, FeatureOutput<W, E>>,
    ) -> FeatureOutput<W, E>;
}

pub struct FeatureOutput<W, E> {
    pub value: Option<E>,
    pub cleanup: Option<Box<dyn Fn(Option<&E>, &mut W)>>,
}

pub struct Timeline<W, E> {
    entries: Vec<(FeatureId, Box<dyn Feature<W, E>>)>,
    outputs: HashMap<FeatureId, FeatureOutput<W, E>>,
    next_id: u64,
}

impl<W, E> Timeline<W, E> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            outputs: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn push(
        &mut self,
        f: impl Feature<W, E> + 'static,
    ) -> FeatureId {
        let id = FeatureId(self.next_id);
        self.next_id += 1;
        self.entries.push((id, Box::new(f)));
        id
    }

    pub fn regen(&mut self, world: &mut W) {
        let Self {
            entries, outputs, ..
        } = self;

        for (id, feature) in entries.iter() {
            let output = feature.apply(world, outputs);
            outputs.insert(*id, output);
        }
    }

    pub fn cleanup(&mut self, world: &mut W) {
        for output in self.outputs.values() {
            if let Some(cleanup) = &output.cleanup {
                cleanup(output.value.as_ref(), world);
            }
        }

        self.outputs.clear();
    }
}

impl<W, E> Default for Timeline<W, E> {
    fn default() -> Self {
        Self::new()
    }
}
