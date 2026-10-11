//! re3 reference: references/re-GTA/src/animation/AnimBlendAssociation.cpp
//! re3 reference: references/re-GTA/src/animation/AnimBlendAssociation.h
//!
//! Animation hierarchy associated with one clump/template.

use crate::animation::{AnimBlendHierarchy::CAnimBlendHierarchy, AnimBlendNode::CAnimBlendNode};

pub const ASSOC_RUNNING: i32 = 1;
pub const ASSOC_REPEAT: i32 = 2;
pub const ASSOC_DELETEFADEDOUT: i32 = 4;
pub const ASSOC_FADEOUTWHENDONE: i32 = 8;
pub const ASSOC_PARTIAL: i32 = 0x10;
pub const ASSOC_MOVEMENT: i32 = 0x20;
pub const ASSOC_HAS_TRANSLATION: i32 = 0x40;
pub const ASSOC_WALK: i32 = 0x80;
pub const ASSOC_IDLE: i32 = 0x100;
pub const ASSOC_NOWALK: i32 = 0x200;
pub const ASSOC_BLOCK: i32 = 0x400;
pub const ASSOC_FRONTAL: i32 = 0x800;
pub const ASSOC_HAS_X_TRANSLATION: i32 = 0x1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimBlendCallbackType {
    None,
    Finish,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimBlendCallbackEvent {
    Finish,
    Delete,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CAnimBlendAssociation {
    pub num_nodes: usize,
    pub nodes: Vec<CAnimBlendNode>,
    pub hierarchy: CAnimBlendHierarchy,
    pub blend_amount: f32,
    pub blend_delta: f32,
    pub current_time: f32,
    pub speed: f32,
    pub time_step: f32,
    pub anim_id: i32,
    pub flags: i32,
    pub callback_type: AnimBlendCallbackType,
    pub callback_events: Vec<AnimBlendCallbackEvent>,
    pub deleted: bool,
}

impl CAnimBlendAssociation {
    pub fn new() -> Self {
        Self {
            num_nodes: 0,
            nodes: Vec::new(),
            hierarchy: CAnimBlendHierarchy::default(),
            blend_amount: 1.0,
            blend_delta: 0.0,
            current_time: 0.0,
            speed: 1.0,
            time_step: 0.0,
            anim_id: -1,
            flags: 0,
            callback_type: AnimBlendCallbackType::None,
            callback_events: Vec::new(),
            deleted: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.flags & ASSOC_RUNNING != 0
    }

    pub fn is_repeating(&self) -> bool {
        self.flags & ASSOC_REPEAT != 0
    }

    pub fn is_partial(&self) -> bool {
        self.flags & ASSOC_PARTIAL != 0
    }

    pub fn is_movement(&self) -> bool {
        self.flags & ASSOC_MOVEMENT != 0
    }

    pub fn has_translation(&self) -> bool {
        self.flags & ASSOC_HAS_TRANSLATION != 0
    }

    pub fn has_x_translation(&self) -> bool {
        self.flags & ASSOC_HAS_X_TRANSLATION != 0
    }

    pub fn get_blend_amount(&self, weight: f32) -> f32 {
        if self.is_partial() {
            self.blend_amount
        } else {
            self.blend_amount * weight
        }
    }

    pub fn get_time_left(&self) -> f32 {
        self.hierarchy.total_length - self.current_time
    }

    pub fn allocate_anim_blend_node_array(&mut self, count: usize) {
        self.nodes = (0..count).map(|_| CAnimBlendNode::new()).collect();
        self.num_nodes = count;
    }

    pub fn free_anim_blend_node_array(&mut self) {
        self.nodes.clear();
        self.num_nodes = 0;
    }

    pub fn init_with_hierarchy(&mut self, hierarchy: CAnimBlendHierarchy, num_nodes: usize) {
        self.hierarchy = hierarchy;
        self.allocate_anim_blend_node_array(num_nodes);
        for (sequence_index, sequence) in self.hierarchy.sequences.iter().enumerate() {
            if sequence.num_frames() > 0 && sequence_index < self.nodes.len() {
                self.nodes[sequence_index].sequence_index = Some(sequence_index);
            }
        }
    }

    pub fn init_from(&mut self, assoc: &CAnimBlendAssociation) {
        self.hierarchy = assoc.hierarchy.clone();
        self.num_nodes = assoc.num_nodes;
        self.flags = assoc.flags;
        self.anim_id = assoc.anim_id;
        self.nodes = assoc.nodes.clone();
    }

    pub fn set_blend(&mut self, amount: f32, delta: f32) {
        self.blend_amount = amount;
        self.blend_delta = delta;
    }

    pub fn set_finish_callback(&mut self) {
        self.callback_type = AnimBlendCallbackType::Finish;
    }

    pub fn set_delete_callback(&mut self) {
        self.callback_type = AnimBlendCallbackType::Delete;
    }

    pub fn set_current_time(&mut self, time: f32) {
        self.current_time = time;
        while self.current_time >= self.hierarchy.total_length && self.hierarchy.total_length > 0.0
        {
            self.current_time -= self.hierarchy.total_length;
            if !self.is_repeating() {
                return;
            }
        }
        self.hierarchy.uncompress();

        let repeating = self.is_repeating();
        for node in &mut self.nodes {
            if let Some(sequence_index) = node.sequence_index {
                if let Some(sequence) = self.hierarchy.sequences.get(sequence_index) {
                    node.find_key_frame(sequence, self.current_time, repeating);
                }
            }
        }
    }

    pub fn sync_animation(&mut self, other: &CAnimBlendAssociation) {
        if other.hierarchy.total_length != 0.0 {
            self.set_current_time(
                other.current_time / other.hierarchy.total_length * self.hierarchy.total_length,
            );
        }
    }

    pub fn start(&mut self, time: f32) {
        self.flags |= ASSOC_RUNNING;
        self.set_current_time(time);
    }

    pub fn set_run(&mut self) {
        self.flags |= ASSOC_RUNNING;
    }

    pub fn update_time(&mut self, time_delta: f32, rel_speed: f32) -> bool {
        if !self.is_running() {
            return true;
        }

        self.time_step = if self.flags & ASSOC_MOVEMENT != 0 {
            rel_speed * self.hierarchy.total_length
        } else {
            self.speed
        } * time_delta;
        self.current_time += self.time_step;

        if self.current_time >= self.hierarchy.total_length {
            if self.is_repeating() {
                self.current_time -= self.hierarchy.total_length;
            } else {
                self.current_time = self.hierarchy.total_length;
                self.flags &= !ASSOC_RUNNING;
                if self.flags & ASSOC_FADEOUTWHENDONE != 0 {
                    self.flags |= ASSOC_DELETEFADEDOUT;
                    self.blend_delta = -4.0;
                }
                if self.callback_type == AnimBlendCallbackType::Finish {
                    self.callback_type = AnimBlendCallbackType::None;
                    self.callback_events.push(AnimBlendCallbackEvent::Finish);
                }
            }
        }
        true
    }

    pub fn update_blend(&mut self, time_delta: f32) -> bool {
        self.blend_amount += self.blend_delta * time_delta;

        if self.blend_amount <= 0.0 && self.blend_delta < 0.0 {
            self.blend_amount = 0.0;
            self.blend_delta = self.blend_delta.max(0.0);
            if self.flags & ASSOC_DELETEFADEDOUT != 0 {
                if matches!(
                    self.callback_type,
                    AnimBlendCallbackType::Finish | AnimBlendCallbackType::Delete
                ) {
                    self.callback_events.push(
                        if self.callback_type == AnimBlendCallbackType::Delete {
                            AnimBlendCallbackEvent::Delete
                        } else {
                            AnimBlendCallbackEvent::Finish
                        },
                    );
                }
                self.deleted = true;
                return false;
            }
        }

        if self.blend_amount > 1.0 {
            self.blend_amount = 1.0;
            self.blend_delta = self.blend_delta.min(0.0);
        }

        true
    }
}

impl Default for CAnimBlendAssociation {
    fn default() -> Self {
        Self::new()
    }
}
