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

// TODO: allow appending to the gpu buffer
// to edit existing curves, we need to maintain a cpu-side copy,
// and after any edits, resend the entire thing to the gpu
#[derive(Component, Debug, Clone, Default)]
pub struct SketchCurves(pub Vec<BezierCurve>);

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct BezierCurve {
    // Control points
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,

    pub color: Vec4,

    // Half-width
    pub width: f32,

    // 0: Dont show
    // 1: Show control points
    pub debug: u32,

    // 0: Dont show
    // 1: Show a dot at each endpoint (p0 and p2)
    pub draw_endpoints: u32,

    // Radius of the endpoint dots
    pub endpoint_radius: f32,
}

impl BezierCurve {
    pub fn new(p0: Vec2, p1: Vec2, p2: Vec2) -> Self {
        Self {
            p0,
            p1,
            p2,
            color: Vec4::ONE,
            width: 0.012,
            debug: 0,
            draw_endpoints: 0,
            endpoint_radius: 0.02,
        }
    }

    // constantly checking whether its cubic or quadratic GPU-side is expensive.
    // we compute cubic curves into pair quads on CPU, and pass that to GPU instead
    pub fn cubic(
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
        p3: Vec2,
    ) -> [Self; 2] {
        // TODO: consider if we should return impl Iter instead?
        let m = (p0 + 3.0 * p1 + 3.0 * p2 + p3) * 0.125;
        let l1 = (p0 + p1) * 0.5;
        let l2 = (p0 + 2.0 * p1 + p2) * 0.25;
        let ql = (3.0 * (l1 + l2) - p0 - m) * 0.25;
        let r1 = (p1 + 2.0 * p2 + p3) * 0.25;
        let r2 = (p2 + p3) * 0.5;
        let qr = (3.0 * (r1 + r2) - m - p3) * 0.25;

        [Self::new(p0, ql, m), Self::new(m, qr, p3)]
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

    pub fn with_draw_endpoints(
        mut self,
        draw_endpoints: bool,
    ) -> Self {
        self.draw_endpoints = draw_endpoints as u32;
        self
    }

    pub fn with_endpoint_radius(
        mut self,
        endpoint_radius: f32,
    ) -> Self {
        self.endpoint_radius = endpoint_radius;
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

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
