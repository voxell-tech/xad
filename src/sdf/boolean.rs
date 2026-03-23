use bevy::prelude::*;

pub struct SdfBooleanPlugin;

impl Plugin for SdfBooleanPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SdfGroup>();
        app.add_observer(assign_sdf_group_children);
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum BooleanOp {
    Union = 0,
    Difference = 1,
    Intersection = 2,
    Exclusion = 3,
}

#[derive(Component, Reflect, Debug, Clone, Default)]
pub struct SdfGroup {
    pub(super) operands: Vec<(Entity, BooleanOp)>,
}

impl SdfGroup {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(mut self, entity: Entity) -> Self {
        self.operands.push((entity, BooleanOp::Union));
        self
    }
    pub fn subtract(mut self, entity: Entity) -> Self {
        self.operands.push((entity, BooleanOp::Difference));
        self
    }
    pub fn intersect(mut self, entity: Entity) -> Self {
        self.operands.push((entity, BooleanOp::Intersection));
        self
    }
    pub fn exclude(mut self, entity: Entity) -> Self {
        self.operands.push((entity, BooleanOp::Exclusion));
        self
    }
}

/// Assigns child relationships for the operands in a boolean group.
fn assign_sdf_group_children(
    trigger: On<Add, SdfGroup>,
    mut commands: Commands,
    q_groups: Query<&SdfGroup>,
) {
    let group_entity = trigger.entity;
    let Ok(group) = q_groups.get(group_entity) else {
        return;
    };

    for (operand, _) in &group.operands {
        commands.entity(group_entity).add_child(*operand);
    }
}
