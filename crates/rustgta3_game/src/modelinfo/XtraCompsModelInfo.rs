//! re3 reference: references/re-GTA/src/modelinfo/XtraCompsModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/XtraCompsModelInfo.h
//!
//! Extra component clump model facade.

pub use super::ModelInfo::CClumpModelInfo as CXtraCompsModelInfo;

impl CXtraCompsModelInfo {
    pub fn IsExtraCompsLoaded(&self) -> bool {
        self.base.rw_object_loaded
    }
}
