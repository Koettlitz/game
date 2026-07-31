use std::marker::PhantomData;

use bevy::{
    core_pipeline::{Core2d, Core2dSystems, FullscreenShader},
    material::descriptor::{
        BindGroupLayoutDescriptor, CachedRenderPipelineId, FragmentState, RenderPipelineDescriptor,
    },
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutEntries, ColorTargetState, ColorWrites,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, Sampler,
            SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureFormat,
            TextureSampleType, TextureViewId,
            binding_types::{sampler, texture_2d, uniform_buffer},
            encase::private::WriteInto,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        uniform::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        view::ViewTarget,
    },
};

use crate::asset::Phantom;

pub trait ShaderDescriptor {
    type Uniform;

    fn name() -> &'static str;
    fn shader_path() -> &'static str;
}

pub struct ShaderPlugin<T>(Phantom<T>);

impl<T> Default for ShaderPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S> Plugin for ShaderPlugin<S>
where
    S: ShaderDescriptor + 'static,
    S::Uniform: ExtractComponent + ShaderType + WriteInto + Clone,
{
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<S::Uniform>::default(),
            UniformComponentPlugin::<S::Uniform>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(RenderStartup, init_pipeline::<S>);
        render_app.add_systems(
            Core2d,
            pass_pipeline::<S>.in_set(Core2dSystems::PostProcess),
        );
    }
}

#[derive(Resource)]
struct Pipeline<T> {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
    _marker: Phantom<T>,
}

impl<T> Pipeline<T> {
    fn new(
        layout: BindGroupLayoutDescriptor,
        sampler: Sampler,
        pipeline_id: CachedRenderPipelineId,
    ) -> Self {
        Self {
            layout,
            sampler,
            pipeline_id,
            _marker: PhantomData,
        }
    }
}

fn init_pipeline<S>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) where
    S: ShaderDescriptor + 'static,
    S::Uniform: ShaderType,
{
    let layout = BindGroupLayoutDescriptor::new(
        format!("{}_bind_group_layout", S::name()),
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<S::Uniform>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let shader = asset_server.load(S::shader_path());

    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some(format!("{}_pipeline", S::name()).into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });

    commands.insert_resource(Pipeline::<S::Uniform>::new(layout, sampler, pipeline_id));
}

#[derive(Default)]
struct BindGroupCache {
    cached: Option<(TextureViewId, BindGroup)>,
}

fn pass_pipeline<S>(
    view: ViewQuery<(&ViewTarget, &S::Uniform, &DynamicUniformIndex<S::Uniform>)>,
    pipeline: Option<Res<Pipeline<S::Uniform>>>,
    pipeline_cache: Res<PipelineCache>,
    settings_uniforms: Res<ComponentUniforms<S::Uniform>>,
    mut cache: Local<BindGroupCache>,
    mut ctx: RenderContext,
) where
    S: ShaderDescriptor + 'static,
    S::Uniform: Component + ShaderType + WriteInto,
{
    let Some(pipeline) = pipeline else { return };
    let (view_target, _settings, settings_index) = view.into_inner();

    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
        return;
    };
    let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
        return;
    };

    let post_process = view_target.post_process_write();

    let bind_group = match &mut cache.cached {
        Some((id, bg)) if post_process.source.id() == *id => bg,
        cached => {
            let bg = ctx.render_device().create_bind_group(
                format!("{}_group", S::name()).as_str(),
                &pipeline_cache.get_bind_group_layout(&pipeline.layout),
                &BindGroupEntries::sequential((
                    post_process.source,
                    &pipeline.sampler,
                    settings_binding.clone(),
                )),
            );
            let (_, bg) = cached.insert((post_process.source.id(), bg));
            bg
        }
    };

    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some(format!("{}_pass", S::name()).as_str()),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(render_pipeline);
    pass.set_bind_group(0, bind_group, &[settings_index.index()]);
    pass.draw(0..3, 0..1);
}
