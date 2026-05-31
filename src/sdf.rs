use bevy::core_pipeline::FullscreenShader;
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::ecs::query::QueryItem;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::lifetimeless::Read;
use bevy::math::Affine3;
use bevy::prelude::*;
use bevy::render::extract_component::*;
use bevy::render::render_graph::*;
use bevy::render::render_resource::binding_types::*;
use bevy::render::render_resource::encase::private::WriteInto;
use bevy::render::render_resource::*;
use bevy::render::renderer::{
    RenderContext, RenderDevice, RenderQueue,
};
use bevy::render::sync_world::{MainEntity, RenderEntity};
use bevy::render::view::{
    ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
};
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup,
    RenderSystems,
};
use std::collections::{HashMap, HashSet};

use crate::sdf::boolean::{
    BooleanOp, SdfBooleanOp, SdfBooleanPlugin, SdfExtractedOperand,
    SdfOperandOf, SdfOrder,
};
use crate::sdf::primitves::{
    SdfCapsule, SdfCuboid, SdfPrimitivePlugin, SdfRoundCuboid,
    SdfSphere, SdfTorus,
};
use crate::sdf::transform::{SdfGlobalTransform, SdfTransformPlugin};

pub mod boolean;
pub mod primitves;
pub mod transform;

/// Filter for operands whose relationship or ordering changed.
type ChangedOperandFilter = Or<(
    Changed<SdfOperandOf>,
    Changed<SdfBooleanOp>,
    Changed<SdfOrder>,
)>;

/// Query for extracting operands whose relationship or ordering changed into the render world.
type ChangedOperandQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static RenderEntity,
        &'static SdfOperandOf,
        &'static SdfBooleanOp,
        &'static SdfOrder,
    ),
    ChangedOperandFilter,
>;

/// Filter used to detect any render-relevant change in the input buffer.
type AnyInputChanged = Or<(
    Changed<SdfGlobalTransform>,
    Changed<PrimitiveIndex>,
    Added<PrimitiveIndex>,
    Changed<SdfExtractedOperand>,
)>;

/// Filter used to detect a changed or newly-added primitive component.
type PrimitiveChanged<T> = Or<(Changed<T>, Added<T>)>;

/// [`RenderDevice`] and [`RenderQueue`] grouped.
#[derive(SystemParam)]
struct RenderResources<'w> {
    device: Res<'w, RenderDevice>,
    queue: Res<'w, RenderQueue>,
}

const SHADER_ASSET_PATH: &str = "shaders/sdf_raymarch.wgsl";

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SdfTransformPlugin,
            SdfPrimitivePlugin,
            SdfBooleanPlugin,
            ExtractComponentPlugin::<SdfCamera>::default(),
            UniformComponentPlugin::<SdfCamera>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(ExtractSchedule, extract_sdf_operands)
            .add_systems(
                RenderStartup,
                (init_sdf_pipeline, init_sdf_buffers),
            )
            .add_systems(
                Render,
                (
                    (
                        update_primitive_buffers.with_input((
                            PrimitiveType::Sphere,
                            |b: &mut SdfBuffers| &mut b.sphere_buffer,
                        )),
                        update_primitive_buffers.with_input((
                            PrimitiveType::Cuboid,
                            |b: &mut SdfBuffers| &mut b.cuboid_buffer,
                        )),
                        update_primitive_buffers.with_input((
                            PrimitiveType::RoundCuboid,
                            |b: &mut SdfBuffers| {
                                &mut b.round_cuboid_buffer
                            },
                        )),
                        update_primitive_buffers.with_input((
                            PrimitiveType::Capsule,
                            |b: &mut SdfBuffers| {
                                &mut b.capsule_buffer
                            },
                        )),
                        update_primitive_buffers.with_input((
                            PrimitiveType::Torus,
                            |b: &mut SdfBuffers| &mut b.torus_buffer,
                        )),
                    ),
                    update_input_buffers,
                )
                    .chain()
                    .in_set(RenderSystems::PrepareResources),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<SdfNode>>(
                Core3d,
                SdfRenderLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    SdfRenderLabel,
                    Node3d::StartMainPassPostProcessing,
                ),
            );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SdfRenderLabel;

#[derive(Default)]
pub struct SdfNode;

impl ViewNode for SdfNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<SdfCamera>,
        Read<DynamicUniformIndex<SdfCamera>>,
        Read<ViewUniformOffset>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, _, sdf_camera_index, view_offset): QueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let sdf_pipeline = world.resource::<SdfPipeline>();
        let sdf_buffers = world.resource::<SdfBuffers>();

        if sdf_buffers.input_count == 0 {
            return Ok(());
        }

        let view_uniforms = world.resource::<ViewUniforms>();
        let sdf_cameras =
            world.resource::<ComponentUniforms<SdfCamera>>();

        let (
            Some(pipeline),
            Some(view_uniforms_binding),
            Some(sdf_cameras_binding),
            Some(transform_buffer_binding),
            Some(sphere_buffer_binding),
            Some(cuboid_buffer_binding),
            Some(round_cuboid_buffer_binding),
            Some(capsule_buffer_binding),
            Some(torus_buffer_binding),
        ) = (
            pipeline_cache
                .get_render_pipeline(sdf_pipeline.pipeline_id),
            view_uniforms.uniforms.binding(),
            sdf_cameras.uniforms().binding(),
            sdf_buffers.input_buffer.binding(),
            sdf_buffers.sphere_buffer.binding(),
            sdf_buffers.cuboid_buffer.binding(),
            sdf_buffers.round_cuboid_buffer.binding(),
            sdf_buffers.capsule_buffer.binding(),
            sdf_buffers.torus_buffer.binding(),
        )
        else {
            return Ok(());
        };

        let post_process_write = view_target.post_process_write();

        // The bind_group gets created each frame.
        //
        // Normally, you would create a bind_group in the `Queue` set,
        // but this doesn't work with the `post_process_write()`
        // because each call will alternate the source/destination.
        //
        // The only way to have the correct source/destination for the
        // `bind_group` is to make sure you get it during the node
        // execution.
        let bind_group =
            render_context.render_device().create_bind_group(
                "sdf_bind_group",
                &pipeline_cache
                    .get_bind_group_layout(&sdf_pipeline.layout),
                &BindGroupEntries::sequential((
                    post_process_write.source,
                    &sdf_pipeline.screen_sampler,
                    view_uniforms_binding,
                    sdf_cameras_binding,
                    transform_buffer_binding,
                    sphere_buffer_binding,
                    cuboid_buffer_binding,
                    round_cuboid_buffer_binding,
                    capsule_buffer_binding,
                    torus_buffer_binding,
                )),
            );

        // Begin the render pass.
        let mut render_pass = render_context
            .begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("sdf_pass"),
                color_attachments: &[Some(
                    RenderPassColorAttachment {
                        view: post_process_write.destination,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations::default(),
                    },
                )],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        // This is mostly just wgpu boilerplate for drawing a
        // fullscreen triangle, using the pipeline/bind_group created
        // above.
        render_pass.set_render_pipeline(pipeline);
        // By passing in the index of the post process settings on
        // this view, we ensure that in the event that multiple
        // settings were sent to the GPU (as would be the case with
        // multiple cameras), we use the correct one.
        render_pass.set_bind_group(
            0,
            &bind_group,
            &[view_offset.offset, sdf_camera_index.index()],
        );
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

/// Global data used by the render pipeline. Created once on startup.
#[derive(Resource)]
struct SdfPipeline {
    layout: BindGroupLayoutDescriptor,
    screen_sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

#[derive(Resource)]
struct SdfBuffers {
    input_buffer: BufferVec<SdfInput>,
    sphere_buffer: BufferVec<SdfSphere>,
    cuboid_buffer: BufferVec<SdfCuboid>,
    round_cuboid_buffer: BufferVec<SdfRoundCuboid>,
    capsule_buffer: BufferVec<SdfCapsule>,
    torus_buffer: BufferVec<SdfTorus>,
    input_count: u32,
}

/// Extracts [`SdfExtractedOperand`] for all operand entities into the render world.
///
/// Using [`Changed`] on the main-world components gives correct first-insertion detection
/// without spuriously triggering every frame.
fn extract_sdf_operands(
    q_changed_operands: Extract<ChangedOperandQuery>,
    mut removed: Extract<RemovedComponents<SdfOperandOf>>,
    render_entities: Extract<Query<&RenderEntity>>,
    mut commands: Commands,
) {
    for (render_entity, operand_of, bool_op, order) in
        q_changed_operands.iter()
    {
        commands.entity(render_entity.id()).insert(
            SdfExtractedOperand {
                group_entity: operand_of.0,
                op: bool_op.0,
                order: order.0,
            },
        );
    }

    for main_entity in removed.read() {
        if let Ok(render_entity) = render_entities.get(main_entity) {
            commands
                .entity(render_entity.id())
                .remove::<SdfExtractedOperand>();
        }
    }
}

/// Emit one group into the packed buffer in preorder.
///
/// By emitting the group header before recursing into children, the header's
/// buffer position becomes its stable, unique identity.  Every child record
/// carries `parent_index` = that position, so the shader can detect group
/// boundaries purely from `parent_index` mismatches.
fn emit_group(
    group: Entity,
    parent_header_index: u32,
    boolean_op: u32,
    primitives_of: &HashMap<Entity, Vec<(usize, SdfInput)>>,
    children_of: &HashMap<Entity, Vec<(usize, Entity, u32)>>,
    buffer: &mut BufferVec<SdfInput>,
) -> bool {
    // Save position so it can roll back if this group turns out to be empty.
    let start = buffer.len();
    let header_index = start as u32;

    // Emit group header, rolled back below if no children follow.
    buffer.push(SdfInput {
        has_children: 1,
        boolean_op,
        parent_index: parent_header_index,
        ..SdfInput::default()
    });

    // Build and sort slots by SdfOrder
    // Interleave direct-primitive slots and subgroup slots respecting user order.
    let prim_count = primitives_of.get(&group).map_or(0, |v| v.len());
    let child_count = children_of.get(&group).map_or(0, |v| v.len());
    let mut slots: Vec<(usize, bool, usize)> =
        Vec::with_capacity(prim_count + child_count);

    if let Some(prims) = primitives_of.get(&group) {
        for (i, &(order, _)) in prims.iter().enumerate() {
            slots.push((order, false, i));
        }
    }
    if let Some(children) = children_of.get(&group) {
        for (i, &(order, _, _)) in children.iter().enumerate() {
            slots.push((order, true, i));
        }
    }
    slots.sort_unstable_by_key(|&(order, _, _)| order);

    // Emit children in order
    for (_, is_sub, idx) in slots {
        if is_sub {
            let (_, child, child_op) = children_of[&group][idx];
            // Recursive call returns false and self-truncates if child is empty.
            emit_group(
                child,
                header_index, // child's parent is this group's header
                child_op,
                primitives_of,
                children_of,
                buffer,
            );
        } else {
            // Leaf primitive, stamp the real parent_index before emitting.
            let mut input = primitives_of[&group][idx].1;
            input.parent_index = header_index;
            buffer.push(input);
        }
    }

    // If nothing was added beyond the header, roll back entirely.
    if buffer.len() == start + 1 {
        buffer.truncate(start);
        return false;
    }

    true
}

fn update_input_buffers(
    q_primitives: Query<(
        &SdfGlobalTransform,
        &PrimitiveType,
        &PrimitiveIndex,
        Option<&SdfExtractedOperand>,
    )>,
    q_group_operands: Query<(&SdfExtractedOperand, &MainEntity)>,
    q_changed: Query<(), AnyInputChanged>,
    removed_primitives: RemovedComponents<PrimitiveIndex>,
    removed_operands: RemovedComponents<SdfExtractedOperand>,
    mut buffers: ResMut<SdfBuffers>,
    render: RenderResources,
) {
    // TODO: Optimize this to only update changed/added/removed transform!
    if q_changed.is_empty()
        && removed_primitives.is_empty()
        && removed_operands.is_empty()
    {
        return;
    }

    buffers.input_buffer.clear();

    let mut group_entities: HashSet<Entity> = HashSet::new();
    let mut operand_of: HashMap<Entity, &SdfExtractedOperand> =
        HashMap::new();

    for (operand, main_entity) in q_group_operands.iter() {
        group_entities.insert(operand.group_entity);
        operand_of.insert(main_entity.id(), operand);
    }

    // Stores only entities that are themselves groups (not leaf primitives).
    let mut children_of: HashMap<Entity, Vec<(usize, Entity, u32)>> =
        HashMap::with_capacity(group_entities.len());
    for (&child, operand) in &operand_of {
        if group_entities.contains(&child) {
            children_of
                .entry(operand.group_entity)
                .or_default()
                .push((operand.order, child, operand.op as u32));
        }
    }
    for v in children_of.values_mut() {
        v.sort_unstable_by_key(|&(order, _, _)| order);
    }

    // Collect primitives per group.
    let mut primitives_of: HashMap<Entity, Vec<(usize, SdfInput)>> =
        HashMap::new();

    for (transform, ty, index, operand_opt) in q_primitives.iter() {
        match operand_opt {
            None => {
                // Root-level primitive, lives directly in the scene, no parent group.
                buffers.input_buffer.push(SdfInput::new(
                    transform,
                    ty,
                    index,
                    BooleanOp::Union as u32,
                    u32::MAX, // no parent group
                ));
            }
            Some(operand) => {
                // Grouped primitive, parent_index will be patched.
                primitives_of
                    .entry(operand.group_entity)
                    .or_default()
                    .push((
                        operand.order,
                        SdfInput::new(
                            transform,
                            ty,
                            index,
                            operand.op as u32,
                            0, // placeholder, patched in emit_group
                        ),
                    ));
            }
        }
    }

    for v in primitives_of.values_mut() {
        v.sort_unstable_by_key(|&(order, _)| order);
    }

    // Emit root groups that are not a child of any other group.
    let mut roots: Vec<Entity> = group_entities
        .iter()
        .filter(|g| !operand_of.contains_key(g))
        .copied()
        .collect();
    roots.sort_unstable();

    for root in roots {
        emit_group(
            root,
            u32::MAX,
            BooleanOp::Union as u32,
            &primitives_of,
            &children_of,
            &mut buffers.input_buffer,
        );
    }

    buffers.input_count = buffers.input_buffer.len() as u32;

    if buffers.input_buffer.is_empty() {
        buffers.input_buffer.push(SdfInput::default());
    }
    buffers
        .input_buffer
        .write_buffer(&render.device, &render.queue);
}

fn update_primitive_buffers<
    T: Component + ShaderType + WriteInto + Default + Clone,
    F: FnMut(&mut SdfBuffers) -> &mut BufferVec<T>,
>(
    InMut((ty, get_buffer)): InMut<(PrimitiveType, F)>,
    mut commands: Commands,
    q_primitives: Query<(&T, Entity, Option<&PrimitiveIndex>)>,
    q_changed: Query<(), PrimitiveChanged<T>>,
    removed: RemovedComponents<T>,
    mut buffers: ResMut<SdfBuffers>,
    render: RenderResources,
) {
    // TODO: Optimize this to only update changed/added/removed primitive!
    let buffer = get_buffer(&mut buffers);

    if q_changed.is_empty()
        && removed.is_empty()
        && !buffer.is_empty()
    {
        return;
    }

    buffer.clear();
    // TODO: This could be replaced with a custom self managed empty buffer
    // next time. (This is needed right now since a `BufferVec` won't be created
    // if the data is empty!)
    buffer.push(T::default());
    for (i, (primitive, entity, existing_index)) in
        q_primitives.iter().enumerate()
    {
        buffer.push(primitive.clone());
        // Only insert PrimitiveIndex when it actually changed.
        if existing_index.map(|idx: &PrimitiveIndex| idx.0)
            != Some(i as u32)
        {
            commands
                .entity(entity)
                .insert((*ty, PrimitiveIndex(i as u32)));
        }
    }

    buffer.write_buffer(&render.device, &render.queue);
}

fn init_sdf_buffers(mut commands: Commands) {
    fn create_buffer_vec<T: ShaderType + WriteInto>(
        label: &str,
    ) -> BufferVec<T> {
        let mut buffer = BufferVec::<T>::new(
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        buffer.set_label(Some(label));
        buffer
    }
    let input_buffer =
        create_buffer_vec::<SdfInput>("sdf_input_buffer");
    let sphere_buffer =
        create_buffer_vec::<SdfSphere>("sdf_sphere_buffer");
    let cuboid_buffer =
        create_buffer_vec::<SdfCuboid>("sdf_cuboid_buffer");
    let round_cuboid_buffer = create_buffer_vec::<SdfRoundCuboid>(
        "sdf_round_cuboid_buffer",
    );
    let capsule_buffer =
        create_buffer_vec::<SdfCapsule>("sdf_capsule_buffer");
    let torus_buffer =
        create_buffer_vec::<SdfTorus>("sdf_torus_buffer");

    commands.insert_resource(SdfBuffers {
        input_buffer,
        sphere_buffer,
        cuboid_buffer,
        round_cuboid_buffer,
        capsule_buffer,
        torus_buffer,
        input_count: 0,
    });
}

fn init_sdf_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "sdf_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            // The layout entries will only be visible in the fragment stage.
            ShaderStages::FRAGMENT,
            (
                // The screen texture.
                texture_2d(TextureSampleType::Float {
                    filterable: true,
                }),
                // The sampler that will be used to sample the screen texture.
                sampler(SamplerBindingType::Filtering),
                // The view uniform from the camera.
                uniform_buffer::<ViewUniform>(true),
                // The settings uniform.
                uniform_buffer::<SdfCamera>(true),
                // Transforms of the SDF objects.
                storage_buffer_read_only::<SdfInput>(false),
                storage_buffer_read_only::<SdfSphere>(false),
                storage_buffer_read_only::<SdfCuboid>(false),
                storage_buffer_read_only::<SdfRoundCuboid>(false),
                storage_buffer_read_only::<SdfCapsule>(false),
                storage_buffer_read_only::<SdfTorus>(false),
            ),
        ),
    );
    let screen_sampler =
        render_device.create_sampler(&SamplerDescriptor::default());

    let shader = asset_server.load(SHADER_ASSET_PATH);
    // Setup a fullscreen triangle for the vertex state.
    let vertex_state = fullscreen_shader.to_vertex_state();
    let pipeline_id = pipeline_cache
        // Add the pipeline to the cache and queue its creation.
        .queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("sdf_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: vertex_state,
            fragment: Some(FragmentState {
                shader,
                // Make sure this matches the entry point of your
                // shader. It can be anything as long as it matches
                // here and in the shader.
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
    commands.insert_resource(SdfPipeline {
        layout,
        screen_sampler,
        pipeline_id,
    });
}

#[derive(Component, Debug, Clone, Copy)]
enum PrimitiveType {
    Sphere,
    Cuboid,
    RoundCuboid,
    Capsule,
    Torus,
}

#[derive(Component, Debug, Clone, Copy)]
struct PrimitiveIndex(pub u32);

/// Configurations for a SDF camera.
/// [`Camera`]s must have this component in order to render SDFs.
#[derive(
    Component,
    ExtractComponent,
    ShaderType,
    Reflect,
    Debug,
    Clone,
    Copy,
)]
pub struct SdfCamera {
    /// Max raymarch steps. Defaults to 128.
    pub max_step: u32,
    /// Maximum distance the raymarcher can travel.
    pub far_plane: f32,
}

impl Default for SdfCamera {
    fn default() -> Self {
        Self {
            max_step: 128,
            far_plane: 1000.0,
        }
    }
}

#[derive(
    ExtractComponent,
    Component,
    ShaderType,
    Reflect,
    Default,
    Debug,
    Clone,
    Copy,
)]
struct SdfInput {
    pub local_from_world: [Vec4; 3],
    pub scale: f32,
    pub primitive_type: u32,
    pub primitive_index: u32,
    /// Boolean op this record applies to its parent's accumulator.
    pub boolean_op: u32,
    /// Buffer index of this record's parent group header.
    pub parent_index: u32,
    /// `1` if this record is a group header; `0` if it is a leaf primitive.
    pub has_children: u32,
}

impl SdfInput {
    pub fn new(
        global_transform: &SdfGlobalTransform,
        ty: &PrimitiveType,
        index: &PrimitiveIndex,
        boolean_op: u32,
        parent_index: u32,
    ) -> Self {
        Self {
            local_from_world: Affine3::from(
                &global_transform.world_from_local().inverse(),
            )
            .to_transpose(),
            scale: global_transform.scale(),
            primitive_type: *ty as u32,
            // Index 0 is for default value.
            // TODO: We could cache only primitives with different settings?
            primitive_index: index.0 + 1,
            boolean_op,
            parent_index,
            has_children: 0,
        }
    }
}
