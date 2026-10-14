//! CBuilding::ReplaceWithNewModel's instance and template destruction boundary.
use super::*;
use crate::core::Streaming::STREAMFLAGS_DONT_REMOVE;
use rustgta3_core::core::GameState::CGameState;

#[derive(Debug, Default, Resource)]
pub struct WorldModelReplacementDiagnostics {
    pub consumed: u64,
    pub same_model_replacements: u64,
    pub cold_replacements: u64,
    pub retired_instances: u64,
    pub removed_templates: u64,
    pub remove_model_calls: u64,
    pub evicted_meshes: u64,
    pub evicted_materials: u64,
    pub missing_original_models: u64,
}

fn retire_instance(
    entity_index: usize,
    registry: &mut GtaWorldMeshRegistry,
    commands: &mut Commands,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
) -> u64 {
    let mut retired = 0;
    if let Some(slot) = registry.slots.remove(&entity_index) {
        registry.set_slot_presence(entity_index, false);
        registry.slots_epoch = registry.slots_epoch.wrapping_add(1);
        despawn_world_mesh_slot(commands, materials, &slot);
        retired += 1;
    }
    if let Some(slot) = registry.retired_slots.remove(&entity_index) {
        despawn_world_mesh_slot(commands, materials, &slot);
        retired += 1;
    }
    retired
}

/// CStreaming deletes RwObjects before ScanWorld. bImBeingRendered is only
/// set around the actual native draw call, not for membership in a future
/// render list. Keeping that membership as protection made unused island
/// LODs retain their model reference and prevented RemoveModel indefinitely.
pub(super) fn consume_world_streaming_deletions(
    mut commands: Commands,
    loader: Option<ResMut<CFileLoader>>,
    streaming: Option<ResMut<CStreaming>>,
    mut registry: ResMut<GtaWorldMeshRegistry>,
    mut refs: ResMut<GtaRuntimeEntityModelRefs>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
) {
    let Some(mut loader) = loader else { return };
    let Some(mut streaming) = streaming else {
        warn_once!("world_streaming_retirement_waiting_streaming");
        return;
    };
    for index in loader.take_pending_world_rw_object_deletes() {
        retire_instance(index, &mut registry, &mut commands, &mut materials);
        if let Some(model) = refs.owned.remove(&(GtaRuntimeModelRefSource::World, index)) {
            streaming.remove_model_ref_with_adapter(model, &mut *loader);
        }
    }
    loader.flush_pending_streaming_model_removals(&mut streaming);
}

fn evict_model_assets(
    model: usize,
    cache: &mut GtaWorldModelAssetCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
    diagnostics: &mut WorldModelReplacementDiagnostics,
) {
    let material_keys: Vec<_> = cache
        .entries
        .keys()
        .copied()
        .filter(|key| key.model_index == model)
        .collect();
    for key in material_keys {
        if let Some(assets) = cache.entries.remove(&key) {
            for material in assets.materials {
                materials.remove(material.id());
                diagnostics.evicted_materials += 1;
            }
        }
    }
    let geometry_keys: Vec<_> = cache
        .geometry
        .keys()
        .copied()
        .filter(|key| key.0 == model)
        .collect();
    for key in geometry_keys {
        cache.geometry_idle_prunes.remove(&key);
        if let Some(geometry) = cache.geometry.remove(&key) {
            for mesh in geometry.meshes {
                meshes.remove(mesh.id());
                diagnostics.evicted_meshes += 1;
            }
        }
    }
}

/// This lifecycle stage is outside gameplay/pause/camera guards. Streaming
/// first reconciles prior draw ownership; after SCM/audio, consume exact world
/// commands before RenderPrep can submit any replacement geometry.
pub(super) fn consume_world_model_replacements(
    mut commands: Commands,
    loader: Option<ResMut<CFileLoader>>,
    streaming: Option<ResMut<CStreaming>>,
    game: Option<Res<CGameState>>,
    mut registry: ResMut<GtaWorldMeshRegistry>,
    mut refs: ResMut<GtaRuntimeEntityModelRefs>,
    mut cache: ResMut<GtaWorldModelAssetCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut diagnostics: ResMut<WorldModelReplacementDiagnostics>,
) {
    let Some(mut loader) = loader else {
        return;
    };
    if !loader.world.has_pending_model_replacements() {
        return;
    }
    let (Some(mut streaming), Some(game)) = (streaming, game) else {
        warn_once!("world_model_replacement_waiting_streaming_level");
        return;
    };
    for replacement in loader.world.take_model_replacements() {
        diagnostics.consumed += 1;
        diagnostics.same_model_replacements +=
            u64::from(replacement.old_model_index == replacement.new_model_index);
        let retired = retire_instance(
            replacement.entity_index,
            &mut registry,
            &mut commands,
            &mut materials,
        );
        diagnostics.retired_instances += retired;
        diagnostics.cold_replacements += u64::from(retired == 0);
        let key = (GtaRuntimeModelRefSource::World, replacement.entity_index);
        if let Some(model) = refs.owned.remove(&key) {
            streaming.remove_model_ref_with_adapter(model, &mut *loader);
        }
        let Some(old_model) = loader
            .model_info
            .get_model_info(replacement.old_model_index)
        else {
            diagnostics.missing_original_models += 1;
            warn_once!(
                model = replacement.old_model_index,
                "world_model_replacement_missing_original_old_model"
            );
            continue;
        };
        if old_model.base().ref_count == 0 {
            let had_template = old_model.base().rw_object_loaded;
            streaming.remove_model_with_adapter(replacement.old_model_index, Some(&mut *loader));
            diagnostics.remove_model_calls += 1;
            // Recyclable LOAD_SCENE mirrors hold no native reference. They
            // cannot retain handles into a template that RemoveModel destroyed.
            let recyclable: Vec<_> = registry
                .retired_slots
                .iter()
                .filter_map(|(&index, slot)| {
                    (slot.model_index == replacement.old_model_index).then_some(index)
                })
                .collect();
            for index in recyclable {
                diagnostics.retired_instances +=
                    retire_instance(index, &mut registry, &mut commands, &mut materials);
            }
            evict_model_assets(
                replacement.old_model_index,
                &mut cache,
                &mut meshes,
                &mut materials,
                &mut diagnostics,
            );
            diagnostics.removed_templates += u64::from(
                had_template
                    && loader
                        .model_info
                        .get_model_info(replacement.old_model_index)
                        .is_some_and(|model| !model.base().rw_object_loaded),
            );
        }
        let request = replacement.request_big_building.unwrap_or_else(|| {
            loader
                .world
                .entities
                .get(replacement.entity_index)
                .is_some_and(|entity| {
                    entity.is_big_building
                        && (entity.level == 0 || entity.level == game.curr_level as i32)
                })
        });
        if request {
            if loader
                .model_info
                .get_model_info(replacement.new_model_index)
                .is_none()
            {
                diagnostics.missing_original_models += 1;
                warn_once!(
                    model = replacement.new_model_index,
                    "world_model_replacement_missing_original_new_model"
                );
                continue;
            }
            streaming.request_model_with_adapter(
                replacement.new_model_index,
                STREAMFLAGS_DONT_REMOVE,
                &*loader,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Streaming::StreamLoadState, World::EEntityType};

    #[test]
    fn streaming_retires_previously_visible_lod_and_releases_model_before_next_scan() {
        for explicit_model_removal in [false, true] {
            let (mut app, _, drawable) = app_with_replacement(false, 1, true, false, false);
            app.add_systems(Update, consume_world_streaming_deletions.before(consume_world_model_replacements));
            app.world_mut().resource_scope(|world, mut loader: Mut<CFileLoader>| {
                loader.world.take_model_replacements();
                loader.world.entities[0].model_index = 1000;
                loader.world.entities[0].is_big_building = true;
                loader.world.entities[0].level = 1;
                loader.world.building_pool.push(0);
                if explicit_model_removal {
                    loader.remove_big_buildings(&mut world.resource_mut::<CStreaming>(), 1);
                } else {
                    loader.mark_world_rw_object_deleted(0);
                }
            });
            let mut timer = CTimer::initialise();
            timer.start_user_pause();
            app.insert_resource(timer);
            app.update();
            assert!(app.world().get_entity(drawable.unwrap()).is_err());
            assert!(app.world().resource::<GtaWorldMeshRegistry>().slots.is_empty());
            assert!(app.world().resource::<GtaRuntimeEntityModelRefs>().owned.is_empty());
            assert_eq!(app.world().resource::<CFileLoader>().model_info.get_model_info(1000).unwrap().base().ref_count, 0);
            assert_eq!(app.world().resource::<CStreaming>().info_for_model[1000].load_state,
                if explicit_model_removal { StreamLoadState::NotLoaded } else { StreamLoadState::Loaded });
        }
    }

    fn app_with_replacement(
        same_model: bool,
        refs_count: u16,
        instance: bool,
        recyclable: bool,
        request: bool,
    ) -> (App, Handle<Mesh>, Option<Entity>) {
        let mut app = App::new();
        let mut loader = CFileLoader::default();
        let model = loader.model_info.add_simple_model(1000);
        model.base.rw_object_loaded = true;
        model.base.ref_count = refs_count;
        loader.model_info.add_simple_model(1001);
        let mut entity = super::super::world_render_candidate_tests::world_entity([0.0; 3], true);
        entity.entity_type = EEntityType::Building;
        entity.model_index = 1000;
        loader.world.entities.push(entity);
        loader.world.replace_entity_model_with_request(
            0,
            if same_model { 1000 } else { 1001 },
            Some(request),
        );
        let mut streaming = CStreaming::default();
        streaming.info_for_model[1000].load_state = StreamLoadState::Loaded;
        let mut meshes = Assets::<Mesh>::default();
        let mesh = meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        ));
        let asset_key = GtaWorldModelAssetKey {
            model_index: 1000,
            lod_index: 0,
            txd_generation: 0,
            txd_revision: 0,
        };
        let mut cache = GtaWorldModelAssetCache::default();
        cache.geometry.insert(
            (1000, 0),
            GtaWorldModelGeometry {
                meshes: vec![mesh.clone()],
                recipes: Vec::new(),
            },
        );
        let mut registry = GtaWorldMeshRegistry::default();
        let mut refs = GtaRuntimeEntityModelRefs::default();
        let drawable = instance.then(|| app.world_mut().spawn_empty().id());
        if let Some(drawable) = drawable {
            let slot = GtaWorldMeshSlot {
                bevy_entities: vec![drawable],
                fade_materials: Vec::new(),
                asset_key,
                model_index: 1000,
                pass: GtaWorldRenderPass::EverythingBarRoads,
                submesh_count: 1,
                is_visible: true,
                textured_count: 0,
                missing_texture_count: 0,
                txd_store_revision: 0,
                lod_multiplier: 1.0,
                lod_min_dist: 0.0,
                lod_max_dist: 100.0,
                render_transform: Transform::IDENTITY,
            };
            if recyclable {
                registry.retired_slots.insert(0, slot);
            } else {
                registry.slots.insert(0, slot);
                registry.set_slot_presence(0, true);
                refs.owned
                    .insert((GtaRuntimeModelRefSource::World, 0), 1000);
            }
        }
        app.insert_resource(loader)
            .insert_resource(streaming)
            .insert_resource(CGameState::default())
            .insert_resource(registry)
            .insert_resource(refs)
            .insert_resource(cache)
            .insert_resource(meshes)
            .init_resource::<Assets<GtaUnlitMaterial>>()
            .init_resource::<WorldModelReplacementDiagnostics>()
            .add_systems(Update, consume_world_model_replacements);
        (app, mesh, drawable)
    }

    #[test]
    fn replacement_retires_equal_changed_cold_and_recyclable_instances_but_preserves_shared_models()
    {
        for (same, refs, instance, recyclable, shared) in [
            (true, 1, true, false, false),
            (false, 1, true, false, false),
            (true, 2, true, false, true),
            (false, 2, true, false, true),
            (false, 0, false, false, false),
            (true, 0, true, true, false),
        ] {
            let (mut app, mesh, drawable) =
                app_with_replacement(same, refs, instance, recyclable, false);
            app.update();
            let world = app.world();
            if let Some(drawable) = drawable {
                assert!(world.get_entity(drawable).is_err());
            }
            let registry = world.resource::<GtaWorldMeshRegistry>();
            assert!(
                registry.slots.is_empty()
                    && registry.retired_slots.is_empty()
                    && !registry.has_slot(0)
            );
            assert!(
                world
                    .resource::<GtaRuntimeEntityModelRefs>()
                    .owned
                    .is_empty()
            );
            let source = world
                .resource::<CFileLoader>()
                .model_info
                .get_model_info(1000)
                .unwrap();
            assert_eq!(source.base().ref_count, if shared { 1 } else { 0 });
            assert_eq!(
                world.resource::<CStreaming>().info_for_model[1000].load_state,
                if shared {
                    StreamLoadState::Loaded
                } else {
                    StreamLoadState::NotLoaded
                }
            );
            assert_eq!(world.resource::<Assets<Mesh>>().contains(mesh.id()), shared);
            assert_eq!(
                world
                    .resource::<WorldModelReplacementDiagnostics>()
                    .consumed,
                1
            );
        }
    }

    #[test]
    fn same_model_big_building_request_survives_removal_and_pause() {
        let (mut app, mesh, _) = app_with_replacement(true, 1, true, false, true);
        let mut timer = CTimer::initialise();
        timer.start_user_pause();
        app.insert_resource(timer);
        app.update();
        assert!(app.world().resource::<CTimer>().get_is_paused());
        let streaming = app.world().resource::<CStreaming>();
        assert_eq!(
            streaming.info_for_model[1000].flags & STREAMFLAGS_DONT_REMOVE,
            STREAMFLAGS_DONT_REMOVE
        );
        assert_ne!(
            streaming.info_for_model[1000].load_state,
            StreamLoadState::NotLoaded
        );
        assert!(!app.world().resource::<Assets<Mesh>>().contains(mesh.id()));
        app.update();
        assert_eq!(
            app.world()
                .resource::<WorldModelReplacementDiagnostics>()
                .consumed,
            1
        );
    }
}
