//! re3 reference: references/re-GTA/src/animation/AnimManager.cpp
//! re3 reference: references/re-GTA/src/animation/AnimManager.h
//!
//! Animation asset files loaded by `CAnimManager`.

use std::io;

use bevy::prelude::Resource;
use rust_gta_3_formats::formats::{AnimManager::IfpFile, Directory::CDirectory};

use crate::install::FileMgr::{CFileMgr, CSeekWhence};

#[derive(Debug, Clone, Resource)]
pub struct CAnimationAssets {
    pub ped_ifp: LoadedIfpFile,
}

#[derive(Debug, Clone)]
pub struct LoadedIfpFile {
    pub filename: String,
    pub file: IfpFile,
}

#[derive(Debug, Clone)]
pub struct CutsceneArchiveEntry {
    pub filename: String,
    pub offset_sectors: u32,
    pub size_sectors: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CutsceneArchiveListing {
    pub filename: String,
    pub offset_sectors: u32,
    pub size_sectors: u32,
}

#[derive(Debug, Clone)]
pub struct LoadedCutsceneAssets {
    pub name: String,
    pub directory_entry_count: usize,
    pub animation_entry: Option<CutsceneArchiveEntry>,
    pub animation_file: Option<IfpFile>,
    pub camera_entry: Option<CutsceneArchiveEntry>,
}

impl CAnimationAssets {
    pub const PED_IFP: &'static str = "ANIM\\PED.IFP";
    pub const CUTSCENE_DIR: &'static str = "ANIM\\CUTS.DIR";
    pub const CUTSCENE_IMG: &'static str = "ANIM\\CUTS.IMG";

    pub fn load_anim_files(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        Ok(Self {
            ped_ifp: load_ifp(file_mgr, Self::PED_IFP)?,
        })
    }

    /// re3 `CCutsceneMgr::LoadCutsceneData` archive lookup and sector reads.
    pub fn load_cutscene(
        file_mgr: &mut CFileMgr,
        cutscene_name: &str,
    ) -> io::Result<LoadedCutsceneAssets> {
        let directory_bytes = file_mgr.load_file(Self::CUTSCENE_DIR, 512 * 32 + 1, "rb")?;
        let directory = CDirectory::read_from(directory_bytes.as_slice(), 512)?;
        let image_fd = file_mgr.open_file(Self::CUTSCENE_IMG, "rb")?;

        let animation_name = format!("{cutscene_name}.IFP");
        let camera_name = format!("{cutscene_name}.DAT");
        let loaded = (|| {
            let animation_entry =
                load_cutscene_archive_entry(file_mgr, image_fd, &directory, &animation_name)?;
            let animation_file = animation_entry
                .as_ref()
                .map(|entry| {
                    IfpFile::parse_with_error(&entry.bytes).map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("failed to parse {}: {error}", entry.filename),
                        )
                    })
                })
                .transpose()?;
            let camera_entry =
                load_cutscene_archive_entry(file_mgr, image_fd, &directory, &camera_name)?;
            Ok(LoadedCutsceneAssets {
                name: cutscene_name.to_owned(),
                directory_entry_count: directory.len(),
                animation_entry,
                animation_file,
                camera_entry,
            })
        })();
        let close_result = file_mgr.close_file(image_fd);
        match (loaded, close_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        }
    }

    /// re3 `CDirectory::FindItem` + sector read for any CUTS.IMG member.
    pub fn load_cutscene_archive_file(
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<(usize, Option<CutsceneArchiveEntry>)> {
        let directory_bytes = file_mgr.load_file(Self::CUTSCENE_DIR, 512 * 32 + 1, "rb")?;
        let directory = CDirectory::read_from(directory_bytes.as_slice(), 512)?;
        let image_fd = file_mgr.open_file(Self::CUTSCENE_IMG, "rb")?;
        let loaded = load_cutscene_archive_entry(file_mgr, image_fd, &directory, filename);
        let close_result = file_mgr.close_file(image_fd);
        match (loaded, close_result) {
            (Ok(entry), Ok(())) => Ok((directory.len(), entry)),
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        }
    }

    /// re3 `CDirectory::ReadDirFile` view of `ANIM\CUTS.DIR`.
    pub fn list_cutscene_archive(
        file_mgr: &mut CFileMgr,
    ) -> io::Result<Vec<CutsceneArchiveListing>> {
        let directory_bytes = file_mgr.load_file(Self::CUTSCENE_DIR, 512 * 32 + 1, "rb")?;
        let directory = CDirectory::read_from(directory_bytes.as_slice(), 512)?;
        Ok(directory
            .entries()
            .iter()
            .map(|entry| CutsceneArchiveListing {
                filename: entry.name_string(),
                offset_sectors: entry.offset,
                size_sectors: entry.size,
            })
            .collect())
    }

    /// Read one physical `CUTS.IMG` range described by `CDirectory::ReadDirFile`.
    ///
    /// Unlike `load_cutscene_archive_file`, this intentionally does not run a
    /// name lookup: CUTS.DIR contains duplicate names with distinct sector ranges.
    pub fn load_cutscene_archive_listing(
        file_mgr: &mut CFileMgr,
        listing: &CutsceneArchiveListing,
    ) -> io::Result<CutsceneArchiveEntry> {
        let image_fd = file_mgr.open_file(Self::CUTSCENE_IMG, "rb")?;
        let loaded = load_cutscene_archive_range(
            file_mgr,
            image_fd,
            &listing.filename,
            listing.offset_sectors,
            listing.size_sectors,
        );
        let close_result = file_mgr.close_file(image_fd);
        match (loaded, close_result) {
            (Ok(entry), Ok(())) => Ok(entry),
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        }
    }
}

fn load_cutscene_archive_entry(
    file_mgr: &mut CFileMgr,
    image_fd: usize,
    directory: &CDirectory,
    filename: &str,
) -> io::Result<Option<CutsceneArchiveEntry>> {
    let Some((offset_sectors, size_sectors)) = directory.find_item(filename) else {
        return Ok(None);
    };
    load_cutscene_archive_range(file_mgr, image_fd, filename, offset_sectors, size_sectors)
        .map(Some)
}

fn load_cutscene_archive_range(
    file_mgr: &mut CFileMgr,
    image_fd: usize,
    filename: &str,
    offset_sectors: u32,
    size_sectors: u32,
) -> io::Result<CutsceneArchiveEntry> {
    let byte_offset = i64::from(offset_sectors) << 11;
    let byte_len = usize::try_from(size_sectors)
        .ok()
        .and_then(|size| size.checked_shl(11))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CUTS.IMG entry too large"))?;
    if file_mgr.seek(image_fd, byte_offset, CSeekWhence::Start)? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to seek CUTS.IMG for {filename}"),
        ));
    }
    let mut bytes = vec![0; byte_len];
    let mut read = 0;
    while read < bytes.len() {
        let amount = file_mgr.read(image_fd, &mut bytes[read..])?;
        if amount == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("truncated CUTS.IMG entry {filename}"),
            ));
        }
        read += amount;
    }
    Ok(CutsceneArchiveEntry {
        filename: filename.to_owned(),
        offset_sectors,
        size_sectors,
        bytes,
    })
}

fn load_ifp(file_mgr: &mut CFileMgr, filename: &str) -> io::Result<LoadedIfpFile> {
    let bytes = file_mgr.load_file(filename, 16 * 1024 * 1024, "rb")?;
    let file = IfpFile::parse_with_error(&bytes).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse {filename}: {err}"),
        )
    })?;
    Ok(LoadedIfpFile {
        filename: filename.to_owned(),
        file,
    })
}
