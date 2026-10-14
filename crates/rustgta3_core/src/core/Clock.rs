//! re3 reference: references/re-GTA/src/core/Clock.cpp
//! re3 reference: references/re-GTA/src/core/Clock.h
//!
//! GTA game clock. It advances from `CTimer::GetTimeInMilliseconds()`.

use bevy::prelude::*;

use super::{Schedules::GtaProcessSet, Timer::CTimer};

#[derive(Debug, Clone, Resource)]
pub struct CClock {
    hours: u8,
    minutes: u8,
    seconds: u16,
    stored_hours: u8,
    stored_minutes: u8,
    stored_seconds: u16,
    milliseconds_per_game_minute: u32,
    last_clock_tick: u32,
    clock_has_been_stored: bool,
    pending_day_rollovers: u32,
    fast_time: bool,
}

#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct CClockInput {
    pub debug_fast_forward: bool,
}

impl Default for CClock {
    fn default() -> Self {
        Self::initialise(1000, 0)
    }
}

impl CClock {
    pub fn initialise(milliseconds_per_game_minute: u32, timer_ms: u32) -> Self {
        Self {
            hours: 12,
            minutes: 0,
            seconds: 0,
            stored_hours: 0,
            stored_minutes: 0,
            stored_seconds: 0,
            milliseconds_per_game_minute,
            last_clock_tick: timer_ms,
            clock_has_been_stored: false,
            pending_day_rollovers: 0,
            fast_time: false,
        }
    }

    pub fn days_passed(&self) -> u32 {
        self.pending_day_rollovers
    }

    pub fn update(&mut self, timer_ms: u32, debug_fast_forward: bool) {
        if debug_fast_forward {
            self.minutes = self.minutes.wrapping_add(8);
            self.last_clock_tick = timer_ms;
            if self.minutes >= 60 {
                self.hours = self.hours.wrapping_add(1);
                self.minutes = 0;
                if self.hours >= 24 {
                    self.hours = 0;
                }
            }
        } else if timer_ms.wrapping_sub(self.last_clock_tick) > self.milliseconds_per_game_minute
            || self.fast_time
        {
            self.minutes = self.minutes.wrapping_add(1);
            self.last_clock_tick = self
                .last_clock_tick
                .wrapping_add(self.milliseconds_per_game_minute);

            if self.fast_time {
                self.last_clock_tick = timer_ms;
            }

            if self.minutes >= 60 {
                self.hours = self.hours.wrapping_add(1);
                self.minutes = 0;
                if self.hours >= 24 {
                    self.pending_day_rollovers = self.pending_day_rollovers.wrapping_add(1);
                    self.hours = 0;
                }
            }
        }

        self.seconds = (60 * timer_ms.wrapping_sub(self.last_clock_tick)
            / self.milliseconds_per_game_minute) as u16;
    }

    pub fn set_game_clock(&mut self, hours: u8, minutes: u8, timer_ms: u32) {
        self.hours = hours;
        self.minutes = minutes;
        self.seconds = 0;
        self.last_clock_tick = timer_ms;
    }

    pub fn get_game_clock_minutes_until(&self, hours: u8, minutes: u8) -> i32 {
        let now = i32::from(self.hours) * 60 + i32::from(self.minutes);
        let mut then = i32::from(hours) * 60 + i32::from(minutes);
        if then < now {
            then += 24 * 60;
        }
        then - now
    }

    pub fn get_is_time_in_range(&self, start_hour: u8, end_hour: u8) -> bool {
        if start_hour > end_hour {
            self.hours >= start_hour || self.hours < end_hour
        } else {
            self.hours >= start_hour && self.hours < end_hour
        }
    }

    pub fn store_clock(&mut self) {
        self.stored_hours = self.hours;
        self.stored_minutes = self.minutes;
        self.stored_seconds = self.seconds;
        self.clock_has_been_stored = true;
    }

    pub fn restore_clock(&mut self) {
        self.hours = self.stored_hours;
        self.minutes = self.stored_minutes;
        self.seconds = self.stored_seconds;
    }

    pub fn hours(&self) -> u8 {
        self.hours
    }

    pub fn minutes(&self) -> u8 {
        self.minutes
    }

    pub fn seconds(&self) -> u16 {
        self.seconds
    }

    pub fn milliseconds_per_game_minute(&self) -> u32 {
        self.milliseconds_per_game_minute
    }

    pub fn last_clock_tick(&self) -> u32 {
        self.last_clock_tick
    }

    pub fn restore_save_state(
        &mut self,
        hours: u8,
        minutes: u8,
        milliseconds_per_game_minute: u32,
        last_clock_tick: u32,
    ) {
        self.hours = hours;
        self.minutes = minutes;
        self.seconds = 0;
        self.milliseconds_per_game_minute = milliseconds_per_game_minute.max(1);
        self.last_clock_tick = last_clock_tick;
        self.pending_day_rollovers = 0;
    }

    pub fn take_day_rollovers(&mut self) -> u32 {
        std::mem::take(&mut self.pending_day_rollovers)
    }

    pub fn clock_has_been_stored(&self) -> bool {
        self.clock_has_been_stored
    }

    pub fn fast_time(&self) -> bool {
        self.fast_time
    }

    pub fn set_fast_time(&mut self, fast_time: bool) {
        self.fast_time = fast_time;
    }
}

pub fn update_gta_clock(timer: Res<CTimer>, input: Res<CClockInput>, mut clock: ResMut<CClock>) {
    clock.update(timer.time_in_milliseconds(), input.debug_fast_forward);
}

pub struct GtaClockPlugin;

impl Plugin for GtaClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CClock>()
            .init_resource::<CClockInput>()
            .add_systems(Update, update_gta_clock.in_set(GtaProcessSet::Clock));
    }
}
