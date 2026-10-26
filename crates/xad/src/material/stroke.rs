use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use bezier::BezierCurve;
use encase::ShaderType;

use super::frame::PlaneFrame;

// Translucent pane: must not wall off solids behind the sketch.
const BACKGROUND: LinearRgba = LinearRgba::new(0.06, 0.06, 0.09, 0.35);
// matches the fixed-size array in the WGSL shader
const MAX_CURVES: usize = 128;

pub struct BezierMaterialPlugin;

impl Plugin for BezierMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<BezierMaterial>::default());
    }
}

// matches the WGSL BezierBuffer struct's memory layout
#[derive(ShaderType, Clone, Copy)]
struct BezierBuffer {
    background_color: Vec4,
    origin: Vec4,
    tangent: Vec4,
    bitangent: Vec4,
    curve_count: u32,
    _padding: [f32; 3],
    curves: [BezierCurve; MAX_CURVES],
}

impl Default for BezierBuffer {
    fn default() -> Self {
        Self {
            background_color: Vec4::ZERO,
            origin: Vec4::ZERO,
            tangent: Vec4::X,
            bitangent: Vec4::Y,
            curve_count: 0,
            _padding: [0.0; 3],
            curves: [BezierCurve::default(); MAX_CURVES],
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct BezierMaterial {
    #[storage(0, read_only)]
    pub buffer: Handle<ShaderBuffer>,
}

impl BezierMaterial {
    pub fn from_curves(
        curves: &[BezierCurve],
        frame: PlaneFrame,
        storage_buffers: &mut Assets<ShaderBuffer>,
    ) -> Self {
        Self {
            buffer: storage_buffers.add(ShaderBuffer::from(
                bezier_buffer(curves, frame),
            )),
        }
    }

    // overwrites the material's GPU buffer in place, instead of allocating a new one
    pub fn update_curves(
        &self,
        curves: &[BezierCurve],
        frame: PlaneFrame,
        storage_buffers: &mut Assets<ShaderBuffer>,
    ) {
        if let Some(mut buffer) =
            storage_buffers.get_mut(&self.buffer)
        {
            buffer.set_data(bezier_buffer(curves, frame));
        }
    }
}

fn bezier_buffer(
    curves: &[BezierCurve],
    frame: PlaneFrame,
) -> BezierBuffer {
    let mut buffer = BezierBuffer {
        background_color: Vec4::new(
            BACKGROUND.red,
            BACKGROUND.green,
            BACKGROUND.blue,
            BACKGROUND.alpha,
        ),
        origin: frame.origin.extend(0.0),
        tangent: frame.tangent.extend(0.0),
        bitangent: frame.bitangent.extend(0.0),
        curve_count: curves.len().min(MAX_CURVES) as u32,
        ..Default::default()
    };
    for (i, curve) in curves.iter().enumerate().take(MAX_CURVES) {
        buffer.curves[i] = *curve;
    }
    buffer
}

impl Material for BezierMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/bezier.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
