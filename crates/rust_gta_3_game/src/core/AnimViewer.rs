//! re3 reference: references/re-GTA/src/core/AnimViewer.cpp
//! re3 reference: references/re-GTA/src/core/AnimViewer.h
//!
//! Animation viewer debug state.

use bevy::prelude::Resource;

#[derive(Debug, Clone, Eq, PartialEq, Default, Resource)]
pub struct CAnimViewer {
    pub active: bool,
    pub current_model: String,
    pub current_animation: String,
}

impl CAnimViewer {
    #[allow(non_snake_case)]
    pub fn Initialise(&mut self) {
        *self = Self::default();
        self.active = true;
    }
}
