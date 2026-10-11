//! re3 reference: references/re-GTA/src/fakerw/fake.cpp
//!
//! FakeRW plugin attachment facade.

use crate::fakerw::{rpanisot, rphanim, rpmatfx, rpskin, rpworld};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct FakeRwPluginState {
    pub world: bool,
    pub matfx: bool,
    pub anisot: bool,
    pub hanim: bool,
    pub skin: bool,
}

#[allow(non_snake_case)]
pub fn AttachPlugins() -> FakeRwPluginState {
    FakeRwPluginState {
        world: rpworld::RpWorldPluginAttach(),
        matfx: rpmatfx::RpMatFXPluginAttach(),
        anisot: rpanisot::RpAnisotPluginAttach(),
        hanim: rphanim::RpHAnimPluginAttach(),
        skin: rpskin::RpSkinPluginAttach(),
    }
}
