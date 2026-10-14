//! re3 reference: references/re-GTA/src/modelinfo/ClumpModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/ClumpModelInfo.h
//!
//! C++ file-name facade for `CClumpModelInfo`.

pub use super::ModelInfo::{CClumpModelInfo, CMloModelInfo};

impl CClumpModelInfo {
    pub fn DeleteRwObject(&mut self) {
        self.base.delete_rw_object();
    }

    pub fn GetRwObjectLoaded(&self) -> bool {
        self.base.rw_object_loaded
    }

    pub fn SetClumpLoaded(&mut self, loaded: bool) {
        if self.base.rw_object_loaded != loaded {
            self.base.advance_rw_object_revision();
        }
        self.base.rw_object_loaded = loaded;
    }

    pub fn SetMlo(&mut self, mlo: CMloModelInfo) {
        self.mlo = Some(mlo);
    }

    pub fn GetMlo(&self) -> Option<&CMloModelInfo> {
        self.mlo.as_ref()
    }
}

pub const CLUMP_FLAG_NO_HIERID: u32 = 0x1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RwObjectNameIdAssocation {
    pub name: &'static str,
    pub hier_id: i32,
    pub flags: u32,
}
