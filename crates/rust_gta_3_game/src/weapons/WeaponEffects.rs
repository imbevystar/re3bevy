//! re3 reference: references/re-GTA/src/weapons/WeaponEffects.cpp
//! re3 reference: references/re-GTA/src/weapons/WeaponEffects.h
//!
//! `CWeaponEffects::Init` state.

use bevy::prelude::*;
use rust_gta_3_core::core::Coordinates::CVector;

#[derive(Debug, Clone, Resource)]
pub struct CWeaponEffects {
    pub crosshair_active: bool,
    pub crosshair_pos: CVector,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
    pub size: f32,
    pub rotation: f32,
}

impl CWeaponEffects {
    pub fn init() -> Self {
        Self {
            crosshair_active: false,
            crosshair_pos: CVector::ZERO,
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
            size: 1.0,
            rotation: 0.0,
        }
    }

    pub fn mark_target(
        &mut self,
        position: CVector,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
        size: f32,
    ) {
        self.crosshair_active = true;
        self.crosshair_pos = position;
        self.red = red;
        self.green = green;
        self.blue = blue;
        self.alpha = alpha;
        self.size = size;
    }

    pub fn clear_crosshair(&mut self) {
        self.crosshair_active = false;
    }

    /// Render intent for the additive `crosshair` raster in particle.txd.
    /// re3 ignores `m_nAlpha` at draw time and always submits alpha 255.
    pub fn render_intent(&self) -> Option<WeaponCrosshairRenderIntent> {
        self.crosshair_active
            .then_some(WeaponCrosshairRenderIntent {
                position: self.crosshair_pos,
                colour: [self.red, self.green, self.blue, 255],
                size: self.size,
                z_write: false,
                additive_blend: true,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponCrosshairRenderIntent {
    pub position: CVector,
    pub colour: [u8; 4],
    pub size: f32,
    pub z_write: bool,
    pub additive_blend: bool,
}
