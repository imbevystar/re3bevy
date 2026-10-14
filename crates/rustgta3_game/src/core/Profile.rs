//! re3 reference: references/re-GTA/src/core/Profile.cpp
//! re3 reference: references/re-GTA/src/core/Profile.h
//!
//! CProfile timing slots.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct CProfileSlot {
    pub start_ms: u32,
    pub total_ms: u32,
    pub calls: u32,
    pub running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Resource)]
pub struct CProfile {
    pub slots: BTreeMap<String, CProfileSlot>,
}

impl CProfile {
    #[allow(non_snake_case)]
    pub fn Initialise(&mut self) {
        self.slots.clear();
    }

    #[allow(non_snake_case)]
    pub fn StartProfile(&mut self, name: impl Into<String>, now_ms: u32) {
        let slot = self.slots.entry(name.into()).or_default();
        slot.start_ms = now_ms;
        slot.running = true;
    }

    #[allow(non_snake_case)]
    pub fn EndProfile(&mut self, name: &str, now_ms: u32) {
        if let Some(slot) = self.slots.get_mut(name) {
            if slot.running {
                slot.total_ms = slot
                    .total_ms
                    .saturating_add(now_ms.saturating_sub(slot.start_ms));
                slot.calls = slot.calls.saturating_add(1);
                slot.running = false;
            }
        }
    }
}
