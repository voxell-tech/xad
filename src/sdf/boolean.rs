use crate::sdf::transform::SdfTransform;
use bevy::prelude::*;
use bevy::render::extract_component::{
    ExtractComponent, ExtractComponentPlugin,
};

pub struct SdfBooleanPlugin;

impl Plugin for SdfBooleanPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BooleanOp>()
            .register_type::<SdfBooleanOp>()
            .register_type::<SdfOrder>()
            .add_plugins(
                ExtractComponentPlugin::<SdfOperandOf>::default(),
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

#[derive(Component)]
#[relationship(relationship_target = SdfOperands)]
pub struct SdfOperandOf(pub Entity);

impl ExtractComponent for SdfOperandOf {
    type QueryData = (
        &'static SdfOperandOf,
        &'static SdfBooleanOp,
        &'static SdfOrder,
    );
    type QueryFilter = ();
    type Out = SdfExtractedOperand;

    fn extract_component(
        (operand_of, bool_op, order): <Self::QueryData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    ) -> Option<Self::Out> {
        Some(SdfExtractedOperand {
            group_entity: operand_of.0,
            op: bool_op.0,
            order: order.0,
        })
    }
}

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

/// Only exists in the render world.
/// Produced by SdfOperandOf's ExtractComponent impl.
#[derive(Component, Clone)]
pub struct SdfExtractedOperand {
    pub group_entity: Entity,
    pub op: BooleanOp,
    pub order: usize,
}

/// Object-safe trait for inserting a type-erased primitive bundle into the world.
pub trait PrimitiveBundle: Send + Sync + 'static {
    fn insert_into(
        self: Box<Self>,
        world: &mut World,
        entity: Entity,
    );
}

impl<T: Bundle> PrimitiveBundle for T {
    fn insert_into(
        self: Box<Self>,
        world: &mut World,
        entity: Entity,
    ) {
        world.entity_mut(entity).insert(*self);
    }
}

pub enum SdfOperandDesc {
    Entity(Entity),
    Inline {
        bundle: Box<dyn PrimitiveBundle>,
        transform: SdfTransform,
    },
    NestedGroup {
        builder: SdfGroup,
        transform: SdfTransform,
    },
}

pub struct SdfGroup {
    operands: Vec<(BooleanOp, SdfOperandDesc)>,
}

impl SdfGroup {
    pub fn new() -> Self {
        Self {
            operands: Vec::new(),
        }
    }

    pub fn add(mut self, node: (BooleanOp, SdfOperandDesc)) -> Self {
        self.operands.push(node);
        self
    }
}

/// Anything that can become a group operand by specifying a boolean operation.
pub trait IntoSdfNode: Sized {
    fn into_desc(self) -> SdfOperandDesc;

    fn into_node(self, op: BooleanOp) -> (BooleanOp, SdfOperandDesc) {
        (op, self.into_desc())
    }
    fn union(self) -> (BooleanOp, SdfOperandDesc) {
        self.into_node(BooleanOp::Union)
    }
    fn difference(self) -> (BooleanOp, SdfOperandDesc) {
        self.into_node(BooleanOp::Difference)
    }
    fn intersect(self) -> (BooleanOp, SdfOperandDesc) {
        self.into_node(BooleanOp::Intersection)
    }
    fn exclude(self) -> (BooleanOp, SdfOperandDesc) {
        self.into_node(BooleanOp::Exclusion)
    }
}

impl<T: PrimitiveBundle> IntoSdfNode for (T, SdfTransform) {
    fn into_desc(self) -> SdfOperandDesc {
        SdfOperandDesc::Inline {
            bundle: Box::new(self.0),
            transform: self.1,
        }
    }
}

impl IntoSdfNode for Entity {
    fn into_desc(self) -> SdfOperandDesc {
        SdfOperandDesc::Entity(self)
    }
}

impl IntoSdfNode for (SdfGroup, SdfTransform) {
    fn into_desc(self) -> SdfOperandDesc {
        SdfOperandDesc::NestedGroup {
            builder: self.0,
            transform: self.1,
        }
    }
}

impl EntityCommand for SdfGroup {
    fn apply(self, mut entity: EntityWorldMut) {
        entity.insert(SdfOperands::default());
        let group_entity = entity.id();

        entity.world_scope(|world| {
            for (i, (op, desc)) in
                self.operands.into_iter().enumerate()
            {
                let operand_entity = match desc {
                    SdfOperandDesc::Entity(e) => e,

                    SdfOperandDesc::Inline { bundle, transform } => {
                        let e = world.spawn(transform).id();
                        bundle.insert_into(world, e);
                        e
                    }

                    SdfOperandDesc::NestedGroup {
                        builder,
                        transform,
                    } => {
                        let e = world.spawn(transform).id();
                        builder.apply(world.entity_mut(e));
                        e
                    }
                };

                world.entity_mut(operand_entity).insert((
                    SdfOperandOf(group_entity),
                    SdfBooleanOp(op),
                    SdfOrder(i),
                ));
            }
        });
    }
}

pub trait SdfEntityCommandsExt {
    fn add_to_group(
        &mut self,
        group: Entity,
        op: BooleanOp,
        order: usize,
    ) -> &mut Self;

    fn remove_from_group(&mut self) -> &mut Self;

    fn set_boolean_op(&mut self, op: BooleanOp) -> &mut Self;
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
}
