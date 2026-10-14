//! re3 reference: references/re-GTA/src/core/EventList.cpp
//! re3 reference: references/re-GTA/src/core/EventList.h
//!
//! Fixed 64-slot event list and its frame-to-frame state transitions.

use bevy::prelude::{Res, ResMut, Resource};
use rust_gta_3_core::core::Coordinates::CVector;
use rust_gta_3_core::core::Timer::CTimer;

use super::Crime::eCrimeType;

pub const NUMEVENTS: usize = 64;

#[derive(Debug, Clone, Resource)]
pub struct CEventList {
    pub first_free_slot_index: usize,
    pub events: [CEvent; NUMEVENTS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CEvent {
    pub event_type: EEventType,
    pub entity_type: EEventEntity,
    pub entity_ref: i32,
    pub criminal_ref: Option<i32>,
    pub position: CVector,
    pub timeout: u32,
    pub state: EEventState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EEventType {
    Null = 0,
    Assault = 1,
    RunRedLight = 2,
    AssaultPolice = 3,
    Gunshot = 4,
    InjuredPed = 5,
    DeadPed = 6,
    Fire = 7,
    StealCar = 8,
    HitAndRun = 9,
    HitAndRunCop = 10,
    ShootPed = 11,
    ShootCop = 12,
    Explosion = 13,
    PedSetOnFire = 14,
    CopSetOnFire = 15,
    CarSetOnFire = 16,
    AssaultNastyWeapon = 17,
    Icecream = 18,
    Atm = 19,
    Shopstall = 20,
    LastEvent = 21,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EEventEntity {
    None = 0,
    Ped = 1,
    Vehicle = 2,
    Object = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EEventState {
    State0 = 0,
    CanDelete = 1,
    Clear = 2,
}

impl CEventList {
    pub fn initialise() -> Self {
        Self {
            first_free_slot_index: 0,
            events: [CEvent::default(); NUMEVENTS],
        }
    }

    pub fn update(&mut self, now_ms: u32) {
        self.first_free_slot_index = 0;
        for event in &mut self.events {
            if event.event_type == EEventType::Null {
                continue;
            }
            if now_ms > event.timeout || event.state == EEventState::CanDelete {
                event.event_type = EEventType::Null;
                event.state = EEventState::State0;
            }
            if event.state == EEventState::Clear {
                event.state = EEventState::CanDelete;
            }
        }
    }

    pub fn register_entity_event(
        &mut self,
        event_type: EEventType,
        entity_type: EEventEntity,
        entity_ref: i32,
        criminal_ref: Option<i32>,
        position: CVector,
        timeout_ms: u32,
        now_ms: u32,
    ) -> Option<usize> {
        if let Some((slot, event)) = self.events.iter_mut().enumerate().find(|(_, event)| {
            event.event_type == event_type
                && event.entity_type == entity_type
                && event.entity_ref == entity_ref
        }) {
            event.timeout = now_ms.wrapping_add(timeout_ms);
            return Some(slot);
        }

        let slot = (self.first_free_slot_index..NUMEVENTS)
            .find(|&slot| self.events[slot].event_type == EEventType::Null)?;
        self.first_free_slot_index = slot;
        self.events[slot] = CEvent {
            event_type,
            entity_type,
            entity_ref,
            criminal_ref,
            position,
            timeout: now_ms.wrapping_add(timeout_ms),
            state: if event_type == EEventType::Gunshot {
                EEventState::Clear
            } else {
                EEventState::State0
            },
        };
        Some(slot)
    }

    pub fn register_position_event(
        &mut self,
        event_type: EEventType,
        position: CVector,
        timeout_ms: u32,
        now_ms: u32,
    ) -> Option<usize> {
        if let Some((slot, event)) = self.events.iter_mut().enumerate().find(|(_, event)| {
            event.event_type == event_type
                && event.entity_type == EEventEntity::None
                && event.position == position
        }) {
            event.timeout = now_ms.wrapping_add(timeout_ms);
            return Some(slot);
        }

        let slot = (self.first_free_slot_index..NUMEVENTS)
            .find(|&slot| self.events[slot].event_type == EEventType::Null)?;
        self.first_free_slot_index = slot;
        self.events[slot] = CEvent {
            event_type,
            entity_type: EEventEntity::None,
            entity_ref: 0,
            criminal_ref: None,
            position,
            timeout: now_ms.wrapping_add(timeout_ms),
            state: if event_type == EEventType::Gunshot {
                EEventState::Clear
            } else {
                EEventState::State0
            },
        };
        Some(slot)
    }

    pub fn get_event(&self, event_type: EEventType) -> Option<usize> {
        self.events
            .iter()
            .position(|event| event.event_type == event_type)
    }

    pub fn clear_event(&mut self, slot: usize) {
        if let Some(event) = self.events.get_mut(slot) {
            if event.state != EEventState::CanDelete {
                event.state = EEventState::Clear;
            }
        }
    }

    pub fn find_closest_event(
        &self,
        event_type: EEventType,
        position: CVector,
    ) -> Option<(usize, f32)> {
        let mut closest = None;
        let mut minimum_distance = 60.0_f32;
        for (slot, event) in self.events.iter().enumerate() {
            if event.event_type != event_type {
                continue;
            }
            let dx = position.x - event.position.x;
            let dy = position.y - event.position.y;
            let dz = position.z - event.position.z;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            if distance < minimum_distance {
                minimum_distance = distance;
                closest = Some(slot);
            }
        }
        closest.map(|slot| (slot, minimum_distance))
    }

    pub fn crime_for_event(event_type: EEventType) -> eCrimeType {
        match event_type {
            EEventType::Assault => eCrimeType::CRIME_HIT_PED,
            EEventType::RunRedLight => eCrimeType::CRIME_RUN_REDLIGHT,
            EEventType::AssaultPolice => eCrimeType::CRIME_HIT_COP,
            EEventType::Gunshot => eCrimeType::CRIME_POSSESSION_GUN,
            EEventType::StealCar => eCrimeType::CRIME_STEAL_CAR,
            EEventType::HitAndRun => eCrimeType::CRIME_RUNOVER_PED,
            EEventType::HitAndRunCop => eCrimeType::CRIME_RUNOVER_COP,
            EEventType::ShootPed => eCrimeType::CRIME_SHOOT_PED,
            EEventType::ShootCop => eCrimeType::CRIME_SHOOT_COP,
            EEventType::PedSetOnFire => eCrimeType::CRIME_PED_BURNED,
            EEventType::CopSetOnFire => eCrimeType::CRIME_COP_BURNED,
            EEventType::CarSetOnFire => eCrimeType::CRIME_VEHICLE_BURNED,
            _ => eCrimeType::CRIME_NONE,
        }
    }
}

impl Default for CEventList {
    fn default() -> Self {
        Self::initialise()
    }
}

impl Default for CEvent {
    fn default() -> Self {
        Self {
            event_type: EEventType::Null,
            entity_type: EEventEntity::None,
            entity_ref: 0,
            criminal_ref: None,
            position: CVector::ZERO,
            timeout: 0,
            state: EEventState::State0,
        }
    }
}

// === FAZ10c-eventlist: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn update_event_list(mut events: ResMut<CEventList>, timer: Res<CTimer>) {
    events.update(timer.time_in_milliseconds());
}
