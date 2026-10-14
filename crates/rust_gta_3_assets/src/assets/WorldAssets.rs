//! re3 reference: references/re-GTA/src/vehicles/HandlingMgr.cpp
//! re3 reference: references/re-GTA/src/vehicles/HandlingMgr.h
//! re3 reference: references/re-GTA/src/core/SurfaceTable.cpp
//! re3 reference: references/re-GTA/src/core/SurfaceTable.h
//! re3 reference: references/re-GTA/src/weapons/WeaponInfo.cpp
//! re3 reference: references/re-GTA/src/weapons/WeaponInfo.h
//!
//! DATA table assets loaded during `CGame::InitialiseOnceAfterRW` and `CGame::Initialise`.

use std::io;

use bevy::prelude::Resource;
use rust_gta_3_formats::formats::DataRead::{HandlingCfg, SurfaceDat, WeaponDat};

use crate::install::FileMgr::CFileMgr;

#[derive(Debug, Clone, Resource)]
pub struct CHandlingDataAsset {
    pub filename: String,
    pub data: HandlingCfg,
}

#[derive(Debug, Clone, Resource)]
pub struct CSurfaceTableAsset {
    pub filename: String,
    pub data: SurfaceDat,
}

#[derive(Debug, Clone, Resource)]
pub struct CWeaponDataAsset {
    pub filename: String,
    pub data: WeaponDat,
}

impl CHandlingDataAsset {
    pub const FILENAME: &'static str = "DATA\\HANDLING.CFG";

    pub fn load(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let text = load_text_file(file_mgr, Self::FILENAME)?;
        Ok(Self {
            filename: Self::FILENAME.to_owned(),
            data: HandlingCfg::parse(&text),
        })
    }
}

impl CSurfaceTableAsset {
    pub const FILENAME: &'static str = "DATA\\SURFACE.DAT";

    pub fn load(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let text = load_text_file(file_mgr, Self::FILENAME)?;
        Ok(Self {
            filename: Self::FILENAME.to_owned(),
            data: SurfaceDat::parse(&text)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
        })
    }
}

impl CWeaponDataAsset {
    pub const FILENAME: &'static str = "DATA\\WEAPON.DAT";

    pub fn load(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let text = load_text_file(file_mgr, Self::FILENAME)?;
        Ok(Self {
            filename: Self::FILENAME.to_owned(),
            data: WeaponDat::parse(&text),
        })
    }
}

fn load_text_file(file_mgr: &mut CFileMgr, filename: &str) -> io::Result<String> {
    let bytes = file_mgr.load_file(filename, usize::MAX, "r")?;
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(bytes).into_owned())
}
