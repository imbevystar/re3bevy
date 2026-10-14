//! re3 reference: references/re-GTA/src/rw/RwPS2AlphaTest.cpp
//!
//! PS2-style two-pass alpha test decisions.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ps2AlphaTestState {
    pub enabled: bool,
    pub alpha_blend_enabled: bool,
    pub z_write_enabled: bool,
    pub alpha_ref: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ps2AlphaPass {
    Normal,
    GreaterEqual128,
    Less128NoZWrite,
}

pub fn dual_pass_sequence(state: Ps2AlphaTestState) -> [Option<Ps2AlphaPass>; 2] {
    if state.enabled && state.alpha_blend_enabled && state.z_write_enabled {
        [
            Some(Ps2AlphaPass::GreaterEqual128),
            Some(Ps2AlphaPass::Less128NoZWrite),
        ]
    } else {
        [Some(Ps2AlphaPass::Normal), None]
    }
}
