//! re3 reference: references/re-GTA/src/peds/PedAI.cpp
//!
//! Ped AI state view. State identifiers live on `CPed`, as they do in re3's
//! `Ped.h`; this module owns the objective-processing surface from `PedAI.cpp`.

use std::collections::{HashMap, HashSet};

use bevy::{
    ecs::system::SystemParam,
    prelude::{Commands, Entity, Has, Local, Query, Res, ResMut, Without},
};
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};
#[path = "PedWalkAround.rs"]
mod walk_around;

use crate::{
    audio::{
        DMAudio::cDMAudio,
        soundlist::{
            SOUND_FIGHT_PUNCH_39, SOUND_WEAPON_BAT_ATTACK, SOUND_WEAPON_FLAMETHROWER_FIRE,
            SOUND_WEAPON_HIT_PED, SOUND_WEAPON_HIT_VEHICLE, SOUND_WEAPON_RELOAD,
            SOUND_WEAPON_SHOT_FIRED,
        },
    },
    animation::{
        AnimBlendAssociation::CAnimBlendAssociation, AnimManager::CAnimManager,
        PedVehicleAnimation::PedVehicleAnimationPhase,
    },
    collision::Collision::{CCollision, resolve_ped_static_motion},
    collision::TempColModels::{PED_BODY_MIN_Z, PED_BODY_MAX_Z},
    control::PathFind::{CPathFind, PATH_PED},
    control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity, ScriptVehicleIdentity},
    core::{
        Cam::eCamMode,
        Camera::CCamera,
        EventList::{CEventList, EEventEntity, EEventType},
        FileLoader::CFileLoader,
        Pools::CPoolHandle,
    },
    math::Matrix::CMatrix,
    modelinfo::{ModelInfo::ModelInfoSlot, VehicleModelInfo::vehicle_dummy_model_position},
    peds::{
        PedAnimation::PedAnimation,
        PedLocomotion::PedLocomotion,
        PedVitality::PedVitality,
        PlayerRead::{CWorldPlayersCombat, CWorldPlayersRead},
        VehicleRelations::InVehicle,
        PedRoutes::CPedRoutes,
        PedType::CPedStatsRuntime,
        PlayerPed::{
            PlayerPedState, WeaponRayHit, add_weapon_gunshell, apply_ped_gun_hit_reaction,
            apply_melee_fall_response, apply_player_gun_hit_reaction,
            apply_weapon_force_to_object,
            closest_weapon_ray_hit, emit_bullet_sparks, emit_bullet_surface_impact,
            emit_instant_hit_muzzle_fx, emit_melee_blood, emit_shotgun_muzzle_fx,
            emit_weapon_blood, melee_ped_collision, ped_defend_weakness,
            rotate_weapon_ray,
        },
    },
    renderer::{Particle::{CParticle, ParticleEmitter}, PointLights::CPointLights, SpecialFX::CSpecialFX},
    vehicles::HandlingMgr::tHandlingData,
    vehicles::Vehicle::VehicleCore,
    weapons::{
        WeaponActorAccess::WeaponActorAccess,
        BulletInfo::CBulletInfoPool,
        ProjectileInfo::CProjectileInfoPool, ShotInfo::CShotInfoPool,
        Weapon::{WeaponFireContext, WeaponFireRequest},
        WeaponInfo::CWeaponInfoStore,
        WeaponType::{eWeaponFire, eWeaponState, eWeaponType},
    },
};

use super::Ped::{
    CPed, CPedAnimationOffsets, PedState, eMoveState, eObjective,
    update_non_player_movement_animation, blend_ped_movement_animation,
};
use super::PedWeaponAttack::{
    GroundAttackCandidate, GroundAttackChoice, PedOnGroundState, NpcAttackAnimationSample,
    check_for_peds_on_ground_to_attack, service_attack_finish, sample_attack_animation,
    finish_attack_frame, suppress_contact_finish,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CPedAIState {
    pub state: PedState,
    pub move_state: eMoveState,
    pub objective: eObjective,
    pub previous_objective: eObjective,
    pub objective_timer: u32,
    pub objective_completed: bool,
    pub script_objective_completed: bool,
}

impl CPedAIState {
    pub fn from_ped(ped: &CPed) -> Self {
        Self {
            state: ped.ped_state,
            move_state: ped.move_state,
            objective: ped.objective,
            previous_objective: ped.prev_objective,
            objective_timer: ped.objective_timer,
            objective_completed: ped.flags.objective_completed,
            script_objective_completed: ped.flags.script_objective_completed,
        }
    }
}

/// re3 `CPed::GetPositionToOpenCarDoor`. A vehicle's visual door dummy is
/// not the ped root position: the latter is derived from the authored
/// front/back-seat dummy, HANDLING.CFG seat distance and the final translated
/// key of the matching PED.IFP get-in animation.
pub fn automobile_ped_door_world_position<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    handling: &tHandlingData,
    offsets: &CPedAnimationOffsets,
    door: usize,
    seat_position_multiplier: f32,
) -> Option<CVector>
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let local = automobile_ped_door_local_position(
        file_loader,
        car,
        handling,
        offsets,
        door,
        seat_position_multiplier,
    )?;
    Some(car.vehicle.core.physical.entity.transform_point(local))
}

/// Matrix written by `CPed::LineUpPedWithCar`. While the animation root is
/// still outside the cabin re3 keeps the ped upright at the calculated door
/// position. Once `seatPosMult` reaches 0.2 it copies the complete vehicle
/// matrix and translates it by the zero-offset door position. That final
/// branch is what makes the ped follow vehicle pitch/roll without a visible
/// snap at the end of get-in or the start of get-out.
#[derive(Debug, Clone, Copy)]
pub struct PedVehicleLineUpContext {
    pub current_position: CVector,
    pub current_heading: f32,
    pub offset_seek: CVector,
    pub state_timer_ms: u32,
    pub now_ms: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PedVehicleLineUpResult {
    pub matrix: crate::math::Matrix::CMatrix,
    pub vertical_speed_z: f32,
}

pub fn automobile_ped_line_up_matrix<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    handling: &tHandlingData,
    offsets: &CPedAnimationOffsets,
    door: usize,
    line_up: PedVehicleLineUpContext,
    phase: PedVehicleAnimationPhase,
    association: &CAnimBlendAssociation,
) -> Option<crate::math::Matrix::CMatrix>
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    automobile_ped_line_up(
        file_loader,
        car,
        handling,
        offsets,
        door,
        line_up,
        phase,
        association,
        0.0,
        1.0,
    )
    .map(|result| result.matrix)
}

pub(crate) fn automobile_ped_line_up<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    handling: &tHandlingData,
    offsets: &CPedAnimationOffsets,
    door: usize,
    line_up: PedVehicleLineUpContext,
    phase: PedVehicleAnimationPhase,
    association: &CAnimBlendAssociation,
    current_vertical_speed_z: f32,
    gta_time_step: f32,
) -> Option<PedVehicleLineUpResult>
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let seat_position_multiplier = ped_vehicle_line_up_seat_multiplier(phase, association);
    let upside_down = car.vehicle.core.physical.entity.get_up().z <= -0.8;
    let line_up_kind = ped_vehicle_line_up_kind(car, phase, association, upside_down);
    if seat_position_multiplier <= 0.2 && car.vehicle.core.physical.entity.get_up().z > -0.8 {
        let local =
            automobile_ped_door_local_position(file_loader, car, handling, offsets, door, 0.0)?;
        let mut matrix = car.vehicle.core.physical.entity.matrix;
        matrix.position = matrix.transform_point(local);
        return Some(PedVehicleLineUpResult {
            matrix,
            vertical_speed_z: 0.0,
        });
    }

    let mut position = if line_up_kind == PedVehicleLineUpKind::Second {
        line_up.current_position
    } else {
        automobile_ped_door_world_position(
            file_loader,
            car,
            handling,
            offsets,
            door,
            seat_position_multiplier,
        )?
    };
    let grounded = if car.vehicle.core.physical.flags.is_in_water {
        None
    } else {
        let result = crate::peds::PedPlacement::CPedPlacement::find_z_coor_for_ped(
            position,
            &file_loader.world,
            &file_loader.model_info,
        );
        result.found.then_some(result.position.z)
    };
    let auto_z = grounded.unwrap_or(position.z);
    let mut vertical_speed_z = if line_up_kind == PedVehicleLineUpKind::Start {
        0.0
    } else {
        current_vertical_speed_z
    };
    if matches!(
        line_up_kind,
        PedVehicleLineUpKind::End | PedVehicleLineUpKind::Second
    ) {
        position.z = line_up.current_position.z;
        if !car.vehicle.core.flags.is_bus || upside_down {
            let next_z_speed =
                vertical_speed_z - crate::entities::Physical::GRAVITY * gta_time_step.max(0.0);
            let next_z = position.z + next_z_speed * gta_time_step.max(0.0);
            if next_z >= auto_z {
                vertical_speed_z = next_z_speed;
                position.z = next_z;
            } else {
                vertical_speed_z = 0.0;
                position.z = auto_z;
            }
        }
    }
    position.z = ped_vehicle_line_up_z(
        car,
        door,
        line_up_kind,
        phase,
        association,
        line_up.current_position.z,
        position.z,
        auto_z,
        &mut vertical_speed_z,
    );
    let vehicle_heading = car.vehicle.core.physical.entity.get_heading();
    let target_heading = if matches!(door, 3 | 5) {
        if upside_down {
            -std::f32::consts::PI + vehicle_heading
        } else if car.vehicle.core.flags.is_bus {
            std::f32::consts::FRAC_PI_2 + vehicle_heading
        } else {
            vehicle_heading
        }
    } else if upside_down {
        vehicle_heading
    } else if car.vehicle.core.flags.is_bus {
        -std::f32::consts::FRAC_PI_2 + vehicle_heading
    } else {
        vehicle_heading
    };

    let heading = if line_up.now_ms < line_up.state_timer_ms {
        let time_until_state_change =
            line_up.state_timer_ms.wrapping_sub(line_up.now_ms) as f32 / 600.0;
        let time_until_state_change = time_until_state_change.clamp(0.0, 1.0);
        position.x -= time_until_state_change * line_up.offset_seek.x;
        position.y -= time_until_state_change * line_up.offset_seek.y;

        let mut limited_dest = CGeneral::limit_radian_angle(target_heading);
        if std::f32::consts::PI + line_up.current_heading < limited_dest {
            limited_dest -= std::f32::consts::TAU;
        } else if line_up.current_heading - std::f32::consts::PI > limited_dest {
            limited_dest += std::f32::consts::TAU;
        }
        line_up.current_heading
            - (line_up.current_heading - limited_dest) * (1.0 - time_until_state_change)
    } else {
        target_heading
    };

    let mut matrix = crate::math::Matrix::CMatrix::unity();
    matrix.set_rotate_z_only(heading);
    matrix.position = position;
    Some(PedVehicleLineUpResult {
        matrix,
        vertical_speed_z,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PedVehicleLineUpKind {
    Start,
    End,
    Second,
}

fn ped_vehicle_line_up_kind<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    phase: PedVehicleAnimationPhase,
    association: &CAnimBlendAssociation,
    upside_down: bool,
) -> PedVehicleLineUpKind
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    use crate::animation::AnimationId::{ANIM_STD_CAR_CLOSE_LHS, ANIM_STD_CAR_CLOSE_RHS};

    match phase {
        PedVehicleAnimationPhase::DraggedOut if association.current_time > 1.4 => {
            PedVehicleLineUpKind::Second
        }
        PedVehicleAnimationPhase::CloseExit => PedVehicleLineUpKind::End,
        PedVehicleAnimationPhase::Exit
            if upside_down
                || car.vehicle.core.physical.entity.model_index
                    == crate::modelinfo::ModelIndices::MI_DODO as i16
                || matches!(
                    association.anim_id,
                    ANIM_STD_CAR_CLOSE_LHS | ANIM_STD_CAR_CLOSE_RHS
                )
                || association.current_time > 0.3 =>
        {
            PedVehicleLineUpKind::End
        }
        _ => PedVehicleLineUpKind::Start,
    }
}

fn ped_vehicle_line_up_z<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    door: usize,
    line_up_kind: PedVehicleLineUpKind,
    phase: PedVehicleAnimationPhase,
    association: &CAnimBlendAssociation,
    current_z: f32,
    needed_z: f32,
    ground_z: f32,
    vertical_speed_z: &mut f32,
) -> f32
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    use crate::animation::AnimationId::{
        ANIM_STD_CAR_GET_IN_LHS, ANIM_STD_CAR_GET_IN_LO_LHS, ANIM_STD_CAR_GET_IN_LO_RHS,
        ANIM_STD_CAR_GET_IN_RHS, ANIM_STD_QUICKJACK, ANIM_STD_VAN_CLOSE_DOOR_REAR_LHS,
        ANIM_STD_VAN_CLOSE_DOOR_REAR_RHS, ANIM_STD_VAN_GET_IN_REAR_LHS,
        ANIM_STD_VAN_GET_IN_REAR_RHS,
    };

    let rear_van_door = car.vehicle.core.flags.is_van && matches!(door, 4 | 5);
    if ground_z > needed_z {
        let van_animation = matches!(
            association.anim_id,
            ANIM_STD_VAN_GET_IN_REAR_LHS
                | ANIM_STD_VAN_GET_IN_REAR_RHS
                | ANIM_STD_VAN_CLOSE_DOOR_REAR_LHS
                | ANIM_STD_VAN_CLOSE_DOOR_REAR_RHS
        );
        if !van_animation {
            *vertical_speed_z = 0.0;
            return ground_z;
        }
        if rear_van_door && phase == PedVehicleAnimationPhase::GetIn && needed_z < current_z {
            return vehicle_get_in_z_step(current_z, needed_z, association);
        }
        return needed_z;
    }

    if line_up_kind == PedVehicleLineUpKind::Start && needed_z > current_z {
        if matches!(
            association.anim_id,
            ANIM_STD_CAR_GET_IN_RHS
                | ANIM_STD_CAR_GET_IN_LO_RHS
                | ANIM_STD_CAR_GET_IN_LHS
                | ANIM_STD_CAR_GET_IN_LO_LHS
                | ANIM_STD_QUICKJACK
                | ANIM_STD_VAN_GET_IN_REAR_LHS
                | ANIM_STD_VAN_GET_IN_REAR_RHS
        ) {
            return vehicle_get_in_z_step(current_z, needed_z, association);
        }
        if matches!(
            phase,
            PedVehicleAnimationPhase::Align
                | PedVehicleAnimationPhase::LockedDoor
                | PedVehicleAnimationPhase::OpenDoor
                | PedVehicleAnimationPhase::PullOut
                | PedVehicleAnimationPhase::GetIn
                | PedVehicleAnimationPhase::CloseEnter
                | PedVehicleAnimationPhase::Shuffle
        ) {
            return current_z.max(ground_z);
        }
    }

    needed_z
}

fn vehicle_get_in_z_step(
    current_z: f32,
    needed_z: f32,
    association: &CAnimBlendAssociation,
) -> f32 {
    let adjusted_time_step = association.time_step.max(0.1);
    let frames_left = association.get_time_left() / adjusted_time_step;
    if frames_left > 1.0 {
        current_z + (needed_z - current_z) / frames_left
    } else {
        needed_z
    }
}

fn automobile_ped_door_local_position<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    handling: &tHandlingData,
    offsets: &CPedAnimationOffsets,
    door: usize,
    seat_position_multiplier: f32,
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
    let rear = matches!(door, 4 | 5);
    let right = matches!(door, 3 | 5);
    let frame_name = if rear {
        "ped_backseat"
    } else {
        "ped_frontseat"
    };
    let summary = model.clump.base.clump_summary.as_ref()?;
    let mut seat = vehicle_dummy_model_position(summary, frame_name)?;
    let seat_offset = if car.vehicle.core.flags.is_van && rear {
        0.0
    } else {
        handling.seat_offset_distance * seat_position_multiplier
    };
    let mut animation_offset = if car.vehicle.core.flags.is_van && rear {
        offsets.ped_van_rear_door
    } else if car.vehicle.core.flags.low_vehicle {
        offsets.ped_car_door_low
    } else {
        offsets.ped_car_door
    };
    if right {
        seat.x += seat_offset;
        animation_offset.x = -animation_offset.x;
    } else {
        seat.x = -(seat.x + seat_offset);
    }
    Some(CVector::new(
        seat.x - animation_offset.x,
        seat.y - animation_offset.y,
        seat.z - animation_offset.z,
    ))
}

pub fn ped_vehicle_line_up_seat_multiplier(
    phase: PedVehicleAnimationPhase,
    association: &CAnimBlendAssociation,
) -> f32 {
    use crate::animation::AnimationId::{
        ANIM_STD_BOAT_DRIVE, ANIM_STD_CAR_CLOSE_DOOR_LHS, ANIM_STD_CAR_CLOSE_DOOR_LO_LHS,
        ANIM_STD_CAR_CLOSE_DOOR_LO_RHS, ANIM_STD_CAR_CLOSE_DOOR_RHS, ANIM_STD_CAR_CLOSE_LHS,
        ANIM_STD_CAR_CLOSE_RHS, ANIM_STD_CAR_GET_IN_LHS, ANIM_STD_CAR_GET_IN_LO_LHS,
        ANIM_STD_CAR_GET_IN_LO_RHS, ANIM_STD_CAR_GET_IN_RHS, ANIM_STD_CAR_SHUFFLE_LO_RHS,
        ANIM_STD_CAR_SHUFFLE_RHS, ANIM_STD_COACH_GET_IN_LHS, ANIM_STD_COACH_GET_IN_RHS,
        ANIM_STD_COACH_GET_OUT_LHS, ANIM_STD_COACH_OPEN_LHS, ANIM_STD_COACH_OPEN_RHS,
        ANIM_STD_CRAWLOUT_LHS, ANIM_STD_CRAWLOUT_RHS, ANIM_STD_GETOUT_LHS,
        ANIM_STD_GETOUT_LO_LHS, ANIM_STD_GETOUT_LO_RHS, ANIM_STD_GETOUT_RHS,
        ANIM_STD_JACKEDCAR_LHS, ANIM_STD_JACKEDCAR_LO_LHS, ANIM_STD_JACKEDCAR_LO_RHS,
        ANIM_STD_JACKEDCAR_RHS, ANIM_STD_QUICKJACK, ANIM_STD_QUICKJACKED,
        ANIM_STD_VAN_GET_IN_REAR_LHS, ANIM_STD_VAN_GET_IN_REAR_RHS,
        ANIM_STD_VAN_GET_OUT_REAR_LHS, ANIM_STD_VAN_GET_OUT_REAR_RHS,
    };

    let duration = association.hierarchy.total_length.max(f32::EPSILON);
    let current_ratio = (association.current_time / duration).clamp(0.0, 1.0);
    match association.anim_id {
        ANIM_STD_JACKEDCAR_RHS
        | ANIM_STD_JACKEDCAR_LO_RHS
        | ANIM_STD_JACKEDCAR_LHS
        | ANIM_STD_JACKEDCAR_LO_LHS
        | ANIM_STD_VAN_GET_IN_REAR_LHS
        | ANIM_STD_VAN_GET_IN_REAR_RHS
        | ANIM_STD_QUICKJACKED
        | ANIM_STD_GETOUT_LHS
        | ANIM_STD_GETOUT_LO_LHS
        | ANIM_STD_GETOUT_RHS
        | ANIM_STD_GETOUT_LO_RHS
        | ANIM_STD_CRAWLOUT_LHS
        | ANIM_STD_CRAWLOUT_RHS
        | ANIM_STD_VAN_GET_OUT_REAR_LHS
        | ANIM_STD_VAN_GET_OUT_REAR_RHS => current_ratio,
        ANIM_STD_CAR_GET_IN_RHS
        | ANIM_STD_CAR_GET_IN_LHS
        | ANIM_STD_QUICKJACK
        | ANIM_STD_CAR_GET_IN_LO_LHS
        | ANIM_STD_CAR_GET_IN_LO_RHS
        | ANIM_STD_BOAT_DRIVE => 1.0 - current_ratio,
        ANIM_STD_CAR_CLOSE_DOOR_LHS
        | ANIM_STD_CAR_CLOSE_DOOR_LO_LHS
        | ANIM_STD_CAR_CLOSE_DOOR_RHS
        | ANIM_STD_CAR_CLOSE_DOOR_LO_RHS
        | ANIM_STD_CAR_SHUFFLE_RHS
        | ANIM_STD_CAR_SHUFFLE_LO_RHS => 0.0,
        ANIM_STD_CAR_CLOSE_LHS
        | ANIM_STD_CAR_CLOSE_RHS
        | ANIM_STD_COACH_OPEN_LHS
        | ANIM_STD_COACH_OPEN_RHS
        | ANIM_STD_COACH_GET_IN_LHS
        | ANIM_STD_COACH_GET_IN_RHS
        | ANIM_STD_COACH_GET_OUT_LHS => 1.0,
        _ => match phase {
            PedVehicleAnimationPhase::Align
            | PedVehicleAnimationPhase::LockedDoor
            | PedVehicleAnimationPhase::OpenDoor
            | PedVehicleAnimationPhase::PullOut => 1.0,
            PedVehicleAnimationPhase::DraggedOut | PedVehicleAnimationPhase::Exit => current_ratio,
            PedVehicleAnimationPhase::GetIn => 1.0 - current_ratio,
            PedVehicleAnimationPhase::CloseEnter
            | PedVehicleAnimationPhase::Shuffle
            | PedVehicleAnimationPhase::RollingDoorClose
            | PedVehicleAnimationPhase::External => 0.0,
            PedVehicleAnimationPhase::CloseExit => 1.0,
        },
    }
}

/// Immutable inputs for the existing same-pass objective planning boundary.
/// Only referenced targets/leaders are projected, before any actor advances.
#[derive(Clone, Copy)]
struct PedObjectiveTarget {
    position: CVector,
    move_state: eMoveState,
    state: PedState,
    get_up_timer: u32,
    dying_or_dead: bool,
    in_vehicle: bool,
    is_player: bool,
    locomotion_speed: Option<f32>,
}

#[derive(Default)]
pub(crate) struct PedObjectiveScratch {
    requested_targets: HashSet<usize>,
    targets: HashMap<usize, PedObjectiveTarget>,
    nearby: Vec<Entity>,
    vehicle_colliders: Vec<PedVehicleCollider>,
}

fn objective_target(ped: &CPed, animation: &PedAnimation, in_vehicle: bool, is_player: bool) -> PedObjectiveTarget {
    PedObjectiveTarget {
        position: ped.physical.entity.get_position(),
        move_state: ped.move_state,
        state: ped.ped_state,
        get_up_timer: ped.get_up_timer,
        dying_or_dead: ped.dying_or_dead(),
        in_vehicle,
        is_player,
        locomotion_speed: animation.movement_anim_clump.associations.iter()
            .find(|association| association.is_movement())
            .map(|association| association.speed),
    }
}

/// Advance the on-foot mission objectives that re3 normally feeds through
/// `CPed::Seek`/`SeekFollowingPath` in `Ped.cpp`. Script peds live in the
/// ECS world, so this system consumes their SCM objective fields and advances
/// `bScriptObjectiveCompleted` when the reference behavior reaches its goal.
///
/// This keeps the same observable gates as the reference for the common
/// mission objectives (area/char seek): move state is WALK/RUN while outside
/// the safe radius, the ped is grounded against GTA collision data every tick,
/// and the script-completed flag is raised only after reaching the target.
pub(crate) fn process_script_ped_on_foot_objectives(
    mut commands: Commands,
    obstacles: Query<&crate::peds::PedObstacle::PedWalkAroundObstacle>,
    mut avoidance_states: Query<&mut crate::peds::PedObstacle::PedObstacleAvoidance>,
    index: Res<ScriptEntityIndex>,
    mut peds: Query<
        (Entity, &ScriptPedIdentity, &mut CPed, &PedVitality, &mut PedLocomotion, &mut PedAnimation, Has<InVehicle>),
        Without<PlayerPedState>,
    >,
    vehicles: Query<(&ScriptVehicleIdentity, &VehicleCore)>,
    world_players: CWorldPlayersRead,
    file_loader: Res<CFileLoader>,
    anim_manager: Res<CAnimManager>,
    ped_routes: Res<CPedRoutes>,
    collision: Res<CCollision>,
    weapon_info: Option<Res<CWeaponInfoStore>>,
    spatial: Res<crate::control::SirenTraffic::AutomobileSpatialIndex>,
    mut scratch: Local<PedObjectiveScratch>,
    mut general: ResMut<CGeneral>,
    timer: Res<CTimer>,
) {
    let dt = timer.time_step_in_seconds().clamp(0.0, 0.05);
    if dt <= 0.0 {
        return;
    }
    let now_ms = timer.time_in_milliseconds();
    let scratch = &mut *scratch;
    scratch.requested_targets.clear();
    scratch.targets.clear();
    for (entity, identity, ped, vitality, _, _, seated) in peds.iter() {
        if index.ped(identity.handle) != Some(entity) || seated || vitality.health <= 0.0 {
            continue;
        }
        scratch.requested_targets.extend(ped.ped_in_objective);
        scratch.requested_targets.extend(ped.leader);
    }
    for (entity, identity, ped, _, _, animation, seated) in peds.iter() {
        let slot = CPoolHandle(identity.handle).index();
        if index.ped(identity.handle) == Some(entity) && scratch.requested_targets.contains(&slot) {
            scratch.targets.insert(slot, objective_target(ped, animation, seated, false));
        }
    }
    for (player_index, player) in world_players.iter_slots() {
        let Some(player) = player else { continue; };
        let slot = CPoolHandle(player.pool_handle).index();
        if scratch.requested_targets.contains(&slot) {
            let seated = world_players.relation(player_index as i32).is_some();
            scratch.targets.insert(slot, objective_target(player.ped, player.animation, seated, true));
        }
    }

    for (ped_entity, identity, mut ped, vitality, mut locomotion, mut animation, seated) in &mut peds {
        if index.ped(identity.handle) != Some(ped_entity) || seated || vitality.health <= 0.0 {
            continue;
        }
        let ped = &mut *ped;
        let locomotion = &mut *locomotion;
        let animation = &mut *animation;
        if !ped.is_ped_in_control(vitality) {
            // CPed::ProcessObjective leaves fall/get-up/death to their state
            // handlers. An unfinished SCM objective cannot cancel recovery.
            continue;
        }
        if ped.physical.zone_level > crate::collision::ColModel::LEVEL_GENERIC as i8
            && ped.physical.zone_level != collision.collision_in_memory as i8
        {
            // CPed/CCivilianPed::ProcessControl returns immediately when this
            // ped belongs to an island whose collision is not resident.
            continue;
        }
        let target_slot = ped.ped_in_objective;
        let objective = ped.objective;
        let target = match objective {
            eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS
            | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
            | eObjective::OBJECTIVE_RUN_TO_AREA
            | eObjective::OBJECTIVE_FOLLOW_ROUTE => Some(ped.next_route_point_pos),
            eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
            | eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION
            | eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
            | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
            | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE
            | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS => {
                target_slot.and_then(|slot| scratch.targets.get(&slot).map(|target| target.position))
            }
            eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
            | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
                if ped.ped_state == PedState::PED_SEEK_CAR =>
            {
                Some(ped.seek_pos)
            }
            _ => None,
        };
        let is_kill = matches!(
            objective,
            eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
        );
        let is_flee = matches!(
            objective,
            eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE
                | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS
        );
        if (is_kill || is_flee) && target_slot.and_then(|slot| scratch.targets.get(&slot)).is_some_and(|target| target.dying_or_dead) {
            ped.flags.objective_completed = true;
            ped.flags.script_objective_completed = true;
            ped.set_objective(vitality, eObjective::OBJECTIVE_NONE, now_ms);
            if ped.char_created_by == super::Ped::RANDOM_CHAR {
                ped.flags.start_wander_path_on_foot = true;
            }
            ped.ped_state = PedState::PED_IDLE;
            ped.move_state = eMoveState::PEDMOVE_STILL;
            ped.physical.move_speed = CVector::ZERO;
            continue;
        }
        let Some(mut target) = target else {
            if is_kill || is_flee {
                ped.flags.objective_completed = true;
                ped.flags.script_objective_completed = true;
                ped.set_objective(vitality, eObjective::OBJECTIVE_NONE, now_ms);
                if ped.char_created_by == super::Ped::RANDOM_CHAR {
                    ped.flags.start_wander_path_on_foot = true;
                }
                ped.ped_state = PedState::PED_IDLE;
                ped.move_state = eMoveState::PEDMOVE_STILL;
                ped.physical.move_speed = CVector::ZERO;
            }
            continue;
        };

        // Sector membership stores identities; all bounds and poses are
        // borrowed from the current vehicle components, including new SCM cars.
        let position = ped.physical.entity.get_position();
        spatial.candidates(position, 20.0, &mut scratch.nearby);
        scratch.vehicle_colliders.clear();
        scratch.vehicle_colliders.extend(scratch.nearby.iter().filter_map(|&entity| {
            let (identity, core) = vehicles.get(entity).ok()?;
            if index.vehicle(identity.handle) != Some(entity) || !core.physical.entity.flags.uses_collision {
                return None;
            }
            let Some(col) = file_loader.model_info.col_model(core.physical.entity.model_index.max(0) as usize) else {
                bevy::log::warn_once!(target: "re3bevy::ped_seek", "vehicle obstacle waiting for original COL");
                return None;
            };
            Some(PedVehicleCollider {
                vehicle_slot: CPoolHandle(identity.handle).index(),
                matrix: core.physical.entity.matrix,
                min: CVector::new(col.bounding_box.min.x, col.bounding_box.min.y, col.bounding_box.min.z),
                max: CVector::new(col.bounding_box.max.x, col.bounding_box.max.y, col.bounding_box.max.z),
            })
        }));
        let vehicle_colliders = &scratch.vehicle_colliders;
        let mut position = ped.physical.entity.get_position();
        if is_flee {
            if objective == eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE {
                if ped.objective_timer == 0 {
                    ped.objective_timer = now_ms.wrapping_add(6_000);
                } else if now_ms >= ped.objective_timer {
                    ped.flags.objective_completed = true;
                    ped.flags.script_objective_completed = true;
                    ped.set_objective(vitality, eObjective::OBJECTIVE_NONE, now_ms);
                    if ped.char_created_by == super::Ped::RANDOM_CHAR {
                        ped.flags.start_wander_path_on_foot = true;
                    }
                    ped.ped_state = PedState::PED_IDLE;
                    ped.move_state = eMoveState::PEDMOVE_STILL;
                    ped.physical.move_speed = CVector::ZERO;
                    continue;
                }
            }
            let away_x = position.x - target.x;
            let away_y = position.y - target.y;
            let away_len = away_x.hypot(away_y);
            if away_len > f32::EPSILON {
                target = CVector::new(
                    position.x + away_x / away_len * 12.0,
                    position.y + away_y / away_len * 12.0,
                    position.z,
                );
            } else {
                let (sin, cos) = locomotion.rotation_cur.sin_cos();
                target = CVector::new(position.x - sin * 12.0, position.y + cos * 12.0, position.z);
            }
        }
        let is_seeking_car = matches!(
            objective,
            eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
        );
        if is_seeking_car
            && let Some(vehicle_slot) = ped.car_in_objective
            && let Some(detour) = possibly_find_better_pos_to_seek_car(
                position,
                target,
                ped.veh_door,
                vehicle_slot,
                vehicle_colliders,
            )
        {
            // re3 `CPed::SeekCar` writes the temporary corner into
            // `m_vecSeekPos` for this tick only. The next objective pass still
            // owns the authored door target and recomputes the detour.
            target = detour;
        }
        let area_objective = matches!(
            objective,
            eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS
                | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
                | eObjective::OBJECTIVE_RUN_TO_AREA
        );
        let route_objective = objective == eObjective::OBJECTIVE_FOLLOW_ROUTE;
        let entity_seek_objective = matches!(
            objective,
            eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION
        );
        let path_seek_objective =
            area_objective || route_objective || entity_seek_objective || is_kill || is_flee;
        let target_is_in_vehicle =
            target_slot.and_then(|slot| scratch.targets.get(&slot)).is_some_and(|target| target.in_vehicle);
        let weapon_range = if is_kill {
            if ped.current_weapon == eWeaponType::WEAPONTYPE_UNARMED {
                1.3
            } else {
                let Some(range) = weapon_info
                    .as_deref()
                    .and_then(|store| store.GetWeaponInfo(ped.current_weapon))
                    .map(|info| info.range)
                    .filter(|range| *range > 0.0)
                else {
                    bevy::log::warn_once!("npc_kill_objective_waiting_original_weapon_range");
                    continue;
                };
                range
            }
        } else {
            0.0
        };
        let mut safe_distance = if is_seeking_car {
            crate::peds::Ped::SEEK_CAR_DONE_DISTANCE
        } else if is_kill {
            if ped.current_weapon == eWeaponType::WEAPONTYPE_UNARMED {
                weapon_range
            } else {
                weapon_range / 3.0
            }
        } else if objective == eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT {
            if target_is_in_vehicle { 3.0 } else { 2.0 }
        } else {
            ped.distance_to_count_seek_done.max(0.5)
        };
        if is_kill && target_slot.and_then(|slot| scratch.targets.get(&slot))
            .is_some_and(|target| target.get_up_timer > now_ms) {
            safe_distance = safe_distance.max(2.5);
        }
        // re3 `CPed::Seek`: when the authored seek point is occupied by a car,
        // count the seek done at 2.5 m instead of walking into/pushing it.
        if area_objective && script_ped_vehicle_position_blocked(target, vehicle_colliders) {
            safe_distance = safe_distance.max(2.5);
        }

        let final_dx = target.x - position.x;
        let final_dy = target.y - position.y;
        let mut final_distance = (final_dx * final_dx + final_dy * final_dy).sqrt();
        if is_kill {
            let target_state = target_slot.and_then(|slot| scratch.targets.get(&slot))
                .map(|target| target.state);
            let distance_3d = (target - position).magnitude();
            let reloading = animation.movement_anim_clump.associations.iter().any(|association| {
                matches!(association.anim_id,
                    crate::animation::AnimationId::ANIM_STD_HGUN_RELOAD
                    | crate::animation::AnimationId::ANIM_STD_AK_RELOAD)
            });
            if distance_3d < weapon_range && now_ms > ped.attack_timer && !reloading
                && ped.wait_state != crate::peds::Ped::eWaitState::WAITSTATE_SURPRISE
                && !matches!(target_state, Some(PedState::PED_GETUP | PedState::PED_DRAG_FROM_CAR)) {
                ped.next_path_node = None;
                ped.flags.script_objective_completed = false;
                if ped.ped_state != PedState::PED_ATTACK { ped.set_stored_state(); }
                ped.ped_state = PedState::PED_ATTACK;
                ped.flags.is_attacking = true;
                ped.move_state = eMoveState::PEDMOVE_STILL;
                ped.physical.move_speed = CVector::ZERO;
                if weapon_range <= 5.0 {
                    let shoot_time = general.get_random_number_in_range_f32(0.0, 500.0) as u32;
                    ped.set_shoot_timer(now_ms, shoot_time);
                    let attack_time = general.get_random_number_in_range_f32(0.0, 1500.0) as u32;
                    ped.set_attack_timer(now_ms, attack_time);
                } else {
                    let shoot_time = general.get_random_number_in_range_f32(500.0, 2000.0) as u32;
                    ped.set_shoot_timer(now_ms, shoot_time);
                    let attack_time = if distance_3d <= safe_distance {
                        general.get_random_number_in_range_f32(100.0, 500.0)
                    } else {
                        general.get_random_number_in_range_f32(1500.0, 3000.0)
                    } as u32;
                    ped.set_attack_timer(now_ms, attack_time);
                }
                ped.flags.is_pointing_gun_at = weapon_info.as_ref()
                    .and_then(|store| store.GetWeaponInfo(ped.current_weapon))
                    .is_some_and(|info| info.IsFlagSet(crate::weapons::WeaponInfo::WEAPONFLAG_CANAIM));
            }
            if now_ms > ped.ped_state_timer {
                locomotion.rotation_dest = CGeneral::get_radian_angle_between_points(
                    target.x, target.y, position.x, position.y,
                );
            }
            if ped.ped_state == PedState::PED_ATTACK { continue; }
        }
        if route_objective && ped.have_reached_next_point_on_route(1.0) {
            let Some(next_point) = ped.get_next_point_on_route(&ped_routes) else {
                ped.flags.objective_completed = true;
                ped.flags.script_objective_completed = true;
                ped.ped_state = PedState::PED_IDLE;
                ped.move_state = eMoveState::PEDMOVE_STILL;
                ped.physical.move_speed = CVector::ZERO;
                continue;
            };
            let Some(next_position) = ped_routes.get_point_position(next_point) else {
                bevy::log::warn!(
                    "script ped route {} selected invalid point {}",
                    ped.route_last_point,
                    next_point
                );
                ped.flags.objective_completed = true;
                ped.flags.script_objective_completed = true;
                ped.ped_state = PedState::PED_IDLE;
                ped.move_state = eMoveState::PEDMOVE_STILL;
                ped.physical.move_speed = CVector::ZERO;
                continue;
            };
            ped.next_route_point_pos = next_position;
            target = next_position;
            final_distance = (target.x - position.x).hypot(target.y - position.y);
        } else if final_distance <= safe_distance && !is_flee && !route_objective {
            if is_seeking_car {
                ped.move_state = eMoveState::PEDMOVE_STILL;
                ped.physical.move_speed = CVector::ZERO;
                continue;
            }
            if is_kill {
                // Native maxDistToKeep is a seek radius, not the strike range.
                // Waiting for the attack timer must not renew bIsAttacking.
                ped.ped_state = PedState::PED_IDLE;
                ped.move_state = eMoveState::PEDMOVE_STILL;
                ped.physical.move_speed = CVector::ZERO;
                update_non_player_movement_animation(ped, locomotion, vitality, animation,
                    &anim_manager, &file_loader, dt, false, None);
                ped.calculate_new_velocity(locomotion, vitality, timer.time_step(), false);
                ped.physical.entity.set_heading(locomotion.rotation_cur);
                continue;
            }
            ped.next_path_node = None;
            ped.flags.script_objective_completed = true;
            ped.ped_state = PedState::PED_IDLE;
            ped.move_state = eMoveState::PEDMOVE_STILL;
            ped.physical.move_speed = CVector::ZERO;
            ped.action_x = 0.0;
            ped.action_y = 0.0;
            ped.ped_state_timer = 0;
            // CPed::Seek calls SetMoveAnim on completion as well as while
            // moving. Keep the same clump alive for the idle transition.
            update_non_player_movement_animation(ped, locomotion, vitality, animation,
                &anim_manager, &file_loader, dt, false, None);
            continue;
        }

        let target_move_state = target_slot.and_then(|slot| scratch.targets.get(&slot).map(|target| target.move_state));
        let move_state =
            script_ped_seek_move_state(ped, objective, final_distance, target_move_state);
        ped.move_state = move_state;
        // re3 `ProcessObjective` feeds area objectives through
        // `FindBestCoordsFromNodes` whenever `bUsePedNodeSeek` is set.  Keep
        // the authored destination in `target`; `seek_target` may temporarily
        // be a PATH_PED node and is discarded once that node is reached.
        let mut seek_target = ped
            .next_path_node
            .and_then(|node| file_loader.path_find.path_nodes.get(node))
            .map_or(target, |node| node.pos);
        if path_seek_objective {
            let direct_static_clear = file_loader
                .world
                .process_line_of_sight_static_hit(
                    [position.x, position.y, position.z + 1.0],
                    [seek_target.x, seek_target.y, seek_target.z + 1.0],
                    &file_loader.model_info,
                    None,
                    false,
                )
                .is_none();
            let direct_vehicle_blocked =
                script_ped_segment_blocking_vehicle(position, seek_target, vehicle_colliders);
            if let Some(vehicle_index) = direct_vehicle_blocked
                && let Some(blocker) = vehicle_colliders.get(vehicle_index)
                && let Some(obstacle) = index.vehicle_handle_for_slot(blocker.vehicle_slot)
                    .and_then(|handle| index.vehicle(handle))
            {
                let previous_obstacle = obstacles.get(ped_entity).ok()
                    .map(crate::peds::PedObstacle::PedWalkAroundObstacle::entity);
                let Ok(mut avoidance) = avoidance_states.get_mut(ped_entity) else {
                    bevy::log::warn_once!(target: "re3bevy::ped_avoidance", "ped_avoidance_missing_owner");
                    continue;
                };
                if let Some(walk_around) = walk_around::native_vehicle_corner(
                    ped, locomotion, &mut avoidance, position, blocker, obstacle, previous_obstacle, vehicle_colliders,
                    &file_loader, now_ms, &mut general,
                ) {
                    if previous_obstacle != Some(obstacle) {
                        commands.entity(ped_entity).insert(
                            crate::peds::PedObstacle::PedWalkAroundObstacle::new(obstacle),
                        );
                    }
                    if (walk_around.x - position.x).hypot(walk_around.y - position.y) < 0.5 {
                        // CPed::Seek clears the temporary action at half a metre.
                        ped.action_x = 0.0;
                        ped.action_y = 0.0;
                        ped.ped_state_timer = 0;
                    } else {
                        seek_target = walk_around;
                    }
                }
            }
            if !direct_static_clear
                && seek_target == target
                && let Some(node) = find_script_ped_seek_node(
                    &file_loader.path_find,
                    position,
                    target,
                    vehicle_colliders,
                )
            {
                ped.next_path_node = Some(node);
                seek_target = file_loader.path_find.path_nodes[node].pos;
            }
        }

        if path_seek_objective {
            let action = CVector::new(ped.action_x, ped.action_y, position.z);
            if now_ms <= ped.ped_state_timer && ped.action_x != 0.0 && ped.action_y != 0.0
                && (action.x-position.x).hypot(action.y-position.y) >= 0.5 {
                seek_target = action;
            } else {
                ped.action_x = 0.0;
                ped.action_y = 0.0;
                ped.ped_state_timer = 0;
            }
        }

        let dx = seek_target.x - position.x;
        let dy = seek_target.y - position.y;
        let distance = (dx * dx + dy * dy).sqrt();
        let seeking_path_node = ped.next_path_node
            .and_then(|node| file_loader.path_find.path_nodes.get(node))
            .is_some_and(|node| node.pos == seek_target);
        if seeking_path_node && distance <= safe_distance.max(0.75) {
            ped.last_path_node = ped.next_path_node.take();
            ped.physical.move_speed = CVector::ZERO;
            // `CPed::Seek` clears m_pNextPathNode without setting the script
            // objective complete.  The next objective tick recalculates from
            // the new side of the obstruction.
            continue;
        }

        ped.flags.script_objective_completed = false;
        ped.ped_state = match objective {
            eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
            | eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION
            | eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
            | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS => PedState::PED_SEEK_ENTITY,
            eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE
            | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS => PedState::PED_FLEE_ENTITY,
            eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
            | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER => PedState::PED_SEEK_CAR,
            _ => PedState::PED_SEEK_POS,
        };
        ped.move_state = move_state;
        // `CGeneral::GetRadianAngleBetweenPoints(target, current)` returns
        // the matrix heading whose forward basis is (-sin, cos).
        let heading = CGeneral::get_radian_angle_between_points(
            seek_target.x,
            seek_target.y,
            position.x,
            position.y,
        );
        let turn = CGeneral::limit_radian_angle(heading - locomotion.rotation_cur);
        let max_turn = locomotion.heading_rate.to_radians() * dt * 50.0;
        locomotion.rotation_cur =
            CGeneral::limit_radian_angle(locomotion.rotation_cur + turn.clamp(-max_turn, max_turn));
        locomotion.rotation_dest = heading;
        let leader_is_player = ped
            .leader
            .and_then(|leader| scratch.targets.get(&leader)).is_some_and(|target| target.is_player);
        let leader_animation_speed = ped
            .leader
            .and_then(|leader| scratch.targets.get(&leader).and_then(|target| target.locomotion_speed));
        let local_delta = update_non_player_movement_animation(
            ped, locomotion, vitality, animation,
            &anim_manager,
            &file_loader,
            dt,
            leader_is_player,
            leader_animation_speed,
        );
        let (sin, cos) = locomotion.rotation_cur.sin_cos();
        let mut step_x = local_delta.x * cos - sin * local_delta.y;
        let mut step_y = local_delta.y * cos + local_delta.x * sin;
        let root_distance = step_x.hypot(step_y);
        if root_distance > distance && root_distance > f32::EPSILON {
            let scale = distance / root_distance;
            step_x *= scale;
            step_y *= scale;
        }
        position.x += step_x;
        position.y += step_y;
        if let Some(ground) = file_loader.world.find_ground_z_for_3d_coord(
            position.x,
            position.y,
            position.z + 1.0,
            &file_loader.model_info,
        ) {
            position.z = ground + crate::peds::PedPlacement::PED_FEET_OFFSET;
        }
        let start_position = ped.physical.entity.get_position();
        let resolved = resolve_ped_static_motion(start_position, position, &file_loader);
        // CPhysical's canonical dynamic COL stage resolves the moved ped.
        // A second expanded-bounding-box veto here blocks native tangent
        // movement around a corner before that stage can accept it.
        ped.physical.move_speed = crate::peds::PedLocomotion::grounded_velocity_from_displacement(
            resolved - start_position,
            timer.time_step(),
        );
        ped.physical.entity.set_heading(locomotion.rotation_cur);
        ped.physical.entity.set_position(resolved);
        ped.physical.entity.flags.has_hit_wall = resolved != position;
        ped.flags.is_in_the_air = false;
        ped.flags.is_standing = true;
    }
}

#[derive(Debug, Clone, Copy)]
enum ScriptWeaponTarget {
    Player(usize),
    Ped(i32),
}

#[derive(Debug, Clone, Copy)]
struct ScriptWeaponTargetSnapshot {
    target: ScriptWeaponTarget,
    position: CVector,
    base_position: CVector,
}

#[derive(Debug, Clone, Copy)]
struct ScriptWeaponFireIntent {
    shooter_handle: i32,
    target: ScriptWeaponTarget,
    request: WeaponFireRequest,
    weapon_type: eWeaponType,
    source: CVector,
    direction: CVector,
    range: f32,
    damage: u32,
    radius: f32,
    shooter_position: CVector,
    shooter_forward: CVector,
    shooter_velocity: CVector,
    shooter_audio_entity_id: i32,
    weapon_was_ready: bool,
    emit_automatic_flash: bool,
    anim2_playing: bool,
}

/// CPed::SelectGunIfArmed's accepted weapons and unarmed fallback.
fn select_npc_gun_if_armed(ped: &mut CPed) {
    let count = ped.max_weapon_type_allowed.max(0) as usize;
    ped.current_weapon = ped.weapons.iter()
        .take(count)
        .find(|weapon| weapon.ammo_total > 0 && matches!(weapon.weapon_type,
            eWeaponType::WEAPONTYPE_BASEBALLBAT | eWeaponType::WEAPONTYPE_COLT45
            | eWeaponType::WEAPONTYPE_UZI | eWeaponType::WEAPONTYPE_SHOTGUN
            | eWeaponType::WEAPONTYPE_M16 | eWeaponType::WEAPONTYPE_SNIPERRIFLE
            | eWeaponType::WEAPONTYPE_ROCKETLAUNCHER))
        .map_or(eWeaponType::WEAPONTYPE_UNARMED, |weapon| weapon.weapon_type);
}

#[derive(Default)]
pub(crate) struct ScriptWeaponRuntimeScratch {
    targets: HashMap<usize, ScriptWeaponTargetSnapshot>,
    requested_targets: std::collections::HashSet<usize>,
    intents: Vec<ScriptWeaponFireIntent>,
    ground_attacks: HashMap<i32, GroundAttackChoice>,
    post_frames: Vec<ScriptWeaponPostFrame>,
    ak47_flash_counter: u8,
    m16_flash_counter: u8,
    ground_obstructed_attacks: u64,
}

struct ScriptWeaponPostFrame {
    handle: i32,
    info: crate::weapons::WeaponInfo::CWeaponInfo,
    ground: PedOnGroundState,
    sample: NpcAttackAnimationSample,
}

struct MeleeCandidate {
    target: ScriptWeaponTarget,
    ped_type: super::PedType::ePedType,
}

#[derive(SystemParam)]
pub(crate) struct ScriptPedWeaponRuntime<'w> {
    game: Res<'w, re3bevy_core::core::GameState::CGameState>,
    file_loader: Res<'w, CFileLoader>,
    weapon_info: Res<'w, CWeaponInfoStore>,
    anim_manager: Res<'w, CAnimManager>,
    ped_stats: Res<'w, CPedStatsRuntime>,
    timer: Res<'w, CTimer>,
    particles: ResMut<'w, CParticle>,
    particle_systems: Option<Res<'w, crate::renderer::ParticleMgr::cParticleSystemMgr>>,
    general: ResMut<'w, CGeneral>,
    events: ResMut<'w, CEventList>,
    point_lights: Option<ResMut<'w, CPointLights>>,
    special_fx: Option<ResMut<'w, CSpecialFX>>,
    audio: Option<ResMut<'w, cDMAudio>>,
    projectiles: Option<ResMut<'w, CProjectileInfoPool>>,
    shots: Option<ResMut<'w, CShotInfoPool>>,
    bullets: Option<ResMut<'w, CBulletInfoPool>>,
    camera: Option<Res<'w, CCamera>>,
}

/// ECS counterpart of re3 CPed::Attack for script/ambient peds.
///
/// The authored weapon animation drives the fire frame; the same sampled hand
/// transform feeds muzzle effects and world collision.
pub(crate) fn process_script_ped_weapon_attacks(
    mut actors: WeaponActorAccess,
    mut world_players: CWorldPlayersCombat,
    mut runtime: ScriptPedWeaponRuntime,
    mut scratch: Local<ScriptWeaponRuntimeScratch>,
) {
    let now_ms = runtime.timer.time_in_milliseconds();
    let previous_ms = runtime.timer.previous_time_in_milliseconds();
    let camera_position = runtime.camera.as_ref().map(|camera| camera.position);
    let scratch = &mut *scratch;

    scratch.targets.clear();
    scratch.requested_targets.clear();
    for (_, entity) in actors.peds() {
        if entity.ped.ped_state == PedState::PED_ATTACK && !entity.ped.dying_or_dead()
            && let Some(target) = entity.ped.ped_in_objective.or(entity.ped.point_gun_at).or(entity.ped.seek_target) {
            scratch.requested_targets.insert(target);
        }
    }
    for &target_slot in &scratch.requested_targets {
        let Some(handle) = actors.ped_handle_for_slot(target_slot) else { continue; };
        let Some(entity) = actors.ped(handle) else { continue; };
        let model_index = entity.ped.physical.entity.model_index.max(0) as usize;
        let Some(position) = crate::renderer::BevyRender::ped_frame_world_point_from_clump(
            model_index,
            &runtime.file_loader,
            &entity.animation.movement_anim_clump,
            &entity.ped.physical.entity.matrix,
            "Smid",
            CVector::ZERO,
        ) else {
            bevy::log::warn_once!(handle, model_index, "npc_attack_waiting_original_target_mid");
            continue;
        };
        scratch.targets.insert(
            CPoolHandle(handle).index(),
            ScriptWeaponTargetSnapshot {
                target: ScriptWeaponTarget::Ped(handle),
                position,
                base_position: entity.ped.physical.entity.get_position(),
            },
        );
    }
    for player_index in 0..world_players.slot_count() {
        let Some(player) = world_players.player(player_index as i32) else {
            continue;
        };
        if !scratch.requested_targets.contains(&CPoolHandle(player.pool_handle).index()) { continue; }
        let model_index = player.ped.physical.entity.model_index.max(0) as usize;
        let Some(position) = crate::renderer::BevyRender::ped_frame_world_point_from_clump(
            model_index,
            &runtime.file_loader,
            &player.animation.movement_anim_clump,
            &player.ped.physical.entity.matrix,
            "Smid",
            CVector::ZERO,
        ) else {
            bevy::log::warn_once!(player_index, model_index, "npc_attack_waiting_original_player_mid");
            continue;
        };
        scratch.targets.insert(
            CPoolHandle(player.pool_handle).index(),
            ScriptWeaponTargetSnapshot {
                target: ScriptWeaponTarget::Player(player_index),
                position,
                base_position: player.position(),
            },
        );
    }

    scratch.intents.clear();
    scratch.post_frames.clear();
    scratch.ground_attacks.clear();
    for (handle, actor) in actors.peds() {
        if actor.ped.ped_state != PedState::PED_ATTACK || actor.ped.dying_or_dead()
            || !runtime.weapon_info.GetWeaponInfo(actor.ped.current_weapon)
                .is_some_and(|info| info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE) { continue; }
        let choice = check_for_peds_on_ground_to_attack(actor.ped, actor.locomotion.rotation_cur,
            runtime.game.nasty_game, |near_handle| {
                let (ped, animation) = if let Some(near) = actors.ped(near_handle) {
                    (near.ped, near.animation)
                } else {
                    let near = world_players.iter().find(|player| player.pool_handle == near_handle)?;
                    (near.ped, near.animation)
                };
                let head_height = if ped.on_ground() || ped.ped_state == PedState::PED_GETUP {
                    crate::renderer::BevyRender::ped_frame_world_point_from_clump(
                        ped.physical.entity.model_index.max(0) as usize, &runtime.file_loader,
                        &animation.movement_anim_clump, &ped.physical.entity.matrix, "Shead", CVector::ZERO,
                    ).map(|head| head.z - ped.physical.entity.get_position().z)
                } else { None };
                Some(GroundAttackCandidate { position: ped.physical.entity.get_position(), state: ped.ped_state, head_height })
            });
        if let Some(choice) = choice { scratch.ground_attacks.insert(handle, choice); }
        else { bevy::log::warn_once!(handle, "npc_ground_attack_waiting_original_head"); }
    }
    for (handle, entity) in actors.peds_mut() {
        let ped = &mut *entity.ped;
        let locomotion = &mut *entity.locomotion;
        let vitality = &mut *entity.vitality;
        let animation = &mut *entity.animation;
        if let Some(weapon) = ped.weapons.get_mut(ped.current_weapon as usize) {
            let play_reload = weapon.Update(previous_ms, now_ms, ped.physical.audio_entity_id >= 0, &runtime.weapon_info);
            if play_reload && let Some(audio) = runtime.audio.as_deref_mut() {
                audio.PlayOneShot(ped.physical.audio_entity_id, SOUND_WEAPON_RELOAD as u16, 0.0);
            }
        }
        if ped.ped_state != PedState::PED_ATTACK || ped.dying_or_dead() {
            ped.weapon_attack_animation_id = None;
            ped.flags.is_attacking = false;
            ped.flags.is_shooting = false;
            continue;
        }

        // Objective processing skips locomotion when it enters PED_ATTACK.
        // Advance this clump once here, using the canonical animation updater.
        update_non_player_movement_animation(
            ped, locomotion, vitality, animation, &runtime.anim_manager, &runtime.file_loader,
            runtime.timer.time_step_in_seconds(), false, None,
        );
        // CPed::ProcessControl turns the canonical heading before Attack reads
        // matrix-based fire offsets and forward vectors for melee weapons.
        ped.calculate_new_velocity(locomotion, vitality, runtime.timer.time_step(), false);
        if ped.is_ped_in_control(vitality) {
            ped.physical.entity.set_heading(locomotion.rotation_cur);
        }

        let Some(target_slot) = ped
            .ped_in_objective
            .or(ped.point_gun_at)
            .or(ped.seek_target)
        else {
            continue;
        };
        let Some(target) = scratch.targets.get(&target_slot).copied() else {
            continue;
        };
        if matches!(target.target, ScriptWeaponTarget::Ped(target_handle) if target_handle == handle)
        {
            continue;
        }

        let weapon_type = ped.current_weapon;
        let slot = weapon_type as usize;
        let Some(info) = runtime.weapon_info.GetWeaponInfo(weapon_type).copied() else {
            bevy::log::warn_once!(handle, ?weapon_type, "npc_attack_waiting_original_weapon_info");
            continue;
        };
        let ground = if info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE {
            let Some(choice) = scratch.ground_attacks.get(&handle) else { continue; };
            if let Some(heading) = choice.normalized_heading { locomotion.rotation_cur = heading; }
            choice.state
        } else { PedOnGroundState::NoPed };
        service_attack_finish(ped, vitality, animation, &runtime.anim_manager, &info,
            &runtime.file_loader, &mut runtime.general, now_ms);
        if ped.ped_state != PedState::PED_ATTACK { continue; }
        if info.weapon_fire == eWeaponFire::WEAPON_FIRE_INSTANT_HIT
            && ped.seek_target.is_some()
        {
            // CPed::SetAttack calls HitsGround with a null fireSource.
            let source = ped.physical.entity.transform_point(CVector::new(
                info.fire_offset.x, info.fire_offset.y, info.fire_offset.z + 0.6,
            ));
            let ground_target = CVector::new(
                target.base_position.x, target.base_position.y, target.base_position.z + 0.6,
            );
            if runtime.file_loader.world.process_line_of_sight_building_contact(
                [source.x, source.y, source.z],
                [ground_target.x, ground_target.y, ground_target.z],
                &runtime.file_loader.model_info,
            ).is_some_and(|hit| {
                let difference = hit.point[2] - source.z;
                difference < 0.0 && difference > -3.0
            }) {
                scratch.ground_obstructed_attacks = scratch.ground_obstructed_attacks.saturating_add(1);
                bevy::log::trace!(target: "re3bevy::weapons",
                    blocked_attacks = scratch.ground_obstructed_attacks,
                    shooter_handle = handle, "NPC HitsGround blocked attack");
                continue;
            }
        }
        let Some(sample) =
            sample_attack_animation(ped, animation, &runtime.anim_manager, &info, ground, now_ms)
        else {
            continue;
        };
        let attack_animation_id = sample.animation_id;
        let previous_elapsed = sample.previous_time;
        let elapsed = sample.current_time;
        let running = sample.running;
        scratch.post_frames.push(ScriptWeaponPostFrame { handle, info, ground, sample });
        let fire_offset = super::PedWeaponAttack::fire_offset(ped, &info, attack_animation_id);
        let Some(weapon) = ped.weapons.get_mut(slot) else {
            bevy::log::warn_once!(target: "re3bevy::weapons", "NPC selected weapon slot is invalid");
            continue;
        };
        let source = if matches!(weapon_type, eWeaponType::WEAPONTYPE_UNARMED | eWeaponType::WEAPONTYPE_BASEBALLBAT) {
            ped.physical.entity.transform_point(fire_offset)
        } else {
            let node = if attack_animation_id == crate::animation::AnimationId::ANIM_STD_KICKGROUND {
                "Rfoot"
            } else { "Rhand" };
            let model_index = ped.physical.entity.model_index.max(0) as usize;
            let Some(source) = crate::renderer::BevyRender::ped_frame_world_point_from_clump(
                model_index,
                &runtime.file_loader,
                &animation.movement_anim_clump,
                &ped.physical.entity.matrix,
                node,
                fire_offset,
            ) else {
                bevy::log::warn_once!(handle, model_index, node, "npc_attack_waiting_original_fire_node");
                continue;
            };
            source
        };

        if weapon_type == eWeaponType::WEAPONTYPE_SHOTGUN
            && running && previous_elapsed <= 1.0 && elapsed > 1.0
        {
            let mut shell_source = source;
            shell_source.z -= 0.15 * ped.physical.entity.get_up().z;
            add_weapon_gunshell(
                &mut runtime.particles.emitter_with_context(runtime.particle_systems.as_deref(), camera_position),
                &mut runtime.general,
                shell_source,
                ped.physical.entity.get_forward(),
                0.6,
                0.025,
            );
        }

        let fire_frame = sample.fire_frame;
        let crossed_fire_frame = running && previous_elapsed <= fire_frame && elapsed > fire_frame;
        if !crossed_fire_frame {
            continue;
        }


        let to_target = CVector::new(
            target.position.x - source.x,
            target.position.y - source.y,
            target.position.z - source.z,
        );
        let target_distance = to_target.magnitude();
        if target_distance <= 1.0e-6 {
            continue;
        }
        let context = WeaponFireContext {
            shooter_is_player: false,
            shooter_has_seek_target: true,
            seek_target_distance: target_distance,
            player_attack_button_counter: 0.0,
            m16_first_person: false,
            helicannon_first_person: false,
        };
        let weapon_was_ready = weapon.weapon_state == eWeaponState::WEAPONSTATE_READY;
        let Some(request) = weapon.prepare_fire(now_ms, context, &runtime.weapon_info) else {
            continue;
        };

        let direction = if matches!(request, WeaponFireRequest::Melee) {
            ped.physical.entity.get_forward()
        } else {
            let inv_distance = target_distance.recip();
            let mut aim_target = CVector::new(
                source.x + to_target.x * inv_distance * info.range,
                source.y + to_target.y * inv_distance * info.range,
                source.z + to_target.z * inv_distance * info.range,
            );
            let inaccuracy = 100_i32.saturating_sub(i32::from(ped.wep_accuracy)) as f32;
            if inaccuracy > 0.0 {
                aim_target.x += runtime.general.get_random_number_in_range_f32(-0.2, 0.2)
                    * inaccuracy;
                aim_target.y += runtime.general.get_random_number_in_range_f32(-0.2, 0.2)
                    * inaccuracy;
                aim_target.z += runtime.general.get_random_number_in_range_f32(-0.1, 0.1)
                    * inaccuracy;
            }
            let aim = CVector::new(
                aim_target.x - source.x,
                aim_target.y - source.y,
                aim_target.z - source.z,
            );
            let length = aim.magnitude().max(1.0e-6);
            CVector::new(aim.x / length, aim.y / length, aim.z / length)
        };

        let emit_automatic_flash = match weapon_type {
            eWeaponType::WEAPONTYPE_AK47 => {
                scratch.ak47_flash_counter = scratch.ak47_flash_counter.wrapping_add(1);
                scratch.ak47_flash_counter & 1 == 0
            }
            eWeaponType::WEAPONTYPE_M16 => {
                scratch.m16_flash_counter = scratch.m16_flash_counter.wrapping_add(1);
                scratch.m16_flash_counter & 1 == 0
            }
            _ => true,
        };
        scratch.intents.push(ScriptWeaponFireIntent {
            shooter_handle: handle,
            target: target.target,
            request,
            weapon_type,
            source,
            direction,
            range: info.range,
            damage: info.damage,
            radius: info.radius,
            shooter_position: ped.physical.entity.get_position(),
            shooter_forward: ped.physical.entity.get_forward(),
            shooter_velocity: ped.physical.move_speed,
            shooter_audio_entity_id: ped.physical.audio_entity_id,
            weapon_was_ready,
            emit_automatic_flash,
            anim2_playing: animation.movement_anim_clump.associations.iter()
                .any(|association| association.anim_id == info.anim2_to_play),
        });
    }

    for intent in scratch.intents.drain(..) {
        let mut melee_contact = false;
        let fired = match intent.request {
            WeaponFireRequest::Melee => {
                melee_contact = apply_script_melee_hit(
                    intent,
                    &mut actors,
                    &mut world_players,
                    &runtime.file_loader,
                    &runtime.ped_stats,
                    &mut runtime.particles.emitter_with_context(runtime.particle_systems.as_deref(), camera_position),
                    &mut runtime.general,
                    &mut runtime.events,
                    now_ms,
                
    runtime.timer.frame_counter(),
);
                true
            }
            WeaponFireRequest::Shotgun => {
                emit_shotgun_muzzle_fx(
                    intent.source,
                    intent.direction,
                    &mut runtime.particles.emitter_with_context(runtime.particle_systems.as_deref(), camera_position),
                    runtime.point_lights.as_deref_mut(),
                    camera_position,
                );
                let _ = runtime.events.register_entity_event(
                    EEventType::Gunshot,
                    EEventEntity::Ped,
                    intent.shooter_handle,
                    Some(intent.shooter_handle),
                    intent.source,
                    1_000,
                    now_ms,
                );
                for pellet in 0..5 {
                    let offset = (-15.0 + pellet as f32 * 7.5).to_radians();
                    let pellet_direction = rotate_weapon_ray(intent.direction, offset);
                    apply_script_hitscan_round(
                        intent,
                        pellet_direction,
                        true,
                        &mut actors,
                        &mut world_players,
                        &runtime.file_loader,
                        &runtime.ped_stats,
                        &mut runtime.particles.emitter_with_context(runtime.particle_systems.as_deref(), camera_position),
                        runtime.special_fx.as_deref_mut(),
                        &mut runtime.general,
                        &mut runtime.events,
                        runtime.audio.as_deref_mut(),
                        now_ms,
                    
    runtime.timer.frame_counter(),
);
                }
                true
            }
            WeaponFireRequest::Sniper => {
                let cam = runtime.camera.as_ref().and_then(|camera| {
                    camera.cams.get(camera.active_cam)
                }).filter(|cam| matches!(cam.mode,
                    eCamMode::MODE_SNIPER | eCamMode::MODE_M16_1STPERSON
                    | eCamMode::MODE_ROCKETLAUNCHER));
                if let Some(cam) = cam {
                    if let Some(bullets) = runtime.bullets.as_deref_mut() {
                        let length = cam.front.magnitude();
                        if length > 0.0 {
                            bullets.AddBullet(
                                Some(intent.shooter_handle as u32), intent.weapon_type,
                                cam.source,
                                crate::weapons::WeaponRuntime::scaled_vector(cam.front, 16.0 / length),
                                now_ms, &runtime.weapon_info,
                            )
                        } else {
                            bevy::log::warn_once!(target: "re3bevy::weapons", "NPC sniper camera has invalid front vector");
                            false
                        }
                    } else {
                        bevy::log::warn_once!(target: "re3bevy::weapons", "NPC sniper waiting for moving bullet pool");
                        false
                    }
                } else {
                    false
                }
            }
            WeaponFireRequest::InstantHit
            | WeaponFireRequest::M16FirstPerson => {
                emit_instant_hit_muzzle_fx(
                    intent.weapon_type,
                    intent.source,
                    intent.direction,
                    intent.shooter_velocity,
                    intent.emit_automatic_flash,
                    &mut runtime.particles.emitter_with_context(runtime.particle_systems.as_deref(), camera_position),
                    &mut runtime.general,
                    runtime.point_lights.as_deref_mut(),
                    camera_position,
                );
                let _ = runtime.events.register_entity_event(
                    EEventType::Gunshot,
                    EEventEntity::Ped,
                    intent.shooter_handle,
                    Some(intent.shooter_handle),
                    intent.source,
                    1_000,
                    now_ms,
                );
                apply_script_hitscan_round(
                    intent,
                    intent.direction,
                    false,
                    &mut actors,
                    &mut world_players,
                    &runtime.file_loader,
                    &runtime.ped_stats,
                    &mut runtime.particles.emitter_with_context(runtime.particle_systems.as_deref(), camera_position),
                    runtime.special_fx.as_deref_mut(),
                    &mut runtime.general,
                    &mut runtime.events,
                    runtime.audio.as_deref_mut(),
                    now_ms,
                
    runtime.timer.frame_counter(),
);
                true
            }
            WeaponFireRequest::Projectile { power } => {
                let launch_position =
                    if intent.weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER {
                        CVector::new(
                            intent.source.x + intent.direction.x,
                            intent.source.y + intent.direction.y,
                            intent.source.z + intent.direction.z,
                        )
                    } else {
                        intent.source
                    };
                runtime.projectiles.as_deref_mut().is_some_and(|projectiles| {
                    projectiles.AddProjectileNative(
                        Some(intent.shooter_handle as u32),
                        intent.weapon_type,
                        launch_position,
                        intent.direction,
                        power,
                        now_ms,
                        None,
                    )
                })
            }
            WeaponFireRequest::AreaEffect => {
                // CWeapon::FireAreaEffect uses the shooter's horizontal
                // forward direction for every non-mouse-camera shooter.
                let forward = intent.shooter_forward;
                let area_direction = CVector::new(
                    forward.x * 0.5,
                    forward.y * 0.5,
                    0.0,
                );
                let target = CVector::new(
                    intent.source.x + area_direction.x,
                    intent.source.y + area_direction.y,
                    intent.source.z + area_direction.z,
                );
                let fired = runtime.shots.as_deref_mut().is_some_and(|shots| {
                    shots.AddShot(
                        Some(intent.shooter_handle as u32),
                        intent.weapon_type,
                        intent.source,
                        target,
                        now_ms,
                        &runtime.weapon_info,
                        &mut runtime.general,
                    );
                    true
                });
                if fired {
                    let mut flame_direction = area_direction;
                    for _ in 0..5 {
                        flame_direction.x *= 0.7;
                        flame_direction.y *= 0.7;
                        flame_direction.z *= 0.7;
                        runtime.particles.emitter_with_context(runtime.particle_systems.as_deref(), camera_position).add_particle(
                            crate::renderer::ParticleType::PARTICLE_FIREBALL,
                            intent.source,
                            flame_direction,
                            None,
                            0.0,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                }
                fired
            }
            WeaponFireRequest::UseDetonator => false,
        };

        if let Some(shooter) = actors.ped_mut(intent.shooter_handle) {
            let slot = intent.weapon_type as usize;
            if let Some(weapon) = shooter.ped.weapons.get_mut(slot) {
                if melee_contact {
                    weapon.weapon_state = eWeaponState::WEAPONSTATE_MELEE_MADECONTACT;
                } else {
                    weapon.complete_fire(
                        intent.request,
                        fired,
                        now_ms,
                        false,
                        false,
                        &runtime.weapon_info,
                    );
                }
            }
            shooter.ped.flags.is_shooting =
                fired && !matches!(intent.request, WeaponFireRequest::Melee);
            if shooter.ped.weapons[slot].ammo_total == 0
                && runtime.weapon_info.GetWeaponInfo(intent.weapon_type)
                    .is_some_and(|info| info.weapon_fire != eWeaponFire::WEAPON_FIRE_MELEE)
            {
                select_npc_gun_if_armed(&mut *shooter.ped);
            }
            if melee_contact {
                if let Some(animation_id) = shooter.ped.weapon_attack_animation_id {
                    let continuing = shooter.ped.flags.is_attacking || now_ms < shooter.ped.shoot_timer;
                    suppress_contact_finish(shooter.animation, animation_id, continuing);
                }
            }
            if shooter.ped.current_weapon == intent.weapon_type
                && shooter.ped.weapons[slot].weapon_state == eWeaponState::WEAPONSTATE_RELOADING
                && let Some(info) = runtime.weapon_info.GetWeaponInfo(intent.weapon_type)
                && let Some(reload_id) = crate::weapons::WeaponRuntime::reload_animation(info)
            {
                if let Some(index) = blend_ped_movement_animation(
                    &mut shooter.animation.movement_anim_clump, &runtime.anim_manager,
                    crate::animation::AnimAssocDefinitions::ASSOCGRP_STD, reload_id, 8.0,
                ) {
                    shooter.animation.movement_anim_clump.associations[index].set_finish_callback();
                    shooter.ped.flags.is_attacking = false;
                    shooter.ped.flags.is_pointing_gun_at = false;
                    shooter.ped.flags.is_aiming_gun = false;
                    shooter.ped.flags.is_looking = false;
                    shooter.ped.look_timer = 0;
                    shooter.ped.shoot_timer = now_ms;
                    shooter.ped.weapon_attack_animation_id = Some(reload_id);
                } else {
                    bevy::log::warn_once!(target: "re3bevy::weapons", "NPC reload association is missing");
                }
            }
        }

        if let Some(audio) = runtime.audio.as_deref_mut() {
            if fired && !matches!(intent.request, WeaponFireRequest::Melee) {
                audio.PlayOneShot(
                    intent.shooter_audio_entity_id,
                    SOUND_WEAPON_SHOT_FIRED as u16,
                    0.0,
                );
                if intent.weapon_type == eWeaponType::WEAPONTYPE_FLAMETHROWER
                    && intent.weapon_was_ready
                {
                    audio.PlayOneShot(
                        intent.shooter_audio_entity_id,
                        SOUND_WEAPON_FLAMETHROWER_FIRE as u16,
                        0.0,
                    );
                }
            }
            if melee_contact {
                let sound = if intent.weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT {
                    SOUND_WEAPON_BAT_ATTACK
                } else {
                    SOUND_FIGHT_PUNCH_39
                };
                let parameter = if intent.weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT { 1.0 } else { 0.0 };
                audio.PlayOneShot(intent.shooter_audio_entity_id, sound as u16, parameter);
            }
        }
    }
    for post in scratch.post_frames.drain(..) {
        let Some(shooter) = actors.ped_mut(post.handle) else { continue; };
        if shooter.ped.ped_state != PedState::PED_ATTACK { continue; }
        if let Some(sound) = finish_attack_frame(shooter.ped, shooter.animation, &runtime.anim_manager,
            &post.info, post.ground, post.sample, now_ms)
            && let Some(audio) = runtime.audio.as_deref_mut() {
            audio.PlayOneShot(shooter.ped.physical.audio_entity_id, sound, 0.0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_script_hitscan_round(
    intent: ScriptWeaponFireIntent,
    direction: CVector,
    shotgun: bool,
    actors: &mut WeaponActorAccess<'_, '_>,
    world_players: &mut CWorldPlayersCombat<'_, '_>,
    file_loader: &CFileLoader,
    ped_stats: &CPedStatsRuntime,
    particles: &mut ParticleEmitter<'_>,
    mut special_fx: Option<&mut CSpecialFX>,
    general: &mut CGeneral,
    events: &mut CEventList,
    mut audio: Option<&mut cDMAudio>,
    now_ms: u32,

    frame_counter: u32,
) {
    let target_player = match intent.target {
        ScriptWeaponTarget::Player(index) => world_players.player(index as i32),
        ScriptWeaponTarget::Ped(_) => None,
    };
    let hit = closest_weapon_ray_hit(
        intent.source,
        direction,
        intent.range,
        target_player,
        Some(intent.shooter_handle),
        None,
        actors,
        file_loader,
    );
    bevy::log::trace!(target: "re3bevy::weapons", shooter = intent.shooter_handle,
        source = ?intent.source, direction = ?direction, hit = ?hit, "NPC weapon ray");
    let mut trace_end = CVector::new(
        intent.source.x + direction.x * intent.range.min(30.0),
        intent.source.y + direction.y * intent.range.min(30.0),
        intent.source.z + direction.z * intent.range.min(30.0),
    );
    let blood_amount = if shotgun { 32 } else { 8 };

    match hit {
        Some(WeaponRayHit::Player { point, .. }) => {
            trace_end = point;
            if let ScriptWeaponTarget::Player(index) = intent.target
                && let Some(mut victim) = world_players.player_mut(index as i32)
            {
                let victim_position = victim.position();
                let can_be_damaged = victim.can_be_damaged;
                let local_direction =
                    apply_player_gun_hit_reaction(&mut victim, intent.weapon_type, intent.source, now_ms,
    frame_counter,
);
                let weakness = ped_defend_weakness(&victim.ped, Some(ped_stats));
                let _ = victim.ped.inflict_weapon_damage(victim.vitality, 
                    intent.weapon_type,
                    intent.damage as f32,
                    weakness,
                    false,
                    can_be_damaged,
                    now_ms,
                    local_direction,
                );
                emit_weapon_blood(
                    particles,
                    point,
                    intent.source,
                    victim_position,
                    blood_amount,
                );
                let _ = events.register_entity_event(
                    EEventType::ShootPed,
                    EEventEntity::Ped,
                    victim.pool_handle,
                    Some(intent.shooter_handle),
                    victim_position,
                    10_000,
                    now_ms,
                );
                if let Some(audio) = audio.as_deref_mut() {
                    audio.PlayOneShot(victim.ped.physical.audio_entity_id, SOUND_WEAPON_HIT_PED as u16, 1.0);
                }
            }
        }
        Some(WeaponRayHit::Ped { handle, point, .. }) => {
            trace_end = point;
            if let Some(victim) = actors.ped_mut(handle) {
                let victim_position = victim.ped.physical.entity.get_position();
                let local_direction =
                    apply_ped_gun_hit_reaction(&mut *victim.ped, victim.locomotion.rotation_cur, &victim.vitality, intent.weapon_type, intent.source, now_ms,
    frame_counter,
);
                let weakness = ped_defend_weakness(&victim.ped, Some(ped_stats));
                let _ = victim.ped.inflict_weapon_damage(victim.vitality, 
                    intent.weapon_type,
                    intent.damage as f32,
                    weakness,
                    false,
                    true,
                    now_ms,
                    local_direction,
                );
                emit_weapon_blood(
                    particles,
                    point,
                    intent.source,
                    victim_position,
                    blood_amount,
                );
                let event_type = if victim.ped.ped_type == crate::peds::PedType::ePedType::PEDTYPE_COP {
                    EEventType::ShootCop
                } else {
                    EEventType::ShootPed
                };
                let _ = events.register_entity_event(
                    event_type,
                    EEventEntity::Ped,
                    handle,
                    Some(intent.shooter_handle),
                    victim_position,
                    10_000,
                    now_ms,
                );
                if let Some(audio) = audio.as_deref_mut() {
                    audio.PlayOneShot(victim.ped.physical.audio_entity_id, SOUND_WEAPON_HIT_PED as u16, 1.0);
                }
            }
        }
        Some(WeaponRayHit::Vehicle {
            handle,
            point,
            normal,
            ..
        }) => {
            trace_end = point;
            if let Some((core, vitality)) = actors.vehicle_damage(handle) {
                vitality.inflict_weapon_damage_from_source(
                    core,
                    intent.weapon_type,
                    intent.damage as f32,
                    false,
                );
                emit_bullet_surface_impact(particles, point, normal, 16, general);
                if let Some(audio) = audio.as_deref_mut() {
                    audio.PlayOneShot(
                        core.physical.audio_entity_id,
                        SOUND_WEAPON_HIT_VEHICLE as u16,
                        1.0,
                    );
                }
            }
        }
        Some(WeaponRayHit::Object {
            handle,
            point,
            normal,
            ..
        }) => {
            trace_end = point;
            emit_bullet_sparks(particles, point, normal, 8);
            if let Some(object) = actors.object_mut(handle) {
                apply_weapon_force_to_object(
                    object,
                    normal,
                    if shotgun { 5.0 } else { 4.0 },
                );
            }
        }
        Some(WeaponRayHit::Static { point, normal, .. }) => {
            trace_end = point;
            emit_bullet_surface_impact(particles, point, normal, 16, general);
        }
        None => {}
    }

    if let Some(special_fx) = special_fx.as_deref_mut() {
        special_fx.add_native_bullet_trace(intent.source, trace_end, general);
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_script_melee_hit(
    intent: ScriptWeaponFireIntent,
    actors: &mut WeaponActorAccess<'_, '_>,
    world_players: &mut CWorldPlayersCombat<'_, '_>,
    file_loader: &CFileLoader,
    ped_stats: &CPedStatsRuntime,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
    events: &mut CEventList,
    now_ms: u32,

    frame_counter: u32,
) -> bool {
    let Some(shooter) = actors.ped(intent.shooter_handle) else {
        bevy::log::warn_once!("npc_melee_shooter_retired_before_fire");
        return false;
    };
    let near_peds = shooter.ped.near_peds;
    let count = usize::from(shooter.ped.num_near_peds);
    let ped_type = shooter.ped.ped_type;
    let seek_target = shooter.ped.seek_target;
    let leader = shooter.ped.leader;
    let mut made_contact = false;
    for handle in near_peds.iter().take(count).flatten().copied() {
        let candidate = if let Some(victim) = actors.ped(handle) {
            Some(MeleeCandidate { target: ScriptWeaponTarget::Ped(handle), ped_type: victim.ped.ped_type })
        } else {
            (0..world_players.slot_count()).find_map(|index| {
                let victim = world_players.player(index as i32)?;
                (victim.pool_handle == handle).then_some(MeleeCandidate {
                    target: ScriptWeaponTarget::Player(index), ped_type: victim.ped.ped_type,
                })
            })
        };
        let Some(candidate) = candidate else { continue; };
        let slot = CPoolHandle(handle).index();
        if !((candidate.ped_type != ped_type || seek_target == Some(slot)) && leader != Some(slot))
            && general.get_random_number() & 31 != 0 { continue; }
        let strike = ScriptWeaponFireIntent { target: candidate.target, ..intent };
        made_contact |= apply_script_melee_target(strike, actors, world_players, file_loader,
            ped_stats, particles, general, events, now_ms,
    frame_counter,
);
    }
    made_contact
}

#[allow(clippy::too_many_arguments)]
fn apply_script_melee_target(
    intent: ScriptWeaponFireIntent,
    actors: &mut WeaponActorAccess<'_, '_>,
    world_players: &mut CWorldPlayersCombat<'_, '_>,
    file_loader: &CFileLoader,
    ped_stats: &CPedStatsRuntime,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
    events: &mut CEventList,
    now_ms: u32,

    frame_counter: u32,
) -> bool {
    match intent.target {
        ScriptWeaponTarget::Player(index) => {
            let Some(mut victim) = world_players.player_mut(index as i32) else {
                return false;
            };
            let Some(collision_dist) =
                melee_ped_collision(&victim.ped, &victim.animation, intent.source, intent.radius, file_loader)
            else {
                return false;
            };
            if victim.ped.ped_state == PedState::PED_GETUP {
                return false;
            }
            let victim_position = victim.position();
            let health_before = victim.vitality.health;
            let local_direction = victim.ped.get_local_direction(victim.locomotion.rotation_cur, CVector::new(
                intent.shooter_position.x - victim_position.x,
                intent.shooter_position.y - victim_position.y,
                0.0,
            ));
            let weakness = ped_defend_weakness(&victim.ped, Some(ped_stats));
            let damage = if intent.weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT {
                2.0 * intent.damage as f32
            } else {
                intent.damage as f32
            };
            let can_be_damaged = victim.can_be_damaged;
            if !victim.ped.dying_or_dead() { let _ = victim.ped.inflict_weapon_damage(victim.vitality,
                intent.weapon_type,
                damage,
                weakness,
                false,
                can_be_damaged,
                now_ms,
                local_direction,
            ); }
            apply_melee_fall_response(
                &mut victim.ped, &victim.vitality,
                intent.weapon_type,
                intent.anim2_playing,
                health_before,
                intent.shooter_position,
                victim_position,
                local_direction,
                now_ms,
            
    frame_counter,
);
            // TODO(parity): CWeapon::FireMelee's GetIsOnScreen gate and
            // ReactToAttack/StartFightDefend need the canonical fight interface.
            emit_melee_blood(
                particles,
                intent.source,
                collision_dist,
                intent.weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT,
                general,
            );
            let _ = events.register_entity_event(
                EEventType::Assault,
                EEventEntity::Ped,
                victim.pool_handle,
                Some(intent.shooter_handle),
                victim_position,
                2_000,
                now_ms,
            );
            true
        }
        ScriptWeaponTarget::Ped(handle) => {
            let Some(victim) = actors.ped_mut(handle) else {
                return false;
            };
            let Some(collision_dist) =
                melee_ped_collision(&victim.ped, &victim.animation, intent.source, intent.radius, file_loader)
            else {
                return false;
            };
            let victim_position = victim.ped.physical.entity.get_position();
            let health_before = victim.vitality.health;
            let local_direction = victim.ped.get_local_direction(victim.locomotion.rotation_cur, CVector::new(
                intent.shooter_position.x - victim_position.x,
                intent.shooter_position.y - victim_position.y,
                0.0,
            ));
            let weakness = ped_defend_weakness(&victim.ped, Some(ped_stats));
            if !victim.ped.dying_or_dead() { let _ = victim.ped.inflict_weapon_damage(victim.vitality,
                intent.weapon_type,
                intent.damage as f32,
                weakness,
                false,
                true,
                now_ms,
                local_direction,
            ); }
            apply_melee_fall_response(
                &mut *victim.ped, &victim.vitality,
                intent.weapon_type,
                intent.anim2_playing,
                health_before,
                intent.shooter_position,
                victim_position,
                local_direction,
                now_ms,
            
    frame_counter,
);
            emit_melee_blood(
                particles,
                intent.source,
                collision_dist,
                intent.weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT,
                general,
            );
            let event_type =
                if victim.ped.ped_type == crate::peds::PedType::ePedType::PEDTYPE_COP {
                    EEventType::AssaultPolice
                } else {
                    EEventType::Assault
                };
            let _ = events.register_entity_event(
                event_type,
                EEventEntity::Ped,
                handle,
                Some(intent.shooter_handle),
                victim_position,
                2_000,
                now_ms,
            );
            true
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PedVehicleCollider {
    vehicle_slot: usize,
    matrix: CMatrix,
    min: CVector,
    max: CVector,
}

fn script_ped_vehicle_position_blocked(
    position: CVector,
    colliders: &[PedVehicleCollider],
) -> bool {
    script_ped_vehicle_position_blocked_except(position, colliders, None)
}

fn script_ped_vehicle_position_blocked_except(
    position: CVector,
    colliders: &[PedVehicleCollider],
    ignored_vehicle_slot: Option<usize>,
) -> bool {
    colliders
        .iter()
        .filter(|collider| Some(collider.vehicle_slot) != ignored_vehicle_slot)
        .any(|collider| script_ped_position_blocked_by_vehicle(position, collider))
}

fn script_ped_position_blocked_by_vehicle(
    position: CVector,
    collider: &PedVehicleCollider,
) -> bool {
    let local = collider.matrix.multiply_inverse_point(position);
    let body_min_z = local.z + PED_BODY_MIN_Z;
    let body_max_z = local.z + PED_BODY_MAX_Z;
    let nearest_x = local.x.clamp(collider.min.x, collider.max.x);
    let nearest_y = local.y.clamp(collider.min.y, collider.max.y);
    let dx = local.x - nearest_x;
    let dy = local.y - nearest_y;
    // CPed's 0.35-radius collision sphere may touch a box corner without
    // penetrating it. Expanding both axes into a square falsely blocks the
    // native walk-around waypoint and prevents following the next face.
    body_min_z <= collider.max.z
        && dx * dx + dy * dy < 0.35 * 0.35
        && body_max_z >= collider.min.z
}

fn script_ped_seek_move_state(
    ped: &CPed,
    objective: eObjective,
    seek_distance: f32,
    target_move_state: Option<eMoveState>,
) -> eMoveState {
    if objective == eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT {
        if seek_distance >= 5.0 {
            return eMoveState::PEDMOVE_RUN;
        }
        if ped.leader.is_some() {
            if let Some(target_state) = target_move_state
                && !matches!(
                    target_state,
                    eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL
                )
            {
                return target_state;
            }
        }
        return if seek_distance > 3.0 {
            eMoveState::PEDMOVE_RUN
        } else {
            eMoveState::PEDMOVE_WALK
        };
    }

    if objective == eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION {
        if seek_distance > 3.0 {
            return eMoveState::PEDMOVE_RUN;
        }
        return target_move_state
            .filter(|state| !matches!(state, eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL))
            .unwrap_or(eMoveState::PEDMOVE_WALK);
    }

    if matches!(
        objective,
        eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
    ) {
        if ped.flags.is_running {
            return eMoveState::PEDMOVE_RUN;
        }
        return if seek_distance < 2.0 {
            eMoveState::PEDMOVE_WALK
        } else {
            ped.move_state
        };
    }

    if matches!(
        objective,
        eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_TILL_SAFE
            | eObjective::OBJECTIVE_FLEE_CHAR_ON_FOOT_ALWAYS
    ) {
        return eMoveState::PEDMOVE_RUN;
    }

    let mut next_move =
        if seek_distance < 2.0 || objective == eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT {
            eMoveState::PEDMOVE_WALK
        } else if matches!(
            objective,
            eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
                | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
                | eObjective::OBJECTIVE_RUN_TO_AREA
        ) || ped.flags.is_running
        {
            eMoveState::PEDMOVE_RUN
        } else {
            eMoveState::PEDMOVE_WALK
        };

    if ped.flags.is_running {
        next_move = eMoveState::PEDMOVE_RUN;
    }
    next_move
}

/// Segment/expanded native vehicle COL-box intersection. The closest live
/// obstruction wins; runtime cost is independent of seek distance.
fn script_ped_segment_blocking_vehicle(
    start: CVector,
    end: CVector,
    vehicle_colliders: &[PedVehicleCollider],
) -> Option<usize> {
    script_ped_segment_blocking_vehicle_except(start, end, vehicle_colliders, None)
}

fn script_ped_segment_blocking_vehicle_except(
    start: CVector,
    end: CVector,
    vehicle_colliders: &[PedVehicleCollider],
    ignored_vehicle_slot: Option<usize>,
) -> Option<usize> {
    let mut closest: Option<(f32, usize)> = None;
    for (index, collider) in vehicle_colliders.iter().enumerate() {
        if Some(collider.vehicle_slot) == ignored_vehicle_slot { continue; }
        let a = collider.matrix.multiply_inverse_point(start);
        let b = collider.matrix.multiply_inverse_point(CVector::new(end.x, end.y, start.z));
        let min = [collider.min.x - 0.35, collider.min.y - 0.35,
            collider.min.z - PED_BODY_MAX_Z];
        let max = [collider.max.x + 0.35, collider.max.y + 0.35,
            collider.max.z - PED_BODY_MIN_Z];
        let origin = [a.x, a.y, a.z];
        let delta = [b.x-a.x, b.y-a.y, b.z-a.z];
        let mut enter = 0.0_f32;
        let mut leave = 1.0_f32;
        for axis in 0..3 {
            if delta[axis].abs() <= f32::EPSILON {
                if origin[axis] < min[axis] || origin[axis] > max[axis] { leave = -1.0; break; }
            } else {
                let p = (min[axis]-origin[axis]) / delta[axis];
                let q = (max[axis]-origin[axis]) / delta[axis];
                enter = enter.max(p.min(q));
                leave = leave.min(p.max(q));
            }
        }
        if enter <= leave && closest.is_none_or(|(time, _)| enter < time) { closest = Some((enter, index)); }
    }
    closest.map(|(_, index)| index)

}

fn script_ped_sphere_hits_vehicle_except(
    center: CVector,
    radius: f32,
    vehicle_colliders: &[PedVehicleCollider],
    ignored_vehicle_slot: usize,
) -> bool {
    let radius_sq = radius * radius;
    vehicle_colliders.iter().any(|collider| {
        if collider.vehicle_slot == ignored_vehicle_slot {
            return false;
        }
        let local = collider.matrix.multiply_inverse_point(center);
        let closest = CVector::new(
            local.x.clamp(collider.min.x, collider.max.x),
            local.y.clamp(collider.min.y, collider.max.y),
            local.z.clamp(collider.min.z, collider.max.z),
        );
        let dx = local.x - closest.x;
        let dy = local.y - closest.y;
        let dz = local.z - closest.z;
        dx * dx + dy * dy + dz * dz <= radius_sq
    })
}

/// ECS port of re3 `CPed::PossiblyFindBetterPosToSeekCar` (`Ped.cpp:7838`).
/// The target vehicle is ignored by the midpoint sphere probe exactly like the
/// CWorld call's `ignoreEntity`; only another vehicle between the ped and the
/// authored door can redirect the current SeekCar tick.
fn possibly_find_better_pos_to_seek_car(
    ped_position: CVector,
    destination: CVector,
    door: i32,
    target_vehicle_slot: usize,
    vehicle_colliders: &[PedVehicleCollider],
) -> Option<CVector> {
    let target = vehicle_colliders
        .iter()
        .find(|collider| collider.vehicle_slot == target_vehicle_slot)?;

    let mut helper_pos = ped_position;
    helper_pos.z = destination.z - 0.5;
    let mut found_pos = destination;
    found_pos.z -= 0.5;
    let midpoint = CVector::new(
        (found_pos.x + helper_pos.x) * 0.5,
        (found_pos.y + helper_pos.y) * 0.5,
        (found_pos.z + helper_pos.z) * 0.5,
    );
    if !script_ped_sphere_hits_vehicle_except(
        midpoint,
        0.25,
        vehicle_colliders,
        target_vehicle_slot,
    ) {
        return None;
    }

    let left_rear = target.matrix.transform_point(CVector::new(
        target.min.x - 0.5,
        target.min.y - 0.5,
        0.0,
    ));
    let right_rear = target.matrix.transform_point(CVector::new(
        target.max.x + 0.5,
        target.min.y - 0.5,
        0.0,
    ));
    let left_front = target.matrix.transform_point(CVector::new(
        target.min.x - 0.5,
        target.max.y + 0.5,
        0.0,
    ));
    let right_front = target.matrix.transform_point(CVector::new(
        target.max.x + 0.5,
        target.max.y + 0.5,
        0.0,
    ));

    helper_pos.x -= target.matrix.position.x;
    helper_pos.y -= target.matrix.position.y;
    helper_pos.z -= target.matrix.position.z;
    // This deliberately preserves re3's unusual full matrix multiply after
    // subtracting the vehicle position; it is observable in the corner order.
    helper_pos = target.matrix.transform_point(helper_pos);

    let vehicle_forward_heading = (-target.matrix.forward.x).atan2(target.matrix.forward.y);
    let relative_heading = |corner: CVector| {
        CGeneral::limit_radian_angle(
            (corner.x - helper_pos.x).atan2(corner.y - helper_pos.y)
                - vehicle_forward_heading,
        )
    };
    let lr = relative_heading(left_rear);
    let rr = relative_heading(right_rear);
    let lf = relative_heading(left_front);
    let rf = relative_heading(right_front);
    let can_lr = lr <= -std::f32::consts::PI || lr >= -std::f32::consts::FRAC_PI_2;
    let can_rr = rr <= std::f32::consts::FRAC_PI_2 || rr >= std::f32::consts::PI;
    let can_lf = lf >= 0.0 || lf <= -std::f32::consts::FRAC_PI_2;
    let can_rf = rf <= 0.0 || rf >= std::f32::consts::FRAC_PI_2;

    let ordered = match door {
        5 => [
            (can_rr, right_rear),
            (can_rf, right_front),
            (can_lr, left_rear),
            (can_lf, left_front),
        ],
        3 => [
            (can_rf, right_front),
            (can_rr, right_rear),
            (can_lf, left_front),
            (can_lr, left_rear),
        ],
        2 => [
            (can_lf, left_front),
            (can_lr, left_rear),
            (can_rf, right_front),
            (can_rr, right_rear),
        ],
        4 => [
            (can_lr, left_rear),
            (can_lf, left_front),
            (can_rr, right_rear),
            (can_rf, right_front),
        ],
        _ => return None,
    };
    let corner = ordered
        .into_iter()
        .find_map(|(can_head, corner)| can_head.then_some(corner))?;
    if (ped_position.x - corner.x).powi(2) + (ped_position.y - corner.y).powi(2) <= 0.25 {
        return None;
    }
    Some(CVector::new(corner.x, corner.y, destination.z))
}

fn script_ped_segment_vehicle_blocked(
    start: CVector,
    end: CVector,
    vehicle_colliders: &[PedVehicleCollider],
) -> bool {
    script_ped_segment_blocking_vehicle(start, end, vehicle_colliders).is_some()
}

/// ECS form of re3 `CPed::FindBestCoordsFromNodes` / `SelectClosestNodeForSeek`.
/// Search only a few PATH_PED links from the closest node, as the original
/// helper does, and return the first hop whose branch makes progress toward the
/// authored destination.  Filtering the first hop against live vehicle bounds
/// supplies the collision steering that the original CPed physics path gets
/// automatically but the direct ECS mover otherwise lacks.
fn find_script_ped_seek_node(
    paths: &CPathFind,
    position: CVector,
    destination: CVector,
    vehicle_colliders: &[PedVehicleCollider],
) -> Option<usize> {
    let closest = usize::try_from(paths.find_node_closest_to_coors(
        position, PATH_PED, 999_999.9, false, false,
    )).ok()?;
    let nearest = paths.path_nodes.get(closest)?;
    let distance_squared = |p: CVector| (destination.x-p.x).powi(2)+(destination.y-p.y).powi(2);
    let mut next = None;
    select_closest_node_for_seek(paths, closest, closest, None,
        distance_squared(nearest.pos), distance_squared(position), 3,
        position, destination, vehicle_colliders, &mut next);
    let next = next?;
    let hop = paths.path_nodes.get(next)?;
    ((hop.pos.x-position.x).powi(2)+(hop.pos.y-position.y).powi(2)
        < distance_squared(position)).then_some(next)
}

/// PedAI.cpp SelectClosestNodeForSeek passes close/far distances by value to
/// each recursive branch and decrements the remaining count across siblings.
/// Dynamic contact remains an interface filter on the first live seek segment.
#[allow(clippy::too_many_arguments)]
fn select_closest_node_for_seek(
    paths: &CPathFind, node: usize, closest: usize, first_hop: Option<usize>,
    mut close_distance: f32, mut far_distance: f32, mut remaining: i32,
    position: CVector, destination: CVector, vehicles: &[PedVehicleCollider], next: &mut Option<usize>,
) {
    let Some(node) = paths.path_nodes.get(node) else { return; };
    for offset in 0..node.num_links as usize {
        let Some(candidate) = paths.connected_node(node.first_link as usize+offset) else { continue; };
        if candidate == closest || Some(candidate) == first_hop { continue; }
        let Some(test) = paths.path_nodes.get(candidate) else { continue; };
        let distance = (destination.x-test.pos.x).powi(2)+(destination.y-test.pos.y).powi(2);
        let hop = first_hop.unwrap_or(candidate);
        let reachable = paths.path_nodes.get(hop).is_some_and(|node|
            !script_ped_segment_vehicle_blocked(position,node.pos,vehicles));
        if far_distance > distance && reachable {
            if close_distance <= distance { *next = Some(closest); close_distance = distance; }
            else { *next = Some(hop); far_distance = distance; }
        }
        remaining -= 1;
        if remaining > 0 {
            select_closest_node_for_seek(paths,candidate,closest,Some(hop),close_distance,
                far_distance,remaining,position,destination,vehicles,next);
        }
    }
}

#[cfg(test)]
mod script_ped_path_tests {
    use super::*;
    use crate::{
        control::PathFind::CPathNode,
        peds::{PedPlacement::PED_FEET_OFFSET, PedType::ePedType},
    };
    #[test]
    fn blocked_first_hop_selects_clear_ped_path_branch() {
        let mut paths = CPathFind::init();
        paths.num_car_path_nodes = 0;
        paths.num_ped_path_nodes = 5;
        paths.num_path_nodes = 5;
        paths.connections = vec![1, 2, 4, 3];

        let node = |x: f32, y: f32, first_link: i16, num_links: u8| CPathNode {
            pos: CVector::new(x, y, PED_FEET_OFFSET),
            first_link,
            num_links,
            unk_bits: 1,
            ..CPathNode::default()
        };
        paths.path_nodes[0] = node(0.0, 0.0, 0, 2);
        paths.path_nodes[1] = node(2.0, 0.0, 2, 1);
        paths.path_nodes[2] = node(0.0, 2.0, 3, 1);
        paths.path_nodes[3] = node(5.0, 2.0, 0, 0);
        paths.path_nodes[4] = node(5.0, 0.0, 0, 0);

        let vehicle = PedVehicleCollider {
            vehicle_slot: 1,
            matrix: CMatrix::unity(),
            min: CVector::new(0.4, -0.2, -0.2),
            max: CVector::new(0.8, 0.2, 1.0),
        };
        let selected = find_script_ped_seek_node(
            &paths,
            CVector::new(0.0, 0.0, PED_FEET_OFFSET),
            CVector::new(10.0, 0.0, PED_FEET_OFFSET),
            &[vehicle],
        );

        assert_eq!(selected, Some(2));
    }

    #[test]
    fn vehicle_walk_around_selects_clear_corner() {
        let vehicle = PedVehicleCollider {
            vehicle_slot: 1,
            matrix: CMatrix::unity(),
            min: CVector::new(2.0, -1.0, -0.2),
            max: CVector::new(4.0, 1.0, 1.0),
        };
        let start = CVector::new(0.0, 0.0, PED_FEET_OFFSET);
        let destination = CVector::new(6.0, 0.0, PED_FEET_OFFSET);
        let colliders = [vehicle];

        assert_eq!(
            script_ped_segment_blocking_vehicle(start, destination, &colliders),
            Some(0)
        );
        let mut general = CGeneral::with_seed(1);
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        ped.move_state = eMoveState::PEDMOVE_WALK;
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion {
            rotation_dest: -std::f32::consts::FRAC_PI_2,
            ..Default::default()
        };
        let mut avoidance = crate::peds::PedObstacle::PedObstacleAvoidance::default();
        let corner = walk_around::native_vehicle_corner(&mut ped, &mut locomotion, &mut avoidance, start,
            &vehicle, bevy::prelude::World::new().spawn_empty().id(), None, &colliders, &CFileLoader::default(), 100, &mut general).unwrap();
        assert_eq!(corner.x, 1.65);
        assert_eq!(corner.y, -1.35);
        assert_eq!(ped.action_x, corner.x);
        assert_eq!(ped.action_y, corner.y);
        assert_eq!(locomotion.rotation_dest, (-corner.x).atan2(corner.y));
        assert_eq!(ped.ped_state_timer, 100 + (560.0 * corner.x.hypot(corner.y)) as u32);
        assert!(!script_ped_position_blocked_by_vehicle(corner, &vehicle));
    }

    #[test]
    fn seek_car_detours_only_for_another_vehicle_between_ped_and_target() {
        let mut target_matrix = CMatrix::unity();
        target_matrix.position = CVector::new(6.0, 0.0, 0.0);
        let target = PedVehicleCollider {
            vehicle_slot: 10,
            matrix: target_matrix,
            min: CVector::new(-1.0, -2.0, -0.5),
            max: CVector::new(1.0, 2.0, 1.0),
        };
        let start = CVector::new(0.0, 0.0, PED_FEET_OFFSET);
        let door = CVector::new(4.5, 1.0, PED_FEET_OFFSET);

        // `CWorld::TestSphereAgainstWorld(..., ignore = target)` must not let
        // the target car itself create a bogus detour.
        assert!(
            possibly_find_better_pos_to_seek_car(start, door, 2, 10, &[target]).is_none()
        );

        let mut blocker_matrix = CMatrix::unity();
        blocker_matrix.position = CVector::new(2.25, 0.5, 0.0);
        let blocker = PedVehicleCollider {
            vehicle_slot: 20,
            matrix: blocker_matrix,
            min: CVector::new(-0.8, -0.8, -0.5),
            max: CVector::new(0.8, 0.8, 1.0),
        };
        let detour = possibly_find_better_pos_to_seek_car(
            start,
            door,
            2,
            10,
            &[target, blocker],
        )
        .expect("another car at the midpoint should trigger the re3 corner detour");

        assert!(
            (detour.x - door.x).abs() > 1.0e-6 || (detour.y - door.y).abs() > 1.0e-6
        );
        assert_eq!(detour.z, door.z);
    }

    #[test]
    fn seek_move_state_matches_re3_walk_run_rules() {
        let mut general = CGeneral::new();
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        ped.flags.is_running = false;

        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_FOLLOW_ROUTE, 12.0, None),
            eMoveState::PEDMOVE_WALK
        );
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT, 12.0, None),
            eMoveState::PEDMOVE_WALK
        );
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT, 6.0, None),
            eMoveState::PEDMOVE_RUN
        );
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT, 2.5, None),
            eMoveState::PEDMOVE_WALK
        );
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_RUN_TO_AREA, 1.5, None),
            eMoveState::PEDMOVE_WALK
        );
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_RUN_TO_AREA, 4.0, None),
            eMoveState::PEDMOVE_RUN
        );
        ped.move_state = eMoveState::PEDMOVE_WALK;
        assert_eq!(
            script_ped_seek_move_state(
                &ped,
                eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER,
                8.0,
                None
            ),
            eMoveState::PEDMOVE_WALK
        );
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER, 1.0, None),
            eMoveState::PEDMOVE_WALK
        );

        ped.flags.is_running = true;
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_FOLLOW_ROUTE, 1.5, None),
            eMoveState::PEDMOVE_RUN
        );
        assert_eq!(
            script_ped_seek_move_state(&ped, eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER, 8.0, None),
            eMoveState::PEDMOVE_RUN
        );
    }
}
