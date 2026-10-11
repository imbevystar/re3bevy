//! re3 reference: references/re-GTA/src/fakerw/rpanisot.h
//!
//! Anisotropy plugin facade.

use super::rwcore::RwTexture;
use super::rwplcore::RwInt8;

pub fn RpAnisotPluginAttach() -> bool {
    true
}

pub fn RpAnisotGetMaxSupportedMaxAnisotropy() -> RwInt8 {
    16
}

pub fn RpAnisotTextureSetMaxAnisotropy(texture: &mut RwTexture, value: RwInt8) {
    texture.max_anisotropy = value.clamp(1, RpAnisotGetMaxSupportedMaxAnisotropy());
}

pub fn RpAnisotTextureGetMaxAnisotropy(texture: &RwTexture) -> RwInt8 {
    texture.max_anisotropy
}
