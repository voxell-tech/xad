use bevy::math::{Affine3, Affine3A};
use bevy::prelude::*;
use bevy::render::extract_component::*;
use bevy::render::render_resource::ShaderType;

pub struct SdfTransformPlugin;

impl Plugin for SdfTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostStartup,
            propagate_transform
                .in_set(SdfTransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            propagate_transform
                .in_set(SdfTransformSystems::Propagate),
        );
    }
}

/// Set enum for the systems relating to transform propagation
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SdfTransformSystems {
    /// Propagates changes in transform to children's [`SdfGlobalTransform`]
    Propagate,
}

fn propagate_transform(
    mut q_root_transforms: Query<
        (&mut SdfGlobalTransform, &SdfTransform, Entity),
        Without<ChildOf>,
    >,
    mut q_child_transforms: Query<
        (&mut SdfGlobalTransform, &SdfTransform),
        With<ChildOf>,
    >,
    q_children: Query<&Children>,
) {
    for (mut global_transform, transform, entity) in
        q_root_transforms.iter_mut()
    {
        *global_transform = SdfGlobalTransform::from(*transform);

        if let Ok(children) = q_children.get(entity) {
            // Iteratively propagate the transform.
            // TODO: Optimize this:
            // - Offer stack based recursive?
            // - Parallelize this algo!
            // - Static analysis, skip subtrees that are not mutated.
            let mut global_transforms = vec![*global_transform];
            let mut children =
                children.iter().map(|e| (e, 0)).collect::<Vec<_>>();

            while let Some((child, idx)) = children.pop()
                && let Ok((
                    mut child_global_transform,
                    child_transform,
                )) = q_child_transforms.get_mut(child)
            {
                let global_transform = &global_transforms[idx];
                *child_global_transform =
                    global_transform.mul_transform(*child_transform);

                let new_idx = global_transforms.len();
                global_transforms.push(*child_global_transform);
                if let Ok(nested_children) = q_children.get(child) {
                    children.extend(
                        nested_children.iter().map(|e| (e, new_idx)),
                    );
                }
            }
        }
    }
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
pub struct SdfTransform {
    translation: Vec3,
    rotation: Quat,
    scale: f32,
}

impl Default for SdfTransform {
    fn default() -> Self {
        Self {
            translation: Default::default(),
            rotation: Default::default(),
            scale: 1.0,
        }
    }
}

impl From<SdfTransform> for SdfGlobalTransform {
    fn from(value: SdfTransform) -> Self {
        Self {
            world_from_local: Affine3A::from_rotation_translation(
                value.rotation,
                value.translation,
            ),
            scale: value.scale,
        }
    }
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
pub struct SdfGlobalTransform {
    world_from_local: Affine3A,
    scale: f32,
}

impl SdfGlobalTransform {
    pub fn world_from_local(&self) -> &Affine3A {
        &self.world_from_local
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn mul_transform(&self, transform: SdfTransform) -> Self {
        let other = Self::from(transform);
        Self {
            world_from_local: self.world_from_local
                * other.world_from_local,
            scale: self.scale * other.scale,
        }
    }
}

#[derive(
    Component,
    ExtractComponent,
    ShaderType,
    Reflect,
    Debug,
    Clone,
    Copy,
)]
pub struct SdfTransformUniform {
    pub world_from_local: [Vec4; 3],
    pub scale: f32,
}

impl SdfTransformUniform {
    pub fn from_global_transform(
        global_transform: &SdfGlobalTransform,
    ) -> Self {
        Self {
            world_from_local: Affine3::from(
                &global_transform.world_from_local,
            )
            .to_transpose(),
            scale: global_transform.scale,
        }
    }
}
