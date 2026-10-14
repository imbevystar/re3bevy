//! CPed::Attack association lifecycle and CheckForPedsOnGroundToAttack.
use super::{
    Ped::{CPed, PedState, blend_ped_movement_animation, eMoveState, eObjective, eWaitState},
    PedAnimation::PedAnimation,
    PedVitality::PedVitality,
};
use crate::{
    animation::{
        AnimAssocDefinitions::ASSOCGRP_STD,
        AnimBlendAssociation::{
            ASSOC_DELETEFADEDOUT, ASSOC_RUNNING, AnimBlendCallbackEvent, AnimBlendCallbackType,
        },
        AnimManager::CAnimManager,
        AnimationId::*,
    },
    core::FileLoader::CFileLoader,
    weapons::{
        WeaponInfo::{CWeaponInfo, WEAPONFLAG_THROW},
        WeaponType::{eWeaponFire, eWeaponState, eWeaponType},
    },
};
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum PedOnGroundState {
    #[default]
    NoPed,
    InFrontOfAttacker,
    OnTheFloor,
    DeadOnTheFloor,
}

pub(crate) struct GroundAttackCandidate {
    pub position: CVector,
    pub state: PedState,
    pub head_height: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GroundAttackChoice {
    pub state: PedOnGroundState,
    pub handle: Option<i32>,
    pub normalized_heading: Option<f32>,
}

/// Lookup borrows the canonical ten-entry near list, preserving its ordering.
/// Missing head input blocks classification only where native code reads it.
pub(crate) fn check_for_peds_on_ground_to_attack(
    attacker: &CPed,
    heading: f32,
    nasty_game: bool,
    mut lookup: impl FnMut(i32) -> Option<GroundAttackCandidate>,
) -> Option<GroundAttackChoice> {
    let mut result = GroundAttackChoice::default();
    if !nasty_game {
        return Some(result);
    }
    let mut floor = None;
    let mut dead = None;
    let mut below = None;
    for handle in attacker
        .near_peds
        .iter()
        .take(usize::from(attacker.num_near_peds))
        .flatten()
        .copied()
    {
        let Some(candidate) = lookup(handle) else {
            continue;
        };
        let position = attacker.physical.entity.get_position();
        let distance = (candidate.position - position).magnitude();
        if distance >= 2.0 {
            continue;
        }
        let rotation = CGeneral::limit_radian_angle(heading);
        result.normalized_heading = Some(rotation);
        let angle = CGeneral::limit_radian_angle(CGeneral::get_radian_angle_between_points(
            candidate.position.x,
            candidate.position.y,
            position.x,
            position.y,
        ));
        let mut difference = (angle - rotation).abs();
        if difference > std::f32::consts::PI {
            difference = std::f32::consts::TAU - difference;
        }
        if matches!(
            candidate.state,
            PedState::PED_FALL | PedState::PED_GETUP | PedState::PED_DIE | PedState::PED_DEAD
        ) {
            if difference < 65.0_f32.to_radians() {
                if candidate.state == PedState::PED_DEAD {
                    dead.get_or_insert(handle);
                } else if candidate.head_height? <= -0.6 {
                    floor.get_or_insert(handle);
                }
            }
        } else if (distance < 0.8 && difference < 75.0_f32.to_radians())
            || (distance < 1.3 && difference < 55.0_f32.to_radians())
            || (distance < 1.7 && difference < 35.0_f32.to_radians())
            || (distance < 2.0 && difference < 30.0_f32.to_radians())
        {
            below = Some(handle);
            break;
        } else if difference < 75.0_f32.to_radians() {
            below.get_or_insert(handle);
        }
    }
    (result.state, result.handle) = if let Some(handle) = floor {
        (PedOnGroundState::OnTheFloor, Some(handle))
    } else if let Some(handle) = dead {
        (PedOnGroundState::DeadOnTheFloor, Some(handle))
    } else if let Some(handle) = below {
        (PedOnGroundState::InFrontOfAttacker, Some(handle))
    } else {
        (PedOnGroundState::NoPed, None)
    };
    Some(result)
}

fn clear_attack(
    ped: &mut CPed,
    vitality: &PedVitality,
    loader: &CFileLoader,
    general: &mut CGeneral,
    now: u32,
) {
    if ped.ped_state != PedState::PED_ATTACK
        || ped.flags.is_ducking
        || ped.wait_state == eWaitState::WAITSTATE_PLAYANIM_DUCK
    {
        return;
    }
    if ped.flags.is_pointing_gun_at && ped.look_target.is_some() {
        ped.ped_state = PedState::PED_AIM_GUN;
        ped.point_gun_at = ped.look_target;
        ped.flags.is_aiming_gun = true;
    } else if ped.objective != eObjective::OBJECTIVE_NONE {
        ped.ped_state = PedState::PED_IDLE;
        ped.move_state = eMoveState::PEDMOVE_STILL;
    } else {
        super::PedEvasion::restore_previous_state(ped, vitality, loader, general, now);
    }
}

/// Consume actual finish events, including native START_THROW -> WEAPON_THROW.
pub(crate) fn service_attack_finish(
    ped: &mut CPed,
    vitality: &PedVitality,
    animation: &mut PedAnimation,
    manager: &CAnimManager,
    info: &CWeaponInfo,
    loader: &CFileLoader,
    general: &mut CGeneral,
    now: u32,
) {
    let clump = &mut animation.movement_anim_clump;
    if !ped.flags.is_ducking
        && crate::weapons::WeaponRuntime::reload_animation(info).is_some_and(|id| {
            clump
                .associations
                .iter()
                .any(|association| association.anim_id == id)
        })
    {
        clear_attack(ped, vitality, loader, general, now);
        return;
    }
    let has_attack = clump.associations.iter().any(|association| {
        association.anim_id == info.anim_to_play
            || association.anim_id == info.anim2_to_play
            || (info.IsFlagSet(WEAPONFLAG_THROW) && association.anim_id == ANIM_STD_THROW_UNDER)
    });
    if !has_attack && !ped.flags.is_attacking && now >= ped.shoot_timer {
        clear_attack(ped, vitality, loader, general, now);
        return;
    }
    let Some(index) = clump.associations.iter().position(|association| {
        (association.anim_id == info.anim_to_play
            || association.anim_id == info.anim2_to_play
            || (info.IsFlagSet(WEAPONFLAG_THROW) && association.anim_id == ANIM_STD_THROW_UNDER))
            && association
                .callback_events
                .contains(&AnimBlendCallbackEvent::Finish)
    }) else {
        return;
    };
    let id = clump.associations[index].anim_id;
    if id == ANIM_STD_START_THROW {
        // CAnimManager::AddAnimation: full initial weight, no blend of other
        // associations. The original finish callback selects long throw for NPCs.
        let Some(mut next) =
            manager.create_anim_association(ASSOCGRP_STD, ANIM_STD_WEAPON_THROW as usize)
        else {
            bevy::log::warn_once!("npc_throw_waiting_original_followup");
            return;
        };
        clump.associations[index].callback_events.clear();
        clump.associations[index].blend_delta = -1000.0;
        next.start(0.0);
        next.set_finish_callback();
        clump.associations.insert(0, next);
        return;
    }
    clump.associations[index].callback_events.clear();
    if id == ANIM_STD_PARTIAL_PUNCH {
        clump.associations[index].blend_delta = -8.0;
        clump.associations[index].flags |= ASSOC_DELETEFADEDOUT;
        clear_attack(ped, vitality, loader, general, now);
    } else {
        if matches!(id, ANIM_STD_WEAPON_THROW | ANIM_STD_THROW_UNDER)
            && ped.weapons[ped.current_weapon as usize].ammo_total > 0
        {
            ped.weapon_model_id = info.model_id;
        }
        if !ped.flags.is_attacking {
            clear_attack(ped, vitality, loader, general, now);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NpcAttackAnimationSample {
    pub animation_id: i32,
    pub previous_time: f32,
    pub current_time: f32,
    pub time_step: f32,
    pub fire_frame: f32,
    pub running: bool,
    pub attack_should_continue: bool,
}

pub(crate) fn sample_attack_animation(
    ped: &mut CPed,
    animation: &mut PedAnimation,
    manager: &CAnimManager,
    info: &CWeaponInfo,
    ground: PedOnGroundState,
    now: u32,
) -> Option<NpcAttackAnimationSample> {
    if ped.flags.is_ducking {
        return None;
    }
    let clump = &mut animation.movement_anim_clump;
    if crate::weapons::WeaponRuntime::reload_animation(info).is_some_and(|id| {
        clump
            .associations
            .iter()
            .any(|association| association.anim_id == id)
    }) {
        return None;
    }
    let attack_should_continue = ped.flags.is_attacking || now < ped.shoot_timer;
    let index = clump
        .associations
        .iter()
        .position(|association| association.anim_id == info.anim_to_play)
        .or_else(|| {
            clump
                .associations
                .iter()
                .position(|association| association.anim_id == info.anim2_to_play)
        })
        .or_else(|| {
            info.IsFlagSet(WEAPONFLAG_THROW)
                .then(|| {
                    clump
                        .associations
                        .iter()
                        .position(|association| association.anim_id == ANIM_STD_THROW_UNDER)
                })
                .flatten()
        });
    let Some(index) = index else {
        if !attack_should_continue {
            return None;
        }
        let id = if info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE
            && ground >= PedOnGroundState::OnTheFloor
        {
            info.anim2_to_play
        } else {
            info.anim_to_play
        };
        let initial = ped.weapon_attack_animation_id.is_none();
        let delta = if initial && ped.current_weapon == eWeaponType::WEAPONTYPE_BASEBALLBAT {
            1000.0
        } else {
            8.0
        };
        let Some(index) = blend_ped_movement_animation(clump, manager, ASSOCGRP_STD, id, delta)
        else {
            bevy::log::warn_once!(id, "npc_attack_waiting_original_association");
            return None;
        };
        clump.associations[index].set_finish_callback();
        clump.associations[index].set_run();
        ped.weapon_attack_animation_id = Some(id);
        ped.weapon_attack_animation_start_ms = now;
        return None;
    };
    let association = &mut clump.associations[index];
    let id = association.anim_id;
    let fire_frame = if id == info.anim_to_play {
        info.anim_frame_fire
    } else if id == ANIM_STD_THROW_UNDER {
        0.2
    } else {
        info.anim2_frame_fire
    };
    let previous_time = association.current_time - association.time_step;
    let crossed = association.is_running()
        && previous_time <= fire_frame
        && association.current_time > fire_frame;
    if !crossed && association.speed < 1.0 {
        association.speed = 1.0;
    }
    ped.weapon_attack_animation_id = Some(id);
    Some(NpcAttackAnimationSample {
        animation_id: id,
        previous_time,
        current_time: association.current_time,
        time_step: association.time_step,
        fire_frame,
        running: association.is_running(),
        attack_should_continue,
    })
}

/// This runs after the world fire result so reload/contact state drives native
/// continuation. Muzzle sampling precedes the loop's pose reset.
pub(crate) fn finish_attack_frame(
    ped: &mut CPed,
    animation: &mut PedAnimation,
    manager: &CAnimManager,
    info: &CWeaponInfo,
    ground: PedOnGroundState,
    sample: NpcAttackAnimationSample,
    now: u32,
) -> Option<u16> {
    use crate::audio::soundlist::*;
    let clump = &mut animation.movement_anim_clump;
    let index = clump
        .associations
        .iter()
        .position(|association| association.anim_id == sample.animation_id)?;
    if ped.weapons[ped.current_weapon as usize].weapon_state == eWeaponState::WEAPONSTATE_RELOADING
        && crate::weapons::WeaponRuntime::reload_animation(info).is_some_and(|id| {
            clump
                .associations
                .iter()
                .any(|association| association.anim_id == id)
        })
    {
        return None;
    }
    let loop_end = if info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE
        && sample.animation_id == info.anim2_to_play
    {
        3.4 / 6.0
    } else {
        info.anim_loop_end
    };
    let mut echo = None;
    if sample.current_time > loop_end
        || (!sample.running && info.weapon_fire != eWeaponFire::WEAPON_FIRE_PROJECTILE)
    {
        if sample.current_time - 2.0 * sample.time_step <= loop_end
            && (ped.flags.is_attacking || now < ped.shoot_timer)
            && ped.weapons[ped.current_weapon as usize].weapon_state
                != eWeaponState::WEAPONSTATE_RELOADING
        {
            if info.weapon_fire != eWeaponFire::WEAPON_FIRE_MELEE
                || ground < PedOnGroundState::OnTheFloor
            {
                if sample.animation_id != info.anim2_to_play
                    || sample.animation_id == ANIM_STD_RBLOCK_SHOOT
                {
                    clump.associations[index].start(info.anim_loop_start);
                } else if blend_ped_movement_animation(
                    clump,
                    manager,
                    ASSOCGRP_STD,
                    info.anim_to_play,
                    8.0,
                )
                .is_none()
                {
                    bevy::log::warn_once!("npc_attack_waiting_original_primary_loop");
                }
            } else if sample.animation_id == info.anim2_to_play {
                clump.associations[index].set_current_time(0.1);
            } else if blend_ped_movement_animation(
                clump,
                manager,
                ASSOCGRP_STD,
                info.anim2_to_play,
                8.0,
            )
            .is_none()
            {
                bevy::log::warn_once!("npc_attack_waiting_original_ground_loop");
            }
        } else {
            ped.flags.is_aiming_gun = false;
            if sample.previous_time <= info.anim_loop_end {
                echo = match ped.current_weapon {
                    eWeaponType::WEAPONTYPE_UZI => Some(SOUND_WEAPON_UZI_BULLET_ECHO as u16),
                    eWeaponType::WEAPONTYPE_AK47 => Some(SOUND_WEAPON_AK47_BULLET_ECHO as u16),
                    eWeaponType::WEAPONTYPE_M16 => Some(SOUND_WEAPON_M16_BULLET_ECHO as u16),
                    _ => None,
                };
            }
            if ped.current_weapon == eWeaponType::WEAPONTYPE_FLAMETHROWER
                && clump.associations[index].is_running()
            {
                clump.associations[index].flags |= ASSOC_DELETEFADEDOUT;
                clump.associations[index].flags &= !ASSOC_RUNNING;
                clump.associations[index].blend_delta = -4.0;
            }
        }
    }
    ped.flags.is_attacking =
        sample.attack_should_continue && sample.current_time <= sample.fire_frame;
    echo
}

pub(crate) fn suppress_contact_finish(animation: &mut PedAnimation, id: i32, continuing: bool) {
    if let Some(association) = animation
        .movement_anim_clump
        .associations
        .iter_mut()
        .find(|association| association.anim_id == id)
    {
        association.speed = 0.5;
        if continuing {
            association.callback_type = AnimBlendCallbackType::None;
        }
    }
}

pub(crate) fn fire_offset(ped: &CPed, info: &CWeaponInfo, id: i32) -> CVector {
    let mut offset = info.fire_offset;
    if ped.current_weapon == eWeaponType::WEAPONTYPE_BASEBALLBAT && id == info.anim2_to_play {
        offset.z = 0.7 * info.radius - 1.0;
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{
            AnimBlendAssocGroup::CAnimBlendAssocGroup, AnimBlendAssociation::CAnimBlendAssociation,
        },
        peds::PedType::ePedType,
    };

    fn ped() -> CPed {
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut CGeneral::with_seed(1));
        ped.ped_state = PedState::PED_ATTACK;
        ped.flags.is_attacking = true;
        ped.near_peds[..3].copy_from_slice(&[Some(1), Some(2), Some(3)]);
        ped.num_near_peds = 3;
        ped
    }
    fn association(id: i32, time: f32) -> CAnimBlendAssociation {
        let mut association = CAnimBlendAssociation::new();
        association.anim_id = id;
        association.hierarchy.total_length = 1.0;
        association.current_time = time;
        association.time_step = 0.04;
        association.flags = ASSOC_RUNNING;
        association.set_finish_callback();
        association
    }
    fn bat_info() -> CWeaponInfo {
        CWeaponInfo {
            weapon_fire: eWeaponFire::WEAPON_FIRE_MELEE,
            radius: 0.4,
            anim_to_play: ANIM_STD_WEAPON_BAT_H,
            anim2_to_play: ANIM_STD_WEAPON_BAT_V,
            anim_loop_start: 0.1,
            anim_loop_end: 0.7,
            anim_frame_fire: 0.3,
            anim2_frame_fire: 0.5,
            ..Default::default()
        }
    }
    #[test]
    fn ground_selection_preserves_native_priority_order_head_and_standing_break() {
        let attacker = ped();
        let lookup = |handle| {
            Some(GroundAttackCandidate {
                position: CVector::new(0.0, 1.0, 0.0),
                state: if handle == 1 {
                    PedState::PED_DEAD
                } else {
                    PedState::PED_FALL
                },
                head_height: Some(if handle == 2 { 0.1 } else { -0.7 }),
            })
        };
        let selected = check_for_peds_on_ground_to_attack(&attacker, 0.0, true, lookup).unwrap();
        assert_eq!(selected.state, PedOnGroundState::OnTheFloor);
        assert_eq!(selected.handle, Some(3));
        let selected = check_for_peds_on_ground_to_attack(&attacker, 0.0, true, |handle| {
            Some(GroundAttackCandidate {
                position: CVector::new(0.0, 1.0, 0.0),
                state: if handle == 1 {
                    PedState::PED_IDLE
                } else {
                    PedState::PED_FALL
                },
                head_height: Some(-0.7),
            })
        })
        .unwrap();
        assert_eq!(selected.state, PedOnGroundState::InFrontOfAttacker);
        assert_eq!(selected.handle, Some(1));
        let selected = check_for_peds_on_ground_to_attack(&attacker, 0.0, true, |_| {
            Some(GroundAttackCandidate {
                position: CVector::new(0.0, 1.0, 2.0),
                state: PedState::PED_DEAD,
                head_height: None,
            })
        })
        .unwrap();
        assert_eq!(selected.state, PedOnGroundState::NoPed);
        assert!(
            check_for_peds_on_ground_to_attack(&attacker, 0.0, true, |_| Some(
                GroundAttackCandidate {
                    position: CVector::new(0.0, 1.0, 0.0),
                    state: PedState::PED_GETUP,
                    head_height: None
                }
            ))
            .is_none()
        );
        assert_eq!(
            check_for_peds_on_ground_to_attack(&attacker, 0.0, false, |_| panic!(
                "nasty gate must precede lookup"
            ))
            .unwrap()
            .state,
            PedOnGroundState::NoPed
        );
    }
    #[test]
    fn secondary_fire_is_sampled_before_ground_loop_pose_reset() {
        let mut ped = ped();
        ped.current_weapon = eWeaponType::WEAPONTYPE_BASEBALLBAT;
        let info = bat_info();
        let mut animation = PedAnimation::default();
        animation
            .movement_anim_clump
            .associations
            .push(association(info.anim2_to_play, 0.52));
        let manager = CAnimManager::default();
        let sample = sample_attack_animation(
            &mut ped,
            &mut animation,
            &manager,
            &info,
            PedOnGroundState::OnTheFloor,
            1000,
        )
        .unwrap();
        assert_eq!(sample.fire_frame, 0.5);
        assert!(
            sample.previous_time <= sample.fire_frame && sample.current_time > sample.fire_frame
        );
        assert_eq!(
            animation.movement_anim_clump.associations[0].current_time,
            0.52
        );
        animation.movement_anim_clump.associations[0].current_time = 0.58;
        ped.flags.is_attacking = true;
        let sample = sample_attack_animation(
            &mut ped,
            &mut animation,
            &manager,
            &info,
            PedOnGroundState::OnTheFloor,
            1040,
        )
        .unwrap();
        assert_eq!(
            animation.movement_anim_clump.associations[0].current_time,
            0.58
        );
        finish_attack_frame(
            &mut ped,
            &mut animation,
            &manager,
            &info,
            PedOnGroundState::OnTheFloor,
            sample,
            1040,
        );
        assert_eq!(
            animation.movement_anim_clump.associations[0].current_time,
            0.1
        );
        assert!(!ped.flags.is_attacking);
        assert!((fire_offset(&ped, &info, info.anim2_to_play).z - (-0.72)).abs() < 0.00001);
    }
    #[test]
    fn late_or_unrequested_attack_does_not_restart_and_reload_presence_clears_state() {
        let mut ped = ped();
        ped.current_weapon = eWeaponType::WEAPONTYPE_BASEBALLBAT;
        let info = bat_info();
        let manager = CAnimManager::default();
        let mut animation = PedAnimation::default();
        animation
            .movement_anim_clump
            .associations
            .push(association(info.anim2_to_play, 0.9));
        let sample = sample_attack_animation(
            &mut ped,
            &mut animation,
            &manager,
            &info,
            PedOnGroundState::OnTheFloor,
            1000,
        )
        .unwrap();
        finish_attack_frame(
            &mut ped,
            &mut animation,
            &manager,
            &info,
            PedOnGroundState::OnTheFloor,
            sample,
            1000,
        );
        assert_eq!(
            animation.movement_anim_clump.associations[0].current_time,
            0.9
        );
        ped.flags.is_attacking = false;
        animation.movement_anim_clump.associations[0].current_time = 0.58;
        let sample = sample_attack_animation(
            &mut ped,
            &mut animation,
            &manager,
            &info,
            PedOnGroundState::OnTheFloor,
            1000,
        )
        .unwrap();
        finish_attack_frame(
            &mut ped,
            &mut animation,
            &manager,
            &info,
            PedOnGroundState::OnTheFloor,
            sample,
            1000,
        );
        assert_eq!(
            animation.movement_anim_clump.associations[0].current_time,
            0.58
        );
        let info = CWeaponInfo {
            anim_to_play: ANIM_STD_WEAPON_HGUN_BODY,
            ..Default::default()
        };
        let mut reload = association(ANIM_STD_HGUN_RELOAD, 0.0);
        reload.blend_amount = 0.0;
        animation.movement_anim_clump.associations.push(reload);
        ped.objective = eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT;
        service_attack_finish(
            &mut ped,
            &PedVitality::default(),
            &mut animation,
            &manager,
            &info,
            &CFileLoader::default(),
            &mut CGeneral::with_seed(1),
            1000,
        );
        assert_eq!(ped.ped_state, PedState::PED_IDLE);
    }
    #[test]
    fn actual_throw_finish_starts_long_throw_and_partial_punch_restores_state() {
        let mut ped = ped();
        let mut animation = PedAnimation::default();
        let mut start = association(ANIM_STD_START_THROW, 0.99);
        start.update_time(0.02, 1.0);
        animation.movement_anim_clump.associations.push(start);
        let mut manager = CAnimManager::default();
        manager.anim_assoc_groups.push(CAnimBlendAssocGroup {
            assoc_list: vec![association(ANIM_STD_WEAPON_THROW, 0.0)],
        });
        let info = CWeaponInfo {
            flags: WEAPONFLAG_THROW,
            anim_to_play: ANIM_STD_START_THROW,
            anim2_to_play: ANIM_STD_WEAPON_THROW,
            ..Default::default()
        };
        service_attack_finish(
            &mut ped,
            &PedVitality::default(),
            &mut animation,
            &manager,
            &info,
            &CFileLoader::default(),
            &mut CGeneral::with_seed(1),
            1000,
        );
        assert_eq!(
            animation.movement_anim_clump.associations[0].anim_id,
            ANIM_STD_WEAPON_THROW
        );
        assert!(animation.movement_anim_clump.associations[0].is_running());
        assert_eq!(
            animation.movement_anim_clump.associations[0].blend_amount,
            1.0
        );
        animation.movement_anim_clump.associations.clear();
        let mut punch = association(ANIM_STD_PARTIAL_PUNCH, 0.99);
        punch.update_time(0.02, 1.0);
        animation.movement_anim_clump.associations.push(punch);
        ped.last_ped_state = PedState::PED_IDLE;
        let info = CWeaponInfo {
            anim_to_play: ANIM_STD_PARTIAL_PUNCH,
            ..Default::default()
        };
        service_attack_finish(
            &mut ped,
            &PedVitality::default(),
            &mut animation,
            &manager,
            &info,
            &CFileLoader::default(),
            &mut CGeneral::with_seed(1),
            1000,
        );
        assert_eq!(ped.ped_state, PedState::PED_IDLE);
        assert_eq!(
            animation.movement_anim_clump.associations[0].blend_delta,
            -8.0
        );
    }
    #[test]
    fn association_lookup_is_primary_first_and_contact_removes_finish_callback() {
        let mut ped = ped();
        let info = bat_info();
        let mut animation = PedAnimation::default();
        animation.movement_anim_clump.associations = vec![
            association(info.anim2_to_play, 0.52),
            association(info.anim_to_play, 0.32),
        ];
        let sample = sample_attack_animation(
            &mut ped,
            &mut animation,
            &CAnimManager::default(),
            &info,
            PedOnGroundState::OnTheFloor,
            1000,
        )
        .unwrap();
        assert_eq!(sample.animation_id, info.anim_to_play);
        suppress_contact_finish(&mut animation, sample.animation_id, true);
        let association = &animation.movement_anim_clump.associations[1];
        assert_eq!(association.speed, 0.5);
        assert_eq!(association.callback_type, AnimBlendCallbackType::None);
    }
}
