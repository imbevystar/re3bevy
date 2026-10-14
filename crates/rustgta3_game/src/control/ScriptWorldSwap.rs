//! CTheScripts::AddToBuildingSwapArray: original identity and model provenance.
use super::{Script::CTheScripts, ScriptTypes::TBuildingSwap};
use crate::core::FileLoader::CFileLoader;
use rustgta3_core::core::Coordinates::CVector;

pub(crate) struct BuildingSwapRequest {
    pub position: CVector,
    pub radius: f32,
    pub old_model: i32,
    pub new_model: i32,
}

/// Script4.cpp SWAP_NEAREST_BUILDING_MODEL and Building.cpp replacement request.
/// The caller retains the effect until all canonical inputs exist.
pub(crate) fn apply_building_swap(
    scripts: &mut CTheScripts,
    loader: &mut CFileLoader,
    current_level: i32,
    request: BuildingSwapRequest,
) {
    let BuildingSwapRequest {
        position,
        radius,
        old_model,
        new_model,
    } = request;
    let mut position = [position.x, position.y, position.z];
    if position[2] <= super::ScriptTypes::MAP_Z_LOW_LIMIT {
        position[2] =
            loader
                .world
                .find_ground_z_for_coord(position[0], position[1], &loader.model_info);
    }
    let Some(entity_index) =
        loader
            .world
            .find_nearest_building_of_model(old_model.max(0) as usize, position, radius)
    else {
        bevy::log::warn_once!(
            old_model,
            ?position,
            radius,
            "scm_building_swap_target_missing"
        );
        return;
    };
    let Some(building) = loader.world.entities.get(entity_index) else {
        bevy::log::warn_once!(entity_index, "scm_building_swap_invalid_world_identity");
        return;
    };
    let request_replacement =
        building.is_big_building && (building.level == 0 || building.level == current_level);
    let new_model_index = new_model.max(0) as usize;
    if !loader.world.replace_entity_model_with_request(
        entity_index,
        new_model_index,
        Some(request_replacement),
    ) {
        bevy::log::warn_once!(entity_index, "scm_building_swap_replacement_failed");
        return;
    }
    // Area-query inputs are derived from the current model's original COL.
    scripts.static_area_world_entity_count = usize::MAX;
    if !scripts.add_to_building_swap_array(entity_index as i32, old_model, new_model) {
        bevy::log::warn_once!("scm_building_swap_record_capacity_exhausted");
    }
    // The world command retains native destruction -> zero-ref removal ->
    // optional new-model request order before any replacement draw submission.
}

impl CTheScripts {
    pub(crate) fn add_to_building_swap_array(
        &mut self,
        building_ref: i32,
        old_model: i32,
        new_model: i32,
    ) -> bool {
        if let Some(record) = self
            .building_swaps
            .iter_mut()
            .find(|record| record.building_ref == Some(building_ref))
        {
            if record.old_model == new_model {
                *record = TBuildingSwap::default();
            } else {
                record.new_model = new_model;
            }
            return true;
        }
        let Some(record) = self
            .building_swaps
            .iter_mut()
            .find(|record| record.building_ref.is_none())
        else {
            return false;
        };
        *record = TBuildingSwap {
            building_ref: Some(building_ref),
            old_model,
            new_model,
        };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{
        ScriptTypes::ScriptCommandEffect, ScriptWorldEffects::apply_script_effects_to_world,
    };
    use crate::core::Game::ScriptGameplayBridge;
    use crate::core::Streaming::CStreaming;
    use rustgta3_core::core::GameState::CGameState;
    use rustgta3_formats::formats::FileLoader::GtaIplInstance;
    use rustgta3_formats::formats::Script::ScmFile;

    #[test]
    fn replacement_waits_before_mutation_and_requests_only_native_big_building_levels() {
        let mut loader = CFileLoader::default();
        loader.model_info.add_simple_model(1000).is_big_building = true;
        loader.model_info.add_simple_model(1001);
        let building = loader
            .world
            .load_object_instance(
                &GtaIplInstance {
                    id: 1000,
                    name: "unit_building".into(),
                    position: [0.0; 3],
                    scale: [1.0; 3],
                    rotation_axis: [0.0, 0.0, 1.0],
                    angle: 1.0,
                },
                &loader.model_info,
                &Default::default(),
            )
            .unwrap();
        let game = CGameState::default();
        loader.world.entities[building].level = game.curr_level as i32;
        let mut game = game;
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        let mut bridge = ScriptGameplayBridge::default();
        let mut streaming = CStreaming::default();
        scripts
            .pending_effects
            .push(ScriptCommandEffect::SwapNearestBuildingModel {
                position: CVector::ZERO,
                radius: 10.0,
                old_model: 1000,
                new_model: 1001,
            });
        for have_game in [false, true] {
            apply_script_effects_to_world(
                &mut scripts,
                &mut bridge,
                have_game.then_some(&mut game),
                None,
                None,
                (!have_game).then_some(&mut streaming),
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&mut loader),
                0,
            );
            assert_eq!(loader.world.entities[building].model_index, 1000);
            assert_eq!(loader.world.model_swap_epoch, 0);
            assert_eq!(scripts.pending_effects.len(), 1);
            assert!(
                scripts
                    .building_swaps
                    .iter()
                    .all(|record| record.building_ref.is_none())
            );
        }
        apply_script_effects_to_world(
            &mut scripts,
            &mut bridge,
            Some(&mut game),
            None,
            None,
            Some(&mut streaming),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut loader),
            0,
        );
        assert!(scripts.pending_effects.is_empty());
        assert_eq!(loader.world.entities[building].model_index, 1001);
        let replacements = loader.world.take_model_replacements();
        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].request_big_building, Some(true));
        // Removal must precede the request, including same-ID replacements.
        assert_eq!(streaming.info_for_model[1001].flags, 0);

        loader.world.entities[building].level = 3;
        apply_building_swap(
            &mut scripts,
            &mut loader,
            1,
            BuildingSwapRequest {
                position: CVector::ZERO,
                radius: 10.0,
                old_model: 1001,
                new_model: 1000,
            },
        );
        assert_eq!(loader.world.entities[building].model_index, 1000);
        let replacements = loader.world.take_model_replacements();
        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].request_big_building, Some(false));
        assert!(
            scripts
                .building_swaps
                .iter()
                .all(|record| record.building_ref.is_none())
        );
    }

    #[test]
    fn restoration_frees_the_first_native_slot_without_losing_original_provenance() {
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        assert!(scripts.add_to_building_swap_array(3, 100, 101));
        assert!(scripts.add_to_building_swap_array(3, 101, 102));
        assert_eq!(scripts.building_swaps[0].old_model, 100);
        assert_eq!(scripts.building_swaps[0].new_model, 102);
        assert!(scripts.add_to_building_swap_array(7, 200, 201));
        assert!(scripts.add_to_building_swap_array(3, 102, 100));
        assert_eq!(scripts.building_swaps[0], TBuildingSwap::default());
        assert_eq!(scripts.building_swaps[1].building_ref, Some(7));
        assert!(scripts.add_to_building_swap_array(9, 300, 301));
        assert_eq!(scripts.building_swaps[0].building_ref, Some(9));
    }

    #[test]
    fn repeated_native_restore_cycles_do_not_exhaust_the_swap_array() {
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        for building in 0..100 {
            assert!(scripts.add_to_building_swap_array(building, 100, 101));
            assert!(scripts.add_to_building_swap_array(building, 101, 100));
        }
        assert!(
            scripts
                .building_swaps
                .iter()
                .all(|record| *record == TBuildingSwap::default())
        );
        for building in 0..scripts.building_swaps.len() as i32 {
            assert!(scripts.add_to_building_swap_array(building, 100, 101));
        }
        assert!(!scripts.add_to_building_swap_array(1000, 100, 101));
        assert_eq!(scripts.building_swaps[0].building_ref, Some(0));
    }
}
