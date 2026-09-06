use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bezier::BezierCurve;
use encase::ShaderType;
use sketch::Sketch;

const BACKGROUND: LinearRgba = LinearRgba::new(0.06, 0.06, 0.09, 1.0);
const MESH_SIZE: f32 = 1.0;
// matches the fixed-size array in the WGSL shader
const MAX_CURVES: usize = 32;

pub struct BezierMaterialPlugin;

impl Plugin for BezierMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<BezierMaterial>::default())
            .add_systems(Update, sync_sketch_render);
    }
}

// matches the WGSL BezierBuffer struct's memory layout
#[derive(ShaderType, Clone, Copy, Default)]
struct BezierBuffer {
    background_color: Vec4,
    curve_count: u32,
    _padding: [f32; 3],
    curves: [BezierCurve; MAX_CURVES],
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct BezierMaterial {
    #[storage(0, read_only)]
    pub buffer: Handle<ShaderStorageBuffer>,
}

impl BezierMaterial {
    pub fn from_curves(
        curves: &[BezierCurve],
        storage_buffers: &mut Assets<ShaderStorageBuffer>,
    ) -> Self {
        Self {
            buffer: storage_buffers.add(ShaderStorageBuffer::from(
                bezier_buffer(curves),
            )),
        }
    }

    // overwrites the material's GPU buffer in place, instead of allocating a new one
    pub fn update_curves(
        &self,
        curves: &[BezierCurve],
        storage_buffers: &mut Assets<ShaderStorageBuffer>,
    ) {
        if let Some(buffer) = storage_buffers.get_mut(&self.buffer) {
            buffer.set_data(bezier_buffer(curves));
        }
    }
}

fn bezier_buffer(curves: &[BezierCurve]) -> BezierBuffer {
    let mut buffer = BezierBuffer {
        background_color: Vec4::new(
            BACKGROUND.red,
            BACKGROUND.green,
            BACKGROUND.blue,
            BACKGROUND.alpha,
        ),
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

// syncs a sketch's curves and its BezierMaterial
fn sync_sketch_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BezierMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    query: Query<(Entity, &Sketch), Changed<Sketch>>,
) {
    for (entity, sketch) in &query {
        let mesh = meshes.add(Plane3d::new(
            sketch.plane.normal(),
            Vec2::splat(MESH_SIZE),
        ));
        let material = materials.add(BezierMaterial::from_curves(
            &sketch.curves,
            &mut storage_buffers,
        ));

        commands
            .entity(entity)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}
