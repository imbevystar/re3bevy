//! re3 reference: references/re-GTA/src/renderer/Fluff.cpp
//! re3 reference: references/re-GTA/src/renderer/Fluff.h
//!
//! Moving signs, scroll bars, tower clocks and digital clock state.

use bevy::prelude::Resource;
use rust_gta_3_core::core::Coordinates::CVector;

pub const NUMMOVINGTHINGS: usize = 128;
pub const NUM_SCROLL_BARS: usize = 11;
pub const NUM_TOWER_CLOCKS: usize = 2;
pub const NUM_DIGITAL_CLOCKS: usize = 3;
pub const MOVING_THINGS_TIME_SPAN: u32 = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct CMovingThing {
    pub next: Option<usize>,
    pub prev: Option<usize>,
    pub thing_type: i16,
    pub hidden: i16,
    pub posn: CVector,
    pub entity_id: Option<usize>,
    pub last_updated_frame: u32,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CMovingThings {
    pub num: i16,
    pub moving_things: Vec<CMovingThing>,
    pub scroll_bars: Vec<CScrollBar>,
    pub tower_clocks: Vec<CTowerClock>,
    pub digital_clocks: Vec<CDigitalClock>,
    pub render_intents: Vec<FluffRenderIntent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CScrollBar {
    pub counter: u8,
    pub message: String,
    pub position: CVector,
    pub message_current_char: u32,
    pub message_length: u32,
    pub size: CVector,
    pub intensity: f32,
    pub message_bar: [u8; 40],
    pub scroll_type: u8,
    pub visible: bool,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CTowerClock {
    pub position: CVector,
    pub size: CVector,
    pub draw_distance: f32,
    pub scale: f32,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub visible: bool,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CDigitalClock {
    pub position: CVector,
    pub size: CVector,
    pub draw_distance: f32,
    pub scale: f32,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub visible: bool,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluffRenderIntent {
    ScrollBar(usize),
    TowerClock(usize),
    DigitalClock(usize),
}

impl CMovingThing {
    pub fn update(&mut self, frame_counter: u32) {
        self.last_updated_frame = frame_counter;
    }

    pub fn add_to_list(&mut self, thing: usize) {
        self.next = Some(thing);
    }

    pub fn remove_from_list(&mut self) {
        self.next = None;
        self.prev = None;
    }
}

impl CMovingThings {
    pub fn init(&mut self) {
        self.num = 0;
        self.moving_things = vec![CMovingThing::default(); NUMMOVINGTHINGS];
        self.scroll_bars = default_scroll_bars();
        self.tower_clocks = default_tower_clocks();
        self.digital_clocks = default_digital_clocks();
    }

    pub fn shutdown(&mut self) {
        for scroll_bar in &mut self.scroll_bars {
            scroll_bar.set_visibility(false);
        }
        for tower_clock in &mut self.tower_clocks {
            tower_clock.set_visibility(false);
        }
        for digital_clock in &mut self.digital_clocks {
            digital_clock.set_visibility(false);
        }
    }

    pub fn update(&mut self, frame_counter: u32) {
        let block = frame_counter % MOVING_THINGS_TIME_SPAN;
        let start = (block as usize * NUMMOVINGTHINGS) / MOVING_THINGS_TIME_SPAN as usize;
        let end = ((block as usize + 1) * NUMMOVINGTHINGS) / MOVING_THINGS_TIME_SPAN as usize;
        for moving_thing in &mut self.moving_things[start..end] {
            if moving_thing.hidden == 1 {
                moving_thing.update(frame_counter);
            }
        }
        for moving_thing in self.moving_things.iter_mut().take(self.num.max(0) as usize) {
            if moving_thing.hidden == 0 {
                moving_thing.update(frame_counter);
            }
        }
    }

    pub fn render(&mut self) {
        self.render_intents.clear();
        for (index, scroll_bar) in self.scroll_bars.iter().enumerate() {
            if scroll_bar.is_visible() {
                self.render_intents
                    .push(FluffRenderIntent::ScrollBar(index));
            }
        }
        for (index, tower_clock) in self.tower_clocks.iter().enumerate() {
            if tower_clock.is_visible() {
                self.render_intents
                    .push(FluffRenderIntent::TowerClock(index));
            }
        }
        for (index, digital_clock) in self.digital_clocks.iter().enumerate() {
            if digital_clock.is_visible() {
                self.render_intents
                    .push(FluffRenderIntent::DigitalClock(index));
            }
        }
    }

    pub fn drain_render_intents(&mut self) -> Vec<FluffRenderIntent> {
        std::mem::take(&mut self.render_intents)
    }
}

impl CScrollBar {
    pub fn set_visibility(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl CTowerClock {
    pub fn set_visibility(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl CDigitalClock {
    pub fn set_visibility(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for CMovingThing {
    fn default() -> Self {
        Self {
            next: None,
            prev: None,
            thing_type: 0,
            hidden: 0,
            posn: CVector::ZERO,
            entity_id: None,
            last_updated_frame: 0,
        }
    }
}

impl Default for CMovingThings {
    fn default() -> Self {
        let mut moving_things = Self {
            num: 0,
            moving_things: Vec::new(),
            scroll_bars: Vec::new(),
            tower_clocks: Vec::new(),
            digital_clocks: Vec::new(),
            render_intents: Vec::new(),
        };
        moving_things.init();
        moving_things
    }
}

fn default_scroll_bars() -> Vec<CScrollBar> {
    let defs = [
        (
            CVector::new(228.3, -669.0, 39.0),
            0,
            0.0,
            0.5,
            0.5,
            255,
            128,
            0,
            0.3,
        ),
        (
            CVector::new(772.0, 164.0, -9.5),
            1,
            0.0,
            0.5,
            0.25,
            128,
            255,
            0,
            0.3,
        ),
        (
            CVector::new(-1089.61, -584.224, 13.246),
            3,
            0.0,
            -0.1706,
            0.107,
            255,
            0,
            0,
            0.11,
        ),
        (
            CVector::new(-1089.61, -602.04602, 13.246),
            3,
            0.0,
            -0.1706,
            0.107,
            0,
            255,
            0,
            0.11,
        ),
        (
            CVector::new(-1089.61, -619.81702, 13.246),
            3,
            0.0,
            -0.1706,
            0.107,
            255,
            128,
            0,
            0.11,
        ),
        (
            CVector::new(-754.578, -633.50897, 18.411),
            4,
            0.0,
            0.591,
            0.52,
            100,
            100,
            255,
            0.3,
        ),
        (
            CVector::new(-754.578, -586.672, 18.411),
            4,
            0.0,
            0.591,
            0.52,
            100,
            100,
            255,
            0.3,
        ),
        (
            CVector::new(85.473, -1069.512, 30.5),
            5,
            0.625,
            -0.3125,
            0.727,
            100,
            100,
            255,
            0.5,
        ),
        (
            CVector::new(74.823, -1086.879, 31.495),
            2,
            -0.2083,
            0.1041,
            0.5,
            255,
            255,
            128,
            0.3,
        ),
        (
            CVector::new(-36.459, -1031.2371, 32.534),
            2,
            -0.1442,
            0.0721,
            0.229,
            150,
            255,
            50,
            0.3,
        ),
        (
            CVector::new(1208.0, -62.208, 19.157),
            6,
            0.0642,
            -0.20365,
            0.229,
            255,
            128,
            0,
            0.3,
        ),
    ];
    defs.into_iter()
        .map(
            |(position, scroll_type, x, y, z, red, green, blue, scale)| CScrollBar {
                counter: 0,
                message: String::new(),
                position,
                message_current_char: 0,
                message_length: 0,
                size: CVector::new(x, y, z),
                intensity: 0.0,
                message_bar: [0; 40],
                scroll_type,
                visible: false,
                red,
                green,
                blue,
                scale,
            },
        )
        .collect()
}

fn default_tower_clocks() -> Vec<CTowerClock> {
    vec![
        CTowerClock::new(
            CVector::new(59.4, -1081.3, 54.15),
            -1.0,
            0.0,
            0,
            0,
            0,
            80.0,
            2.0,
        ),
        CTowerClock::new(
            CVector::new(55.4, -1083.6, 54.15),
            0.0,
            -1.0,
            0,
            0,
            0,
            80.0,
            2.0,
        ),
    ]
}

fn default_digital_clocks() -> Vec<CDigitalClock> {
    vec![
        CDigitalClock::new(
            CVector::new(54.485, -1081.679, 32.803),
            3.7,
            2.144,
            255,
            0,
            0,
            100.0,
            0.8,
        ),
        CDigitalClock::new(
            CVector::new(60.564, -1083.089, 32.803),
            -2.144,
            3.7,
            0,
            0,
            255,
            100.0,
            0.8,
        ),
        CDigitalClock::new(
            CVector::new(58.145, -1079.268, 32.803),
            -3.7,
            -2.144,
            0,
            255,
            0,
            100.0,
            0.8,
        ),
    ]
}

impl CTowerClock {
    fn new(
        position: CVector,
        x: f32,
        y: f32,
        red: u8,
        green: u8,
        blue: u8,
        draw_distance: f32,
        scale: f32,
    ) -> Self {
        Self {
            position,
            size: CVector::new(x, y, 0.0),
            draw_distance,
            scale,
            red,
            green,
            blue,
            visible: false,
            intensity: 0.0,
        }
    }
}

impl CDigitalClock {
    fn new(
        position: CVector,
        x: f32,
        y: f32,
        red: u8,
        green: u8,
        blue: u8,
        draw_distance: f32,
        scale: f32,
    ) -> Self {
        Self {
            position,
            size: CVector::new(x, y, 0.0),
            draw_distance,
            scale,
            red,
            green,
            blue,
            visible: false,
            intensity: 0.0,
        }
    }
}
