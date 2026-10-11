//! re3 reference: references/re-GTA/src/animation/AnimBlendClumpData.cpp
//! re3 reference: references/re-GTA/src/animation/AnimBlendClumpData.h
//!
//! RenderWare clump animation plugin data represented as Rust state.

use crate::{animation::AnimBlendAssociation::CAnimBlendAssociation, math::Vector::CVector};

pub const IGNORE_ROTATION: u8 = 2;
pub const IGNORE_TRANSLATION: u8 = 4;
pub const VELOCITY_EXTRACTION: u8 = 8;
pub const VELOCITY_EXTRACTION_3D: u8 = 0x10;

#[derive(Debug, Clone, PartialEq)]
pub struct AnimBlendFrameData {
    pub flag: u8,
    pub reset_pos: CVector,
    pub frame_name: String,
    pub node_id: Option<i32>,
}

impl Default for AnimBlendFrameData {
    fn default() -> Self {
        Self {
            flag: 0,
            reset_pos: CVector::ZERO,
            frame_name: String::new(),
            node_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CAnimBlendClumpData {
    pub associations: Vec<CAnimBlendAssociation>,
    pub num_frames: usize,
    pub velocity2d: Option<(f32, f32)>,
    pub velocity3d: Option<CVector>,
    pub frames: Vec<AnimBlendFrameData>,
}

impl CAnimBlendClumpData {
    pub fn new() -> Self {
        Self {
            associations: Vec::new(),
            num_frames: 0,
            velocity2d: None,
            velocity3d: None,
            frames: Vec::new(),
        }
    }

    pub fn set_number_of_frames(&mut self, count: usize) {
        self.num_frames = count;
        self.frames = vec![AnimBlendFrameData::default(); count];
    }

    pub fn for_all_frames<F: FnMut(&mut AnimBlendFrameData)>(&mut self, mut callback: F) {
        for frame in &mut self.frames {
            callback(frame);
        }
    }
}

impl Default for CAnimBlendClumpData {
    fn default() -> Self {
        Self::new()
    }
}
