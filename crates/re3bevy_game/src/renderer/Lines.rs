//! re3 reference: references/re-GTA/src/renderer/Lines.cpp
//! re3 reference: references/re-GTA/src/renderer/Lines.h

use bevy::prelude::Resource;
use re3bevy_core::core::Coordinates::CVector;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineVertex {
    pub position: CVector,
    pub colour: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineIntent {
    pub start: LineVertex,
    pub end: LineVertex,
}

#[derive(Debug, Clone, Default, Resource)]
pub struct CLines {
    pub lines: Vec<LineIntent>,
}

impl CLines {
    pub fn render_line_with_clipping(&mut self, start: CVector, end: CVector, c1: u32, c2: u32) {
        self.lines.push(LineIntent {
            start: LineVertex {
                position: start,
                colour: rgba_from_u32(c1),
            },
            end: LineVertex {
                position: end,
                colour: rgba_from_u32(c2),
            },
        });
    }

    pub fn drain_lines(&mut self) -> Vec<LineIntent> {
        std::mem::take(&mut self.lines)
    }
}

fn rgba_from_u32(colour: u32) -> [u8; 4] {
    [
        (colour >> 24) as u8,
        (colour >> 16) as u8,
        (colour >> 8) as u8,
        colour as u8,
    ]
}
