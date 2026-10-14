//! re3 reference: references/re-GTA/src/text/Messages.cpp
//! re3 reference: references/re-GTA/src/text/Messages.h
//!
//! Brief, big-message and previous-brief queues.

use std::array;

use bevy::prelude::*;
use rust_gta_3_core::core::{Schedules::GtaProcessSet, Timer::CTimer};

use crate::{
    control::Script::{CTheScripts, ScriptCommandEffect},
    core::ControllerConfig::{CControllerConfigManager, e_ControllerAction},
    renderer::Hud::CHud,
    text::Text::{CText, GtaWideString, ascii_to_unicode},
};

pub const NUMBRIEFMESSAGES: usize = 8;
pub const NUMBIGMESSAGES: usize = 6;
pub const NUMPREVIOUSBRIEFS: usize = 5;
pub const BIG_MESSAGE_STACK: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct tMessage {
    pub text: Option<GtaWideString>,
    pub flag: u16,
    pub time: u32,
    pub start_time: u32,
    pub numbers: [i32; 6],
    pub string: Option<GtaWideString>,
}

impl Default for tMessage {
    fn default() -> Self {
        Self {
            text: None,
            flag: 0,
            time: 0,
            start_time: 0,
            numbers: [-1; 6],
            string: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct tBigMessage {
    pub stack: [tMessage; BIG_MESSAGE_STACK],
}

impl Default for tBigMessage {
    fn default() -> Self {
        Self {
            stack: array::from_fn(|_| tMessage::default()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct tPreviousBrief {
    pub text: Option<GtaWideString>,
    pub numbers: [i32; 6],
    pub string: Option<GtaWideString>,
}

#[derive(Debug, Clone, Resource)]
pub struct CMessages {
    pub brief_messages: [tMessage; NUMBRIEFMESSAGES],
    pub big_messages: [tBigMessage; NUMBIGMESSAGES],
    pub previous_briefs: [tPreviousBrief; NUMPREVIOUSBRIEFS],
    pub previous_mission_title: [u8; 16],
}

impl CMessages {
    pub fn init() -> Self {
        let mut messages = Self {
            brief_messages: array::from_fn(|_| tMessage::default()),
            big_messages: array::from_fn(|_| tBigMessage::default()),
            previous_briefs: array::from_fn(|_| tPreviousBrief::default()),
            previous_mission_title: [0; 16],
        };
        messages.clear_messages();
        messages
    }

    pub fn get_wide_string_length(src: &[u16]) -> u16 {
        src.iter().take_while(|ch| **ch != 0).count() as u16
    }

    pub fn wide_string_copy(dst: &mut GtaWideString, src: Option<&[u16]>, size: usize) {
        dst.clear();
        if let Some(src) = src {
            dst.extend(src.iter().take(size.saturating_sub(1)).copied());
        }
    }

    pub fn wide_string_compare(str1: &[u16], str2: &[u16], size: usize) -> bool {
        let len1 = Self::get_wide_string_length(str1) as usize;
        let len2 = Self::get_wide_string_length(str2) as usize;
        if len1 != len2 && (len1 < size || len2 < size) {
            return false;
        }
        for i in 0..size.min(str1.len()).min(str2.len()) {
            if str1[i] == 0 {
                return true;
            }
            if str1[i] != str2[i] {
                return false;
            }
        }
        true
    }

    pub fn process(&mut self, now_ms: u32) {
        for style in 0..NUMBIGMESSAGES {
            let head_expired = self.big_messages[style].stack[0].text.is_some()
                && now_ms
                    > self.big_messages[style].stack[0]
                        .start_time
                        .saturating_add(self.big_messages[style].stack[0].time);
            if head_expired {
                self.big_messages[style].stack[0].text = None;
                let mut i = 0;
                while i < BIG_MESSAGE_STACK - 1
                    && self.big_messages[style].stack[i + 1].text.is_some()
                {
                    self.big_messages[style].stack[i] =
                        self.big_messages[style].stack[i + 1].clone();
                    i += 1;
                }
                self.big_messages[style].stack[i] = tMessage::default();
                self.big_messages[style].stack[0].start_time = now_ms;
            }
        }

        let brief_expired = self.brief_messages[0].text.is_some()
            && now_ms
                > self.brief_messages[0]
                    .start_time
                    .saturating_add(self.brief_messages[0].time);
        if brief_expired {
            self.brief_messages[0].text = None;
            let mut i = 0;
            while i < NUMBRIEFMESSAGES - 1 && self.brief_messages[i + 1].text.is_some() {
                self.brief_messages[i] = self.brief_messages[i + 1].clone();
                i += 1;
            }
            self.brief_messages[i] = tMessage::default();
            self.brief_messages[0].start_time = now_ms;
            if let Some(next) = self.brief_messages[0].text.clone() {
                let numbers = self.brief_messages[0].numbers;
                let string = self.brief_messages[0].string.clone();
                self.add_to_previous_brief_array(next, numbers, string);
            }
        }
    }

    pub fn display(&self, hud: &mut CHud) {
        for style in 0..NUMBIGMESSAGES {
            let msg = &self.big_messages[style].stack[0];
            let mut out = Self::insert_number_in_string(msg.text.as_deref(), msg.numbers);
            Self::insert_string_in_string(&mut out, msg.string.as_deref());
            hud.set_big_message(&out, style as u16);
        }

        let brief = &self.brief_messages[0];
        let mut out = Self::insert_number_in_string(brief.text.as_deref(), brief.numbers);
        Self::insert_string_in_string(&mut out, brief.string.as_deref());
        hud.set_message(&out);
    }

    pub fn add_message(&mut self, msg: GtaWideString, time: u32, flag: u16, now_ms: u32) {
        let Some(i) = self
            .brief_messages
            .iter()
            .position(|slot| slot.text.is_none())
        else {
            return;
        };
        self.brief_messages[i] = message_slot(msg, time, flag, now_ms, [-1; 6], None);
        if i == 0 {
            let slot = &self.brief_messages[0];
            self.add_to_previous_brief_array(
                slot.text.clone().unwrap_or_default(),
                slot.numbers,
                slot.string.clone(),
            );
        }
    }

    pub fn add_message_jump_q(&mut self, msg: GtaWideString, time: u32, flag: u16, now_ms: u32) {
        self.brief_messages[0] = message_slot(msg.clone(), time, flag, now_ms, [-1; 6], None);
        self.add_to_previous_brief_array(msg, [-1; 6], None);
    }

    pub fn add_message_soon(&mut self, msg: GtaWideString, time: u32, flag: u16, now_ms: u32) {
        if self.brief_messages[0].text.is_some() {
            for i in (2..NUMBRIEFMESSAGES).rev() {
                self.brief_messages[i] = self.brief_messages[i - 1].clone();
            }
            self.brief_messages[1] = message_slot(msg, time, flag, now_ms, [-1; 6], None);
        } else {
            self.brief_messages[0] = message_slot(msg.clone(), time, flag, now_ms, [-1; 6], None);
            self.add_to_previous_brief_array(msg, [-1; 6], None);
        }
    }

    /// re3 `CRunningScript::Process` MESSAGE_WAIT skip path. Setting the
    /// current heads' start times to zero makes normal message processing
    /// expire them without discarding queued messages.
    pub fn expire_current_messages_for_skip(&mut self) {
        for big in &mut self.big_messages {
            if big.stack[0].text.is_some() {
                big.stack[0].start_time = 0;
            }
        }
        if self.brief_messages[0].text.is_some() {
            self.brief_messages[0].start_time = 0;
        }
    }

    pub fn clear_messages(&mut self) {
        for big in &mut self.big_messages {
            for slot in &mut big.stack {
                *slot = tMessage::default();
            }
        }
        self.clear_small_messages_only();
    }

    pub fn clear_small_messages_only(&mut self) {
        for slot in &mut self.brief_messages {
            *slot = tMessage::default();
        }
    }

    pub fn add_big_message(&mut self, msg: GtaWideString, time: u32, style: u16, now_ms: u32) {
        let style = style as usize;
        if style >= NUMBIGMESSAGES {
            return;
        }
        self.big_messages[style].stack[0] = message_slot(msg, time, 0, now_ms, [-1; 6], None);
    }

    pub fn add_big_message_q(&mut self, msg: GtaWideString, time: u32, style: u16, now_ms: u32) {
        let style = style as usize;
        if style >= NUMBIGMESSAGES {
            return;
        }
        let Some(i) = self.big_messages[style]
            .stack
            .iter()
            .position(|slot| slot.text.is_none())
        else {
            return;
        };
        self.big_messages[style].stack[i] = message_slot(msg, time, 0, now_ms, [-1; 6], None);
    }

    pub fn add_message_with_number(
        &mut self,
        msg: GtaWideString,
        time: u32,
        flag: u16,
        numbers: [i32; 6],
        now_ms: u32,
    ) {
        let Some(i) = self
            .brief_messages
            .iter()
            .position(|slot| slot.text.is_none())
        else {
            return;
        };
        self.brief_messages[i] = message_slot(msg, time, flag, now_ms, numbers, None);
        if i == 0 {
            let slot = &self.brief_messages[0];
            self.add_to_previous_brief_array(
                slot.text.clone().unwrap_or_default(),
                slot.numbers,
                slot.string.clone(),
            );
        }
    }

    pub fn add_message_jump_q_with_number(
        &mut self,
        msg: GtaWideString,
        time: u32,
        flag: u16,
        numbers: [i32; 6],
        now_ms: u32,
    ) {
        self.brief_messages[0] = message_slot(msg.clone(), time, flag, now_ms, numbers, None);
        self.add_to_previous_brief_array(msg, numbers, None);
    }

    pub fn add_message_soon_with_number(
        &mut self,
        msg: GtaWideString,
        time: u32,
        flag: u16,
        numbers: [i32; 6],
        now_ms: u32,
    ) {
        if self.brief_messages[0].text.is_some() {
            for i in (2..NUMBRIEFMESSAGES).rev() {
                self.brief_messages[i] = self.brief_messages[i - 1].clone();
            }
            self.brief_messages[1] = message_slot(msg, time, flag, now_ms, numbers, None);
        } else {
            self.brief_messages[0] = message_slot(msg.clone(), time, flag, now_ms, numbers, None);
            self.add_to_previous_brief_array(msg, numbers, None);
        }
    }

    pub fn add_big_message_with_number(
        &mut self,
        msg: GtaWideString,
        time: u32,
        style: u16,
        numbers: [i32; 6],
        now_ms: u32,
    ) {
        let style = style as usize;
        if style < NUMBIGMESSAGES {
            self.big_messages[style].stack[0] = message_slot(msg, time, 0, now_ms, numbers, None);
        }
    }

    pub fn add_big_message_with_number_q(
        &mut self,
        msg: GtaWideString,
        time: u32,
        style: u16,
        numbers: [i32; 6],
        now_ms: u32,
    ) {
        let style = style as usize;
        if style >= NUMBIGMESSAGES {
            return;
        }
        let Some(i) = self.big_messages[style]
            .stack
            .iter()
            .position(|slot| slot.text.is_none())
        else {
            return;
        };
        self.big_messages[style].stack[i] = message_slot(msg, time, 0, now_ms, numbers, None);
    }

    pub fn add_message_with_string(
        &mut self,
        text: GtaWideString,
        time: u32,
        flag: u16,
        string: GtaWideString,
        now_ms: u32,
    ) {
        let Some(i) = self
            .brief_messages
            .iter()
            .position(|slot| slot.text.is_none())
        else {
            return;
        };
        self.brief_messages[i] = message_slot(text, time, flag, now_ms, [-1; 6], Some(string));
        if i == 0 {
            let slot = &self.brief_messages[0];
            self.add_to_previous_brief_array(
                slot.text.clone().unwrap_or_default(),
                slot.numbers,
                slot.string.clone(),
            );
        }
    }

    pub fn add_message_jump_q_with_string(
        &mut self,
        text: GtaWideString,
        time: u32,
        flag: u16,
        string: GtaWideString,
        now_ms: u32,
    ) {
        self.brief_messages[0] = message_slot(
            text.clone(),
            time,
            flag,
            now_ms,
            [-1; 6],
            Some(string.clone()),
        );
        self.add_to_previous_brief_array(text, [-1; 6], Some(string));
    }

    pub fn add_to_previous_brief_array(
        &mut self,
        text: GtaWideString,
        numbers: [i32; 6],
        string: Option<GtaWideString>,
    ) {
        if self.previous_briefs.iter().any(|brief| {
            brief.text.as_ref() == Some(&text) && brief.numbers == numbers && brief.string == string
        }) {
            return;
        }

        let mut i = self
            .previous_briefs
            .iter()
            .position(|brief| brief.text.is_none())
            .unwrap_or(NUMPREVIOUSBRIEFS);
        if i != 0 {
            if i == NUMPREVIOUSBRIEFS {
                i -= 2;
            } else {
                i -= 1;
            }
            loop {
                self.previous_briefs[i + 1] = self.previous_briefs[i].clone();
                if i == 0 {
                    break;
                }
                i -= 1;
            }
        }

        self.previous_briefs[0] = tPreviousBrief {
            text: Some(text),
            numbers,
            string,
        };
    }

    pub fn insert_number_in_string(src: Option<&[u16]>, numbers: [i32; 6]) -> GtaWideString {
        let Some(src) = src else {
            return Vec::new();
        };
        if src.is_empty() {
            return Vec::new();
        }

        let mut out = Vec::with_capacity(src.len() + 24);
        let mut next_number = 0;
        let mut i = 0;
        while i < src.len() {
            if i + 2 < src.len()
                && src[i] == b'~' as u16
                && src[i + 1] == b'1' as u16
                && src[i + 2] == b'~' as u16
            {
                let number = numbers.get(next_number).copied().unwrap_or(-1);
                out.extend(ascii_to_unicode(&number.to_string()));
                next_number += 1;
                i += 3;
            } else {
                out.push(src[i]);
                i += 1;
            }
        }
        out
    }

    pub fn insert_string_in_string(str1: &mut GtaWideString, str2: Option<&[u16]>) {
        let Some(str2) = str2 else {
            return;
        };
        let mut out = Vec::with_capacity(str1.len() + str2.len());
        let mut i = 0;
        while i < str1.len() {
            if i + 2 < str1.len()
                && str1[i] == b'~' as u16
                && str1[i + 1] == b'a' as u16
                && str1[i + 2] == b'~' as u16
            {
                out.extend_from_slice(str2);
                i += 3;
            } else {
                out.push(str1[i]);
                i += 1;
            }
        }
        *str1 = out;
    }

    pub fn insert_player_control_keys_in_string_with_resolver<F>(
        text: &mut GtaWideString,
        mut resolve_action_name: F,
    ) where
        F: FnMut(&[u16]) -> Option<GtaWideString>,
    {
        let mut out = Vec::with_capacity(text.len());
        let mut i = 0;
        while i < text.len() {
            if i + 2 < text.len()
                && text[i] == b'~' as u16
                && text[i + 1] == b'k' as u16
                && text[i + 2] == b'~' as u16
            {
                i += 3;
                let action_start = i;
                while i < text.len() && text[i] != b'~' as u16 {
                    i += 1;
                }
                if let Some(keys) = resolve_action_name(&text[action_start..i]) {
                    out.extend(keys);
                }
                if i < text.len() && text[i] == b'~' as u16 {
                    i += 1;
                }
            } else {
                out.push(text[i]);
                i += 1;
            }
        }
        *text = out;
    }

    pub fn insert_player_control_keys_in_string(text: &mut GtaWideString) {
        Self::insert_player_control_keys_in_string_with_resolver(text, |_| None);
    }

    pub fn clear_this_print(&mut self, str: &[u16], now_ms: u32) {
        while let Some(mut i) = self
            .brief_messages
            .iter()
            .position(|slot| slot.text.as_deref() == Some(str))
        {
            while i < NUMBRIEFMESSAGES - 1 && self.brief_messages[i + 1].text.is_some() {
                self.brief_messages[i] = self.brief_messages[i + 1].clone();
                i += 1;
            }
            self.brief_messages[i] = tMessage::default();
            self.brief_messages[0].start_time = now_ms;
            if let Some(text) = self.brief_messages[0].text.clone() {
                self.add_to_previous_brief_array(
                    text,
                    self.brief_messages[0].numbers,
                    self.brief_messages[0].string.clone(),
                );
            }
        }
    }

    pub fn clear_this_big_print(&mut self, str: &[u16], now_ms: u32) {
        loop {
            let mut found = None;
            for style in 0..NUMBIGMESSAGES {
                for i in 0..BIG_MESSAGE_STACK {
                    if self.big_messages[style].stack[i].text.as_deref() == Some(str) {
                        found = Some((style, i));
                        break;
                    }
                    if self.big_messages[style].stack[i].text.is_none() {
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            let Some((style, mut i)) = found else {
                break;
            };
            while i < BIG_MESSAGE_STACK - 1 && self.big_messages[style].stack[i + 1].text.is_some()
            {
                self.big_messages[style].stack[i] = self.big_messages[style].stack[i + 1].clone();
                i += 1;
            }
            self.big_messages[style].stack[i] = tMessage::default();
            self.big_messages[style].stack[0].start_time = now_ms;
        }
    }

    pub fn clear_all_messages_displayed_by_game(&mut self, hud: &mut CHud) {
        self.clear_messages();
        for brief in &mut self.previous_briefs {
            *brief = tPreviousBrief::default();
        }
        hud.get_rid_of_all_hud_messages();
    }
}

fn message_slot(
    text: GtaWideString,
    time: u32,
    flag: u16,
    start_time: u32,
    numbers: [i32; 6],
    string: Option<GtaWideString>,
) -> tMessage {
    tMessage {
        text: Some(text),
        flag,
        time,
        start_time,
        numbers,
        string,
    }
}

pub struct GtaMessagesPlugin;

impl Plugin for GtaMessagesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_script_text_effects,
                process_messages.after(apply_script_text_effects)
                    .run_if(crate::core::World::world_actor_processing_enabled),
                display_messages.after(process_messages),
            )
                .in_set(GtaProcessSet::UserDisplay),
        );
    }
}

fn apply_script_text_effects(
    scripts: Option<ResMut<CTheScripts>>,
    text: Option<Res<CText>>,
    controls: Option<Res<CControllerConfigManager>>,
    messages: Option<ResMut<CMessages>>,
    hud: Option<ResMut<CHud>>,
    timer: Option<Res<CTimer>>,
) {
    let (
        Some(mut scripts),
        Some(text),
        Some(controls),
        Some(mut messages),
        Some(mut hud),
        Some(timer),
    ) = (scripts, text, controls, messages, hud, timer)
    else {
        return;
    };
    let now = timer.time_in_milliseconds();
    let effects = std::mem::take(&mut scripts.pending_effects);
    for effect in effects {
        match effect {
            ScriptCommandEffect::PrintBig {
                key,
                time_ms,
                style,
                queue,
            } => {
                if queue {
                    messages.add_big_message_q(
                        text.get_key_bytes(&key),
                        time_ms.max(0) as u32,
                        style.max(0) as u16,
                        now,
                    );
                } else {
                    messages.add_big_message(
                        text.get_key_bytes(&key),
                        time_ms.max(0) as u32,
                        style.max(0) as u16,
                        now,
                    );
                }
            }
            ScriptCommandEffect::Print { key, time_ms, flag } => messages.add_message(
                text.get_key_bytes(&key),
                time_ms.max(0) as u32,
                flag.max(0) as u16,
                now,
            ),
            ScriptCommandEffect::PrintNow { key, time_ms, flag } => messages.add_message_jump_q(
                text.get_key_bytes(&key),
                time_ms.max(0) as u32,
                flag.max(0) as u16,
                now,
            ),
            ScriptCommandEffect::PrintSoon { key, time_ms, flag } => messages.add_message_soon(
                text.get_key_bytes(&key),
                time_ms.max(0) as u32,
                flag.max(0) as u16,
                now,
            ),
            ScriptCommandEffect::PrintBigWithNumbers {
                key,
                time_ms,
                style,
                numbers,
                queue,
            } => {
                if queue {
                    messages.add_big_message_with_number_q(
                        text.get_key_bytes(&key),
                        time_ms.max(0) as u32,
                        style.max(0) as u16,
                        numbers,
                        now,
                    );
                } else {
                    messages.add_big_message_with_number(
                        text.get_key_bytes(&key),
                        time_ms.max(0) as u32,
                        style.max(0) as u16,
                        numbers,
                        now,
                    );
                }
            }
            ScriptCommandEffect::PrintWithNumbers {
                key,
                time_ms,
                flag,
                numbers,
                mode,
            } => match mode {
                1 => messages.add_message_jump_q_with_number(
                    text.get_key_bytes(&key),
                    time_ms.max(0) as u32,
                    flag.max(0) as u16,
                    numbers,
                    now,
                ),
                2 => messages.add_message_soon_with_number(
                    text.get_key_bytes(&key),
                    time_ms.max(0) as u32,
                    flag.max(0) as u16,
                    numbers,
                    now,
                ),
                _ => messages.add_message_with_number(
                    text.get_key_bytes(&key),
                    time_ms.max(0) as u32,
                    flag.max(0) as u16,
                    numbers,
                    now,
                ),
            },
            ScriptCommandEffect::PrintStringNow {
                source_key,
                inserted_key,
                time_ms,
                flag,
            } => messages.add_message_jump_q_with_string(
                text.get_key_bytes(&source_key),
                time_ms.max(0) as u32,
                flag.max(0) as u16,
                text.get_key_bytes(&inserted_key),
                now,
            ),
            ScriptCommandEffect::PrintHelp { key } => {
                let message =
                    insert_player_control_keys_in_string(&text.get_key_bytes(&key), &controls);
                hud.set_help_message(&message, false);
            }
            ScriptCommandEffect::ClearHelp => {
                hud.set_help_message(&[], false);
            }
            ScriptCommandEffect::ClearThisPrint { key } => {
                messages.clear_this_print(&text.get_key_bytes(&key), now);
            }
            ScriptCommandEffect::ClearThisBigPrint { key } => {
                messages.clear_this_big_print(&text.get_key_bytes(&key), now);
            }
            ScriptCommandEffect::ClearPrints => {
                messages.clear_all_messages_displayed_by_game(&mut hud);
            }
            ScriptCommandEffect::SkipMessageWait => {
                messages.expire_current_messages_for_skip();
            }
            other => scripts.pending_effects.push(other),
        }
    }
}

/// re3 `CMessages::InsertPlayerControlKeysInString`. GXT stores controller
/// substitutions as `~k~~ACTION_NAME~`; the message queue receives the
/// resolved key names, while ordinary font tokens (`~h~`, `~w~`, `~n~`, ...)
/// remain for `CFont::ParseToken` at draw time.
fn insert_player_control_keys_in_string(
    input: &[u16],
    controls: &CControllerConfigManager,
) -> GtaWideString {
    const MAX_MESSAGE_CHARS: usize = 255;
    const TILDE: u16 = b'~' as u16;
    let mut output = Vec::with_capacity(input.len().min(MAX_MESSAGE_CHARS));
    let mut i = 0;
    while i < input.len() && output.len() < MAX_MESSAGE_CHARS {
        if input.get(i..i + 4) == Some(&[TILDE, b'k' as u16, TILDE, TILDE]) {
            let action_start = i + 4;
            if let Some(relative_end) = input[action_start..]
                .iter()
                .position(|character| *character == TILDE)
            {
                let action_end = action_start + relative_end;
                let action_name = String::from_utf16_lossy(&input[action_start..action_end]);
                if let Some(action) = e_ControllerAction::from_gxt_name(&action_name) {
                    output.extend(
                        controls
                            .binding_text(action)
                            .encode_utf16()
                            .take(MAX_MESSAGE_CHARS - output.len()),
                    );
                    i = action_end + 1;
                    continue;
                }
            }
        }
        output.push(input[i]);
        i += 1;
    }
    output
}

fn process_messages(messages: Option<ResMut<CMessages>>, timer: Option<Res<CTimer>>) {
    let (Some(mut messages), Some(timer)) = (messages, timer) else {
        return;
    };
    messages.process(timer.time_in_milliseconds());
}

fn display_messages(messages: Option<Res<CMessages>>, hud: Option<ResMut<CHud>>) {
    let (Some(messages), Some(mut hud)) = (messages, hud) else {
        return;
    };
    messages.display(&mut hud);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_and_immediate_message_paths_preserve_cpp_stack_semantics() {
        let mut messages = CMessages::init();
        messages.add_big_message(ascii_to_unicode("A"), 1_000, 1, 10);
        messages.add_big_message_q(ascii_to_unicode("B"), 2_000, 1, 20);
        assert_eq!(
            messages.big_messages[1].stack[0].text,
            Some(ascii_to_unicode("A"))
        );
        assert_eq!(
            messages.big_messages[1].stack[1].text,
            Some(ascii_to_unicode("B"))
        );
        messages.add_big_message(ascii_to_unicode("C"), 3_000, 1, 30);
        assert_eq!(
            messages.big_messages[1].stack[0].text,
            Some(ascii_to_unicode("C"))
        );

        messages.add_message(ascii_to_unicode("D"), 1_000, 0, 40);
        messages.add_message(ascii_to_unicode("E"), 1_000, 0, 50);
        assert_eq!(messages.brief_messages[0].text, Some(ascii_to_unicode("D")));
        assert_eq!(messages.brief_messages[1].text, Some(ascii_to_unicode("E")));
        messages.add_message_jump_q(ascii_to_unicode("F"), 1_000, 0, 60);
        assert_eq!(messages.brief_messages[0].text, Some(ascii_to_unicode("F")));
    }
}
