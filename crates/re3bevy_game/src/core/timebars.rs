//! re3 reference: references/re-GTA/src/core/timebars.cpp
//! re3 reference: references/re-GTA/src/core/timebars.h
//!
//! Timebar debug profiler rows.

use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq)]
pub struct CTimeBar {
    pub name: String,
    pub start_ms: u32,
    pub end_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Resource)]
pub struct CTimeBars {
    pub bars: Vec<CTimeBar>,
}

impl CTimeBars {
    #[allow(non_snake_case)]
    pub fn AddTimeBar(&mut self, name: impl Into<String>, start_ms: u32, end_ms: u32) {
        self.bars.push(CTimeBar {
            name: name.into(),
            start_ms,
            end_ms,
        });
    }
}
