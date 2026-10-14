//! re3 reference: references/re-GTA/src/core/Accident.cpp
//! re3 reference: references/re-GTA/src/core/Accident.h
//!
//! Fixed-size accident state consumed by ambulance/medic AI.

use bevy::prelude::{Res, ResMut, Resource};
use rust_gta_3_core::core::Coordinates::CVector;

use crate::{
    core::{
        EventList::{CEventList, EEventType},
        FileLoader::CFileLoader,
        Game::ScriptEntityWorld,
        Pools::CPoolHandle,
    },
    peds::Ped::{CPed, MISSION_CHAR},
};

pub const NUM_ACCIDENTS: usize = 20;
pub const MAX_MEDICS_TO_ATTEND_ACCIDENT: u32 = 2;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct CAccident {
    pub victim: Option<usize>,
    pub medics_attending: u32,
    pub medics_performing_cpr: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CAccidentVictimState {
    pub position: CVector,
    pub char_created_by: u8,
    pub health: f32,
}

#[derive(Debug, Clone, Eq, PartialEq, Resource)]
pub struct CAccidentManager {
    pub accidents: [CAccident; NUM_ACCIDENTS],
}

impl CAccidentManager {
    pub fn initialise() -> Self {
        Self {
            accidents: [CAccident::default(); NUM_ACCIDENTS],
        }
    }

    pub fn get_next_free_accident(&self) -> Option<usize> {
        self.accidents
            .iter()
            .position(|accident| accident.victim.is_none())
    }

    /// `vertical_line_hit` is the result of re3's line from two units below
    /// the victim down to Z=-100. An accident is registered only when that
    /// line does not hit world geometry.
    pub fn report_accident(
        &mut self,
        victim: usize,
        ped: &mut CPed,
        vertical_line_hit: bool,
    ) -> Option<usize> {
        if ped.is_player()
            || ped.char_created_by == MISSION_CHAR
            || ped.physical.entity.flags.render_scorched
            || ped.flags.body_part_just_came_off
            || !ped.flags.allow_medics_to_revive_me
            || ped.physical.flags.is_in_water
            || ped.current_phys_surface.is_some()
            || vertical_line_hit
            || self
                .accidents
                .iter()
                .any(|accident| accident.victim == Some(victim))
        {
            return None;
        }

        let slot = self.get_next_free_accident()?;
        self.accidents[slot] = CAccident {
            victim: Some(victim),
            medics_attending: 0,
            medics_performing_cpr: 0,
        };
        ped.last_accident = Some(slot);
        Some(slot)
    }

    pub fn clear_accident(&mut self, slot: usize) {
        if let Some(accident) = self.accidents.get_mut(slot) {
            *accident = CAccident::default();
        }
    }

    /// Searches CPR tiers in the same outer-loop order as re3. The callback
    /// resolves the ECS entity to the current ped state without storing a
    /// borrowed world object in this resource.
    pub fn find_nearest_accident<F>(
        &self,
        position: CVector,
        mut victim_state: F,
    ) -> Option<(usize, f32)>
    where
        F: FnMut(usize) -> Option<CAccidentVictimState>,
    {
        for cpr_count in 0..MAX_MEDICS_TO_ATTEND_ACCIDENT {
            let mut nearest = None;
            let mut minimum_distance = 999_999.0_f32;
            for (slot, accident) in self.accidents.iter().enumerate() {
                let Some(victim) = accident.victim else {
                    continue;
                };
                let Some(state) = victim_state(victim) else {
                    continue;
                };
                if state.char_created_by == MISSION_CHAR
                    || state.health != 0.0
                    || accident.medics_performing_cpr != cpr_count
                {
                    continue;
                }
                let dx = state.position.x - position.x;
                let dy = state.position.y - position.y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance / 2.0 > state.position.z - position.z && distance < minimum_distance {
                    minimum_distance = distance;
                    nearest = Some(slot);
                }
            }
            if let Some(slot) = nearest {
                return Some((slot, minimum_distance));
            }
        }
        None
    }

    pub fn count_active_accidents(&self) -> u16 {
        self.accidents
            .iter()
            .filter(|accident| accident.victim.is_some())
            .count() as u16
    }

    pub fn work_to_do_for_medics(&self) -> bool {
        self.accidents.iter().any(|accident| {
            accident.victim.is_some() && accident.medics_attending < MAX_MEDICS_TO_ATTEND_ACCIDENT
        })
    }

    pub fn unattended_accidents(&self) -> bool {
        self.accidents
            .iter()
            .any(|accident| accident.victim.is_some() && accident.medics_attending == 0)
    }
}

impl Default for CAccidentManager {
    fn default() -> Self {
        Self::initialise()
    }
}

// === FAZ10c-accident: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn update_accident_manager(
    mut events: ResMut<CEventList>,
    mut accidents: ResMut<CAccidentManager>,
    mut script_entities: ScriptEntityWorld,
    file_loader: Res<CFileLoader>,
) {
    let Some(event_slot) = events.get_event(EEventType::InjuredPed) else {
        return;
    };
    let entity_ref = events.events[event_slot].entity_ref;
    let ped_key = if script_entities.peds.contains_key(&script_entities.index, &entity_ref) {
        Some(entity_ref)
    } else {
        let pool_index = CPoolHandle(entity_ref).index();
        script_entities
            .peds
            .iter()
            .find(|(_, entity)| CPoolHandle(entity.handle).index() == pool_index)
            .map(|(&handle, _)| handle)
    };
    let Some(ped_key) = ped_key else {
        return;
    };
    let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &ped_key) else {
        return;
    };
    let position = entity.ped.physical.entity.get_position();
    let vertical_line_hit = file_loader
        .world
        .process_vertical_line_static(
            [position.x, position.y, position.z - 2.0],
            -100.0,
            &file_loader.model_info,
        )
        .is_some();
    let victim = CPoolHandle(entity.handle).index();
    let _ = accidents.report_accident(victim, &mut *entity.ped, vertical_line_hit);
    events.clear_event(event_slot);
}
