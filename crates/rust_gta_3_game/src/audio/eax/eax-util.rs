//! re3 reference: references/re-GTA/src/audio/eax/eax-util.cpp
//! re3 reference: references/re-GTA/src/audio/eax/eax-util.h

use super::eax::EaxState;

pub fn set_enabled(state: &mut EaxState, enabled: bool) {
    state.enabled = enabled;
}
