//! Shared CPed::Fall, SetGetUp and PedGetupCB for canonical ped/player actors.
use super::{
    Ped::{CPed, PedState, blend_ped_movement_animation, update_non_player_movement_animation},
    PedAnimation::PedAnimation,
    PedLocomotion::PedLocomotion,
    PedPlacement::PED_CAR_CLEAR_RADIUS,
    PedVitality::PedVitality,
    PlayerPed::{PlayerPedState, sphere_hits_col_model},
    VehicleRelations::InVehicle,
};
use crate::{
    animation::{
        AnimAssocDefinitions::ASSOCGRP_STD,
        AnimBlendAssociation::{ASSOC_DELETEFADEDOUT, ASSOC_FRONTAL},
        AnimManager::CAnimManager,
        AnimationId::*,
    },
    collision::{Collision::CCollision, VehicleSpatial::VehicleCollisionSpatial},
    control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity, ScriptVehicleIdentity},
    core::{FileLoader::CFileLoader, Pad::CPads},
    renderer::BevyRender::ped_frame_world_point_from_clump,
    vehicles::Vehicle::VehicleCore,
};
use bevy::prelude::*;
use re3bevy_core::core::{General::CGeneral, Timer::CTimer};

fn critical_health_requires_death(health: f32, head_height_from_position: f32) -> bool {
    health < 1.0 && head_height_from_position <= -0.3
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GetUpSpace {
    Clear,
    Blocked,
    WaitingForCol,
}

fn previous_car_requires_test(frame: u32, seed: u16) -> bool {
    frame.wrapping_add(u32::from(seed)).wrapping_add(5) as u8 % 8 == 0
}

/// Placement queries borrow current poses from the shared bounds index. No
/// actor matrices or vehicle arrays are copied into a recovery world snapshot.
pub(crate) fn getup_space<'a>(
    ped: &CPed,
    timer: &CTimer,
    index: &ScriptEntityIndex,
    spatial: &VehicleCollisionSpatial,
    loader: &CFileLoader,
    mut vehicle_core: impl FnMut(Entity) -> Option<&'a VehicleCore>,
    candidates: &mut Vec<Entity>,
) -> GetUpSpace {
    let position = ped.physical.entity.get_position();
    if loader
        .world
        .sphere_contact_buildings_only(
            [position.x, position.y, position.z],
            PED_CAR_CLEAR_RADIUS,
            &loader.model_info,
        )
        .is_some()
    {
        return GetUpSpace::Blocked;
    }
    if !spatial
        .grid
        .candidates(position, PED_CAR_CLEAR_RADIUS, candidates)
    {
        warn_once!("npc_getup_invalid_spatial_position");
        return GetUpSpace::WaitingForCol;
    }
    for &entity in candidates.iter() {
        let Some(core) = vehicle_core(entity) else {
            continue;
        };
        if !core.physical.entity.flags.uses_collision {
            continue;
        }
        let Some(col) = loader
            .model_info
            .col_model(core.physical.entity.model_index.max(0) as usize)
        else {
            warn_once!("npc_getup_waiting_original_vehicle_col");
            return GetUpSpace::WaitingForCol;
        };
        if sphere_hits_col_model(
            position,
            PED_CAR_CLEAR_RADIUS,
            &core.physical.entity.matrix,
            col,
        ) {
            return GetUpSpace::Blocked;
        }
    }
    if let Some(entity) = ped
        .colliding_entity
        .and_then(|slot| index.vehicle_handle_for_slot(slot))
        .and_then(|handle| index.vehicle(handle))
        && let Some(core) = vehicle_core(entity)
    {
        // Native m_pCollidingEntity is distinct from the flee/walk-around
        // relationship. Its retry remains blocked on seven of every eight frames.
        if !previous_car_requires_test(timer.frame_counter(), ped.physical.entity.random_seed) {
            return GetUpSpace::Blocked;
        }
        let Some(col) = loader
            .model_info
            .col_model(core.physical.entity.model_index.max(0) as usize)
        else {
            warn_once!("npc_getup_waiting_previous_vehicle_col");
            return GetUpSpace::WaitingForCol;
        };
        let Ok(model) = usize::try_from(ped.physical.entity.model_index) else {
            warn_once!("ped_getup_invalid_original_ped_col");
            return GetUpSpace::WaitingForCol;
        };
        let Some(ped_col) = loader.model_info.col_model(model) else {
            warn_once!(model, "ped_getup_waiting_original_ped_col");
            return GetUpSpace::WaitingForCol;
        };
        if CCollision::test_col_models_spheres(
            &ped.physical.entity.matrix,
            ped_col,
            &core.physical.entity.matrix,
            col,
        ) {
            return GetUpSpace::Blocked;
        }
    }
    GetUpSpace::Clear
}

/// Borrowed inputs for the native recovery state handler. No second clock,
/// clump, car poses or player damage state is retained by this domain.
pub(crate) struct PedRecoveryInput<'a> {
    pub(crate) anim_manager: &'a CAnimManager,
    pub(crate) file_loader: &'a CFileLoader,
    pub(crate) timer: &'a CTimer,
    pub(crate) player_can_be_damaged: bool,
    pub(crate) player_controls_disabled: bool,
}

/// Apply the accepted command to the same clump gameplay/rendering borrow.
/// Native World advances clumps before control; damage after that boundary
/// installs its association before presentation and advances next frame.
pub fn apply_pending_fall_animation(
    ped: &mut CPed,
    animation: &mut PedAnimation,
    manager: &CAnimManager,
    loader: &CFileLoader,
) -> bool {
    let Some(request) = ped.fall_animation_request else {
        return true;
    };
    if ped.ped_state != PedState::PED_FALL {
        ped.fall_animation_request = None;
        return true;
    }
    let Ok(model) = usize::try_from(ped.physical.entity.model_index) else {
        warn_once!("ped_fall_invalid_original_model");
        return false;
    };
    if !animation.ensure_original_frames(model, loader) {
        return false;
    }
    let clump = &mut animation.movement_anim_clump;
    let index = if let Some(index) = clump
        .associations
        .iter()
        .position(|association| association.anim_id == request.animation_id)
    {
        let association = &mut clump.associations[index];
        association.set_current_time(0.0);
        association.blend_amount = 0.0;
        association.blend_delta = 8.0;
        association.set_run();
        association.callback_events.clear();
        index
    } else {
        let Some(index) =
            blend_ped_movement_animation(clump, manager, ASSOCGRP_STD, request.animation_id, 8.0)
        else {
            warn_once!(
                "ped_fall_waiting_original_association: {}",
                request.animation_id
            );
            return false;
        };
        index
    };
    let animation_ms = (1000.0 * clump.associations[index].hierarchy.total_length) as u32;
    ped.get_up_timer = if request.extra_time_ms == -1 {
        u32::MAX
    } else if ped.is_player() {
        request.now_ms.wrapping_add(animation_ms).wrapping_add(500)
    } else {
        request
            .now_ms
            .wrapping_add(animation_ms)
            .wrapping_add(request.extra_time_ms as u32)
            .wrapping_add(
                (u32::from(ped.physical.entity.random_seed).wrapping_add(request.frame_counter))
                    % 1000,
            )
    };
    ped.fall_animation_request = None;
    true
}

pub(crate) fn process_pending_fall_animations(
    mut actors: Query<(&mut CPed, &mut PedAnimation)>,
    manager: Res<CAnimManager>,
    loader: Res<CFileLoader>,
) {
    for (mut ped, mut animation) in &mut actors {
        if ped.fall_animation_request.is_some() {
            apply_pending_fall_animation(&mut ped, &mut animation, &manager, &loader);
        }
    }
}

pub(crate) fn service_fall_getup(
    ped: &mut CPed,
    vitality: &mut PedVitality,
    locomotion: &mut PedLocomotion,
    animation: &mut PedAnimation,
    inputs: &PedRecoveryInput<'_>,
    mut position_clear: impl FnMut(&CPed) -> GetUpSpace,
) -> bool {
    if !matches!(ped.ped_state, PedState::PED_FALL | PedState::PED_GETUP) {
        return false;
    }
    let Ok(model) = usize::try_from(ped.physical.entity.model_index) else {
        warn_once!("ped_recovery_invalid_original_model");
        return false;
    };
    if !animation.ensure_original_frames(model, inputs.file_loader) {
        return false;
    }
    let now_ms = inputs.timer.time_in_milliseconds();
    if !apply_pending_fall_animation(ped, animation, inputs.anim_manager, inputs.file_loader) {
        return false;
    }
    let falling_getup_due =
        ped.ped_state == PedState::PED_FALL && now_ms > ped.get_up_timer && ped.can_set_ped_state();
    if falling_getup_due || (ped.ped_state == PedState::PED_GETUP && !ped.flags.get_up_anim_started)
    {
        // SetGetUp checks the real PED_HEAD before changing heading/state.
        // Sample the same clump used by gameplay damage and presentation.
        if vitality.health < 1.0 {
            let Ok(model_id) = usize::try_from(ped.physical.entity.model_index) else {
                warn_once!("npc_getup_invalid_original_model");
                return false;
            };
            let Some(head) = ped_frame_world_point_from_clump(
                model_id,
                inputs.file_loader,
                &animation.movement_anim_clump,
                &ped.physical.entity.matrix,
                "Shead",
                Default::default(),
            ) else {
                warn_once!("npc_getup_waiting_original_head_frame");
                return false;
            };
            if critical_health_requires_death(
                vitality.health,
                head.z - ped.physical.entity.get_position().z,
            ) {
                ped.set_die(vitality, None, 4.0, 0.0, now_ms);
                return false;
            }
        }
        if falling_getup_due {
            ped.set_stored_state();
            ped.ped_state = PedState::PED_GETUP;
            ped.flags.get_up_anim_started = false;
        }
    }
    if ped.ped_state == PedState::PED_GETUP && !ped.flags.get_up_anim_started {
        if ped.flags.update_anim_heading {
            locomotion.rotation_cur =
                CGeneral::limit_radian_angle(locomotion.rotation_cur) - std::f32::consts::FRAC_PI_2;
            ped.physical.entity.set_heading(locomotion.rotation_cur);
            ped.flags.update_anim_heading = false;
        }
        match position_clear(ped) {
            GetUpSpace::WaitingForCol => return false,
            GetUpSpace::Blocked => {
                if ped.is_player() {
                    if inputs.player_can_be_damaged {
                        ped.apply_getup_runover_damage(vitality, inputs.timer.time_step(), now_ms);
                    }
                } else if inputs.player_controls_disabled {
                    ped.apply_getup_runover_damage(vitality, 1000.0, now_ms);
                }
            }
            GetUpSpace::Clear => {
                let fast = animation
                    .movement_anim_clump
                    .associations
                    .iter()
                    .any(|association| association.anim_id == ANIM_STD_RUNFAST);
                if fast {
                    let id = if animation
                        .movement_anim_clump
                        .associations
                        .iter()
                        .any(|association| association.anim_id == ANIM_STD_RUN)
                    {
                        ANIM_STD_RUN
                    } else {
                        ANIM_STD_IDLE
                    };
                    let Ok(group) = usize::try_from(ped.anim_group) else {
                        warn_once!("npc_getup_invalid_original_animation_group");
                        return false;
                    };
                    if blend_ped_movement_animation(
                        &mut animation.movement_anim_clump,
                        inputs.anim_manager,
                        group,
                        id,
                        8.0,
                    )
                    .is_none()
                    {
                        warn_once!("npc_getup_waiting_original_runfast_replacement");
                        return false;
                    }
                    for association in &mut animation.movement_anim_clump.associations {
                        if association.anim_id == ANIM_STD_RUNFAST {
                            association.flags |= ASSOC_DELETEFADEDOUT;
                        }
                    }
                }
                let frontal = animation
                    .movement_anim_clump
                    .associations
                    .iter()
                    .any(|association| association.flags & ASSOC_FRONTAL != 0);
                let id = if frontal {
                    ANIM_STD_GET_UP_FRONT
                } else {
                    ANIM_STD_GET_UP
                };
                let Some(index) = blend_ped_movement_animation(
                    &mut animation.movement_anim_clump,
                    inputs.anim_manager,
                    ASSOCGRP_STD,
                    id,
                    1000.0,
                ) else {
                    warn_once!("npc_getup_waiting_original_association: {id}");
                    return false;
                };
                animation.movement_anim_clump.associations[index].set_finish_callback();
                ped.flags.get_up_anim_started = true;
                ped.colliding_entity = None;
                ped.flags.knocked_up_into_air = false;
                ped.impact_animation_id = None;
            }
        }
    }

    true
}

pub(crate) fn process_script_ped_fall_getup(
    mut peds: Query<
        (
            Entity,
            &ScriptPedIdentity,
            &mut CPed,
            &mut PedVitality,
            &mut PedLocomotion,
            &mut PedAnimation,
            Has<InVehicle>,
        ),
        Without<PlayerPedState>,
    >,
    vehicles: Query<(&ScriptVehicleIdentity, &VehicleCore)>,
    index: Res<ScriptEntityIndex>,
    spatial: Res<VehicleCollisionSpatial>,
    collision: Res<CCollision>,
    file_loader: Res<CFileLoader>,
    anim_manager: Res<CAnimManager>,
    timer: Res<CTimer>,
    pads: Res<CPads>,
    mut general: ResMut<CGeneral>,
    mut candidates: Local<Vec<Entity>>,
) {
    let now_ms = timer.time_in_milliseconds();
    for (entity, identity, mut ped, mut vitality, mut locomotion, mut animation, seated) in
        &mut peds
    {
        if index.ped(identity.handle) != Some(entity)
            || seated
            || !matches!(ped.ped_state, PedState::PED_FALL | PedState::PED_GETUP)
        {
            continue;
        }
        let zone = ped.physical.zone_level;
        if zone > crate::collision::ColModel::LEVEL_GENERIC as i8
            && zone != collision.collision_in_memory as i8
        {
            warn_once!("npc_recovery_waiting_original_collision_level");
            continue;
        }
        let inputs = PedRecoveryInput {
            anim_manager: &anim_manager,
            file_loader: &file_loader,
            timer: &timer,
            player_can_be_damaged: true,
            player_controls_disabled: pads.get_pad(0).disable_player_controls,
        };
        if !service_fall_getup(
            &mut ped,
            &mut vitality,
            &mut locomotion,
            &mut animation,
            &inputs,
            |ped| {
                getup_space(
                    ped,
                    &timer,
                    &index,
                    &spatial,
                    &file_loader,
                    |entity| {
                        let (identity, core) = vehicles.get(entity).ok()?;
                        (index.vehicle(identity.handle) == Some(entity)).then_some(core)
                    },
                    &mut candidates,
                )
            },
        ) {
            continue;
        }
        update_non_player_movement_animation(
            &mut ped,
            &mut locomotion,
            &vitality,
            &mut animation,
            &anim_manager,
            &file_loader,
            timer.time_step_in_seconds(),
            false,
            None,
        );
        if super::PedEvasion::service_getup_finish(
            &mut ped,
            &vitality,
            &mut animation,
            &file_loader,
            &mut general,
            now_ms,
        ) {
            super::Ped::set_ped_move_animation(
                &mut ped,
                &vitality,
                &mut animation,
                &anim_manager,
                &file_loader,
                false,
                None,
            );
        }
    }
}

/// P1/P2 call this at their existing control boundary. The shared native
/// handler reads the same indexed live cars as NPC recovery, without changing
/// the PedTalk -> Remote -> player-control schedule.
pub(crate) fn service_player_fall_getup<'a>(
    player: &mut super::PlayerPed::PlayerPedMut<'_>,
    inputs: &PedRecoveryInput<'_>,
    index: &ScriptEntityIndex,
    spatial: &VehicleCollisionSpatial,
    mut vehicle_core: impl FnMut(Entity) -> Option<&'a VehicleCore>,
    general: &mut CGeneral,
) {
    if !matches!(
        player.ped.ped_state,
        PedState::PED_FALL | PedState::PED_GETUP
    ) {
        return;
    }
    let mut candidates = Vec::new();
    if !service_fall_getup(
        player.ped,
        player.vitality,
        player.locomotion,
        player.animation,
        inputs,
        |ped| {
            getup_space(
                ped,
                inputs.timer,
                index,
                spatial,
                inputs.file_loader,
                &mut vehicle_core,
                &mut candidates,
            )
        },
    ) {
        return;
    }
    update_non_player_movement_animation(
        player.ped,
        player.locomotion,
        player.vitality,
        player.animation,
        inputs.anim_manager,
        inputs.file_loader,
        inputs.timer.time_step_in_seconds(),
        false,
        None,
    );
    if super::PedEvasion::service_getup_finish(
        player.ped,
        player.vitality,
        player.animation,
        inputs.file_loader,
        general,
        inputs.timer.time_in_milliseconds(),
    ) {
        super::Ped::set_ped_move_animation(
            player.ped,
            player.vitality,
            player.animation,
            inputs.anim_manager,
            inputs.file_loader,
            false,
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fall_fixture(player: bool) -> (CPed, PedAnimation, CGeneral) {
        let mut general = CGeneral::with_seed(41);
        let mut ped = CPed::new(
            if player {
                super::super::PedType::ePedType::PEDTYPE_PLAYER1
            } else {
                super::super::PedType::ePedType::PEDTYPE_CIVMALE
            },
            &mut general,
        );
        ped.physical.entity.model_index = 0;
        ped.ped_state = PedState::PED_IDLE;
        let mut animation = PedAnimation::default();
        animation.movement_anim_clump.set_number_of_frames(1);
        animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        (ped, animation, general)
    }

    #[test]
    fn set_fall_reuses_the_live_clip_and_keeps_the_original_saved_state() {
        use crate::animation::{
            AnimBlendAssociation::{AnimBlendCallbackEvent, CAnimBlendAssociation},
            AnimBlendHierarchy::CAnimBlendHierarchy,
        };
        let (mut ped, mut animation, _) = fall_fixture(false);
        ped.last_ped_state = PedState::PED_WANDER_PATH;
        ped.stored_move_state = super::super::Ped::eMoveState::PEDMOVE_WALK;
        let mut association = CAnimBlendAssociation::new();
        association.init_with_hierarchy(
            CAnimBlendHierarchy {
                total_length: 0.75,
                ..Default::default()
            },
            0,
        );
        association.anim_id = ANIM_STD_HIGHIMPACT_BACK;
        association.speed = 0.6;
        association.set_current_time(0.5);
        association.blend_amount = 1.0;
        association
            .callback_events
            .push(AnimBlendCallbackEvent::Finish);
        animation.movement_anim_clump.associations.push(association);
        assert!(ped.set_fall(
            &PedVitality::default(),
            2000,
            ANIM_STD_HIGHIMPACT_BACK,
            false,
            1000,
            17
        ));
        assert!(apply_pending_fall_animation(
            &mut ped,
            &mut animation,
            &CAnimManager::initialise(),
            &CFileLoader::default()
        ));
        let association = &animation.movement_anim_clump.associations[0];
        assert_eq!(association.current_time, 0.0);
        assert_eq!(association.blend_amount, 0.0);
        assert_eq!(association.blend_delta, 8.0);
        assert_eq!(association.speed, 0.6, "reuse preserves native speed");
        assert!(association.is_running() && association.callback_events.is_empty());
        assert_eq!(ped.last_ped_state, PedState::PED_WANDER_PATH);
        assert_eq!(
            ped.stored_move_state,
            super::super::Ped::eMoveState::PEDMOVE_WALK
        );
        assert_eq!(
            ped.get_up_timer,
            3750 + (u32::from(ped.physical.entity.random_seed) + 17) % 1000
        );
    }

    #[test]
    fn set_fall_uses_authored_duration_source_frame_and_only_minus_one_sentinel() {
        use crate::animation::{
            AnimBlendAssocGroup::CAnimBlendAssocGroup, AnimBlendAssociation::CAnimBlendAssociation,
            AnimBlendHierarchy::CAnimBlendHierarchy,
        };
        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups = vec![CAnimBlendAssocGroup::new(); ASSOCGRP_STD + 1];
        let mut template = CAnimBlendAssociation::new();
        template.init_with_hierarchy(
            CAnimBlendHierarchy {
                total_length: 1.2,
                ..Default::default()
            },
            0,
        );
        template.anim_id = ANIM_STD_HIGHIMPACT_FRONT;
        manager.anim_assoc_groups[ASSOCGRP_STD]
            .assoc_list
            .push(template);
        for player in [false, true] {
            for extra in [-2, -1, 2000] {
                let (mut ped, mut animation, _) = fall_fixture(player);
                assert!(ped.set_fall(
                    &PedVitality::default(),
                    extra,
                    ANIM_STD_HIGHIMPACT_FRONT,
                    false,
                    9000,
                    321
                ));
                assert!(apply_pending_fall_animation(
                    &mut ped,
                    &mut animation,
                    &manager,
                    &CFileLoader::default()
                ));
                let expected = if extra == -1 {
                    u32::MAX
                } else if player {
                    10700
                } else {
                    10200_u32
                        .wrapping_add(extra as u32)
                        .wrapping_add((u32::from(ped.physical.entity.random_seed) + 321) % 1000)
                };
                assert_eq!(ped.get_up_timer, expected);
                assert_eq!(animation.movement_anim_clump.associations.len(), 1);
                assert!(ped.fall_animation_request.is_none());
            }
        }
    }

    #[test]
    fn missing_fall_clip_waits_without_a_guessed_timer_and_death_cancels_request() {
        let (mut ped, mut animation, _) = fall_fixture(false);
        assert!(ped.set_fall(
            &PedVitality::default(),
            1500,
            ANIM_STD_HIGHIMPACT_FRONT,
            false,
            500,
            23
        ));
        assert!(!apply_pending_fall_animation(
            &mut ped,
            &mut animation,
            &CAnimManager::initialise(),
            &CFileLoader::default()
        ));
        assert_eq!(ped.get_up_timer, u32::MAX);
        assert!(ped.fall_animation_request.is_some());
        assert!(animation.movement_anim_clump.associations.is_empty());
        ped.ped_state = PedState::PED_DIE;
        assert!(apply_pending_fall_animation(
            &mut ped,
            &mut animation,
            &CAnimManager::initialise(),
            &CFileLoader::default()
        ));
        assert!(ped.fall_animation_request.is_none());
    }
    #[test]
    fn previous_vehicle_retries_keep_native_eight_frame_phase_and_wrapping() {
        for seed in [0, 1, 255, u16::MAX] {
            assert_eq!(
                (0..8)
                    .filter(|&frame| previous_car_requires_test(frame, seed))
                    .count(),
                1
            );
        }
        assert!(previous_car_requires_test(u32::MAX, 4));
    }

    #[test]
    fn critical_health_and_head_height_preserve_native_strict_boundary() {
        assert!(!critical_health_requires_death(1.0, -0.4));
        assert!(!critical_health_requires_death(0.5, -0.299));
        assert!(critical_health_requires_death(0.5, -0.3));
        assert!(critical_health_requires_death(0.0, -0.4));
    }

    #[test]
    fn player_recovery_waits_for_the_live_callback_and_keeps_protected_blocked_state() {
        use crate::animation::{
            AnimBlendAssocGroup::CAnimBlendAssocGroup,
            AnimBlendAssociation::{ASSOC_PARTIAL, CAnimBlendAssociation},
            AnimBlendHierarchy::CAnimBlendHierarchy,
        };
        let loader = CFileLoader::default();
        let mut general = CGeneral::with_seed(19);
        let mut ped = CPed::new(
            super::super::PedType::ePedType::PEDTYPE_PLAYER1,
            &mut general,
        );
        ped.physical.entity.model_index = 0;
        ped.ped_state = PedState::PED_FALL;
        ped.last_ped_state = PedState::PED_IDLE;
        ped.flags.fallen_down = true;
        ped.flags.update_anim_heading = true;
        ped.get_up_timer = 100;
        let mut vitality = PedVitality::default();
        let mut locomotion = PedLocomotion::default();
        let mut animation = PedAnimation::default();
        // Explicit unit fixture clump; production only binds installed DFF.
        animation.movement_anim_clump.set_number_of_frames(1);
        animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups = vec![CAnimBlendAssocGroup::new(); ASSOCGRP_STD + 1];
        let mut getup = CAnimBlendAssociation::new();
        getup.init_with_hierarchy(
            CAnimBlendHierarchy {
                total_length: 0.5,
                ..Default::default()
            },
            0,
        );
        getup.anim_id = ANIM_STD_GET_UP;
        manager.anim_assoc_groups[ASSOCGRP_STD]
            .assoc_list
            .push(getup);
        let mut timer = CTimer::initialise();
        timer.set_time_in_milliseconds(101);
        timer.set_time_step(1.0);
        let inputs = PedRecoveryInput {
            anim_manager: &manager,
            file_loader: &loader,
            timer: &timer,
            player_can_be_damaged: false,
            player_controls_disabled: false,
        };
        assert!(service_fall_getup(
            &mut ped,
            &mut vitality,
            &mut locomotion,
            &mut animation,
            &inputs,
            |_| GetUpSpace::Blocked
        ));
        assert_eq!(ped.ped_state, PedState::PED_GETUP);
        assert!(!ped.flags.get_up_anim_started);
        assert_eq!(vitality.health, 100.0);
        assert!((locomotion.rotation_cur + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!(service_fall_getup(
            &mut ped,
            &mut vitality,
            &mut locomotion,
            &mut animation,
            &inputs,
            |_| GetUpSpace::Clear
        ));
        assert!(ped.flags.get_up_anim_started);
        let association = &mut animation.movement_anim_clump.associations[0];
        association.speed = 0.5;
        association.update_time(0.5, 1.0);
        assert!(!super::super::PedEvasion::service_getup_finish(
            &mut ped,
            &vitality,
            &mut animation,
            &loader,
            &mut general,
            10_000
        ));
        assert_eq!(
            ped.ped_state,
            PedState::PED_GETUP,
            "wall time must not fabricate a finish"
        );
        animation.movement_anim_clump.associations[0].update_time(0.51, 1.0);
        let mut partial = CAnimBlendAssociation::new();
        partial.flags |= ASSOC_PARTIAL;
        animation.movement_anim_clump.associations.push(partial);
        assert!(super::super::PedEvasion::service_getup_finish(
            &mut ped,
            &vitality,
            &mut animation,
            &loader,
            &mut general,
            10_001
        ));
        assert_eq!(ped.ped_state, PedState::PED_IDLE);
        assert!(!ped.flags.fallen_down && !ped.flags.get_up_anim_started);
        assert_eq!(
            animation.movement_anim_clump.associations[1].blend_delta,
            -1000.0
        );
    }

    #[test]
    fn missing_getup_template_preserves_the_player_wait() {
        let mut general = CGeneral::with_seed(5);
        let mut ped = CPed::new(
            super::super::PedType::ePedType::PEDTYPE_PLAYER1,
            &mut general,
        );
        ped.physical.entity.model_index = 0;
        ped.ped_state = PedState::PED_GETUP;
        ped.flags.fallen_down = true;
        let mut animation = PedAnimation::default();
        animation.movement_anim_clump.set_number_of_frames(1);
        animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        let manager = CAnimManager::initialise();
        let loader = CFileLoader::default();
        let timer = CTimer::initialise();
        assert!(!service_fall_getup(
            &mut ped,
            &mut PedVitality::default(),
            &mut PedLocomotion::default(),
            &mut animation,
            &PedRecoveryInput {
                anim_manager: &manager,
                file_loader: &loader,
                timer: &timer,
                player_can_be_damaged: true,
                player_controls_disabled: false
            },
            |_| GetUpSpace::Clear
        ));
        assert!(ped.flags.fallen_down);
        assert!(!ped.flags.get_up_anim_started);
        assert!(animation.movement_anim_clump.associations.is_empty());
    }
}
