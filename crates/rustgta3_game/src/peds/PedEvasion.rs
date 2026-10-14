//! CPed::SetEvasiveStep, SetEvasiveDive and PedEvadeCB on the actor's clump.
use super::{
    Ped::{
        CPed, PedState, apply_script_ped_wander_path, blend_ped_movement_animation, eMoveState,
        eObjective,
    },
    PedAnimation::PedAnimation,
    PedLocomotion::PedLocomotion,
    PedVitality::PedVitality,
};
use crate::{
    animation::{
        AnimAssocDefinitions::ASSOCGRP_STD, AnimBlendAssociation::*, AnimManager::CAnimManager,
        AnimationId::*,
    },
    core::FileLoader::CFileLoader,
    modelinfo::ModelIndices::MI_RCBANDIT,
};
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};

#[derive(Debug, Clone, Copy)]
pub(crate) enum EvasionKind {
    Step,
    Dive,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EvasionReason {
    pub slot: usize,
    pub position: CVector,
    pub move_speed: CVector,
    pub model_id: usize,
    pub horn: bool,
}

struct EvasionAnimation {
    id: i32,
    heading: f32,
    say_evade: bool,
}

fn select_animation(
    ped: &CPed,
    locomotion: &PedLocomotion,
    reason: EvasionReason,
    kind: EvasionKind,
    no_dive: bool,
    general: &mut CGeneral,
) -> Option<EvasionAnimation> {
    if !ped.flags.responds_to_threats {
        return None;
    }
    let position = ped.physical.entity.get_position();
    let mut angle = CGeneral::limit_radian_angle(CGeneral::get_radian_angle_between_points(
        reason.position.x,
        reason.position.y,
        position.x,
        position.y,
    ));
    let current = CGeneral::limit_radian_angle(locomotion.rotation_cur);
    let direction = CGeneral::get_radian_angle_between_points(
        reason.move_speed.x,
        reason.move_speed.y,
        0.0,
        0.0,
    );
    let turn = CGeneral::limit_radian_angle(angle - current).abs();
    let away = CGeneral::limit_radian_angle(angle + std::f32::consts::PI);
    let danger_zone = CGeneral::limit_radian_angle(away - direction);
    match kind {
        EvasionKind::Step => {
            if turn > std::f32::consts::FRAC_PI_2 && reason.model_id != MI_RCBANDIT && !reason.horn
            {
                return None;
            }
            let hail = general.get_random_number() & 1 != 0
                && reason.model_id != MI_RCBANDIT
                && !reason.horn;
            let id = if hail {
                ANIM_STD_HAILTAXI
            } else {
                angle = if danger_zone > 0.0 {
                    direction - std::f32::consts::FRAC_PI_2
                } else {
                    direction + std::f32::consts::FRAC_PI_2
                };
                ANIM_STD_EVADE_STEP
            };
            Some(EvasionAnimation {
                id,
                heading: CGeneral::limit_radian_angle(angle),
                say_evade: !reason.horn,
            })
        }
        EvasionKind::Dive => {
            let mut hands_up = false;
            if reason.horn {
                angle = (general.get_random_number() & 1) as f32 * std::f32::consts::PI
                    - std::f32::consts::FRAC_PI_2
                    + direction;
            } else {
                // FIX_BUGS reverses SetEvasiveStep's side: the dive root travels
                // in the opposite direction to the step root in original PED.IFP.
                angle = if danger_zone > 0.0 {
                    direction + std::f32::consts::FRAC_PI_2
                } else {
                    direction - std::f32::consts::FRAC_PI_2
                };
                let turn = CGeneral::limit_radian_angle(angle - current).abs();
                if turn <= std::f32::consts::FRAC_PI_2 {
                    hands_up = general.get_random_number() & 1 != 0;
                } else if general.get_random_number() & 7 != 0 {
                    return None;
                }
            }
            Some(EvasionAnimation {
                id: if hands_up || no_dive {
                    ANIM_STD_HANDSUP
                } else {
                    ANIM_STD_EVADE_DIVE
                },
                heading: CGeneral::limit_radian_angle(angle),
                say_evade: !reason.horn,
            })
        }
    }
}

pub(crate) fn begin_evasion(
    ped: &mut CPed,
    vitality: &PedVitality,
    locomotion: &mut PedLocomotion,
    animation: &mut PedAnimation,
    manager: &CAnimManager,
    reason: EvasionReason,
    kind: EvasionKind,
    no_dive: bool,
    general: &mut CGeneral,
) -> Option<u16> {
    if !ped.is_ped_in_control(vitality) {
        return None;
    }
    let Some(selected) = select_animation(ped, locomotion, reason, kind, no_dive, general) else {
        return None;
    };
    let clump = &mut animation.movement_anim_clump;
    if selected.id != ANIM_STD_EVADE_DIVE
        && clump
            .associations
            .iter()
            .any(|association| association.anim_id == selected.id)
    {
        return None;
    }
    let Some(index) = blend_ped_movement_animation(clump, manager, ASSOCGRP_STD, selected.id, 8.0)
    else {
        bevy::log::warn_once!("ped_evasion_waiting_original_association: {}", selected.id);
        return None;
    };
    let association = &mut clump.associations[index];
    if selected.id != ANIM_STD_EVADE_DIVE {
        association.flags &= !ASSOC_DELETEFADEDOUT;
    }
    association.set_finish_callback();
    ped.set_stored_state();
    ped.ped_state = if selected.id == ANIM_STD_EVADE_DIVE {
        PedState::PED_DIVE_AWAY
    } else {
        PedState::PED_STEP_AWAY
    };
    ped.flags.is_aiming_gun = false;
    ped.flags.is_looking = selected.id != ANIM_STD_EVADE_DIVE;
    ped.look_target = ped.flags.is_looking.then_some(reason.slot);
    locomotion.rotation_cur = selected.heading;
    ped.physical.entity.set_heading(selected.heading);
    // Move state is preserved by SetStoredState. The partial evasion clip
    // contributes its own original root motion to CalculateNewVelocity.
    selected
        .say_evade
        .then_some(crate::audio::soundlist::SOUND_PED_EVADE as u16)
}

pub(crate) fn restore_previous_state(
    ped: &mut CPed,
    vitality: &PedVitality,
    loader: &CFileLoader,
    general: &mut CGeneral,
    now_ms: u32,
) {
    if !ped.can_set_ped_state()
        || ped.ped_state == PedState::PED_FALL
        || (ped.ped_state == PedState::PED_GETUP && !ped.flags.get_up_anim_started)
    {
        return;
    }
    let previous = ped.last_ped_state;
    match previous {
        PedState::PED_NONE
            if !ped.is_player()
                && ped.char_created_by != super::Ped::MISSION_CHAR
                && ped.objective == eObjective::OBJECTIVE_NONE =>
        {
            ped.ped_state = PedState::PED_IDLE;
            let path = (general.get_random_number() & 7) as i32;
            apply_script_ped_wander_path(ped, vitality, path, loader, general, now_ms);
        }
        PedState::PED_WANDER_PATH => {
            ped.ped_state = PedState::PED_WANDER_PATH;
            ped.flags.is_running = false;
            let nearby_node = ped.flags.find_new_node_after_state_restore
                && ped
                    .next_path_node
                    .and_then(|node| loader.path_find.path_nodes.get(node))
                    .is_some_and(|node| {
                        (node.pos - ped.physical.entity.get_position()).magnitude_sqr() < 49.0
                    });
            if nearby_node {
                ped.move_state = eMoveState::PEDMOVE_WALK;
            } else {
                let path = (general.get_random_number() & 7) as i32;
                apply_script_ped_wander_path(ped, vitality, path, loader, general, now_ms);
            }
        }
        PedState::PED_NONE | PedState::PED_IDLE => {
            ped.ped_state = PedState::PED_IDLE;
            ped.move_state = eMoveState::PEDMOVE_STILL;
        }
        _ => {
            ped.ped_state = previous;
            ped.move_state = ped.stored_move_state;
        }
    }
    ped.last_ped_state = PedState::PED_NONE;
}

pub(crate) fn service_evasion_finish(
    ped: &mut CPed,
    vitality: &PedVitality,
    animation: &mut PedAnimation,
    loader: &CFileLoader,
    general: &mut CGeneral,
    now_ms: u32,
) {
    let Some(index) = animation
        .movement_anim_clump
        .associations
        .iter()
        .position(|association| {
            matches!(
                association.anim_id,
                ANIM_STD_EVADE_DIVE | ANIM_STD_EVADE_STEP | ANIM_STD_HANDSUP | ANIM_STD_HAILTAXI
            ) && association
                .callback_events
                .contains(&AnimBlendCallbackEvent::Finish)
        })
    else {
        return;
    };
    let association = &mut animation.movement_anim_clump.associations[index];
    association.callback_events.clear();
    ped.flags.is_looking = false;
    ped.look_target = None;
    if association.anim_id == ANIM_STD_EVADE_DIVE {
        ped.flags.update_anim_heading = true;
        if ped.ped_state == PedState::PED_DIVE_AWAY {
            ped.get_up_timer = now_ms.wrapping_add(1);
            ped.ped_state = PedState::PED_FALL;
        }
        association.flags &= !ASSOC_FADEOUTWHENDONE;
        association.flags |= ASSOC_DELETEFADEDOUT;
    } else {
        if association.flags & ASSOC_FADEOUTWHENDONE == 0 && ped.ped_state != PedState::PED_ARRESTED
        {
            association.flags |= ASSOC_DELETEFADEDOUT;
            if association.blend_delta >= 0.0 {
                association.blend_delta = -4.0;
            }
        }
        if matches!(
            ped.ped_state,
            PedState::PED_STEP_AWAY | PedState::PED_DIVE_AWAY
        ) {
            restore_previous_state(ped, vitality, loader, general, now_ms);
        }
    }
}

pub(crate) fn service_getup_finish(
    ped: &mut CPed,
    vitality: &PedVitality,
    animation: &mut PedAnimation,
    loader: &CFileLoader,
    general: &mut CGeneral,
    now_ms: u32,
) -> bool {
    let clump = &mut animation.movement_anim_clump;
    let Some(index) = clump.associations.iter().position(|association| {
        matches!(association.anim_id, ANIM_STD_GET_UP | ANIM_STD_GET_UP_FRONT)
            && association
                .callback_events
                .contains(&AnimBlendCallbackEvent::Finish)
    }) else {
        return false;
    };
    clump.associations[index].callback_events.clear();
    clump.associations[index].blend_delta = -1000.0;
    if ped.ped_state == PedState::PED_GETUP {
        for association in &mut clump.associations {
            if association.is_partial() {
                association.blend_delta = -1000.0;
            }
        }
        restore_previous_state(ped, vitality, loader, general, now_ms);
    }
    ped.flags.fallen_down = false;
    ped.flags.get_up_anim_started = false;
    ped.impact_animation_id = None;
    ped.move_state = if matches!(
        ped.ped_state,
        PedState::PED_FLEE_POS | PedState::PED_FLEE_ENTITY
    ) {
        eMoveState::PEDMOVE_RUN
    } else {
        eMoveState::PEDMOVE_STILL
    };
    true
}

#[cfg(test)]
mod tests {
    use super::super::PedType::ePedType;
    use super::*;
    #[test]
    fn evade_step_facing_gate_and_no_dive_use_native_branches() {
        let mut general = CGeneral::with_seed(123);
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        let reason = EvasionReason {
            slot: 1,
            position: CVector::new(0.0, 3.0, 0.0),
            move_speed: CVector::new(0.0, -0.2, 0.0),
            model_id: 90,
            horn: false,
        };
        let angle = CGeneral::get_radian_angle_between_points(0.0, 3.0, 0.0, 0.0);
        let locomotion = PedLocomotion {
            rotation_cur: CGeneral::limit_radian_angle(angle + std::f32::consts::PI),
            ..Default::default()
        };
        assert!(
            select_animation(
                &ped,
                &locomotion,
                reason,
                EvasionKind::Step,
                false,
                &mut general
            )
            .is_none()
        );
        let horn = EvasionReason {
            horn: true,
            ..reason
        };
        assert_eq!(
            select_animation(
                &ped,
                &locomotion,
                horn,
                EvasionKind::Step,
                false,
                &mut general
            )
            .unwrap()
            .id,
            ANIM_STD_EVADE_STEP
        );
        assert_eq!(
            select_animation(
                &ped,
                &locomotion,
                horn,
                EvasionKind::Dive,
                true,
                &mut general
            )
            .unwrap()
            .id,
            ANIM_STD_HANDSUP
        );
        ped.flags.responds_to_threats = false;
        assert!(
            select_animation(
                &ped,
                &locomotion,
                horn,
                EvasionKind::Dive,
                false,
                &mut general
            )
            .is_none()
        );
    }

    #[test]
    fn dive_finish_requires_fall_then_authored_getup_finish_restores_state() {
        let mut general = CGeneral::with_seed(4);
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        ped.ped_state = PedState::PED_DIVE_AWAY;
        ped.last_ped_state = PedState::PED_FLEE_ENTITY;
        let vitality = PedVitality::default();
        let loader = CFileLoader::default();
        let mut animation = PedAnimation::default();
        let mut dive = CAnimBlendAssociation::new();
        dive.anim_id = ANIM_STD_EVADE_DIVE;
        dive.flags = ASSOC_PARTIAL | ASSOC_FRONTAL;
        dive.callback_events.push(AnimBlendCallbackEvent::Finish);
        animation.movement_anim_clump.associations.push(dive);
        service_evasion_finish(
            &mut ped,
            &vitality,
            &mut animation,
            &loader,
            &mut general,
            1000,
        );
        assert_eq!(ped.ped_state, PedState::PED_FALL);
        assert_eq!(ped.get_up_timer, 1001);
        assert!(ped.flags.update_anim_heading);
        assert_eq!(ped.last_ped_state, PedState::PED_FLEE_ENTITY);
        ped.ped_state = PedState::PED_GETUP;
        ped.flags.get_up_anim_started = true;
        let mut getup = CAnimBlendAssociation::new();
        getup.anim_id = ANIM_STD_GET_UP_FRONT;
        getup.callback_events.push(AnimBlendCallbackEvent::Finish);
        animation.movement_anim_clump.associations.push(getup);
        service_getup_finish(
            &mut ped,
            &vitality,
            &mut animation,
            &loader,
            &mut general,
            2000,
        );
        assert_eq!(ped.ped_state, PedState::PED_FLEE_ENTITY);
        assert_eq!(ped.move_state, eMoveState::PEDMOVE_RUN);
        assert!(!ped.flags.get_up_anim_started);
        assert!(!ped.flags.fallen_down);
        assert_eq!(
            animation.movement_anim_clump.associations[0].blend_delta,
            -1000.0
        );
    }
}
