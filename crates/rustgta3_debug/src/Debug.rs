//! re3 reference: references/re-GTA/src/core/Debug.cpp
//! re3 reference: references/re-GTA/src/core/Debug.h
//!
//! `CDebug` text/line buffers represented as data; drawing is handled by the
//! renderer/debug UI adapter.

use bevy::prelude::Resource;

pub const MAX_LINES: usize = 15;
pub const MAX_STR_LEN: usize = 80;
pub const MAX_SCREEN_STRS: usize = 100;
pub const MAX_DEBUG_LINES: usize = 100;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CDebug {
    pub current_text_line: usize,
    pub text_buffer: Vec<String>,
    pub screen_strings: Vec<ScreenStr>,
    pub lines: Vec<DebugLine>,
    pub debug_stuff_in_release: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenStr {
    pub x: i32,
    pub y: i32,
    pub str: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebugLine {
    pub p1: [f32; 3],
    pub p2: [f32; 3],
    pub c1: u32,
    pub c2: u32,
}

impl Default for CDebug {
    fn default() -> Self {
        Self {
            current_text_line: 0,
            text_buffer: vec![String::new(); MAX_LINES],
            screen_strings: Vec::with_capacity(MAX_SCREEN_STRS),
            lines: Vec::with_capacity(MAX_DEBUG_LINES),
            debug_stuff_in_release: false,
        }
    }
}

impl CDebug {
    pub fn DebugInitTextBuffer(&mut self) {
        self.current_text_line = 0;
        self.text_buffer.fill(String::new());
    }

    pub fn DebugAddText(&mut self, str: impl AsRef<str>) {
        if self.text_buffer.len() != MAX_LINES {
            self.text_buffer.resize(MAX_LINES, String::new());
        }
        self.text_buffer[self.current_text_line] = truncate(str.as_ref(), MAX_STR_LEN);
        self.current_text_line = (self.current_text_line + 1) % MAX_LINES;
    }

    pub fn PrintAt(&mut self, str: impl AsRef<str>, x: i32, y: i32) {
        if self.screen_strings.len() == MAX_SCREEN_STRS {
            self.screen_strings.remove(0);
        }
        self.screen_strings.push(ScreenStr {
            x,
            y,
            str: truncate(str.as_ref(), 255),
        });
    }

    pub fn AddLine(&mut self, p1: [f32; 3], p2: [f32; 3], c1: u32, c2: u32) {
        if self.lines.len() == MAX_DEBUG_LINES {
            self.lines.remove(0);
        }
        self.lines.push(DebugLine { p1, p2, c1, c2 });
    }

    pub fn DebugDisplayTextBuffer(&self) -> &[String] {
        &self.text_buffer
    }

    pub fn DisplayScreenStrings(&self) -> &[ScreenStr] {
        &self.screen_strings
    }

    pub fn DrawLines(&self) -> &[DebugLine] {
        &self.lines
    }
}

fn truncate(value: &str, max_len: usize) -> String {
    value.chars().take(max_len.saturating_sub(1)).collect()
}
