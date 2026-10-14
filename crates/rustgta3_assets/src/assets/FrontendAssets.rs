//! re3 reference: references/re-GTA/src/text/Text.cpp
//! re3 reference: references/re-GTA/src/text/Text.h
//!
//! Frontend text asset loaded by `CText::Load`.

use std::io;

use bevy::prelude::Resource;
use rustgta3_formats::formats::Text::GxtFile;

use crate::install::FileMgr::CFileMgr;

#[derive(Debug, Clone, Resource)]
pub struct CFrontendAssets {
    pub language_file: String,
    pub text: GxtFile,
}

impl CFrontendAssets {
    pub const DEFAULT_LANGUAGE_FILE: &'static str = "TEXT\\AMERICAN.GXT";

    /// CText::Load chooses this file directly from CMenuManager::m_PrefsLanguage.
    /// Keep this table here so every frontend reload reads the original install,
    /// rather than embedding or copying a GTA text asset.
    pub fn language_file(language: i32) -> Option<&'static str> {
        match language {
            0 => Some("TEXT\\AMERICAN.GXT"),
            1 => Some("TEXT\\FRENCH.GXT"),
            2 => Some("TEXT\\GERMAN.GXT"),
            3 => Some("TEXT\\ITALIAN.GXT"),
            4 => Some("TEXT\\SPANISH.GXT"),
            _ => None,
        }
    }

    pub fn load_default_text(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        Self::load_language_text(file_mgr, 0)
    }

    pub fn load_language_text(file_mgr: &mut CFileMgr, language: i32) -> io::Result<Self> {
        let language_file = Self::language_file(language).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported GTA III language index {language}"),
            )
        })?;
        let bytes = file_mgr.load_file(language_file, usize::MAX, "rb")?;
        let text = GxtFile::parse(strip_file_mgr_terminator(&bytes)).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse {language_file}"),
            )
        })?;

        Ok(Self {
            language_file: language_file.to_owned(),
            text,
        })
    }
}

fn strip_file_mgr_terminator(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(&[0]).unwrap_or(bytes)
}
