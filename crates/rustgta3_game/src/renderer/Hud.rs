//! re3 reference: references/re-GTA/src/renderer/Hud.cpp
//! re3 reference: references/re-GTA/src/renderer/Hud.h
//!
//! `CHud` message and frontend-facing HUD state. Drawing is represented as
//! state until the renderer phase owns the final 2D draw backend.

use std::array;

use bevy::prelude::*;

use crate::text::Text::GtaWideString;

pub const HELP_MSG_LENGTH: usize = 256;
pub const NUM_HUD_SPRITES: usize = 23;
pub const NUM_BIG_MESSAGES: usize = 6;
pub const ITEM_NONE: i16 = -1;
pub const ITEM_ARMOUR: i16 = 3;
pub const ITEM_HEALTH: i16 = 4;
pub const ITEM_RADAR: i16 = 8;

#[derive(Debug, Clone, Resource)]
pub struct CHud {
    pub item_to_flash: i16,
    pub zone_name: Option<GtaWideString>,
    pub last_zone_name: Option<GtaWideString>,
    pub zone_to_print: Option<GtaWideString>,
    pub message: GtaWideString,
    pub big_message: [GtaWideString; NUM_BIG_MESSAGES],
    pub last_big_message: [GtaWideString; NUM_BIG_MESSAGES],
    pub pager_message: GtaWideString,
    pub zone_name_timer: u32,
    pub zone_fade_timer: i32,
    pub zone_state: u32,
    pub help_message: GtaWideString,
    pub last_help_message: GtaWideString,
    pub help_message_to_print: GtaWideString,
    pub help_message_timer: u32,
    pub help_message_fade_timer: i32,
    pub help_message_state: u32,
    pub help_message_quick: bool,
    pub help_message_display_time: f32,
    pub sprite_brightness: i32,
    pub wants_to_draw_hud: bool,
    pub wants_to_draw_3d_markers: bool,
    pub vehicle_name: Option<GtaWideString>,
    pub last_vehicle_name: Option<GtaWideString>,
    pub vehicle_name_to_print: Option<GtaWideString>,
    pub vehicle_name_timer: u32,
    pub vehicle_fade_timer: i32,
    pub vehicle_state: u32,
    pub big_message_in_use: [f32; NUM_BIG_MESSAGES],
    pub big_message_x: [f32; NUM_BIG_MESSAGES],
    pub big_message_alpha: [f32; NUM_BIG_MESSAGES],
    pub pager_on: i16,
    pub pager_timer: i16,
    pub pager_x_offset: f32,
    pub pager_sound_played: i16,
    pub odd_job2_on: i16,
    pub odd_job2_timer: u16,
    pub odd_job2_x_offset: f32,
    pub odd_job2_off_timer: f32,
    pub counter_on_last_frame: bool,
    pub counter_flash_timer: u16,
    pub timer_on_last_frame: bool,
    pub timer_flash_timer: u16,
}

impl CHud {
    pub fn initialise() -> Self {
        let mut hud = Self {
            item_to_flash: ITEM_NONE,
            zone_name: None,
            last_zone_name: None,
            zone_to_print: None,
            message: Vec::new(),
            big_message: array::from_fn(|_| Vec::new()),
            last_big_message: array::from_fn(|_| Vec::new()),
            pager_message: Vec::new(),
            zone_name_timer: 0,
            zone_fade_timer: 0,
            zone_state: 0,
            help_message: Vec::new(),
            last_help_message: Vec::new(),
            help_message_to_print: Vec::new(),
            help_message_timer: 0,
            help_message_fade_timer: 0,
            help_message_state: 0,
            help_message_quick: false,
            help_message_display_time: 1.0,
            sprite_brightness: 0,
            wants_to_draw_hud: true,
            wants_to_draw_3d_markers: true,
            vehicle_name: None,
            last_vehicle_name: None,
            vehicle_name_to_print: None,
            vehicle_name_timer: 0,
            vehicle_fade_timer: 0,
            vehicle_state: 0,
            big_message_in_use: [0.0; NUM_BIG_MESSAGES],
            big_message_x: [0.0; NUM_BIG_MESSAGES],
            big_message_alpha: [0.0; NUM_BIG_MESSAGES],
            pager_on: 0,
            pager_timer: 0,
            pager_x_offset: 150.0,
            pager_sound_played: 0,
            odd_job2_on: 0,
            odd_job2_timer: 0,
            odd_job2_x_offset: 0.0,
            odd_job2_off_timer: 0.0,
            counter_on_last_frame: false,
            counter_flash_timer: 0,
            timer_on_last_frame: false,
            timer_flash_timer: 0,
        };
        hud.get_rid_of_all_hud_messages();
        hud
    }

    pub fn re_initialise(&mut self) {
        self.wants_to_draw_hud = true;
        self.wants_to_draw_3d_markers = true;
        self.get_rid_of_all_hud_messages();
        self.counter_on_last_frame = false;
        self.item_to_flash = ITEM_NONE;
        self.odd_job2_timer = 0;
        self.odd_job2_off_timer = 0.0;
        self.odd_job2_on = 0;
        self.odd_job2_x_offset = 0.0;
        self.counter_flash_timer = 0;
        self.timer_on_last_frame = false;
        self.timer_flash_timer = 0;
        self.sprite_brightness = 0;
        self.pager_on = 0;
        self.pager_timer = 0;
        self.pager_sound_played = 0;
        self.pager_x_offset = 150.0;
    }

    pub fn get_rid_of_all_hud_messages(&mut self) {
        self.zone_state = 0;
        self.last_zone_name = None;
        self.zone_name_timer = 0;
        self.zone_name = None;
        self.help_message.clear();
        self.last_help_message.clear();
        self.help_message_to_print.clear();
        self.help_message_timer = 0;
        self.help_message_fade_timer = 0;
        self.help_message_state = 0;
        self.help_message_quick = false;
        self.help_message_display_time = 1.0;
        self.vehicle_name = None;
        self.last_vehicle_name = None;
        self.vehicle_name_to_print = None;
        self.vehicle_name_timer = 0;
        self.vehicle_fade_timer = 0;
        self.vehicle_state = 0;
        self.message.clear();
        for i in 0..NUM_BIG_MESSAGES {
            self.big_message_in_use[i] = 0.0;
            self.big_message[i].clear();
        }
    }

    pub fn set_zone_name(&mut self, name: GtaWideString) {
        self.zone_name = Some(name);
    }

    pub fn set_help_message(&mut self, message: &[u16], quick: bool) {
        self.help_message = message.iter().copied().take(HELP_MSG_LENGTH - 1).collect();
        self.last_help_message.clear();
        self.help_message_state = 0;
        self.help_message_quick = quick;
    }

    pub fn set_vehicle_name(&mut self, name: GtaWideString) {
        self.vehicle_name = Some(name);
    }

    pub fn set_message(&mut self, message: &[u16]) {
        self.message = message.iter().copied().take(255).collect();
    }

    pub fn set_big_message(&mut self, message: &[u16], style: u16) {
        let style = style as usize;
        if style >= NUM_BIG_MESSAGES {
            return;
        }
        if style == 5 {
            for (i, ch) in message.iter().take(127).enumerate() {
                if self.last_big_message[5].get(i) != Some(ch) {
                    self.odd_job2_on = 0;
                    self.odd_job2_off_timer = 0.0;
                }
            }
        }
        self.big_message[style] = message.iter().copied().take(127).collect();
        self.last_big_message[style] = self.big_message[style].clone();
    }

    pub fn set_pager_message(&mut self, message: &[u16]) {
        self.pager_message = message.iter().copied().take(255).collect();
    }

    /// re3 `CHud::Draw`/`DrawAfterFade` temporal state, kept independent from
    /// Bevy's presentation tree. Values are milliseconds exactly as in Hud.cpp.
    pub fn process_draw_state(&mut self, step_ms: u32, time_step: f32) {
        let dt = step_ms as i32;
        process_named_fade(
            self.zone_name.as_ref(),
            &mut self.last_zone_name,
            &mut self.zone_to_print,
            &mut self.zone_state,
            &mut self.zone_name_timer,
            &mut self.zone_fade_timer,
            dt,
        );
        process_named_fade(
            self.vehicle_name.as_ref(),
            &mut self.last_vehicle_name,
            &mut self.vehicle_name_to_print,
            &mut self.vehicle_state,
            &mut self.vehicle_name_timer,
            &mut self.vehicle_fade_timer,
            dt,
        );

        if self.help_message != self.last_help_message {
            self.help_message_fade_timer = 0;
            self.help_message_state = 2;
            self.help_message_timer = 0;
            self.help_message_to_print = self.help_message.clone();
            self.help_message_display_time = self.help_message.len() as f32 / 20.0 + 3.0;
            self.last_help_message = self.help_message.clone();
        }
        if !self.help_message.is_empty() && self.help_message_state != 0 {
            match self.help_message_state {
                1 => {
                    self.help_message_fade_timer = 600;
                    if self.help_message_timer > (self.help_message_display_time * 1000.0) as u32
                        || self.help_message_quick && self.help_message_timer > 1500
                    {
                        self.help_message_state = 3;
                    }
                }
                2 => {
                    self.help_message_fade_timer += 2 * dt;
                    if self.help_message_fade_timer > 0 {
                        self.help_message_state = 1;
                        self.help_message_fade_timer = 0;
                    }
                }
                3 => {
                    self.help_message_fade_timer -= 2 * dt;
                    if self.help_message_fade_timer < 0 {
                        self.help_message_state = 0;
                        self.help_message_fade_timer = 0;
                    }
                }
                4 => {
                    self.help_message_fade_timer -= 2 * dt;
                    if self.help_message_fade_timer < 0 {
                        self.help_message_state = 2;
                        self.help_message_fade_timer = 0;
                        self.help_message_to_print = self.last_help_message.clone();
                    }
                }
                _ => {}
            }
            self.help_message_timer = self.help_message_timer.saturating_add(step_ms);
        }

        if self.pager_message.is_empty() && self.pager_on == 1 {
            self.pager_sound_played = 0;
            self.pager_on = 2;
        }
        if !self.pager_message.is_empty() || self.pager_on == 2 {
            if self.pager_on == 0 {
                self.pager_on = 1;
                self.pager_x_offset = 150.0;
            }
            if self.pager_on == 1 {
                let step = (self.pager_x_offset * 0.1).min(10.0);
                self.pager_x_offset = (self.pager_x_offset - step * time_step).max(0.0);
            } else {
                self.pager_x_offset += (self.pager_x_offset * 0.1).max(2.0);
                if self.pager_x_offset > 150.0 {
                    self.pager_x_offset = 150.0;
                    self.pager_on = 0;
                }
            }
        }

        for style in [0usize, 1] {
            if self.big_message[style].is_empty() {
                self.big_message_in_use[style] = 0.0;
                continue;
            }
            if self.big_message_in_use[style] == 0.0 {
                self.big_message_alpha[style] = 0.0;
                self.big_message_x[style] = -60.0;
                self.big_message_in_use[style] = 1.0;
            } else if self.big_message_x[style] < 620.0 {
                self.big_message_x[style] += step_ms as f32 * 0.3;
                self.big_message_alpha[style] =
                    (self.big_message_alpha[style] + step_ms as f32 * 0.3).min(255.0);
            } else {
                self.big_message_in_use[style] += time_step;
                if self.big_message_in_use[style] >= 120.0 {
                    self.big_message_in_use[style] = 120.0;
                    self.big_message_alpha[style] -= step_ms as f32 * 0.3;
                    if self.big_message_alpha[style] <= 0.0 {
                        self.big_message[style].clear();
                        self.big_message_alpha[style] = 0.0;
                    }
                }
            }
        }
        if !self.big_message[2].is_empty() {
            self.big_message_in_use[2] = 1.0;
            self.big_message_alpha[2] =
                (self.big_message_alpha[2] + step_ms as f32 * 0.4).min(255.0);
        } else {
            self.big_message_in_use[2] = 0.0;
            self.big_message_alpha[2] = 0.0;
        }

        // re3 CHud::DrawAfterFade odd-job result lifecycle. This is also used
        // by the SCM insane-stunt bonus messages: a result is held briefly,
        // then locked out for five seconds instead of being redrawn every
        // frame while CMessages still owns the same string.
        if self.odd_job2_off_timer > 0.0 {
            self.odd_job2_off_timer -= step_ms as f32;
        }
        if !self.big_message[5].is_empty() && self.odd_job2_off_timer <= 0.0 {
            match self.odd_job2_on {
                0 => {
                    self.odd_job2_on = 1;
                    self.odd_job2_x_offset = 380.0;
                }
                1 => {
                    if self.odd_job2_x_offset <= 2.0 {
                        self.odd_job2_timer = 0;
                        self.odd_job2_on = 2;
                    } else {
                        let step = (self.odd_job2_x_offset / 6.0).min(40.0);
                        self.odd_job2_x_offset -= step;
                    }
                }
                2 => {
                    self.odd_job2_timer = self
                        .odd_job2_timer
                        .saturating_add(step_ms.min(u16::MAX as u32) as u16);
                    if self.odd_job2_timer > 1500 {
                        self.odd_job2_on = 3;
                    }
                }
                3 => {
                    let step = (self.odd_job2_x_offset / 5.0).max(30.0);
                    self.odd_job2_x_offset -= step;
                    if self.odd_job2_x_offset < -380.0 {
                        self.odd_job2_off_timer = 5000.0;
                        self.odd_job2_on = 0;
                    }
                }
                _ => {}
            }
        }
    }
}

fn process_named_fade(
    current: Option<&GtaWideString>,
    last: &mut Option<GtaWideString>,
    printed: &mut Option<GtaWideString>,
    state: &mut u32,
    timer: &mut u32,
    fade: &mut i32,
    dt: i32,
) {
    let Some(current) = current else {
        *state = 0;
        return;
    };
    if last.as_ref() != Some(current) {
        if *state == 0 {
            *state = 2;
            *printed = Some(current.clone());
            *timer = 0;
            *fade = 0;
        } else {
            *timer = 5;
            *state = 4;
        }
        *last = Some(current.clone());
    }
    match *state {
        1 => {
            *fade = 1000;
            *timer = timer.saturating_add(dt.max(0) as u32);
            if *timer > 10_000 {
                *state = 3;
            }
        }
        2 => {
            *fade += dt;
            if *fade > 1000 {
                *fade = 1000;
                *state = 1;
            }
        }
        3 => {
            *fade -= dt;
            if *fade < 0 {
                *fade = 0;
                *state = 0;
            }
        }
        4 => {
            *fade -= dt;
            if *fade < 0 {
                *fade = 0;
                *printed = last.clone();
                *state = 2;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::CHud;

    #[test]
    fn odd_job_result_uses_re3_hold_and_lockout_cycle() {
        let mut hud = CHud::initialise();
        hud.set_big_message(
            &['B' as u16, 'O' as u16, 'N' as u16, 'U' as u16, 'S' as u16],
            5,
        );

        hud.process_draw_state(20, 1.0);
        assert_eq!(hud.odd_job2_on, 1);
        assert_eq!(hud.odd_job2_x_offset, 380.0);

        for _ in 0..200 {
            hud.process_draw_state(20, 1.0);
            if hud.odd_job2_off_timer > 0.0 {
                break;
            }
        }
        assert_eq!(hud.odd_job2_on, 0);
        assert_eq!(hud.odd_job2_off_timer, 5000.0);

        hud.process_draw_state(20, 1.0);
        assert_eq!(hud.odd_job2_off_timer, 4980.0);
        assert_eq!(hud.odd_job2_on, 0);
    }
}
