#[derive(Debug, Clone, Copy)]
pub struct Sketch<P> {
    // we dont define Plane here
    // Plane needs a Vec3 def, and we want to keep it completely agnostic
    pub plane: P,
}
