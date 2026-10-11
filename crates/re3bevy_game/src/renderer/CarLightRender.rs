//! GPU submission for CShadows' transient additive automobile light polygons.
use super::*;
use crate::renderer::{
    RendererMaterials::{GTA_GLOBAL_LIGHTING_BUFFER, GtaUnlitUniforms},
    Shadows::{ShadowProjectionDiagnostics, project_additive_shadow, CarLightShadowIntent},
};
use bevy::{
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
        RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

/// Same WESL RenderWare modulation as other GTA draws, with the original
/// SHADOWTYPE_ADDITIVE ONE/ONE blend and depth-test/no-depth-write state.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GtaCarLightMaterial {
    #[uniform(0)]
    pub uniforms: GtaUnlitUniforms,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    #[storage(3, read_only)]
    pub global_lighting: Handle<ShaderBuffer>,
}

impl Material for GtaCarLightMaterial {
    fn fragment_shader() -> ShaderRef {
        crate::renderer::ShaderAssets::RW_UNLIT.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
        }
        let additive = BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: additive,
                    alpha: additive,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Component)]
pub struct GtaCarLightProjection {
    pub key: u32,
}

#[derive(Default)]
pub(super) struct CarLightMeshSlots {
    entities: Vec<Entity>,
}

pub(super) fn flush_car_light_meshes(
    mut shadows: Option<ResMut<CShadows>>,
    loader: Option<Res<CFileLoader>>,
    mut diagnostics: ResMut<ShadowProjectionDiagnostics>,
    mut slots: Local<CarLightMeshSlots>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaCarLightMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut textures: ResMut<GtaWorldTextureCache>,
    mut draws: Query<(
        &mut Mesh3d,
        &mut MeshMaterial3d<GtaCarLightMaterial>,
        &mut Visibility,
        &mut GtaCarLightProjection,
    )>,
) {
    diagnostics.begin_frame();
    let mut active = 0;
    let intents = shadows
        .as_mut()
        .map(|shadows| {
            let mut intents: Vec<_> = std::mem::take(&mut shadows.car_light_shadows)
                .into_iter().map(|intent| (intent, 6.0)).collect();
            intents.extend(std::mem::take(&mut shadows.pickup_shadows).into_iter().map(|intent| (
                CarLightShadowIntent { key: intent.key, texture: intent.texture as usize, position: intent.position,
                    front: intent.front, side: intent.side, color: intent.color }, 4.0)));
            intents
        });
    if let (Some(shadows), Some(loader), Some(intents)) =
        (shadows.as_ref(), loader.as_ref(), intents)
    {
        if !shadows.tidy_up_requested && !intents.is_empty() {
            diagnostics.intents = intents.len();
            if let Some(slot) = loader.txd_store.find_txd_slot("particle") {
                let _ = sync_txd_slot_texture_cache(slot, &loader, &mut textures, &mut images);
                for (intent, z_distance) in intents {
                    let Some(texture_name) = shadows.texture_names.get(intent.texture) else {
                        diagnostics.missing_texture += 1;
                        warn_once!(target: "re3bevy::car_light_projection", "invalid native shadow texture index");
                        continue;
                    };
                    let Some(texture) = texture_cache_lookup(&textures, slot, texture_name)
                        .map(|(handle, _)| handle)
                    else {
                        diagnostics.missing_texture += 1;
                        warn_once!(target: "re3bevy::car_light_projection", texture_name, "required original light-shadow texture is not resident");
                        continue;
                    };
                    let geometry = project_additive_shadow(
                        &intent,
                        z_distance,
                        &loader.world,
                        &loader.model_info,
                        &mut diagnostics,
                    );
                    if geometry.indices.is_empty() {
                        continue;
                    }
                    let vertices = geometry.positions.len();
                    let mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::default(),
                    )
                    .with_inserted_indices(Indices::U32(geometry.indices))
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        geometry
                            .positions
                            .into_iter()
                            .map(|p| p.to_bevy().to_array())
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, geometry.uvs)
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_NORMAL,
                        vec![[0.0, 1.0, 0.0]; vertices],
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0; 4]; vertices]);
                    let source = GtaUnlitMaterial::from_gta(
                        Color::srgba_u8(intent.color[0], intent.color[1], intent.color[2], 128),
                        Some(texture.clone()),
                        true,
                        true,
                        [0.0; 3],
                        0.0,
                    );
                    let mut uniforms = source.uniforms;
                    // ONE/ONE uses RGB directly; the native alpha 128 must not
                    // premultiply it as Bevy's standard Add material would.
                    uniforms.flags.z = 0.0;
                    let material = GtaCarLightMaterial {
                        uniforms,
                        texture,
                        global_lighting: GTA_GLOBAL_LIGHTING_BUFFER.clone(),
                    };
                    let entity = if let Some(&entity) = slots.entities.get(active)
                        && let Ok((
                            mut mesh_handle,
                            mut material_handle,
                            mut visibility,
                            mut projection,
                        )) = draws.get_mut(entity)
                    {
                        if let Some(mut asset) = meshes.get_mut(&mesh_handle.0) {
                            *asset = mesh;
                        } else {
                            diagnostics.missing_dependencies += 1;
                            mesh_handle.0 = meshes.add(mesh);
                        }
                        if let Some(mut asset) = materials.get_mut(&material_handle.0) {
                            *asset = material;
                        } else {
                            diagnostics.missing_dependencies += 1;
                            material_handle.0 = materials.add(material);
                        }
                        *visibility = Visibility::Visible;
                        projection.key = intent.key;
                        entity
                    } else {
                        commands
                            .spawn((
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(materials.add(material)),
                                Transform::IDENTITY,
                                Visibility::Visible,
                                NoFrustumCulling,
                                GtaCarLightProjection { key: intent.key },
                            ))
                            .id()
                    };
                    if let Some(slot) = slots.entities.get_mut(active) {
                        *slot = entity;
                    } else {
                        slots.entities.push(entity);
                    }
                    active += 1;
                }
            } else {
                diagnostics.missing_texture += 1;
                warn_once!(target: "re3bevy::car_light_projection", "required original particle TXD is absent");
            }
        }
    } else if shadows.is_some() {
        diagnostics.missing_dependencies += 1;
    }
    diagnostics.visible_meshes = active;
    for &entity in &slots.entities[active..] {
        if let Ok((_, _, mut visibility, _)) = draws.get_mut(entity) {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

pub(super) fn reset_car_light_meshes(
    mut shadows: Option<ResMut<CShadows>>,
    mut draws: Query<&mut Visibility, With<GtaCarLightProjection>>,
) {
    if let Some(shadows) = shadows.as_mut() {
        shadows.car_light_shadows.clear();
        shadows.pickup_shadows.clear();
    }
    for mut visibility in &mut draws {
        *visibility = Visibility::Hidden;
    }
}
