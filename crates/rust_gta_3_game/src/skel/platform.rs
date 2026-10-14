//! re3 reference: references/re-GTA/src/skel/platform.h
//!
//! Platform skeleton facade. re3 keeps the same signatures across Win32/GLFW
//! backends; this module keeps that contract while the Bevy app owns the
//! actual window, renderer, filesystem, and input devices.

use bevy::prelude::Resource;

use crate::fakerw::rwcore::{RwCamera, RwImage};
use crate::fakerw::rwplcore::{RwBool, RwInt32, RwUInt32, RwV2d};
use crate::skel::skeleton::RsKeyCodes;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct PsPlatformState {
    pub initialised: RwBool,
    pub terminated: RwBool,
    pub selected_device: RwInt32,
    pub selected_video_mode: RwInt32,
    pub mouse_pos: RwV2d,
    pub native_texture_support: RwBool,
    pub language_initialised: RwBool,
    pub exit_requested: RwBool,
}

impl Default for PsPlatformState {
    fn default() -> Self {
        Self {
            initialised: false,
            terminated: false,
            selected_device: 0,
            selected_video_mode: 0,
            mouse_pos: RwV2d::default(),
            native_texture_support: true,
            language_initialised: false,
            exit_requested: false,
        }
    }
}

#[allow(non_snake_case)]
pub fn psTimer() -> RwUInt32 {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis() as RwUInt32
}

#[allow(non_snake_case)]
pub fn psInitialize(state: &mut PsPlatformState) -> RwBool {
    state.initialised = true;
    state.terminated = false;
    true
}

#[allow(non_snake_case)]
pub fn psTerminate(state: &mut PsPlatformState) {
    state.terminated = true;
    state.initialised = false;
}

#[allow(non_snake_case)]
pub fn psCameraShowRaster(_camera: &RwCamera) {}

#[allow(non_snake_case)]
pub fn psCameraBeginUpdate(_camera: &mut RwCamera) -> RwBool {
    true
}

#[allow(non_snake_case)]
pub fn psGrabScreen(_camera: &RwCamera) -> Option<RwImage> {
    None
}

#[allow(non_snake_case)]
pub fn psMouseSetPos(state: &mut PsPlatformState, pos: RwV2d) {
    state.mouse_pos = pos;
}

#[allow(non_snake_case)]
pub fn psSelectDevice(state: &mut PsPlatformState) -> RwBool {
    state.selected_device = 0;
    true
}

#[allow(non_snake_case)]
pub fn psInstallFileSystem() -> RwBool {
    true
}

#[allow(non_snake_case)]
pub fn psNativeTextureSupport(state: &PsPlatformState) -> RwBool {
    state.native_texture_support
}

#[allow(non_snake_case)]
pub fn _InputTranslateShiftKeyUpDown(_rs: &mut RsKeyCodes) {}

#[allow(non_snake_case)]
pub fn _InputInitialiseMouse() -> i64 {
    0
}

#[allow(non_snake_case)]
pub fn _InputInitialiseJoys() {}

#[allow(non_snake_case)]
pub fn HandleExit(state: &mut PsPlatformState) {
    state.exit_requested = true;
}

#[allow(non_snake_case)]
pub fn _psSelectScreenVM(state: &mut PsPlatformState, video_mode: RwInt32) {
    state.selected_video_mode = video_mode;
}

#[allow(non_snake_case)]
pub fn InitialiseLanguage(state: &mut PsPlatformState) {
    state.language_initialised = true;
}

#[allow(non_snake_case)]
pub fn _psSetVideoMode(
    state: &mut PsPlatformState,
    sub_system: RwInt32,
    video_mode: RwInt32,
) -> RwBool {
    state.selected_device = sub_system;
    state.selected_video_mode = video_mode;
    true
}

#[allow(non_snake_case)]
pub fn _psGetVideoModeList() -> Vec<String> {
    // A platform facade cannot query Bevy's ECS window entity directly.
    // `FrontendFlow::sync_frontend_window_settings` exposes the native render
    // target to the menu. Do not fabricate a "Bevy window" mode here.
    Vec::new()
}

#[allow(non_snake_case)]
pub fn _psGetNumVideModes() -> RwInt32 {
    _psGetVideoModeList().len() as RwInt32
}
