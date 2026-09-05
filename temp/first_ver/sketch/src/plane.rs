use bevy::prelude::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Plane(Vec3); // TODO: complete
// with top, bottom, right (+mirrored direction) vecs

impl Plane {
    // the default planes in any timeline
    pub fn top() -> Self {
        Plane(Vec3::Y)
    }

    pub fn front() -> Self {
        Plane(Vec3::X)
    }

    pub fn right() -> Self {
        Plane(Vec3::Z)
    }

    pub fn normal(&self) -> Vec3 {
        self.0
    }
}
