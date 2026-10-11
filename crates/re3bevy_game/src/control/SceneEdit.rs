//! re3 reference: references/re-GTA/src/control/SceneEdit.cpp
//! re3 reference: references/re-GTA/src/control/SceneEdit.h
//!
//! Cutscene/scene editor state preserved as a debug-only data surface.

use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, Eq, Default, Resource)]
pub struct CSceneEdit {
    pub enabled: bool,
    pub current_command: usize,
    pub selected_actor: Option<usize>,
    pub recorded_commands: Vec<String>,
}

impl CSceneEdit {
    #[allow(non_snake_case)]
    pub fn Initialise(&mut self) {
        *self = Self::default();
    }

    #[allow(non_snake_case)]
    pub fn LoadMovie(&mut self, name: impl Into<String>) {
        self.recorded_commands.push(name.into());
        self.enabled = true;
    }
}
