use bevy::core_pipeline::FullscreenShader;
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::ecs::query::QueryItem;
use bevy::ecs::system::lifetimeless::Read;
use bevy::prelude::*;
use bevy::render::extract_component::*;
use bevy::render::render_graph::*;
use bevy::render::render_resource::binding_types::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::{
    RenderContext, RenderDevice, RenderQueue,
};
use bevy::render::view::{
    ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};

use crate::sdf::transform::{
    SdfTransformPlugin, SdfTransformUniform,
};

pub mod primitves;
pub mod transform;

const SHADER_ASSET_PATH: &str = "shaders/sdf_raymarch.wgsl";

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SdfTransformPlugin,
            ExtractComponentPlugin::<SdfCamera>::default(),
            UniformComponentPlugin::<SdfCamera>::default(),
            ExtractComponentPlugin::<SdfTransformUniform>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                RenderStartup,
                (init_sdf_pipeline, init_sdf_buffers),
            )
            .add_systems(
                Render,
                update_transform_buffer
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
        let view_uniforms = world.resource::<ViewUniforms>();
        let sdf_cameras =
            world.resource::<ComponentUniforms<SdfCamera>>();

        let (
            Some(pipeline),
            Some(view_uniforms_binding),
            Some(sdf_cameras_binding),
            Some(transform_buffer_binding),
        ) = (
            pipeline_cache
                .get_render_pipeline(sdf_pipeline.pipeline_id),
            view_uniforms.uniforms.binding(),
            sdf_cameras.uniforms().binding(),
            sdf_buffers.transform_buffer.binding(),
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
    transform_buffer: BufferVec<SdfTransformUniform>,
}

fn update_transform_buffer(
    q_transforms: Query<&SdfTransformUniform>,
    mut buffers: ResMut<SdfBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // TODO: Optimize this to only update changed/added/removed transform!
    buffers.transform_buffer.clear();
    let mut capacity = 0;
    for transform in q_transforms.iter() {
        buffers.transform_buffer.push(*transform);
        capacity += 1;
    }

    if capacity > 0 {
        buffers
            .transform_buffer
            .write_buffer(&render_device, &render_queue);
    }
}

fn init_sdf_buffers(mut commands: Commands) {
    let mut transform_buffer = BufferVec::<SdfTransformUniform>::new(
        BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    );
    transform_buffer.set_label(Some("sdf_transform_buffer"));

    commands.insert_resource(SdfBuffers { transform_buffer });
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
                storage_buffer_read_only::<SdfTransformUniform>(
                    false,
                ),
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
