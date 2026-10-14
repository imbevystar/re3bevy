//! re3 reference: references/re-GTA/src/control/GameLogic.cpp
//! re3 reference: references/re-GTA/src/control/GameLogic.h
//!
//! Wasted/busted timing and resurrection state. World-sector cleanup, camera,
//! streaming and pad mutations are exposed as intents and remain owned by their
//! respective Bevy resources.

use bevy::prelude::*;
use rustgta3_core::core::{Clock::CClock, Coordinates::CVector};

use crate::{
    control::Pickups::CPickups,
    core::{
        PlayerInfo::{CPlayerInfo, eWastedBustedState},
        Stats::CStats,
    },
    peds::{
        Ped::{PedState, eMoveState},
    },
};

pub const WASTED_BUSTED_FADE_TIME_MS: u32 = 0x800;
pub const WASTED_BUSTED_RESTART_TIME_MS: u32 = 0x1000;
pub const RESURRECTION_TIME_ADVANCE_MINUTES: u32 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResurrectionKind {
    Hospital,
    Police,
    FailedCriticalMission,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameLogicIntent {
    FadeOut {
        colour: [u8; 3],
        seconds: f32,
    },
    Resurrect {
        kind: ResurrectionKind,
        clear_weapons: bool,
        charge: i32,
    },
}

#[derive(Debug, Clone, Resource)]
pub struct CGameLogic {
    pub active_players: u8,
}

impl CGameLogic {
    pub fn init_at_start_of_game() -> Self {
        Self { active_players: 1 }
    }

    /// re3 `CGameLogic::PassTime` including day rollover and pickup timers.
    pub fn pass_time(
        time_minutes: u32,
        now_ms: u32,
        clock: &mut CClock,
        stats: &mut CStats,
        pickups: &mut CPickups,
    ) {
        let mut minutes = time_minutes + u32::from(clock.minutes());
        let mut hours = u32::from(clock.hours());
        while minutes >= 60 {
            minutes -= 60;
            hours += 1;
        }
        while hours >= 24 {
            hours -= 24;
            stats.days_passed += 1;
        }
        clock.set_game_clock(hours as u8, minutes as u8, now_ms);
        pickups.pass_time(time_minutes.saturating_mul(1000));
    }

    /// Pure transition part of `CGameLogic::Update`. The caller applies camera,
    /// restart-point, world and streaming side effects in C++ order.
    pub fn update_wasted_busted_state(
        player_info: &mut CPlayerInfo,
        current_ms: u32,
        previous_ms: u32,
        wanted_level: i32,
    ) -> Vec<GameLogicIntent> {
        let elapsed = current_ms.wrapping_sub(player_info.m_nWBTime);
        let previous_elapsed = previous_ms.wrapping_sub(player_info.m_nWBTime);
        let mut intents = Vec::with_capacity(2);
        let (fade_colour, resurrection) = match player_info.m_WBState {
            eWastedBustedState::WBSTATE_PLAYING => return intents,
            eWastedBustedState::WBSTATE_WASTED => ([200, 200, 200], ResurrectionKind::Hospital),
            eWastedBustedState::WBSTATE_BUSTED => ([0, 0, 0], ResurrectionKind::Police),
            eWastedBustedState::WBSTATE_FAILED_CRITICAL_MISSION => {
                ([0, 0, 0], ResurrectionKind::FailedCriticalMission)
            }
        };
        if elapsed > WASTED_BUSTED_FADE_TIME_MS && previous_elapsed <= WASTED_BUSTED_FADE_TIME_MS {
            intents.push(GameLogicIntent::FadeOut {
                colour: fade_colour,
                seconds: 2.0,
            });
        }
        if elapsed < WASTED_BUSTED_RESTART_TIME_MS {
            return intents;
        }

        player_info.m_WBState = eWastedBustedState::WBSTATE_PLAYING;
        let (charge, clear_weapons) = match resurrection {
            ResurrectionKind::Hospital => {
                let free = player_info.m_bGetOutOfHospitalFree;
                player_info.m_bGetOutOfHospitalFree = false;
                (if free { 0 } else { 1000 }, !free)
            }
            ResurrectionKind::Police => {
                let free = player_info.m_bGetOutOfJailFree;
                player_info.m_bGetOutOfJailFree = false;
                (if free { 0 } else { busted_charge(wanted_level) }, true)
            }
            ResurrectionKind::FailedCriticalMission => (0, false),
        };
        player_info.m_nMoney = player_info.m_nMoney.saturating_sub(charge).max(0);
        intents.push(GameLogicIntent::Resurrect {
            kind: resurrection,
            clear_weapons,
            charge,
        });
        intents
    }

    /// Player-local part of `RestorePlayerStuffDuringResurrection`.
    pub fn restore_player_stuff_during_resurrection(
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        mut position: CVector,
        angle_degrees: f32,
    ) {
        player.vitality.health = 100.0;
        player.vitality.armour = 0.0;
        player.ped.physical.entity.flags.is_visible = true;
        player.vitality.bloody_footprint_count_or_death_time = 0;
        player.ped.flags.do_bloody_footprints = false;
        player.adrenaline_active = false;
        player.adrenaline_time = 0;
        player.current_stamina = player.max_stamina;
        player.vitality.fire = None;
        player.wanted.Reset();
        player.weapon_lock_on_target = None;
        player.weapon_lock_on_object = None;
        player.have_target_selected = false;
        player.has_lock_on_target = false;
        player.ped.point_gun_at = None;
        player.ped.physical.entity.flags.remove_from_world = false;
        player.ped.seek_target = None;
        player.ped.seek_pos = CVector::ZERO;
        player.ped.flee_from_pos_x = 0.0;
        player.ped.flee_from_pos_y = 0.0;
        player.ped.flee_from = None;
        player.ped.flee_timer = 0;
        player.ped.prev_objective = crate::peds::Ped::eObjective::OBJECTIVE_NONE;
        player.ped.physical.entity.flags.uses_collision = true;
        player.ped.flags.is_aiming_gun = false;
        player.ped.flags.is_looking = false;
        player.ped.flags.keep_trying_to_look = false;
        player.ped.flags.is_pointing_gun_at = false;
        player.ped.flags.render_ped_in_car = true;
        player.ped.flags.is_ped_die_anim_playing = false;
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.last_ped_state = PedState::PED_NONE;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        player.ped.objective = crate::peds::Ped::eObjective::OBJECTIVE_NONE;
        player.ped.anim_group = crate::animation::AnimAssocDefinitions::ASSOCGRP_PLAYER as i32;
        player.animation.movement_anim_clump.associations.clear();
        // RemoveAllAssociations invalidates pending requests and presentation
        // metadata for the old clump, as well as its last extracted root delta.
        player.ped.fall_animation_request = None;
        player.ped.impact_animation_id = None;
        player.ped.impact_animation_start_ms = 0;
        player.ped.impact_animation_speed = 1.0;
        player.ped.impact_animation_blend_delta = 0.0;
        player.ped.weapon_attack_animation_id = None;
        player.ped.weapon_attack_animation_start_ms = 0;
        player.locomotion.anim_move_delta = rustgta3_core::core::Coordinates::CVector2D::new(0.0, 0.0);
        player.locomotion.moved = rustgta3_core::core::Coordinates::CVector2D::new(0.0, 0.0);
        if let Some(velocity) = player.animation.movement_anim_clump.velocity2d.as_mut() {
            *velocity = (0.0, 0.0);
        }
        if let Some(velocity) = player.animation.movement_anim_clump.velocity3d.as_mut() {
            *velocity = CVector::ZERO;
        }
        player.jump_association = None;
        player.jump_animation_time = 0.0;
        player.selected_wep_slot = crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_UNARMED;
        player.evade_amount = 0;
        player.can_be_damaged = true;
        player.walk_angle = 0.0;
        player.mouse_leg_turn_angle = None;
        position.z += 1.0;
        player.set_position(position);
        player.ped.physical.move_speed = CVector::ZERO;
        player.move_speed = 0.0;
        player.set_heading_degrees(angle_degrees);
        player.locomotion.heading_rate = 15.0;
        player.active_in_world = true;
    }
}

pub fn busted_charge(wanted_level: i32) -> i32 {
    match wanted_level {
        i32::MIN..=1 => 100,
        2 => 200,
        3 => 400,
        4 => 600,
        5 => 900,
        _ => 1500,
    }
}
