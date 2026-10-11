//! re3 reference: references/re-GTA/src/fakerw/rpmatfx.h
//!
//! MatFX material metadata facade.

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RpMatFXMaterial {
    pub flags: RpMatFXMaterialFlags,
    pub env_map_coefficient: f32,
    pub use_frame_buffer_alpha: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum RpMatFXMaterialFlags {
    #[default]
    rpMATFXEFFECTNULL = 0,
    rpMATFXEFFECTBUMPMAP = 1,
    rpMATFXEFFECTENVMAP = 2,
    rpMATFXEFFECTBUMPENVMAP = 3,
    rpMATFXEFFECTDUAL = 4,
    rpMATFXEFFECTUVTRANSFORM = 5,
    rpMATFXEFFECTDUALUVTRANSFORM = 6,
}

pub fn RpMatFXPluginAttach() -> bool {
    true
}

pub fn RpMatFXMaterialSetEffects(material: &mut RpMatFXMaterial, flags: RpMatFXMaterialFlags) {
    material.flags = flags;
}

pub fn RpMatFXMaterialGetEffects(material: &RpMatFXMaterial) -> RpMatFXMaterialFlags {
    material.flags
}

pub fn RpMatFXMaterialSetEnvMapCoefficient(material: &mut RpMatFXMaterial, coef: f32) {
    material.env_map_coefficient = coef;
}

pub fn RpMatFXMaterialGetEnvMapCoefficient(material: &RpMatFXMaterial) -> f32 {
    material.env_map_coefficient
}
