//! re3 reference: references/re-GTA/src/rw/TxdStore.cpp
//! re3 reference: references/re-GTA/src/rw/TxdStore.h
//!
//! Asset-side texture dictionary loading helpers. Runtime slot/ref-count state
//! lives in `rustgta3_game::rw::TxdStore`; this crate resolves original files
//! and parses TXD summaries without owning gameplay state.

use std::io;

use bevy::prelude::Resource;
use rustgta3_formats::formats::TexRead::{
    RwDecodedTexture, RwTexDictionarySummary, decode_txd_textures, read_txd,
};

use crate::install::OriginalGtaInstall::OriginalGtaInstall;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CTxdStoreAsset {
    pub gta_path: String,
    pub summary: RwTexDictionarySummary,
    pub decoded_textures: Vec<RwDecodedTexture>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxdLoadRequest {
    pub slot: usize,
    pub gta_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Resource)]
pub struct TxdLoadQueue {
    pub requests: Vec<TxdLoadRequest>,
}

impl CTxdStoreAsset {
    pub fn load(install: &OriginalGtaInstall, gta_path: impl AsRef<str>) -> io::Result<Self> {
        let gta_path = gta_path.as_ref();
        let bytes = install.read(gta_path)?;
        let summary = read_txd(&bytes).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse TXD `{gta_path}`"),
            )
        })?;
        let decoded_textures = decode_txd_textures(&summary);
        Ok(Self {
            gta_path: gta_path.to_owned(),
            summary,
            decoded_textures,
        })
    }
}

impl TxdLoadQueue {
    pub fn request(&mut self, slot: usize, gta_path: impl Into<String>) {
        let gta_path = gta_path.into();
        if self
            .requests
            .iter()
            .any(|request| request.slot == slot && request.gta_path.eq_ignore_ascii_case(&gta_path))
        {
            return;
        }
        self.requests.push(TxdLoadRequest { slot, gta_path });
    }

    pub fn drain(&mut self) -> Vec<TxdLoadRequest> {
        std::mem::take(&mut self.requests)
    }
}
