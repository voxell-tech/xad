pub mod features;

use std::collections::HashMap;

use bevy::prelude::*;
use bezier::BezierCurve;
use command_system::{
    Feature, FeatureId, FeatureOutput, FeatureType,
};

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    normal: Vec3,
}

impl Plane {
    pub fn top() -> Self {
        Self { normal: Vec3::Y }
    }

    pub fn front() -> Self {
        Self { normal: Vec3::X }
    }

    pub fn right() -> Self {
        Self { normal: Vec3::Z }
    }

    pub fn normal(&self) -> Vec3 {
        self.normal
    }
}

#[derive(Component)]
pub struct Sketch {
    pub curves: Vec<BezierCurve>,
    pub plane: Plane,
}

impl Sketch {
    // Orthonormal (tangent, bitangent) basis spanning the plane's face
    // https://blog.claytonsanford.com/2021/07/16/orthogonality.html
    fn basis(&self) -> (Vec3, Vec3) {
        let normal = self.plane.normal();
        let helper =
            if normal == Vec3::Y { Vec3::Z } else { Vec3::Y };
        let tangent = helper.cross(normal).normalize();
        let bitangent = normal.cross(tangent).normalize();
        (tangent, bitangent)
    }

    // Maps a plane-local coordinate to a world offset, from the plane origin
    pub fn plane_offset(&self, local: Vec2) -> Vec3 {
        let (tangent, bitangent) = self.basis();
        tangent * local.x + bitangent * local.y
    }

    // World-space (tangent, bitangent) spanning the plane's face.
    pub fn plane_basis(&self) -> (Vec3, Vec3) {
        self.basis()
    }

    // Projects a world point back into plane-local coordinates.
    pub fn world_to_local(&self, world: Vec3) -> Vec2 {
        let (tangent, bitangent) = self.basis();
        Vec2::new(world.dot(tangent), world.dot(bitangent))
    }

    pub fn into_bundle(self) -> impl Bundle {
        // Transform and Visibility let the render layers spawn as children.
        (self, Transform::default(), Visibility::default())
    }
}

pub struct CreateSketch {
    pub plane: Plane,
}

impl Feature<World, Entity> for CreateSketch {
    fn kind(&self) -> FeatureType {
        FeatureType::Sketch
    }

    fn apply(
        &self,
        world: &mut World,
        _outputs: &HashMap<FeatureId, FeatureOutput<World, Entity>>,
    ) -> FeatureOutput<World, Entity> {
        let sketch = Sketch {
            plane: self.plane,
            curves: vec![],
        };

        FeatureOutput {
            value: Some(world.spawn(sketch.into_bundle()).id()),
            cleanup: Some(Box::new(|entity, world: &mut World| {
                if let Some(&entity) = entity {
                    world.despawn(entity);
                }
            })),
        }
    }
}
