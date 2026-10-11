//! re3 reference: references/re-GTA/src/renderer/Console.cpp
//! re3 reference: references/re-GTA/src/renderer/Console.h
//!
//! Developer console ring buffer.

use bevy::prelude::Resource;

pub const MAX_LINES: usize = 8;
pub const MAX_STR_LEN: usize = 40;
pub const CONSOLE_X_POS: f32 = 30.0;
pub const CONSOLE_Y_POS: f32 = 10.0;
pub const CONSOLE_LINE_HEIGHT: f32 = 12.0;
pub const CONSOLE_LINE_LIFETIME_MS: u32 = 20_000;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct CConsole {
    pub line_count: u8,
    pub current_line: u8,
    pub buffers: [String; MAX_LINES],
    pub timers: [u32; MAX_LINES],
    pub red: [u8; MAX_LINES],
    pub green: [u8; MAX_LINES],
    pub blue: [u8; MAX_LINES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleDisplayLine {
    pub text: String,
    pub colour: [u8; 4],
    pub shadow_colour: [u8; 4],
}

impl CConsole {
    pub fn init(&mut self) {
        self.current_line = 0;
        self.line_count = 0;
    }

    pub fn add_line(&mut self, text: &str, r: u8, g: u8, b: u8, time_ms: u32) {
        let mut start = 0;
        while text[start..].len() > MAX_STR_LEN {
            let end = start + MAX_STR_LEN - 1;
            self.add_one_line(&text[start..end], r, g, b, time_ms);
            start = end;
        }
        self.add_one_line(&text[start..], r, g, b, time_ms);
    }

    pub fn add_one_line(&mut self, text: &str, r: u8, g: u8, b: u8, time_ms: u32) {
        let str_index = (self.line_count + self.current_line) as usize % MAX_LINES;
        self.buffers[str_index] = text.chars().take(MAX_STR_LEN - 1).collect();
        self.timers[str_index] = time_ms;
        self.red[str_index] = r;
        self.green[str_index] = g;
        self.blue[str_index] = b;

        let next_count = self.line_count.saturating_add(1);
        if next_count >= MAX_LINES as u8 {
            self.current_line = (self.current_line + 1) % MAX_LINES as u8;
        } else {
            self.line_count = next_count;
        }
    }

    pub fn display(&mut self, time_ms: u32) -> Vec<ConsoleDisplayLine> {
        while self.line_count != 0
            && time_ms.wrapping_sub(self.timers[self.current_line as usize])
                > CONSOLE_LINE_LIFETIME_MS
        {
            self.line_count -= 1;
            self.current_line = (self.current_line + 1) % MAX_LINES as u8;
        }

        (0..self.line_count as usize)
            .map(|i| {
                let line = (i + self.current_line as usize) % MAX_LINES;
                ConsoleDisplayLine {
                    text: self.buffers[line].clone(),
                    colour: [self.red[line], self.green[line], self.blue[line], 200],
                    shadow_colour: [0, 0, 0, 200],
                }
            })
            .collect()
    }
}

impl Default for CConsole {
    fn default() -> Self {
        Self {
            line_count: 0,
            current_line: 0,
            buffers: std::array::from_fn(|_| String::new()),
            timers: [0; MAX_LINES],
            red: [0; MAX_LINES],
            green: [0; MAX_LINES],
            blue: [0; MAX_LINES],
        }
    }
}
