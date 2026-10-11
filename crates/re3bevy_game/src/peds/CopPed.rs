//! re3 reference: references/re-GTA/src/peds/CopPed.cpp
//! re3 reference: references/re-GTA/src/peds/CopPed.h
//!
//! Cop ped state used by wanted and crime systems.

use std::collections::HashSet;

use bevy::prelude::{Entity, Local, Query, Res};
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    animation::{AnimAssocDefinitions::ASSOCGRP_STD, AnimManager::CAnimManager,
        AnimationId::ANIM_STD_ARREST},
    collision::Collision::CCollision,
    control::RoadBlocks::{CRoadBlocks, RoadblockCop, map_object_matrix, roadblock_map_object},
    core::{FileLoader::CFileLoader, Game::ScriptEntityWorld, Pools::CPoolHandle},
    modelinfo::ModelIndices::{MI_ARMY, MI_COP, MI_FBI, MI_SWAT},
    peds::{
        Ped::{CPed, PedState, RANDOM_CHAR, eMoveState, eObjective,
            blend_ped_movement_animation, update_non_player_movement_animation},
        PedType::ePedType,
        PlayerPed::CWorldPlayers,
    },
    weapons::WeaponType::eWeaponType,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum eCopType {
    COP_STREET = 0,
    COP_FBI = 1,
    COP_SWAT = 2,
    COP_ARMY = 3,
}

/// Owned constructor form. Live cops use CPed, PedVitality and their ECS
/// relationships; this form is never a second component on that actor.
#[derive(Debug, Clone, PartialEq)]
pub struct CCopPed {
    pub ped: CPed,
    pub vitality: crate::peds::PedVitality::PedVitality,
    pub roadblock_node: i16,
    pub distance_to_target: f32,
    pub is_in_pursuit: bool,
    pub is_disabled_cop: bool,
    pub field_1350: i8,
    pub beating_suspect: bool,
    pub stop_and_shoot_disabled_zone: bool,
    pub zone_disabled: bool,
    pub abseil_pos: f32,
    pub cop_type: eCopType,
    pub field_1364: i8,
}

impl CCopPed {
    pub fn new(cop_type: eCopType, general: &mut CGeneral) -> Self {
        let mut ped = CPed::new(ePedType::PEDTYPE_COP, general);
        let mut vitality = crate::peds::PedVitality::PedVitality::default();
        let model = cop_model_for_type(cop_type);
        ped.physical.entity.set_model_index_no_create(model);
        initialise_cop_ped(&mut ped, &mut vitality, cop_type);

        Self {
            ped,
            vitality,
            roadblock_node: -1,
            distance_to_target: 0.0,
            is_in_pursuit: false,
            is_disabled_cop: false,
            field_1350: 1,
            beating_suspect: false,
            stop_and_shoot_disabled_zone: false,
            zone_disabled: false,
            abseil_pos: 0.0,
            cop_type,
            field_1364: -1,
        }
    }

    pub fn clear_pursuit_state(&mut self, is_vehicle_driver: bool, current_vehicle: Option<usize>) {
        if !self.is_in_pursuit {
            return;
        }
        self.is_in_pursuit = false;
        self.ped.objective = eObjective::OBJECTIVE_NONE;
        self.ped.prev_objective = eObjective::OBJECTIVE_NONE;
        self.ped.last_ped_state = PedState::PED_NONE;
        self.ped.flags.is_running = false;
        self.ped.flags.not_allowed_to_duck = false;
        self.ped.flags.kinda_stay_in_same_place = false;
        self.stop_and_shoot_disabled_zone = false;
        self.zone_disabled = false;
        self.ped.clear_objective(&self.vitality, is_vehicle_driver, current_vehicle);
    }

    /// Applies the ped-side portion of `SetArrestPlayer`; the player runtime
    /// stores the arresting cop handle separately.
    pub fn set_arrest_player(&mut self, player: &mut CPed, player_handle: usize, now_ms: u32) {
        if !self.ped.is_ped_in_control(&self.vitality, ) {
            return;
        }
        if !matches!(
            player.ped_state,
            PedState::PED_DIE | PedState::PED_DEAD | PedState::PED_ARRESTED
        ) {
            player.last_ped_state = player.ped_state;
            player.ped_state = PedState::PED_ARRESTED;
        }
        self.ped.ped_state = PedState::PED_ARREST_PLAYER;
        self.ped.set_objective(&self.vitality, eObjective::OBJECTIVE_NONE, now_ms);
        self.ped.prev_objective = eObjective::OBJECTIVE_NONE;
        self.ped.flags.is_pointing_gun_at = false;
        self.ped.seek_target = Some(player_handle);
        self.ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
    }
}

pub(crate) const fn cop_type_for_model(model: usize) -> eCopType {
    match model {
        MI_FBI => eCopType::COP_FBI,
        MI_SWAT => eCopType::COP_SWAT,
        MI_ARMY => eCopType::COP_ARMY,
        _ => eCopType::COP_STREET,
    }
}

pub(crate) const fn cop_model_for_type(cop_type: eCopType) -> usize {
    match cop_type {
        eCopType::COP_STREET => MI_COP,
        eCopType::COP_FBI => MI_FBI,
        eCopType::COP_SWAT => MI_SWAT,
        eCopType::COP_ARMY => MI_ARMY,
    }
}

/// Per-type values from re3 CCopPed::CCopPed.
struct CopConstructorParameters {
    armour: f32,
    weapon_skills: u8,
    weapon_accuracy: u8,
    current_weapon: eWeaponType,
}

/// Apply CCopPed constructor behavior to the canonical ECS ped payload.
pub(crate) fn initialise_cop_ped(ped: &mut CPed, vitality: &mut crate::peds::PedVitality::PedVitality, cop_type: eCopType) {
    // TODO(parity): CCopPed::CCopPed — mission/standalone construction must
    // supply the original loaded weapon metadata.
    initialise_cop_inventory(ped, vitality, cop_type, |ped, weapon, ammo| {
        let _ = crate::peds::Ped::give_ped_weapon(ped, weapon as i32, ammo);
    });
}

pub(crate) fn initialise_cop_ped_with_info(
    ped: &mut CPed, vitality: &mut crate::peds::PedVitality::PedVitality,
    cop_type: eCopType,
    info: &crate::weapons::WeaponInfo::CWeaponInfoStore,
) {
    initialise_cop_inventory(ped, vitality, cop_type, |ped, weapon, ammo| {
        let _ = crate::peds::Ped::give_ped_weapon_with_info(ped, weapon as i32, ammo, info);
    });
}

fn initialise_cop_inventory(
    ped: &mut CPed, vitality: &mut crate::peds::PedVitality::PedVitality,
    cop_type: eCopType,
    mut give_weapon: impl FnMut(&mut CPed, eWeaponType, i32),
) {
    let parameters = match cop_type {
        eCopType::COP_STREET => CopConstructorParameters {
            armour: 0.0, weapon_skills: 208, weapon_accuracy: 60,
            current_weapon: eWeaponType::WEAPONTYPE_UNARMED,
        },
        eCopType::COP_FBI => CopConstructorParameters {
            armour: 100.0, weapon_skills: 176, weapon_accuracy: 76,
            current_weapon: eWeaponType::WEAPONTYPE_AK47,
        },
        eCopType::COP_SWAT => CopConstructorParameters {
            armour: 50.0, weapon_skills: 32, weapon_accuracy: 68,
            current_weapon: eWeaponType::WEAPONTYPE_UZI,
        },
        eCopType::COP_ARMY => CopConstructorParameters {
            armour: 100.0, weapon_skills: 32, weapon_accuracy: 84,
            current_weapon: eWeaponType::WEAPONTYPE_M16,
        },
    };
    give_weapon(ped, eWeaponType::WEAPONTYPE_COLT45, 1_000);
    match cop_type {
        eCopType::COP_FBI => {
            give_weapon(ped, eWeaponType::WEAPONTYPE_AK47, 1_000)
        }
        eCopType::COP_SWAT => {
            give_weapon(ped, eWeaponType::WEAPONTYPE_UZI, 1_000)
        }
        eCopType::COP_ARMY => {
            give_weapon(ped, eWeaponType::WEAPONTYPE_M16, 1_000);
            give_weapon(ped, eWeaponType::WEAPONTYPE_GRENADE, 10);
        }
        eCopType::COP_STREET => {}
    }
    ped.current_weapon = parameters.current_weapon;
    vitality.armour = parameters.armour;
    ped.wep_skills = parameters.weapon_skills;
    ped.wep_accuracy = parameters.weapon_accuracy;
    ped.attack_timer = 0;
    ped.point_gun_at = None;
}

#[derive(Debug, Clone, Copy)]
struct CopPursuitCandidate {
    handle: i32,
    slot: usize,
    distance_sq: f32,
}

#[derive(Default)]
pub(crate) struct CopAiScratch {
    seated: HashSet<i32>,
    roadblock_cops: HashSet<i32>,
    candidates: Vec<CopPursuitCandidate>,
    selected: HashSet<i32>,
}

/// ECS form of re3 `CCopPed::ProcessControl/CopAI`.
///
/// Pursuit membership remains authoritative in the focused player's
/// `CWanted::m_pCops` array, exactly where the C++ keeps it. Cop movement is
/// applied to the canonical `ScriptPedEntity::ped`; no reverse cop/ped pool
/// or duplicate ownership is introduced.
pub(crate) fn process_cop_ai(
    mut script_entities: ScriptEntityWorld,
    mut world_players: CWorldPlayers,
    file_loader: Res<CFileLoader>,
    roadblocks: Res<CRoadBlocks>,
    roadblock_cops: Query<(Entity, &RoadblockCop)>,
    collision: Res<CCollision>,
    timer: Res<CTimer>,
    anim_manager: Res<CAnimManager>,
    mut scratch: Local<CopAiScratch>,
) {
    let focus = world_players.player_in_focus() as i32;
    let Some(player) = world_players.focused() else {
        return;
    };
    let player_slot = CPoolHandle(player.pool_handle).index();
    let player_state = player.ped.ped_state;
    let player_dead = player.ped.dying_or_dead();
    let player_on_physical_surface = player.ped.current_phys_surface.is_some();
    let wanted_level = player.wanted.get_wanted_level();
    let max_cops = usize::from(player.wanted.m_MaxCops);
    let ignored = player.wanted.m_bIgnoredByEveryone || player.wanted.m_bIgnoredByCops;
    let player_relation = world_players.relation(focus);
    let player_vehicle = player_relation
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle));
    let (target_position, target_speed) = player_vehicle
        .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
        .map(|vehicle| {
            (
                vehicle.base().physical.entity.get_position(),
                vehicle.base().physical.move_speed,
            )
        })
        .unwrap_or((player.position(), player.ped.physical.move_speed));
    let no_police_zone = file_loader.world.cull_zones.player_no_police();
    let player_car_alarm = player_vehicle.and_then(|handle|
        script_entities.vehicles.get(&script_entities.index, &handle))
        .is_some_and(|vehicle| matches!(vehicle,
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { .. })
            && vehicle.base().audio.alarm_state != 0 && vehicle.base().audio.alarm_state != -1
            && vehicle.base().physical.entity.status != crate::entities::Entity::eEntityStatus::STATUS_WRECKED);

    let scratch = &mut *scratch;
    scratch.seated.clear();
    scratch
        .seated
        .extend(script_entities.script_relations().map(|(handle, _)| handle));
    // CCopPed::ScanForCrimes checks the player car's alarm at 20 metres.
    // It runs after CopAI in native, so a newly raised star affects the next
    // pursuit decision rather than rebuilding this frame's decisions.
    if player_car_alarm && script_entities.peds.values().any(|entity| {
        let ped = &entity.ped;
        let vitality = &entity.vitality;
        if ped.ped_type != ePedType::PEDTYPE_COP
            || !(ped.is_ped_in_control(vitality, ) || ped.ped_state == PedState::PED_DRIVING)
        { return false; }
        let position = ped.physical.entity.get_position();
        let delta = CVector::new(target_position.x - position.x,
            target_position.y - position.y, target_position.z - position.z);
        delta.magnitude_sqr() < 20.0 * 20.0
    }) {
        if let Some(mut player) = world_players.focused_mut() {
            player.wanted.set_wanted_level_no_drop(1);
        }
    }
    scratch.roadblock_cops.clear();
    let roadblock_states = roadblock_cops
        .iter()
        .filter_map(|(entity, state)| {
            script_entities
                .ped_handle_for_entity(entity)
                .map(|handle| (entity, handle, *state))
        })
        .collect::<Vec<_>>();
    scratch
        .roadblock_cops
        .extend(roadblock_states.iter().map(|(_, handle, _)| *handle));

    // re3 keeps freshly spawned roadblock cops disabled until the player has
    // reached/crossed the authored roadblock line (or gets very close). They
    // aim from their fixed position and must not consume an ordinary pursuit
    // slot before that transition.
    for (entity, handle, state) in roadblock_states {
        let _roadblock_vehicle_still_exists =
            script_entities.vehicle_get(state.vehicle_handle).is_some();
        let Some(ped_entity) = script_entities.ped_get_mut(handle) else {
            continue;
        };
        let ped = &mut *ped_entity.ped;
        let position = ped.physical.entity.get_position();
        let target_delta = CVector::new(
            target_position.x - position.x,
            target_position.y - position.y,
            target_position.z - position.z,
        );
        let distance = target_delta.magnitude();
        let target_moving_away = distance > 30.0 && target_speed.dot(target_delta) > 0.0;
        let target_close = distance < 5.0
            && (player_vehicle.is_none()
                || target_speed.magnitude_sqr() < (1.0_f32 / 200.0).powi(2));
        let straddles_roadblock =
            roadblock_map_object(&roadblocks, &file_loader, state.roadblock_node)
                .map(|map_object| {
                    let matrix = map_object_matrix(map_object);
                    let cop_delta = CVector::new(
                        position.x - map_object.position.x,
                        position.y - map_object.position.y,
                        0.0,
                    );
                    let target_road_delta = CVector::new(
                        target_position.x - map_object.position.x,
                        target_position.y - map_object.position.y,
                        0.0,
                    );
                    let cop_dot = cop_delta.x * matrix.forward.x + cop_delta.y * matrix.forward.y;
                    let target_dot = target_road_delta.x * matrix.forward.x
                        + target_road_delta.y * matrix.forward.y;
                    (cop_dot >= 0.0 || target_dot >= 0.0) && (cop_dot <= 0.0 || target_dot <= 0.0)
                })
                .unwrap_or(false);

        if target_moving_away || target_close || !straddles_roadblock {
            ped.flags.kinda_stay_in_same_place = false;
            ped.flags.not_allowed_to_duck = false;
            ped.flags.crouch_when_shooting = false;
            ped.flags.is_pointing_gun_at = false;
            ped.point_gun_at = None;
            ped.seek_target = None;
            scratch.roadblock_cops.remove(&handle);
            script_entities
                .commands
                .entity(entity)
                .remove::<RoadblockCop>();
            continue;
        }

        ped.flags.kinda_stay_in_same_place = true;
        ped.flags.is_running = false;
        ped.flags.is_pointing_gun_at = true;
        ped.point_gun_at = Some(player_slot);
        ped.seek_target = Some(player_slot);
        ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
        ped.ped_state = PedState::PED_ATTACK;
        ped.move_state = eMoveState::PEDMOVE_STILL;
        ped.physical.move_speed = CVector::ZERO;
    }
    scratch.candidates.clear();
    if wanted_level > 0 && !ignored && !no_police_zone {
        for (&handle, entity) in script_entities.peds.iter() {
            let ped = &entity.ped;
            let vitality = &entity.vitality;
            if ped.ped_type != ePedType::PEDTYPE_COP
                || scratch.seated.contains(&handle)
                || scratch.roadblock_cops.contains(&handle)
                || !ped.is_ped_in_control(vitality, )
                || (ped.physical.zone_level > crate::collision::ColModel::LEVEL_GENERIC as i8
                    && ped.physical.zone_level != collision.collision_in_memory as i8)
            {
                continue;
            }
            let position = ped.physical.entity.get_position();
            let dx = target_position.x - position.x;
            let dy = target_position.y - position.y;
            let distance_sq = dx * dx + dy * dy;
            // CopAI drops an on-foot pursuit when the player is pulling away
            // in a vehicle: m_fDistanceToTarget * FindPlayerSpeed > 4.
            if player_vehicle.is_some() && distance_sq.sqrt() * target_speed.magnitude() > 4.0 {
                continue;
            }
            scratch.candidates.push(CopPursuitCandidate {
                handle,
                slot: CPoolHandle(handle).index(),
                distance_sq,
            });
        }
        scratch
            .candidates
            .sort_by(|a, b| a.distance_sq.total_cmp(&b.distance_sq));
        scratch.candidates.truncate(max_cops);
    }

    scratch.selected.clear();
    scratch
        .selected
        .extend(scratch.candidates.iter().map(|candidate| candidate.handle));
    if let Some(mut player) = world_players.focused_mut() {
        player.wanted.m_pCops.fill(None);
        for (index, candidate) in scratch.candidates.iter().enumerate() {
            if let Some(slot) = player.wanted.m_pCops.get_mut(index) {
                *slot = Some(candidate.slot);
            }
        }
        player.wanted.m_CurrentCops = scratch.candidates.len().min(u8::MAX as usize) as u8;
    }

    let now_ms = timer.time_in_milliseconds();
    let mut arresting_cop: Option<(i32, f32)> = None;

    for (handle, entity) in script_entities.peds.iter_mut() {
        let ped = &mut *entity.ped;
        let locomotion = &mut *entity.locomotion;
        let vitality = &mut *entity.vitality;
        if ped.ped_type != ePedType::PEDTYPE_COP || scratch.seated.contains(&handle) {
            continue;
        }
        if ped.ped_state == PedState::PED_ARREST_PLAYER {
            if ped.seek_target == Some(player_slot)
                && (player_state == PedState::PED_ARRESTED || player_dead
                    || player_state == PedState::PED_ENTER_CAR)
            {
                if !entity.animation.movement_anim_clump.associations.iter()
                    .any(|association| association.anim_id == ANIM_STD_ARREST && association.blend_delta >= 0.0)
                {
                    if blend_ped_movement_animation(&mut entity.animation.movement_anim_clump,
                        &anim_manager, ASSOCGRP_STD, ANIM_STD_ARREST, 4.0).is_none()
                    {
                        bevy::log::warn_once!(target: "re3bevy::cop_ai", "arrest association unavailable");
                    }
                }
                update_non_player_movement_animation(ped, locomotion, vitality, entity.animation, &anim_manager, &file_loader,
                    timer.time_step_in_seconds(), false, None);
                locomotion.rotation_dest = CGeneral::get_radian_angle_between_points(
                    target_position.x, target_position.y,
                    ped.physical.entity.get_position().x, ped.physical.entity.get_position().y);
                locomotion.rotation_cur = locomotion.rotation_dest;
                ped.physical.entity.set_heading(locomotion.rotation_cur);
                continue;
            }
            ped.ped_state = PedState::PED_IDLE;
            ped.seek_target = None;
        }
        if scratch.roadblock_cops.contains(&handle) {
            continue;
        }
        if ped.physical.zone_level > crate::collision::ColModel::LEVEL_GENERIC as i8
            && ped.physical.zone_level != collision.collision_in_memory as i8
        {
            continue;
        }
        let selected = scratch.selected.contains(&handle);
        if !selected {
            let was_our_pursuit = matches!(
                ped.objective,
                eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
            ) && ped.ped_in_objective == Some(player_slot);
            if was_our_pursuit && ped.char_created_by == RANDOM_CHAR {
                ped.set_objective(vitality, eObjective::OBJECTIVE_NONE, now_ms);
                ped.ped_in_objective = None;
                ped.flags.is_running = false;
                ped.flags.not_allowed_to_duck = false;
                if ped.is_ped_in_control(vitality, ) {
                    ped.ped_state = PedState::PED_WANDER_PATH;
                    ped.move_state = eMoveState::PEDMOVE_WALK;
                }
            }
            continue;
        }

        ped.set_objective_entity(eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT,
            player_slot,
            now_ms,
            None,
        );
        ped.flags.is_running = true;
        ped.flags.not_allowed_to_duck = true;
        if wanted_level > 1 && ped.current_weapon == eWeaponType::WEAPONTYPE_UNARMED {
            ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
        } else if wanted_level == 1 && !player_on_physical_surface {
            ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;
        }

        let position = ped.physical.entity.get_position();
        let dx = target_position.x - position.x;
        let dy = target_position.y - position.y;
        let distance_sq = dx * dx + dy * dy;
        let should_arrest = (player_dead || player_state == PedState::PED_ARRESTED)
            && distance_sq < 25.0
            || player_state == PedState::PED_ENTER_CAR && distance_sq < 1.3 * 1.3;
        if should_arrest {
            if arresting_cop.is_none_or(|(_, best)| distance_sq < best) {
                arresting_cop = Some((handle, distance_sq));
            }
            continue;
        }

        // CCopPed::CopAI normal pursuit ends after setting the objective.
        // CPed::ProcessObjective/Seek applies the authored movement once.
    }

    if let Some((cop_handle, _)) = arresting_cop {
        let Some(mut player) = world_players.focused_mut() else {
            return;
        };
        if !matches!(
            player.ped.ped_state,
            PedState::PED_DIE | PedState::PED_DEAD | PedState::PED_ARRESTED
        ) {
            player.ped.last_ped_state = player.ped.ped_state;
            player.ped.ped_state = PedState::PED_ARRESTED;
        }
        player.can_be_damaged = false;
        player.arresting_cop = Some(CPoolHandle(cop_handle).index());
        if let Some(vehicle_handle) = player_vehicle
            && let Some(mut vehicle) = script_entities.vehicle_get_mut(vehicle_handle)
        {
            let mut base = vehicle.base_mut();
            base.entry.num_getting_in = 0;
            base.entry.getting_in_flags = 0;
            base.drive.is_handbrake_on = true;
            base.physical.entity.status = crate::entities::Entity::eEntityStatus::STATUS_PLAYER_DISABLED;
        }
        if let Some(cop) = script_entities.peds.get_mut(&script_entities.index, &cop_handle) {
            cop.ped.ped_state = PedState::PED_ARREST_PLAYER;
            cop.ped.set_objective(&cop.vitality, eObjective::OBJECTIVE_NONE, now_ms);
            cop.ped.prev_objective = eObjective::OBJECTIVE_NONE;
            cop.ped.flags.is_pointing_gun_at = false;
            cop.ped.seek_target = Some(player_slot);
            cop.ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
            cop.ped.move_state = eMoveState::PEDMOVE_STILL;
        }
    }
}
