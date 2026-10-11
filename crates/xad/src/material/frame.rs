use bevy::prelude::Vec3;

// A sketch plane's world-space origin and axes, used to map fragment world
// positions into plane-local curve coordinates.
#[derive(Clone, Copy)]
pub struct PlaneFrame {
    pub origin: Vec3,
    pub tangent: Vec3,
    pub bitangent: Vec3,
}
