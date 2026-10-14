//! re3 reference: references/re-GTA/src/renderer/Credits.cpp
//! re3 reference: references/re-GTA/src/renderer/Credits.h
//!
//! `CCredits::Init` state.

use bevy::prelude::*;

#[derive(Debug, Clone, Resource)]
pub struct CCredits {
    pub credits_going: bool,
    pub credits_start_time: u32,
}

impl CCredits {
    pub fn init() -> Self {
        let mut credits = Self {
            credits_going: false,
            credits_start_time: 0,
        };
        credits.stop();
        credits
    }

    pub fn start(&mut self, now_ms: u32) {
        self.credits_going = true;
        self.credits_start_time = now_ms;
    }

    pub fn stop(&mut self) {
        self.credits_going = false;
    }

    pub fn are_credits_done(&self) -> bool {
        !self.credits_going
    }

    /// `CCredits::Render` completion gate after the caller has produced the
    /// exact final line offset from the original credit table.
    pub fn finish_if_scrolled_past_end(
        &mut self,
        line_offset: f32,
        scroll_offset: f32,
        default_screen_height: f32,
    ) {
        if line_offset + default_screen_height - scroll_offset < -10.0 {
            self.credits_going = false;
        }
    }
}
