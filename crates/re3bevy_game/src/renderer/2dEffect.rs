//! re3 reference: references/re-GTA/src/renderer/2dEffect.h
//!
//! `C2dEffect` data layout used by model info extras.

use re3bevy_core::core::Coordinates::CVector;

use crate::objects::ParticleObject::CRGBA;

pub const EFFECT_LIGHT: u8 = 0;
pub const EFFECT_PARTICLE: u8 = 1;
pub const EFFECT_ATTRACTOR: u8 = 2;

pub const LIGHT_ON: u8 = 0;
pub const LIGHT_ON_NIGHT: u8 = 1;
pub const LIGHT_FLICKER: u8 = 2;
pub const LIGHT_FLICKER_NIGHT: u8 = 3;
pub const LIGHT_FLASH1: u8 = 4;
pub const LIGHT_FLASH1_NIGHT: u8 = 5;
pub const LIGHT_FLASH2: u8 = 6;
pub const LIGHT_FLASH2_NIGHT: u8 = 7;
pub const LIGHT_FLASH3: u8 = 8;
pub const LIGHT_FLASH3_NIGHT: u8 = 9;
pub const LIGHT_RANDOM_FLICKER: u8 = 10;
pub const LIGHT_RANDOM_FLICKER_NIGHT: u8 = 11;
pub const LIGHT_SPECIAL: u8 = 12;
pub const LIGHT_BRIDGE_FLASH1: u8 = 13;
pub const LIGHT_BRIDGE_FLASH2: u8 = 14;

pub const ATTRACTORTYPE_ICECREAM: i8 = 0;
pub const ATTRACTORTYPE_STARE: i8 = 1;

pub const LIGHTFLAG_LOSCHECK: u8 = 1;
pub const LIGHTFLAG_FOG_NORMAL: u8 = 2;
pub const LIGHTFLAG_FOG_ALWAYS: u8 = 4;
pub const LIGHTFLAG_FOG: u8 = LIGHTFLAG_FOG_NORMAL | LIGHTFLAG_FOG_ALWAYS;

#[derive(Debug, Clone, PartialEq)]
pub struct C2dEffect {
    pub pos: CVector,
    pub col: CRGBA,
    pub effect_type: u8,
    pub data: C2dEffectData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum C2dEffectData {
    Light(C2dEffectLight),
    Particle(C2dEffectParticle),
    Attractor(C2dEffectAttractor),
}

#[derive(Debug, Clone, PartialEq)]
pub struct C2dEffectLight {
    pub dist: f32,
    pub range: f32,
    pub size: f32,
    pub shadow_size: f32,
    pub light_type: u8,
    pub road_reflection: u8,
    pub flare_type: u8,
    pub shadow_intensity: u8,
    pub flags: u8,
    pub corona_texture: Option<String>,
    pub shadow_texture: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct C2dEffectParticle {
    pub particle_type: i32,
    pub dir: CVector,
    pub scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct C2dEffectAttractor {
    pub dir: CVector,
    pub attractor_type: i8,
    pub probability: u8,
}

impl C2dEffect {
    pub fn shutdown(&mut self) {
        if let C2dEffectData::Light(light) = &mut self.data {
            light.corona_texture = None;
            light.shadow_texture = None;
        }
    }
}
