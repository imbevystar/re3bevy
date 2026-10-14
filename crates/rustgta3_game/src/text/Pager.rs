//! re3 reference: references/re-GTA/src/text/Pager.cpp
//! re3 reference: references/re-GTA/src/text/Pager.h
//!
//! `CPager` scrolling message queue.

use std::array;

use bevy::prelude::*;
use rustgta3_core::core::{Schedules::GtaProcessSet, Timer::CTimer};

use crate::{
    renderer::Hud::CHud,
    text::{Messages::CMessages, Text::GtaWideString},
};

pub const NUMPAGERMESSAGES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagerMessage {
    pub text: Option<GtaWideString>,
    pub speed_ms: u16,
    pub current_position: i16,
    pub string_length: u16,
    pub priority: u16,
    pub time_to_change_position: u32,
    pub unused: i16,
    pub numbers: [i32; 6],
}

impl Default for PagerMessage {
    fn default() -> Self {
        Self {
            text: None,
            speed_ms: 0,
            current_position: 0,
            string_length: 0,
            priority: 0,
            time_to_change_position: 0,
            unused: 0,
            numbers: [-1; 6],
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CPager {
    pub num_display_letters: i16,
    pub messages: [PagerMessage; NUMPAGERMESSAGES],
}

impl CPager {
    pub fn init() -> Self {
        let mut pager = Self {
            num_display_letters: 8,
            messages: array::from_fn(|_| PagerMessage::default()),
        };
        pager.clear_messages();
        pager
    }

    pub fn process(
        &mut self,
        now_ms: u32,
        hud: &mut CHud,
        messages: &mut CMessages,
        wide_screen_on: bool,
    ) {
        if self.messages[0].text.is_some()
            && self.messages[0].current_position >= self.messages[0].string_length as i16
        {
            self.messages[0].text = None;
            let mut i = 0;
            while i < NUMPAGERMESSAGES - 1 && self.messages[i + 1].text.is_some() {
                self.messages[i] = self.messages[i + 1].clone();
                i += 1;
            }
            self.messages[i] = PagerMessage::default();
            if let Some(text) = self.messages[0].text.clone() {
                messages.add_to_previous_brief_array(text, self.messages[0].numbers, None);
            }
        }

        self.display(hud);
        if self.messages[0].text.is_some() {
            if wide_screen_on
                || !hud.wants_to_draw_hud
                || !hud.big_message[0].is_empty()
                || !hud.big_message[2].is_empty()
            {
                self.restart_current_message(now_ms);
            } else if now_ms > self.messages[0].time_to_change_position {
                self.messages[0].current_position += 1;
                self.messages[0].time_to_change_position =
                    now_ms.saturating_add(self.messages[0].speed_ms.into());
            }
        }
    }

    pub fn display(&self, hud: &mut CHud) {
        let mut out = Vec::new();
        if let Some(text) = self.messages[0].text.as_deref() {
            let expanded = CMessages::insert_number_in_string(Some(text), self.messages[0].numbers);
            for i in 0..self.num_display_letters {
                let pos = self.messages[0].current_position + i;
                if pos >= 0 {
                    let pos = pos as usize;
                    if pos >= expanded.len() {
                        break;
                    }
                    out.push(expanded[pos]);
                } else {
                    out.push(b' ' as u16);
                }
            }
        }
        hud.set_pager_message(&out);
    }

    pub fn add_message(
        &mut self,
        str: GtaWideString,
        speed: u16,
        priority: u16,
        unused: u16,
        now_ms: u32,
    ) {
        self.add_message_internal(str, [-1; 6], speed, priority, unused, now_ms, None);
    }

    pub fn add_message_with_number(
        &mut self,
        str: GtaWideString,
        numbers: [i32; 6],
        speed: u16,
        priority: u16,
        unused: u16,
        now_ms: u32,
    ) {
        let expanded = CMessages::insert_number_in_string(Some(&str), numbers);
        self.add_message_internal(
            str,
            numbers,
            speed,
            priority,
            unused,
            now_ms,
            Some(expanded.len()),
        );
    }

    pub fn clear_messages(&mut self) {
        for msg in &mut self.messages {
            *msg = PagerMessage::default();
        }
    }

    pub fn restart_current_message(&mut self, now_ms: u32) {
        if self.messages[0].text.is_some() {
            self.messages[0].current_position = -(self.num_display_letters + 10);
            self.messages[0].time_to_change_position =
                now_ms.saturating_add(self.messages[0].speed_ms.into());
        }
    }

    fn add_message_internal(
        &mut self,
        str: GtaWideString,
        numbers: [i32; 6],
        speed: u16,
        priority: u16,
        unused: u16,
        now_ms: u32,
        expanded_len: Option<usize>,
    ) {
        let size = expanded_len.unwrap_or(str.len()) as u16;
        for i in 0..NUMPAGERMESSAGES {
            if self.messages[i].text.is_some() {
                if self.messages[i].priority >= priority {
                    continue;
                }
                for j in (i + 1..NUMPAGERMESSAGES).rev() {
                    self.messages[j] = self.messages[j - 1].clone();
                }
            }

            self.messages[i] = PagerMessage {
                text: Some(str),
                speed_ms: speed,
                priority,
                unused: unused as i16,
                current_position: -(self.num_display_letters + 10),
                time_to_change_position: now_ms.saturating_add(speed.into()),
                string_length: size,
                numbers,
            };
            return;
        }
    }
}

pub struct GtaPagerPlugin;

impl Plugin for GtaPagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_pager.in_set(GtaProcessSet::UserDisplay));
    }
}

fn process_pager(
    pager: Option<ResMut<CPager>>,
    hud: Option<ResMut<CHud>>,
    messages: Option<ResMut<CMessages>>,
    timer: Option<Res<CTimer>>,
) {
    let (Some(mut pager), Some(mut hud), Some(mut messages), Some(timer)) =
        (pager, hud, messages, timer)
    else {
        return;
    };
    pager.process(timer.time_in_milliseconds(), &mut hud, &mut messages, false);
}
