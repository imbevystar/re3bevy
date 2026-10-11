//! re3 reference: references/re-GTA/src/skel/glfw/glfw.cpp
//!
//! GLFW-style skeleton backend descriptor for non-Win32 builds. Bevy/winit is
//! the actual window backend, but these fields match the re3 skeleton state.

use crate::fakerw::rwplcore::{RwBool, RwInt8, RwV2d};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct psGlobalType {
    pub fullScreen: RwBool,
    pub lastMousePos: RwV2d,
    pub mouseWheel: f64,
    pub cursorIsInWindow: bool,
    pub joy1id: RwInt8,
    pub joy2id: RwInt8,
}

impl Default for psGlobalType {
    fn default() -> Self {
        Self {
            fullScreen: false,
            lastMousePos: RwV2d::default(),
            mouseWheel: 0.0,
            cursorIsInWindow: true,
            joy1id: -1,
            joy2id: -1,
        }
    }
}

#[allow(non_snake_case)]
pub fn CapturePad(_padID: i32) {}

#[allow(non_snake_case)]
pub fn joysChangeCB(state: &mut psGlobalType, jid: i32, event: i32) {
    let slot = if state.joy1id < 0 {
        &mut state.joy1id
    } else {
        &mut state.joy2id
    };
    *slot = if event == 0 { -1 } else { jid as RwInt8 };
}
