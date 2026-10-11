//! CCarCtrl::SlowCarDownForPedsSectorList's pedestrian decisions.
use super::{
    Ped::{CPed, PedState, RANDOM_CHAR, eMoveState},
    PedAnimation::{PedAnimation, PedAnimationSound},
    PedEvasion::{EvasionKind, EvasionReason, begin_evasion},
    PedLocomotion::PedLocomotion,
    PedType::{CPedStatsRuntime, STAT_NO_DIVE},
    PedVitality::PedVitality,
    PlayerPed::PlayerPedState,
    VehicleRelations::InVehicle,
};
use crate::{
    animation::AnimManager::CAnimManager,
    control::{
        CarRails::GAME_SPEED_TO_METERS_PER_SECOND,
        ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity, ScriptVehicleIdentity},
        SirenTraffic::AutomobileSpatialIndex,
    },
    core::{FileLoader::CFileLoader, Pools::CPoolHandle},
    entities::Entity::eEntityStatus,
    modelinfo::ModelIndices::MI_RCBANDIT,
    vehicles::{
        Automobile::AutomobileState,
        Vehicle::{VehicleAudioState, VehicleCore},
    },
};
use bevy::prelude::*;
use re3bevy_core::core::{General::CGeneral, Timer::CTimer};

#[derive(Default, Resource)]
pub struct PedEvasionDiagnostics {
    pub scanned_peds: u64,
    pub nearby_vehicles: u64,
    pub projected_threats: u64,
    pub close_steps: u64,
    pub close_dives: u64,
    pub animations_started: u64,
}

fn close_evasion(
    model: usize,
    velocity: f32,
    distance: f32,
    side: f32,
    lateral: f32,
) -> Option<EvasionKind> {
    if model == MI_RCBANDIT {
        return (velocity * GAME_SPEED_TO_METERS_PER_SECOND / 2.0 > distance)
            .then_some(EvasionKind::Step);
    }
    let step = if velocity > 0.3 {
        side + 0.1 < lateral
    } else if velocity > 0.1 {
        side - 0.5 < lateral
    } else {
        return None;
    };
    Some(if step {
        EvasionKind::Step
    } else {
        EvasionKind::Dive
    })
}

/// Read sector membership and live readonly car owners. Only the ped's
/// behavior/animation owners are mutable; no copied world poses are retained.
pub(crate) fn process_ambient_ped_evasion(
    index: Res<ScriptEntityIndex>,
    spatial: Res<AutomobileSpatialIndex>,
    vehicles: Query<
        (&ScriptVehicleIdentity, &VehicleCore, &VehicleAudioState),
        With<AutomobileState>,
    >,
    mut peds: Query<
        (
            Entity,
            &ScriptPedIdentity,
            &mut CPed,
            &PedVitality,
            &mut PedLocomotion,
            &mut PedAnimation,
            Has<InVehicle>,
        ),
        Without<PlayerPedState>,
    >,
    loader: Res<CFileLoader>,
    manager: Res<CAnimManager>,
    stats: Res<CPedStatsRuntime>,
    mut general: ResMut<CGeneral>,
    timer: Res<CTimer>,
    mut commands: Commands,
    mut candidates: Local<Vec<Entity>>,
    mut diagnostics: ResMut<PedEvasionDiagnostics>,
) {
    crate::profile_scope!("process_ambient_ped_evasion");
    let now = timer.time_in_milliseconds();
    for (entity, identity, mut ped, vitality, mut locomotion, mut animation, seated) in &mut peds {
        if index.ped(identity.handle) != Some(entity)
            || seated
            || vitality.health <= 0.0
            || !ped.physical.entity.flags.uses_collision
        {
            continue;
        }
        let position = ped.physical.entity.get_position();
        diagnostics.scanned_peds += 1;
        let scan = crate::control::CarRails::DISTANCE_TO_SCAN_FOR_DANGER;
        spatial.candidates(position, scan, &mut candidates);
        // TODO(parity): CWorld sector/source list ordering. Entity uniqueness
        // preserves scan-code membership, but is not native moving-list order.
        for &vehicle_entity in candidates.iter() {
            if matches!(
                ped.ped_state,
                PedState::PED_STEP_AWAY | PedState::PED_DIVE_AWAY
            ) {
                break;
            }
            let Ok((identity, core, audio)) = vehicles.get(vehicle_entity) else {
                continue;
            };
            if index.vehicle(identity.handle) != Some(vehicle_entity)
                || !core.physical.entity.flags.uses_collision
                || core.physical.entity.status == eEntityStatus::STATUS_WRECKED
            {
                continue;
            }
            let car_position = core.physical.entity.get_position();
            diagnostics.nearby_vehicles += 1;
            if (car_position.x - position.x).abs() > scan
                || (car_position.y - position.y).abs() > scan
                || (car_position.z - position.z).abs() >= 4.0
            {
                continue;
            }
            let model = core.physical.entity.model_index.max(0) as usize;
            let Some(col) = loader.model_info.col_model(model) else {
                warn_once!("ped_evasion_waiting_original_vehicle_col");
                continue;
            };
            let to_ped = position - car_position;
            let forward = core.physical.entity.get_forward();
            let right = core.physical.entity.get_right();
            let direction = forward.x * to_ped.x + forward.y * to_ped.y;
            let front = col.bounding_box.max.y;
            if direction <= front {
                continue;
            }
            let distance = direction - front;
            let velocity =
                forward.x * core.physical.move_speed.x + forward.y * core.physical.move_speed.y;
            let towards = GAME_SPEED_TO_METERS_PER_SECOND * velocity;
            if 4.0 * towards <= distance {
                continue;
            }
            let lateral = (right.x * to_ped.x + right.y * to_ped.y).abs();
            let side = col.bounding_box.max.x;
            if side + 0.5 < lateral {
                continue;
            }
            diagnostics.projected_threats += 1;
            if distance < towards {
                let Some(kind) = close_evasion(model, velocity, distance, side, lateral) else {
                    continue;
                };
                match kind {
                    EvasionKind::Step => diagnostics.close_steps += 1,
                    EvasionKind::Dive => diagnostics.close_dives += 1,
                }
                let previous_state = ped.ped_state;
                let reason = EvasionReason {
                    slot: CPoolHandle(identity.handle).index(),
                    position: car_position,
                    move_speed: core.physical.move_speed,
                    model_id: model,
                    horn: audio.car_horn_timer != 0,
                };
                let no_dive = stats.GetPedStats(ped.ped_stats).m_flags & STAT_NO_DIVE != 0;
                if let Some(sound) = begin_evasion(
                    &mut ped,
                    vitality,
                    &mut locomotion,
                    &mut animation,
                    &manager,
                    reason,
                    kind,
                    no_dive,
                    &mut general,
                ) {
                    commands.entity(entity).insert(PedAnimationSound(sound));
                }
                if ped.ped_state != previous_state { diagnostics.animations_started += 1; }
            } else if core.physical.entity.status == eEntityStatus::STATUS_PLAYER
                && ped.ped_state != PedState::PED_FLEE_ENTITY
                && ped.char_created_by == RANDOM_CHAR
            {
                let angle =
                    CGeneral::limit_radian_angle(CGeneral::get_radian_angle_between_points(
                        car_position.x,
                        car_position.y,
                        position.x,
                        position.y,
                    ));
                // Preserve the native heading-rate quirk, including its write.
                locomotion.heading_rate = CGeneral::limit_radian_angle(locomotion.heading_rate);
                let mut visibility = (angle - locomotion.heading_rate).abs();
                if visibility > std::f32::consts::PI {
                    visibility = std::f32::consts::TAU - visibility;
                }
                if visibility >= std::f32::consts::FRAC_PI_2 && audio.car_horn_timer == 0 {
                    continue;
                }
                if !ped.is_ped_in_control(vitality) || ped.flags.kinda_stay_in_same_place {
                    continue;
                }
                ped.set_stored_state();
                ped.ped_state = PedState::PED_FLEE_ENTITY;
                ped.move_state = eMoveState::PEDMOVE_RUN;
                ped.flee_from = Some(CPoolHandle(identity.handle).index());
                ped.flee_from_pos_x = car_position.x;
                ped.flee_from_pos_y = car_position.y;
                ped.flee_timer = now.wrapping_add(2000);
                ped.flags.use_ped_node_seek = false;
                ped.next_path_node = None;
                locomotion.rotation_dest =
                    CGeneral::limit_radian_angle(CGeneral::get_radian_angle_between_points(
                        position.x,
                        position.y,
                        car_position.x,
                        car_position.y,
                    ));
                if locomotion.rotation_cur - std::f32::consts::PI > locomotion.rotation_dest {
                    locomotion.rotation_dest += std::f32::consts::TAU;
                } else if locomotion.rotation_cur + std::f32::consts::PI < locomotion.rotation_dest
                {
                    locomotion.rotation_dest -= std::f32::consts::TAU;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_slow_vehicle_and_rcbandit_branches_do_not_invent_dives() {
        assert!(close_evasion(90, 0.1, 0.5, 1.0, 0.0).is_none());
        assert!(close_evasion(MI_RCBANDIT, 0.1, 3.0, 1.0, 0.0).is_none());
        assert!(matches!(
            close_evasion(MI_RCBANDIT, 0.2, 2.0, 1.0, 0.0),
            Some(EvasionKind::Step)
        ));
        assert!(matches!(
            close_evasion(90, 0.2, 2.0, 1.0, 0.6),
            Some(EvasionKind::Step)
        ));
        assert!(matches!(
            close_evasion(90, 0.31, 2.0, 1.0, 0.6),
            Some(EvasionKind::Dive)
        ));
    }
}
