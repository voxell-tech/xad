use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor,
    SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use encase::ShaderType;
use volume::{BoolOp, Bounds, Node, Volume};

// Margin on the proxy box, so a surface never sits exactly on the face
// the ray enters through.
const PROXY_PAD: f32 = 0.02;

const NODE_SOLID: u32 = 0;
const NODE_OP: u32 = 1;

/// Body colour, read by [`sync_volume_render`].
#[derive(Component, Clone, Copy)]
pub struct VolumeColor(pub LinearRgba);

impl Default for VolumeColor {
    fn default() -> Self {
        Self(LinearRgba::new(0.9, 0.55, 0.2, 1.0))
    }
}

// A postfix node: `index` is a solid for NODE_SOLID, else a BoolOp.
#[derive(ShaderType, Clone, Copy, Default)]
struct GpuNode {
    kind: u32,
    index: u32,
}

#[derive(ShaderType, Clone, Copy, Default)]
struct GpuCurve {
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
}

// One extruded face. The bases let every solid share the flat curve,
// tile and index arrays.
#[derive(ShaderType, Clone, Copy, Default)]
struct GpuSolid {
    origin: Vec4,
    tangent: Vec4,
    bitangent: Vec4,
    normal: Vec4,
    grid_min: Vec2,
    inv_tile_size: Vec2,
    extent_min: f32,
    extent_max: f32,
    grid_cols: u32,
    grid_rows: u32,
    curve_base: u32,
    tile_base: u32,
    index_base: u32,
    _pad: u32,
}

#[derive(ShaderType, Clone, Copy, Default)]
struct GpuParams {
    color: Vec4,
    // The proxy box, for the ray's entry and exit.
    bounds_min: Vec4,
    bounds_max: Vec4,
    node_count: u32,
}

/// Raymarches a [`Volume`]: each leaf is an extruded sketch face, and
/// the postfix node list combines them.
///
/// Every array is a runtime-sized storage buffer, so a body is limited
/// only by its own size.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct VolumeMaterial {
    #[uniform(0)]
    params: GpuParams,
    #[storage(1, read_only)]
    nodes: Handle<ShaderBuffer>,
    #[storage(2, read_only)]
    solids: Handle<ShaderBuffer>,
    #[storage(3, read_only)]
    curves: Handle<ShaderBuffer>,
    #[storage(4, read_only)]
    tiles: Handle<ShaderBuffer>,
    #[storage(5, read_only)]
    tile_indices: Handle<ShaderBuffer>,
}

// A zero-length storage buffer has no valid binding, so every array
// keeps at least one (unread) element.
fn upload<T>(
    mut values: Vec<T>,
    buffers: &mut Assets<ShaderBuffer>,
) -> Handle<ShaderBuffer>
where
    T: Default,
    Vec<T>: ShaderType + encase::internal::WriteInto,
{
    if values.is_empty() {
        values.push(T::default());
    }
    buffers.add(ShaderBuffer::from(values))
}

impl VolumeMaterial {
    pub fn new(
        volume: &Volume,
        bounds: Bounds,
        color: LinearRgba,
        buffers: &mut Assets<ShaderBuffer>,
    ) -> Self {
        let mut curves = Vec::new();
        let mut tiles = Vec::new();
        let mut tile_indices = Vec::new();
        let mut solids = Vec::with_capacity(volume.solids.len());

        for solid in &volume.solids {
            let grid = &solid.grid;
            let frame = solid.frame;
            solids.push(GpuSolid {
                origin: frame.origin.extend(0.0),
                tangent: frame.tangent.extend(0.0),
                bitangent: frame.bitangent.extend(0.0),
                normal: frame.normal.extend(0.0),
                grid_min: grid.bounds.min,
                inv_tile_size: Vec2::new(
                    grid.cols as f32,
                    grid.rows as f32,
                ) / grid.bounds.size(),
                extent_min: solid.extent.min,
                extent_max: solid.extent.max,
                grid_cols: grid.cols,
                grid_rows: grid.rows,
                curve_base: curves.len() as u32,
                tile_base: tiles.len() as u32,
                index_base: tile_indices.len() as u32,
                _pad: 0,
            });

            curves.extend(solid.curves.iter().map(|c| GpuCurve {
                p0: c.p0,
                p1: c.p1,
                p2: c.p2,
            }));
            tiles.extend(
                grid.tiles
                    .iter()
                    .map(|t| UVec2::new(t.offset, t.count)),
            );
            tile_indices.extend_from_slice(&grid.indices);
        }

        let nodes = volume
            .nodes
            .iter()
            .map(|node| match node {
                Node::Solid(index) => GpuNode {
                    kind: NODE_SOLID,
                    index: *index,
                },
                Node::Op(op) => GpuNode {
                    kind: NODE_OP,
                    index: match op {
                        BoolOp::Union => 0,
                        BoolOp::Subtract => 1,
                        BoolOp::Intersect => 2,
                    },
                },
            })
            .collect::<Vec<_>>();

        let params = GpuParams {
            color: Vec4::new(
                color.red,
                color.green,
                color.blue,
                color.alpha,
            ),
            bounds_min: bounds.min.extend(0.0),
            bounds_max: bounds.max.extend(0.0),
            node_count: nodes.len() as u32,
        };

        Self {
            params,
            nodes: upload(nodes, buffers),
            solids: upload(solids, buffers),
            curves: upload(curves, buffers),
            tiles: upload(tiles, buffers),
            tile_indices: upload(tile_indices, buffers),
        }
    }
}

impl Material for VolumeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/volume.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Draw the proxy's back faces: the ray needs a fragment even
        // when the camera is inside the box.
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

/// The render entity spawned for a [`Volume`], freed when the component
/// is overwritten, removed, or its entity despawns.
#[derive(Component)]
#[component(on_discard = despawn_render)]
struct VolumeRender {
    proxy: Entity,
}

fn despawn_render(mut world: DeferredWorld, ctx: HookContext) {
    let proxy = world.get::<VolumeRender>(ctx.entity).unwrap().proxy;
    world.commands().entity(proxy).try_despawn();
}

/// Rebuilds a volume's proxy box and material whenever its body changes,
/// so a boolean folded into a `Volume` shows up on its own.
pub(super) fn sync_volume_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VolumeMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    query: Query<
        (Entity, &Volume, Option<&VolumeColor>),
        Or<(Changed<Volume>, Changed<VolumeColor>)>,
    >,
) {
    for (entity, volume, color) in &query {
        let Some(bounds) = volume.bounds() else {
            commands.entity(entity).remove::<VolumeRender>();
            continue;
        };
        let bounds = bounds.padded(PROXY_PAD);
        let color = color.copied().unwrap_or_default().0;

        let proxy = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::from_size(bounds.size()))),
                MeshMaterial3d(materials.add(VolumeMaterial::new(
                    volume,
                    bounds,
                    color,
                    &mut buffers,
                ))),
                Transform::from_translation(bounds.center()),
                ChildOf(entity),
            ))
            .id();

        commands.entity(entity).try_insert(VolumeRender { proxy });
    }
}

pub struct VolumeMaterialPlugin;

impl Plugin for VolumeMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<VolumeMaterial>::default())
            .add_systems(Update, sync_volume_render);
    }
}
