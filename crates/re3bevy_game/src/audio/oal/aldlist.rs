//! re3 reference: references/re-GTA/src/audio/oal/aldlist.cpp
//! re3 reference: references/re-GTA/src/audio/oal/aldlist.h

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OalDeviceList {
    pub devices: Vec<String>,
    pub default_device: Option<String>,
}
