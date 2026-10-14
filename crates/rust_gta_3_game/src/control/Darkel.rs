//! re3 reference: references/re-GTA/src/control/Darkel.cpp
//! re3 reference: references/re-GTA/src/control/Darkel.h

use bevy::{prelude::*, window::PrimaryWindow};
use rust_gta_3_core::core::Timer::CTimer;

use crate::{
    audio::{DMAudio::cDMAudio, soundlist::*},
    control::{
        Replay::{CReplay, ReplayMode},
        Script::{CTheScripts, ScriptCommandEffect, script_key_to_string},
    },
    core::{
        PlayerInfo::CPlayerInfo,
        Stats::CStats,
        common::{DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH},
    },
    modelinfo::ModelIndices::NUM_DEFAULT_MODELS,
    objects::ParticleObject::CRGBA,
    peds::{
        PlayerPed::CWorldPlayers,
        Population::CPopulation,
    },
    renderer::Font::{CFont, FONT_BANK, FONT_HEADING},
    text::Text::{CText, GtaWideString},
    weapons::{
        Weapon::CWeapon,
        WeaponInfo::CWeaponInfoStore,
        WeaponType::{WEAPONTYPE_TOTALWEAPONS, eWeaponType, script_weapon_type},
    },
};

pub const FRENZY_ANY_PED: i32 = -1;
pub const FRENZY_ANY_CAR: i32 = -2;
pub const KILLFRENZY_NONE: u16 = 0;
pub const KILLFRENZY_ONGOING: u16 = 1;
pub const KILLFRENZY_PASSED: u16 = 2;
pub const KILLFRENZY_FAILED: u16 = 3;

pub(crate) fn draw_darkel_messages(
    darkel: Option<Res<CDarkel>>,
    timer: Res<CTimer>,
    replay: Option<Res<CReplay>>,
    text: Option<Res<CText>>,
    mut font: Option<ResMut<CFont>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let (Some(darkel), Some(text), Some(font)) = (darkel.as_ref(), text.as_ref(), font.as_mut())
    else {
        return;
    };
    let (width, height) = windows
        .single()
        .map(|window| (window.width(), window.height()))
        .unwrap_or((DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT));
    darkel.draw_messages(
        timer.time_in_milliseconds(),
        timer.frame_counter(),
        replay
            .as_ref()
            .is_some_and(|replay| replay.mode == ReplayMode::Playback),
        width,
        height,
        font,
        text,
    );
}

#[derive(Debug, Clone, Resource)]
pub struct CDarkel {
    pub time_limit: i32,
    pub previous_time: i32,
    pub time_of_frenzy_start: u32,
    pub weapon_type: eWeaponType,
    pub ammo_interrupted_weapon: i32,
    pub kills_needed: i32,
    pub interrupted_weapon: eWeaponType,
    pub standard_sound_and_messages: bool,
    pub need_head_shot: bool,
    pub proper_kill_frenzy: bool,
    pub status: u16,
    pub registered_kills: [u16; NUM_DEFAULT_MODELS],
    pub models_to_kill: [i32; 4],
    pub start_message: Option<GtaWideString>,
}

impl CDarkel {
    pub fn init() -> Self {
        Self {
            time_limit: 0,
            previous_time: 0,
            time_of_frenzy_start: 0,
            weapon_type: eWeaponType::WEAPONTYPE_UNARMED,
            ammo_interrupted_weapon: 0,
            kills_needed: 0,
            interrupted_weapon: eWeaponType::WEAPONTYPE_UNARMED,
            standard_sound_and_messages: false,
            need_head_shot: false,
            proper_kill_frenzy: false,
            status: KILLFRENZY_NONE,
            registered_kills: [0; NUM_DEFAULT_MODELS],
            models_to_kill: [0; 4],
            start_message: None,
        }
    }

    pub fn calc_fade(time: u32, start: u32, end: u32) -> u8 {
        if time < start || time > end {
            return 0;
        }
        if time < start.saturating_add(500) {
            return (255 * (time - start) / 500) as u8;
        }
        if time <= end.saturating_sub(500) {
            255
        } else {
            (255 * (end - time) / 500) as u8
        }
    }

    pub fn frenzy_on_going(&self) -> bool {
        self.status == KILLFRENZY_ONGOING
    }

    pub fn read_status(&self) -> u16 {
        self.status
    }

    pub fn query_models_killed_by_player(&self, model_id: i32) -> u16 {
        usize::try_from(model_id)
            .ok()
            .and_then(|index| self.registered_kills.get(index).copied())
            .unwrap_or(0)
    }

    pub fn reset_models_killed_by_player(&mut self) {
        self.registered_kills.fill(0);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_frenzy(
        &mut self,
        weapon_type: eWeaponType,
        time: i32,
        kills: u16,
        models: [i32; 4],
        start_message: GtaWideString,
        proper_kill_frenzy: bool,
        standard_sound: bool,
        need_head_shot: bool,
        now_ms: u32,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        weapon_info: &CWeaponInfoStore,
        dmaudio: Option<&mut cDMAudio>,
    ) {
        let fixed_weapon = fixed_weapon(weapon_type);
        self.weapon_type = weapon_type;
        self.status = KILLFRENZY_ONGOING;
        self.kills_needed = i32::from(kills);
        self.models_to_kill = models;
        self.proper_kill_frenzy = proper_kill_frenzy;
        self.start_message = (!proper_kill_frenzy).then_some(start_message);
        self.standard_sound_and_messages = standard_sound;
        self.need_head_shot = need_head_shot;
        self.time_of_frenzy_start = now_ms;
        self.time_limit = time;
        self.previous_time = time / 1000;

        if (fixed_weapon as usize) < WEAPONTYPE_TOTALWEAPONS {
            self.interrupted_weapon = player.ped.current_weapon;
            let slot = fixed_weapon as usize;
            if player.ped.weapons[slot].weapon_type != fixed_weapon {
                player.ped.weapons[slot] = CWeapon::initialise(fixed_weapon, 0);
            }
            self.ammo_interrupted_weapon = player.ped.weapons[slot].ammo_total;
            player.ped.weapons[slot].ammo_total = player.ped.weapons[slot]
                .ammo_total
                .saturating_add(30_000)
                .min(99_999);
            player.selected_wep_slot = fixed_weapon;
            player.ped.current_weapon = fixed_weapon;
            player.ped.weapons[slot].ammo_in_clip = player.ped.weapons[slot].ammo_total.min(
                weapon_info
                    .GetWeaponInfo(fixed_weapon)
                    .map_or(0, |info| info.amount_of_ammunition),
            );
        }
        if standard_sound {
            if let Some(audio) = dmaudio {
                audio.PlayFrontEndSound(SOUND_RAMPAGE_START as u16, 0);
            }
        }
    }

    pub fn register_car_blown_up_by_player(
        &mut self,
        model_id: i32,
        replay_playing: bool,
        stats: &mut CStats,
        dmaudio: Option<&mut cDMAudio>,
    ) {
        if replay_playing {
            return;
        }
        if self.frenzy_on_going() && self.model_matches(model_id, FRENZY_ANY_CAR) {
            self.kills_needed -= 1;
            if let Some(audio) = dmaudio {
                audio.PlayFrontEndSound(SOUND_RAMPAGE_CAR_BLOWN as u16, 0);
            }
        }
        self.increment_model_kill(model_id);
        stats.cars_exploded += 1;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_kill_by_player(
        &mut self,
        model_id: i32,
        ped_type: usize,
        criminal: bool,
        weapon: eWeaponType,
        headshot: bool,
        replay_playing: bool,
        stats: &mut CStats,
        dmaudio: Option<&mut cDMAudio>,
    ) {
        if replay_playing {
            return;
        }
        let weapon_matches = weapon == self.weapon_type
            || weapon == eWeaponType::WEAPONTYPE_EXPLOSION
            || (weapon == eWeaponType::WEAPONTYPE_UZI_DRIVEBY
                && self.weapon_type == eWeaponType::WEAPONTYPE_UZI)
            || (weapon == eWeaponType::WEAPONTYPE_RAMMEDBYCAR
                && self.weapon_type == eWeaponType::WEAPONTYPE_RUNOVERBYCAR)
            || (weapon == eWeaponType::WEAPONTYPE_RUNOVERBYCAR
                && self.weapon_type == eWeaponType::WEAPONTYPE_RAMMEDBYCAR)
            || (weapon == eWeaponType::WEAPONTYPE_FLAMETHROWER
                && self.weapon_type == eWeaponType::WEAPONTYPE_MOLOTOV);
        if self.frenzy_on_going()
            && weapon_matches
            && self.model_matches(model_id, FRENZY_ANY_PED)
            && (!self.need_head_shot || headshot)
        {
            self.kills_needed -= 1;
            if let Some(audio) = dmaudio {
                audio.PlayFrontEndSound(SOUND_RAMPAGE_KILL as u16, 0);
            }
        }
        stats.people_killed_by_player += 1;
        self.increment_model_kill(model_id);
        let stat_type = if criminal { 18 } else { ped_type };
        if let Some(value) = stats.peds_killed_of_this_type.get_mut(stat_type) {
            *value += 1;
        }
        if headshot {
            stats.heads_popped += 1;
        }
        stats.kills_since_last_checkpoint += 1;
    }

    pub fn register_kill_not_by_player(&self, replay_playing: bool, stats: &mut CStats) {
        if !replay_playing {
            stats.people_killed_by_others += 1;
        }
    }

    pub fn reset_on_player_death(
        &mut self,
        now_ms: u32,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        in_vehicle: bool,
        population: &mut CPopulation,
        weapon_info: &CWeaponInfoStore,
    ) {
        if !self.frenzy_on_going() {
            return;
        }
        population.all_random_peds_this_type = -1;
        self.status = KILLFRENZY_FAILED;
        self.time_of_frenzy_start = now_ms;
        restore_interrupted_weapon(self, player, in_vehicle, weapon_info);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        now_ms: u32,
        replay_playing: bool,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        in_vehicle: bool,
        population: &mut CPopulation,
        stats: &mut CStats,
        weapon_info: &CWeaponInfoStore,
        mut dmaudio: Option<&mut cDMAudio>,
    ) {
        if replay_playing || !self.frenzy_on_going() {
            return;
        }
        let frame_time = self.time_limit - now_ms.wrapping_sub(self.time_of_frenzy_start) as i32;
        if frame_time > 0 || self.time_limit < 0 {
            if let Some(audio) = dmaudio.as_deref_mut() {
                audio.PlayFrontEndSound(SOUND_RAMPAGE_ONGOING as u16, frame_time as u32);
            }
            let seconds = frame_time / 1000;
            if seconds != self.previous_time {
                if self.previous_time < 12 {
                    if let Some(audio) = dmaudio.as_deref_mut() {
                        audio.PlayFrontEndSound(SOUND_CLOCK_TICK as u16, seconds as u32);
                    }
                }
                self.previous_time = seconds;
            }
        } else {
            population.all_random_peds_this_type = -1;
            self.status = KILLFRENZY_FAILED;
            self.time_of_frenzy_start = now_ms;
            restore_interrupted_weapon(self, player, in_vehicle, weapon_info);
            if self.standard_sound_and_messages {
                if let Some(audio) = dmaudio.as_deref_mut() {
                    audio.PlayFrontEndSound(SOUND_RAMPAGE_FAILED as u16, 0);
                }
            }
        }

        // re3 uses a separate `if`, so completion can replace failure in this frame.
        if self.kills_needed <= 0 {
            population.all_random_peds_this_type = -1;
            self.status = KILLFRENZY_PASSED;
            if self.proper_kill_frenzy {
                stats.number_kill_frenzies_passed += 1;
            }
            self.time_of_frenzy_start = now_ms;
            player.wanted.set_wanted_level(0);
            restore_interrupted_weapon(self, player, in_vehicle, weapon_info);
            if self.standard_sound_and_messages {
                if let Some(audio) = dmaudio {
                    audio.PlayFrontEndSound(SOUND_RAMPAGE_PASSED as u16, 0);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_messages(
        &self,
        now_ms: u32,
        frame_counter: u32,
        replay_playing: bool,
        screen_width: f32,
        screen_height: f32,
        font: &mut CFont,
        text: &CText,
    ) {
        if replay_playing {
            return;
        }
        let sx = screen_width / DEFAULT_SCREEN_WIDTH;
        let sy = screen_height / DEFAULT_SCREEN_HEIGHT;
        match self.status {
            KILLFRENZY_ONGOING => {
                font.set_justify_off();
                font.set_background_off();
                font.set_centre_size((DEFAULT_SCREEN_WIDTH - 30.0) * sx);
                font.set_centre_on();
                font.set_prop_on();
                let elapsed = now_ms.wrapping_sub(self.time_of_frenzy_start);
                let message_range = if self.standard_sound_and_messages {
                    (3000, 11000)
                } else {
                    (0, 8000)
                };
                if elapsed >= message_range.0 && elapsed < message_range.1 {
                    if let Some(message) = self.start_message.as_ref() {
                        font.set_scale(1.3 * sx, 1.3 * sy);
                        font.set_justify_off();
                        font.set_color(CRGBA::new(
                            255,
                            255,
                            128,
                            Self::calc_fade(elapsed, message_range.0, message_range.1),
                        ));
                        font.set_font_style(FONT_BANK);
                        font.print_string(screen_width / 2.0, screen_height / 2.0, message);
                    }
                }
                font.set_scale(0.75 * sx, 1.5 * sy);
                font.set_centre_off();
                font.set_right_justify_on();
                font.set_font_style(FONT_HEADING);
                if self.time_limit >= 0 {
                    let left = self.time_limit - elapsed as i32;
                    let left = left.max(0) as u32;
                    if left > 4000 || frame_counter & 1 != 0 {
                        let timer =
                            ascii_wide(&format!("{}:{:02}", left / 60000, left % 60000 / 1000));
                        font.set_color(CRGBA::new(0, 0, 0, 255));
                        font.print_string(screen_width - 33.0 * sx, 109.0 * sy, &timer);
                        font.set_color(CRGBA::new(150, 100, 255, 255));
                        font.print_string(screen_width - 34.0 * sx, 108.0 * sy, &timer);
                    }
                }
                let kills = ascii_wide(&self.kills_needed.max(0).to_string());
                font.set_color(CRGBA::new(0, 0, 0, 255));
                font.print_string(screen_width - 33.0 * sx, 144.0 * sy, &kills);
                font.set_color(CRGBA::new(255, 128, 128, 255));
                font.print_string(screen_width - 34.0 * sx, 143.0 * sy, &kills);
            }
            KILLFRENZY_PASSED if self.standard_sound_and_messages => {
                let elapsed = now_ms.wrapping_sub(self.time_of_frenzy_start);
                if elapsed < 5000 {
                    font.set_background_off();
                    font.set_centre_size((DEFAULT_SCREEN_WIDTH - 20.0) * sx);
                    font.set_centre_on();
                    font.set_scale(1.5 * sx, 1.5 * sy);
                    font.set_justify_off();
                    font.set_color(CRGBA::new(128, 255, 128, Self::calc_fade(elapsed, 0, 5000)));
                    font.set_font_style(FONT_BANK);
                    let y = screen_height / 2.0 + (25.0 - elapsed as f32 * 0.01) * sy;
                    font.print_string(screen_width / 2.0, y, &text.get("KF_3"));
                }
            }
            _ => {}
        }
    }

    fn model_matches(&self, model_id: i32, wildcard: i32) -> bool {
        self.models_to_kill[0] == wildcard || self.models_to_kill.contains(&model_id)
    }

    fn increment_model_kill(&mut self, model_id: i32) {
        if let Some(value) = usize::try_from(model_id)
            .ok()
            .and_then(|index| self.registered_kills.get_mut(index))
        {
            *value = value.wrapping_add(1);
        }
    }
}

fn fixed_weapon(weapon: eWeaponType) -> eWeaponType {
    if weapon == eWeaponType::WEAPONTYPE_UZI_DRIVEBY {
        eWeaponType::WEAPONTYPE_UZI
    } else {
        weapon
    }
}

fn restore_interrupted_weapon(
    darkel: &CDarkel,
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    in_vehicle: bool,
    weapon_info: &CWeaponInfoStore,
) {
    let fixed = fixed_weapon(darkel.weapon_type);
    if (fixed as usize) < WEAPONTYPE_TOTALWEAPONS {
        player.selected_wep_slot = darkel.interrupted_weapon;
        player.ped.weapons[fixed as usize].ammo_total = darkel.ammo_interrupted_weapon;
    }
    if in_vehicle {
        player.ped.current_weapon = player.selected_wep_slot;
        let slot = player.ped.current_weapon as usize;
        if slot < player.ped.weapons.len() {
            player.ped.weapons[slot].ammo_in_clip = player.ped.weapons[slot].ammo_total.min(
                weapon_info
                    .GetWeaponInfo(player.ped.current_weapon)
                    .map_or(0, |info| info.amount_of_ammunition),
            );
        }
    }
}

fn ascii_wide(value: &str) -> GtaWideString {
    value.encode_utf16().collect()
}

// === FAZ8c-darkel: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn apply_script_darkel_effects(
    mut scripts: ResMut<CTheScripts>,
    mut darkel: Option<ResMut<CDarkel>>,
    mut world_players: CWorldPlayers,
    mut player_info: Option<ResMut<CPlayerInfo>>,
    mut population: Option<ResMut<CPopulation>>,
    weapon_info: Option<Res<CWeaponInfoStore>>,
    text: Option<Res<CText>>,
    timer: Res<CTimer>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::StartKillFrenzy {
                weapon_type,
                time_ms,
                kills,
                models,
                text_key,
                standard_sound,
                need_head_shot,
            } => {
                let Some(type_) = script_weapon_type(weapon_type) else {
                    continue;
                };
                let (Some(darkel), Some(mut player), Some(weapon_info), Some(text)) = (
                    darkel.as_mut(),
                    world_players.focused_mut(),
                    weapon_info.as_ref(),
                    text.as_ref(),
                ) else {
                    leftover.push(ScriptCommandEffect::StartKillFrenzy {
                        weapon_type,
                        time_ms,
                        kills,
                        models,
                        text_key,
                        standard_sound,
                        need_head_shot,
                    });
                    continue;
                };
                let key = script_key_to_string(&text_key);
                darkel.start_frenzy(
                    type_,
                    time_ms,
                    kills,
                    models,
                    text.get(&key),
                    key == "PAGE_00",
                    standard_sound,
                    need_head_shot,
                    timer.time_in_milliseconds(),
                    &mut player,
                    weapon_info,
                    dmaudio.as_deref_mut(),
                );
                scripts.kill_frenzy_status = darkel.status;
            }
            ScriptCommandEffect::ResetModelsKilledByPlayer => {
                if let Some(darkel) = darkel.as_mut() {
                    darkel.reset_models_killed_by_player();
                    scripts.models_killed_by_player = darkel.registered_kills;
                } else {
                    leftover.push(ScriptCommandEffect::ResetModelsKilledByPlayer);
                }
            }
            ScriptCommandEffect::PlayerFailedCriticalMission => {
                let focused_in_vehicle = world_players
                    .relation(world_players.player_in_focus() as i32)
                    .is_some();
                let (
                    Some(player_info),
                    Some(darkel),
                    Some(mut player),
                    Some(population),
                    Some(weapon_info),
                ) = (
                    player_info.as_mut(),
                    darkel.as_mut(),
                    world_players.focused_mut(),
                    population.as_mut(),
                    weapon_info.as_ref(),
                )
                else {
                    leftover.push(ScriptCommandEffect::PlayerFailedCriticalMission);
                    continue;
                };
                player_info.player_failed_critical_mission(
                    timer.time_in_milliseconds(),
                    &mut player,
                    focused_in_vehicle,
                    darkel,
                    population,
                    weapon_info,
                );
                scripts.kill_frenzy_status = darkel.status;
                scripts.models_killed_by_player = darkel.registered_kills;
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

pub(crate) fn update_darkel(
    mut darkel: Option<ResMut<CDarkel>>,
    mut world_players: CWorldPlayers,
    mut population: Option<ResMut<CPopulation>>,
    mut stats: Option<ResMut<CStats>>,
    weapon_info: Option<Res<CWeaponInfoStore>>,
    replay: Option<Res<CReplay>>,
    timer: Res<CTimer>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut scripts: ResMut<CTheScripts>,
) {
    let focused_in_vehicle = world_players
        .relation(world_players.player_in_focus() as i32)
        .is_some();
    let (Some(darkel), Some(mut player), Some(population), Some(stats), Some(weapon_info)) = (
        darkel.as_mut(),
        world_players.focused_mut(),
        population.as_mut(),
        stats.as_mut(),
        weapon_info.as_ref(),
    ) else {
        return;
    };
    darkel.update(
        timer.time_in_milliseconds(),
        replay
            .as_ref()
            .is_some_and(|replay| replay.mode == ReplayMode::Playback),
        &mut player,
        focused_in_vehicle,
        population,
        stats,
        weapon_info,
        dmaudio.as_deref_mut(),
    );
    scripts.kill_frenzy_status = darkel.status;
    scripts.models_killed_by_player = darkel.registered_kills;
}
