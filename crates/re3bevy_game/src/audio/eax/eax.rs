//! re3 reference: references/re-GTA/src/audio/eax/eax.cpp
//! re3 reference: references/re-GTA/src/audio/eax/eax.h

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EaxState {
    pub enabled: bool,
    pub environment: u32,
}
