//! re3 reference: references/re-GTA/src/core/main.cpp
//! re3 reference: references/re-GTA/src/core/main.h
//!
//! re3 main-loop state mirrored as Bevy resources.

use bevy::prelude::Resource;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Resource)]
pub struct Re3MainState {
    pub quit_requested: bool,
    pub frame_counter: u32,
    pub initialised: bool,
}

#[allow(non_snake_case)]
pub fn InitialiseGame(state: &mut Re3MainState) {
    state.initialised = true;
}

#[allow(non_snake_case)]
pub fn DoRWStuffStartOfFrame(state: &mut Re3MainState) {
    state.frame_counter = state.frame_counter.saturating_add(1);
}
