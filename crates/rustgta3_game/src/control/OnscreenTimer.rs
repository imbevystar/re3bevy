//! re3 reference: references/re-GTA/src/control/OnscreenTimer.cpp
//! re3 reference: references/re-GTA/src/control/OnscreenTimer.h
//!
//! Script-visible onscreen timer/counter state and `CUserDisplay` processing.

use bevy::prelude::*;
use rustgta3_core::core::{Schedules::GtaProcessSet, Timer::CTimer};

use crate::{
    audio::{DMAudio::cDMAudio, soundlist::SOUND_CLOCK_TICK},
    control::{
        Replay::{CReplay, ReplayMode},
        Script::CTheScripts,
    },
    renderer::Hud::CHud,
};

pub const NUM_ONSCREEN_TIMER_ENTRIES: usize = 1;
pub const COUNTER_DISPLAY_NUMBER: u16 = 0;
pub const COUNTER_DISPLAY_BAR: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct COnscreenTimerEntry {
    pub timer_offset: u32,
    pub counter_offset: u32,
    pub timer_text: String,
    pub counter_text: String,
    pub counter_type: u16,
    pub timer_buffer: String,
    pub counter_buffer: String,
    pub timer_processed: bool,
    pub counter_processed: bool,
}

impl Default for COnscreenTimerEntry {
    fn default() -> Self {
        Self {
            timer_offset: 0,
            counter_offset: 0,
            timer_text: String::new(),
            counter_text: String::new(),
            counter_type: COUNTER_DISPLAY_NUMBER,
            timer_buffer: String::new(),
            counter_buffer: String::new(),
            timer_processed: false,
            counter_processed: false,
        }
    }
}

impl COnscreenTimerEntry {
    fn clear_clock(&mut self) {
        self.timer_offset = 0;
        self.timer_text.clear();
        self.timer_processed = false;
    }

    fn clear_counter(&mut self) {
        self.counter_offset = 0;
        self.counter_text.clear();
        self.counter_type = COUNTER_DISPLAY_NUMBER;
        self.counter_processed = false;
    }

    fn process_for_display(&mut self, scripts: &CTheScripts) -> bool {
        self.timer_processed = false;
        self.counter_processed = false;

        if self.timer_offset == 0 && self.counter_offset == 0 {
            return false;
        }

        if self.timer_offset != 0
            && let Ok(time) = scripts.read_i32_at_script_space(self.timer_offset as usize)
        {
            self.timer_processed = true;
            let time = time.max(0) as u32;
            self.timer_buffer = format!("{:02}:{:02}", time / 1000 / 60, time / 1000 % 60);
        }

        if self.counter_offset != 0
            && let Ok(counter) = scripts.read_i32_at_script_space(self.counter_offset as usize)
        {
            self.counter_processed = true;
            self.counter_buffer = counter.to_string();
        }

        self.timer_processed || self.counter_processed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct COnscreenTimer {
    pub entries: [COnscreenTimerEntry; NUM_ONSCREEN_TIMER_ENTRIES],
    pub processed: bool,
    pub disabled: bool,
}

impl Default for COnscreenTimer {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| COnscreenTimerEntry::default()),
            processed: false,
            disabled: false,
        }
    }
}

impl COnscreenTimer {
    #[allow(non_snake_case)]
    pub fn Init(&mut self) {
        *self = Self::default();
    }

    #[allow(non_snake_case)]
    pub fn AddClock(&mut self, offset: u32, text: impl Into<String>) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.timer_offset == 0)
        {
            entry.timer_offset = offset;
            entry.timer_text = timer_text_key(text.into());
        }
    }

    #[allow(non_snake_case)]
    pub fn AddCounter(&mut self, offset: u32, counter_type: u16, text: impl Into<String>) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.counter_offset == 0)
        {
            entry.counter_offset = offset;
            entry.counter_text = timer_text_key(text.into());
            entry.counter_type = counter_type;
        }
    }

    #[allow(non_snake_case)]
    pub fn ClearClock(&mut self, offset: u32) {
        for entry in &mut self.entries {
            if entry.timer_offset == offset {
                entry.clear_clock();
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn ClearCounter(&mut self, offset: u32) {
        for entry in &mut self.entries {
            if entry.counter_offset == offset {
                entry.clear_counter();
            }
        }
    }

    fn process(&mut self, scripts: &mut CTheScripts, time_step_ms: u32) -> Vec<u32> {
        let mut tick_seconds = Vec::with_capacity(NUM_ONSCREEN_TIMER_ENTRIES);
        let step = time_step_ms.min(i32::MAX as u32) as i32;

        for entry in &mut self.entries {
            if entry.timer_offset == 0 {
                continue;
            }
            let offset = entry.timer_offset as usize;
            let Ok(old_time) = scripts.read_i32_at_script_space(offset) else {
                continue;
            };
            let new_time = old_time.saturating_sub(step);
            if new_time < 0 {
                let _ = scripts.write_i32_at_script_space(offset, 0);
                entry.clear_clock();
                continue;
            }

            let _ = scripts.write_i32_at_script_space(offset, new_time);
            let old_seconds = old_time / 1000;
            let new_seconds = new_time / 1000;
            if old_seconds < 12 && old_seconds != new_seconds {
                tick_seconds.push(new_seconds.max(0) as u32);
            }
        }

        tick_seconds
    }

    fn process_for_display(&mut self, scripts: &CTheScripts, wants_to_draw_hud: bool) {
        if !wants_to_draw_hud {
            return;
        }
        self.processed = false;
        for entry in &mut self.entries {
            if entry.process_for_display(scripts) {
                self.processed = true;
            }
        }
    }
}

fn timer_text_key(text: String) -> String {
    text.bytes().take(10).map(char::from).collect()
}

pub struct GtaOnscreenTimerPlugin;

impl Plugin for GtaOnscreenTimerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            process_onscreen_timer.in_set(GtaProcessSet::UserDisplay),
        )
        .add_systems(
            Update,
            update_onscreen_timer_hud_state
                .in_set(GtaProcessSet::RenderPrep)
                .before(crate::renderer::HudBevy::flush_hud),
        )
        .add_systems(
            Update,
            prepare_onscreen_timer_display
                .in_set(GtaProcessSet::RenderPrep)
                .after(crate::renderer::HudBevy::flush_hud),
        );
    }
}

fn process_onscreen_timer(
    mut onscreen_timer: ResMut<COnscreenTimer>,
    mut scripts: ResMut<CTheScripts>,
    timer: Res<CTimer>,
    replay: Option<Res<CReplay>>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
) {
    if onscreen_timer.disabled
        || replay
            .as_ref()
            .is_some_and(|replay| replay.mode == ReplayMode::Playback)
    {
        return;
    }

    for seconds in onscreen_timer.process(&mut scripts, timer.time_step_in_milliseconds()) {
        if let Some(audio) = dmaudio.as_deref_mut() {
            audio.PlayFrontEndSound(SOUND_CLOCK_TICK as u16, seconds);
        }
    }
}

fn update_onscreen_timer_hud_state(onscreen_timer: Res<COnscreenTimer>, mut hud: ResMut<CHud>) {
    if !hud.wants_to_draw_hud {
        return;
    }
    let entry = &onscreen_timer.entries[0];
    if !entry.timer_processed {
        hud.timer_on_last_frame = false;
    }
    if !entry.counter_processed {
        hud.counter_on_last_frame = false;
    }
    if !onscreen_timer.processed {
        return;
    }

    if entry.timer_processed {
        if !hud.timer_on_last_frame {
            hud.timer_flash_timer = 1;
        }
        hud.timer_on_last_frame = true;
        if hud.timer_flash_timer != 0 {
            hud.timer_flash_timer += 1;
            if hud.timer_flash_timer > 50 {
                hud.timer_flash_timer = 0;
            }
        }
    }

    if entry.counter_processed {
        if !hud.counter_on_last_frame {
            hud.counter_flash_timer = 1;
        }
        hud.counter_on_last_frame = true;
        if hud.counter_flash_timer != 0 {
            hud.counter_flash_timer += 1;
            if hud.counter_flash_timer > 50 {
                hud.counter_flash_timer = 0;
            }
        }
    }
}

fn prepare_onscreen_timer_display(
    mut onscreen_timer: ResMut<COnscreenTimer>,
    scripts: Res<CTheScripts>,
    hud: Res<CHud>,
) {
    onscreen_timer.process_for_display(&scripts, hud.wants_to_draw_hud);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_and_counter_share_the_single_re3_entry() {
        let mut timer = COnscreenTimer::default();
        timer.AddClock(4, "TIME");
        timer.AddCounter(8, COUNTER_DISPLAY_BAR, "COUNT");

        let entry = &timer.entries[0];
        assert_eq!(entry.timer_offset, 4);
        assert_eq!(entry.counter_offset, 8);
        assert_eq!(entry.timer_text, "TIME");
        assert_eq!(entry.counter_text, "COUNT");
        assert_eq!(entry.counter_type, COUNTER_DISPLAY_BAR);

        timer.AddClock(12, "OTHER");
        timer.AddCounter(16, COUNTER_DISPLAY_NUMBER, "OTHER");
        assert_eq!(timer.entries[0].timer_offset, 4);
        assert_eq!(timer.entries[0].counter_offset, 8);
    }

    #[test]
    fn clearing_one_half_preserves_the_other_half_of_the_entry() {
        let mut timer = COnscreenTimer::default();
        timer.AddClock(4, "TIME");
        timer.AddCounter(8, COUNTER_DISPLAY_BAR, "COUNT");
        timer.entries[0].timer_processed = true;
        timer.entries[0].counter_processed = true;

        timer.ClearClock(4);
        assert_eq!(timer.entries[0].timer_offset, 0);
        assert_eq!(timer.entries[0].counter_offset, 8);
        assert!(!timer.entries[0].timer_processed);
        assert!(timer.entries[0].counter_processed);

        timer.ClearCounter(8);
        assert_eq!(timer.entries[0].counter_offset, 0);
        assert_eq!(timer.entries[0].counter_type, COUNTER_DISPLAY_NUMBER);
        assert!(!timer.entries[0].counter_processed);
    }

    #[test]
    fn text_keys_follow_the_fixed_ten_byte_re3_buffer() {
        let mut timer = COnscreenTimer::default();
        timer.AddClock(4, "ABCDEFGHIJKL");
        assert_eq!(timer.entries[0].timer_text, "ABCDEFGHIJ");
    }
}
