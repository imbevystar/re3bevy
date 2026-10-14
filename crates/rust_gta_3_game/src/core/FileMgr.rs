//! re3 reference: references/re-GTA/src/core/FileMgr.cpp
//! re3 reference: references/re-GTA/src/core/FileMgr.h
//!
//! File manager path helpers.

use std::path::{Path, PathBuf};

use bevy::prelude::Resource;

#[derive(Debug, Clone, Eq, PartialEq, Resource)]
pub struct CFileMgr {
    pub root: PathBuf,
    pub current_dir: PathBuf,
}

impl Default for CFileMgr {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            current_dir: PathBuf::new(),
        }
    }
}

impl CFileMgr {
    #[allow(non_snake_case)]
    pub fn Initialise(&mut self, root: impl Into<PathBuf>) {
        self.root = root.into();
        self.current_dir = self.root.clone();
    }

    #[allow(non_snake_case)]
    pub fn SetDir(&mut self, path: impl AsRef<Path>) {
        self.current_dir = self.root.join(path);
    }

    #[allow(non_snake_case)]
    pub fn GetFullPath(&self, path: impl AsRef<Path>) -> PathBuf {
        self.current_dir.join(path)
    }
}
