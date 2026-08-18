use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;

pub struct BezierPlugin;

impl Plugin for BezierPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<BezierMaterial>::default());
    }
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct BezierCurve {
    // Control points
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,

    pub color: Vec4,

    // Half-width
    pub width: f32,

    // 0: quadratic (p0..p2)
    // 1: cubic (p0..p3)
    pub kind: u32,

    // 0: Dont show
    // 1: Show control points
    pub debug: u32,
}

impl BezierCurve {
    pub fn new(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self {
            p0,
            p1,
            p2,
            p3,
            color: Vec4::ONE,
            width: 0.012,
            kind: 1,
            debug: 0,
        }
    }

    pub fn new_quadratic(p0: Vec2, p1: Vec2, p2: Vec2) -> Self {
        Self {
            p0,
            p1,
            p2,
            p3: Vec2::ZERO, // TODO: consider to maybe set p3 = p2? does the math work?
            color: Vec4::ONE,
            width: 0.012,
            kind: 0,
            debug: 0,
        }
    }

    pub fn with_color(mut self, color: LinearRgba) -> Self {
        self.color = Vec4::new(
            color.red,
            color.green,
            color.blue,
            color.alpha,
        );
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug as u32;
        self
    }
}

// Matches `MAX_CURVES` in WGSL shader
pub const MAX_BEZIER_CURVES: usize = 32;

// Fixed-size GPU layout: background + count + curves
#[derive(ShaderType, Clone, Copy, Default)]
struct BezierGpuData {
    background_color: Vec4, // 16 bytes
    curve_count: u32,       // 4 bytes
    // cache localisation
    // 12 bytes explicit padding so `curves` lands at a 16-byte boundary (offset 32).
    _pad0: [f32; 3], // 12 bytes
    curves: [BezierCurve; 32],
}

// Material that draws cubic Bezier curves on a mesh surface
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct BezierMaterial {
    #[storage(0, read_only)]
    pub buffer: Handle<ShaderStorageBuffer>,
}

impl BezierMaterial {
    // Constructs material based on list of curves and a background colour
    pub fn from_curves(
        background: LinearRgba,
        curves: Vec<BezierCurve>,
        storage_buffers: &mut Assets<ShaderStorageBuffer>,
    ) -> Self {
        let mut data = BezierGpuData {
            background_color: Vec4::new(
                background.red,
                background.green,
                background.blue,
                background.alpha,
            ),
            curve_count: curves.len().min(MAX_BEZIER_CURVES) as u32,
            ..Default::default()
        };
        for (i, curve) in
            curves.into_iter().enumerate().take(MAX_BEZIER_CURVES)
        {
            data.curves[i] = curve;
        }
        Self {
            buffer: storage_buffers
                .add(ShaderStorageBuffer::from(data)),
        }
    }

    // Overwrites the material's GPU buffer in place, with new background
    // and curve
    pub fn update_curves(
        &self,
        background: LinearRgba,
        curves: &[BezierCurve],
        storage_buffers: &mut Assets<ShaderStorageBuffer>,
    ) {
        let mut data = BezierGpuData {
            background_color: Vec4::new(
                background.red,
                background.green,
                background.blue,
                background.alpha,
            ),
            curve_count: curves.len().min(MAX_BEZIER_CURVES) as u32,
            ..Default::default()
        };
        for (i, curve) in
            curves.iter().enumerate().take(MAX_BEZIER_CURVES)
        {
            data.curves[i] = *curve;
        }
        if let Some(buffer) = storage_buffers.get_mut(&self.buffer) {
            buffer.set_data(data);
        }
    }
}

impl Material for BezierMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/bezier.wgsl".into()
    }
}
