//! re3 reference: references/re-GTA/src/text/Text.cpp
//! re3 reference: references/re-GTA/src/text/Text.h
//!
//! `CText` keeps the loaded GXT key table and returns GTA wide strings.

use std::collections::BTreeMap;

use bevy::prelude::*;
use re3bevy_assets::assets::FrontendAssets::CFrontendAssets;

pub type GtaWideString = Vec<u16>;

#[derive(Debug, Clone, Resource)]
pub struct CText {
    pub encoding: u8,
    pub entries: BTreeMap<String, GtaWideString>,
}

impl CText {
    pub fn from_frontend_assets(assets: &CFrontendAssets) -> Self {
        Self {
            encoding: 0,
            entries: assets.text.strings(),
        }
    }

    pub fn unload(&mut self) {
        self.entries.clear();
    }

    pub fn get(&self, key: &str) -> GtaWideString {
        self.entries
            .get(key)
            .cloned()
            .unwrap_or_else(|| ascii_to_unicode(key))
    }

    pub fn get_key_bytes(&self, key: &[u8]) -> GtaWideString {
        let end = key.iter().position(|byte| *byte == 0).unwrap_or(key.len());
        let key = String::from_utf8_lossy(&key[..end]);
        self.get(key.as_ref())
    }

    pub fn get_upper_case(&self, c: u16) -> u16 {
        const LOWER_A: u16 = b'a' as u16;
        const LOWER_Z: u16 = b'z' as u16;
        if (LOWER_A..=LOWER_Z).contains(&c) || ((0x00E0..=0x00FE).contains(&c) && c != 0x00F7) {
            c - 32
        } else {
            c
        }
    }

    pub fn upper_case(&self, text: &mut [u16]) {
        for ch in text {
            *ch = self.get_upper_case(*ch);
        }
    }
}

pub fn ascii_to_unicode(src: &str) -> GtaWideString {
    src.bytes().map(u16::from).collect()
}

pub fn unicode_to_ascii(src: &[u16]) -> String {
    src.iter()
        .take_while(|ch| **ch != 0)
        .map(|ch| char::from_u32((*ch).into()).unwrap_or('?'))
        .collect()
}

pub fn text_copy(dst: &mut GtaWideString, src: &[u16]) {
    dst.clear();
    dst.extend_from_slice(src);
}
