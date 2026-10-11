//! re3 reference: references/re-GTA/src/renderer/Draw.cpp
//! re3 reference: references/re-GTA/src/renderer/Draw.h
//!
//! `CDraw` render scalar state.

use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CDraw {
    pub near_clip_z: f32,
    pub far_clip_z: f32,
    pub fov: f32,
    pub lod_distance: f32,
    pub fade_value: u8,
    pub fade_red: u8,
    pub fade_green: u8,
    pub fade_blue: u8,
}

impl CDraw {
    pub fn initialise_for_game() -> Self {
        let mut draw = Self::default();
        draw.set_fov(120.0);
        draw.lod_distance = 500.0;
        draw
    }

    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }
}

impl Default for CDraw {
    fn default() -> Self {
        Self {
            near_clip_z: 0.0,
            far_clip_z: 0.0,
            fov: 45.0,
            lod_distance: 0.0,
            fade_value: 0,
            fade_red: 0,
            fade_green: 0,
            fade_blue: 0,
        }
    }
}
