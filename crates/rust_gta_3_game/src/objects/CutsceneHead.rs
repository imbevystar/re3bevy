//! re3 reference: references/re-GTA/src/objects/CutsceneHead.cpp
//! re3 reference: references/re-GTA/src/objects/CutsceneHead.h
//!
//! Cutscene head state.

use rust_gta_3_core::core::General::CGeneral;

use crate::objects::{CutsceneObject::CCutsceneObject, Object::CObject};

#[derive(Debug, Clone, PartialEq)]
pub struct CCutsceneHead {
    pub cutscene_object: CCutsceneObject,
    pub head_node_loaded: bool,
    pub parent_object: Option<usize>,
    pub is_skinned: bool,
    pub last_animation_name: Option<String>,
}

impl CCutsceneHead {
    pub fn new(parent: &mut CObject, general: &mut CGeneral) -> Self {
        let mut cutscene_object = CCutsceneObject::new(general);
        cutscene_object.object.physical.entity.model_index = parent.physical.entity.model_index;
        Self {
            cutscene_object,
            head_node_loaded: parent.physical.entity.rw_object_loaded,
            parent_object: None,
            is_skinned: false,
            last_animation_name: None,
        }
    }

    pub fn play_animation(&mut self, anim_name: &str) {
        self.last_animation_name = Some(anim_name.to_string());
    }
}
