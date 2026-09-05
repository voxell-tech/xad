use bevy::prelude::*;
use bevy::render::extract_component::{
    ExtractComponent, ExtractComponentPlugin,
};
use command_system::{
    Feature, FeatureContext, FeatureError, FeatureId, FeatureKind,
    FeatureOutput,
};

use crate::sdf::transform::SdfTransform;

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

#[derive(Component, Reflect, Debug, Clone)]
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

    for (i, &(operand_entity, op)) in
        group.operands.iter().enumerate()
    {
        commands.entity(operand_entity).insert(SdfOperand {
            group_id,
            op,
            order: i,
        });

        child_entities.push(operand_entity);
    }

    commands.entity(group_entity).add_children(&child_entities);
}

#[derive(Debug)]
pub struct BooleanSubtract {
    pub target: FeatureId,
    pub tool: FeatureId,
}

impl Feature<World> for BooleanSubtract {
    fn kind(&self) -> FeatureKind {
        FeatureKind::Volume
    }

    fn depends_on(&self) -> Vec<FeatureId> {
        vec![self.target, self.tool]
    }

    fn apply(
        &self,
        world: &mut World,
        ctx: &FeatureContext<World>,
    ) -> Result<FeatureOutput<World>, FeatureError> {
        let target = *ctx
            .get::<Entity>(self.target)
            .ok_or(FeatureError::MissingDependency(self.target))?;
        let tool = *ctx
            .get::<Entity>(self.tool)
            .ok_or(FeatureError::MissingDependency(self.tool))?;

        let entity = world
            .spawn((
                SdfGroup::new(target).difference(tool),
                SdfTransform::default(),
            ))
            .id();

        Ok(FeatureOutput::with_cleanup(
            entity,
            move |world: &mut World| {
                if world.get_entity(entity).is_ok() {
                    world.entity_mut(entity).despawn();
                }
            },
        ))
    }
}
