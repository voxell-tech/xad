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
    // constrained to the plane's face
    pub position: Vec2,
    pub curves: Vec<BezierCurve>,
    pub plane: Plane,
}

impl Sketch {
    // normalises pos to the plane's face, in world space
    pub fn get_position(&self) -> Vec3 {
        let normal = self.plane.normal();
        let helper =
            if normal == Vec3::Y { Vec3::Z } else { Vec3::Y };
        let tangent = helper.cross(normal).normalize();
        let bitangent = normal.cross(tangent).normalize();
        tangent * self.position.x + bitangent * self.position.y
    }

    pub fn into_bundle(self) -> impl Bundle {
        let transform =
            Transform::from_translation(self.get_position());
        (self, transform)
    }
}

pub struct CreateSketch {
    pub position: Vec2,
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
            position: self.position,
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
