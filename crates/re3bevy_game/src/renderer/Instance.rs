//! re3 reference: references/re-GTA/src/renderer/Instance.cpp
//! re3 reference: references/re-GTA/src/renderer/Instance.h
//!
//! Unused `CInstance` placeable wrapper.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CInstance {
    pub model_index: i32,
    pub matrix_attached: bool,
}

impl CInstance {
    pub fn shutdown(&mut self) {
        self.matrix_attached = false;
    }
}
