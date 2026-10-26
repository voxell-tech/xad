use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor,
    SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use bezier::{Aabb, BezierCurve, Polarity, TileGrid};
use encase::ShaderType;

use super::frame::PlaneFrame;

// Fixed-size arrays mirrored in sketch_faces.wgsl.
const MAX_EDGES: usize = 128;
const MAX_FACES: usize = 32;
const MAX_TILES: usize = 256;
const MAX_TILE_INDICES: usize = 2048;
// Tiles along the sketch bounds' longer axis.
// TODO: tune adaptively (edge count / density) instead of a constant.
const GRID_RESOLUTION: u32 = 16;

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
    grid_min: Vec2,
    inv_tile_size: Vec2,
    edge_count: u32,
    // Non-zero = tint by nearest edge instead of face color.
    debug_tint: u32,
    grid_cols: u32,
    grid_rows: u32,
    colors: [Vec4; MAX_FACES],
    edges: [Edge; MAX_EDGES],
    // Per tile, (offset, count) into `tile_indices`, row-major.
    tiles: [UVec2; MAX_TILES],
    tile_indices: [u32; MAX_TILE_INDICES],
}

impl Default for FaceBuffer {
    fn default() -> Self {
        Self {
            origin: Vec4::ZERO,
            tangent: Vec4::X,
            bitangent: Vec4::Y,
            grid_min: Vec2::ZERO,
            inv_tile_size: Vec2::ONE,
            edge_count: 0,
            debug_tint: 0,
            grid_cols: 1,
            grid_rows: 1,
            colors: [Vec4::ZERO; MAX_FACES],
            edges: [Edge::default(); MAX_EDGES],
            tiles: [UVec2::ZERO; MAX_TILES],
            tile_indices: [0; MAX_TILE_INDICES],
        }
    }
}

/// Fills every face of a sketch in one pass. The fragment shader looks up
/// its tile's candidate list, finds the curve nearest to the pixel among
/// them, and fills with the color of the face on the pixel's side of that
/// curve.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct FaceMaterial {
    #[storage(0, read_only)]
    pub buffer: Handle<ShaderBuffer>,
}

impl FaceMaterial {
    /// `curves` and `polarity` come from `detect_faces` on the
    /// intersection-split curves; `colors` is indexed by face id;
    /// `bounds` is the plane-local rect the fill mesh covers.
    pub fn new(
        curves: &[BezierCurve],
        polarity: &[Polarity],
        colors: &[LinearRgba],
        bounds: Aabb,
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

        // Per-tile candidate lists over the same uploaded prefix.
        let uploaded = &curves[..buffer.edge_count as usize];
        let grid = TileGrid::build_within(
            uploaded,
            bounds,
            GRID_RESOLUTION,
            MAX_TILE_INDICES,
        );
        buffer.grid_min = bounds.min;
        buffer.inv_tile_size =
            Vec2::new(grid.cols as f32, grid.rows as f32)
                / bounds.size().max(Vec2::splat(f32::EPSILON));
        buffer.grid_cols = grid.cols;
        buffer.grid_rows = grid.rows;
        for (i, tile) in grid.tiles.iter().enumerate() {
            buffer.tiles[i] = UVec2::new(tile.offset, tile.count);
        }
        buffer.tile_indices[..grid.indices.len()]
            .copy_from_slice(&grid.indices);

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

    // Sorts nearer than the coplanar stroke quad, so faces draw on top
    // of the sketch background and its edges.
    fn depth_bias(&self) -> f32 {
        -1.0
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // The sketch reads from both sides.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

pub struct FaceMaterialPlugin;

impl Plugin for FaceMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FaceMaterial>::default());
    }
}
