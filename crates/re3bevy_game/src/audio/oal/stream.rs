//! re3 reference: references/re-GTA/src/audio/oal/stream.cpp
//! re3 reference: references/re-GTA/src/audio/oal/stream.h

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OalStream {
    pub track: Option<u32>,
    pub position: u32,
    pub paused: bool,
}

impl OalStream {
    pub fn start(&mut self, track: u32, position: u32) {
        self.track = Some(track);
        self.position = position;
        self.paused = false;
    }

    pub fn stop(&mut self) {
        self.track = None;
        self.position = 0;
        self.paused = false;
    }
}
