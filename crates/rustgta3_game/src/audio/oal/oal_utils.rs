//! re3 reference: references/re-GTA/src/audio/oal/oal_utils.cpp
//! re3 reference: references/re-GTA/src/audio/oal/oal_utils.h

pub fn clamp_re3_volume(volume: u32) -> u8 {
    volume.min(127) as u8
}
