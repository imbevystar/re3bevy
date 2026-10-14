//! GTA III PC environment mapping: re3 VehicleModelInfo.cpp and Game.cpp;
//! librw 5501c4fdc7425ff926be59369a13593bb6c81b54, gl3matfx.cpp,
//! d3d9matfx.cpp and shaders/matfx_env.{vert,frag}/matfx_env_{VS,PS}.hlsl.
//! This reference-derived adapter retains Bevy's actor and asset ownership.

use bevy::{
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        render_resource::{
            AsBindGroup, BlendState, RenderPipelineDescriptor, ShaderType,
            SpecializedMeshPipelineError, TextureFormat,
        },
        storage::ShaderBuffer,
    },
    shader::{ShaderDefVal, ShaderRef},
};
use std::collections::BTreeMap;

use super::{
    BevyRender::ModelMaterialDrawable,
    NeoRender::NeoMaterialLink,
    RendererMaterials::{GtaUnlitMaterial, GtaUnlitUniforms},
    ShaderAssets,
};
use crate::{core::FileLoader::CFileLoader, modelinfo::ModelInfo::ModelInfoSlot};

/// Retain the source material through pipeline swaps. Runtime retirement must
/// not depend on which MeshMaterial3d currently draws this same entity.
#[derive(Component, Clone)]
pub(super) struct GtaOriginalMaterial(pub Handle<GtaUnlitMaterial>);

#[derive(Component, Clone, Copy)]
pub(super) struct GtaPcEnvSurface {
    pub coefficient: f32,
}

pub(super) fn surface_for_drawable(
    drawable: &ModelMaterialDrawable,
    model_index: usize,
    loader: &CFileLoader,
) -> Option<GtaPcEnvSurface> {
    // The owner is the vehicle clump, including its separate wheel model.
    // A ped/world material's surface specular alone does not install MatFX.
    matches!(
        loader.model_info.get_model_info(model_index),
        Some(ModelInfoSlot::Vehicle(_))
    )
    .then(|| pc_environment_coefficient(drawable.specular))
    .flatten()
    .map(|coefficient| GtaPcEnvSurface { coefficient })
}

fn pc_environment_coefficient(specular: f32) -> Option<f32> {
    (specular.is_finite() && specular > 0.0).then_some(specular * 0.5)
}

#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct GtaPcEnvParams {
    pub base: GtaUnlitUniforms,
    /// x=PC coefficient, y=Game.cpp's grey envMapColor, zw=reserved.
    pub environment: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
#[data(0, GtaPcEnvParams, binding_array(10))]
// Two samplers per material: keep the same 128-descriptor budget as RW_UNLIT.
// Actor-origin changes can then update slab data without rebuilding GPU binds.
#[bindless(limit(64))]
pub struct GtaPcEnvMaterial {
    pub params: GtaPcEnvParams,
    #[texture(1)]
    #[sampler(2)]
    pub diffuse_texture: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub environment_texture: Handle<Image>,
    #[storage(5, read_only, binding_array(11))]
    pub global_lighting: Handle<ShaderBuffer>,
    pub alpha_mode: AlphaMode,
}

impl From<&GtaPcEnvMaterial> for GtaPcEnvParams {
    fn from(material: &GtaPcEnvMaterial) -> Self {
        material.params
    }
}

impl GtaPcEnvMaterial {
    fn from_original(
        base: &GtaUnlitMaterial,
        coefficient: f32,
        environment: Handle<Image>,
    ) -> Self {
        Self {
            params: GtaPcEnvParams {
                base: base.uniforms,
                environment: Vec4::new(coefficient, 128.0 / 255.0, 0.0, 0.0),
            },
            diffuse_texture: base.base_color_texture.clone(),
            environment_texture: environment,
            global_lighting: base.global_lighting.clone(),
            alpha_mode: match base.alpha_mode {
                AlphaMode::Blend => AlphaMode::Premultiplied,
                mode => mode,
            },
        }
    }
}

impl Material for GtaPcEnvMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderAssets::RW_PC_ENVMAP.into()
    }
    fn fragment_shader() -> ShaderRef {
        ShaderAssets::RW_PC_ENVMAP.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &bevy::mesh::MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // CRenderer::RenderOneNonRoad disables vehicle culling. MatFX retains
        // that pass's Z test/write even for the sorted alpha atomics; it is
        // one combined draw, not a second transparent reflection overlay.
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(true);
        }
        if let Some(fragment) = descriptor.fragment.as_mut() {
            // Match the parent's GTA camera raster specialization without
            // changing its shared RendererMaterials implementation.
            let encoded = fragment
                .targets
                .first()
                .and_then(Option::as_ref)
                .is_some_and(|target| {
                    matches!(
                        target.format,
                        TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm
                    )
                });
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("GTA_ENCODED_TARGET".into(), encoded));
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
            }
        }
        Ok(())
    }
}

#[derive(Component)]
pub(super) struct GtaPcEnvMaterialLink(pub Handle<GtaPcEnvMaterial>);

#[derive(Resource, Default)]
pub(super) struct GtaPcEnvMaterials {
    handles: BTreeMap<Entity, Handle<GtaPcEnvMaterial>>,
}

#[derive(Resource, Default)]
pub struct GtaPcEnvDiagnostics {
    /// Required reflection01 is unresolved; the original diffuse draw remains.
    pub missing_environment_frames: u64,
    pub missing_source_material_frames: u64,
}

/// Reinstating a source draw after MatFX/Neo must publish any CPU-only origin
/// updates retained while that source material wasn't being rendered.
pub(super) fn refresh_restored_original_materials(
    mut originals: ResMut<Assets<GtaUnlitMaterial>>,
    restored: Query<&MeshMaterial3d<GtaUnlitMaterial>, (With<GtaOriginalMaterial>, Changed<MeshMaterial3d<GtaUnlitMaterial>>)>,
) {
    for handle in &restored {
        if let Some(material) = originals.get_mut(&handle.0) {
            // Publish a Modified event even though the CPU value is current.
            let _ = material.into_inner();
        }
    }
}

pub(super) fn sync_pc_environment_materials(
    mut commands: Commands,
    cache: Res<super::BevyRender::GtaWorldTextureCache>,
    originals: Res<Assets<GtaUnlitMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GtaPcEnvMaterial>>,
    mut owned: ResMut<GtaPcEnvMaterials>,
    mut diagnostics: ResMut<GtaPcEnvDiagnostics>,
    drawables: Query<(
        Entity,
        &GtaOriginalMaterial,
        &GtaPcEnvSurface,
        Option<&GtaPcEnvMaterialLink>,
        Option<&NeoMaterialLink>,
    )>,
    links: Query<&GtaPcEnvMaterialLink>,
    no_effect: Query<
        (Entity, &GtaOriginalMaterial, &GtaPcEnvMaterialLink),
        Without<GtaPcEnvSurface>,
    >,
) {
    // LoadEnvironmentMaps explicitly overrides both original rasters to
    // rwFILTERLINEAR, retaining the TXD's addressing modes.
    for handle in [cache.env_map.as_ref(), cache.white_texture.as_ref()]
        .into_iter()
        .flatten()
    {
        if let Some(image) = images.get(handle) {
            let mut sampler = match &image.sampler {
                ImageSampler::Descriptor(sampler) => sampler.clone(),
                ImageSampler::Default => ImageSamplerDescriptor::default(),
            };
            if sampler.mag_filter != ImageFilterMode::Linear
                || sampler.min_filter != ImageFilterMode::Linear
            {
                sampler.mag_filter = ImageFilterMode::Linear;
                sampler.min_filter = ImageFilterMode::Linear;
                if let Some(mut image) = images.get_mut(handle) {
                    image.sampler = ImageSampler::Descriptor(sampler);
                }
            }
        }
    }
    // Entity retirement (including New Game and replay refresh) also releases
    // the derivative material; no second mesh or gameplay lifetime is created.
    owned.handles.retain(|entity, handle| {
        let live = links.get(*entity).is_ok();
        if !live {
            materials.remove(handle.id());
        }
        live
    });
    for (entity, original, link) in &no_effect {
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<GtaPcEnvMaterial>>()
            .remove::<GtaPcEnvMaterialLink>()
            .insert(MeshMaterial3d(original.0.clone()));
        materials.remove(link.0.id());
        owned.handles.remove(&entity);
    }
    let mut missing_environment = false;
    let mut missing_source = false;
    for (entity, original, surface, link, neo) in &drawables {
        if neo.is_some_and(NeoMaterialLink::replaces_base) {
            continue;
        }
        let base = originals.get(&original.0);
        let environment = cache.env_map.as_ref();
        if base.is_none() || environment.is_none() {
            missing_environment |= environment.is_none();
            missing_source |= base.is_none();
            if let Some(link) = link {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<GtaPcEnvMaterial>>()
                    .remove::<GtaPcEnvMaterialLink>()
                    .insert(MeshMaterial3d(original.0.clone()));
                materials.remove(link.0.id());
                owned.handles.remove(&entity);
            }
            continue;
        }
        let material = GtaPcEnvMaterial::from_original(
            base.unwrap(),
            surface.coefficient,
            environment.unwrap().clone(),
        );
        if let Some(link) = link {
            if materials.get(&link.0) != Some(&material)
                && let Some(mut current) = materials.get_mut(&link.0)
            {
                *current = material;
            }
        } else {
            let handle = materials.add(material);
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<GtaUnlitMaterial>>()
                .insert((
                    MeshMaterial3d(handle.clone()),
                    GtaPcEnvMaterialLink(handle.clone()),
                ));
            owned.handles.insert(entity, handle);
        }
    }
    diagnostics.missing_environment_frames += u64::from(missing_environment);
    diagnostics.missing_source_material_frames += u64::from(missing_source);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_source_draw_publishes_the_current_cpu_origin() {
        use bevy::{asset::AssetPlugin, ecs::message::MessageCursor};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<GtaUnlitMaterial>()
            .add_systems(Update, refresh_restored_original_materials);
        let handle = app.world_mut().resource_mut::<Assets<GtaUnlitMaterial>>()
            .add(GtaUnlitMaterial::from_gta(Color::WHITE, None, false, false, [0.0; 3], 0.0));
        let entity = app.world_mut().spawn(GtaOriginalMaterial(handle.clone())).id();
        app.update();
        let mut cursor = MessageCursor::<AssetEvent<GtaUnlitMaterial>>::default();
        let messages = app.world().resource::<Messages<AssetEvent<GtaUnlitMaterial>>>();
        cursor.read(messages).for_each(drop);
        let origin = Vec4::new(12.0, 3.0, -8.0, 1.0);
        app.world_mut().resource_mut::<Assets<GtaUnlitMaterial>>()
            .get_mut_untracked(&handle).unwrap().uniforms.actor_origin = origin;
        app.update();
        assert!(!cursor.read(app.world().resource::<Messages<AssetEvent<GtaUnlitMaterial>>>())
            .any(|event| matches!(event, AssetEvent::Modified { id } if *id == handle.id())));
        app.world_mut().entity_mut(entity).insert(MeshMaterial3d(handle.clone()));
        app.update();
        assert!(cursor.read(app.world().resource::<Messages<AssetEvent<GtaUnlitMaterial>>>())
            .any(|event| matches!(event, AssetEvent::Modified { id } if *id == handle.id())));
        assert_eq!(app.world().resource::<Assets<GtaUnlitMaterial>>()
            .get(&handle).unwrap().uniforms.actor_origin, origin);
    }

    #[test]
    fn detached_wheels_follow_vehicle_owner_while_world_specular_stays_default() {
        let mut loader = CFileLoader::new();
        loader.model_info.add_vehicle_model(90);
        loader.model_info.add_simple_model(250);
        let mut drawable = ModelMaterialDrawable {
            mesh: Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            ),
            base_color: Color::WHITE,
            texture_name: None,
            dual_texture_name: None,
            texture_model_index: 250,
            texture_txd_slot: None,
            vertex_count: 0,
            index_count: 0,
            vehicle_colour_slot: None,
            specular: 1.0,
            surface_ambient: 1.0,
            surface_diffuse: 1.0,
            has_prelit: false,
            dynamic_vehicle_component: true,
            vehicle_component_transform: None,
        };
        assert_eq!(
            surface_for_drawable(&drawable, 90, &loader)
                .unwrap()
                .coefficient,
            0.5
        );
        assert!(surface_for_drawable(&drawable, 250, &loader).is_none());
        drawable.specular = 0.0;
        assert!(surface_for_drawable(&drawable, 90, &loader).is_none());
    }

    #[test]
    fn original_pc_coefficients_are_not_neo_shininess_or_ps2_intensities() {
        assert_eq!(GtaPcEnvParams::min_size().get(), 112);
        assert_eq!(pc_environment_coefficient(1.0), Some(0.5));
        assert_eq!(pc_environment_coefficient(0.001), Some(0.0005));
        assert_eq!(pc_environment_coefficient(0.0), None);
        assert_eq!(pc_environment_coefficient(-1.0), None);
        assert_eq!(pc_environment_coefficient(f32::NAN), None);
    }

    #[test]
    fn pc_material_refresh_wait_and_retirement_share_the_original_entity() {
        let mut app = App::new();
        app.init_resource::<Assets<GtaUnlitMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<GtaPcEnvMaterial>>()
            .init_resource::<super::super::BevyRender::GtaWorldTextureCache>()
            .init_resource::<GtaPcEnvMaterials>()
            .init_resource::<GtaPcEnvDiagnostics>()
            .add_systems(Update, sync_pc_environment_materials);
        let original = app
            .world_mut()
            .resource_mut::<Assets<GtaUnlitMaterial>>()
            .add(GtaUnlitMaterial::from_gta(
                Color::WHITE,
                None,
                false,
                false,
                [0.0; 3],
                0.0,
            ));
        let entity = app
            .world_mut()
            .spawn((
                GtaOriginalMaterial(original.clone()),
                GtaPcEnvSurface { coefficient: 0.5 },
                MeshMaterial3d(original.clone()),
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .get::<MeshMaterial3d<GtaUnlitMaterial>>(entity)
                .is_some()
        );
        assert_eq!(
            app.world()
                .resource::<GtaPcEnvDiagnostics>()
                .missing_environment_frames,
            1
        );
        app.world_mut()
            .resource_mut::<super::super::BevyRender::GtaWorldTextureCache>()
            .env_map = Some(Handle::default());
        app.update();
        let reflected = app
            .world()
            .get::<GtaPcEnvMaterialLink>(entity)
            .unwrap()
            .0
            .clone();
        assert!(
            app.world()
                .get::<MeshMaterial3d<GtaUnlitMaterial>>(entity)
                .is_none()
        );
        assert_eq!(
            app.world().get::<GtaOriginalMaterial>(entity).unwrap().0,
            original
        );
        {
            let mut originals = app.world_mut().resource_mut::<Assets<GtaUnlitMaterial>>();
            let mut base = originals.get_mut(&original).unwrap();
            base.uniforms.colour.w = 0.4;
            base.alpha_mode = AlphaMode::Blend;
        }
        app.update();
        assert_eq!(
            app.world().get::<GtaPcEnvMaterialLink>(entity).unwrap().0,
            reflected
        );
        let materials = app.world().resource::<Assets<GtaPcEnvMaterial>>();
        let refreshed = materials.get(&reflected).unwrap();
        assert_eq!(refreshed.params.base.colour.w, 0.4);
        assert_eq!(refreshed.alpha_mode, AlphaMode::Premultiplied);
        app.world_mut()
            .resource_mut::<super::super::BevyRender::GtaWorldTextureCache>()
            .env_map = None;
        app.update();
        assert!(
            app.world()
                .get::<MeshMaterial3d<GtaUnlitMaterial>>(entity)
                .is_some()
        );
        assert!(
            app.world()
                .resource::<Assets<GtaPcEnvMaterial>>()
                .get(&reflected)
                .is_none()
        );
        app.world_mut()
            .resource_mut::<super::super::BevyRender::GtaWorldTextureCache>()
            .env_map = Some(Handle::default());
        app.update();
        app.world_mut()
            .entity_mut(entity)
            .remove::<GtaPcEnvSurface>();
        app.update();
        assert!(app.world().get::<GtaPcEnvMaterialLink>(entity).is_none());
        assert!(
            app.world()
                .resource::<Assets<GtaPcEnvMaterial>>()
                .is_empty()
        );
        app.world_mut()
            .entity_mut(entity)
            .insert(GtaPcEnvSurface { coefficient: 0.5 });
        app.update();
        app.world_mut().despawn(entity);
        app.update();
        assert!(
            app.world()
                .resource::<Assets<GtaPcEnvMaterial>>()
                .is_empty()
        );
        assert!(
            app.world()
                .resource::<Assets<GtaUnlitMaterial>>()
                .get(&original)
                .is_some()
        );
    }
}
