//! re3 reference: references/re-GTA/src/objects/CutsceneObject.cpp
//! re3 reference: references/re-GTA/src/objects/CutsceneObject.h
//!
//! Cutscene object state.

use rustgta3_core::core::General::CGeneral;

use crate::{
    entities::Entity::eEntityStatus,
    objects::Object::{CObject, CUTSCENE_OBJECT},
};

#[derive(Debug, Clone, PartialEq)]
pub struct CCutsceneObject {
    pub object: CObject,
    pub render_head: bool,
    pub render_right_hand: bool,
    pub render_left_hand: bool,
}

impl CCutsceneObject {
    pub fn new(general: &mut CGeneral) -> Self {
        let mut object = CObject::new(general);
        object
            .physical
            .entity
            .set_status(eEntityStatus::STATUS_SIMPLE);
        object.physical.entity.flags.uses_collision = false;
        object.physical.entity.flags.streaming_dont_delete = true;
        object.object_created_by = CUTSCENE_OBJECT;
        object.physical.mass = 1.0;
        object.physical.turn_mass = 1.0;
        Self {
            object,
            render_head: true,
            render_right_hand: true,
            render_left_hand: true,
        }
    }

    pub fn set_render_head(&mut self, render: bool) {
        self.render_head = render;
    }

    pub fn set_render_right_hand(&mut self, render: bool) {
        self.render_right_hand = render;
    }

    pub fn set_render_left_hand(&mut self, render: bool) {
        self.render_left_hand = render;
    }
}
