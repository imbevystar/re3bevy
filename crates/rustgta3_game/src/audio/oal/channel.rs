//! re3 reference: references/re-GTA/src/audio/oal/channel.cpp
//! re3 reference: references/re-GTA/src/audio/oal/channel.h

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OalChannel {
    pub sample: Option<u32>,
    pub frequency: u32,
    pub volume: u8,
    pub pan: u8,
    pub looping: bool,
}

impl OalChannel {
    pub fn stop(&mut self) {
        self.sample = None;
        self.looping = false;
    }
}
