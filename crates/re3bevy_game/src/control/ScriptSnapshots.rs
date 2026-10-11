//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Split from control/Script.rs (verbatim, parity-preserving).

#[allow(unused_imports)]
use super::Script::{
    ScriptEntityIndex, ScriptEntitySnapshot, ScriptEntityWorld, ScriptObjectEntity, ScriptObjectRef,
    ScriptPedEntity, ScriptPlayerSnapshot, ScriptVehicleEntity, remove_script_object,
    script_vehicle_is_stopped, set_script_snapshot_bounds, spawn_script_object,
};
#[allow(unused_imports)]
use crate::audio::soundlist::*;
#[allow(unused_imports)]
use crate::control::CarCtrl::CCarCtrl;
#[allow(unused_imports)]
use crate::control::Pickups::{CPickups, PICKUP_IN_SHOP, PICKUP_IN_SHOP_OUT_OF_STOCK};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPoolHandle, CPools};
#[allow(unused_imports)]
use crate::entities::Physical::{GTA_PHYSICS_HZ, physical_collision_records};
#[allow(unused_imports)]
use crate::math::Matrix::CMatrix;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_PICKUP_BONUS;
#[allow(unused_imports)]
use crate::objects::Object::{CObject, MISSION_OBJECT};
#[allow(unused_imports)]
use crate::peds::Ped::{
    CPed, MISSION_CHAR, PedState, eMoveState, eObjective, ped_weapon_ammo_snapshot,
    script_ped_can_see_player,
};
#[allow(unused_imports)]
use crate::peds::PlayerPed::{CWorldPlayers, WorldPlayerAccess};
#[allow(unused_imports)]
use crate::peds::Population::{CPopulation, remove_ped_from_population_counts};
#[allow(unused_imports)]
use crate::peds::VehicleRelations::VehicleSeat;
#[allow(unused_imports)]
use crate::vehicles::Automobile::CAutomobile;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::{
    CVehicle, MISSION_VEHICLE, PARKED_VEHICLE, PERMANENT_VEHICLE, RANDOM_VEHICLE,
};
#[allow(unused_imports)]
use crate::weapons::WeaponType::WEAPONTYPE_TOTAL_INVENTORY_WEAPONS;
#[allow(unused_imports)]
use bevy::prelude::*;
#[allow(unused_imports)]
use re3bevy_core::core::Timer::CTimer;
#[allow(unused_imports)]
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};
pub(crate) fn script_ped_snapshot_from_entity(
    entity: &ScriptPedEntity,
    current_vehicle: Option<i32>,
) -> ScriptEntitySnapshot {
    let ped = &entity.ped;
    let vitality = &entity.vitality;
    ScriptEntitySnapshot {
        position: ped.physical.entity.get_position(),
        bound_center: ped.physical.entity.get_position(),
        bound_radius: 0.8,
        heading_degrees: ped.physical.entity.get_heading_degrees(),
        up_z: ped.physical.entity.get_up().z,
        model_index: i32::from(ped.physical.entity.model_index),
        entity_status: ped.physical.entity.status as i32,
        alive: vitality.health > 0.0,
        current_vehicle,
        current_vehicle_model: None,
        vehicle_colour1: None,
        vehicle_colour2: None,
        vehicle_is_boat: false,
        vehicle_in_water: false,
        automobile_drive_wheels_on_ground: None,
        stopped: matches!(
            ped.move_state,
            eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL
        ),
        is_shooting: ped.flags.is_shooting,
        collision_records: physical_collision_records(&ped.physical),
        speed: ped.physical.move_speed.magnitude(),
        health: vitality.health,
        current_weapon: ped.current_weapon as i32,
        weapon_ammo: ped_weapon_ammo_snapshot(ped),
        script_objective_completed: ped.flags.script_objective_completed,
        leader_slot: ped.leader,
        ped_type: Some(ped.ped_type as i32),
        vehicle_bomb_type: None,
        vehicle_damaged: false,
        last_weapon_damage: i32::from(vitality.last_wep_dam),
        vehicle_num_passengers: 0,
        vehicle_num_max_passengers: 0,
        vehicle_passenger_handles: [None; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS],
        vehicle_closed_doors: 0,
        can_see_player: false,
    }
}

pub(crate) fn script_vehicle_snapshot_from_entity(
    vehicle: crate::control::ScriptEntities::ScriptVehicleRef<'_>,
    mut passenger_handle_for_seat: impl FnMut(usize) -> Option<i32>,
) -> ScriptEntitySnapshot {
    let base = vehicle.base();
    let mut passenger_handles = [None; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS];
    for (index, passenger) in passenger_handles.iter_mut().enumerate() {
        *passenger = passenger_handle_for_seat(index);
    }
    let passenger_count = passenger_handles.iter().flatten().count() as u8;
    let closed_doors = match vehicle {
        crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
            let mut bits = 0_u8;
            for door in 0..crate::vehicles::Automobile::NUM_CAR_DOORS {
                if car.damage.get_door_status(door as u8)
                    != crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_MISSING as u32
                    && car.doors[door].is_closed()
                {
                    bits |= 1 << door;
                }
            }
            bits
        }
        crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
        | crate::control::ScriptEntities::ScriptVehicleRef::Heli { .. }
        | crate::control::ScriptEntities::ScriptVehicleRef::Plane { .. } => 0,
    };
    ScriptEntitySnapshot {
        position: base.physical.entity.get_position(),
        bound_center: base.physical.entity.get_position(),
        bound_radius: 1.2,
        heading_degrees: base.physical.entity.get_heading_degrees(),
        up_z: base.physical.entity.get_up().z,
        model_index: i32::from(base.physical.entity.model_index),
        entity_status: base.physical.entity.status as i32,
        alive: base.vitality.health > 0.0,
        current_vehicle: None,
        current_vehicle_model: Some(i32::from(base.physical.entity.model_index)),
        vehicle_colour1: Some(i32::from(base.appearance.current_colour1)),
        vehicle_colour2: Some(i32::from(base.appearance.current_colour2)),
        vehicle_is_boat: matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }),
        vehicle_in_water: base.physical.flags.is_in_water,
        automobile_drive_wheels_on_ground: match vehicle {
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => Some(car.drive_wheels_on_ground),
            _ => None,
        },
        stopped: base.physical.move_speed.magnitude() <= 0.01,
        is_shooting: false,
        collision_records: physical_collision_records(&base.physical),
        speed: base.physical.move_speed.magnitude(),
        health: base.vitality.health,
        current_weapon: 0,
        weapon_ammo: [0; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
        script_objective_completed: false,
        leader_slot: None,
        ped_type: None,
        vehicle_bomb_type: match vehicle {
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => Some(car.bomb_type as i32),
            crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
            | crate::control::ScriptEntities::ScriptVehicleRef::Heli { .. }
            | crate::control::ScriptEntities::ScriptVehicleRef::Plane { .. } => None,
        },
        vehicle_damaged: base.appearance.is_damaged,
        last_weapon_damage: i32::from(base.vitality.last_weapon_damage),
        vehicle_num_passengers: passenger_count,
        vehicle_num_max_passengers: base.entry.num_max_passengers,
        vehicle_passenger_handles: passenger_handles,
        vehicle_closed_doors: closed_doors,
        can_see_player: false,
    }
}

pub(crate) fn script_object_snapshot(entity: &ScriptObjectRef<'_>) -> ScriptEntitySnapshot {
    let object = &entity.object;
    ScriptEntitySnapshot {
        position: object.physical.entity.get_position(),
        bound_center: object.physical.entity.get_position(),
        bound_radius: 0.5,
        heading_degrees: object.physical.entity.get_heading_degrees(),
        up_z: object.physical.entity.get_up().z,
        model_index: i32::from(object.physical.entity.model_index),
        entity_status: object.physical.entity.status as i32,
        alive: !object.flags.has_been_damaged,
        current_vehicle: None,
        current_vehicle_model: Some(i32::from(object.physical.entity.model_index)),
        vehicle_colour1: None,
        vehicle_colour2: None,
        vehicle_is_boat: false,
        vehicle_in_water: false,
        automobile_drive_wheels_on_ground: None,
        stopped: object.physical.move_speed.magnitude() < 0.01,
        is_shooting: false,
        collision_records: physical_collision_records(&object.physical),
        speed: object.physical.move_speed.magnitude(),
        health: if object.flags.has_been_damaged {
            0.0
        } else {
            100.0
        },
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
        can_see_player: false,
    }
}

pub(crate) fn script_char_entity_snapshot(
    entities: &ScriptEntityWorld,
    handle: i32,
    player_position: CVector,
    file_loader: &CFileLoader,
) -> Option<ScriptEntitySnapshot> {
    let mut snapshot = entities.char_snapshot(handle)?;
    let entity = entities.peds.get(&entities.index, &handle)?;
    set_script_snapshot_bounds(
        &mut snapshot,
        &entity.ped.physical.entity.matrix,
        entity.ped.physical.entity.model_index.max(0) as usize,
        file_loader,
    );
    snapshot.can_see_player = script_ped_can_see_player(
        entity.ped.physical.entity.get_position(),
        entity.ped.physical.entity.get_forward(),
        player_position,
        file_loader,
    );
    Some(snapshot)
}

pub(crate) fn script_vehicle_entity_snapshot(
    entities: &ScriptEntityWorld,
    handle: i32,
    time_step: f32,
    world_players: &CWorldPlayers,
    file_loader: &CFileLoader,
) -> Option<ScriptEntitySnapshot> {
    let mut snapshot = entities.vehicle_snapshot(handle)?;
    let base = entities.vehicles.get(&entities.index, &handle)?.base();
    snapshot.stopped = script_vehicle_is_stopped(base.physical.distance_travelled, time_step);
    let vehicle_entity = entities.vehicle_entity(handle)?;
    for local_player in 0..world_players.slot_count() {
        let Some(relation) = world_players.relation(local_player as i32) else {
            continue;
        };
        if relation.vehicle != vehicle_entity {
            continue;
        }
        let VehicleSeat::Passenger(index) = relation.seat else {
            continue;
        };
        let Some(slot) = snapshot
            .vehicle_passenger_handles
            .get_mut(usize::from(index))
        else {
            continue;
        };
        if slot.is_none() {
            *slot = world_players
                .player(local_player as i32)
                .map(|player| player.pool_handle);
        }
    }
    snapshot.vehicle_num_passengers =
        snapshot.vehicle_passenger_handles.iter().flatten().count() as u8;
    set_script_snapshot_bounds(
        &mut snapshot,
        &base.physical.entity.matrix,
        base.physical.entity.model_index.max(0) as usize,
        file_loader,
    );
    Some(snapshot)
}

pub(crate) fn script_object_entity_snapshot(
    entity: &ScriptObjectRef<'_>,
    file_loader: &CFileLoader,
) -> ScriptEntitySnapshot {
    let mut snapshot = script_object_snapshot(entity);
    let object = &entity.object;
    set_script_snapshot_bounds(
        &mut snapshot,
        &object.physical.entity.matrix,
        object.physical.entity.model_index.max(0) as usize,
        file_loader,
    );
    snapshot
}

pub(crate) fn script_object_handle_for_pool_slot(
    index: &ScriptEntityIndex,
    slot: usize,
) -> Option<i32> {
    index.objects.handle_for_slot(slot)
}

/// re3 `CPickup::GiveUsAPickUpObject(-1)` followed by `CWorld::Add`.
///
/// `CPickups` keeps the pickup-generation handle while radar and world code
/// refer to the separately allocated CObject pool slot.
pub(crate) fn materialize_pickup_object(
    pickup_handle: i32,
    pickups: &mut CPickups,
    commands: &mut Commands,
    script_entity_index: &mut ScriptEntityIndex,
    pools: &mut CPools,
    general: &mut CGeneral,
    file_loader: &CFileLoader,
) -> Option<i32> {
    let pickup_slot = pickups.actual_pickup_index(pickup_handle)?;
    let pickup = pickups.pickups[pickup_slot];
    let Some(object_handle) = pools
        .object_pool
        .new_entry(CPoolEntry::new(CPoolEntryKind::Object))
    else {
        pickups.pickups[pickup_slot].object_present = false;
        pickups.pickups[pickup_slot].object_pool_index = None;
        return None;
    };

    let mut object = CObject::new_with_model(i32::from(pickup.model_index), false, general);
    object.initialize_world_state(file_loader);
    object.object_created_by = MISSION_OBJECT;
    object.physical.entity.set_position(pickup.pos);
    object
        .physical
        .entity
        .set_orientation(0.0, 0.0, -std::f32::consts::FRAC_PI_2);
    object.object_matrix = object.physical.entity.matrix;
    object.physical.flags.affected_by_gravity = false;
    object.physical.entity.flags.explosion_proof = true;
    object.physical.entity.flags.uses_collision = false;
    object.flags.is_pickup = true;
    object.bonus_value = if usize::from(pickup.model_index as u16) == MI_PICKUP_BONUS {
        pickup.quantity as i8
    } else {
        0
    };
    object.flags.pickup_obj_with_message = pickup.pickup_type == PICKUP_IN_SHOP;
    object.flags.out_of_stock = pickup.pickup_type == PICKUP_IN_SHOP_OUT_OF_STOCK;
    object.physical.entity.flags.render_scorched =
        pickup.pickup_type == PICKUP_IN_SHOP_OUT_OF_STOCK;

    pickups.pickups[pickup_slot].object_present = true;
    pickups.pickups[pickup_slot].object_pool_index = Some(object_handle.index() as i32);
    spawn_script_object(
        commands,
        script_entity_index,
        ScriptObjectEntity {
            handle: object_handle.0,
            object,
        },
    );
    Some(object_handle.0)
}

pub(crate) fn remove_materialized_pickup(
    pickup_handle: i32,
    pickups: &mut CPickups,
    commands: &mut Commands,
    script_entity_index: &mut ScriptEntityIndex,
    pools: &mut CPools,
) -> bool {
    let Some(pickup_slot) = pickups.actual_pickup_index(pickup_handle) else {
        return false;
    };
    if let Some(object_pool_index) = pickups.pickups[pickup_slot].object_pool_index {
        if let Some(object_handle) =
            script_object_handle_for_pool_slot(script_entity_index, object_pool_index as usize)
        {
            let _ = remove_script_object(commands, script_entity_index, Some(pools), object_handle);
        } else {
            pools.object_pool.delete_by_slot(object_pool_index as usize);
        }
    }
    pickups.remove_pickup(pickup_handle)
}

pub(crate) fn set_all_occupants_of_car_leave_car(
    script_entities: &mut ScriptEntityWorld,
    world_players: &mut dyn WorldPlayerAccess,
    vehicle_handle: i32,
    now_ms: u32,
) -> bool {
    let Some(vehicle_entity) = script_entities.vehicle_entity(vehicle_handle) else {
        return false;
    };
    let vehicle_slot = CPoolHandle(vehicle_handle).index();

    // `InVehicle` on the ped is the sole occupant authority.  Build the
    // native driver-then-passenger order from the typed seat value instead of
    // reading either a vehicle-side integer slot or a ped-side slot mirror.
    let mut occupants: Vec<(u8, bool, i32)> = script_entities
        .script_occupant_handles(vehicle_entity)
        .map(|(handle, seat)| {
            let order = match seat {
                VehicleSeat::Driver => 0,
                VehicleSeat::Passenger(index) => index.saturating_add(1),
            };
            (order, false, handle)
        })
        .collect();
    for index in 0..crate::peds::PlayerPed::MAX_PLAYERS {
        let Some(relation) = world_players.vehicle_relation(index as i32) else {
            continue;
        };
        if relation.vehicle != vehicle_entity {
            continue;
        }
        let Some(player) = world_players.player(index as i32) else {
            continue;
        };
        let order = match relation.seat {
            VehicleSeat::Driver => 0,
            VehicleSeat::Passenger(seat) => seat.saturating_add(1),
        };
        occupants.push((order, true, player.pool_handle));
    }
    occupants.sort_unstable_by_key(|(order, is_player, handle)| (*order, *is_player, *handle));

    for (_, is_player, handle) in occupants {
        if is_player {
            let Some(player_index) = (0..crate::peds::PlayerPed::MAX_PLAYERS).find(|index| {
                world_players
                    .player(*index as i32)
                    .is_some_and(|player| player.pool_handle == handle)
            }) else {
                continue;
            };
            let Some(player) = world_players.player_mut(player_index as i32) else {
                continue;
            };
            player.ped.flags.script_objective_completed = false;
            player.ped.set_objective_entity(eObjective::OBJECTIVE_LEAVE_CAR,
                vehicle_slot,
                now_ms,
                Some(vehicle_slot),
            );
        } else if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
            entity.ped.flags.script_objective_completed = false;
            entity.ped.set_objective_entity(eObjective::OBJECTIVE_LEAVE_CAR,
                vehicle_slot,
                now_ms,
                Some(vehicle_slot),
            );
        }
    }
    true
}

pub(crate) fn script_vehicle_base_can_be_deleted<VehCore, VehVitality, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    entities: &ScriptEntityWorld,
    vehicle_handle: i32,
    vehicle: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
) -> bool
where
    VehCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let Some(vehicle_entity) = entities.vehicle_entity(vehicle_handle) else {
        return false;
    };
    let occupants_can_be_deleted = entities.relation_occupants(vehicle_entity).all(|(ped, _)| {
        let Some(handle) = entities.ped_handle_for_entity(ped) else {
            // `relations` includes player `PedEntity`s. A player occupant blocks
            // CVehicle::CanBeDeleted just like re3's pointer walk.
            return false;
        };
        entities.peds.get(&entities.index, &handle).is_none_or(|ped| {
            ped.ped.char_created_by != MISSION_CHAR
                && matches!(
                    ped.ped.ped_state,
                    PedState::PED_DRIVING | PedState::PED_DEAD
                )
        })
    });
    vehicle.can_be_deleted_with(occupants_can_be_deleted)
}

pub(crate) fn remove_script_vehicle_and_occupants(
    entities: &mut ScriptEntityWorld,
    pools: &mut CPools,
    handle: i32,
    mut population: Option<&mut CPopulation>,
    car_ctrl: Option<&mut CCarCtrl>,
) {
    let occupant_handles: Vec<i32> = entities
        .vehicle_entity(handle)
        .map(|vehicle_entity| {
            entities
                .script_occupant_handles(vehicle_entity)
                .map(|(ped_handle, _)| ped_handle)
                .collect()
        })
        .unwrap_or_default();
    for ped_handle in occupant_handles {
        if let Some(ped_type) = entities.peds.get(&entities.index, &ped_handle)
            .map(|entity| entity.ped.ped_type)
        {
            if let Some(population) = population.as_deref_mut() {
                remove_ped_from_population_counts(population, ped_type);
            }
        }
        entities.remove_char(pools, ped_handle);
    }
    if let (Some(car_ctrl), Some(vehicle)) = (car_ctrl, entities.vehicles.get(&entities.index, &handle)) {
        match vehicle.base().vehicle_created_by {
            RANDOM_VEHICLE => {
                car_ctrl.num_random_cars = car_ctrl.num_random_cars.saturating_sub(1);
                if vehicle.base().flags.is_law_enforcer {
                    car_ctrl.num_law_enforcer_cars =
                        car_ctrl.num_law_enforcer_cars.saturating_sub(1);
                }
            }
            PARKED_VEHICLE => {
                car_ctrl.num_parked_cars = car_ctrl.num_parked_cars.saturating_sub(1);
            }
            MISSION_VEHICLE => {
                car_ctrl.num_mission_cars = car_ctrl.num_mission_cars.saturating_sub(1);
            }
            PERMANENT_VEHICLE => {
                car_ctrl.num_permanent_cars = car_ctrl.num_permanent_cars.saturating_sub(1);
            }
            _ => {}
        }
    }
    entities.remove_car(pools, handle);
}

pub(crate) fn script_sound_frontend_sound(sound_id: i32) -> Option<i32> {
    match sound_id {
        SCRIPT_SOUND_EVIDENCE_PICKUP => Some(SOUND_EVIDENCE_PICKUP),
        SCRIPT_SOUND_UNLOAD_GOLD => Some(SOUND_UNLOAD_GOLD),
        SCRIPT_SOUND_PART_MISSION_COMPLETE => Some(SOUND_PART_MISSION_COMPLETE),
        SCRIPT_SOUND_RACE_START_3 => Some(SOUND_RACE_START_3),
        SCRIPT_SOUND_RACE_START_2 => Some(SOUND_RACE_START_2),
        SCRIPT_SOUND_RACE_START_1 => Some(SOUND_RACE_START_1),
        SCRIPT_SOUND_RACE_START_GO => Some(SOUND_RACE_START_GO),
        _ => None,
    }
}

pub(crate) fn mission_audio_sample_from_key(key: &[u8; 8]) -> Option<u32> {
    use crate::audio::AudioSamples as sfx;
    let name = script_key_to_string(key).to_ascii_lowercase();
    Some(match name.as_str() {
        "lib_a1" => sfx::SFX_MISSION_LIB_A1,
        "lib_a2" => sfx::SFX_MISSION_LIB_A2,
        "lib_a" => sfx::SFX_MISSION_LIB_A,
        "lib_b" => sfx::SFX_MISSION_LIB_B,
        "lib_c" => sfx::SFX_MISSION_LIB_C,
        "lib_d" => sfx::SFX_MISSION_LIB_D,
        "l2_a" => sfx::SFX_MISSION_L2_A,
        "j4t_1" => sfx::SFX_MISSION_J4T_1,
        "j4t_2" => sfx::SFX_MISSION_J4T_2,
        "j4t_3" => sfx::SFX_MISSION_J4T_3,
        "j4t_4" => sfx::SFX_MISSION_J4T_4,
        "j4_a" => sfx::SFX_MISSION_J4_A,
        "j4_b" => sfx::SFX_MISSION_J4_B,
        "j4_c" => sfx::SFX_MISSION_J4_C,
        "j4_d" => sfx::SFX_MISSION_J4_D,
        "j4_e" => sfx::SFX_MISSION_J4_E,
        "j4_f" => sfx::SFX_MISSION_J4_F,
        "j6_1" => sfx::SFX_MISSION_J6_1,
        "j6_a" => sfx::SFX_MISSION_J6_A,
        "j6_b" => sfx::SFX_MISSION_J6_B,
        "j6_c" => sfx::SFX_MISSION_J6_C,
        "j6_d" => sfx::SFX_MISSION_J6_D,
        "t4_a" => sfx::SFX_MISSION_T4_A,
        "s1_a" => sfx::SFX_MISSION_S1_A,
        "s1_a1" => sfx::SFX_MISSION_S1_A1,
        "s1_b" => sfx::SFX_MISSION_S1_B,
        "s1_c" => sfx::SFX_MISSION_S1_C,
        "s1_c1" => sfx::SFX_MISSION_S1_C1,
        "s1_d" => sfx::SFX_MISSION_S1_D,
        "s1_e" => sfx::SFX_MISSION_S1_E,
        "s1_f" => sfx::SFX_MISSION_S1_F,
        "s1_g" => sfx::SFX_MISSION_S1_G,
        "s1_h" => sfx::SFX_MISSION_S1_H,
        "s1_i" => sfx::SFX_MISSION_S1_I,
        "s1_j" => sfx::SFX_MISSION_S1_J,
        "s1_k" => sfx::SFX_MISSION_S1_K,
        "s1_l" => sfx::SFX_MISSION_S1_L,
        "s3_a" => sfx::SFX_MISSION_S3_A,
        "s3_b" => sfx::SFX_MISSION_S3_B,
        "el3_a" => sfx::SFX_MISSION_EL3_A,
        "mf1_a" => sfx::SFX_MISSION_MF1_A,
        "mf2_a" => sfx::SFX_MISSION_MF2_A,
        "mf3_a" => sfx::SFX_MISSION_MF3_A,
        "mf3_b" => sfx::SFX_MISSION_MF3_B,
        "mf3_b1" => sfx::SFX_MISSION_MF3_B1,
        "mf3_c" => sfx::SFX_MISSION_MF3_C,
        "mf4_a" => sfx::SFX_MISSION_MF4_A,
        "mf4_b" => sfx::SFX_MISSION_MF4_B,
        "mf4_c" => sfx::SFX_MISSION_MF4_C,
        "a1_a" => sfx::SFX_MISSION_A1_A,
        "a3_a" => sfx::SFX_MISSION_A3_A,
        "a5_a" => sfx::SFX_MISSION_A5_A,
        "a4_a" => sfx::SFX_MISSION_A4_A,
        "a4_b" => sfx::SFX_MISSION_A4_B,
        "a4_c" => sfx::SFX_MISSION_A4_C,
        "a4_d" => sfx::SFX_MISSION_A4_D,
        "k1_a" => sfx::SFX_MISSION_K1_A,
        "k3_a" => sfx::SFX_MISSION_K3_A,
        "r1_a" => sfx::SFX_MISSION_R1_A,
        "r2_a" => sfx::SFX_MISSION_R2_A,
        "r2_b" => sfx::SFX_MISSION_R2_B,
        "r2_c" => sfx::SFX_MISSION_R2_C,
        "r2_d" => sfx::SFX_MISSION_R2_D,
        "r2_e" => sfx::SFX_MISSION_R2_E,
        "r2_f" => sfx::SFX_MISSION_R2_F,
        "r2_g" => sfx::SFX_MISSION_R2_G,
        "r2_h" => sfx::SFX_MISSION_R2_H,
        "r5_a" => sfx::SFX_MISSION_R5_A,
        "r6_a" => sfx::SFX_MISSION_R6_A,
        "r6_a1" => sfx::SFX_MISSION_R6_A1,
        "r6_b" => sfx::SFX_MISSION_R6_B,
        "lo2_a" => sfx::SFX_MISSION_LO2_A,
        "lo6_a" => sfx::SFX_MISSION_LO6_A,
        "yd2_a" => sfx::SFX_MISSION_YD2_A,
        "yd2_b" => sfx::SFX_MISSION_YD2_B,
        "yd2_c" => sfx::SFX_MISSION_YD2_C,
        "yd2_c1" => sfx::SFX_MISSION_YD2_C1,
        "yd2_d" => sfx::SFX_MISSION_YD2_D,
        "yd2_e" => sfx::SFX_MISSION_YD2_E,
        "yd2_f" => sfx::SFX_MISSION_YD2_F,
        "yd2_g" => sfx::SFX_MISSION_YD2_G,
        "yd2_h" => sfx::SFX_MISSION_YD2_H,
        "yd2_ass" => sfx::SFX_MISSION_YD2_ASS,
        "yd2_ok" => sfx::SFX_MISSION_YD2_OK,
        "h5_a" => sfx::SFX_MISSION_H5_A,
        "h5_b" => sfx::SFX_MISSION_H5_B,
        "h5_c" => sfx::SFX_MISSION_H5_C,
        "ammu_a" => sfx::SFX_MISSION_AMMU_A,
        "ammu_b" => sfx::SFX_MISSION_AMMU_B,
        "ammu_c" => sfx::SFX_MISSION_AMMU_C,
        "door_1" => sfx::SFX_MISSION_DOOR_1,
        "door_2" => sfx::SFX_MISSION_DOOR_2,
        "door_3" => sfx::SFX_MISSION_DOOR_3,
        "door_4" => sfx::SFX_MISSION_DOOR_4,
        "door_5" => sfx::SFX_MISSION_DOOR_5,
        "door_6" => sfx::SFX_MISSION_DOOR_6,
        "t3_a" => sfx::SFX_MISSION_T3_A,
        "t3_b" => sfx::SFX_MISSION_T3_B,
        "t3_c" => sfx::SFX_MISSION_T3_C,
        "k1_b" => sfx::SFX_MISSION_K1_B,
        "c_1" => sfx::SFX_MISSION_CAT1,
        _ => return None,
    } as u32)
}

pub(crate) fn script_char_say_sound(sound_id: i32) -> Option<i16> {
    match sound_id {
        SCRIPT_SOUND_CHUNKY_RUN_SHOUT | SCRIPT_SOUND_SECURITY_GUARD_AWAY_SHOUT => {
            Some(SOUND_PED_FLEE_RUN as i16)
        }
        SCRIPT_SOUND_SWAT_PED_SHOUT => Some(SOUND_PED_PURSUIT_SWAT as i16),
        SCRIPT_SOUND_AMMUNATION_CHAT_1 => Some(SOUND_AMMUNATION_WELCOME_1 as i16),
        SCRIPT_SOUND_AMMUNATION_CHAT_2 => Some(SOUND_AMMUNATION_WELCOME_2 as i16),
        SCRIPT_SOUND_AMMUNATION_CHAT_3 => Some(SOUND_AMMUNATION_WELCOME_3 as i16),
        _ => None,
    }
}

pub(crate) fn queue_ped_script_say(ped: &mut CPed, sound: i16) {
    if sound < ped.queued_sound {
        ped.last_queued_sound = ped.queued_sound;
        ped.queued_sound = sound;
        ped.flags.is_talking = true;
    }
}

pub(crate) fn script_key_to_string(key: &[u8]) -> String {
    let end = key.iter().position(|&b| b == 0).unwrap_or(key.len());
    String::from_utf8_lossy(&key[..end]).trim().to_string()
}
