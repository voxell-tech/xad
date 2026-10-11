mod color;
mod curve;
mod face;
pub mod shapes;

pub use color::gen_color;
pub use curve::{
    Aabb, BezierCurve, bounding_box, split_intersections,
};
pub use face::{Face, FaceId, Faces, Polarity, detect_faces};
