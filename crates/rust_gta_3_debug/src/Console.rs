//! re3 reference: references/re-GTA/src/renderer/Console.cpp
//! re3 reference: references/re-GTA/src/renderer/Console.h
//!
//! Console message ring buffer. Rendering is performed by the debug UI layer.

use bevy::prelude::Resource;

pub const MAX_LINES: usize = 8;
pub const MAX_STR_LEN: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct CConsole {
    pub line_count: u8,
    pub current_line: u8,
    pub buffers: Vec<ConsoleLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleLine {
    pub text: String,
    pub timer: u32,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Default for CConsole {
    fn default() -> Self {
        let mut console = Self {
            line_count: 0,
            current_line: 0,
            buffers: Vec::new(),
        };
        console.Init();
        console
    }
}

impl CConsole {
    pub fn Init(&mut self) {
        self.current_line = 0;
        self.line_count = 0;
        self.buffers = vec![ConsoleLine::default(); MAX_LINES];
    }

    pub fn AddLine(&mut self, s: impl AsRef<str>, r: u8, g: u8, b: u8) {
        let index = self.current_line as usize % MAX_LINES;
        self.buffers[index] = ConsoleLine {
            text: truncate(s.as_ref()),
            timer: 0,
            red: r,
            green: g,
            blue: b,
        };
        self.current_line = ((self.current_line as usize + 1) % MAX_LINES) as u8;
        self.line_count = self.line_count.saturating_add(1).min(MAX_LINES as u8);
    }

    pub fn AddOneLine(&mut self, s: impl AsRef<str>, r: u8, g: u8, b: u8) {
        self.Init();
        self.AddLine(s, r, g, b);
    }

    pub fn Display(&self) -> impl Iterator<Item = &ConsoleLine> {
        self.buffers
            .iter()
            .take(self.line_count as usize)
            .filter(|line| !line.text.is_empty())
    }
}

impl Default for ConsoleLine {
    fn default() -> Self {
        Self {
            text: String::new(),
            timer: 0,
            red: 255,
            green: 255,
            blue: 255,
        }
    }
}

pub fn cprintf(console: &mut CConsole, s: impl AsRef<str>) {
    console.AddLine(s, 255, 255, 255);
}

fn truncate(value: &str) -> String {
    value.chars().take(MAX_STR_LEN.saturating_sub(1)).collect()
}
