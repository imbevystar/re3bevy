//! re3 reference: references/re-GTA/src/modelinfo/BaseModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/BaseModelInfo.h
//!
//! C++ file-name facade for `CBaseModelInfo`.

pub use super::ModelInfo::CBaseModelInfo;

impl CBaseModelInfo {
    pub fn Shutdown(&mut self) {
        self.delete_rw_object();
        self.col_model = None;
        self.ref_count = 0;
    }

    pub fn GetModelType(&self) -> u8 {
        self.model_type as u8
    }

    pub fn IsSimple(&self) -> bool {
        self.is_simple()
    }

    pub fn IsClump(&self) -> bool {
        self.model_type.is_clump()
    }

    pub fn GetModelName(&self) -> &str {
        &self.name
    }

    pub fn SetModelName(&mut self, name: &str) {
        self.set_model_name(name);
    }

    pub fn GetObjectID(&self) -> i16 {
        self.object_id
    }

    pub fn SetObjectID(&mut self, id: i16) {
        self.object_id = id;
    }

    pub fn GetTxdSlot(&self) -> i16 {
        self.txd_slot
    }

    pub fn ClearTexDictionary(&mut self) {
        self.txd_slot = -1;
    }

    pub fn GetNum2dEffects(&self) -> u8 {
        self.num_2d_effects
    }

    pub fn GetNumRefs(&self) -> u16 {
        self.ref_count
    }

    pub fn DoesOwnColModel(&self) -> bool {
        self.owns_col_model
    }
}
