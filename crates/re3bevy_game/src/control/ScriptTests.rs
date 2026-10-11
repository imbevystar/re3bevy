//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Script VM unit tests (verbatim move from control/Script.rs).

#![cfg(test)]
use crate::entities::EntityReference::EntityReference;
use super::Script::*;
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};
use re3bevy_formats::formats::Script::ScmFile;

use crate::control::Garages::EGarageState;
use crate::control::Pickups::PICKUP_MINE_INACTIVE;
use crate::core::Pools::CPoolHandle;
use crate::entities::Entity::eEntityStatus;
use crate::modelinfo::ModelIndices::{MI_CARMINE, MI_DODO, MI_NAUTICALMINE};
use crate::weapons::WeaponType::{WEAPONTYPE_TOTAL_INVENTORY_WEAPONS, eWeaponType};

const PROGRAM_IP: usize = 512;

#[test]
fn store_player_car_on_foot_consumes_output_without_changing_its_value() {
    for output_kind in [ARGUMENT_GLOBALVAR, ARGUMENT_LOCALVAR] {
        let mut program = Vec::new();
        push_opcode(&mut program, COMMAND_STORE_CAR_PLAYER_IS_IN);
        push_i16(&mut program, 0);
        program.push(output_kind);
        program.extend_from_slice(&8u16.to_le_bytes());
        let next_instruction = program.len();
        push_opcode(&mut program, COMMAND_WAIT);
        push_i16(&mut program, 0);
        let (mut scripts, mut script) = vm_with_program(&program);
        scripts.write_i32_at_script_space(8, 321).unwrap();
        script.local_variables[8] = 654;
        scripts.store_vehicle_index = 987;
        let mut player = |_| Some(player_snapshot(0));
        script.process_one_command(&mut scripts, 0, 1.0,
            None, None, None, None, None, Some(&mut player), None, None, None, None).unwrap();
        assert_eq!(script.ip as usize, PROGRAM_IP + next_instruction);
        assert_eq!(scripts.read_i32_at_script_space(8).unwrap(), 321);
        assert_eq!(script.local_variables[8], 654);
        assert_eq!(scripts.store_vehicle_index, 987);
        assert!(scripts.pending_effects.is_empty());
        let outcome = script.process_one_command(&mut scripts, 0, 1.0,
            None, None, None, None, None, None, None, None, None, None).unwrap();
        assert!(outcome.yielded);
        assert_eq!(outcome.command, COMMAND_WAIT);
    }
}

#[test]
fn clear_player_weapons_opcode_preserves_player_argument_and_advances() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_REMOVE_ALL_PLAYER_WEAPONS);
    push_i16(&mut program, 1);
    let (mut scripts, mut script) = vm_with_program(&program);
    script.process_one_command(&mut scripts, 0, 1.0,
        None, None, None, None, None, None, None, None, None, None).unwrap();
    assert_eq!(script.ip as usize, PROGRAM_IP + program.len());
    assert!(matches!(scripts.pending_effects.as_slice(),
        [ScriptCommandEffect::RemoveAllPlayerWeapons { player_index: 1 }]));
}

#[test]
fn end_game_tune_opcodes_decode_without_parameters_and_preserve_order() {
    let mut program = Vec::new();
    for command in [COMMAND_LOAD_END_OF_GAME_TUNE, COMMAND_PLAY_END_OF_GAME_TUNE, COMMAND_STOP_END_OF_GAME_TUNE] {
        push_opcode(&mut program, command);
    }
    let (mut scripts, mut script) = vm_with_program(&program);
    for _ in 0..3 {
        script.process_one_command(&mut scripts, 0, 1.0,
            None, None, None, None, None, None, None, None, None, None).unwrap();
    }
    assert_eq!(script.ip as usize, PROGRAM_IP + program.len());
    assert!(matches!(scripts.pending_effects.as_slice(), [ScriptCommandEffect::LoadEndOfGameTune,
        ScriptCommandEffect::PlayEndOfGameTune, ScriptCommandEffect::StopEndOfGameTune]));
}

struct TestMutationRuntime<'a> {
    callback: &'a mut dyn for<'b> FnMut(
        ScriptEntityMutation,
        Option<&'b mut CGeneral>,
    ) -> Option<ScriptEntityMutationResult>,
    supports: &'a dyn Fn(&ScriptEntityMutation) -> bool,
}

impl ScriptEntityRuntimeAccess for TestMutationRuntime<'_> {
    fn supports_mutation(&self, request: &ScriptEntityMutation) -> bool {
        (self.supports)(request)
    }

    fn mutate(
        &mut self,
        request: ScriptEntityMutation,
        general: Option<&mut CGeneral>,
    ) -> Option<ScriptEntityMutationResult> {
        (self.callback)(request, general)
    }

    fn char_snapshot(&mut self, _handle: i32) -> Option<ScriptEntitySnapshot> {
        None
    }

    fn vehicle_snapshot(&mut self, _handle: i32) -> Option<ScriptEntitySnapshot> {
        None
    }

    fn object_snapshot(&mut self, _handle: i32) -> Option<ScriptEntitySnapshot> {
        None
    }
}

fn vm_with_program(program: &[u8]) -> (CTheScripts, CRunningScript) {
    let mut scm = ScmFile::parse_main_scm(&[]);
    scm.script_space[PROGRAM_IP..PROGRAM_IP + program.len()].copy_from_slice(program);
    scm.loaded_main_bytes = PROGRAM_IP + program.len();
    let mut script = CRunningScript::new();
    script.set_ip(PROGRAM_IP as u32);
    (CTheScripts::from_scm(scm, program.to_vec()), script)
}

fn push_opcode(bytes: &mut Vec<u8>, opcode: i32) {
    bytes.extend_from_slice(&(opcode as u16).to_le_bytes());
}

fn push_i16(bytes: &mut Vec<u8>, value: i16) {
    bytes.push(ARGUMENT_INT16);
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_scm_float(bytes: &mut Vec<u8>, value: f32) {
    bytes.push(ARGUMENT_FLOAT);
    bytes.extend_from_slice(&((value * 16.0) as i16).to_le_bytes());
}

fn push_global_output(bytes: &mut Vec<u8>, offset: u16) {
    bytes.push(ARGUMENT_GLOBALVAR);
    bytes.extend_from_slice(&offset.to_le_bytes());
}

fn entity_snapshot(position: CVector) -> ScriptEntitySnapshot {
    ScriptEntitySnapshot {
        position,
        bound_center: position,
        bound_radius: 1.0,
        heading_degrees: 0.0,
        up_z: 1.0,
        model_index: 23,
        entity_status: eEntityStatus::STATUS_PHYSICS as i32,
        alive: true,
        current_vehicle: None,
        current_vehicle_model: None,
        vehicle_colour1: None,
        vehicle_colour2: None,
        vehicle_is_boat: false,
        vehicle_in_water: false,
        stopped: true,
        is_shooting: false,
        collision_records: [None; crate::entities::Physical::PHYSICAL_MAX_COLLISIONRECORDS],
        speed: 0.0,
        health: 100.0,
        current_weapon: 0,
        weapon_ammo: [0; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
        script_objective_completed: false,
        leader_slot: None,
        ped_type: None,
        vehicle_bomb_type: None,
        vehicle_damaged: false,
        last_weapon_damage: -1,
        vehicle_num_passengers: 0,
        vehicle_num_max_passengers: 0,
        vehicle_passenger_handles: [None; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS],
        vehicle_closed_doors: 0,
        automobile_drive_wheels_on_ground: None,
        can_see_player: false,
    }
}

fn player_snapshot(ped_handle: i32) -> ScriptPlayerSnapshot {
    ScriptPlayerSnapshot {
        ped_handle,
        position: CVector::ZERO,
        current_vehicle: None,
        current_vehicle_model: None,
        stopped: true,
        is_shooting: false,
        horn_pressed: false,
        is_in_control: true,
        is_driving: false,
        is_making_call: false,
        health: 100.0,
        wanted_level: 0,
        money: 0,
        current_weapon: 0,
        weapon_ammo: [0; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
    }
}

#[test]
fn set_char_threat_search_preserves_scm_bitmask() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_SET_CHAR_THREAT_SEARCH);
    push_i16(&mut program, 42);
    push_i16(&mut program, 0x1234);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut char_probe = |handle| (handle == 42).then(|| entity_snapshot(CVector::ZERO));

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut char_probe),
            None,
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::SetCharThreatSearch {
            handle: 42,
            threats: 0x1234,
        }]
    );
}

#[test]
fn has_model_loaded_reads_streaming_ids_above_default_model_table() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_HAS_MODEL_LOADED);
    push_i16(&mut program, 257);
    let (mut scripts, mut script) = vm_with_program(&program);
    scripts.model_loaded[257] = true;

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    assert!(
        script.cond_result,
        "SCM HAS_MODEL_LOADED must cover map-model ids such as Luigi's Club 257"
    );
}

#[test]
fn objective_passed_reads_live_ped_flag() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CHAR_OBJECTIVE_PASSED);
    push_i16(&mut program, 7);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut char_probe = |handle| {
        (handle == 7).then(|| {
            let mut snapshot = entity_snapshot(CVector::ZERO);
            snapshot.script_objective_completed = true;
            snapshot
        })
    };

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut char_probe),
            None,
            None,
            None,
        )
        .unwrap();

    assert!(script.cond_result);
}

#[test]
fn all_car_occupants_leave_command_requires_live_vehicle_and_emits_effect() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_SET_ALL_OCCUPANTS_OF_CAR_LEAVE_CAR);
    push_i16(&mut program, 77);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe = |handle| (handle == 77).then(|| entity_snapshot(CVector::ZERO));

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::SetAllOccupantsOfCarLeaveCar { handle: 77 }]
    );
}

#[test]
fn coordinate_blip_decodes_two_byte_scm_floats_and_stores_handle() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_ADD_BLIP_FOR_COORD);
    push_scm_float(&mut program, 12.5);
    push_scm_float(&mut program, -3.25);
    push_scm_float(&mut program, 8.0);
    push_global_output(&mut program, 100);
    let (mut scripts, mut script) = vm_with_program(&program);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    assert_eq!(scripts.read_i32_at_script_space(100).unwrap(), 1);
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::CreateCoordinateBlip {
            handle: 1,
            position: CVector::new(12.5, -3.25, 8.0),
            sprite: 0,
            contact_point: false,
            colour: 5,
            display: 3,
            scale: 3,
        }]
    );
}

#[test]
fn clear_onscreen_timer_uses_raw_global_variable_offset() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_CLEAR_ONSCREEN_TIMER);
    push_global_output(&mut program, 0x234);
    let (mut scripts, mut script) = vm_with_program(&program);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::ClearOnscreenClock {
            variable_offset: 0x234,
        }]
    );
}

#[test]
fn is_car_on_screen_passes_exact_model_sphere_to_camera() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CAR_ON_SCREEN);
    push_i16(&mut program, 77);
    let (mut scripts, mut script) = vm_with_program(&program);
    let expected_center = CVector::new(10.0, 20.0, 30.0);
    let mut vehicle_probe = |handle| {
        (handle == 77).then(|| {
            let mut snapshot = entity_snapshot(CVector::ZERO);
            snapshot.bound_center = expected_center;
            snapshot.bound_radius = 2.75;
            snapshot
        })
    };
    let mut observed = None;
    let mut camera_probe = |center, radius| {
        observed = Some((center, radius));
        true
    };

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            Some(&mut camera_probe),
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();

    assert!(script.cond_result);
    assert_eq!(observed, Some((expected_center, 2.75)));
}

#[test]
fn player_group_test_compares_live_pool_slot() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CHAR_IN_PLAYERS_GROUP);
    push_i16(&mut program, 12);
    push_i16(&mut program, 0);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut char_probe = |handle| {
        (handle == 12).then(|| {
            let mut snapshot = entity_snapshot(CVector::ZERO);
            snapshot.leader_slot = Some(5);
            snapshot
        })
    };
    let mut player_probe = |index| (index == 0).then(|| player_snapshot((5_i32 << 8) | 3));

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            Some(&mut player_probe),
            Some(&mut char_probe),
            None,
            None,
            None,
        )
        .unwrap();

    assert!(script.cond_result);
}

#[test]
fn closest_car_node_with_heading_uses_live_path_query() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_GET_CLOSEST_CAR_NODE_WITH_HEADING);
    push_scm_float(&mut program, 10.0);
    push_scm_float(&mut program, -20.0);
    push_scm_float(&mut program, 5.0);
    push_global_output(&mut program, 100);
    push_global_output(&mut program, 104);
    push_global_output(&mut program, 108);
    push_global_output(&mut program, 112);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut path_probe =
        |position: CVector, path_type: u8, ignore_disabled: bool, ignore_between_levels: bool| {
            assert_eq!(position, CVector::new(10.0, -20.0, 5.0));
            assert_eq!(path_type, crate::control::PathFind::PATH_CAR);
            assert!(ignore_disabled);
            assert!(ignore_between_levels);
            Some(ScriptPathNodeSnapshot {
                position: CVector::new(11.25, -18.5, 6.0),
                heading_degrees: 135.0,
            })
        };

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            Some(&mut path_probe),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        f32::from_bits(scripts.read_i32_at_script_space(100).unwrap() as u32),
        11.25
    );
    assert_eq!(
        f32::from_bits(scripts.read_i32_at_script_space(104).unwrap() as u32),
        -18.5
    );
    assert_eq!(
        f32::from_bits(scripts.read_i32_at_script_space(108).unwrap() as u32),
        6.0
    );
    assert_eq!(
        f32::from_bits(scripts.read_i32_at_script_space(112).unwrap() as u32),
        135.0
    );
}

#[test]
fn accurate_car_goto_preserves_reference_mission_inputs() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_CAR_GOTO_COORDINATES_ACCURATE);
    push_i16(&mut program, 77);
    push_scm_float(&mut program, 100.0);
    push_scm_float(&mut program, 200.0);
    push_scm_float(&mut program, -101.0);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe =
        |handle| (handle == 77).then(|| entity_snapshot(CVector::new(1.0, 2.0, 3.0)));

    script
        .process_one_command(
            &mut scripts,
            4321,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::CarGotoCoordinates {
            handle: 77,
            position: CVector::new(100.0, 200.0, -101.0),
            needs_ground: true,
            accurate: true,
            anti_reverse_timer_ms: 4321,
        }]
    );
}

#[test]
fn slide_object_uses_fix_bugs_time_step_and_synchronous_result() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_SLIDE_OBJECT);
    push_i16(&mut program, 9);
    push_scm_float(&mut program, 10.0);
    push_scm_float(&mut program, 20.0);
    push_scm_float(&mut program, 30.0);
    push_scm_float(&mut program, 0.5);
    push_scm_float(&mut program, 1.0);
    push_scm_float(&mut program, 1.5);
    push_i16(&mut program, 1);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut observed = None;
    let mut entity_mutation_callback =
        |request: ScriptEntityMutation, _general: Option<&mut CGeneral>| match request {
            ScriptEntityMutation::ObjectMotion(request) => {
                observed = Some(request);
                Some(ScriptEntityMutationResult::ObjectMotion(true))
            }
            _ => None,
        };
    let supports_entity_mutation =
        |request: &ScriptEntityMutation| matches!(request, ScriptEntityMutation::ObjectMotion(_));
    let mut entity_runtime = TestMutationRuntime {
        callback: &mut entity_mutation_callback,
        supports: &supports_entity_mutation,
    };

    script
        .process_one_command_with_3d_ground_runtime(
            &mut scripts,
            0,
            re3bevy_core::core::Timer::CTimer::DEFAULT_TIME_STEP * 2.0,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut entity_runtime),
            None,
            None,
            None,
        )
        .unwrap();

    assert!(script.cond_result);
    assert_eq!(
        observed,
        Some(ScriptObjectMotionRequest::Slide {
            handle: 9,
            target: CVector::new(10.0, 20.0, 30.0),
            slide_by: CVector::new(1.0, 2.0, 3.0),
            check_collision: true,
        })
    );
}

#[test]
fn mission_garage_target_and_predicate_share_synchronous_state() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_SET_TARGET_CAR_FOR_MISSION_GARAGE);
    push_i16(&mut program, 1);
    push_i16(&mut program, 77);
    push_opcode(&mut program, COMMAND_IS_CAR_IN_MISSION_GARAGE);
    push_i16(&mut program, 1);
    let (mut scripts, mut script) = vm_with_program(&program);
    scripts.garage_states[1] = EGarageState::ClosedContainsCar as u8;
    let mut vehicle_probe = |handle| (handle == 77).then(|| entity_snapshot(CVector::ZERO));

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert_eq!(scripts.garage_states[1], EGarageState::FullyClosed as u8);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert!(!script.cond_result);
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::SetTargetCarForMissionGarage {
            handle: 1,
            vehicle_handle: Some(77),
        }]
    );
}

#[test]
fn garage_state_predicates_match_reference_state_sets() {
    for state in [EGarageState::Opened, EGarageState::OpenedContainsCar] {
        let mut program = Vec::new();
        push_opcode(&mut program, COMMAND_IS_GARAGE_OPEN);
        push_i16(&mut program, 1);
        let (mut scripts, mut script) = vm_with_program(&program);
        scripts.garage_states[1] = state as u8;
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        assert!(script.cond_result, "{state:?} must count as open");
    }

    for (state, expected) in [
        (EGarageState::FullyClosed, true),
        (EGarageState::ClosedContainsCar, false),
    ] {
        let mut program = Vec::new();
        push_opcode(&mut program, COMMAND_IS_GARAGE_CLOSED);
        push_i16(&mut program, 1);
        let (mut scripts, mut script) = vm_with_program(&program);
        scripts.garage_states[1] = state as u8;
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(script.cond_result, expected, "state was {state:?}");
    }
}

#[test]
fn respray_predicate_consumes_flag_and_containment_requires_exact_pair() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_HAS_RESPRAY_HAPPENED);
    push_i16(&mut program, 2);
    push_opcode(&mut program, COMMAND_HAS_RESPRAY_HAPPENED);
    push_i16(&mut program, 2);
    let (mut scripts, mut script) = vm_with_program(&program);
    scripts.garage_respray_happened[2] = true;
    for expected in [true, false] {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(script.cond_result, expected);
    }

    let mut containment_program = Vec::new();
    push_opcode(&mut containment_program, COMMAND_DOES_GARAGE_CONTAIN_CAR);
    push_i16(&mut containment_program, 3);
    push_i16(&mut containment_program, 77);
    let (mut scripts, mut script) = vm_with_program(&containment_program);
    scripts
        .garage_vehicle_containment
        .push(ScriptGarageVehicleContainment {
            garage_handle: 3,
            vehicle_handle: 77,
        });
    scripts
        .garage_vehicle_containment
        .push(ScriptGarageVehicleContainment {
            garage_handle: 4,
            vehicle_handle: 88,
        });
    let mut vehicle_probe = |handle| (handle == 77).then(|| entity_snapshot(CVector::ZERO));
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn is_car_crushed_compares_the_raw_pool_handle() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CAR_CRUSHED);
    push_i16(&mut program, 77);
    let (mut scripts, mut script) = vm_with_program(&program);
    scripts.crushed_car_id = 77;
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn car_bomb_and_visible_damage_predicates_use_live_vehicle_fields() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CAR_ARMED_WITH_BOMB);
    push_i16(&mut program, 42);
    push_i16(&mut program, 3);
    push_opcode(&mut program, COMMAND_IS_CAR_VISIBLY_DAMAGED);
    push_i16(&mut program, 42);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut snapshot = entity_snapshot(CVector::ZERO);
    snapshot.vehicle_bomb_type = Some(3);
    snapshot.vehicle_damaged = true;
    let mut vehicle_probe = |handle| (handle == 42).then_some(snapshot);

    for _ in 0..2 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&mut vehicle_probe),
                None,
                None,
            )
            .unwrap();
        assert!(script.cond_result);
    }
}

#[test]
fn passenger_seat_and_door_predicates_use_live_vehicle_layout() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CAR_PASSENGER_SEAT_FREE);
    push_i16(&mut program, 42);
    push_i16(&mut program, 0);
    push_opcode(&mut program, COMMAND_GET_CHAR_IN_CAR_PASSENGER_SEAT);
    push_i16(&mut program, 42);
    push_i16(&mut program, 1);
    push_global_output(&mut program, 100);
    push_opcode(&mut program, COMMAND_IS_CAR_DOOR_CLOSED);
    push_i16(&mut program, 42);
    push_i16(&mut program, 2);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut snapshot = entity_snapshot(CVector::ZERO);
    snapshot.vehicle_num_max_passengers = 4;
    snapshot.vehicle_passenger_handles[1] = Some(123);
    snapshot.vehicle_closed_doors = 1 << 2;
    let mut vehicle_probe = |handle| (handle == 42).then_some(snapshot);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert_eq!(scripts.read_i32_at_script_space(100).unwrap(), 123);
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn vehicle_visual_bomb_and_particle_commands_preserve_cpp_arguments() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_SET_TAXI_LIGHTS);
    push_i16(&mut program, 42);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_SET_ALL_TAXI_LIGHTS);
    push_i16(&mut program, 0);
    push_opcode(&mut program, COMMAND_FLASH_CAR);
    push_i16(&mut program, 42);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_FLASH_CHAR);
    push_i16(&mut program, 43);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_FLASH_OBJECT);
    push_i16(&mut program, 44);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_ARM_CAR_WITH_BOMB);
    push_i16(&mut program, 42);
    push_i16(&mut program, 3);
    push_opcode(&mut program, COMMAND_CREATE_SINGLE_PARTICLE);
    push_i16(&mut program, 18);
    for value in [1.0, 2.0, 3.0, 0.25, 0.5, 0.75, 1.5] {
        push_scm_float(&mut program, value);
    }
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe = |handle| (handle == 42).then(|| entity_snapshot(CVector::ZERO));
    let mut char_probe = |handle| (handle == 43).then(|| entity_snapshot(CVector::ZERO));
    let mut object_probe = |handle| (handle == 44).then(|| entity_snapshot(CVector::ZERO));

    for _ in 0..7 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&mut char_probe),
                Some(&mut vehicle_probe),
                Some(&mut object_probe),
                None,
            )
            .unwrap();
    }
    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::SetTaxiLights {
                handle: 42,
                enabled: true,
            },
            ScriptCommandEffect::SetAllTaxiLights(false),
            ScriptCommandEffect::FlashCar {
                handle: 42,
                enabled: true,
            },
            ScriptCommandEffect::FlashChar {
                handle: 43,
                enabled: true,
            },
            ScriptCommandEffect::FlashObject {
                handle: 44,
                enabled: true,
            },
            ScriptCommandEffect::ArmCarWithBomb {
                handle: 42,
                bomb_type: 3,
            },
            ScriptCommandEffect::CreateSingleParticle {
                particle_type: 18,
                position: CVector::new(1.0, 2.0, 3.0),
                direction: CVector::new(0.25, 0.5, 0.75),
                size: 1.5,
            },
        ]
    );
}

#[test]
fn mission_entity_cube_uses_origin_sphere_and_normalises_negative_extents() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_POINT_OBSCURED_BY_A_MISSION_ENTITY);
    push_scm_float(&mut program, 10.0);
    push_scm_float(&mut program, 20.0);
    push_scm_float(&mut program, 30.0);
    push_scm_float(&mut program, -2.0);
    push_scm_float(&mut program, -3.0);
    push_scm_float(&mut program, -4.0);
    let (mut scripts, mut script) = vm_with_program(&program);
    scripts
        .mission_entity_bounds
        .push(ScriptMissionEntityBounds {
            position: CVector::new(12.5, 20.0, 30.0),
            radius: 0.5,
        });

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn pager_message_with_number_preserves_cpp_argument_mapping() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_ADD_PAGER_MESSAGE_WITH_NUMBER);
    program.extend_from_slice(b"PAGER01\0");
    push_i16(&mut program, 123);
    push_i16(&mut program, 40);
    push_i16(&mut program, 2);
    push_i16(&mut program, 9);
    let (mut scripts, mut script) = vm_with_program(&program);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::AddPagerMessage {
            key: *b"PAGER01\0",
            speed: 40,
            priority: 2,
            unused: 9,
            numbers: Some([123, -1, -1, -1, -1, -1]),
        }]
    );
}

#[test]
fn import_garage_slot_and_force_random_ped_type_match_script4() {
    let mut import_program = Vec::new();
    push_opcode(
        &mut import_program,
        COMMAND_HAS_IMPORT_GARAGE_SLOT_BEEN_FILLED,
    );
    push_i16(&mut import_program, 2);
    push_i16(&mut import_program, 5);
    let (mut scripts, mut script) = vm_with_program(&import_program);
    scripts.garage_collection_masks[2] = 1 << 4;
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);

    let mut force_program = Vec::new();
    push_opcode(&mut force_program, COMMAND_FORCE_RANDOM_PED_TYPE);
    push_i16(&mut force_program, 12);
    let (mut scripts, mut script) = vm_with_program(&force_program);
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::ForceRandomPedType(12)]
    );
}

#[test]
fn clear_area_of_cars_normalises_all_three_axes() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_CLEAR_AREA_OF_CARS);
    push_scm_float(&mut program, 10.0);
    push_scm_float(&mut program, -2.0);
    push_scm_float(&mut program, 8.0);
    push_scm_float(&mut program, -4.0);
    push_scm_float(&mut program, 6.0);
    push_scm_float(&mut program, 1.0);
    let (mut scripts, mut script) = vm_with_program(&program);
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::ClearAreaOfCars {
            min: CVector::new(-4.0, -2.0, 1.0),
            max: CVector::new(10.0, 6.0, 8.0),
        }]
    );
}

#[test]
fn random_car_for_car_park_uses_loaded_whitelist_and_thirty_car_cap() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_CREATE_RANDOM_CAR_FOR_CAR_PARK);
    push_scm_float(&mut program, 1.0);
    push_scm_float(&mut program, 2.0);
    push_scm_float(&mut program, 3.0);
    push_scm_float(&mut program, 90.0);
    let (mut scripts, mut script) = vm_with_program(&program);
    scripts.loaded_vehicle_models.fill(91);
    scripts.num_random_cars = 29;
    let mut general = CGeneral::with_seed(1);
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            Some(&mut general),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(scripts.num_random_cars, 30);
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::CreateRandomCarForCarPark {
            model_id: 91,
            position: CVector::new(1.0, 2.0, 3.0),
            heading_degrees: 90.0,
        }]
    );

    let (mut capped_scripts, mut capped_script) = vm_with_program(&program);
    capped_scripts.loaded_vehicle_models.fill(91);
    capped_scripts.num_random_cars = 30;
    capped_script
        .process_one_command(
            &mut capped_scripts,
            0,
            1.0,
            Some(&mut general),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(capped_scripts.pending_effects.is_empty());
}

#[test]
fn car_in_air_uses_collision_record_count_and_usj_dodo_guard() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CAR_IN_AIR_PROPER);
    push_i16(&mut program, 7);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe = |handle| {
        (handle == 7).then(|| {
            let mut snapshot = entity_snapshot(CVector::ZERO);
            snapshot.current_vehicle_model = Some(91);
            snapshot
        })
    };
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);

    let (mut scripts, mut script) = vm_with_program(&program);
    script.script_name = [0; 8];
    script.script_name[..3].copy_from_slice(b"usj");
    let mut dodo_probe = |handle| {
        (handle == 7).then(|| {
            let mut snapshot = entity_snapshot(CVector::ZERO);
            snapshot.current_vehicle_model = Some(MI_DODO as i32);
            snapshot
        })
    };
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut dodo_probe),
            None,
            None,
        )
        .unwrap();
    assert!(!script.cond_result);
}

#[test]
fn camera_wanted_and_particle_area_commands_preserve_cpp_arguments() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_ENABLE_PLAYER_CONTROL_CAMERA);
    push_opcode(&mut program, COMMAND_SET_WANTED_MULTIPLIER);
    push_scm_float(&mut program, 0.5);
    push_opcode(&mut program, COMMAND_SHAKE_CAM);
    push_i16(&mut program, 600);
    push_opcode(&mut program, COMMAND_SHAKE_CAM_WITH_POINT);
    push_i16(&mut program, 500);
    for value in [10.0, -20.0, 30.0] {
        push_scm_float(&mut program, value);
    }
    push_opcode(&mut program, COMMAND_REMOVE_PARTICLE_EFFECTS_IN_AREA);
    for value in [10.0, -2.0, 8.0, -4.0, 6.0, 1.0] {
        push_scm_float(&mut program, value);
    }
    let (mut scripts, mut script) = vm_with_program(&program);
    for _ in 0..5 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
    }
    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::EnablePlayerControlCamera,
            ScriptCommandEffect::SetWantedMultiplier(0.5),
            ScriptCommandEffect::ShakeCamera {
                strength: 0.6,
                position: None,
            },
            ScriptCommandEffect::ShakeCamera {
                strength: 0.5,
                position: Some(CVector::new(10.0, -20.0, 30.0)),
            },
            ScriptCommandEffect::RemoveParticleEffectsInArea {
                min: CVector::new(10.0, -2.0, 8.0),
                max: CVector::new(-4.0, 6.0, 1.0),
            },
        ]
    );
}

#[test]
fn drug_plane_commands_share_synchronous_runtime_snapshots() {
    let mut start_program = Vec::new();
    push_opcode(&mut start_program, COMMAND_START_DRUG_RUN);
    let (mut scripts, mut script) = vm_with_program(&start_program);
    scripts.drug_run_completed = true;
    scripts.drug_plane_shot_down = true;
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(!scripts.drug_run_completed);
    assert!(!scripts.drug_plane_shot_down);
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::StartDrugRun]
    );

    let mut find_program = Vec::new();
    push_opcode(&mut find_program, COMMAND_FIND_DRUG_PLANE_COORDINATES);
    push_global_output(&mut find_program, 100);
    push_global_output(&mut find_program, 104);
    push_global_output(&mut find_program, 108);
    let (mut scripts, mut script) = vm_with_program(&find_program);
    scripts.drug_plane_position = Some(CVector::new(12.5, -3.25, 40.0));
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        f32::from_bits(scripts.read_i32_at_script_space(100).unwrap() as u32),
        12.5
    );
    assert_eq!(
        f32::from_bits(scripts.read_i32_at_script_space(104).unwrap() as u32),
        -3.25
    );
    assert_eq!(
        f32::from_bits(scripts.read_i32_at_script_space(108).unwrap() as u32),
        40.0
    );
}

#[test]
fn catalina_heli_commands_use_runtime_handle_state_and_exact_effects() {
    let mut start_program = Vec::new();
    push_opcode(&mut start_program, COMMAND_START_CATALINA_HELI);
    let (mut scripts, mut script) = vm_with_program(&start_program);
    scripts.catalina_heli_shot_down = true;
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(!scripts.catalina_heli_shot_down);
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::StartCatalinaHeli]
    );

    let mut grab_program = Vec::new();
    push_opcode(&mut grab_program, COMMAND_GRAB_CATALINA_HELI);
    push_global_output(&mut grab_program, 100);
    let (mut scripts, mut script) = vm_with_program(&grab_program);
    scripts.catalina_heli_handle = 0x1234;
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(scripts.read_i32_at_script_space(100).unwrap(), 0x1234);

    let mut state_program = Vec::new();
    push_opcode(&mut state_program, COMMAND_HAS_CATALINA_HELI_BEEN_SHOT_DOWN);
    let (mut scripts, mut script) = vm_with_program(&state_program);
    scripts.catalina_heli_shot_down = true;
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn pacman_commands_preserve_cpp_arguments_and_synchronous_counters() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_START_PACMAN_RACE);
    push_i16(&mut program, 0);
    push_opcode(&mut program, COMMAND_GET_NUMBER_OF_POWER_PILLS_EATEN);
    push_global_output(&mut program, 100);
    push_opcode(&mut program, COMMAND_CLEAR_NUMBER_OF_POWER_PILLS_CARRIED);
    push_opcode(&mut program, COMMAND_GET_NUMBER_OF_POWER_PILLS_CARRIED);
    push_global_output(&mut program, 104);
    push_opcode(&mut program, COMMAND_CLEAR_NUMBER_OF_POWER_PILLS_EATEN);
    let (mut scripts, mut script) = vm_with_program(&program);
    scripts.pacman_pills_eaten = 17;
    scripts.pacman_pills_carried = 4;
    for _ in 0..5 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
    }
    assert_eq!(scripts.read_i32_at_script_space(100).unwrap(), 0);
    assert_eq!(scripts.read_i32_at_script_space(104).unwrap(), 0);
    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::StartPacManRace(0),
            ScriptCommandEffect::ResetPacManPillsCarried,
            ScriptCommandEffect::ResetPacManPillsEaten,
        ]
    );

    let mut placement_program = Vec::new();
    push_opcode(&mut placement_program, COMMAND_START_PACMAN_SCRAMBLE);
    push_scm_float(&mut placement_program, 10.0);
    push_scm_float(&mut placement_program, 20.0);
    push_scm_float(&mut placement_program, -100.0);
    push_scm_float(&mut placement_program, 7.5);
    push_i16(&mut placement_program, 12);
    push_opcode(&mut placement_program, COMMAND_ADD_POWER_PILL);
    push_scm_float(&mut placement_program, -5.0);
    push_scm_float(&mut placement_program, 6.0);
    push_scm_float(&mut placement_program, -100.0);
    let (mut scripts, mut script) = vm_with_program(&placement_program);
    let mut ground = |_x: f32, _y: f32| 31.25;
    for _ in 0..2 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                Some(&mut ground),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
    }
    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::StartPacManScramble {
                position: CVector::new(10.0, 20.0, 31.25),
                scramble_mult: 7.5,
                count: 12,
            },
            ScriptCommandEffect::AddPowerPill(CVector::new(-5.0, 6.0, 31.25)),
        ]
    );
}

#[test]
fn police_projectile_and_explosion_commands_use_live_cpp_snapshots() {
    let mut police_program = Vec::new();
    for (command, required) in [
        (COMMAND_SET_SWAT_REQUIRED, 1),
        (COMMAND_SET_FBI_REQUIRED, 0),
        (COMMAND_SET_ARMY_REQUIRED, 1),
    ] {
        push_opcode(&mut police_program, command);
        push_i16(&mut police_program, required);
    }
    let (mut scripts, mut script) = vm_with_program(&police_program);
    for _ in 0..3 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
    }
    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::SetSpecialPoliceRequired {
                kind: 0,
                required: true,
            },
            ScriptCommandEffect::SetSpecialPoliceRequired {
                kind: 1,
                required: false,
            },
            ScriptCommandEffect::SetSpecialPoliceRequired {
                kind: 2,
                required: true,
            },
        ]
    );

    let mut projectile_program = Vec::new();
    push_opcode(&mut projectile_program, COMMAND_DESTROY_PROJECTILES_IN_AREA);
    for value in [5.0, 6.0, 7.0, -5.0, -6.0, -7.0] {
        push_scm_float(&mut projectile_program, value);
    }
    push_opcode(&mut projectile_program, COMMAND_IS_PROJECTILE_IN_AREA);
    for value in [-5.0, -6.0, -7.0, 5.0, 6.0, 7.0] {
        push_scm_float(&mut projectile_program, value);
    }
    let (mut scripts, mut script) = vm_with_program(&projectile_program);
    scripts.projectile_snapshots = vec![
        (
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER as u8,
            CVector::new(1.0, 2.0, 3.0),
        ),
        (
            eWeaponType::WEAPONTYPE_FLAMETHROWER as u8,
            CVector::new(1.0, 2.0, 3.0),
        ),
    ];
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
    assert_eq!(
        scripts.projectile_snapshots,
        vec![(
            eWeaponType::WEAPONTYPE_FLAMETHROWER as u8,
            CVector::new(1.0, 2.0, 3.0),
        )]
    );
    assert_eq!(
        scripts.pending_effects,
        vec![ScriptCommandEffect::DestroyProjectilesInArea {
            min: CVector::new(-5.0, -6.0, -7.0),
            max: CVector::new(5.0, 6.0, 7.0),
        }]
    );
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(!script.cond_result);

    let mut explosion_program = Vec::new();
    push_opcode(&mut explosion_program, COMMAND_IS_EXPLOSION_IN_AREA);
    push_i16(&mut explosion_program, 2);
    for value in [10.0, 10.0, 10.0, -10.0, -10.0, -10.0] {
        push_scm_float(&mut explosion_program, value);
    }
    let (mut scripts, mut script) = vm_with_program(&explosion_program);
    scripts.explosion_snapshots = vec![(2, CVector::new(1.0, 2.0, 3.0))];
    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn vehicle_status_level_ignore_and_craig_strength_commands_are_typed_effects() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_SET_CAR_STATUS);
    push_i16(&mut program, 7);
    push_i16(&mut program, 5);
    push_opcode(&mut program, COMMAND_SET_CAR_IGNORE_LEVEL_TRANSITIONS);
    push_i16(&mut program, 7);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_MAKE_CRAIGS_CAR_A_BIT_STRONGER);
    push_i16(&mut program, 7);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_SET_CHAR_IGNORE_LEVEL_TRANSITIONS);
    push_i16(&mut program, 8);
    push_i16(&mut program, 0);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe = |handle| (handle == 7).then(|| entity_snapshot(CVector::ZERO));
    let mut char_probe = |handle| (handle == 8).then(|| entity_snapshot(CVector::ZERO));
    for _ in 0..4 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&mut char_probe),
                Some(&mut vehicle_probe),
                None,
                None,
            )
            .unwrap();
    }
    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::SetCarStatus {
                handle: 7,
                status: 5,
            },
            ScriptCommandEffect::SetCarIgnoreLevelTransitions {
                handle: 7,
                enabled: true,
            },
            ScriptCommandEffect::SetCarMoreResistantToDamage {
                handle: 7,
                enabled: true,
            },
            ScriptCommandEffect::SetCharIgnoreLevelTransitions {
                handle: 8,
                enabled: false,
            },
        ]
    );
}

#[test]
fn mine_and_sprite_pickup_blip_commands_preserve_cpp_contracts() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_DROP_MINE);
    for value in [10.0, 20.0, -100.0] {
        push_scm_float(&mut program, value);
    }
    push_opcode(&mut program, COMMAND_DROP_NAUTICAL_MINE);
    for value in [-4.0, 5.0, 6.0] {
        push_scm_float(&mut program, value);
    }
    push_opcode(&mut program, COMMAND_ADD_SPRITE_BLIP_FOR_PICKUP);
    push_i16(&mut program, 77);
    push_i16(&mut program, 20);
    push_global_output(&mut program, 8);

    let (mut scripts, mut script) = vm_with_program(&program);
    let mut ground = |_x: f32, _y: f32| 30.75;
    for _ in 0..3 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                Some(&mut ground),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
    }

    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::CreateUntrackedPickup {
                model_id: MI_CARMINE as i32,
                pickup_type: i32::from(PICKUP_MINE_INACTIVE),
                quantity: 0,
                position: CVector::new(10.0, 20.0, 31.25),
            },
            ScriptCommandEffect::CreateUntrackedPickup {
                model_id: MI_NAUTICALMINE as i32,
                // Script3.cpp intentionally does not use
                // PICKUP_NAUTICAL_MINE_INACTIVE here.
                pickup_type: i32::from(PICKUP_MINE_INACTIVE),
                quantity: 0,
                position: CVector::new(-4.0, 5.0, 6.0),
            },
            ScriptCommandEffect::CreateSpritePickupBlip {
                handle: 1,
                pickup_handle: 77,
                sprite: 20,
            },
        ]
    );
    assert_eq!(scripts.read_i32_at_script_space(8).unwrap(), 1);
}

#[test]
fn is_char_model_reads_the_live_ped_entity_model() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CHAR_MODEL);
    push_i16(&mut program, 42);
    push_i16(&mut program, 23);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut char_probe = |handle| (handle == 42).then(|| entity_snapshot(CVector::ZERO));

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut char_probe),
            None,
            None,
            None,
        )
        .unwrap();

    assert!(script.cond_result);
}

#[test]
fn dead_car_area_commands_require_wrecked_status_and_use_placeable_bounds() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_CAR_DEAD_IN_AREA_2D);
    push_i16(&mut program, 7);
    for value in [5.0, 5.0, -5.0, -5.0] {
        push_scm_float(&mut program, value);
    }
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_IS_CAR_DEAD_IN_AREA_3D);
    push_i16(&mut program, 7);
    for value in [5.0, 5.0, 5.0, -5.0, -5.0, -5.0] {
        push_scm_float(&mut program, value);
    }
    push_i16(&mut program, 1);

    let (mut scripts, mut script) = vm_with_program(&program);
    scripts.dbg_flag = true;
    let mut vehicle_probe = |handle| {
        (handle == 7).then(|| {
            let mut snapshot = entity_snapshot(CVector::ZERO);
            snapshot.entity_status = eEntityStatus::STATUS_WRECKED as i32;
            snapshot
        })
    };
    for _ in 0..2 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&mut vehicle_probe),
                None,
                None,
            )
            .unwrap();
        assert!(script.cond_result);
    }

    assert!(matches!(
        scripts.pending_effects[0],
        ScriptCommandEffect::HighlightArea {
            x1: 5.0,
            y1: 5.0,
            x2: -5.0,
            y2: -5.0,
            z: MAP_Z_LOW_LIMIT,
            ..
        }
    ));
    assert_eq!(
        scripts.pending_effects[1],
        ScriptCommandEffect::DebugSquare {
            x1: 5.0,
            y1: 5.0,
            x2: -5.0,
            y2: -5.0,
        }
    );
    assert!(matches!(
        scripts.pending_effects[2],
        ScriptCommandEffect::HighlightArea { z: 0.0, .. }
    ));
    assert!(matches!(
        scripts.pending_effects[3],
        ScriptCommandEffect::DebugCube {
            x1: 5.0,
            y1: 5.0,
            z1: 5.0,
            x2: -5.0,
            y2: -5.0,
            z2: -5.0,
        }
    ));
}

#[test]
fn save_menu_and_military_crane_commands_follow_re3_state_reads() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_ACTIVATE_SAVE_MENU);
    push_opcode(&mut program, COMMAND_HAS_SAVE_GAME_FINISHED);
    push_opcode(&mut program, COMMAND_HAS_MILITARY_CRANE_COLLECTED_ALL_CARS);
    let (mut scripts, mut script) = vm_with_program(&program);

    // re3 Script4.cpp COMMAND_ACTIVATE_SAVE_MENU only raises the frontend
    // flag through the effect bridge.
    script
        .process_one_command_with_3d_ground(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(matches!(
        scripts.pending_effects[0],
        ScriptCommandEffect::ActivateSaveMenu
    ));

    let state = std::cell::Cell::new(ScriptWorldStateSnapshot {
        menu_active: true,
        military_crane_all_cars_collected: false,
    });
    let mut world_probe = || state.get();
    // Menu still open: re3 Script4.cpp UpdateCompareFlag(!m_bMenuActive).
    script
        .process_one_command_with_3d_ground(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut world_probe),
        )
        .unwrap();
    assert!(!script.cond_result);

    state.set(ScriptWorldStateSnapshot {
        menu_active: false,
        military_crane_all_cars_collected: true,
    });
    // Save finished and all military-crane cars collected.
    script
        .process_one_command_with_3d_ground(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut world_probe),
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn world_state_commands_error_without_probe() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_HAS_SAVE_GAME_FINISHED);
    let (mut scripts, mut script) = vm_with_program(&program);
    let error = script
        .process_one_command_with_3d_ground(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
    assert!(matches!(error, ScriptVmError::MissingWorldStateProbe));
}

#[test]
fn upside_down_car_check_commands_share_the_cpp_timer_table() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_ADD_UPSIDEDOWN_CAR_CHECK);
    push_i16(&mut program, 7);
    push_opcode(&mut program, COMMAND_IS_CAR_STUCK_ON_ROOF);
    push_i16(&mut program, 7);
    push_opcode(&mut program, COMMAND_REMOVE_UPSIDEDOWN_CAR_CHECK);
    push_i16(&mut program, 7);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe = |handle| (handle == 7).then(|| entity_snapshot(CVector::ZERO));

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    let slot = scripts
        .upside_down_cars
        .cars
        .iter_mut()
        .find(|car| car.vehicle_index == 7)
        .unwrap();
    slot.upside_down_timer = UPSIDEDOWN_TIMER_THRESHOLD + 1;

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert!(script.cond_result);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut vehicle_probe),
            None,
            None,
        )
        .unwrap();
    assert!(
        scripts
            .upside_down_cars
            .cars
            .iter()
            .all(|car| car.vehicle_index != 7)
    );
}

#[test]
fn generic_crane_and_heavy_car_commands_preserve_cpp_arguments() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_ACTIVATE_CRANE);
    for value in [100.0, 200.0, 5.0, 6.0, -5.0, -6.0, 7.0, 8.0, 9.0, 90.0] {
        push_scm_float(&mut program, value);
    }
    push_opcode(&mut program, COMMAND_DEACTIVATE_CRANE);
    push_scm_float(&mut program, 100.0);
    push_scm_float(&mut program, 200.0);
    push_opcode(&mut program, COMMAND_SET_CAR_HEAVY);
    push_i16(&mut program, 7);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_SHAKE_PAD);
    push_i16(&mut program, 0);
    push_i16(&mut program, 300);
    push_i16(&mut program, 128);
    push_opcode(&mut program, COMMAND_PRINT_BIG_Q);
    program.extend_from_slice(b"TEST\0\0\0\0");
    push_i16(&mut program, 2_000);
    push_i16(&mut program, 2);
    push_opcode(&mut program, COMMAND_SWITCH_HELICOPTER);
    push_i16(&mut program, 1);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe = |handle| (handle == 7).then(|| entity_snapshot(CVector::ZERO));

    for _ in 0..6 {
        script
            .process_one_command(
                &mut scripts,
                0,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&mut vehicle_probe),
                None,
                None,
            )
            .unwrap();
    }

    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::ActivateCrane {
                pickup_min: CVector::new(-5.0, -6.0, 0.0),
                pickup_max: CVector::new(5.0, 6.0, 0.0),
                dropoff: CVector::new(7.0, 8.0, 9.0),
                heading_radians: std::f32::consts::FRAC_PI_2,
                crane_position: CVector::new(100.0, 200.0, 0.0),
                is_crusher: false,
                is_military: false,
            },
            ScriptCommandEffect::DeactivateCrane {
                position: CVector::new(100.0, 200.0, 0.0),
            },
            ScriptCommandEffect::SetCarHeavy {
                handle: 7,
                enabled: true,
            },
            ScriptCommandEffect::ShakePad {
                pad: 0,
                duration: 300,
                frequency: 128,
            },
            ScriptCommandEffect::PrintBig {
                key: *b"TEST\0\0\0\0",
                time_ms: 2_000,
                style: 1,
                queue: true,
            },
            ScriptCommandEffect::SwitchHelicopter(true),
        ]
    );
}

#[test]
fn touching_object_on_foot_uses_only_the_ped_collision_records() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_IS_PLAYER_TOUCHING_OBJECT_ON_FOOT);
    push_i16(&mut program, 0);
    push_i16(&mut program, 44);
    push_opcode(&mut program, COMMAND_IS_CHAR_TOUCHING_OBJECT_ON_FOOT);
    push_i16(&mut program, 12);
    push_i16(&mut program, 44);
    push_opcode(&mut program, COMMAND_IS_CHAR_TOUCHING_OBJECT);
    push_i16(&mut program, 12);
    push_i16(&mut program, 44);
    let (mut scripts, mut script) = vm_with_program(&program);

    let mut player_probe = |index| (index == 0).then(|| player_snapshot(11));
    let mut char_probe = |handle| {
        let mut snapshot = entity_snapshot(CVector::ZERO);
        snapshot.collision_records[0] = Some(EntityReference::Object(44));
        match handle {
            11 => Some(snapshot),
            12 => {
                snapshot.current_vehicle = Some(77);
                Some(snapshot)
            }
            _ => None,
        }
    };
    let mut vehicle_probe = |handle| {
        (handle == 77).then(|| {
            let mut snapshot = entity_snapshot(CVector::ZERO);
            snapshot.collision_records[0] = Some(EntityReference::Object(44));
            snapshot
        })
    };
    let mut object_probe = |handle| (handle == 44).then(|| entity_snapshot(CVector::ZERO));

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            Some(&mut player_probe),
            Some(&mut char_probe),
            Some(&mut vehicle_probe),
            Some(&mut object_probe),
            None,
        )
        .unwrap();
    assert!(script.cond_result);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            Some(&mut player_probe),
            Some(&mut char_probe),
            Some(&mut vehicle_probe),
            Some(&mut object_probe),
            None,
        )
        .unwrap();
    assert!(!script.cond_result);

    script
        .process_one_command(
            &mut scripts,
            0,
            1.0,
            None,
            None,
            None,
            None,
            None,
            Some(&mut player_probe),
            Some(&mut char_probe),
            Some(&mut vehicle_probe),
            Some(&mut object_probe),
            None,
        )
        .unwrap();
    assert!(script.cond_result);
}

#[test]
fn restart_critical_mission_overrides_restart_before_failing_player() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_RESTART_CRITICAL_MISSION);
    push_scm_float(&mut program, 1.0);
    push_scm_float(&mut program, 2.0);
    push_scm_float(&mut program, MAP_Z_LOW_LIMIT);
    push_scm_float(&mut program, 90.0);
    let (mut scripts, mut script) = vm_with_program(&program);

    script
        .process_one_command(
            &mut scripts,
            123,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::OverrideNextRestart {
                position: CVector::new(1.0, 2.0, MAP_Z_LOW_LIMIT),
                heading: 90.0,
                needs_ground: true,
            },
            ScriptCommandEffect::PlayerFailedCriticalMission,
        ]
    );
}

#[test]
fn garage_taxi_and_sprite_entity_blips_preserve_script3_contracts() {
    let mut program = Vec::new();
    push_opcode(&mut program, COMMAND_ACTIVATE_GARAGE);
    push_i16(&mut program, 2);
    push_opcode(&mut program, COMMAND_SWITCH_TAXI_TIMER);
    push_i16(&mut program, 1);
    push_opcode(&mut program, COMMAND_ADD_SPRITE_BLIP_FOR_CAR);
    push_i16(&mut program, 41);
    push_i16(&mut program, 5);
    push_global_output(&mut program, 100);
    push_opcode(&mut program, COMMAND_ADD_SPRITE_BLIP_FOR_CHAR);
    push_i16(&mut program, 42);
    push_i16(&mut program, 6);
    push_global_output(&mut program, 104);
    push_opcode(&mut program, COMMAND_ADD_SPRITE_BLIP_FOR_OBJECT);
    push_i16(&mut program, 43);
    push_i16(&mut program, 7);
    push_global_output(&mut program, 108);
    push_opcode(&mut program, COMMAND_DEACTIVATE_GARAGE);
    push_i16(&mut program, 2);
    let (mut scripts, mut script) = vm_with_program(&program);
    let mut vehicle_probe = |handle| (handle == 41).then(|| entity_snapshot(CVector::ZERO));
    let mut char_probe = |handle| (handle == 42).then(|| entity_snapshot(CVector::ZERO));
    let mut object_probe = |handle| (handle == 43).then(|| entity_snapshot(CVector::ZERO));

    for _ in 0..6 {
        script
            .process_one_command(
                &mut scripts,
                123,
                1.0,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&mut char_probe),
                Some(&mut vehicle_probe),
                Some(&mut object_probe),
                None,
            )
            .unwrap();
    }

    assert_eq!(
        scripts.pending_effects,
        vec![
            ScriptCommandEffect::SetGarageActive {
                handle: 2,
                active: true,
            },
            ScriptCommandEffect::SwitchTaxiTimer {
                enabled: true,
                time_ms: 123,
            },
            ScriptCommandEffect::CreateEntityBlip {
                handle: 1,
                entity_handle: 41,
                entity_type: 1,
                colour: 0,
                display: 3,
                sprite: 5,
                scale: 1,
            },
            ScriptCommandEffect::CreateEntityBlip {
                handle: 2,
                entity_handle: 42,
                entity_type: 2,
                colour: 1,
                display: 3,
                sprite: 6,
                scale: 1,
            },
            ScriptCommandEffect::CreateEntityBlip {
                handle: 3,
                entity_handle: 43,
                entity_type: 3,
                colour: 6,
                display: 3,
                sprite: 7,
                scale: 1,
            },
            ScriptCommandEffect::SetGarageActive {
                handle: 2,
                active: false,
            },
        ]
    );
}
