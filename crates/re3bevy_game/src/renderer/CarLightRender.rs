//! GPU submission for CShadows' projected additive light polygons.
use super::*;
use crate::renderer::{
    RendererMaterials::{GTA_GLOBAL_LIGHTING_BUFFER, GtaUnlitUniforms},
    Shadows::{CarLightShadowIntent, ShadowProjectionDiagnostics, project_additive_shadow},
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

/// CBrightLights::Render's untextured automobile cubes (SpecialFX.cpp:694-862).
/// Shared by front/rear types; handling selects long, small, big or tall.
pub(super) fn build_vehicle_bright_light_mesh(
    intent: &crate::renderer::SpecialFX::BrightLightRenderIntent,
) -> Option<Mesh> {
    let shape = match intent.light_type {
        4..=7 => intent.light_type - 4,
        8..=11 => intent.light_type - 8,
        _ => return None,
    };
    let [side, front, up] = match shape {
        0 => [0.2, 0.1, 0.1],
        1 => [0.08, 0.08, 0.08],
        2 => [0.15, 0.15, 0.15],
        _ => [0.08, 0.08, 0.2],
    };
    let positions = (0..8)
        .map(|i| {
            (intent.position
                + intent.side * if i & 1 == 0 { -side } else { side }
                + intent.front * if i & 2 == 0 { front } else { -front }
                + intent.up * if i & 4 == 0 { up } else { -up })
            .to_bevy()
            .to_array()
        })
        .collect::<Vec<_>>();
    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(vec![
            0, 2, 1, 1, 2, 3, 3, 5, 1, 3, 7, 5, 2, 7, 3, 2, 6, 7, 4, 0, 1, 4, 1, 5, 6, 0, 4, 6, 2,
            0, 6, 5, 7, 6, 4, 5,
        ]))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 8]),
    )
}

/// The caller consumes one native random draw per bulb, including white fronts.
pub(super) fn vehicle_bright_light_colour(
    intent: &crate::renderer::SpecialFX::BrightLightRenderIntent,
    flicker: u8,
) -> Option<[u8; 4]> {
    let rgb = match intent.light_type {
        4..=7 => [255, 255, 255],
        8..=11 => [255, flicker, flicker],
        _ => return None,
    };
    let distance = intent.camera_distance;
    let alpha = if distance < 15.0 {
        31.0
    } else if distance < 30.0 {
        31.0 + 224.0 * ((distance - 15.0) / 15.0)
    } else if distance < 45.0 {
        255.0
    } else {
        255.0 * (1.0 - (distance - 45.0) / 15.0)
    };
    Some([rgb[0], rgb[1], rgb[2], alpha.clamp(0.0, 255.0) as u8])
}

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
    let intents = shadows.as_mut().map(|shadows| {
        let mut intents: Vec<_> = std::mem::take(&mut shadows.car_light_shadows)
            .into_iter()
            .map(|intent| (intent, 6.0))
            .collect();
        intents.extend(
            std::mem::take(&mut shadows.pickup_shadows)
                .into_iter()
                .map(|intent| {
                    (
                        CarLightShadowIntent {
                            key: intent.key,
                            texture: intent.texture as usize,
                            position: intent.position,
                            front: intent.front,
                            side: intent.side,
                            color: intent.color,
                        },
                        4.0,
                    )
                }),
        );
        // re3 StoreStaticShadow uses the same polygon-clipping and ONE/ONE
        // render path as automobile lights, with its authored 15m vertical
        // reach. A flat StandardMaterial quad at the lamp's ground height
        // misses slopes and multiplies additive RGB by alpha a second time.
        intents.extend(
            std::mem::take(&mut shadows.static_shadows)
                .into_iter()
                .filter(|intent| intent.color != [0, 0, 0])
                .map(|intent| {
                    (
                        CarLightShadowIntent {
                            key: intent.key,
                            texture: intent.texture as usize,
                            position: intent.position,
                            front: intent.front,
                            side: intent.side,
                            color: intent.color,
                        },
                        15.0,
                    )
                }),
        );
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
                    // re3 adds the encoded D3D8 shadow RGB without fog. The
                    // scene's HDR attachment otherwise decodes that small
                    // encoded value *before* addition, hiding road glows.
                    uniforms.flags.w = 1.0;
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

#[cfg(test)]
mod bulb_tests {
    use super::*;
    use crate::renderer::SpecialFX::BrightLightRenderIntent;

    fn bulb(light_type: u8, distance: f32) -> BrightLightRenderIntent {
        BrightLightRenderIntent {
            position: CVector::new(10.0, 20.0, 30.0),
            side: CVector::new(0.0, 1.0, 0.0),
            front: CVector::new(-1.0, 0.0, 0.0),
            up: CVector::new(0.0, 0.0, 1.0),
            light_type,
            camera_distance: distance,
        }
    }

    #[test]
    fn native_shapes_follow_actor_basis_and_share_front_rear_topology() {
        for (shape, [side, front, up]) in [
            [0.2, 0.1, 0.1],
            [0.08, 0.08, 0.08],
            [0.15, 0.15, 0.15],
            [0.08, 0.08, 0.2],
        ]
        .into_iter()
        .enumerate()
        {
            for base in [4, 8] {
                let mesh =
                    build_vehicle_bright_light_mesh(&bulb(base + shape as u8, 30.0)).unwrap();
                let bevy::mesh::VertexAttributeValues::Float32x3(positions) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                else {
                    panic!("position format")
                };
                assert_eq!(positions.len(), 8);
                assert_eq!(
                    positions[0],
                    CVector::new(10.0 - front, 20.0 - side, 30.0 + up)
                        .to_bevy()
                        .to_array()
                );
                assert_eq!(
                    positions[7],
                    CVector::new(10.0 + front, 20.0 + side, 30.0 - up)
                        .to_bevy()
                        .to_array()
                );
                assert_eq!(mesh.indices().unwrap().len(), 36);
            }
        }
        assert!(build_vehicle_bright_light_mesh(&bulb(3, 30.0)).is_none());
    }

    #[test]
    fn near_and_far_fades_preserve_native_bulb_colours() {
        for (distance, alpha) in [
            (0.0, 31),
            (15.0, 31),
            (22.5, 143),
            (30.0, 255),
            (45.0, 255),
            (52.5, 127),
            (60.0, 0),
        ] {
            assert_eq!(
                vehicle_bright_light_colour(&bulb(4, distance), 42),
                Some([255, 255, 255, alpha])
            );
            assert_eq!(
                vehicle_bright_light_colour(&bulb(8, distance), 42),
                Some([255, 42, 42, alpha])
            );
        }
    }
}
