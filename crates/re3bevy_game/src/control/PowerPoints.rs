//! re3 reference: references/re-GTA/src/control/PowerPoints.cpp
//! re3 reference: references/re-GTA/src/control/PowerPoints.h
//!
//! This GTA III beta feature is intentionally empty in re3: Init, Update,
//! GenerateNewOne, Save and Load all have empty bodies. Do not invent runtime
//! trigger entities for these calls.

use bevy::prelude::Resource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Resource)]
pub struct CPowerPoints;

impl CPowerPoints {
    pub fn Init(&mut self) {}

    #[allow(clippy::too_many_arguments)]
    pub fn GenerateNewOne(
        &mut self,
        _x1: f32,
        _y1: f32,
        _z1: f32,
        _x2: f32,
        _y2: f32,
        _z2: f32,
        _kind: u8,
    ) {
    }

    pub fn Update(&mut self) {}

    pub fn Save(&self, _buffer: &mut Vec<u8>) {}

    pub fn Load(&mut self, _buffer: &[u8]) {}
}
