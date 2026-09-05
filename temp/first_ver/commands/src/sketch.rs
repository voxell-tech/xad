// C defaults to () since most sketches don't need to track their elements here yet
#[derive(Debug, Clone)]
pub struct Sketch<P, C = ()> {
    // we dont define Plane here
    // Plane needs a Vec3 def, and we want to keep it completely agnostic
    pub plane: P,
    // we also dont define curves here
    pub elements: Vec<C>,
}
