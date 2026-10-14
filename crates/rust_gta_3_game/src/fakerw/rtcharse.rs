//! re3 reference: references/re-GTA/src/fakerw/rtcharse.h
//!
//! RenderWare toolkit charset command buffer.

use super::rwplcore::RwRGBA;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtCharset {
    pub foreground: RwRGBA,
    pub background: RwRGBA,
    pub buffered: Vec<RtCharsetPrintCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtCharsetDesc {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtCharsetPrintCommand {
    pub string: String,
    pub x: i32,
    pub y: i32,
    pub hide_spaces: bool,
}

pub fn RtCharsetOpen() -> bool {
    true
}

pub fn RtCharsetClose() {}

pub fn RtCharsetCreate(foreground: RwRGBA, background: RwRGBA) -> RtCharset {
    RtCharset {
        foreground,
        background,
        buffered: Vec::new(),
    }
}

pub fn RtCharsetDestroy(_charset: RtCharset) -> bool {
    true
}

pub fn RtCharsetPrint(charset: &mut RtCharset, string: impl Into<String>, x: i32, y: i32) {
    RtCharsetPrintBuffered(charset, string, x, y, true);
}

pub fn RtCharsetPrintBuffered(
    charset: &mut RtCharset,
    string: impl Into<String>,
    x: i32,
    y: i32,
    hide_spaces: bool,
) {
    charset.buffered.push(RtCharsetPrintCommand {
        string: string.into(),
        x,
        y,
        hide_spaces,
    });
}

pub fn RtCharsetBufferFlush(charset: &mut RtCharset) -> bool {
    charset.buffered.clear();
    true
}

pub fn RtCharsetSetColors(charset: &mut RtCharset, foreground: RwRGBA, background: RwRGBA) {
    charset.foreground = foreground;
    charset.background = background;
}

pub fn RtCharsetGetDesc(_charset: &RtCharset) -> RtCharsetDesc {
    RtCharsetDesc {
        width: 8,
        height: 16,
    }
}
