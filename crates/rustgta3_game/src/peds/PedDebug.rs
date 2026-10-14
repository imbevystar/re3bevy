//! re3 reference: references/re-GTA/src/peds/PedDebug.cpp
//!
//! Ped debug flags and selected entity.

use bevy::prelude::{Entity, Resource};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Resource)]
pub struct CPedDebug {
    pub selected_ped: Option<Entity>,
    pub draw_paths: bool,
    pub draw_states: bool,
}
