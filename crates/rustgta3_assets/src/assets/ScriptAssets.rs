//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Asset-side loading for `data\main.scm`. Runtime SCM state and opcode
//! execution live in `rustgta3_game::control::Script`.

use std::io;

use bevy::prelude::Resource;
use rustgta3_formats::formats::Script::ScmFile;

use crate::install::FileMgr::CFileMgr;

#[derive(Debug, Clone, Resource)]
pub struct CScriptAssets {
    pub main_scm: LoadedScmFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedScmFile {
    pub filename: String,
    pub file: ScmFile,
    /// Complete on-disk MAIN.SCM, including mission scripts stored after the
    /// fixed re3 main-script segment.
    pub source_bytes: Vec<u8>,
}

impl CScriptAssets {
    pub const MAIN_SCM: &'static str = "DATA\\MAIN.SCM";

    pub fn load_main(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        Ok(Self {
            main_scm: load_scm(file_mgr, Self::MAIN_SCM)?,
        })
    }
}

fn load_scm(file_mgr: &mut CFileMgr, filename: &str) -> io::Result<LoadedScmFile> {
    let bytes = file_mgr.read_file_to_end(filename)?;
    let file = ScmFile::try_parse_main_scm(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(LoadedScmFile {
        filename: filename.to_owned(),
        file,
        source_bytes: bytes,
    })
}
