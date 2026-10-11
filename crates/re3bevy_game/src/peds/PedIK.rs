//! re3 reference: references/re-GTA/src/peds/PedIK.cpp
//! re3 reference: references/re-GTA/src/peds/PedIK.h
//!
//! Ped look/aim IK state.

use bevy::prelude::{Component, Entity};
use re3bevy_core::core::Coordinates::CVector;

#[derive(Debug, Copy, Clone, PartialEq, Component)]
pub struct CPedIK {
    pub look_at: Option<Entity>,
    pub look_at_position: Option<CVector>,
    pub torso_yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
}

impl Default for CPedIK {
    fn default() -> Self {
        Self {
            look_at: None,
            look_at_position: None,
            torso_yaw: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
        }
    }
}

impl CPedIK {
    #[allow(non_snake_case)]
    pub fn PointGunAtPosition(&mut self, position: CVector) {
        self.look_at = None;
        self.look_at_position = Some(position);
    }

    #[allow(non_snake_case)]
    pub fn RestoreGunPosn(&mut self) {
        self.look_at_position = None;
    }
}
