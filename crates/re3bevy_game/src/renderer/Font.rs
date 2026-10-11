//! re3 reference: references/re-GTA/src/renderer/Font.cpp
//! re3 reference: references/re-GTA/src/renderer/Font.h
//!
//! Font details and queued text draw intents.

use bevy::prelude::*;

use crate::{
    entities::Entity::CRect, objects::ParticleObject::CRGBA, renderer::Sprite2d::CSprite2d,
    text::Text::GtaWideString,
};

pub const FONT_BANK: i16 = 0;
pub const FONT_PAGER: i16 = 1;
pub const FONT_HEADING: i16 = 2;
pub const MAX_FONTS: usize = 3;
pub const FONT_SIZE_TABLE_LEN: usize = 193;
pub const FONT_SIZE: [[i16; FONT_SIZE_TABLE_LEN]; MAX_FONTS] = [
    [
        13, 12, 31, 35, 23, 35, 31, 9, 14, 15, 25, 30, 11, 17, 13, 31, 23, 16, 22, 21, 24, 23, 23,
        20, 23, 22, 10, 35, 26, 26, 26, 26, 30, 26, 24, 23, 24, 22, 21, 24, 26, 10, 20, 26, 22, 29,
        26, 25, 23, 25, 24, 24, 22, 25, 24, 29, 29, 23, 25, 37, 22, 37, 35, 37, 35, 21, 22, 21, 21,
        22, 13, 22, 21, 10, 16, 22, 11, 32, 21, 21, 23, 22, 16, 20, 14, 21, 20, 30, 25, 21, 21, 33,
        33, 33, 33, 35, 27, 27, 27, 27, 32, 24, 23, 23, 23, 23, 11, 11, 11, 11, 26, 26, 26, 26, 26,
        26, 26, 25, 26, 21, 21, 21, 21, 32, 23, 22, 22, 22, 22, 11, 11, 11, 11, 22, 22, 22, 22, 22,
        22, 22, 22, 26, 21, 24, 12, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26,
        26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 18, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26,
        26, 26, 26, 26, 26, 26, 26, 26, 20,
    ],
    [
        13, 9, 21, 35, 23, 35, 35, 11, 35, 35, 25, 35, 11, 17, 13, 33, 28, 14, 22, 21, 24, 23, 23,
        21, 23, 22, 10, 35, 13, 35, 13, 33, 5, 25, 22, 23, 24, 21, 21, 24, 24, 9, 20, 24, 21, 27,
        25, 25, 22, 25, 23, 20, 23, 23, 23, 31, 23, 23, 23, 37, 33, 37, 35, 37, 35, 21, 19, 19, 21,
        19, 17, 21, 21, 8, 17, 18, 14, 24, 21, 21, 20, 22, 19, 20, 20, 19, 20, 26, 21, 20, 21, 33,
        33, 33, 33, 35, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 19, 19, 19, 19, 19, 16,
    ],
    [
        15, 14, 16, 25, 19, 26, 22, 11, 18, 18, 27, 26, 13, 19, 9, 27, 19, 18, 19, 19, 22, 19, 20,
        18, 19, 20, 12, 32, 15, 32, 15, 35, 15, 19, 19, 19, 19, 19, 16, 19, 20, 9, 19, 20, 14, 29,
        19, 20, 19, 19, 19, 19, 21, 19, 20, 32, 20, 19, 19, 33, 31, 39, 37, 39, 37, 21, 21, 21, 23,
        21, 19, 23, 23, 10, 19, 20, 16, 26, 23, 23, 20, 20, 20, 22, 21, 22, 22, 26, 22, 22, 23, 35,
        35, 35, 35, 37, 19, 19, 19, 19, 29, 19, 19, 19, 19, 19, 9, 9, 9, 9, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 19, 19, 19, 19, 30, 19, 19, 19, 19, 19, 10, 10, 10, 10, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 23, 35, 12, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 11, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
        19, 19, 19, 19, 19, 19, 19, 19,
    ],
];

#[derive(Debug, Clone, PartialEq)]
pub struct CFontDetails {
    pub color: CRGBA,
    pub scale_x: f32,
    pub scale_y: f32,
    pub slant: f32,
    pub slant_ref_x: f32,
    pub slant_ref_y: f32,
    pub justify: bool,
    pub centre: bool,
    pub right_justify: bool,
    pub background: bool,
    pub background_only_text: bool,
    pub proportional: bool,
    pub alpha_fade: f32,
    pub background_color: CRGBA,
    pub wrap_x: f32,
    pub centre_size: f32,
    pub right_justify_wrap: f32,
    pub style: i16,
    pub bank: i32,
    pub drop_shadow_position: i16,
    pub drop_color: CRGBA,
}

impl Default for CFontDetails {
    fn default() -> Self {
        Self {
            color: CRGBA::new(0, 0, 0, 255),
            scale_x: 1.0,
            scale_y: 1.0,
            slant: 0.0,
            slant_ref_x: 0.0,
            slant_ref_y: 0.0,
            justify: false,
            centre: false,
            right_justify: false,
            background: false,
            background_only_text: false,
            proportional: true,
            alpha_fade: 255.0,
            background_color: CRGBA::new(128, 128, 128, 128),
            wrap_x: 640.0,
            centre_size: 640.0,
            right_justify_wrap: 0.0,
            style: FONT_BANK,
            bank: 0,
            drop_shadow_position: 0,
            drop_color: CRGBA::new(0, 0, 0, 255),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontPrintCommand {
    pub x: f32,
    pub y: f32,
    pub text: GtaWideString,
    pub details: CFontDetails,
    pub from_bottom: bool,
}

#[derive(Debug, Clone, Resource)]
pub struct CFont {
    pub sprite: [CSprite2d; MAX_FONTS],
    pub details: CFontDetails,
    pub new_line: bool,
    pub print_queue: Vec<FontPrintCommand>,
}

impl CFont {
    pub fn initialise() -> Self {
        let mut font = Self {
            sprite: [
                CSprite2d::default(),
                CSprite2d::default(),
                CSprite2d::default(),
            ],
            details: CFontDetails::default(),
            new_line: false,
            print_queue: Vec::new(),
        };
        // CFont::Initialise: Sprite[0]=font2/font2_mask,
        // Sprite[1]=pager/pager_mask, Sprite[2]=font1/font1_mask.
        font.sprite[FONT_BANK as usize].set_texture("font2", Some("font2_mask"));
        font.sprite[FONT_PAGER as usize].set_texture("pager", Some("pager_mask"));
        font.sprite[FONT_HEADING as usize].set_texture("font1", Some("font1_mask"));
        font
    }

    pub fn shutdown(&mut self) {
        for sprite in &mut self.sprite {
            sprite.delete();
        }
        self.print_queue.clear();
    }

    pub fn init_per_frame(&mut self) {
        self.print_queue.clear();
        self.details = CFontDetails::default();
        self.new_line = false;
    }

    pub fn print_string(&mut self, x: f32, y: f32, text: &[u16]) {
        self.print_queue.push(FontPrintCommand {
            x,
            y,
            text: text.to_vec(),
            details: self.details.clone(),
            from_bottom: false,
        });
    }

    pub fn print_string_from_bottom(&mut self, x: f32, y: f32, text: &[u16]) {
        self.print_queue.push(FontPrintCommand {
            x,
            y,
            text: text.to_vec(),
            details: self.details.clone(),
            from_bottom: true,
        });
    }

    pub fn get_character_width(&self, c: u16) -> f32 {
        let style = self.details.style.clamp(0, (MAX_FONTS - 1) as i16) as usize;
        let index = c.min((FONT_SIZE_TABLE_LEN - 1) as u16) as usize;
        if self.details.proportional {
            FONT_SIZE[style][index] as f32
        } else {
            FONT_SIZE[style][FONT_SIZE_TABLE_LEN - 1] as f32
        }
    }

    pub fn get_character_size(&self, c: u16) -> f32 {
        self.get_character_width(c) * self.details.scale_x
    }

    pub fn get_string_width(&self, text: &[u16], spaces: bool) -> f32 {
        let mut width = 0.0;
        let mut i = 0;
        while i < text.len() {
            let ch = text[i];
            if ch == 0 || (ch == b' ' as u16 && !spaces) {
                break;
            }
            if ch == b'~' as u16 {
                i += 1;
                while i < text.len() && text[i] != b'~' as u16 {
                    i += 1;
                }
                if i < text.len() {
                    i += 1;
                    if i < text.len() && text[i] == b' ' as u16 && !spaces {
                        break;
                    }
                } else {
                    break;
                }
            }
            if i >= text.len() || text[i] == 0 || (text[i] == b' ' as u16 && !spaces) {
                break;
            }
            let ch = text[i];
            width += self.get_character_size(ch.saturating_sub(b' ' as u16));
            i += 1;
        }
        width
    }

    pub fn get_number_lines(&self, xstart: f32, _ystart: f32, text: &[u16]) -> i32 {
        let mut lines = 0;
        let mut x = if self.details.centre || self.details.right_justify {
            0.0
        } else {
            xstart
        };
        let mut i = 0;
        while i < text.len() && text[i] != 0 {
            let line_end = if self.details.centre {
                self.details.centre_size
            } else {
                self.details.wrap_x
            };
            if x + self.get_string_width(&text[i..], false) > line_end {
                x = if self.details.centre || self.details.right_justify {
                    0.0
                } else {
                    xstart
                };
                lines += 1;
            } else {
                let next = self.next_space_index(text, i);
                if next >= text.len() || text[next] == 0 {
                    x += self.get_string_width(&text[i..], false);
                    lines += 1;
                    i = next;
                } else {
                    x += self.get_string_width(&text[i..], false)
                        + self.get_character_size(text[next].saturating_sub(b' ' as u16));
                    i = next + 1;
                }
            }
        }
        lines
    }

    pub fn get_text_rect(&self, xstart: f32, ystart: f32, text: &[u16]) -> CRect {
        let mut max_length: i16 = 0;
        let mut num_lines = 0;
        let mut x = if self.details.centre { 0.0 } else { xstart };
        let mut i = 0;
        while i < text.len() && text[i] != 0 {
            let line_end = if self.details.centre {
                self.details.centre_size
            } else {
                self.details.wrap_x
            };
            if x + self.get_string_width(&text[i..], false) > line_end {
                if x > max_length as f32 {
                    max_length = x as i16;
                }
                x = if self.details.centre { 0.0 } else { xstart };
                num_lines += 1;
            } else {
                let next = self.next_space_index(text, i);
                if next >= text.len() || text[next] == 0 {
                    x += self.get_string_width(&text[i..], false);
                    if x > max_length as f32 {
                        max_length = x as i16;
                    }
                    num_lines += 1;
                    i = next;
                } else {
                    x += self.get_string_width(&text[i..], false)
                        + self.get_character_size(text[next].saturating_sub(b' ' as u16));
                    i = next + 1;
                }
            }
        }

        let line_height = 32.0 * self.details.scale_y * 0.5 + 2.0 * self.details.scale_y;
        if self.details.centre {
            if self.details.background_only_text {
                CRect::new(
                    xstart - (max_length / 2) as f32 - 4.0,
                    ystart - 2.0,
                    xstart + (max_length / 2) as f32 + 4.0,
                    line_height * num_lines as f32 + ystart + 2.0,
                )
            } else {
                CRect::new(
                    xstart - self.details.centre_size * 0.5 - 4.0,
                    ystart - 2.0,
                    xstart + self.details.centre_size * 0.5 + 4.0,
                    line_height * num_lines as f32 + ystart + 2.0,
                )
            }
        } else {
            CRect::new(
                xstart - 4.0,
                line_height * num_lines as f32 + ystart + 4.0,
                self.details.wrap_x,
                ystart,
            )
        }
    }

    fn next_space_index(&self, text: &[u16], mut index: usize) -> usize {
        while index < text.len() && text[index] != b' ' as u16 && text[index] != 0 {
            if text[index] == b'~' as u16 {
                index += 1;
                while index < text.len() && text[index] != b'~' as u16 {
                    index += 1;
                }
                if index < text.len() {
                    index += 1;
                    if index < text.len() && text[index] == b' ' as u16 {
                        break;
                    }
                }
            } else {
                index += 1;
            }
        }
        index
    }

    pub fn set_scale(&mut self, x: f32, y: f32) {
        self.details.scale_x = x;
        self.details.scale_y = y;
    }

    pub fn set_slant_ref_point(&mut self, x: f32, y: f32) {
        self.details.slant_ref_x = x;
        self.details.slant_ref_y = y;
    }

    pub fn set_slant(&mut self, value: f32) {
        self.details.slant = value;
    }

    pub fn set_color(&mut self, color: CRGBA) {
        let mut color = color;
        if self.details.alpha_fade < 255.0 {
            color.alpha = ((color.alpha as f32) * self.details.alpha_fade / 255.0) as u8;
        }
        self.details.color = color;
    }

    pub fn set_justify_on(&mut self) {
        self.details.justify = true;
        self.details.centre = false;
        self.details.right_justify = false;
    }

    pub fn set_justify_off(&mut self) {
        self.details.justify = false;
        self.details.right_justify = false;
    }

    pub fn set_right_justify_on(&mut self) {
        self.details.right_justify = true;
        self.details.justify = false;
        self.details.centre = false;
    }

    pub fn set_right_justify_off(&mut self) {
        self.details.right_justify = false;
        self.details.justify = false;
        self.details.centre = false;
    }

    pub fn set_centre_on(&mut self) {
        self.details.centre = true;
        self.details.justify = false;
        self.details.right_justify = false;
    }

    pub fn set_centre_off(&mut self) {
        self.details.centre = false;
    }

    pub fn set_wrapx(&mut self, x: f32) {
        self.details.wrap_x = x;
    }

    pub fn set_centre_size(&mut self, size: f32) {
        self.details.centre_size = size;
    }

    pub fn set_background_on(&mut self) {
        self.details.background = true;
    }

    pub fn set_background_off(&mut self) {
        self.details.background = false;
    }

    pub fn set_background_color(&mut self, color: CRGBA) {
        self.details.background_color = color;
    }

    pub fn set_background_only_text_on(&mut self) {
        self.details.background_only_text = true;
    }

    pub fn set_background_only_text_off(&mut self) {
        self.details.background_only_text = false;
    }

    pub fn set_prop_on(&mut self) {
        self.details.proportional = true;
    }

    pub fn set_prop_off(&mut self) {
        self.details.proportional = false;
    }

    pub fn set_font_style(&mut self, style: i16) {
        self.details.style = style;
    }

    pub fn set_right_justify_wrap(&mut self, wrap: f32) {
        self.details.right_justify_wrap = wrap;
    }

    pub fn set_alpha_fade(&mut self, fade: f32) {
        self.details.alpha_fade = fade;
    }

    pub fn set_drop_color(&mut self, color: CRGBA) {
        let mut color = color;
        if self.details.alpha_fade < 255.0 {
            color.alpha = ((color.alpha as f32) * self.details.alpha_fade / 255.0) as u8;
        }
        self.details.drop_color = color;
    }

    pub fn set_drop_shadow_position(&mut self, pos: i16) {
        self.details.drop_shadow_position = pos;
    }
}
