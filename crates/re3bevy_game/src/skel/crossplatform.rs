//! re3 reference: references/re-GTA/src/skel/crossplatform.cpp
//! re3 reference: references/re-GTA/src/skel/crossplatform.h
//!
//! Cross-platform compatibility helpers used by the original skeleton.

use std::path::{Path, PathBuf};

use crate::fakerw::rwplcore::{RwBool, RwUInt16, RwUInt32};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum eWinVersion {
    OS_WIN95 = 0,
    OS_WIN98,
    OS_WINNT,
    OS_WIN2000,
    OS_WINXP,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum eGameState {
    GS_START_UP = 0,
    GS_INIT_LOGO_MPEG,
    GS_LOGO_MPEG,
    GS_INIT_INTRO_MPEG,
    GS_INTRO_MPEG,
    GS_INIT_ONCE,
    GS_INIT_FRONTEND,
    GS_FRONTEND,
    GS_INIT_PLAYING_GAME,
    GS_PLAYING_GAME,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct SYSTEMTIME {
    pub wYear: RwUInt16,
    pub wMonth: RwUInt16,
    pub wDayOfWeek: RwUInt16,
    pub wDay: RwUInt16,
    pub wHour: RwUInt16,
    pub wMinute: RwUInt16,
    pub wSecond: RwUInt16,
    pub wMilliseconds: RwUInt16,
}

pub const MAX_PATH: usize = 260;
pub const LANG_OTHER: i32 = 0;
pub const LANG_GERMAN: i32 = 1;
pub const LANG_FRENCH: i32 = 2;
pub const LANG_ENGLISH: i32 = 3;
pub const LANG_ITALIAN: i32 = 4;
pub const LANG_SPANISH: i32 = 5;
pub const SUBLANG_OTHER: i32 = 0;
pub const SUBLANG_ENGLISH_AUS: i32 = 1;

#[allow(non_upper_case_globals)]
pub const _dwOperatingSystemVersion: RwUInt32 = eWinVersion::OS_WINXP as RwUInt32;

#[allow(non_upper_case_globals)]
pub const gGameState: eGameState = eGameState::GS_START_UP;

#[allow(non_snake_case)]
pub fn strupr(input: &str) -> String {
    input.to_ascii_uppercase()
}

#[allow(non_snake_case)]
pub fn strlwr(input: &str) -> String {
    input.to_ascii_lowercase()
}

pub fn casepath(path: impl AsRef<Path>, check_path_first: bool) -> PathBuf {
    let path = path.as_ref();
    if !check_path_first || path.exists() {
        return path.to_path_buf();
    }

    let Some(parent) = path.parent() else {
        return path.to_path_buf();
    };
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path.to_path_buf();
    };

    let Ok(entries) = std::fs::read_dir(parent) else {
        return path.to_path_buf();
    };

    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(file_name)
        {
            return entry.path();
        }
    }

    path.to_path_buf()
}

pub fn caserename(
    old_filename: impl AsRef<Path>,
    new_filename: impl AsRef<Path>,
) -> std::io::Result<()> {
    std::fs::rename(casepath(old_filename, true), new_filename)
}

#[allow(non_snake_case)]
pub fn GetLocalTime_CP() -> SYSTEMTIME {
    SYSTEMTIME::default()
}

#[allow(non_snake_case)]
pub fn IsForegroundApp() -> RwBool {
    true
}
