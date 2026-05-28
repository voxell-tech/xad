use bevy::prelude::*;

pub struct SdfBooleanPlugin;

impl Plugin for SdfBooleanPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BooleanOp>()
            .register_type::<SdfBooleanOp>()
            .register_type::<SdfOrder>()
            .register_type::<SdfGroup>()
            .add_observer(assign_sdf_group_children);
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

/// This entity is a member of an SDF group which the group entity carries [`SdfOperands`]).
#[derive(Component)]
#[relationship(relationship_target = SdfOperands)]
pub struct SdfOperandOf(pub Entity);

/// The group entity owns references to all its operand children.
#[derive(Component, Default)]
#[relationship_target(relationship = SdfOperandOf, linked_spawn)]
pub struct SdfOperands(Vec<Entity>);

impl SdfOperands {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// The boolean operation this operand applies.
#[derive(Component, Reflect, Debug, Clone, Copy)]
pub struct SdfBooleanOp(pub BooleanOp);

/// Explicit draw order within a group.
#[derive(
    Component,
    Reflect,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub struct SdfOrder(pub usize);

/// Render-world copy produced by [`SdfOperandOf`]'s [`ExtractComponent`] impl.
#[derive(Component, Clone)]
pub struct SdfExtractedOperand {
    pub group_entity: Entity,
    pub op: BooleanOp,
    pub order: usize,
}

#[derive(Component, Reflect, Debug, Clone)]
pub struct SdfGroup {
    pub(crate) operands: Vec<(Entity, BooleanOp)>,
}

impl SdfGroup {
    /// Start a new group with `entity` as the first operand (unioned).
    pub fn new(entity: Entity) -> Self {
        Self {
            operands: vec![(entity, BooleanOp::Union)],
        }
    }

    pub fn push_operand(
        mut self,
        entity: Entity,
        op: BooleanOp,
    ) -> Self {
        self.operands.push((entity, op));
        self
    }

    pub fn union(self, entity: Entity) -> Self {
        self.push_operand(entity, BooleanOp::Union)
    }

    pub fn difference(self, entity: Entity) -> Self {
        self.push_operand(entity, BooleanOp::Difference)
    }

    pub fn intersect(self, entity: Entity) -> Self {
        self.push_operand(entity, BooleanOp::Intersection)
    }

    pub fn exclude(self, entity: Entity) -> Self {
        self.push_operand(entity, BooleanOp::Exclusion)
    }
}

/// Fires when [`SdfGroup`] is added to an entity.
/// Inserts [`SdfOperandOf`], [`SdfBooleanOp`], and [`SdfOrder`] on each
/// operand so the `SdfOperandOf`/`SdfOperands` relationship is established.
fn assign_sdf_group_children(
    trigger: On<Add, SdfGroup>,
    mut commands: Commands,
    q_groups: Query<&SdfGroup>,
) {
    let group_entity = trigger.entity;
    let Ok(group) = q_groups.get(group_entity) else {
        return;
    };

    for (i, &(operand_entity, op)) in
        group.operands.iter().enumerate()
    {
        commands.entity(operand_entity).insert((
            SdfOperandOf(group_entity),
            SdfBooleanOp(op),
            SdfOrder(i),
        ));
    }
}

pub trait SdfEntityCommandsExt {
    /// Add this entity to a group as an operand.
    fn add_to_group(
        &mut self,
        group: Entity,
        op: BooleanOp,
        order: usize,
    ) -> &mut Self;

    /// Remove this entity from its current group.
    fn remove_from_group(&mut self) -> &mut Self;

    /// Change the boolean operation without moving the entity.
    fn set_boolean_op(&mut self, op: BooleanOp) -> &mut Self;

    /// Change the evaluation order within the group.
    fn set_order(&mut self, order: usize) -> &mut Self;
}

impl SdfEntityCommandsExt for EntityCommands<'_> {
    fn add_to_group(
        &mut self,
        group: Entity,
        op: BooleanOp,
        order: usize,
    ) -> &mut Self {
        self.insert((
            SdfOperandOf(group),
            SdfBooleanOp(op),
            SdfOrder(order),
        ));
        self
    }

    fn remove_from_group(&mut self) -> &mut Self {
        self.remove::<(SdfOperandOf, SdfBooleanOp, SdfOrder)>();
        self
    }

    fn set_boolean_op(&mut self, op: BooleanOp) -> &mut Self {
        self.insert(SdfBooleanOp(op));
        self
    }

    fn set_order(&mut self, order: usize) -> &mut Self {
        self.insert(SdfOrder(order));
        self
    }
}
