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

/// CBrightLights fills one untextured vertex/index buffer in registration order.
#[derive(Default)]
pub(super) struct BrightLightMeshBatch {
    positions: Vec<[f32; 3]>,
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl BrightLightMeshBatch {
    pub(super) fn clear(&mut self) {
        self.positions.clear();
        self.colours.clear();
        self.indices.clear();
    }

    pub(super) fn append(&mut self, mesh: Mesh, colour: Color) {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            warn_once!("bright_light_batch_missing_positions");
            return;
        };
        let Some(indices) = mesh.indices() else {
            warn_once!("bright_light_batch_missing_indices");
            return;
        };
        let base = self.positions.len() as u32;
        self.indices.extend(indices.iter().map(|index| base + index as u32));
        self.colours.extend(std::iter::repeat_n(colour.to_linear().to_vec4().to_array(), positions.len()));
        self.positions.extend_from_slice(positions);
    }

    pub(super) fn mesh(&self) -> Option<Mesh> {
        if self.indices.is_empty() { return None; }
        Some(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_indices(Indices::U32(self.indices.clone()))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; self.positions.len()])
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colours.clone()))
    }
}

/// Same WESL RenderWare modulation as other GTA draws, with the original
/// SHADOWTYPE_ADDITIVE ONE/ONE blend and depth-test/no-depth-write state.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
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
    fn vertex_shader() -> ShaderRef {
        crate::renderer::ShaderAssets::RW_UNLIT.into()
    }

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
    batches: Vec<CarLightBatch>,
    submitted: Vec<Entity>,
}

#[derive(Default)]
struct CarLightBatch {
    texture_index: usize,
    texture: Option<Handle<Image>>,
    key: u32,
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl CarLightBatch {
    fn append(&mut self, geometry: crate::renderer::Shadows::ProjectedShadowGeometry, colour: [u8; 3]) {
        let base = self.positions.len() as u32;
        let colour = [colour[0] as f32 / 255.0, colour[1] as f32 / 255.0, colour[2] as f32 / 255.0, 1.0];
        self.colours.extend(std::iter::repeat_n(colour, geometry.positions.len()));
        self.positions.extend(geometry.positions.into_iter().map(|p| p.to_bevy().to_array()));
        self.uvs.extend(geometry.uvs);
        self.indices.extend(geometry.indices.into_iter().map(|i| base + i));
    }
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
    let mut submitted = std::mem::take(&mut slots.submitted);
    submitted.clear();
    // RenderStoredShadows batches all ONE/ONE polygons by texture. Keep the
    // same native vertex colours, rather than one material and draw per lamp.
    for batch in &mut slots.batches {
        batch.positions.clear();
        batch.uvs.clear();
        batch.colours.clear();
        batch.indices.clear();
    }
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
                    let batch_index = slots.batches.iter().position(|b| b.texture_index == intent.texture)
                        .unwrap_or_else(|| {
                            slots.batches.push(CarLightBatch { texture_index: intent.texture, ..default() });
                            slots.batches.len() - 1
                        });
                    let batch = &mut slots.batches[batch_index];
                    batch.texture = Some(texture);
                    batch.key = intent.key;
                    batch.append(geometry, intent.color);
                }
                for batch in &slots.batches {
                    if batch.indices.is_empty() { continue; }
                    let texture = batch.texture.as_ref().expect("projected batch has a texture").clone();
                    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
                        .with_inserted_indices(Indices::U32(batch.indices.clone()))
                        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, batch.positions.clone())
                        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, batch.uvs.clone())
                        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; batch.positions.len()])
                        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, batch.colours.clone());
                    let source = GtaUnlitMaterial::from_gta(
                        Color::srgba_u8(255, 255, 255, 128), Some(texture.clone()), true, true, [0.0; 3], 0.0,
                    );
                    let mut uniforms = source.uniforms;
                    uniforms.flags.z = 0.0;
                    uniforms.flags.w = 1.0;
                    let material = GtaCarLightMaterial {
                        uniforms, texture, global_lighting: GTA_GLOBAL_LIGHTING_BUFFER.clone(),
                    };
                    let entity = if let Some(&entity) = slots.entities.get(active)
                        && let Ok((mut mesh_handle, mut material_handle, mut visibility, mut projection)) = draws.get_mut(entity)
                    {
                        // A changed polygon count must receive a matching GPU
                        // allocation; unchanged bytes need no asset event.
                        if !reuse_persistent_mesh_shape(&mut meshes, &mesh_handle.0, &mesh) {
                            meshes.remove(mesh_handle.0.id());
                            mesh_handle.0 = meshes.add(mesh);
                        }
                        if materials.get(&material_handle.0) != Some(&material) {
                            if let Some(mut asset) = materials.get_mut(&material_handle.0) {
                                *asset = material;
                            } else {
                                diagnostics.missing_dependencies += 1;
                                material_handle.0 = materials.add(material);
                            }
                        }
                        if *visibility != Visibility::Visible { *visibility = Visibility::Visible; }
                        if projection.key != batch.key { projection.key = batch.key; }
                        entity
                    } else {
                        commands.spawn((
                            Mesh3d(meshes.add(mesh)), MeshMaterial3d(materials.add(material)),
                            Transform::IDENTITY, Visibility::Visible, NoFrustumCulling,
                            GtaCarLightProjection { key: batch.key },
                        )).id()
                    };
                    // Store after the immutable batch iteration below.
                    submitted.push(entity);
                    active += 1;
                }
                let reused = active.min(slots.entities.len());
                slots.entities[..reused].copy_from_slice(&submitted[..reused]);
                if active > slots.entities.len() {
                    let previous_len = slots.entities.len();
                    slots.entities.extend_from_slice(&submitted[previous_len..]);
                }

            } else {
                diagnostics.missing_texture += 1;
                warn_once!(target: "re3bevy::car_light_projection", "required original particle TXD is absent");
            }
        }
    } else if shadows.is_some() {
        diagnostics.missing_dependencies += 1;
    }
    slots.submitted = submitted;
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

    #[test]
    fn bulb_batch_keeps_all_shapes_in_registration_order_with_per_vertex_alpha() {
        let mut batch = BrightLightMeshBatch::default();
        let colour = Color::srgba_u8(255, 42, 42, 31);
        batch.append(build_vehicle_bright_light_mesh(&bulb(4, 0.0)).unwrap(), colour);
        batch.append(build_vehicle_bright_light_mesh(&bulb(11, 0.0)).unwrap(), Color::WHITE);
        let mesh = batch.mesh().unwrap();
        assert_eq!(mesh.count_vertices(), 16);
        let indices: Vec<_> = mesh.indices().unwrap().iter().collect();
        assert_eq!(indices.len(), 72);
        assert!(indices[..36].iter().all(|&i| i < 8));
        assert!(indices[36..].iter().all(|&i| (8..16).contains(&i)));
        let bevy::mesh::VertexAttributeValues::Float32x4(colours) = mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap() else { panic!("colour format"); };
        assert!(colours[..8].iter().all(|c| *c == colour.to_linear().to_vec4().to_array()));
        assert!(colours[8..].iter().all(|c| *c == [1.0; 4]));
        batch.clear();
        assert!(batch.mesh().is_none());
    }

    #[test]
    fn additive_batch_preserves_native_colours_and_offsets_each_polygon() {
        let mut batch = CarLightBatch::default();
        let geometry = || crate::renderer::Shadows::ProjectedShadowGeometry {
            positions: vec![CVector::ZERO, CVector::new(1.0, 0.0, 0.0), CVector::new(0.0, 1.0, 0.0)],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            indices: vec![0, 2, 1],
        };
        batch.append(geometry(), [45, 128, 255]);
        batch.append(geometry(), [0, 20, 40]);
        assert_eq!(batch.indices, [0, 2, 1, 3, 5, 4]);
        assert_eq!(batch.positions.len(), 6);
        assert_eq!(batch.uvs.len(), 6);
        assert!(batch.colours[..3].iter().all(|c| *c == [45.0/255.0, 128.0/255.0, 1.0, 1.0]));
        assert!(batch.colours[3..].iter().all(|c| *c == [0.0, 20.0/255.0, 40.0/255.0, 1.0]));
    }

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
