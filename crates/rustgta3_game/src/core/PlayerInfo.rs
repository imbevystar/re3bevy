//! re3 reference: references/re-GTA/src/core/PlayerInfo.cpp
//! re3 reference: references/re-GTA/src/core/PlayerInfo.h
//!
//! `CPlayerInfo::SavePlayerInfo` / `LoadPlayerInfo` payload state.

use bevy::prelude::{Res, ResMut, Resource};
use rustgta3_core::core::Timer::CTimer;

use crate::{
    control::{
        Darkel::CDarkel,
        Replay::{CReplay, ReplayMode},
    },
    core::{
        Game::ScriptEntityWorld,
        Stats::CStats,
    },
    peds::{
        PlayerPed::CWorldPlayers,
        Population::CPopulation,
        VehicleRelations::VehicleSeat,
    },
    save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter},
    text::{Messages::CMessages, Text::CText},
    weapons::WeaponInfo::CWeaponInfoStore,
};

pub const PLAYER_INFO_SAVE_SIZE: usize = 0x13C;
pub const PLAYER_NAME_LEN: usize = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum eWastedBustedState {
    WBSTATE_PLAYING = 0,
    WBSTATE_WASTED = 1,
    WBSTATE_BUSTED = 2,
    WBSTATE_FAILED_CRITICAL_MISSION = 3,
}

impl eWastedBustedState {
    fn from_i8(value: i8) -> Self {
        match value {
            1 => Self::WBSTATE_WASTED,
            2 => Self::WBSTATE_BUSTED,
            3 => Self::WBSTATE_FAILED_CRITICAL_MISSION,
            _ => Self::WBSTATE_PLAYING,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CPlayerInfo {
    pub m_nMoney: i32,
    pub previous_explosion_reward_ms: u32,
    pub explosions_since_reward: i32,
    pub m_WBState: eWastedBustedState,
    pub m_nWBTime: u32,
    pub m_nTrafficMultiplier: i16,
    pub m_fRoadDensity: f32,
    pub m_nVisibleMoney: i32,
    pub m_nCollectedPackages: i32,
    pub m_nTotalPackages: i32,
    pub m_nUnusedTaxiTimer: u32,
    pub m_bUnusedTaxiThing: bool,
    pub m_bInfiniteSprint: bool,
    pub m_bFastReload: bool,
    pub m_bGetOutOfJailFree: bool,
    pub m_bGetOutOfHospitalFree: bool,
    pub m_nUpsideDownCounter: i32,
    pub m_aPlayerName: [u8; PLAYER_NAME_LEN],
}

impl Default for CPlayerInfo {
    fn default() -> Self {
        Self::clear()
    }
}

impl CPlayerInfo {
    /// CPlayerInfo::AwardMoneyForExplosion, using loaded handling monetary value.
    pub(crate) fn award_money_for_explosion(&mut self, monetary_value: i32,
        now_ms: u32, general: &mut crate::core::General::CGeneral) {
        self.explosions_since_reward = if now_ms.wrapping_sub(self.previous_explosion_reward_ms) < 6000 {
            self.explosions_since_reward.wrapping_add(1)
        } else { 1 };
        self.previous_explosion_reward_ms = now_ms;
        let award = (monetary_value as f32 * 0.002) as i32;
        self.m_nMoney = self.m_nMoney.wrapping_add(award);
        for _ in 1..self.explosions_since_reward {
            general.get_random_number();
            self.m_nMoney = self.m_nMoney.wrapping_add(award);
        }
    }

    pub fn clear() -> Self {
        Self {
            m_nMoney: 0,
            previous_explosion_reward_ms: 0,
            explosions_since_reward: 0,
            m_WBState: eWastedBustedState::WBSTATE_PLAYING,
            m_nWBTime: 0,
            m_nTrafficMultiplier: 0,
            m_fRoadDensity: 1.0,
            m_nVisibleMoney: 0,
            m_nCollectedPackages: 0,
            m_nTotalPackages: 0,
            m_nUnusedTaxiTimer: 0,
            m_bUnusedTaxiThing: false,
            m_bInfiniteSprint: false,
            m_bFastReload: false,
            m_bGetOutOfJailFree: false,
            m_bGetOutOfHospitalFree: false,
            m_nUpsideDownCounter: 0,
            m_aPlayerName: [0; PLAYER_NAME_LEN],
        }
    }

    pub fn is_restarting_after_death(&self) -> bool {
        self.m_WBState == eWastedBustedState::WBSTATE_WASTED
    }

    pub fn is_restarting_after_arrest(&self) -> bool {
        self.m_WBState == eWastedBustedState::WBSTATE_BUSTED
    }

    #[allow(clippy::too_many_arguments)]
    pub fn kill_player(
        &mut self,
        now_ms: u32,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        in_vehicle: bool,
        darkel: &mut CDarkel,
        population: &mut CPopulation,
        weapon_info: &CWeaponInfoStore,
        messages: &mut CMessages,
        text: &CText,
        stats: &mut CStats,
    ) {
        if self.m_WBState != eWastedBustedState::WBSTATE_PLAYING {
            return;
        }
        self.m_WBState = eWastedBustedState::WBSTATE_WASTED;
        self.m_nWBTime = now_ms;
        darkel.reset_on_player_death(now_ms, player, in_vehicle, population, weapon_info);
        messages.add_big_message(text.get("DEAD"), 4000, 2, now_ms);
        stats.times_died += 1;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn arrest_player(
        &mut self,
        now_ms: u32,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        in_vehicle: bool,
        darkel: &mut CDarkel,
        population: &mut CPopulation,
        weapon_info: &CWeaponInfoStore,
        messages: &mut CMessages,
        text: &CText,
        stats: &mut CStats,
    ) {
        if self.m_WBState != eWastedBustedState::WBSTATE_PLAYING {
            return;
        }
        self.m_WBState = eWastedBustedState::WBSTATE_BUSTED;
        self.m_nWBTime = now_ms;
        darkel.reset_on_player_death(now_ms, player, in_vehicle, population, weapon_info);
        messages.add_big_message(text.get("BUSTED"), 5000, 2, now_ms);
        stats.times_arrested += 1;
    }

    pub fn player_failed_critical_mission(
        &mut self,
        now_ms: u32,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        in_vehicle: bool,
        darkel: &mut CDarkel,
        population: &mut CPopulation,
        weapon_info: &CWeaponInfoStore,
    ) {
        if self.m_WBState != eWastedBustedState::WBSTATE_PLAYING {
            return;
        }
        self.m_WBState = eWastedBustedState::WBSTATE_FAILED_CRITICAL_MISSION;
        self.m_nWBTime = now_ms;
        darkel.reset_on_player_death(now_ms, player, in_vehicle, population, weapon_info);
    }

    /// Money-counter easing from `CPlayerInfo::Process`. This runs once per
    /// gameplay frame; deliberately do not clamp overshoot, matching re3.
    pub fn process_visible_money(&mut self) {
        if self.m_nVisibleMoney == self.m_nMoney {
            return;
        }
        let diff = self.m_nMoney.wrapping_sub(self.m_nVisibleMoney);
        let change = match diff.unsigned_abs() {
            n if n > 100_000 => 12_345,
            n if n > 10_000 => 1_234,
            n if n > 1_000 => 123,
            n if n > 50 => 42,
            _ => 1,
        };
        if diff < 0 {
            self.m_nVisibleMoney = self.m_nVisibleMoney.wrapping_sub(change);
        } else {
            self.m_nVisibleMoney = self.m_nVisibleMoney.wrapping_add(change);
        }
    }

    /// Unused GTA III taxi-passenger reward from `CPlayerInfo::Process`.
    pub fn process_taxi_timer(&mut self, now_ms: u32, qualifying_taxi: bool) {
        if self.m_bUnusedTaxiThing && qualifying_taxi {
            let mut time_passed = now_ms.wrapping_sub(self.m_nUnusedTaxiTimer);
            while time_passed >= 1_000 {
                time_passed -= 1_000;
                self.m_nUnusedTaxiTimer = self.m_nUnusedTaxiTimer.wrapping_add(1_000);
                self.m_nMoney = self.m_nMoney.wrapping_add(1);
            }
        } else {
            self.m_nUnusedTaxiTimer = now_ms;
        }
    }

    pub fn SavePlayerInfo(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write(self.m_nMoney);
        writer.write(self.m_WBState as i8);
        writer.write(self.m_nWBTime);
        writer.write(self.m_nTrafficMultiplier);
        writer.write(self.m_fRoadDensity);
        writer.write(self.m_nVisibleMoney);
        writer.write(self.m_nCollectedPackages);
        writer.write(self.m_nTotalPackages);
        writer.write(bool_to_u8(self.m_bInfiniteSprint));
        writer.write(bool_to_u8(self.m_bFastReload));
        writer.write(bool_to_u8(self.m_bGetOutOfJailFree));
        writer.write(bool_to_u8(self.m_bGetOutOfHospitalFree));
        for byte in self.m_aPlayerName {
            writer.write(byte);
        }
        writer.write(self.m_nUnusedTaxiTimer);
        writer.write(bool_to_u8(self.m_bUnusedTaxiThing));
        if writer.offset() < PLAYER_INFO_SAVE_SIZE {
            writer.zero(PLAYER_INFO_SAVE_SIZE - writer.offset());
        }
        writer.into_inner()
    }

    pub fn LoadPlayerInfo(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let mut name = [0_u8; PLAYER_NAME_LEN];
        let out = Self {
            m_nMoney: reader.read::<i32>()?,
            previous_explosion_reward_ms: 0,
            explosions_since_reward: 0,
            m_WBState: eWastedBustedState::from_i8(reader.read::<i8>()?),
            m_nWBTime: reader.read::<u32>()?,
            m_nTrafficMultiplier: reader.read::<i16>()?,
            m_fRoadDensity: reader.read::<f32>()?,
            m_nVisibleMoney: reader.read::<i32>()?,
            m_nCollectedPackages: reader.read::<i32>()?,
            m_nTotalPackages: reader.read::<i32>()?,
            m_bInfiniteSprint: reader.read::<u8>()? != 0,
            m_bFastReload: reader.read::<u8>()? != 0,
            m_bGetOutOfJailFree: reader.read::<u8>()? != 0,
            m_bGetOutOfHospitalFree: reader.read::<u8>()? != 0,
            m_nUpsideDownCounter: 0,
            m_aPlayerName: {
                for byte in &mut name {
                    *byte = reader.read::<u8>()?;
                }
                name
            },
            m_nUnusedTaxiTimer: reader.read::<u32>()?,
            m_bUnusedTaxiThing: reader.read::<u8>()? != 0,
        };
        if reader.offset() < bytes.len().min(PLAYER_INFO_SAVE_SIZE) {
            reader.skip(bytes.len().min(PLAYER_INFO_SAVE_SIZE) - reader.offset())?;
        }
        Ok(out)
    }
}

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxi_timer_pays_each_complete_second_and_resets_when_not_qualifying() {
        let mut info = CPlayerInfo::clear();
        info.m_bUnusedTaxiThing = true;
        info.m_nUnusedTaxiTimer = 1_000;

        info.process_taxi_timer(3_550, true);
        assert_eq!(info.m_nMoney, 2);
        assert_eq!(info.m_nUnusedTaxiTimer, 3_000);

        info.process_taxi_timer(3_700, false);
        assert_eq!(info.m_nMoney, 2);
        assert_eq!(info.m_nUnusedTaxiTimer, 3_700);
    }

    #[test]
    fn taxi_state_roundtrips_through_player_info_save_padding() {
        let mut info = CPlayerInfo::clear();
        info.m_nUnusedTaxiTimer = 42_000;
        info.m_bUnusedTaxiThing = true;
        let bytes = info.SavePlayerInfo();
        assert_eq!(bytes.len(), PLAYER_INFO_SAVE_SIZE);

        let loaded = CPlayerInfo::LoadPlayerInfo(&bytes).unwrap();
        assert_eq!(loaded.m_nUnusedTaxiTimer, 42_000);
        assert!(loaded.m_bUnusedTaxiThing);
    }
}

// === FAZ8c-playerinfo: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn process_player_info_runtime(
    mut player_info: Option<ResMut<CPlayerInfo>>,
    players: CWorldPlayers,
    mut script_entities: ScriptEntityWorld,
    timer: Res<CTimer>,
    replay: Option<Res<CReplay>>,
) {
    // FIX_BUGS CPlayerInfo::Process returns immediately during replay playback.
    if replay
        .as_ref()
        .is_some_and(|replay| replay.mode == ReplayMode::Playback)
    {
        return;
    }
    let player_index = players.player_in_focus() as i32;
    let player_relation = players.relation(player_index);
    let (Some(player_info), Some(_)) = (player_info.as_mut(), players.focused()) else {
        return;
    };
    let qualifying_taxi = player_relation.is_some_and(|relation| relation.is_driver())
        && player_relation
            .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle))
            .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
            .is_some_and(|vehicle| {
                let base = vehicle.base();
                matches!(
                    i32::from(base.physical.entity.model_index),
                    value
                        if value == crate::modelinfo::ModelIndices::MI_TAXI as i32
                            || value == crate::modelinfo::ModelIndices::MI_CABBIE as i32
                            || value == crate::modelinfo::ModelIndices::MI_BORGNINE as i32
                ) && player_relation.is_some_and(|relation| {
                    relation.is_driver()
                        && script_entities
                            .relation_occupants(relation.vehicle)
                            .any(|(_, seat)| matches!(seat, VehicleSeat::Passenger(_)))
                })
            });
    player_info.process_taxi_timer(timer.time_in_milliseconds(), qualifying_taxi);
    player_info.process_visible_money();

    if timer.frame_counter() & 31 == 0 {
        let vehicle_handle = player_relation
            .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle));
        let rollover_burning = vehicle_handle
            .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
            .is_some_and(|vehicle| {
                let base = vehicle.base();
                base.physical.entity.get_up().z < 0.0
                    && base.physical.move_speed.magnitude() < 0.05
                    && !base.physical.flags.is_in_water
            });
        if rollover_burning {
            let up_z = vehicle_handle
                .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
                .map_or(0.0, |vehicle| vehicle.base().physical.entity.get_up().z);
            player_info.m_nUpsideDownCounter += if up_z < -0.5 { 2 } else { 1 };
        } else {
            player_info.m_nUpsideDownCounter = 0;
        }
        if player_info.m_nUpsideDownCounter > 6
            && let Some(handle) = vehicle_handle
            && let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                script_entities.vehicles.get_mut(&script_entities.index, &handle)
            && car.vehicle.vitality.can_be_damaged
        {
            car.vehicle.vitality.health = car.vehicle.vitality.health.min(249.0);
            car.damage.set_engine_status(225);
            car.set_on_fire_entity = None;
        }
    }
}
