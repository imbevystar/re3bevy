//! re3 reference: references/re-GTA/src/core/Debug.cpp
//! re3 reference: references/re-GTA/src/core/Debug.h
//!
//! re3 debug print buffer.

use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, Eq, Default, Resource)]
pub struct CDebug {
    pub lines: Vec<String>,
    pub enabled: bool,
}

impl CDebug {
    #[allow(non_snake_case)]
    pub fn DebugInit(&mut self) {
        self.enabled = true;
        self.lines.clear();
    }

    #[allow(non_snake_case)]
    pub fn DebugShutdown(&mut self) {
        self.enabled = false;
        self.lines.clear();
    }

    #[allow(non_snake_case)]
    pub fn DebugAddText(&mut self, text: impl Into<String>) {
        if self.enabled {
            self.lines.push(text.into());
        }
    }
}
