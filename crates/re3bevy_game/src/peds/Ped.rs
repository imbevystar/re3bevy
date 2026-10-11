//! re3 reference: references/re-GTA/src/peds/Ped.cpp
//! re3 reference: references/re-GTA/src/peds/Ped.h
//!
//! `CPed` constructor state and top-level initialise state.

use crate::peds::PedLocomotion::PedLocomotion;
use crate::peds::PedObstacle::PedObstacleAvoidance;
use bevy::prelude::{Entity, Resource};
use super::{PedAnimation::PedAnimation, PedVitality::PedVitality, PedDeath::PedDeathLifecycle};
use re3bevy_core::core::{
    Coordinates::{CVector, CVector2D},
    General::CGeneral,
};

use crate::{
    animation::{
        AnimAssocDefinitions::{ASSOCGRP_PLAYER, ASSOCGRP_STD},
        AnimBlendAssociation::ASSOC_DELETEFADEDOUT,
        AnimBlendClumpData::CAnimBlendClumpData,
        AnimManager::CAnimManager,
        AnimationId::{
            ANIM_STD_CAR_GET_IN_LHS, ANIM_STD_CAR_GET_IN_LO_LHS, ANIM_STD_IDLE,
            ANIM_STD_JACKEDCAR_LHS, ANIM_STD_QUICKJACKED, ANIM_STD_RUN, ANIM_STD_RUNFAST,
            ANIM_STD_TRAIN_GETOUT, ANIM_STD_VAN_GET_IN_REAR_LHS, ANIM_STD_WALK, AnimationId,
        },
        FrameUpdate::frame_update_callback_with_velocity_extraction_non_skinned,
    },
    control::PathFind::PATH_PED,
    core::{FileLoader::CFileLoader, Pools::CPoolHandle},
    entities::{
        Entity::eEntityType,
        Physical::{CPhysical, GRAVITY},
    },
    modelinfo::{ModelInfo::ModelInfoSlot, VehicleModelInfo::vehicle_dummy_model_position},
    peds::{
        PedRoutes::{CPedRoutes, NUMPEDROUTES},
        PedType::{ePedStats, ePedType},
        VehicleRelations::VehicleSeat,
    },
    weapons::{
        Weapon::CWeapon,
        WeaponInfo::CWeaponInfoStore,
        WeaponType::{WEAPONTYPE_TOTAL_INVENTORY_WEAPONS, eWeaponType, script_weapon_type},
    },
};

/// re3 `CPed::SetChat`. Script ped references are represented by CPools
/// slots in the ECS runtime, matching every other CPed entity pointer field.
pub(crate) fn set_script_ped_chat(
    ped: &mut CPed,
    partner_slot: usize,
    duration_ms: u32,
    now_ms: u32,
) {
    if ped.ped_state != PedState::PED_CHAT && ped.last_ped_state == PedState::PED_NONE {
        if ped.ped_state == PedState::PED_WANDER_PATH {
            ped.flags.find_new_node_after_state_restore = true;
            if matches!(
                ped.move_state,
                eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL
            ) {
                ped.move_state = eMoveState::PEDMOVE_WALK;
            }
        }
        ped.last_ped_state = ped.ped_state;
        if ped.move_state as u8 >= ped.stored_move_state as u8 {
            ped.stored_move_state = ped.move_state;
        }
    }
    ped.ped_state = PedState::PED_CHAT;
    ped.move_state = eMoveState::PEDMOVE_STILL;
    ped.look_target = Some(partner_slot);
    ped.flags.is_looking = true;
    ped.flags.keep_trying_to_look = true;
    ped.chat_timer = now_ms.wrapping_add(duration_ms);
    ped.look_timer = now_ms.wrapping_add(3_000);
}

/// re3 `CPed::ClearAll` (Ped.cpp:1561) as reached from script
/// COMMAND_CHAR_WANDER_DIR. The GTA3 branch clears the aim and look flags
/// (`ClearAimFlag`/`ClearLookFlag`); VC_PED_PORTS-only `ClearPointGunAt` is
/// not part of this target.
pub(crate) fn clear_all_script_ped_state(ped: &mut CPed, vitality: &PedVitality, now_ms: u32) {
    if !ped.is_ped_in_control(vitality) && ped.ped_state != PedState::PED_DEAD {
        return;
    }
    ped.ped_state = PedState::PED_NONE;
    ped.move_state = eMoveState::PEDMOVE_NONE;
    ped.seek_target = None;
    ped.seek_pos = CVector::ZERO;
    ped.flee_from_pos_x = 0.0;
    ped.flee_from_pos_y = 0.0;
    ped.flee_from = None;
    ped.flee_timer = 0;
    ped.physical.entity.flags.uses_collision = true;
    if ped.flags.is_aiming_gun {
        ped.flags.is_aiming_gun = false;
        ped.flags.is_restoring_gun = true;
        ped.look_timer = 0;
    }
    if ped.flags.is_looking {
        ped.flags.is_looking = false;
        ped.flags.is_restoring_look = true;
        ped.flags.shake_fist = false;
        ped.look_timer = now_ms.wrapping_add(if ped.is_player() { 2000 } else { 4000 });
    }
    // TODO(parity): CPed::ClearAimFlag/ClearLookFlag common PedIK flag access.
    ped.flags.is_pointing_gun_at = false;
    ped.flags.render_ped_in_car = true;
    ped.flags.knocked_up_into_air = false;
    ped.colliding_entity = None;
}

/// re3 `CPed::SetIdle` (Ped.cpp:4994).
pub(crate) fn set_script_ped_idle(ped: &mut CPed, general: &mut CGeneral, now_ms: u32) {
    if !matches!(
        ped.ped_state,
        PedState::PED_IDLE | PedState::PED_MUG | PedState::PED_FLEE_ENTITY
    ) {
        ped.last_ped_state = PedState::PED_NONE;
        ped.ped_state = PedState::PED_IDLE;
        ped.move_state = eMoveState::PEDMOVE_STILL;
    }
    if ped.wait_state == eWaitState::WAITSTATE_FALSE {
        ped.wait_timer =
            now_ms.wrapping_add(general.get_random_number_in_range_i32(2000, 4000) as u32);
    }
}

/// re3 `CPed::SetWanderPath` (Ped.cpp:5866) for script
/// COMMAND_CHAR_WANDER_DIR (Script.cpp:1897). The direction loop mirrors
/// `ThePaths.FindNextNodeWandering` retries, including direction 0 as a valid
/// successful heading.
pub(crate) fn apply_script_ped_wander_path(
    ped: &mut CPed, vitality: &PedVitality,
    path: i32,
    file_loader: &CFileLoader,
    general: &mut CGeneral,
    now_ms: u32,
) {
    if !ped.is_ped_in_control(vitality) {
        ped.path_dir = path as i16;
        ped.flags.start_wander_path_on_foot = true;
        return;
    }
    ped.flags.start_wander_path_on_foot = false;
    if ped.flags.kinda_stay_in_same_place {
        set_script_ped_idle(ped, general, now_ms);
        return;
    }
    ped.path_dir = path as i16;
    let mut search_dir = path as u8;
    let loop_exit_dir = if search_dir == 0 {
        general.get_random_number_in_range_i32(1, 7) as u8
    } else {
        search_dir
    };
    let position = ped.physical.entity.get_position();
    let paths = &file_loader.path_find;
    loop {
        if let Some((node, next_dir)) = paths.find_next_node_wandering(
            PATH_PED,
            position,
            ped.last_path_node,
            search_dir,
            // re3 `FindNextNodeWandering`: buildings-only LOS at +1m on
            // both ends. Selecting an occluded initial node leaves peds
            // walking straight into the wall before the normal wander tick.
            |from, to| {
                file_loader.world.process_line_of_sight_buildings_hit(
                    [from.x, from.y, from.z],
                    [to.x, to.y, to.z],
                    &file_loader.model_info,
                    None,
                ).is_none()
            },
        ) {
            ped.path_dir = next_dir as i16;
            ped.next_path_node = Some(node);
            ped.ped_state = PedState::PED_WANDER_PATH;
            ped.move_state = eMoveState::PEDMOVE_WALK;
            ped.flags.is_running = false;
            return;
        }
        search_dir = (search_dir + 1) % 8;
        ped.path_dir = search_dir as i16;
        if search_dir == loop_exit_dir {
            clear_all_script_ped_state(ped, vitality, now_ms);
            set_script_ped_idle(ped, general, now_ms);
            return;
        }
    }
}

pub(crate) fn script_objective(value: i32) -> Option<eObjective> {
    Some(match value {
        0 => eObjective::OBJECTIVE_NONE,
        1 => eObjective::OBJECTIVE_WAIT_ON_FOOT,
        2 => eObjective::OBJECTIVE_FLEE_ON_FOOT_TILL_SAFE,
        3 => eObjective::OBJECTIVE_GUARD_SPOT,
        4 => eObjective::OBJECTIVE_GUARD_AREA,
        5 => eObjective::OBJECTIVE_WAIT_IN_CAR,
        6 => eObjective::OBJECTIVE_WAIT_IN_CAR_THEN_GET_OUT,
        7 => eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT,
        8 => eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS,
        9 => eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE,
        10 => eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS,
        11 => eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT,
        12 => eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION,
        13 => eObjective::OBJECTIVE_LEAVE_CAR,
        14 => eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER,
        15 => eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER,
        16 => eObjective::OBJECTIVE_FOLLOW_CAR_IN_CAR,
        17 => eObjective::OBJECTIVE_FIRE_AT_OBJECT_FROM_VEHICLE,
        18 => eObjective::OBJECTIVE_DESTROY_OBJECT,
        19 => eObjective::OBJECTIVE_DESTROY_CAR,
        20 => eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS,
        21 => eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT,
        22 => eObjective::OBJECTIVE_RUN_TO_AREA,
        23 => eObjective::OBJECTIVE_GOTO_AREA_IN_CAR,
        24 => eObjective::OBJECTIVE_FOLLOW_CAR_ON_FOOT_WITH_OFFSET,
        25 => eObjective::OBJECTIVE_GUARD_ATTACK,
        26 => eObjective::OBJECTIVE_SET_LEADER,
        27 => eObjective::OBJECTIVE_FOLLOW_ROUTE,
        28 => eObjective::OBJECTIVE_SOLICIT_VEHICLE,
        29 => eObjective::OBJECTIVE_HAIL_TAXI,
        30 => eObjective::OBJECTIVE_CATCH_TRAIN,
        31 => eObjective::OBJECTIVE_BUY_ICE_CREAM,
        32 => eObjective::OBJECTIVE_STEAL_ANY_CAR,
        33 => eObjective::OBJECTIVE_MUG_CHAR,
        34 => eObjective::OBJECTIVE_FLEE_CAR,
        _ => return None,
    })
}

pub(crate) fn script_formation(value: i32) -> eFormation {
    match value {
        1 => eFormation::FORMATION_REAR,
        2 => eFormation::FORMATION_REAR_LEFT,
        3 => eFormation::FORMATION_REAR_RIGHT,
        4 => eFormation::FORMATION_FRONT_LEFT,
        5 => eFormation::FORMATION_FRONT_RIGHT,
        6 => eFormation::FORMATION_LEFT,
        7 => eFormation::FORMATION_RIGHT,
        8 => eFormation::FORMATION_FRONT,
        _ => eFormation::FORMATION_UNDEFINED,
    }
}

pub(crate) fn script_wait_state(value: i32) -> Option<eWaitState> {
    Some(match value {
        0 => eWaitState::WAITSTATE_FALSE,
        1 => eWaitState::WAITSTATE_TRAFFIC_LIGHTS,
        2 => eWaitState::WAITSTATE_CROSS_ROAD,
        3 => eWaitState::WAITSTATE_CROSS_ROAD_LOOK,
        4 => eWaitState::WAITSTATE_LOOK_PED,
        5 => eWaitState::WAITSTATE_LOOK_SHOP,
        6 => eWaitState::WAITSTATE_LOOK_ACCIDENT,
        7 => eWaitState::WAITSTATE_FACEOFF_GANG,
        8 => eWaitState::WAITSTATE_DOUBLEBACK,
        9 => eWaitState::WAITSTATE_HITWALL,
        10 => eWaitState::WAITSTATE_TURN180,
        11 => eWaitState::WAITSTATE_SURPRISE,
        12 => eWaitState::WAITSTATE_STUCK,
        13 => eWaitState::WAITSTATE_LOOK_ABOUT,
        14 => eWaitState::WAITSTATE_PLAYANIM_DUCK,
        15 => eWaitState::WAITSTATE_PLAYANIM_COWER,
        16 => eWaitState::WAITSTATE_PLAYANIM_TAXI,
        17 => eWaitState::WAITSTATE_PLAYANIM_HANDSUP,
        18 => eWaitState::WAITSTATE_PLAYANIM_HANDSCOWER,
        19 => eWaitState::WAITSTATE_PLAYANIM_CHAT,
        20 => eWaitState::WAITSTATE_FINISH_FLEE,
        _ => return None,
    })
}

pub(crate) fn apply_ped_wait_state(ped: &mut CPed, locomotion: &mut crate::peds::PedLocomotion::PedLocomotion, state: i32, time_ms: Option<i32>, now_ms: u32) {
    let Some(wait_state) = script_wait_state(state) else {
        ped.wait_state = eWaitState::WAITSTATE_FALSE;
        locomotion.heading_rate = 15.0;
        return;
    };
    match wait_state {
        eWaitState::WAITSTATE_TRAFFIC_LIGHTS => {
            ped.wait_timer = now_ms.saturating_add(500);
            ped.move_state = eMoveState::PEDMOVE_STILL;
        }
        eWaitState::WAITSTATE_CROSS_ROAD => {
            ped.wait_timer = now_ms.saturating_add(1000);
        }
        eWaitState::WAITSTATE_CROSS_ROAD_LOOK => {
            ped.wait_timer = now_ms.saturating_add(time_ms.unwrap_or(3000).max(0) as u32);
        }
        eWaitState::WAITSTATE_DOUBLEBACK => {
            locomotion.heading_rate = 0.0;
            ped.wait_timer = now_ms.saturating_add(3500);
        }
        eWaitState::WAITSTATE_HITWALL => {
            locomotion.heading_rate = 2.0;
            ped.wait_timer = now_ms.saturating_add(5000);
        }
        eWaitState::WAITSTATE_TURN180 => {
            locomotion.heading_rate = 0.0;
            ped.wait_timer = now_ms.saturating_add(5000);
        }
        eWaitState::WAITSTATE_SURPRISE => {
            locomotion.heading_rate = 0.0;
            ped.wait_timer = now_ms.saturating_add(2000);
        }
        eWaitState::WAITSTATE_STUCK | eWaitState::WAITSTATE_LOOK_ABOUT => {
            ped.move_state = eMoveState::PEDMOVE_STILL;
            locomotion.heading_rate = 0.0;
            ped.wait_timer = now_ms.saturating_add(5000);
        }
        eWaitState::WAITSTATE_PLAYANIM_COWER
        | eWaitState::WAITSTATE_PLAYANIM_HANDSUP
        | eWaitState::WAITSTATE_PLAYANIM_HANDSCOWER
        | eWaitState::WAITSTATE_PLAYANIM_DUCK
        | eWaitState::WAITSTATE_PLAYANIM_TAXI
        | eWaitState::WAITSTATE_PLAYANIM_CHAT => {
            if matches!(
                wait_state,
                eWaitState::WAITSTATE_PLAYANIM_COWER
                    | eWaitState::WAITSTATE_PLAYANIM_HANDSUP
                    | eWaitState::WAITSTATE_PLAYANIM_HANDSCOWER
            ) {
                locomotion.heading_rate = 0.0;
            }
            ped.wait_timer = now_ms.saturating_add(time_ms.unwrap_or(3000).max(0) as u32);
        }
        eWaitState::WAITSTATE_FINISH_FLEE => {
            ped.move_state = eMoveState::PEDMOVE_STILL;
            locomotion.heading_rate = 0.0;
            ped.wait_timer = now_ms.saturating_add(2500);
        }
        eWaitState::WAITSTATE_LOOK_PED
        | eWaitState::WAITSTATE_LOOK_SHOP
        | eWaitState::WAITSTATE_LOOK_ACCIDENT
        | eWaitState::WAITSTATE_FACEOFF_GANG
        | eWaitState::WAITSTATE_FALSE => {}
    }
    ped.wait_state = wait_state;
}

fn weapon_slot(value: i32) -> Option<usize> {
    let slot = value as usize;
    (slot < WEAPONTYPE_TOTAL_INVENTORY_WEAPONS).then_some(slot)
}

pub(crate) fn weapon_clip_capacity(weapon: eWeaponType) -> i32 {
    match weapon {
        eWeaponType::WEAPONTYPE_COLT45 => 12,
        eWeaponType::WEAPONTYPE_UZI => 25,
        eWeaponType::WEAPONTYPE_SHOTGUN => 1000,
        eWeaponType::WEAPONTYPE_SNIPERRIFLE
        | eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
        | eWeaponType::WEAPONTYPE_GRENADE
        | eWeaponType::WEAPONTYPE_MOLOTOV => 1,
        eWeaponType::WEAPONTYPE_AK47 => 30,
        eWeaponType::WEAPONTYPE_M16 => 60,
        eWeaponType::WEAPONTYPE_FLAMETHROWER => 500,
        _ => 1,
    }
}

pub(crate) fn set_ped_ammo(
    ped: &mut CPed,
    weapon_type: i32,
    ammo: i32,
    info: &CWeaponInfoStore,
) {
    let Some(slot) = weapon_slot(weapon_type) else {
        return;
    };
    if let Some(type_) = script_weapon_type(weapon_type) {
        if ped.weapons[slot].weapon_type != type_ {
            ped.weapons[slot] = CWeapon::initialise_with_info(type_, ammo, info);
            ped.max_weapon_type_allowed += 1;
        } else {
            ped.weapons[slot].ammo_total = ammo;
        }
    }
}

pub(crate) fn add_ped_ammo(
    ped: &mut CPed,
    weapon_type: i32,
    ammo: i32,
    info: &CWeaponInfoStore,
) {
    let Some(slot) = weapon_slot(weapon_type) else {
        return;
    };
    if let Some(type_) = script_weapon_type(weapon_type) {
        if ped.weapons[slot].weapon_type != type_ {
            ped.weapons[slot] = CWeapon::initialise_with_info(type_, ammo, info);
            ped.max_weapon_type_allowed += 1;
        } else {
            ped.weapons[slot].ammo_total = ped.weapons[slot].ammo_total.wrapping_add(ammo);
        }
    }
}

pub(crate) fn give_ped_weapon(ped: &mut CPed, weapon_type: i32, ammo: i32) -> Option<eWeaponType> {
    // TODO(parity): CPed::GiveWeapon — remaining construction/cheat/garage
    // callers must provide the loaded CWeaponInfoStore.
    let type_ = grant_ped_weapon_inventory(ped, weapon_type, ammo)?;
    let weapon = &mut ped.weapons[type_ as usize];
    if weapon.ammo_total != 0 {
        weapon.ammo_in_clip = weapon.ammo_total.min(weapon_clip_capacity(type_));
    }
    Some(type_)
}

pub(crate) fn give_ped_weapon_with_info(
    ped: &mut CPed,
    weapon_type: i32,
    ammo: i32,
    info: &CWeaponInfoStore,
) -> Option<eWeaponType> {
    let type_ = grant_ped_weapon_inventory(ped, weapon_type, ammo)?;
    ped.weapons[type_ as usize].Reload(info);
    Some(type_)
}

/// CPed::GiveWeapon inventory accounting; callers own selection/model changes.
fn grant_ped_weapon_inventory(ped: &mut CPed, weapon_type: i32, ammo: i32) -> Option<eWeaponType> {
    let slot = weapon_slot(weapon_type)?;
    let type_ = script_weapon_type(weapon_type)?;
    if ped.weapons[slot].weapon_type != type_ {
        ped.weapons[slot] = CWeapon::initialise(type_, ammo);
        ped.max_weapon_type_allowed += 1;
    } else {
        // CPed::GiveWeapon adds uint32 ammo before applying the upper limit.
        ped.weapons[slot].ammo_total = (ped.weapons[slot].ammo_total as u32)
            .wrapping_add(ammo as u32)
            .min(99999) as i32;
    }
    if ped.weapons[slot].weapon_state == crate::weapons::WeaponType::eWeaponState::WEAPONSTATE_OUT_OF_AMMO {
        ped.weapons[slot].weapon_state = crate::weapons::WeaponType::eWeaponState::WEAPONSTATE_READY;
    }
    Some(type_)
}

pub(crate) fn ped_weapon_ammo_snapshot(ped: &CPed) -> [i32; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS] {
    let mut ammo = [0; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS];
    for (slot, weapon) in ped.weapons.iter().enumerate() {
        ammo[slot] = weapon.ammo_total;
    }
    ammo
}

/// Script-facing use of re3 `CPed::OurPedCanSeeThisOne` for the player target.
pub(crate) fn script_ped_can_see_player(
    ped_position: CVector,
    ped_forward: CVector,
    player_position: CVector,
    file_loader: &CFileLoader,
) -> bool {
    script_ped_can_see_entity(ped_position, ped_forward, player_position, file_loader)
}

/// re3 `CPed::OurPedCanSeeThisOne`: forward-half-plane and 40 m XY range
/// checks followed by a head-height static-world LOS ray.
pub(crate) fn script_ped_can_see_entity(
    ped_position: CVector,
    ped_forward: CVector,
    target_position: CVector,
    file_loader: &CFileLoader,
) -> bool {
    let delta = CVector::new(
        target_position.x - ped_position.x,
        target_position.y - ped_position.y,
        target_position.z - ped_position.z,
    );
    if delta.x * ped_forward.x + delta.y * ped_forward.y < 0.0 {
        return false;
    }
    if delta.x * delta.x + delta.y * delta.y >= 40.0 * 40.0 {
        return false;
    }
    let head = CVector::new(ped_position.x, ped_position.y, ped_position.z + 1.0);
    file_loader
        .world
        .process_line_of_sight_static_hit(
            [head.x, head.y, head.z],
            [target_position.x, target_position.y, target_position.z],
            &file_loader.model_info,
            None,
            false,
        )
        .is_none()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_ped_objective(
    ped: &mut CPed, vitality: &PedVitality,
    objective: i32,
    target_char: Option<i32>,
    target_car: Option<i32>,
    position: Option<CVector>,
    radius: f32,
    route: Option<(i32, i32)>,
    routes: &CPedRoutes,
    now_ms: u32,
    current_vehicle: Option<usize>,
) {
    ped.flags.script_objective_completed = false;
    let Some(objective) = script_objective(objective) else {
        ped.set_objective(vitality, eObjective::OBJECTIVE_NONE, now_ms);
        ped.ped_in_objective = None;
        ped.car_in_objective = None;
        return;
    };

    if let Some((route_point, route_type)) = route {
        let route_type = match route_type {
            1 => PedRouteType::PEDROUTE_STOP_WHEN_DONE,
            2 => PedRouteType::PEDROUTE_GO_BACKWARD_WHEN_DONE,
            3 => PedRouteType::PEDROUTE_GO_TO_START_WHEN_DONE,
            _ => return,
        };
        ped.set_objective_route(route_point as i16, route_type, routes, now_ms);
    } else if let Some(position) = position {
        ped.set_objective_position(
            objective,
            position,
            (radius > 0.0).then_some(radius),
            now_ms,
        );
    } else if let Some(handle) = target_char {
        ped.set_objective_entity(
            objective,
            CPoolHandle(handle).index(),
            now_ms,
            current_vehicle,
        );
    } else if let Some(handle) = target_car {
        ped.set_objective_entity(
            objective,
            CPoolHandle(handle).index(),
            now_ms,
            current_vehicle,
        );
    } else {
        ped.set_objective(vitality, objective, now_ms);
    }

    if objective == eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION {
        ped.ped_formation = script_formation(radius as i32);
    }
    if matches!(
        objective,
        eObjective::OBJECTIVE_WAIT_ON_FOOT | eObjective::OBJECTIVE_WAIT_IN_CAR
    ) {
        ped.flags.script_objective_completed = true;
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CPedRuntime {
    pub ped_type_initialised: bool,
    pub fight_data_loaded: bool,
    pub enter_exit_vehicle_anim_offsets_set: bool,
}

impl CPedRuntime {
    pub fn initialise() -> Self {
        Self {
            ped_type_initialised: false,
            fight_data_loaded: false,
            enter_exit_vehicle_anim_offsets_set: false,
        }
    }
}

impl Default for CPedRuntime {
    fn default() -> Self {
        Self::initialise()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Resource, Default)]
pub struct CPedAnimationOffsets {
    pub ped_dragged_out_car: CVector,
    pub ped_car_door: CVector,
    pub ped_car_door_low: CVector,
    pub ped_quick_dragged_out_car: CVector,
    pub ped_van_rear_door: CVector,
    pub ped_train_door: CVector,
}

impl CPedAnimationOffsets {
    /// `CPed::SetAnimOffsetForEnterOrExitVehicle`. re3 reads the last
    /// translated key of sequence zero from each standard animation.
    pub fn from_anim_manager(anim_manager: &CAnimManager) -> Option<Self> {
        Some(Self {
            ped_dragged_out_car: final_sequence_translation(anim_manager, ANIM_STD_JACKEDCAR_LHS)?,
            ped_car_door: final_sequence_translation(anim_manager, ANIM_STD_CAR_GET_IN_LHS)?,
            ped_car_door_low: final_sequence_translation(anim_manager, ANIM_STD_CAR_GET_IN_LO_LHS)?,
            ped_quick_dragged_out_car: final_sequence_translation(
                anim_manager,
                ANIM_STD_QUICKJACKED,
            )?,
            ped_van_rear_door: final_sequence_translation(
                anim_manager,
                ANIM_STD_VAN_GET_IN_REAR_LHS,
            )?,
            ped_train_door: final_sequence_translation(anim_manager, ANIM_STD_TRAIN_GETOUT)?,
        })
    }
}

fn final_sequence_translation(anim_manager: &CAnimManager, anim_id: i32) -> Option<CVector> {
    let association = anim_manager.get_anim_association(0, anim_id as usize)?;
    let sequence = association.hierarchy.sequences.first()?;
    if sequence.num_frames() == 0 {
        return Some(CVector::ZERO);
    }
    let translation = sequence
        .get_key_frame(sequence.num_frames() - 1)
        .map(|frame| frame.translation())
        .unwrap_or(crate::math::Vector::CVector::ZERO);
    Some(CVector::new(translation.x, translation.y, translation.z))
}

/// Seat index used by re3 `CPed::SetPedPositionInCar`: `None` is the driver,
/// otherwise the passenger slot. Occupancy identity comes from the ECS
/// `InVehicle` relation, never from a reverse slot list on `CVehicle`.
pub(crate) fn player_seat_index(seat: VehicleSeat) -> Option<usize> {
    match seat {
        VehicleSeat::Driver => None,
        VehicleSeat::Passenger(index) => Some(usize::from(index)),
    }
}

/// re3 `CPed::SetPedPositionInCar` (Ped.cpp:6991-7011) seat lookup: the ped
/// rides at the authored `ped_frontseat`/`ped_backseat` dummy of the vehicle
/// model, with the left-side seats mirrored on X. Missing dummies are a zero
/// offset in re3 (uninitialised CAR_POS slot), so the origin is the faithful
/// result for those models.
pub(crate) fn ped_seat_world_position<VehCore, VehVitality, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    vehicle: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    seat: Option<usize>,
    is_boat: bool,
    file_loader: &CFileLoader,
) -> CVector
where
    VehCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let is_bike =
        vehicle.vehicle_type == crate::modelinfo::VehicleModelInfo::eVehicleType::VEHICLE_TYPE_BIKE;
    let entity = &vehicle.physical.entity;
    let model_index = entity.model_index.max(0) as usize;
    let summary = file_loader
        .model_info
        .get_model_info(model_index)
        .and_then(|slot| match slot {
            ModelInfoSlot::Vehicle(model) => model.clump.base.clump_summary.as_ref(),
            _ => None,
        });
    let dummy = match seat {
        None | Some(0) => summary.and_then(|s| vehicle_dummy_model_position(s, "ped_frontseat")),
        Some(1) | Some(2) => summary.and_then(|s| vehicle_dummy_model_position(s, "ped_backseat")),
        Some(_) => summary.and_then(|s| vehicle_dummy_model_position(s, "ped_frontseat")),
    };
    let mut seat_local = dummy.unwrap_or(CVector::ZERO);
    match seat {
        None if !is_boat && !is_bike => seat_local.x = -seat_local.x,
        Some(1) => seat_local.x = -seat_local.x,
        _ => {}
    }
    entity.transform_point(seat_local)
}

/// re3 `CPed::SetPedPositionInCar` driver branch. This is deliberately
/// independent of road contact / ped placement: a seated ped inherits the
/// vehicle matrix and the authored `ped_frontseat` model-space position.
pub fn automobile_driver_world_position<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
) -> Option<CVector>
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let ModelInfoSlot::Vehicle(model) = file_loader
        .model_info
        .get_model_info(car.vehicle.core.physical.entity.model_index.max(0) as usize)?
    else {
        return None;
    };
    let summary = model.clump.base.clump_summary.as_ref()?;
    let mut seat = vehicle_dummy_model_position(summary, "ped_frontseat")?;
    if model.vehicle_type
        != crate::modelinfo::VehicleModelInfo::eVehicleType::VEHICLE_TYPE_BOAT as i32
        && model.vehicle_type
            != crate::modelinfo::VehicleModelInfo::eVehicleType::VEHICLE_TYPE_BIKE as i32
    {
        seat.x = -seat.x;
    }
    Some(car.vehicle.core.physical.entity.transform_point(seat))
}

/// re3 `CPed::SetPedPositionInCar`: inherit the complete vehicle matrix and
/// place the occupant on the authored front/back-seat dummy every frame.
pub fn vehicle_occupant_matrix<VehCore, VehVitality, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    file_loader: &CFileLoader,
    vehicle: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    seat: VehicleSeat,
) -> Option<crate::math::Matrix::CMatrix>
where
    VehCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let ModelInfoSlot::Vehicle(model) = file_loader
        .model_info
        .get_model_info(vehicle.physical.entity.model_index.max(0) as usize)?
    else {
        return None;
    };
    let summary = model.clump.base.clump_summary.as_ref();
    // Native CAR_POS slots are zero-initialised. A model with no authored
    // seat dummy therefore keeps the occupant at the vehicle origin rather
    // than leaving the ped's previous world transform behind.
    let front = summary
        .and_then(|summary| vehicle_dummy_model_position(summary, "ped_frontseat"))
        .unwrap_or(CVector::ZERO);
    let back = summary
        .and_then(|summary| vehicle_dummy_model_position(summary, "ped_backseat"))
        .unwrap_or(front);
    let seat_position = if seat == VehicleSeat::Driver {
        let mut seat = front;
        if model.vehicle_type
            != crate::modelinfo::VehicleModelInfo::eVehicleType::VEHICLE_TYPE_BOAT as i32
            && model.vehicle_type
                != crate::modelinfo::VehicleModelInfo::eVehicleType::VEHICLE_TYPE_BIKE as i32
        {
            seat.x = -seat.x;
        }
        seat
    } else {
        match seat {
            VehicleSeat::Passenger(0) => front,
            VehicleSeat::Passenger(1) => {
                let mut seat = back;
                seat.x = -seat.x;
                seat
            }
            VehicleSeat::Passenger(2) => back,
            VehicleSeat::Passenger(_) => front,
            VehicleSeat::Driver => unreachable!(),
        }
    };

    let mut matrix = vehicle.physical.entity.matrix;
    matrix.position = matrix.transform_point(seat_position);
    if vehicle.flags.is_van {
        let angle = match seat {
            VehicleSeat::Passenger(1) => -std::f32::consts::FRAC_PI_2,
            VehicleSeat::Passenger(2) => std::f32::consts::FRAC_PI_2,
            _ => 0.0,
        };
        if angle != 0.0 {
            let mut rotation = crate::math::Matrix::CMatrix::unity();
            rotation.set_rotate_z(angle);
            matrix = matrix.multiply(&rotation);
        }
    }
    Some(matrix)
}

pub fn automobile_occupant_matrix<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    seat: VehicleSeat,
) -> Option<crate::math::Matrix::CMatrix>
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    vehicle_occupant_matrix(file_loader, &car.vehicle, seat)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eFormation {
    FORMATION_UNDEFINED = 0,
    FORMATION_REAR = 1,
    FORMATION_REAR_LEFT = 2,
    FORMATION_REAR_RIGHT = 3,
    FORMATION_FRONT_LEFT = 4,
    FORMATION_FRONT_RIGHT = 5,
    FORMATION_LEFT = 6,
    FORMATION_RIGHT = 7,
    FORMATION_FRONT = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PedFightMoves {
    FIGHTMOVE_NULL = 0,
    FIGHTMOVE_STDPUNCH = 1,
    FIGHTMOVE_IDLE = 2,
    FIGHTMOVE_SHUFFLE_F = 3,
    FIGHTMOVE_KNEE = 4,
    FIGHTMOVE_HEADBUTT = 5,
    FIGHTMOVE_PUNCHJAB = 6,
    FIGHTMOVE_PUNCHHOOK = 7,
    FIGHTMOVE_KICK = 8,
    FIGHTMOVE_LONGKICK = 9,
    FIGHTMOVE_ROUNDHOUSE = 10,
    FIGHTMOVE_BODYBLOW = 11,
    FIGHTMOVE_GROUNDKICK = 12,
    FIGHTMOVE_HITFRONT = 13,
    FIGHTMOVE_HITBACK = 14,
    FIGHTMOVE_HITRIGHT = 15,
    FIGHTMOVE_HITLEFT = 16,
    FIGHTMOVE_HITBODY = 17,
    FIGHTMOVE_HITCHEST = 18,
    FIGHTMOVE_HITHEAD = 19,
    FIGHTMOVE_HITBIGSTEP = 20,
    FIGHTMOVE_HITONFLOOR = 21,
    FIGHTMOVE_HITBEHIND = 22,
    FIGHTMOVE_IDLE2NORM = 23,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eWaitState {
    WAITSTATE_FALSE = 0,
    WAITSTATE_TRAFFIC_LIGHTS = 1,
    WAITSTATE_CROSS_ROAD = 2,
    WAITSTATE_CROSS_ROAD_LOOK = 3,
    WAITSTATE_LOOK_PED = 4,
    WAITSTATE_LOOK_SHOP = 5,
    WAITSTATE_LOOK_ACCIDENT = 6,
    WAITSTATE_FACEOFF_GANG = 7,
    WAITSTATE_DOUBLEBACK = 8,
    WAITSTATE_HITWALL = 9,
    WAITSTATE_TURN180 = 10,
    WAITSTATE_SURPRISE = 11,
    WAITSTATE_STUCK = 12,
    WAITSTATE_LOOK_ABOUT = 13,
    WAITSTATE_PLAYANIM_DUCK = 14,
    WAITSTATE_PLAYANIM_COWER = 15,
    WAITSTATE_PLAYANIM_TAXI = 16,
    WAITSTATE_PLAYANIM_HANDSUP = 17,
    WAITSTATE_PLAYANIM_HANDSCOWER = 18,
    WAITSTATE_PLAYANIM_CHAT = 19,
    WAITSTATE_FINISH_FLEE = 20,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eObjective {
    OBJECTIVE_NONE = 0,
    OBJECTIVE_WAIT_ON_FOOT = 1,
    OBJECTIVE_FLEE_ON_FOOT_TILL_SAFE = 2,
    OBJECTIVE_GUARD_SPOT = 3,
    OBJECTIVE_GUARD_AREA = 4,
    OBJECTIVE_WAIT_IN_CAR = 5,
    OBJECTIVE_WAIT_IN_CAR_THEN_GET_OUT = 6,
    OBJECTIVE_KILL_CHAR_ON_FOOT = 7,
    OBJECTIVE_KILL_CHAR_ANY_MEANS = 8,
    OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE = 9,
    OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS = 10,
    OBJECTIVE_GOTO_CHAR_ON_FOOT = 11,
    OBJECTIVE_FOLLOW_CHAR_IN_FORMATION = 12,
    OBJECTIVE_LEAVE_CAR = 13,
    OBJECTIVE_ENTER_CAR_AS_PASSENGER = 14,
    OBJECTIVE_ENTER_CAR_AS_DRIVER = 15,
    OBJECTIVE_FOLLOW_CAR_IN_CAR = 16,
    OBJECTIVE_FIRE_AT_OBJECT_FROM_VEHICLE = 17,
    OBJECTIVE_DESTROY_OBJECT = 18,
    OBJECTIVE_DESTROY_CAR = 19,
    OBJECTIVE_GOTO_AREA_ANY_MEANS = 20,
    OBJECTIVE_GOTO_AREA_ON_FOOT = 21,
    OBJECTIVE_RUN_TO_AREA = 22,
    OBJECTIVE_GOTO_AREA_IN_CAR = 23,
    OBJECTIVE_FOLLOW_CAR_ON_FOOT_WITH_OFFSET = 24,
    OBJECTIVE_GUARD_ATTACK = 25,
    OBJECTIVE_SET_LEADER = 26,
    OBJECTIVE_FOLLOW_ROUTE = 27,
    OBJECTIVE_SOLICIT_VEHICLE = 28,
    OBJECTIVE_HAIL_TAXI = 29,
    OBJECTIVE_CATCH_TRAIN = 30,
    OBJECTIVE_BUY_ICE_CREAM = 31,
    OBJECTIVE_STEAL_ANY_CAR = 32,
    OBJECTIVE_MUG_CHAR = 33,
    OBJECTIVE_FLEE_CAR = 34,
}

pub const RANDOM_CHAR: u8 = 1;
pub const MISSION_CHAR: u8 = 2;
pub const SOUND_NO_SOUND: i16 = 172;

fn default_ped_stats_for_type(ped_type: ePedType) -> ePedStats {
    use ePedStats::*;
    use ePedType::*;

    match ped_type {
        PEDTYPE_PLAYER1 | PEDTYPE_PLAYER2 | PEDTYPE_PLAYER3 | PEDTYPE_PLAYER4 => PEDSTAT_PLAYER,
        PEDTYPE_COP => PEDSTAT_COP,
        PEDTYPE_GANG1 => PEDSTAT_GANG1,
        PEDTYPE_GANG2 => PEDSTAT_GANG2,
        PEDTYPE_GANG3 => PEDSTAT_GANG3,
        PEDTYPE_GANG4 => PEDSTAT_GANG4,
        PEDTYPE_GANG5 => PEDSTAT_GANG5,
        PEDTYPE_GANG6 => PEDSTAT_GANG6,
        PEDTYPE_GANG7 | PEDTYPE_GANG8 | PEDTYPE_GANG9 => PEDSTAT_GANG7,
        PEDTYPE_FIREMAN => PEDSTAT_FIREMAN,
        PEDTYPE_CRIMINAL => PEDSTAT_CRIMINAL,
        PEDTYPE_PROSTITUTE => PEDSTAT_PROSTITUTE,
        PEDTYPE_EMERGENCY => PEDSTAT_MEDIC,
        _ => PEDSTAT_STREET_GUY,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PedState {
    PED_NONE = 0,
    PED_IDLE = 1,
    PED_LOOK_ENTITY = 2,
    PED_LOOK_HEADING = 3,
    PED_WANDER_RANGE = 4,
    PED_WANDER_PATH = 5,
    PED_SEEK_POS = 6,
    PED_SEEK_ENTITY = 7,
    PED_FLEE_POS = 8,
    PED_FLEE_ENTITY = 9,
    PED_PURSUE = 10,
    PED_FOLLOW_PATH = 11,
    PED_SNIPER_MODE = 12,
    PED_ROCKET_MODE = 13,
    PED_DUMMY = 14,
    PED_PAUSE = 15,
    PED_ATTACK = 16,
    PED_FIGHT = 17,
    PED_FACE_PHONE = 18,
    PED_MAKE_CALL = 19,
    PED_CHAT = 20,
    PED_MUG = 21,
    PED_AIM_GUN = 22,
    PED_AI_CONTROL = 23,
    PED_SEEK_CAR = 24,
    PED_SEEK_IN_BOAT = 25,
    PED_FOLLOW_ROUTE = 26,
    PED_CPR = 27,
    PED_SOLICIT = 28,
    PED_BUY_ICECREAM = 29,
    PED_INVESTIGATE = 30,
    PED_STEP_AWAY = 31,
    PED_ON_FIRE = 32,
    PED_UNKNOWN = 33,
    PED_STATES_NO_AI = 34,
    PED_JUMP = 35,
    PED_FALL = 36,
    PED_GETUP = 37,
    PED_STAGGER = 38,
    PED_DIVE_AWAY = 39,
    PED_STATES_NO_ST = 40,
    PED_ENTER_TRAIN = 41,
    PED_EXIT_TRAIN = 42,
    PED_ARREST_PLAYER = 43,
    PED_DRIVING = 44,
    PED_PASSENGER = 45,
    PED_TAXI_PASSENGER = 46,
    PED_OPEN_DOOR = 47,
    PED_DIE = 48,
    PED_DEAD = 49,
    PED_CARJACK = 50,
    PED_DRAG_FROM_CAR = 51,
    PED_ENTER_CAR = 52,
    PED_STEAL_CAR = 53,
    PED_EXIT_CAR = 54,
    PED_HANDS_UP = 55,
    PED_ARRESTED = 56,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eMoveState {
    PEDMOVE_NONE = 0,
    PEDMOVE_STILL = 1,
    PEDMOVE_WALK = 2,
    PEDMOVE_RUN = 3,
    PEDMOVE_SPRINT = 4,
}

/// re3 `CPed::SetSeekCar` uses a 0.5 m 2D seek completion radius before
/// `SeekCar` is allowed to transition into the enter-car state.
pub const SEEK_CAR_DONE_DISTANCE: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PedCarCollisionOutcome {
    None,
    Fell,
    Killed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PedWeaponDamageResult {
    Ignored,
    Applied,
    Killed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PedCarCollisionInput {
    pub vehicle_slot: usize,
    pub impulse: f32,
    pub vehicle_is_player: bool,
    pub vehicle_is_train: bool,
    pub current_surface_is_vehicle: bool,
    pub current_surface_is_boat: bool,
    pub vehicle_position: CVector,
    pub vehicle_move_speed: CVector,
    pub vehicle_right: CVector,
    pub vehicle_forward: CVector,
    pub vehicle_up: CVector,
    pub vehicle_mass: f32,
    pub vehicle_col_min: CVector,
    pub vehicle_col_max: CVector,
    pub damage_normal: CVector,
    pub ped_has_hit_wall: bool,
    pub player_vehicle_process_hit: bool,
    pub player_vehicle_speed_dot: f32,
    pub defend_weakness: f32,
    pub now_ms: u32,
    pub frame_counter: u32,
}

impl Default for PedCarCollisionInput {
    fn default() -> Self {
        Self {
            vehicle_slot: 0,
            impulse: 0.0,
            vehicle_is_player: false,
            vehicle_is_train: false,
            current_surface_is_vehicle: false,
            current_surface_is_boat: false,
            vehicle_position: CVector::ZERO,
            vehicle_move_speed: CVector::ZERO,
            vehicle_right: CVector::new(1.0, 0.0, 0.0),
            vehicle_forward: CVector::new(0.0, 1.0, 0.0),
            vehicle_up: CVector::new(0.0, 0.0, 1.0),
            vehicle_mass: 1_000.0,
            vehicle_col_min: CVector::new(-1.0, -2.0, -0.5),
            vehicle_col_max: CVector::new(1.0, 2.0, 1.0),
            damage_normal: CVector::ZERO,
            ped_has_hit_wall: false,
            player_vehicle_process_hit: false,
            player_vehicle_speed_dot: 0.0,
            defend_weakness: 1.0,
            now_ms: 0,
            frame_counter: 0,

        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PedCarCollisionResult {
    pub outcome: PedCarCollisionOutcome,
    pub damage: f32,
    pub damage_direction: u8,
    pub damage_piece: u8,
    pub kill_method: eWeaponType,
    pub animation_id: Option<i32>,
    pub vehicle_move_force: CVector,
    pub knocked_up_into_air: bool,
}

impl PedCarCollisionResult {
    fn none() -> Self {
        Self {
            outcome: PedCarCollisionOutcome::None,
            damage: 0.0,
            damage_direction: 0,
            damage_piece: 0,
            kill_method: eWeaponType::WEAPONTYPE_UNARMED,
            animation_id: None,
            vehicle_move_force: CVector::ZERO,
            knocked_up_into_air: false,
        }
    }
}

fn dot_vec(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn sub_vec(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale_vec(value: CVector, scale: f32) -> CVector {
    CVector::new(value.x * scale, value.y * scale, value.z * scale)
}

fn normalised(value: CVector) -> CVector {
    let magnitude = value.magnitude();
    if magnitude > 1.0e-6 {
        scale_vec(value, 1.0 / magnitude)
    } else {
        CVector::ZERO
    }
}

fn transform_vehicle_point_z(input: &PedCarCollisionInput, local: CVector) -> f32 {
    input.vehicle_position.z
        + input.vehicle_right.z * local.x
        + input.vehicle_forward.z * local.y
        + input.vehicle_up.z * local.z
}

fn car_death_animation(random: u8, damage_piece: u8, state: PedState) -> i32 {
    use crate::animation::AnimationId::{
        ANIM_STD_HIGHIMPACT_BACK, ANIM_STD_HIGHIMPACT_FRONT, ANIM_STD_HIGHIMPACT_LEFT,
        ANIM_STD_HIGHIMPACT_RIGHT, ANIM_STD_SPINFORWARD_LEFT, ANIM_STD_SPINFORWARD_RIGHT,
    };

    match random & 3 {
        0 => ANIM_STD_HIGHIMPACT_FRONT,
        1 => {
            if state == PedState::PED_DIVE_AWAY {
                ANIM_STD_SPINFORWARD_LEFT
            } else {
                ANIM_STD_HIGHIMPACT_LEFT
            }
        }
        2 => match damage_piece {
            2 => ANIM_STD_SPINFORWARD_LEFT,
            1 | 3 => ANIM_STD_SPINFORWARD_RIGHT,
            _ => ANIM_STD_HIGHIMPACT_BACK,
        },
        _ => {
            if state == PedState::PED_DIVE_AWAY {
                ANIM_STD_SPINFORWARD_RIGHT
            } else {
                ANIM_STD_HIGHIMPACT_RIGHT
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum PedRouteType {
    PEDROUTE_STOP_WHEN_DONE = 1,
    PEDROUTE_GO_BACKWARD_WHEN_DONE = 2,
    PEDROUTE_GO_TO_START_WHEN_DONE = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CPedFlags {
    pub is_standing: bool,
    pub was_standing: bool,
    pub is_attacking: bool,
    pub is_pointing_gun_at: bool,
    pub is_looking: bool,
    pub keep_trying_to_look: bool,
    pub is_restoring_look: bool,
    pub is_aiming_gun: bool,
    pub is_restoring_gun: bool,
    pub can_point_gun_at_target: bool,
    pub is_talking: bool,
    pub is_in_the_air: bool,
    pub is_landing: bool,
    pub is_running: bool,
    pub hit_something_last_frame: bool,
    pub veh_enter_door_is_blocked: bool,
    pub can_ped_enter_seeked_car: bool,
    pub responds_to_threats: bool,
    pub render_ped_in_car: bool,
    pub changed_seat: bool,
    pub update_anim_heading: bool,
    pub body_part_just_came_off: bool,
    pub is_shooting: bool,
    pub find_new_node_after_state_restore: bool,
    pub gonna_investigate_event: bool,
    pub ped_is_bleeding: bool,
    pub stop_and_shoot: bool,
    pub is_ped_die_anim_playing: bool,
    pub use_ped_node_seek: bool,
    pub objective_completed: bool,
    pub script_objective_completed: bool,
    pub kinda_stay_in_same_place: bool,
    pub being_chased_by_police: bool,
    pub not_allowed_to_duck: bool,
    pub crouch_when_shooting: bool,
    pub is_ducking: bool,
    pub get_up_anim_started: bool,
    pub do_bloody_footprints: bool,
    pub flee_after_exiting_car: bool,
    pub wander_path_after_exiting_car: bool,
    pub is_leader: bool,
    pub dont_drag_me_out_car: bool,
    pub ped_flag_f8: bool,
    pub will_be_quick_jacked: bool,
    pub cancel_entering_car: bool,
    pub obstacle_showed_up_during_kill_objective: bool,
    pub duck_and_cover: bool,
    pub still_on_valid_poly: bool,
    pub allow_medics_to_revive_me: bool,
    pub reset_walk_anims: bool,
    pub start_wander_path_on_foot: bool,
    pub on_boat: bool,
    pub bus_jacked: bool,
    pub gonna_kill_the_car_jacker: bool,
    pub fade_out: bool,
    pub knocked_up_into_air: bool,
    pub hit_steep_slope: bool,
    pub cull_extra_far_away: bool,
    pub clear_objective: bool,
    pub trying_to_reach_dry_land: bool,
    pub collided_with_my_vehicle: bool,
    pub rich_from_mugging: bool,
    pub chris_criminal: bool,
    pub shake_fist: bool,
    pub no_critical_hits: bool,
    pub veh_exit_will_be_instant: bool,
    pub has_already_been_recorded: bool,
    pub fallen_down: bool,
    pub has_a_camera: bool,
}

impl CPedFlags {
    pub fn new(general: &mut CGeneral) -> Self {
        Self {
            is_standing: false,
            was_standing: false,
            is_attacking: false,
            is_pointing_gun_at: false,
            is_looking: false,
            keep_trying_to_look: false,
            is_restoring_look: false,
            is_aiming_gun: false,
            is_restoring_gun: false,
            can_point_gun_at_target: false,
            is_talking: false,
            is_in_the_air: false,
            is_landing: false,
            is_running: false,
            hit_something_last_frame: false,
            veh_enter_door_is_blocked: false,
            can_ped_enter_seeked_car: false,
            responds_to_threats: true,
            render_ped_in_car: true,
            changed_seat: false,
            update_anim_heading: false,
            body_part_just_came_off: false,
            is_shooting: false,
            find_new_node_after_state_restore: false,
            gonna_investigate_event: false,
            ped_is_bleeding: false,
            stop_and_shoot: false,
            is_ped_die_anim_playing: false,
            use_ped_node_seek: false,
            objective_completed: false,
            script_objective_completed: false,
            kinda_stay_in_same_place: false,
            being_chased_by_police: false,
            not_allowed_to_duck: false,
            crouch_when_shooting: false,
            is_ducking: false,
            get_up_anim_started: false,
            do_bloody_footprints: false,
            flee_after_exiting_car: false,
            wander_path_after_exiting_car: false,
            is_leader: false,
            dont_drag_me_out_car: false,
            ped_flag_f8: false,
            will_be_quick_jacked: false,
            cancel_entering_car: false,
            obstacle_showed_up_during_kill_objective: false,
            duck_and_cover: false,
            still_on_valid_poly: false,
            allow_medics_to_revive_me: true,
            reset_walk_anims: false,
            start_wander_path_on_foot: false,
            on_boat: false,
            bus_jacked: false,
            gonna_kill_the_car_jacker: false,
            fade_out: false,
            knocked_up_into_air: false,
            hit_steep_slope: false,
            cull_extra_far_away: false,
            clear_objective: false,
            trying_to_reach_dry_land: false,
            collided_with_my_vehicle: false,
            rich_from_mugging: false,
            chris_criminal: false,
            shake_fist: false,
            no_critical_hits: false,
            veh_exit_will_be_instant: false,
            has_already_been_recorded: false,
            fallen_down: false,
            has_a_camera: general.get_random_number() & 3 == 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
#[require(PedAnimation, PedVitality, PedLocomotion, PedObstacleAvoidance, PedDeathLifecycle)]
pub struct CPed {
    pub physical: CPhysical,
    pub collision_speed: f32,
    pub ped_type: ePedType,
    /// re3 `CPed::m_pedStats->m_type`; SET_CHAR_PERSONALITY replaces this.
    pub ped_stats: ePedStats,
    /// re3 `CPed::m_animGroup` (`AssocGroupId`).
    pub anim_group: i32,
    pub last_sound_start: u32,
    pub sound_start: u32,
    pub last_queued_sound: i16,
    pub queued_sound: i16,
    pub objective: eObjective,
    pub prev_objective: eObjective,
    pub objective_timer: u32,
    pub char_created_by: u8,
    pub leader: Option<usize>,
    pub ped_in_objective: Option<usize>,
    pub car_in_objective: Option<usize>,
    pub offset_seek: CVector,
    pub ped_formation: eFormation,
    pub ped_state_timer: u32,
    pub action_x: f32,
    pub action_y: f32,
    pub phone_talk_timer: u32,
    pub state_unused: u32,
    pub leave_car_timer: u32,
    pub get_up_timer: u32,
    pub attack_timer: u32,
    pub timer_unused: u32,
    pub look_timer: u32,
    pub chat_timer: u32,
    pub shoot_timer: u32,
    pub car_jack_timer: u32,
    pub duck_and_cover_timer: u32,
    pub veh_door: i32,
    pub current_phys_surface: Option<Entity>,
    pub offset_from_phys_surface: CVector,
    pub seek_target: Option<usize>,
    pub seek_pos: CVector,
    pub wep_skills: u8,
    pub distance_to_count_seek_done: f32,
    pub running_to_phone: bool,
    pub phone_id: i32,
    pub last_accident: Option<usize>,
    pub flee_from: Option<usize>,
    pub flee_from_pos_x: f32,
    pub flee_from_pos_y: f32,
    pub flee_timer: u32,
    pub seek_pos_ex: CVector,
    pub distance_to_count_seek_done_ex: f32,
    pub wait_state: eWaitState,
    pub wait_timer: u32,
    pub colliding_entity: Option<usize>,
    pub ped_state: PedState,
    pub last_ped_state: PedState,
    pub move_state: eMoveState,
    pub stored_move_state: eMoveState,
    pub point_gun_at: Option<usize>,
    pub look_target: Option<usize>,
    pub look_direction: f32,
    pub cur_surface: Option<usize>,
    pub wander_range_bounds: Option<usize>,
    pub path_nodes: i16,
    pub cur_path_node: i16,
    pub path_dir: i16,
    pub last_path_node: Option<usize>,
    pub next_path_node: Option<usize>,
    pub route_last_point: i16,
    pub route_start_point: u16,
    pub route_points_passed: i16,
    pub route_type: i16,
    pub route_points_being_passed: i16,
    pub next_route_point_pos: CVector,
    pub flags: CPedFlags,
    pub queued_footstep: Option<u16>,
    pub fear_flags: u32,
    pub threat_entity: Option<usize>,
    pub event_or_threat: CVector2D,
    pub event_entity: Option<usize>,
    pub angle_to_event: f32,
    /// re3 `m_numNearPeds` / `m_nearPeds`: canonical generation-bearing ped
    /// pool handles, refreshed by `CPed::BuildPedLists` cadence.
    pub num_near_peds: u8,
    pub near_peds: [Option<i32>; 10],
    /// CPed inventory ownership count, including the initial unarmed slot.
    pub max_weapon_type_allowed: i32,
    pub current_weapon: eWeaponType,
    pub stored_weapon: eWeaponType,
    pub weapons: [CWeapon; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
    pub cur_fight_move: PedFightMoves,
    /// re3 `m_fightButtonPressure` buffered from the attack button while a
    /// fight association is running.
    pub fight_button_pressure: u8,
    /// re3 `m_fightState` (`MOVE_FINISHED=-2`, `JUST_ATTACKED=-1`,
    /// `NO_MOVE=0`, `1=active strike`).
    pub fight_state: i8,
    /// re3 `m_takeAStepAfterAttack`, set by PlayerControlFighter when the
    /// stick is held while an attack is in progress.
    pub take_step_after_attack: bool,
    /// Local-player equivalent of re3's nPlayerInComboMove guard. It is kept
    /// per ped so the developer arena's two controllers cannot consume one
    /// another's combo window.
    pub fight_combo_move: bool,
    pub wep_accuracy: u8,
    pub coll_poly_valid: bool,
    pub weapon_model_id: i32,
    /// Script/NPC equivalent of the live attack association owned by re3's
    /// RpAnimBlend clump. The shared PedAnimation component owns the full association state;
    /// this metadata selects the active authored attack clip/time.
    pub weapon_attack_animation_id: Option<i32>,
    pub weapon_attack_animation_start_ms: u32,
    /// Live non-looping association selected by SetFall/SetDie. Render-side
    /// sampling uses these values for both player and script-owned peds.
    pub impact_animation_id: Option<i32>,
    pub impact_animation_start_ms: u32,
    pub impact_animation_speed: f32,
    pub impact_animation_blend_delta: f32,
    /// One accepted SetFall command; consumed by the actor's canonical clump.
    pub(crate) fall_animation_request: Option<FallAnimationRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FallAnimationRequest {
    pub animation_id: i32,
    pub extra_time_ms: i32,
    pub now_ms: u32,
    pub frame_counter: u32,
}

pub(crate) fn blend_ped_movement_animation(
    clump: &mut CAnimBlendClumpData,
    manager: &CAnimManager,
    group: usize,
    animation_id: AnimationId,
    delta: f32,
) -> Option<usize> {
    let mut template = manager.create_anim_association(group, animation_id as usize)?;
    let is_movement = template.is_movement();
    let is_partial = template.is_partial();
    let mut found = None;
    let mut movement_sync = None;
    let mut removed_previous = false;

    for (index, association) in clump.associations.iter_mut().enumerate() {
        if is_movement && association.is_movement() {
            movement_sync = Some((association.current_time, association.hierarchy.total_length));
        }
        if association.anim_id == animation_id {
            found = Some(index);
            continue;
        }
        if association.is_partial() == is_partial {
            if association.blend_amount > 0.0 {
                let blend_delta = -delta * association.blend_amount;
                if blend_delta < association.blend_delta || !is_partial {
                    association.blend_delta = blend_delta;
                }
            } else {
                association.blend_delta = -1.0;
            }
            association.flags |= ASSOC_DELETEFADEDOUT;
            removed_previous = true;
        }
    }

    if let Some(index) = found {
        let association = &mut clump.associations[index];
        association.blend_delta = (1.0 - association.blend_amount) * delta;
        if !association.is_running()
            && association.current_time == association.hierarchy.total_length
        {
            association.start(0.0);
        }
        return Some(index);
    }

    if is_movement {
        if let Some((sync_time, sync_length)) = movement_sync {
            if sync_length > 0.0 && template.hierarchy.total_length > 0.0 {
                template.set_current_time(
                    sync_time / sync_length * template.hierarchy.total_length,
                );
            }
            template.set_run();
        } else {
            template.start(0.0);
        }
    } else {
        template.start(0.0);
    }

    if !removed_previous && !is_partial {
        template.blend_amount = 1.0;
    } else {
        template.blend_amount = 0.0;
        template.blend_delta = delta;
    }
    clump.associations.insert(0, template);
    Some(0)
}

fn ped_movement_animation_group(
    ped: &CPed,
    manager: &CAnimManager,
    file_loader: &CFileLoader,
    leader_is_player: bool,
) -> usize {
    if leader_is_player {
        return ASSOCGRP_PLAYER;
    }
    if ped.anim_group > ASSOCGRP_STD as i32 {
        return ped.anim_group as usize;
    }
    let model_index = ped.physical.entity.model_index.max(0) as usize;
    file_loader
        .model_info
        .get_model_info(model_index)
        .and_then(|model| match model {
            ModelInfoSlot::Ped(model) => manager.assoc_group_id_by_name(&model.anim_group),
            _ => None,
        })
        .unwrap_or(ASSOCGRP_STD)
}

/// CPed::SetMoveAnim association changes, without advancing the clump.
pub(crate) fn set_ped_move_animation(
    ped: &mut CPed,
    vitality: &PedVitality,
    animation: &mut super::PedAnimation::PedAnimation,
    manager: &CAnimManager,
    file_loader: &CFileLoader,
    leader_is_player: bool,
    leader_animation_speed: Option<f32>,
) {
    let group = ped_movement_animation_group(ped, manager, file_loader, leader_is_player);
    let requested_state = ped.move_state;
    let can_change_move_anim = ped.is_ped_in_control(vitality);
    let random_seed = ped.physical.entity.random_seed;
    let char_created_by = ped.char_created_by;
    let ped_state = ped.ped_state;

    let clump = &mut animation.movement_anim_clump;

    if ped.stored_move_state != requested_state && can_change_move_anim {
        if requested_state == eMoveState::PEDMOVE_NONE {
            ped.stored_move_state = eMoveState::PEDMOVE_NONE;
        } else {
            ped.stored_move_state = requested_state;
            let request = match requested_state {
                eMoveState::PEDMOVE_STILL => Some((ANIM_STD_IDLE, 4.0)),
                eMoveState::PEDMOVE_WALK => Some((ANIM_STD_WALK, 1.0)),
                eMoveState::PEDMOVE_RUN => Some((
                    ANIM_STD_RUN,
                    if ped_state == PedState::PED_FLEE_ENTITY || ped_state == PedState::PED_ON_FIRE {
                        3.0
                    } else {
                        1.0
                    },
                )),
                eMoveState::PEDMOVE_SPRINT => Some((
                    ANIM_STD_RUNFAST,
                    if ped_state == PedState::PED_ON_FIRE {
                        3.0
                    } else {
                        1.0
                    },
                )),
                _ => None,
            };
            if let Some((animation_id, blend_delta)) = request {
                if let Some(index) = blend_ped_movement_animation(
                    clump,
                    manager,
                    group,
                    animation_id,
                    blend_delta,
                ) {
                    clump.associations[index].speed =
                        leader_animation_speed.unwrap_or_else(|| {
                            if char_created_by == MISSION_CHAR {
                                1.0
                            } else {
                                1.2
                                    - f32::from(random_seed) * 0.4
                                        / f32::from(CGeneral::MYRAND_MAX)
                            }
                        });
                } else {
                    bevy::log::warn_once!(
                        target: "re3bevy::ped_animation",
                        "missing locomotion association {animation_id} in group {group}"
                    );
                }
            }
        }
    }

}

/// Advance a non-player CPed's authored locomotion association and return the
/// root translation produced for this frame.
///
/// This mirrors CPed::SetMoveAnim plus RpAnimBlendClumpUpdateAnimations:
/// movement speed is the IFP root motion, including the original
/// mission/random-ped association speed rules.
pub(crate) fn update_non_player_movement_animation(
    ped: &mut CPed,
    locomotion: &mut crate::peds::PedLocomotion::PedLocomotion, vitality: &PedVitality,
    animation: &mut super::PedAnimation::PedAnimation,
    manager: &CAnimManager,
    file_loader: &CFileLoader,
    dt: f32,
    leader_is_player: bool,
    leader_animation_speed: Option<f32>,
) -> CVector2D {
    if !animation.ensure_original_frames(ped.physical.entity.model_index.max(0) as usize, file_loader) {
        locomotion.anim_move_delta = CVector2D::new(0.0, 0.0);
        return locomotion.anim_move_delta;
    }
    set_ped_move_animation(ped, vitality, animation, manager, file_loader,
        leader_is_player, leader_animation_speed);
    let clump = &mut animation.movement_anim_clump;

    let mut index = 0;
    while index < clump.associations.len() {
        if clump.associations[index].update_blend(dt) {
            index += 1;
        } else {
            clump.associations.remove(index);
        }
    }
    let transform =
        frame_update_callback_with_velocity_extraction_non_skinned(clump, 0, true);

    let mut total_length = 0.0;
    let mut total_blend = 0.0;
    for association in &clump.associations {
        if association.is_movement() && association.speed != 0.0 {
            total_length += association.hierarchy.total_length / association.speed
                * association.blend_amount;
            total_blend += association.blend_amount;
        }
    }
    let relative_speed = if total_length == 0.0 {
        1.0
    } else {
        total_blend / total_length
    };
    for association in &mut clump.associations {
        association.update_time(dt, relative_speed);
        if matches!(association.anim_id,
            crate::animation::AnimationId::ANIM_STD_FALL_LAND
            | crate::animation::AnimationId::ANIM_STD_FALL_COLLAPSE)
            && association.callback_events.contains(
                &crate::animation::AnimBlendAssociation::AnimBlendCallbackEvent::Finish,
            )
        {
            // CPed::PedLandCB runs on this same live association.
            association.callback_events.clear();
            association.blend_delta = -1000.0;
            ped.flags.is_landing = false;
            if ped.ped_state == PedState::PED_JUMP {
                ped.ped_state = ped.last_ped_state;
                ped.last_ped_state = PedState::PED_NONE;
            }
        }
    }

    let (x, y) = transform.velocity2d.unwrap_or((0.0, 0.0));
    locomotion.anim_move_delta = CVector2D::new(x, y);
    locomotion.anim_move_delta
}

impl CPed {
    /// CPed::ClearWeapons: retire the hand model and clear each inventory slot.
    /// Native m_bAddRotOffset survives this operation.
    pub(crate) fn clear_weapons(&mut self) {
        self.weapon_model_id = -1;
        self.max_weapon_type_allowed = eWeaponType::WEAPONTYPE_BASEBALLBAT as i32;
        self.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;
        for weapon in &mut self.weapons {
            weapon.weapon_type = eWeaponType::WEAPONTYPE_UNARMED;
            weapon.weapon_state = crate::weapons::WeaponType::eWeaponState::WEAPONSTATE_READY;
            weapon.ammo_in_clip = 0;
            weapon.ammo_total = 0;
            weapon.timer = 0;
        }
    }

    pub fn new(ped_type: ePedType, general: &mut CGeneral) -> Self {
        let mut physical = CPhysical::new(general);
        physical.entity.set_type(eEntityType::ENTITY_TYPE_PED);
        physical.entity.flags.ped_physics = true;
        physical.entity.flags.use_collision_records = true;
        physical.mass = 70.0;
        physical.turn_mass = 100.0;
        physical.air_resistance = 0.4 / physical.mass;
        physical.elasticity = 0.05;
        physical.buoyancy = 100.0 / 100.0 * GRAVITY * physical.mass;

        let mut weapons = [CWeapon::empty_unarmed(); WEAPONTYPE_TOTAL_INVENTORY_WEAPONS];
        weapons[0] = CWeapon::initialise(eWeaponType::WEAPONTYPE_UNARMED, 0);

        Self {
            physical,
            collision_speed: 0.0,
            ped_type,
            ped_stats: default_ped_stats_for_type(ped_type),
            anim_group: 0,
            last_sound_start: 0,
            sound_start: 0,
            last_queued_sound: SOUND_NO_SOUND,
            queued_sound: SOUND_NO_SOUND,
            objective: eObjective::OBJECTIVE_NONE,
            prev_objective: eObjective::OBJECTIVE_NONE,
            objective_timer: 0,
            char_created_by: RANDOM_CHAR,
            leader: None,
            ped_in_objective: None,
            car_in_objective: None,
            offset_seek: CVector::ZERO,
            ped_formation: eFormation::FORMATION_UNDEFINED,
            ped_state_timer: 0,
            action_x: 0.0,
            action_y: 0.0,
            phone_talk_timer: 0,
            state_unused: 0,
            leave_car_timer: 0,
            get_up_timer: 0,
            attack_timer: 0,
            timer_unused: 0,
            look_timer: 0,
            chat_timer: 0,
            shoot_timer: 0,
            car_jack_timer: 0,
            duck_and_cover_timer: 0,
            veh_door: 2,
            current_phys_surface: None,
            offset_from_phys_surface: CVector::ZERO,
            seek_target: None,
            seek_pos: CVector::ZERO,
            wep_skills: 0,
            distance_to_count_seek_done: 1.0,
            running_to_phone: false,
            phone_id: -1,
            last_accident: None,
            flee_from: None,
            flee_from_pos_x: 0.0,
            flee_from_pos_y: 0.0,
            flee_timer: 0,
            seek_pos_ex: CVector::ZERO,
            distance_to_count_seek_done_ex: 0.0,
            wait_state: eWaitState::WAITSTATE_FALSE,
            wait_timer: 0,
            colliding_entity: None,
            ped_state: PedState::PED_IDLE,
            last_ped_state: PedState::PED_NONE,
            move_state: eMoveState::PEDMOVE_STILL,
            stored_move_state: eMoveState::PEDMOVE_NONE,
            point_gun_at: None,
            look_target: None,
            look_direction: 0.0,
            cur_surface: None,
            wander_range_bounds: None,
            path_nodes: 0,
            cur_path_node: 0,
            path_dir: 0,
            last_path_node: None,
            next_path_node: None,
            route_last_point: -1,
            route_start_point: 0,
            route_points_passed: 0,
            route_type: 0,
            route_points_being_passed: 0,
            next_route_point_pos: CVector::ZERO,
            flags: CPedFlags::new(general),
            queued_footstep: None,
            fear_flags: 0,
            threat_entity: None,
            event_or_threat: CVector2D::new(0.0, 0.0),
            event_entity: None,
            angle_to_event: 0.0,
            num_near_peds: 0,
            near_peds: [None; 10],
            max_weapon_type_allowed: 1,
            current_weapon: eWeaponType::WEAPONTYPE_UNARMED,
            stored_weapon: eWeaponType::WEAPONTYPE_UNIDENTIFIED,
            weapons,
            cur_fight_move: PedFightMoves::FIGHTMOVE_NULL,
            fight_button_pressure: 0,
            fight_state: -2,
            take_step_after_attack: false,
            fight_combo_move: false,
            wep_accuracy: 60,
            coll_poly_valid: false,
            weapon_model_id: -1,
            weapon_attack_animation_id: None,
            weapon_attack_animation_start_ms: 0,
            impact_animation_id: None,
            impact_animation_start_ms: 0,
            impact_animation_speed: 1.0,
            impact_animation_blend_delta: 0.0,
            fall_animation_request: None,
        }
    }

    /// Constructor tail in re3: `m_fearFlags = CPedType::GetThreats(m_nPedType)`.
    /// Kept separate from allocation so ECS callers can borrow the ped-type
    /// resource without introducing global mutable state.
    pub fn initialise_fear_flags(&mut self, ped_types: &crate::peds::PedType::CPedTypeRuntime) {
        self.fear_flags = ped_types.GetThreats(self.ped_type as usize);
    }

    pub fn is_player(&self) -> bool {
        matches!(
            self.ped_type,
            ePedType::PEDTYPE_PLAYER1
                | ePedType::PEDTYPE_PLAYER2
                | ePedType::PEDTYPE_PLAYER3
                | ePedType::PEDTYPE_PLAYER4
        )
    }

    pub fn dying_or_dead(&self) -> bool {
        matches!(self.ped_state, PedState::PED_DIE | PedState::PED_DEAD)
    }

    /// CPed::SetDie state/request half. PedDeath services the same canonical
    /// clump, entry relationship, native speech and completion barrier.
    pub fn set_die(&mut self, vitality: &mut PedVitality,
        animation_id: Option<i32>, mut delta: f32, speed: f32, now_ms: u32) {
        self.threat_entity = None;
        if self.dying_or_dead() {
            return;
        }
        if matches!(self.ped_state, PedState::PED_FALL | PedState::PED_GETUP) {
            delta *= 0.5;
        }
        self.set_stored_state();
        clear_all_script_ped_state(self, vitality, now_ms);
        vitality.health = 0.0;
        if self.ped_state == PedState::PED_DRIVING && !self.is_player() {
            self.physical.entity.flags.remove_from_world = true;
        }
        self.ped_state = PedState::PED_DIE;
        self.flags.is_ped_die_anim_playing = animation_id.is_some();
        self.impact_animation_id = animation_id;
        self.impact_animation_start_ms = now_ms;
        self.impact_animation_blend_delta = delta;
        self.impact_animation_speed = speed;
        vitality.bloody_footprint_count_or_death_time = now_ms;
    }

    /// Compatibility entry point for parity callers that only exercise re3's
    /// non-player impulse threshold.
    pub fn kill_ped_with_car(&mut self, locomotion: &crate::peds::PedLocomotion::PedLocomotion, vitality: &mut PedVitality, vehicle_slot: usize, impulse: f32, is_train: bool) -> bool {
        let mut general = CGeneral::default();
        self.kill_ped_with_car_detailed(locomotion, vitality, 
            PedCarCollisionInput {
                vehicle_slot,
                impulse,
                vehicle_is_train: is_train,
                ..Default::default()
            },
            &mut general,
        )
        .outcome
            == PedCarCollisionOutcome::Killed
    }

    /// re3 `CPed::GetLocalDirection`: 0 forward, 1 left, 2 backward, 3 right.
    pub fn get_local_direction(&self, heading: f32, offset: CVector) -> u8 {
        let mut angle = offset.x.atan2(offset.y) - heading + std::f32::consts::FRAC_PI_4;
        while angle < 0.0 {
            angle += std::f32::consts::TAU;
        }
        ((angle.to_degrees() / 90.0) as i32).rem_euclid(4) as u8
    }

    /// Full state/force half of re3 `CPed::KillPedWithCar`. Model-dependent
    /// bonnet detachment and audio/event emission remain owned by the caller.
    pub fn kill_ped_with_car_detailed(
        &mut self, locomotion: &crate::peds::PedLocomotion::PedLocomotion, vitality: &mut PedVitality,
        input: PedCarCollisionInput,
        general: &mut CGeneral,
    ) -> PedCarCollisionResult {
        use crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_FRONT;

        let mut result = PedCarCollisionResult::none();
        if matches!(self.ped_state, PedState::PED_FALL | PedState::PED_DIE) {
            if self.colliding_entity.is_none() || input.vehicle_is_player {
                self.colliding_entity = Some(input.vehicle_slot);
            }
            return result;
        }
        if self.ped_state == PedState::PED_DEAD {
            return result;
        }
        if input.current_surface_is_vehicle && (input.current_surface_is_boat || self.is_player()) {
            return result;
        }

        let mut dist = sub_vec(self.physical.entity.get_position(), input.vehicle_position);
        if (input.impulse > 12.0 || input.vehicle_is_train) && !self.is_player() {
            let mut damage_piece = 0_u8; // PEDPIECE_TORSO
            let mut damage_node = 0_u8; // PED_TORSO
            let mut kill_method = eWeaponType::WEAPONTYPE_RAMMEDBYCAR;
            let random_branch = (general.get_random_number() & 3) as u8;
            self.flags.is_standing = false;
            let mut damage_direction =
                self.get_local_direction(locomotion.rotation_cur,scale_vec(self.physical.move_speed, -1.0));
            let side_dot = dot_vec(dist, input.vehicle_right);

            if input.vehicle_is_train {
                kill_method = eWeaponType::WEAPONTYPE_RUNOVERBYCAR;
                damage_node = 2; // PED_HEAD
                damage_piece = 6; // PEDPIECE_HEAD
                self.physical.move_speed = scale_vec(input.vehicle_move_speed, 0.9);
                self.physical.move_speed.z = 0.0;
                if matches!(damage_direction, 1 | 3) {
                    damage_direction = 2;
                }
            } else if dot_vec(input.vehicle_move_speed, input.vehicle_forward) >= 0.0 {
                if 0.99 * input.vehicle_col_max.x < side_dot.abs() {
                    if side_dot <= 0.0 {
                        damage_node = 3; // PED_UPPERARML
                        damage_piece = 2; // PEDPIECE_LEFTARM
                    } else {
                        damage_node = 4; // PED_UPPERARMR
                        damage_piece = 3; // PEDPIECE_RIGHTARM
                    }
                    if dot_vec(dist, input.vehicle_forward).abs() < 0.85 * input.vehicle_col_max.y {
                        kill_method = eWeaponType::WEAPONTYPE_RUNOVERBYCAR;
                        self.physical.move_speed = scale_vec(input.vehicle_move_speed, 0.9);
                        self.physical.move_speed.z = 0.0;
                        if matches!(damage_direction, 1 | 3) {
                            damage_direction = 2;
                        }
                    }
                } else {
                    let front_dot = dot_vec(dist, input.vehicle_forward);
                    if (front_dot <= 0.1 || random_branch == 1) && random_branch != 0 {
                        kill_method = eWeaponType::WEAPONTYPE_RUNOVERBYCAR;
                        damage_node = 2;
                        damage_piece = 6;
                        self.physical.move_speed = scale_vec(input.vehicle_move_speed, 0.9);
                        self.physical.move_speed.z = 0.0;
                        if matches!(damage_direction, 1 | 3) {
                            damage_direction = 2;
                        }
                    } else {
                        damage_node = 1; // PED_MID
                        damage_piece = 1; // PEDPIECE_MID
                        let car_highest_z = if input.vehicle_forward.z < -0.2 {
                            transform_vehicle_point_z(
                                &input,
                                CVector::new(0.0, input.vehicle_col_min.y, input.vehicle_col_max.z),
                            )
                        } else {
                            transform_vehicle_point_z(
                                &input,
                                CVector::new(0.0, input.vehicle_col_max.y, input.vehicle_col_max.z),
                            )
                        };
                        let car_length = if input.vehicle_forward.z < -0.2 {
                            input.vehicle_col_max.y - input.vehicle_col_min.y
                        } else {
                            input.vehicle_col_max.y
                        };
                        let car_speed = input.vehicle_move_speed.magnitude();
                        if car_length.abs() > 1.0e-6 && car_speed > 1.0e-6 {
                            let jump_speed = (car_highest_z
                                - self.physical.entity.get_position().z)
                                / (car_length / car_speed);
                            let unknown = (f32::from(general.get_random_number() % 256) * 0.002
                                + 1.5)
                                * jump_speed;
                            let mut launch = normalised(input.vehicle_move_speed);
                            launch = scale_vec(launch, 0.2 * unknown);
                            launch.z += if matches!(damage_direction, 1 | 3) {
                                1.5 * unknown
                            } else {
                                unknown
                            };
                            self.physical.move_speed = launch;
                        }
                        damage_direction = (damage_direction + 2) & 3;
                    }
                }
            }

            let die_random = (general.get_random_number() & 3) as u8;
            let animation_id = car_death_animation(die_random, damage_piece, self.ped_state);
            let killed = self.apply_vehicle_damage(vitality, 
                1_000.0,
                input.defend_weakness,
                kill_method,
                animation_id,
                input.vehicle_move_speed.magnitude(),
                input.now_ms,
            );
            if self.dying_or_dead()
                && self.flags.is_ped_die_anim_playing
                && self.colliding_entity.is_none()
            {
                self.colliding_entity = Some(input.vehicle_slot);
            }
            self.flags.knocked_up_into_air = damage_node == 1;
            dist = normalised(dist);
            result.outcome = if killed {
                PedCarCollisionOutcome::Killed
            } else {
                PedCarCollisionOutcome::None
            };
            result.damage = 1_000.0;
            result.damage_direction = damage_direction;
            result.damage_piece = damage_piece;
            result.kill_method = kill_method;
            result.animation_id = Some(animation_id);
            result.vehicle_move_force = scale_vec(dist, -100.0);
            result.knocked_up_into_air = self.flags.knocked_up_into_air;
            return result;
        }

        let should_fall = input.player_vehicle_process_hit
            || (input.damage_normal.z < -0.8 && input.impulse > 3.0)
            || (input.impulse > 6.0 && (!self.is_player() || input.impulse > 10.0));
        if !should_fall {
            return result;
        }

        self.flags.is_standing = false;
        let fall_direction = self.get_local_direction(locomotion.rotation_cur,scale_vec(input.vehicle_move_speed, -1.0));
        let damage = if input.player_vehicle_process_hit && input.vehicle_is_player {
            input.player_vehicle_speed_dot * 1_000.0
        } else if self.is_player() && input.vehicle_is_train {
            150.0
        } else if input.player_vehicle_process_hit && input.vehicle_is_train {
            50.0
        } else {
            30.0
        };
        let animation_id = ANIM_STD_HIGHIMPACT_FRONT + i32::from(fall_direction);
        let killed = self.apply_vehicle_damage(vitality, 
            damage,
            input.defend_weakness,
            eWeaponType::WEAPONTYPE_RAMMEDBYCAR,
            animation_id,
            input.vehicle_move_speed.magnitude(),
            input.now_ms,
        );
        if !killed {
            self.set_fall(vitality, 1_000, animation_id, false,
                input.now_ms, input.frame_counter);
        }
        if matches!(
            self.ped_state,
            PedState::PED_FALL | PedState::PED_DIE | PedState::PED_DEAD
        ) && self.colliding_entity.is_none()
            && (!self.is_player()
                || input.ped_has_hit_wall
                || input.vehicle_is_train
                || input.damage_normal.z < -0.8)
        {
            self.colliding_entity = Some(input.vehicle_slot);
        }
        self.flags.knocked_up_into_air = false;
        if !input.vehicle_is_train && !input.ped_has_hit_wall {
            self.physical.move_speed = scale_vec(input.vehicle_move_speed, 0.75);
        }
        self.physical.move_speed.z = 0.0;
        dist = normalised(dist);
        result.outcome = if killed {
            PedCarCollisionOutcome::Killed
        } else {
            PedCarCollisionOutcome::Fell
        };
        result.damage = damage;
        result.damage_direction = fall_direction;
        result.damage_piece = 0;
        result.kill_method = eWeaponType::WEAPONTYPE_RAMMEDBYCAR;
        result.animation_id = Some(animation_id);
        result.vehicle_move_force = scale_vec(dist, -60.0);
        result
    }

    fn apply_vehicle_damage(
        &mut self, vitality: &mut PedVitality,
        damage: f32,
        defend_weakness: f32,
        method: eWeaponType,
        animation_id: i32,
        vehicle_speed: f32,
        now_ms: u32,
    ) -> bool {
        if self.physical.entity.flags.collision_proof || self.dying_or_dead() {
            return false;
        }
        let mut health_impact = if self.is_player() {
            damage * 0.33
        } else {
            damage * defend_weakness
        };
        if vitality.armour > 0.0 {
            let absorbed = health_impact.min(vitality.armour);
            vitality.armour -= absorbed;
            health_impact -= absorbed;
        }
        if health_impact > 0.0 {
            vitality.last_wep_dam = method as i8;
        }
        if vitality.health - health_impact >= 1.0 {
            vitality.health -= health_impact;
            return false;
        }
        let delta = if method == eWeaponType::WEAPONTYPE_RUNOVERBYCAR {
            12.0 * vehicle_speed + 4.0
        } else {
            8.0 * vehicle_speed + 4.0
        };
        let speed = if method == eWeaponType::WEAPONTYPE_RUNOVERBYCAR {
            16.0 * vehicle_speed + 1.0
        } else {
            1.0
        };
        self.set_die(vitality, Some(animation_id), delta, speed, now_ms);
        true
    }

    /// Damage/state half of re3 `CPed::InflictDamage` for weapon hits.
    ///
    /// Hit reactions remain with the weapon/world adapter because they depend
    /// on the collision point and active animation clump. This method owns the
    /// proof flags, player damage scale, ped-stat weakness, armour, last weapon
    /// damage and lethal animation selection shared by every weapon caller.
    pub(crate) fn inflict_weapon_damage(
        &mut self, vitality: &mut PedVitality,
        method: eWeaponType,
        damage: f32,
        defend_weakness: f32,
        damaged_by_player: bool,
        player_can_be_damaged: bool,
        now_ms: u32,
        direction: u8,
    ) -> PedWeaponDamageResult {
        if self.dying_or_dead()
            || (!self.physical.entity.flags.uses_collision
                && method != eWeaponType::WEAPONTYPE_DROWNING)
            || (self.is_player() && !player_can_be_damaged)
            || (self.physical.entity.flags.only_damaged_by_player && !damaged_by_player
                && !matches!(method, eWeaponType::WEAPONTYPE_DROWNING | eWeaponType::WEAPONTYPE_EXPLOSION))
        {
            return PedWeaponDamageResult::Ignored;
        }

        let proofed = match method {
            eWeaponType::WEAPONTYPE_UNARMED | eWeaponType::WEAPONTYPE_BASEBALLBAT => {
                self.physical.entity.flags.melee_proof
            }
            eWeaponType::WEAPONTYPE_COLT45
            | eWeaponType::WEAPONTYPE_UZI
            | eWeaponType::WEAPONTYPE_SHOTGUN
            | eWeaponType::WEAPONTYPE_AK47
            | eWeaponType::WEAPONTYPE_M16
            | eWeaponType::WEAPONTYPE_SNIPERRIFLE => self.physical.entity.flags.bullet_proof,
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
            | eWeaponType::WEAPONTYPE_GRENADE
            | eWeaponType::WEAPONTYPE_MOLOTOV
            | eWeaponType::WEAPONTYPE_EXPLOSION => self.physical.entity.flags.explosion_proof,
            eWeaponType::WEAPONTYPE_FLAMETHROWER => self.physical.entity.flags.fire_proof,
            eWeaponType::WEAPONTYPE_RAMMEDBYCAR
            | eWeaponType::WEAPONTYPE_RUNOVERBYCAR
            | eWeaponType::WEAPONTYPE_FALL => self.physical.entity.flags.collision_proof,
            _ => false,
        };
        if proofed {
            return PedWeaponDamageResult::Ignored;
        }

        let mut health_impact = if self.is_player() {
            damage * 0.33
        } else {
            damage * defend_weakness
        };
        if vitality.armour > 0.0 && method != eWeaponType::WEAPONTYPE_DROWNING {
            let absorbed = health_impact.min(vitality.armour);
            vitality.armour -= absorbed;
            health_impact -= absorbed;
        }
        if health_impact > 0.0 {
            vitality.last_wep_dam = method as i8;
        }
        if vitality.health - health_impact >= 1.0 {
            vitality.health -= health_impact;
            return PedWeaponDamageResult::Applied;
        }

        let death_animation = match method {
            eWeaponType::WEAPONTYPE_DROWNING => crate::animation::AnimationId::ANIM_STD_DROWN,
            eWeaponType::WEAPONTYPE_SHOTGUN
            | eWeaponType::WEAPONTYPE_UNARMED
            | eWeaponType::WEAPONTYPE_BASEBALLBAT
            | eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
            | eWeaponType::WEAPONTYPE_GRENADE
            | eWeaponType::WEAPONTYPE_MOLOTOV
            | eWeaponType::WEAPONTYPE_EXPLOSION
            | eWeaponType::WEAPONTYPE_FALL => {
                crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_FRONT + i32::from(direction & 3)
            }
            _ => crate::animation::AnimationId::ANIM_STD_KO_FRONT,
        };
        self.set_die(vitality, Some(death_animation), 4.0, 0.0, now_ms);
        PedWeaponDamageResult::Killed
    }

    /// re3 `CPed::SetGetUp` applies `WEAPONTYPE_RUNOVERBYCAR` damage to a
    /// player every frame that a vehicle still occupies the get-up probe.  It
    /// uses the normal player 0.33 damage scale and armour handling, so route
    /// the retry damage through the same vehicle-damage path instead of
    /// subtracting health ad-hoc in the controller.
    pub(crate) fn apply_getup_runover_damage(&mut self, vitality: &mut PedVitality, damage: f32, now_ms: u32) -> bool {
        self.apply_vehicle_damage(vitality, 
            damage,
            1.0,
            eWeaponType::WEAPONTYPE_RUNOVERBYCAR,
            crate::animation::AnimationId::ANIM_STD_HIT_FLOOR,
            0.0,
            now_ms,
        )
    }

    /// CPed::SetDead's canonical state/events. PedDeath owns the actor's
    /// completion side effects and installed-world pickup creation.
    pub fn set_dead(
        &mut self, vitality: &mut PedVitality,
        pool_ref: i32,
        now_ms: u32,
        events: &mut crate::core::EventList::CEventList,
    ) {
        use crate::core::EventList::{EEventEntity, EEventType};

        self.physical.entity.flags.uses_collision = false;
        vitality.health = 0.0;
        if self.ped_state == PedState::PED_DRIVING {
            self.physical.entity.flags.is_visible = false;
        }
        self.ped_state = PedState::PED_DEAD;
        self.colliding_entity = None;
        self.weapon_model_id = -1;
        self.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;
        let position = self.physical.entity.get_position();
        if events.register_entity_event(
            EEventType::InjuredPed,
            EEventEntity::Ped,
            pool_ref,
            None,
            position,
            250,
            now_ms,
        ).is_none() {
            bevy::log::warn_once!(target: "re3bevy::ped_death", "CPed::SetDead injured-ped event capacity exhausted");
        }
        vitality.bloody_footprint_count_or_death_time = now_ms;
        vitality.dead_bleeding = false;
        self.flags.do_bloody_footprints = false;
        self.flags.veh_exit_will_be_instant = false;
        if events.register_entity_event(
            EEventType::DeadPed,
            EEventEntity::Ped,
            pool_ref,
            None,
            position,
            1_000,
            now_ms,
        ).is_none() {
            bevy::log::warn_once!(target: "re3bevy::ped_death", "CPed::SetDead dead-ped event capacity exhausted");
        }
    }

    /// `CPed::IsPedInControl`: the `PED_STATES_NO_AI` boundary is intentional.
    pub fn is_ped_in_control(&self, vitality: &PedVitality) -> bool {
        (self.ped_state as u8) <= PedState::PED_STATES_NO_AI as u8
            && !self.flags.is_in_the_air
            && !self.flags.is_landing
            && vitality.health > 0.0
    }

    /// re3 `CPed::CanPedReturnToState` (`Ped.cpp:4129`).
    pub fn can_ped_return_to_state(&self) -> bool {
        (self.ped_state as u8) <= PedState::PED_STATES_NO_AI as u8
            && !matches!(
                self.ped_state,
                PedState::PED_AIM_GUN
                    | PedState::PED_ATTACK
                    | PedState::PED_FIGHT
                    | PedState::PED_STEP_AWAY
                    | PedState::PED_SNIPER_MODE
                    | PedState::PED_LOOK_ENTITY
            )
    }

    /// re3 `CPed::CanSetPedState` (`Ped.cpp:4136`).
    pub fn can_set_ped_state(&self) -> bool {
        !self.dying_or_dead()
            && self.ped_state != PedState::PED_ARRESTED
            && !matches!(
                self.ped_state,
                PedState::PED_ENTER_CAR | PedState::PED_CARJACK | PedState::PED_STEAL_CAR
            )
    }

    /// re3 `CPed::SetStoredState` (`Ped.cpp:654`).
    pub(crate) fn set_stored_state(&mut self) {
        if self.last_ped_state != PedState::PED_NONE || !self.can_ped_return_to_state() {
            return;
        }
        if self.ped_state == PedState::PED_WANDER_PATH {
            self.flags.find_new_node_after_state_restore = true;
            if matches!(
                self.move_state,
                eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL
            ) {
                self.move_state = eMoveState::PEDMOVE_WALK;
            }
        }
        self.last_ped_state = self.ped_state;
        if self.move_state as u8 >= self.stored_move_state as u8 {
            self.stored_move_state = self.move_state;
        }
    }

    /// re3 `CPed::SetFlee(CVector2D const&, int)` (`Ped.cpp:5601`).
    pub(crate) fn set_flee_from_position(
        &mut self, vitality: &PedVitality, locomotion: &mut crate::peds::PedLocomotion::PedLocomotion,
        from: CVector,
        duration_ms: u32,
        now_ms: u32,
    ) -> bool {
        if now_ms < self.ped_state_timer
            || !self.is_ped_in_control(vitality)
            || self.flags.kinda_stay_in_same_place
        {
            return false;
        }
        if self.ped_state != PedState::PED_FLEE_ENTITY {
            self.set_stored_state();
            self.ped_state = PedState::PED_FLEE_POS;
            self.move_state = eMoveState::PEDMOVE_RUN;
            self.flee_from_pos_x = from.x;
            self.flee_from_pos_y = from.y;
            self.flee_from = None;
        }
        self.flags.use_ped_node_seek = true;
        self.next_path_node = None;
        self.flee_timer = now_ms.wrapping_add(duration_ms);
        self.set_flee_rotation(locomotion, from);
        true
    }

    fn set_flee_rotation(&mut self, locomotion: &mut crate::peds::PedLocomotion::PedLocomotion, from: CVector) {
        let position = self.physical.entity.get_position();
        locomotion.rotation_dest = CGeneral::limit_radian_angle(CGeneral::get_radian_angle_between_points(
            position.x, position.y, from.x, from.y,
        ));
        if locomotion.rotation_cur - std::f32::consts::PI > locomotion.rotation_dest {
            locomotion.rotation_dest += std::f32::consts::TAU;
        } else if std::f32::consts::PI + locomotion.rotation_cur < locomotion.rotation_dest {
            locomotion.rotation_dest -= std::f32::consts::TAU;
        }
    }

    /// re3 `CPed::SetFall`. State mutation is immediately visible to callers;
    /// one actor-owned command resets/blends the canonical PedAnimation clump
    /// before its next advance or presentation. No caller guesses a clip length.
    pub fn set_fall(
        &mut self, vitality: &PedVitality,
        extra_time_ms: i32,
        animation_id: i32,
        even_if_not_in_control: bool,
        now_ms: u32,
        frame_counter: u32,
    ) -> bool {
        if !self.is_ped_in_control(vitality) && (!even_if_not_in_control || self.dying_or_dead()) {
            return false;
        }

        if self.flags.is_looking {
            self.flags.is_looking = false;
            self.flags.is_restoring_look = true;
            self.flags.shake_fist = false;
            self.look_timer = now_ms.wrapping_add(if self.is_player() { 2_000 } else { 4_000 });
        }
        if self.flags.is_aiming_gun {
            self.flags.is_aiming_gun = false;
            self.flags.is_restoring_gun = true;
            self.look_timer = 0;
        }
        // TODO(parity): ClearLookFlag/ClearAimFlag common PedIK flags and
        // CPlayerPed FPS-move heading. Their owners must expose those inputs.
        self.set_stored_state();
        self.ped_state = PedState::PED_FALL;
        self.flags.fallen_down = true;
        self.impact_animation_id = Some(animation_id);
        self.impact_animation_start_ms = now_ms;
        self.impact_animation_speed = 1.0;
        self.impact_animation_blend_delta = 8.0;

        self.fall_animation_request = Some(FallAnimationRequest {
            animation_id, extra_time_ms, now_ms, frame_counter,
        });
        // A required unavailable IFP remains an observable wait. The clump
        // consumer assigns the native deadline from the actual association.
        self.get_up_timer = u32::MAX;
        true
    }

    /// re3 CPed::CanStrafeOrMouseControl without the global FREE_CAM gate.
    /// The camera owns that option in Rust, matching the C++ call site.
    pub fn can_strafe_or_mouse_control(&self) -> bool {
        matches!(
            self.ped_state,
            PedState::PED_NONE
                | PedState::PED_IDLE
                | PedState::PED_FLEE_POS
                | PedState::PED_FLEE_ENTITY
                | PedState::PED_ATTACK
                | PedState::PED_FIGHT
                | PedState::PED_AIM_GUN
                | PedState::PED_JUMP
        )
    }

    /// re3 CPed::UseGroundColModel.
    pub fn use_ground_col_model(&self) -> bool {
        matches!(
            self.ped_state,
            PedState::PED_FALL | PedState::PED_DIVE_AWAY | PedState::PED_DIE | PedState::PED_DEAD
        )
    }

    /// CPed::OnGround is distinct from UseGroundColModel (which includes dive).
    pub(crate) fn on_ground(&self) -> bool {
        matches!(self.ped_state, PedState::PED_FALL | PedState::PED_DIE | PedState::PED_DEAD)
    }

    /// Camera-independent core of re3 `CPed::CalculateNewVelocity`.
    pub fn calculate_new_velocity(&mut self, locomotion: &mut crate::peds::PedLocomotion::PedLocomotion, vitality: &PedVitality, time_step: f32, player_attacking: bool) -> CVector2D {
        if self.is_ped_in_control(vitality) {
            let mut head_amount = locomotion.heading_rate.to_radians() * time_step;
            locomotion.rotation_cur = CGeneral::limit_radian_angle(locomotion.rotation_cur);
            let mut limited_dest = CGeneral::limit_radian_angle(locomotion.rotation_dest);
            if locomotion.rotation_cur - std::f32::consts::PI > limited_dest {
                limited_dest += std::f32::consts::TAU;
            } else if std::f32::consts::PI + locomotion.rotation_cur < limited_dest {
                limited_dest -= std::f32::consts::TAU;
            }
            if player_attacking && self.ped_state == PedState::PED_ATTACK {
                head_amount *= 0.25;
            }
            let needed_turn = limited_dest - locomotion.rotation_cur;
            if needed_turn <= head_amount {
                if needed_turn > -head_amount {
                    locomotion.rotation_cur += needed_turn;
                } else {
                    locomotion.rotation_cur -= head_amount;
                }
            } else {
                locomotion.rotation_cur += head_amount;
            }
        }

        let (sin, cos) = locomotion.rotation_cur.sin_cos();
        locomotion.moved.x = locomotion.anim_move_delta.x * cos - sin * locomotion.anim_move_delta.y;
        locomotion.moved.y = locomotion.anim_move_delta.y * cos + locomotion.anim_move_delta.x * sin;
        let scale = if time_step >= 0.01 {
            1.0 / time_step
        } else {
            1.0 / 100.0
        };
        locomotion.moved.x *= scale;
        locomotion.moved.y *= scale;
        locomotion.moved
    }

    pub fn set_objective_timer(&mut self, now_ms: u32, time_ms: u32) {
        if time_ms == 0 {
            self.objective_timer = 0;
        } else if now_ms > self.objective_timer {
            self.objective_timer = now_ms.wrapping_add(time_ms);
        }
    }

    /// CPed::SetShootTimer retains an active native firing burst.
    pub(crate) fn set_shoot_timer(&mut self, now_ms: u32, time_ms: u32) {
        if now_ms > self.shoot_timer {
            self.shoot_timer = now_ms.wrapping_add(time_ms);
        }
    }

    /// CPed::SetAttackTimer starts the next request after the firing burst.
    pub(crate) fn set_attack_timer(&mut self, now_ms: u32, time_ms: u32) {
        if now_ms > self.attack_timer {
            self.attack_timer = self.shoot_timer.max(now_ms).wrapping_add(time_ms);
        }
    }

    pub fn is_temporary_objective(objective: eObjective) -> bool {
        matches!(
            objective,
            eObjective::OBJECTIVE_LEAVE_CAR
                | eObjective::OBJECTIVE_SET_LEADER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
        )
    }

    pub fn set_stored_objective(&mut self) {
        if self.objective == self.prev_objective {
            return;
        }
        if matches!(
            self.objective,
            eObjective::OBJECTIVE_FLEE_ON_FOOT_TILL_SAFE
                | eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
                | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE
                | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS
                | eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION
                | eObjective::OBJECTIVE_LEAVE_CAR
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
                | eObjective::OBJECTIVE_RUN_TO_AREA
        ) {
            return;
        }
        self.prev_objective = self.objective;
    }

    pub fn force_stored_objective(&mut self, objective: eObjective) {
        if !matches!(
            objective,
            eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
        ) {
            self.prev_objective = self.objective;
            return;
        }
        if matches!(
            self.objective,
            eObjective::OBJECTIVE_FLEE_ON_FOOT_TILL_SAFE
                | eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE
                | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS
                | eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
                | eObjective::OBJECTIVE_RUN_TO_AREA
        ) {
            return;
        }
        self.prev_objective = self.objective;
    }

    /// Port of the entity-free `CPed::SetObjective(eObjective)` overload.
    pub fn set_objective(&mut self, vitality: &PedVitality, new_objective: eObjective, now_ms: u32) {
        if self.dying_or_dead() {
            return;
        }
        if new_objective == eObjective::OBJECTIVE_NONE {
            if matches!(
                self.objective,
                eObjective::OBJECTIVE_LEAVE_CAR
                    | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                    | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
            ) && !self.is_ped_in_control(vitality)
            {
                self.flags.start_wander_path_on_foot = true;
                return;
            }
            self.objective = eObjective::OBJECTIVE_NONE;
            self.prev_objective = eObjective::OBJECTIVE_NONE;
            return;
        }
        if self.prev_objective == new_objective && self.prev_objective != eObjective::OBJECTIVE_NONE
        {
            return;
        }
        self.set_objective_timer(now_ms, 0);
        if self.objective == new_objective {
            return;
        }
        if Self::is_temporary_objective(self.objective) {
            self.prev_objective = new_objective;
        } else {
            self.set_stored_objective();
            self.objective = new_objective;
        }
        self.flags.objective_completed = false;
        if new_objective == eObjective::OBJECTIVE_HAIL_TAXI {
            self.wait_timer = 0;
            self.ped_state = PedState::PED_IDLE;
            self.move_state = eMoveState::PEDMOVE_STILL;
        }
    }

    /// Handle-based counterpart of `CPed::SetObjective(eObjective, void*)`.
    /// Pool handles replace raw C++ pointers; the objective itself determines
    /// whether the handle belongs to a ped or vehicle pool.
    pub fn set_objective_entity(
        &mut self,
        new_objective: eObjective,
        entity: usize,
        now_ms: u32,
        current_vehicle: Option<usize>,
    ) {
        if self.dying_or_dead()
            || (self.prev_objective == new_objective
                && self.prev_objective != eObjective::OBJECTIVE_NONE)
        {
            return;
        }

        if self.objective == new_objective {
            let same_target = match new_objective {
                eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
                | eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION
                | eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS
                | eObjective::OBJECTIVE_GUARD_ATTACK => self.ped_in_objective == Some(entity),
                eObjective::OBJECTIVE_LEAVE_CAR | eObjective::OBJECTIVE_FLEE_CAR => true,
                eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_DESTROY_CAR
                | eObjective::OBJECTIVE_SOLICIT_VEHICLE
                | eObjective::OBJECTIVE_BUY_ICE_CREAM => self.car_in_objective == Some(entity),
                eObjective::OBJECTIVE_SET_LEADER => self.leader == Some(entity),
                _ => false,
            };
            if same_target {
                return;
            }
        } else if matches!(
            new_objective,
            eObjective::OBJECTIVE_LEAVE_CAR | eObjective::OBJECTIVE_FLEE_CAR
        ) && current_vehicle.is_none()
        {
            return;
        }

        self.set_objective_timer(now_ms, 0);
        self.flags.objective_completed = false;
        if Self::is_temporary_objective(self.objective)
            && !Self::is_temporary_objective(new_objective)
        {
            self.prev_objective = new_objective;
        } else {
            if self.objective != new_objective {
                if Self::is_temporary_objective(new_objective) {
                    self.force_stored_objective(new_objective);
                } else {
                    self.set_stored_objective();
                }
            }
            self.objective = new_objective;
        }

        match new_objective {
            eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
            | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
            | eObjective::OBJECTIVE_MUG_CHAR => {
                self.next_path_node = None;
                self.flags.use_ped_node_seek = false;
                self.seek_pos = CVector::ZERO;
                self.ped_in_objective = Some(entity);
                self.look_target = Some(entity);
            }
            eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE
            | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS
            | eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
            | eObjective::OBJECTIVE_GUARD_ATTACK => {
                self.seek_pos = CVector::ZERO;
                self.ped_in_objective = Some(entity);
            }
            eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION => {
                self.ped_in_objective = Some(entity);
                self.ped_formation = eFormation::FORMATION_REAR;
            }
            eObjective::OBJECTIVE_LEAVE_CAR | eObjective::OBJECTIVE_FLEE_CAR => {
                self.car_in_objective = Some(entity);
            }
            eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
            | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
            | eObjective::OBJECTIVE_DESTROY_CAR
            | eObjective::OBJECTIVE_SOLICIT_VEHICLE
            | eObjective::OBJECTIVE_BUY_ICE_CREAM => {
                if matches!(
                    new_objective,
                    eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
                        | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                ) && self.move_state == eMoveState::PEDMOVE_STILL
                {
                    self.move_state = eMoveState::PEDMOVE_RUN;
                }
                self.car_in_objective = Some(entity);
                self.seek_target = Some(entity);
                self.seek_pos = CVector::ZERO;
                self.objective_timer = if new_objective == eObjective::OBJECTIVE_SOLICIT_VEHICLE {
                    now_ms.wrapping_add(10_000)
                } else {
                    0
                };
            }
            eObjective::OBJECTIVE_SET_LEADER => {
                self.leader = Some(entity);
                self.restore_previous_objective(current_vehicle);
            }
            _ => {}
        }
    }

    pub fn set_wait_in_car_then_get_out(&mut self, vitality: &PedVitality, duration_ms: u32, now_ms: u32) {
        self.set_objective(vitality, eObjective::OBJECTIVE_WAIT_IN_CAR_THEN_GET_OUT, now_ms);
        if self.objective == eObjective::OBJECTIVE_WAIT_IN_CAR_THEN_GET_OUT {
            self.set_objective_timer(now_ms, duration_ms);
        }
    }

    /// Port of the position overloads. `safe_distance` corresponds to the
    /// explicit three-argument C++ overload; `None` uses its per-objective defaults.
    pub fn set_objective_position(
        &mut self,
        new_objective: eObjective,
        destination: CVector,
        safe_distance: Option<f32>,
        now_ms: u32,
    ) {
        if self.dying_or_dead()
            || (self.prev_objective != eObjective::OBJECTIVE_NONE
                && self.prev_objective == new_objective)
        {
            return;
        }
        self.set_objective_timer(now_ms, 0);

        let area_objective = matches!(
            new_objective,
            eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS
                | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
                | eObjective::OBJECTIVE_RUN_TO_AREA
        );
        if self.objective == new_objective {
            if area_objective
                && self.next_route_point_pos == destination
                && safe_distance.is_none_or(|distance| self.distance_to_count_seek_done == distance)
            {
                return;
            }
            if new_objective == eObjective::OBJECTIVE_GUARD_SPOT
                && self.seek_pos_ex == destination
                && safe_distance
                    .is_none_or(|distance| self.distance_to_count_seek_done_ex == distance)
            {
                return;
            }
        }

        self.flags.objective_completed = false;
        if new_objective == eObjective::OBJECTIVE_GUARD_SPOT {
            self.seek_pos_ex = destination;
            self.distance_to_count_seek_done_ex = safe_distance.unwrap_or(5.0);
            if safe_distance.is_none() {
                self.move_state = eMoveState::PEDMOVE_STILL;
            }
        } else if area_objective {
            self.next_path_node = None;
            self.next_route_point_pos = destination;
            self.seek_pos = destination;
            self.distance_to_count_seek_done = safe_distance.unwrap_or(0.5);
            self.flags.use_ped_node_seek = true;

            if safe_distance.is_none() {
                self.flags.is_running = new_objective == eObjective::OBJECTIVE_RUN_TO_AREA;
                let position = self.physical.entity.get_position();
                let dx = destination.x - position.x;
                let dy = destination.y - position.y;
                if dx * dx + dy * dy
                    < self.distance_to_count_seek_done * self.distance_to_count_seek_done
                {
                    return;
                }
            }
        }

        if Self::is_temporary_objective(self.objective) {
            self.prev_objective = new_objective;
        } else {
            if self.objective != new_objective {
                self.set_stored_objective();
            }
            self.objective = new_objective;
        }
    }

    pub fn set_objective_route(
        &mut self,
        route: i16,
        route_type: PedRouteType,
        routes: &CPedRoutes,
        now_ms: u32,
    ) {
        let objective = eObjective::OBJECTIVE_FOLLOW_ROUTE;
        if self.dying_or_dead()
            || (self.prev_objective == objective
                && self.prev_objective != eObjective::OBJECTIVE_NONE)
        {
            return;
        }
        self.set_objective_timer(now_ms, 0);
        if self.objective == objective
            && self.route_last_point == route
            && self.route_type == route_type as i16
        {
            return;
        }
        self.flags.objective_completed = false;
        if Self::is_temporary_objective(self.objective) {
            self.prev_objective = objective;
        } else {
            if self.objective != objective {
                self.set_stored_objective();
            }
            self.objective = objective;
        }

        self.route_last_point = route;
        self.route_start_point = routes.get_route_start(route) as u16;
        self.route_points_passed = 0;
        self.route_type = route_type as i16;
        self.route_points_being_passed = 1;
        self.objective = objective;
        if let Some(next_point) = self.get_next_point_on_route(routes) {
            if let Some(position) = routes.get_point_position(next_point) {
                self.next_route_point_pos = position;
            }
        }
    }

    pub fn get_next_point_on_route(&mut self, routes: &CPedRoutes) -> Option<i16> {
        let mut next_point = i32::from(self.route_points_being_passed)
            + i32::from(self.route_points_passed)
            + i32::from(self.route_start_point);
        let route_finished = next_point < 0
            || next_point > NUMPEDROUTES as i32
            || routes.get_route_this_point_is_on(next_point as i16) != Some(self.route_last_point);
        if route_finished {
            match self.route_type {
                value if value == PedRouteType::PEDROUTE_STOP_WHEN_DONE as i16 => return None,
                value if value == PedRouteType::PEDROUTE_GO_BACKWARD_WHEN_DONE as i16 => {
                    self.route_points_being_passed = -self.route_points_being_passed;
                    next_point = i32::from(self.route_points_being_passed)
                        + i32::from(self.route_points_passed)
                        + i32::from(self.route_start_point);
                }
                value if value == PedRouteType::PEDROUTE_GO_TO_START_WHEN_DONE as i16 => {
                    self.route_points_passed = -1;
                    next_point = i32::from(self.route_points_being_passed)
                        + i32::from(self.route_points_passed)
                        + i32::from(self.route_start_point);
                }
                _ => {}
            }
        }
        i16::try_from(next_point).ok()
    }

    pub fn have_reached_next_point_on_route(&mut self, distance: f32) -> bool {
        let position = self.physical.entity.get_position();
        let dx = self.next_route_point_pos.x - position.x;
        let dy = self.next_route_point_pos.y - position.y;
        if (dx * dx + dy * dy).sqrt() >= distance {
            return false;
        }
        self.route_points_passed = self
            .route_points_passed
            .wrapping_add(self.route_points_being_passed);
        true
    }

    pub fn clear_objective(&mut self, vitality: &PedVitality, is_vehicle_driver: bool, current_vehicle: Option<usize>) {
        if self.is_ped_in_control(vitality) || self.ped_state == PedState::PED_DRIVING {
            self.objective = eObjective::OBJECTIVE_NONE;
            if self.ped_state == PedState::PED_DRIVING && current_vehicle.is_some() {
                if !is_vehicle_driver {
                    if !self.is_player() {
                        self.flags.wander_path_after_exiting_car = true;
                    }
                    self.car_in_objective = current_vehicle;
                    self.objective = eObjective::OBJECTIVE_LEAVE_CAR;
                }
            } else {
                self.ped_state = PedState::PED_IDLE;
                self.move_state = eMoveState::PEDMOVE_STILL;
            }
        } else {
            self.flags.clear_objective = true;
        }
    }

    pub fn restore_previous_objective(&mut self, current_vehicle: Option<usize>) {
        if self.objective == eObjective::OBJECTIVE_NONE {
            return;
        }
        if !matches!(
            self.objective,
            eObjective::OBJECTIVE_LEAVE_CAR
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
        ) && self.ped_state != PedState::PED_CARJACK
        {
            self.ped_in_objective = None;
        }

        if self.objective == eObjective::OBJECTIVE_WAIT_IN_CAR_THEN_GET_OUT {
            self.objective = eObjective::OBJECTIVE_NONE;
            if let Some(vehicle) = current_vehicle {
                self.car_in_objective = Some(vehicle);
                self.objective = eObjective::OBJECTIVE_LEAVE_CAR;
            }
        } else {
            self.objective = self.prev_objective;
            self.prev_objective = eObjective::OBJECTIVE_NONE;
        }
        self.flags.objective_completed = false;
    }
}
