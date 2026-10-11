//! re3 reference: references/re-GTA/src/core/timebars.cpp
//! re3 reference: references/re-GTA/src/core/timebars.h
//!
//! Optional timebar capture state. It is inert unless a caller records timers.

use bevy::prelude::Resource;

pub const MAX_TIMEBARS: usize = 32;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct TimeBars {
    pub timers: Vec<TimeBarEntry>,
    pub frame_start_ms: u32,
    pub frame_end_ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimeBarEntry {
    pub id: i32,
    pub name: String,
    pub start_ms: u32,
    pub end_ms: Option<u32>,
}

impl Default for TimeBars {
    fn default() -> Self {
        Self {
            timers: Vec::with_capacity(MAX_TIMEBARS),
            frame_start_ms: 0,
            frame_end_ms: 0,
        }
    }
}

impl TimeBars {
    pub fn tbInit(&mut self) {
        self.timers.clear();
        self.frame_start_ms = 0;
        self.frame_end_ms = 0;
    }

    pub fn tbStartTimer(&mut self, id: i32, name: impl AsRef<str>, now_ms: u32) {
        if self.timers.len() == MAX_TIMEBARS {
            self.timers.remove(0);
        }
        self.timers.push(TimeBarEntry {
            id,
            name: name.as_ref().to_owned(),
            start_ms: now_ms,
            end_ms: None,
        });
    }

    pub fn tbEndTimer(&mut self, name: impl AsRef<str>, now_ms: u32) {
        let name = name.as_ref();
        if let Some(timer) = self
            .timers
            .iter_mut()
            .rev()
            .find(|timer| timer.name == name && timer.end_ms.is_none())
        {
            timer.end_ms = Some(now_ms);
        }
    }

    pub fn tbDisplay(&self) -> &[TimeBarEntry] {
        &self.timers
    }
}
