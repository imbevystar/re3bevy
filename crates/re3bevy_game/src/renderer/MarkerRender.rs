//! C3dMarker::Render: original marker atomics, encoded RW colours and native lights.
use super::*;
use crate::renderer::RendererMaterials::{
    GTA_GLOBAL_LIGHTING_BUFFER, GtaUnlitUniforms, RwTextureAlphaMode,
};
use bevy::{
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MarkerMaterialKey)]
pub struct GtaMarkerMaterial {
    #[uniform(0)]
    pub uniforms: GtaUnlitUniforms,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    #[storage(3, read_only)]
    pub global_lighting: Handle<ShaderBuffer>,
    pub arrow: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MarkerMaterialKey {
    arrow: bool,
}
impl From<&GtaMarkerMaterial> for MarkerMaterialKey {
    fn from(m: &GtaMarkerMaterial) -> Self {
        Self { arrow: m.arrow }
    }
}
impl Material for GtaMarkerMaterial {
    fn fragment_shader() -> ShaderRef {
        crate::renderer::ShaderAssets::RW_UNLIT.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.arrow {
            AlphaMode::Opaque
        } else {
            AlphaMode::Blend
        }
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        _: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        if let Some(depth) = d.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(key.bind_group_data.arrow);
        }
        Ok(())
    }
}

#[derive(Component)]
pub struct GtaMarkerDraw;
pub(super) fn reset_marker_draws(mut draws: Query<&mut Visibility, With<GtaMarkerDraw>>) {
    for mut visibility in &mut draws {
        *visibility = Visibility::Hidden;
    }
}
#[derive(Default)]
pub(super) struct MarkerDrawSlots {
    entities: Vec<Entity>,
}

pub(super) fn flush_markers(
    mut fx: Option<ResMut<CSpecialFX>>,
    install: Option<Res<OriginalGtaInstall>>,
    loader: Option<Res<CFileLoader>>,
    mut cache: ResMut<GtaMarkerAssetCache>,
    mut textures: ResMut<GtaWorldTextureCache>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaMarkerMaterial>>,
    mut slots: Local<MarkerDrawSlots>,
    mut commands: Commands,
    mut draws: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<GtaMarkerMaterial>,
            &mut Transform,
            &mut Visibility,
        ),
        With<GtaMarkerDraw>,
    >,
) {
    let mut active = 0;
    if let (Some(fx), Some(install), Some(loader)) =
        (fx.as_deref_mut(), install.as_deref(), loader.as_deref())
    {
        let intents = fx.drain_marker_intents();
        let particle = loader.txd_store.find_txd_slot("particle");
        if let Some(slot) = particle {
            let _ = sync_txd_slot_texture_cache(slot, loader, &mut textures, &mut images);
        }
        for marker in intents {
            let arrow = marker.marker_type == MARKERTYPE_ARROW;
            let parts = if arrow {
                &mut cache.arrow_parts
            } else {
                &mut cache.cylinder_parts
            };
            if parts.is_empty() {
                match if arrow {
                    load_marker_arrow_parts(install, &mut meshes)
                } else {
                    load_marker_cylinder_parts(install, &mut meshes)
                } {
                    Ok(loaded) => *parts = loaded,
                    Err(error) => {
                        warn_once!(target: "re3bevy::markers", "required original marker asset failed: {error}");
                        continue;
                    }
                }
            }
            for part in parts.iter() {
                let texture = part.texture_name.as_deref().and_then(|name| {
                    particle.and_then(|slot| {
                        texture_cache_lookup(&textures, slot, &name.to_ascii_lowercase())
                            .map(|(image, _)| image)
                    })
                });
                if part.texture_name.is_some() && texture.is_none() {
                    warn_once!(target: "re3bevy::markers", "required particle TXD marker texture is not resident");
                    continue;
                }
                let source = GtaUnlitMaterial::from_gta_lit(
                    Color::srgba_u8(
                        marker.colour[0],
                        marker.colour[1],
                        marker.colour[2],
                        marker.colour[3],
                    ),
                    texture.clone(),
                    RwTextureAlphaMode::Blend,
                    false,
                    [0.0; 3],
                    0.0,
                    0.0,
                    [0.0; 3],
                    [0.0; 3],
                    [0.0; 3],
                    part.surface_ambient,
                    part.surface_diffuse,
                    part.has_prelit,
                    1.0,
                    true,
                );
                let mut uniforms = source.uniforms;
                uniforms.lighting_options.z = 1.0;
                let material = GtaMarkerMaterial {
                    uniforms,
                    texture,
                    global_lighting: GTA_GLOBAL_LIGHTING_BUFFER.clone(),
                    arrow,
                };
                let transform = Transform::from_translation(marker.position.to_bevy())
                    .with_rotation(Quat::from_rotation_y(marker.rotation_degrees.to_radians()))
                    .with_scale(Vec3::splat(marker.size));
                if let Some(&entity) = slots.entities.get(active)
                    && let Ok((mut mesh, mut mat, mut pose, mut visible)) = draws.get_mut(entity)
                {
                    mesh.0 = part.mesh.clone();
                    if let Some(mut asset) = materials.get_mut(&mat.0) {
                        *asset = material;
                    } else {
                        mat.0 = materials.add(material);
                    }
                    *pose = transform;
                    *visible = Visibility::Visible;
                } else {
                    let entity = commands
                        .spawn((
                            GtaMarkerDraw,
                            Mesh3d(part.mesh.clone()),
                            MeshMaterial3d(materials.add(material)),
                            transform,
                            Visibility::Visible,
                            NoFrustumCulling,
                            Name::new("C3dMarker original atomic"),
                        ))
                        .id();
                    if let Some(slot) = slots.entities.get_mut(active) {
                        *slot = entity;
                    } else {
                        slots.entities.push(entity);
                    }
                }
                active += 1;
            }
        }
    }
    for &entity in slots.entities.iter().skip(active) {
        if let Ok((_, _, _, mut visibility)) = draws.get_mut(entity) {
            *visibility = Visibility::Hidden;
        }
    }
}
