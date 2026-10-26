use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use bezier::{BezierCurve, Polarity};
use encase::ShaderType;

use super::frame::PlaneFrame;

// Fixed-size arrays mirrored in sketch_faces.wgsl.
const MAX_EDGES: usize = 128;
const MAX_FACES: usize = 32;

// A curve with the face touching each side, looking along p0 -> p2.
// 0 = the unbounded outside, otherwise face id + 1.
#[derive(ShaderType, Clone, Copy, Default)]
struct Edge {
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    left: u32,
    right: u32,
}

#[derive(ShaderType, Clone)]
struct FaceBuffer {
    origin: Vec4,
    tangent: Vec4,
    bitangent: Vec4,
    edge_count: u32,
    // Non-zero = tint by nearest edge instead of face color.
    debug_tint: u32,
    colors: [Vec4; MAX_FACES],
    edges: [Edge; MAX_EDGES],
}

impl Default for FaceBuffer {
    fn default() -> Self {
        Self {
            origin: Vec4::ZERO,
            tangent: Vec4::X,
            bitangent: Vec4::Y,
            edge_count: 0,
            debug_tint: 0,
            colors: [Vec4::ZERO; MAX_FACES],
            edges: [Edge::default(); MAX_EDGES],
        }
    }
}

/// Fills every face of a sketch in one pass. The fragment shader finds the
/// curve nearest to the pixel and fills with the color of the face on the
/// pixel's side of that curve.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct FaceMaterial {
    #[storage(0, read_only)]
    pub buffer: Handle<ShaderBuffer>,
}

impl FaceMaterial {
    /// `curves` and `polarity` come from `detect_faces` on the
    /// intersection-split curves; `colors` is indexed by face id.
    pub fn new(
        curves: &[BezierCurve],
        polarity: &[Polarity],
        colors: &[LinearRgba],
        frame: PlaneFrame,
        debug_tint: bool,
        storage_buffers: &mut Assets<ShaderBuffer>,
    ) -> Self {
        let mut buffer = FaceBuffer {
            origin: frame.origin.extend(0.0),
            tangent: frame.tangent.extend(0.0),
            bitangent: frame.bitangent.extend(0.0),
            debug_tint: debug_tint as u32,
            ..Default::default()
        };

        for (i, color) in colors.iter().take(MAX_FACES).enumerate() {
            buffer.colors[i] = Vec4::new(
                color.red,
                color.green,
                color.blue,
                color.alpha,
            );
        }

        // Faces past the color table render as the outside.
        let id = |face: Option<usize>| match face {
            Some(f) if f < MAX_FACES => f as u32 + 1,
            _ => 0,
        };
        // Every curve is uploaded, even None/None ones: a dangling curve
        // outside all faces must win nearest-edge queries there, or the
        // faces behind it reach past their own boundaries.
        for (curve, sides) in
            curves.iter().zip(polarity).take(MAX_EDGES)
        {
            buffer.edges[buffer.edge_count as usize] = Edge {
                p0: curve.p0,
                p1: curve.p1,
                p2: curve.p2,
                left: id(sides.left),
                right: id(sides.right),
            };
            buffer.edge_count += 1;
        }

        Self {
            buffer: storage_buffers.add(ShaderBuffer::from(buffer)),
        }
    }
}

impl Material for FaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sketch_faces.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

pub struct FaceMaterialPlugin;

impl Plugin for FaceMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FaceMaterial>::default())
            .init_resource::<super::debug::ShowPolarityLabels>()
            .init_resource::<super::debug::ShowEdgeTint>()
            .init_resource::<super::debug::ShowBvh>()
            .add_systems(
                Update,
                (
                    super::debug::toggle_labels,
                    super::debug::toggle_edge_tint,
                    super::debug::toggle_bvh,
                    super::debug::position_labels,
                    super::debug::draw_bvh,
                )
                    .chain(),
            );
    }
}
