//! re3 reference: references/re-GTA/src/core/Timer.cpp
//! re3 reference: references/re-GTA/src/core/Timer.h
//!
//! GTA-owned timer state. Bevy may call this, but Bevy time does not define
//! gameplay time.

use std::time::Instant;

use bevy::prelude::*;

use super::Schedules::GtaFrameSet;

#[derive(Debug, Clone, Resource)]
pub struct CTimer {
    time_in_milliseconds: u32,
    time_in_milliseconds_pause_mode: u32,
    time_in_milliseconds_non_clipped: u32,
    previous_time_in_milliseconds: u32,
    frame_counter: u32,
    time_scale: f32,
    time_step: f32,
    time_step_non_clipped: f32,
    user_pause: bool,
    code_pause: bool,
    cycles_per_millisecond: u32,
    old_time: Instant,
    suspend_time: Option<Instant>,
    suspend_depth: u32,
    frame_time_fraction: f64,
    frame_time_fraction_scaled: f64,
    logical_frame_time: f64,
    logical_frame_counter: u32,
    logical_frames_passed: u32,
}

impl Default for CTimer {
    fn default() -> Self {
        Self::initialise_at(Instant::now())
    }
}

impl CTimer {
    pub const DEFAULT_TIME_STEP: f32 = 50.0 / 30.0;
    pub const MAX_TIME_STEP: f32 = 3.0;
    pub const MAX_GAME_TIME_ADVANCE_MS: u32 = 60;

    pub fn initialise() -> Self {
        Self::initialise_at(Instant::now())
    }

    pub fn initialise_at(now: Instant) -> Self {
        Self {
            time_in_milliseconds: 0,
            time_in_milliseconds_pause_mode: 1,
            time_in_milliseconds_non_clipped: 0,
            previous_time_in_milliseconds: 0,
            frame_counter: 0,
            time_scale: 1.0,
            time_step: 1.0,
            time_step_non_clipped: 1.0,
            user_pause: false,
            code_pause: false,
            cycles_per_millisecond: 1,
            old_time: now,
            suspend_time: None,
            suspend_depth: 0,
            frame_time_fraction: 0.0,
            frame_time_fraction_scaled: 0.0,
            logical_frame_time: 0.0,
            logical_frame_counter: 0,
            logical_frames_passed: 0,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.old_time).as_secs_f64() * 1000.0;
        self.old_time = now;
        self.update_from_elapsed_milliseconds(elapsed_ms, false, false);
    }

    pub fn update_from_elapsed_milliseconds(
        &mut self,
        elapsed_ms: f64,
        record_playback: bool,
        chase_recording: bool,
    ) {
        self.previous_time_in_milliseconds = self.time_in_milliseconds;

        let scaled_frame_time = if self.get_is_paused() {
            elapsed_ms
        } else {
            elapsed_ms * f64::from(self.time_scale)
        };

        self.logical_frames_passed = 0;
        self.logical_frame_time += elapsed_ms;
        while self.logical_frame_time >= 1000.0 / 30.0 {
            self.logical_frame_time -= 1000.0 / 30.0;
            self.logical_frames_passed += 1;
        }
        self.logical_frame_counter = self
            .logical_frame_counter
            .wrapping_add(self.logical_frames_passed);

        self.frame_time_fraction += elapsed_ms;
        self.frame_time_fraction_scaled += scaled_frame_time;

        let pause_advance = self.frame_time_fraction as u32;
        self.time_in_milliseconds_pause_mode = self
            .time_in_milliseconds_pause_mode
            .wrapping_add(pause_advance);

        if self.get_is_paused() {
            self.time_step = 0.0;
        } else {
            let game_advance = self.frame_time_fraction_scaled as u32;
            self.time_in_milliseconds = self.time_in_milliseconds.wrapping_add(game_advance);
            self.time_in_milliseconds_non_clipped = self
                .time_in_milliseconds_non_clipped
                .wrapping_add(game_advance);
            self.time_step = (scaled_frame_time / 1000.0 * 50.0) as f32;
        }

        self.frame_time_fraction -= f64::from(pause_advance);
        self.frame_time_fraction_scaled -= self.frame_time_fraction_scaled as u32 as f64;

        if self.time_step < 0.01 && !self.get_is_paused() {
            self.time_step = 0.01;
        }

        self.time_step_non_clipped = self.time_step;

        if !record_playback {
            self.time_step = self.time_step.min(Self::MAX_TIME_STEP);

            if self
                .time_in_milliseconds
                .wrapping_sub(self.previous_time_in_milliseconds)
                > Self::MAX_GAME_TIME_ADVANCE_MS
            {
                self.time_in_milliseconds = self
                    .previous_time_in_milliseconds
                    .wrapping_add(Self::MAX_GAME_TIME_ADVANCE_MS);
            }
        }

        if chase_recording {
            self.time_step = 1.0;
            self.time_in_milliseconds = self.previous_time_in_milliseconds.wrapping_add(16);
        }

        self.frame_counter = self.frame_counter.wrapping_add(1);
    }

    pub fn suspend(&mut self) {
        self.suspend_depth = self.suspend_depth.wrapping_add(1);
        if self.suspend_depth == 1 {
            self.suspend_time = Some(Instant::now());
        }
    }

    pub fn resume(&mut self) {
        if self.suspend_depth == 0 {
            return;
        }
        self.suspend_depth -= 1;
        if self.suspend_depth == 0 {
            if let Some(suspend_time) = self.suspend_time.take() {
                self.old_time = self.old_time + Instant::now().duration_since(suspend_time);
            }
        }
    }

    pub fn stop(&mut self) {
        self.previous_time_in_milliseconds = self.time_in_milliseconds;
    }

    pub fn start_user_pause(&mut self) {
        self.user_pause = true;
    }

    pub fn end_user_pause(&mut self) {
        self.user_pause = false;
    }

    pub fn get_is_paused(&self) -> bool {
        self.user_pause || self.code_pause
    }

    pub fn get_is_user_paused(&self) -> bool {
        self.user_pause
    }

    pub fn get_is_code_paused(&self) -> bool {
        self.code_pause
    }

    pub fn set_code_pause(&mut self, pause: bool) {
        self.code_pause = pause;
    }

    pub fn get_is_slow_motion_active(&self) -> bool {
        self.time_scale < 1.0
    }

    pub fn get_time_step_fix(&self) -> f32 {
        self.time_step / Self::DEFAULT_TIME_STEP
    }

    pub fn get_cycles_per_millisecond(&self) -> u32 {
        self.cycles_per_millisecond
    }

    pub fn get_cycles_per_frame(&self) -> u32 {
        20
    }

    pub fn time_in_milliseconds(&self) -> u32 {
        self.time_in_milliseconds
    }

    pub fn set_time_in_milliseconds(&mut self, value: u32) {
        self.time_in_milliseconds = value;
    }

    pub fn time_in_milliseconds_pause_mode(&self) -> u32 {
        self.time_in_milliseconds_pause_mode
    }

    pub fn time_in_milliseconds_non_clipped(&self) -> u32 {
        self.time_in_milliseconds_non_clipped
    }

    pub fn previous_time_in_milliseconds(&self) -> u32 {
        self.previous_time_in_milliseconds
    }

    pub fn frame_counter(&self) -> u32 {
        self.frame_counter
    }

    pub fn set_frame_counter(&mut self, value: u32) {
        self.frame_counter = value;
    }

    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    pub fn set_time_scale(&mut self, value: f32) {
        self.time_scale = value;
    }

    pub fn time_step(&self) -> f32 {
        self.time_step
    }

    pub fn set_time_step(&mut self, value: f32) {
        self.time_step = value;
    }

    pub fn time_step_in_seconds(&self) -> f32 {
        self.time_step / 50.0
    }

    pub fn time_step_in_milliseconds(&self) -> u32 {
        (self.time_step / 50.0 * 1000.0) as u32
    }

    pub fn time_step_non_clipped(&self) -> f32 {
        self.time_step_non_clipped
    }

    pub fn set_time_step_non_clipped(&mut self, value: f32) {
        self.time_step_non_clipped = value;
    }

    pub fn time_step_non_clipped_in_seconds(&self) -> f32 {
        self.time_step_non_clipped / 50.0
    }

    pub fn time_step_non_clipped_in_milliseconds(&self) -> f32 {
        self.time_step_non_clipped / 50.0 * 1000.0
    }

    pub fn logical_frame_counter(&self) -> u32 {
        self.logical_frame_counter
    }

    pub fn logical_frames_passed(&self) -> u32 {
        self.logical_frames_passed
    }
}

pub fn update_gta_timer(mut timer: ResMut<CTimer>) {
    timer.update();
}

pub struct GtaTimerPlugin;

impl Plugin for GtaTimerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CTimer>()
            .add_systems(Update, update_gta_timer.in_set(GtaFrameSet::Timer));
    }
}
