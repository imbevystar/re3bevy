//! re3 reference: references/re-GTA/src/skel/win/win.cpp
//! re3 reference: references/re-GTA/src/skel/win/win.h
//!
//! Win32 skeleton backend descriptor. Native window ownership stays with Bevy.

use crate::fakerw::rwplcore::RwBool;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WinBackendState {
    pub window_class_registered: RwBool,
    pub device_selection_dialog_open: RwBool,
    pub active: RwBool,
}

impl Default for WinBackendState {
    fn default() -> Self {
        Self {
            window_class_registered: false,
            device_selection_dialog_open: false,
            active: false,
        }
    }
}

#[allow(non_snake_case)]
pub fn InitApplication(state: &mut WinBackendState) -> RwBool {
    state.window_class_registered = true;
    true
}

#[allow(non_snake_case)]
pub fn InitInstance(state: &mut WinBackendState) -> RwBool {
    state.active = true;
    true
}

#[allow(non_snake_case)]
pub fn TerminateInstance(state: &mut WinBackendState) {
    state.active = false;
    state.device_selection_dialog_open = false;
}

#[allow(non_snake_case)]
pub fn SelectDeviceDialog(state: &mut WinBackendState) -> RwBool {
    state.device_selection_dialog_open = true;
    true
}
