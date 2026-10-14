//! re3 reference: references/re-GTA/src/core/Wanted.cpp
//! re3 reference: references/re-GTA/src/core/Wanted.h
//!
//! Player wanted state + chaos → star level thresholds.

use bevy::prelude::Resource;
use rustgta3_core::core::Coordinates::CVector;

use super::Crime::{CCrimeBeingQd, eCrimeType};

/// re3 `CWanted::MaximumWantedLevel`
pub const MAXIMUM_WANTED_LEVEL: i32 = 6;
/// re3 `CWanted::nMaximumWantedLevel` default chaos cap.
pub const N_MAXIMUM_WANTED_LEVEL: i32 = 6400;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CWanted {
    pub m_nChaos: i32,
    pub m_nLastUpdateTime: i32,
    pub m_nLastWantedLevelChange: u32,
    pub m_fCrimeSensitivity: f32,
    pub m_CurrentCops: u8,
    pub m_MaxCops: u8,
    pub m_MaximumLawEnforcerVehicles: u8,
    pub m_CopsBeatingSuspect: u8,
    pub m_RoadblockDensity: i16,
    pub m_bIgnoredByCops: bool,
    pub m_bIgnoredByEveryone: bool,
    pub m_bSwatRequired: bool,
    pub m_bFbiRequired: bool,
    pub m_bArmyRequired: bool,
    pub m_nWantedLevel: i32,
    pub m_aCrimes: [CCrimeBeingQd; 16],
    pub m_pCops: [Option<usize>; 10],
    pub maximum_wanted_level: i32,
    pub maximum_chaos: i32,
}

impl Default for CWanted {
    fn default() -> Self {
        Self {
            m_nChaos: 0,
            m_nLastUpdateTime: 0,
            m_nLastWantedLevelChange: 0,
            m_fCrimeSensitivity: 1.0,
            m_CurrentCops: 0,
            m_MaxCops: 0,
            m_MaximumLawEnforcerVehicles: 0,
            m_CopsBeatingSuspect: 0,
            m_RoadblockDensity: 0,
            m_bIgnoredByCops: false,
            m_bIgnoredByEveryone: false,
            m_bSwatRequired: false,
            m_bFbiRequired: false,
            m_bArmyRequired: false,
            m_nWantedLevel: 0,
            m_aCrimes: [CCrimeBeingQd::default(); 16],
            m_pCops: [None; 10],
            maximum_wanted_level: MAXIMUM_WANTED_LEVEL,
            maximum_chaos: N_MAXIMUM_WANTED_LEVEL,
        }
    }
}

impl CWanted {
    pub fn initialise() -> Self {
        Self::default()
    }

    #[allow(non_snake_case)]
    pub fn Initialise(&mut self) {
        let maximum_wanted_level = self.maximum_wanted_level;
        let maximum_chaos = self.maximum_chaos;
        *self = Self::default();
        self.maximum_wanted_level = maximum_wanted_level;
        self.maximum_chaos = maximum_chaos;
    }

    /// re3 `CWanted::Reset`.
    ///
    /// The C++ method first releases cops from pursuit and then calls
    /// `Initialise`. Pursuit objectives live on ECS ped entities here, so
    /// `process_cop_ai` releases those peds on the next World pass after this
    /// authoritative wanted state is cleared.
    #[allow(non_snake_case)]
    pub fn Reset(&mut self) {
        self.Initialise();
    }

    #[allow(non_snake_case)]
    pub fn ClearQdCrimes(&mut self) {
        for crime in &mut self.m_aCrimes {
            crime.crime_type = eCrimeType::CRIME_NONE;
        }
    }

    #[allow(non_snake_case)]
    pub fn SetWantedLevel(&mut self, level: i32) {
        self.set_wanted_level_at(level, 0);
    }

    pub fn set_wanted_level_at(&mut self, level: i32, now_ms: u32) {
        let level = level.min(self.maximum_wanted_level);
        self.ClearQdCrimes();
        self.m_nChaos = match level {
            0 => 0,
            1 => 60,
            2 => 220,
            3 => 420,
            4 => 820,
            5 => 1620,
            6 => 3220,
            _ => self.m_nChaos,
        };
        self.update_wanted_level_at(now_ms);
    }

    #[allow(non_snake_case)]
    pub fn SetWantedLevelNoDrop(&mut self, level: i32) {
        if level > self.m_nWantedLevel {
            self.SetWantedLevel(level);
        }
    }

    pub fn set_wanted_level(&mut self, level: i32) {
        self.SetWantedLevel(level);
    }

    pub fn set_wanted_level_no_drop(&mut self, level: i32) {
        self.SetWantedLevelNoDrop(level);
    }

    pub fn get_wanted_level(&self) -> i32 {
        self.m_nWantedLevel
    }

    pub fn set_maximum_wanted_level(&mut self, level: i32) {
        match level {
            0 => (self.maximum_chaos, self.maximum_wanted_level) = (0, 0),
            1 => (self.maximum_chaos, self.maximum_wanted_level) = (120, 1),
            2 => (self.maximum_chaos, self.maximum_wanted_level) = (300, 2),
            3 => (self.maximum_chaos, self.maximum_wanted_level) = (600, 3),
            4 => (self.maximum_chaos, self.maximum_wanted_level) = (1200, 4),
            5 => (self.maximum_chaos, self.maximum_wanted_level) = (2400, 5),
            6 => (self.maximum_chaos, self.maximum_wanted_level) = (4800, 6),
            _ => {}
        }
    }

    pub fn register_crime(
        &mut self,
        crime_type: eCrimeType,
        position: CVector,
        id: i32,
        police_doesnt_care: bool,
        now_ms: u32,
    ) {
        self.add_crime_to_queue(crime_type, id, position, false, police_doesnt_care, now_ms);
    }

    pub fn register_crime_immediately(
        &mut self,
        crime_type: eCrimeType,
        position: CVector,
        id: i32,
        police_doesnt_care: bool,
        frenzy_ongoing: bool,
        wanted_level_drop: i32,
        now_ms: u32,
    ) {
        if !self.add_crime_to_queue(crime_type, id, position, false, police_doesnt_care, now_ms) {
            self.report_crime_now(
                crime_type,
                police_doesnt_care,
                frenzy_ongoing,
                wanted_level_drop,
                now_ms,
            );
        }
    }

    /// Returns true only when the matching queued crime was already reported.
    pub fn add_crime_to_queue(
        &mut self,
        crime_type: eCrimeType,
        id: i32,
        position: CVector,
        reported: bool,
        police_doesnt_care: bool,
        now_ms: u32,
    ) -> bool {
        if let Some(crime) = self
            .m_aCrimes
            .iter_mut()
            .find(|crime| crime.crime_type == crime_type && crime.id == id)
        {
            if crime.reported {
                return true;
            }
            if reported {
                crime.reported = true;
            }
            return false;
        }
        if let Some(crime) = self
            .m_aCrimes
            .iter_mut()
            .find(|crime| crime.crime_type == eCrimeType::CRIME_NONE)
        {
            *crime = CCrimeBeingQd {
                crime_type,
                id,
                time_ms: now_ms,
                position,
                reported,
                police_doesnt_care,
            };
        }
        false
    }

    pub fn report_crime_now(
        &mut self,
        crime_type: eCrimeType,
        police_doesnt_care: bool,
        frenzy_ongoing: bool,
        wanted_level_drop: i32,
        now_ms: u32,
    ) {
        let sensitivity = self.m_fCrimeSensitivity * if frenzy_ongoing { 0.3 } else { 1.0 };
        let drop = wanted_level_drop.min(100);
        let mut chaos_scale = (1.0 - drop as f32 / 100.0) * sensitivity;
        if police_doesnt_care {
            chaos_scale *= 0.333;
        }
        let base = match crime_type {
            eCrimeType::CRIME_POSSESSION_GUN => 0.0,
            eCrimeType::CRIME_HIT_PED => 5.0,
            eCrimeType::CRIME_HIT_COP => 45.0,
            eCrimeType::CRIME_SHOOT_PED => 30.0,
            eCrimeType::CRIME_SHOOT_COP => 80.0,
            eCrimeType::CRIME_STEAL_CAR => 15.0,
            eCrimeType::CRIME_RUN_REDLIGHT => 10.0,
            eCrimeType::CRIME_RECKLESS_DRIVING | eCrimeType::CRIME_SPEEDING => 5.0,
            eCrimeType::CRIME_RUNOVER_PED => 18.0,
            eCrimeType::CRIME_RUNOVER_COP => 80.0,
            eCrimeType::CRIME_SHOOT_HELI => 400.0,
            eCrimeType::CRIME_PED_BURNED => 20.0,
            eCrimeType::CRIME_COP_BURNED => 80.0,
            eCrimeType::CRIME_VEHICLE_BURNED => 20.0,
            eCrimeType::CRIME_DESTROYED_CESSNA => 500.0,
            eCrimeType::CRIME_NONE => 0.0,
        };
        self.m_nChaos = (self.m_nChaos as f32 + base * chaos_scale) as i32;
        self.update_wanted_level_at(now_ms);
    }

    #[allow(non_snake_case)]
    pub fn UpdateWantedLevel(&mut self) {
        self.update_wanted_level_at(0);
    }

    pub fn update_wanted_level_at(&mut self, now_ms: u32) {
        let previous = self.m_nWantedLevel;
        self.m_nChaos = self.m_nChaos.min(self.maximum_chaos);
        if self.m_nChaos < 0 {
            return;
        }
        let (level, vehicles, cops, roadblocks) = match self.m_nChaos {
            0..=39 => (0, 0, 0, 0),
            40..=199 => (1, 1, 1, 0),
            200..=399 => (2, 2, 3, 0),
            400..=799 => (3, 2, 4, 4),
            800..=1599 => (4, 2, 6, 8),
            1600..=3199 => (5, 3, 8, 10),
            _ => (6, 3, 10, 12),
        };
        self.m_nWantedLevel = level;
        self.m_MaximumLawEnforcerVehicles = vehicles;
        self.m_MaxCops = cops;
        self.m_RoadblockDensity = roadblocks;
        if previous != level {
            self.m_nLastWantedLevelChange = now_ms;
        }
    }

    pub fn update_crimes_queue(
        &mut self,
        now_ms: u32,
        frenzy_ongoing: bool,
        wanted_level_drop: i32,
    ) {
        for index in 0..self.m_aCrimes.len() {
            let crime = self.m_aCrimes[index];
            if crime.crime_type == eCrimeType::CRIME_NONE {
                continue;
            }
            if now_ms > crime.time_ms.wrapping_add(500) && !crime.reported {
                self.report_crime_now(
                    crime.crime_type,
                    crime.police_doesnt_care,
                    frenzy_ongoing,
                    wanted_level_drop,
                    now_ms,
                );
                self.m_aCrimes[index].reported = true;
            }
            if now_ms > crime.time_ms.wrapping_add(10_000) {
                self.m_aCrimes[index].crime_type = eCrimeType::CRIME_NONE;
            }
        }
    }

    pub fn update(
        &mut self,
        now_ms: u32,
        police_presence: i32,
        frenzy_ongoing: bool,
        wanted_level_drop: i32,
    ) {
        if now_ms.wrapping_sub(self.m_nLastUpdateTime as u32) > 1_000 {
            if self.m_nWantedLevel > 1 {
                self.m_nLastUpdateTime = now_ms as i32;
            } else if police_presence == 0 {
                self.m_nLastUpdateTime = now_ms as i32;
                self.m_nChaos = (self.m_nChaos - 1).max(0);
                self.update_wanted_level_at(now_ms);
            }
            self.update_crimes_queue(now_ms, frenzy_ongoing, wanted_level_drop);
            self.m_pCops.sort_by_key(Option::is_none);
            self.m_CurrentCops = self.m_pCops.iter().flatten().count() as u8;
        }
    }

    pub fn register_chaos(&mut self, amount: i32) {
        self.m_nChaos = self.m_nChaos.saturating_add(amount.max(0));
        self.UpdateWantedLevel();
    }

    pub fn are_swat_required(&self) -> bool {
        self.m_nWantedLevel == 4 || self.m_bSwatRequired
    }

    pub fn are_fbi_required(&self) -> bool {
        self.m_nWantedLevel == 5 || self.m_bFbiRequired
    }

    pub fn are_army_required(&self) -> bool {
        self.m_nWantedLevel == 6 || self.m_bArmyRequired
    }

    pub fn num_of_helis_required(&self) -> i32 {
        if self.m_bIgnoredByCops || self.m_bIgnoredByEveryone {
            return 0;
        }
        match self.m_nWantedLevel {
            3 | 4 => 1,
            5 | 6 => 2,
            _ => 0,
        }
    }
}
