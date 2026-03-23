use bevy::prelude::*;
use bevy::render::extract_component::{
    ExtractComponent, ExtractComponentPlugin,
};

pub struct SdfBooleanPlugin;

impl Plugin for SdfBooleanPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SdfGroup>()
            .add_observer(assign_sdf_group_children)
            .add_plugins(
                ExtractComponentPlugin::<SdfOperand>::default(),
            );
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

#[derive(Component, Clone, ExtractComponent)]
pub struct SdfOperand {
    pub group_id: u32,
    pub op: BooleanOp,
    pub order: usize,
}

impl SdfGroup {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn union(mut self, entity: Entity) -> Self {
        self.operands.push((entity, BooleanOp::Union));
        self
    }
    pub fn difference(mut self, entity: Entity) -> Self {
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

/// Assigns child relationships and group data for the operands in a boolean group.
fn assign_sdf_group_children(
    trigger: On<Add, SdfGroup>,
    mut commands: Commands,
    q_groups: Query<&SdfGroup>,
) {
    let group_entity = trigger.entity;
    let Ok(group) = q_groups.get(group_entity) else {
        return;
    };

    let group_id = group_entity.index_u32() + 1;

    let mut child_entities = Vec::with_capacity(group.operands.len());

    for (i, (operand_entity, requested_op)) in
        group.operands.iter().enumerate()
    {
        // Make the first operand always a union
        let op = if i == 0 {
            BooleanOp::Union
        } else {
            *requested_op
        };

        commands.entity(*operand_entity).insert(SdfOperand {
            group_id,
            op,
            order: i,
        });

        child_entities.push(*operand_entity);
    }

    commands.entity(group_entity).add_children(&child_entities);
}
