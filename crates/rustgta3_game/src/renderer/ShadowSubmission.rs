//! CShadows::RenderStoredShadows batches projected quads by raster and blend.
use super::*;

#[derive(Default, Resource)]
pub(super) struct ShadowSubmissionDiagnostics {
    pub missing_inputs_frames: u64,
    pub missing_particle_frames: u64,
    pub missing_texture_intents: u64,
    pub invalid_texture_intents: u64,
    pub submitted_quads: u64,
    pub submitted_batches: u64,
}

fn shadow_corners(shadow: &ScriptShadow) -> [CVector; 4] {
    if let Some(corners) = shadow.corners {
        corners
    } else {
        let front = CVector::new(shadow.front[0], shadow.front[1], 0.0);
        let side = CVector::new(shadow.side[0], shadow.side[1], 0.0);
        let center = shadow.position;
        [
            CVector::new(
                center.x + front.x - side.x,
                center.y + front.y - side.y,
                center.z,
            ), // Front Left (0.0, 0.0)
            CVector::new(
                center.x + front.x + side.x,
                center.y + front.y + side.y,
                center.z,
            ), // Front Right (1.0, 0.0)
            CVector::new(
                center.x - front.x + side.x,
                center.y - front.y + side.y,
                center.z,
            ), // Rear Right (1.0, 1.0)
            CVector::new(
                center.x - front.x - side.x,
                center.y - front.y - side.y,
                center.z,
            ), // Rear Left (0.0, 1.0)
        ]
    }
}

struct ShadowBatch {
    texture_index: usize,
    additive: bool,
    texture: Handle<Image>,
    positions: Vec<[f32; 3]>,
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
}
impl ShadowBatch {
    fn append(&mut self, shadow: &ScriptShadow, colour: Color) {
        let start = self.positions.len() as u32;
        self.positions
            .extend(shadow_corners(shadow).map(|p| p.to_bevy().to_array()));
        self.colours.extend([colour.to_linear().to_f32_array(); 4]);
        self.indices
            .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }
    fn mesh(self) -> Mesh {
        let vertices = self.positions.len();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(self.indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colours)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; vertices])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            (0..vertices / 4)
                .flat_map(|_| [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
                .collect::<Vec<_>>(),
        )
    }
}

fn append_shadow(
    batches: &mut Vec<ShadowBatch>,
    shadow: &ScriptShadow,
    texture_index: usize,
    additive: bool,
    colour: Color,
    particle_slot: usize,
    shadows: &CShadows,
    texture_cache: &GtaWorldTextureCache,
    diagnostics: &mut ShadowSubmissionDiagnostics,
) {
    let Some(name) = shadows.texture_names.get(texture_index) else {
        diagnostics.invalid_texture_intents += 1;
        return;
    };
    let Some(texture) = texture_cache_lookup(texture_cache, particle_slot, name).map(|(h, _)| h)
    else {
        diagnostics.missing_texture_intents += 1;
        return;
    };
    let index = batches
        .iter()
        .position(|b| b.texture_index == texture_index && b.additive == additive)
        .unwrap_or_else(|| {
            batches.push(ShadowBatch {
                texture_index,
                additive,
                texture,
                positions: Vec::new(),
                colours: Vec::new(),
                indices: Vec::new(),
            });
            batches.len() - 1
        });
    batches[index].append(shadow, colour);
    diagnostics.submitted_quads += 1;
}

pub(super) fn flush_shadow_meshes(
    shadows: Option<ResMut<CShadows>>,
    file_loader: Option<Res<CFileLoader>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
    mut diagnostics: ResMut<ShadowSubmissionDiagnostics>,
) {
    let (Some(mut shadows), Some(file_loader)) = (shadows, file_loader) else {
        diagnostics.missing_inputs_frames += 1;
        hide_persistent_billboard_slots(&immediate_cache.shadow_entities, 0, &mut persistent_query);
        return;
    };
    if shadows.tidy_up_requested {
        shadows.tidy_up_requested = false;
        hide_persistent_billboard_slots(&immediate_cache.shadow_entities, 0, &mut persistent_query);
        return;
    }
    let Some(particle_slot) = file_loader.txd_store.find_txd_slot("particle") else {
        diagnostics.missing_particle_frames += 1;
        hide_persistent_billboard_slots(&immediate_cache.shadow_entities, 0, &mut persistent_query);
        return;
    };
    if !sync_txd_slot_texture_cache(particle_slot, &file_loader, &mut texture_cache, &mut images) {
        diagnostics.missing_particle_frames += 1;
        hide_persistent_billboard_slots(&immediate_cache.shadow_entities, 0, &mut persistent_query);
        return;
    }
    let mut batches = Vec::new();
    for shadow in &shadows.script_shadows {
        let (texture_index, additive) = match shadow.texture {
            1 => (0, false),
            2 => (1, false),
            3 => (3, true),
            4 => (2, false),
            5 => (4, true),
            6 => (8, false),
            _ => {
                diagnostics.invalid_texture_intents += 1;
                continue;
            }
        };
        let alpha = f32::from(shadow.intensity.clamp(0, 255)) / 255.0;
        if alpha <= 0.0 {
            continue;
        }
        let colour = if additive || shadow.texture == 6 {
            Color::srgba_u8(
                shadow.color[0],
                shadow.color[1],
                shadow.color[2],
                (alpha * 255.0).round() as u8,
            )
        } else {
            Color::srgba(0.0, 0.0, 0.0, alpha)
        };
        append_shadow(
            &mut batches,
            shadow,
            texture_index,
            additive,
            colour,
            particle_slot,
            &shadows,
            &texture_cache,
            &mut diagnostics,
        );
    }
    let active_count = batches.len();
    diagnostics.submitted_batches += active_count as u64;
    for (index, batch) in batches.into_iter().enumerate() {
        let material = StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(batch.texture.clone()),
            unlit: true,
            alpha_mode: if batch.additive {
                AlphaMode::Add
            } else {
                AlphaMode::Blend
            },
            double_sided: true,
            cull_mode: None,
            depth_bias: 50.0,
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            ..default()
        };
        let entity = update_persistent_mesh_slot(
            immediate_cache.shadow_entities.get(index).copied(),
            batch.mesh(),
            material,
            Transform::IDENTITY,
            &mut persistent_query,
            &mut meshes,
            &mut materials,
            &mut commands,
        );
        if let Some(slot) = immediate_cache.shadow_entities.get_mut(index) {
            *slot = entity;
        } else {
            immediate_cache.shadow_entities.push(entity);
        }
    }
    hide_persistent_billboard_slots(
        &immediate_cache.shadow_entities,
        active_count,
        &mut persistent_query,
    );
}
