use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bezier::BezierCurve;
use encase::ShaderType;
use sketch::{Sketch, SketchCurves};

pub(crate) const SKETCH_BACKGROUND: LinearRgba =
    LinearRgba::new(0.06, 0.06, 0.09, 1.0);

pub struct BezierMaterialPlugin;

impl Plugin for BezierMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<BezierMaterial>::default())
            .add_systems(Update, sync_bezier_materials);
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

// define it here to avoid a cyclic dependency
pub fn construct_bezier_material(
    sketch: &Sketch,
    storage_buffers: &mut Assets<ShaderStorageBuffer>,
) -> BezierMaterial {
    BezierMaterial::from_curves(
        sketch.background_color,
        sketch.curves.clone(),
        storage_buffers,
    )
}

impl Material for BezierMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/bezier.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

// creates a sketch entity's BezierMaterial the first time it gets curves,
// and re-uploads the buffer whenever its SketchCurves changes afterwards
fn sync_bezier_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<BezierMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    query: Query<
        (
            Entity,
            &SketchCurves,
            Option<&MeshMaterial3d<BezierMaterial>>,
        ),
        Changed<SketchCurves>,
    >,
) {
    for (entity, sketch_curves, existing) in &query {
        match existing.and_then(|handle| materials.get(&handle.0)) {
            Some(material) => {
                material.update_curves(
                    SKETCH_BACKGROUND,
                    &sketch_curves.0,
                    &mut storage_buffers,
                );
            }
            None => {
                let material =
                    materials.add(BezierMaterial::from_curves(
                        SKETCH_BACKGROUND,
                        sketch_curves.0.clone(),
                        &mut storage_buffers,
                    ));
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(material));
            }
        }
    }
}
