//! re3 reference: references/re-GTA/src/animation/AnimBlendNode.cpp
//! re3 reference: references/re-GTA/src/animation/AnimBlendNode.h
//!
//! Interpolated state between two key frames in a sequence.

use crate::{
    animation::AnimBlendSequence::CAnimBlendSequence,
    math::{Quaternion::CQuaternion, Vector::CVector},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimNodeUpdateContext {
    pub running: bool,
    pub repeating: bool,
    pub partial: bool,
    pub blend_amount: f32,
    pub time_step: f32,
}

impl AnimNodeUpdateContext {
    pub fn get_blend_amount(self, weight: f32) -> f32 {
        if self.partial {
            self.blend_amount
        } else {
            self.blend_amount * weight
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimNodeUpdateResult {
    pub looped: bool,
    pub translation: CVector,
    pub rotation: CQuaternion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CAnimBlendNode {
    pub theta: f32,
    pub inv_sin: f32,
    pub frame_a: i32,
    pub frame_b: i32,
    pub remaining_time: f32,
    pub sequence_index: Option<usize>,
}

impl CAnimBlendNode {
    pub fn new() -> Self {
        Self {
            theta: 0.0,
            inv_sin: 0.0,
            frame_a: -1,
            frame_b: -1,
            remaining_time: 0.0,
            sequence_index: None,
        }
    }

    pub fn init(&mut self) {
        *self = Self::new();
    }

    pub fn update(
        &mut self,
        sequence: &CAnimBlendSequence,
        ctx: AnimNodeUpdateContext,
        weight: f32,
    ) -> AnimNodeUpdateResult {
        let mut looped = false;
        let mut translation = CVector::ZERO;
        let mut rotation = CQuaternion::ZERO;

        if ctx.running {
            self.remaining_time -= ctx.time_step;
            if self.remaining_time <= 0.0 {
                looped = self.next_key_frame(sequence, ctx.repeating);
            }
        }

        let blend = ctx.get_blend_amount(weight);
        if blend > 0.0 && self.frame_a >= 0 && self.frame_b >= 0 {
            if let (Some(kf_a), Some(kf_b)) = (
                sequence.get_key_frame(self.frame_a as usize),
                sequence.get_key_frame(self.frame_b as usize),
            ) {
                let delta = kf_a.delta_time();
                let t = if delta == 0.0 {
                    0.0
                } else {
                    (delta - self.remaining_time) / delta
                };
                if sequence.has_translation() {
                    translation =
                        kf_b.translation() + t * (kf_a.translation() - kf_b.translation());
                    translation *= blend;
                }
                if sequence.type_flags & crate::animation::AnimBlendSequence::KF_ROT != 0 {
                    rotation = CQuaternion::slerp(
                        kf_b.rotation(),
                        kf_a.rotation(),
                        self.theta,
                        self.inv_sin,
                        t,
                    );
                    rotation *= blend;
                }
            }
        }

        AnimNodeUpdateResult {
            looped,
            translation,
            rotation,
        }
    }

    pub fn next_key_frame(&mut self, sequence: &CAnimBlendSequence, repeating: bool) -> bool {
        let num_frames = sequence.num_frames() as i32;
        if num_frames <= 1 {
            return false;
        }

        let mut looped = false;
        self.frame_b = self.frame_a;
        while self.remaining_time <= 0.0 {
            self.frame_a += 1;
            if self.frame_a >= num_frames {
                if !repeating {
                    self.frame_a -= 1;
                    self.remaining_time = 0.0;
                    return false;
                }
                looped = true;
                self.frame_a = 0;
            }

            if let Some(frame) = sequence.get_key_frame(self.frame_a as usize) {
                self.remaining_time += frame.delta_time();
            }
        }

        self.frame_b = self.frame_a - 1;
        if self.frame_b < 0 {
            self.frame_b += num_frames;
        }
        self.calc_deltas(sequence);
        looped
    }

    pub fn find_key_frame(
        &mut self,
        sequence: &CAnimBlendSequence,
        mut time: f32,
        repeating: bool,
    ) -> bool {
        if sequence.num_frames() < 1 {
            return false;
        }

        self.frame_a = 0;
        self.frame_b = self.frame_a;

        if sequence.num_frames() >= 2 {
            self.frame_a += 1;
            while sequence
                .get_key_frame(self.frame_a as usize)
                .map(|frame| time > frame.delta_time())
                .unwrap_or(false)
            {
                if let Some(frame) = sequence.get_key_frame(self.frame_a as usize) {
                    time -= frame.delta_time();
                }
                self.frame_b = self.frame_a;
                self.frame_a += 1;
                if self.frame_a >= sequence.num_frames() as i32 {
                    if !repeating {
                        return false;
                    }
                    self.frame_a = 0;
                    self.frame_b = 0;
                }
            }

            if let Some(frame) = sequence.get_key_frame(self.frame_a as usize) {
                self.remaining_time = frame.delta_time() - time;
            }
        }

        self.calc_deltas(sequence);
        true
    }

    pub fn calc_deltas(&mut self, sequence: &CAnimBlendSequence) {
        if sequence.type_flags & crate::animation::AnimBlendSequence::KF_ROT == 0 {
            return;
        }
        let Some(kf_a) = sequence.get_key_frame(self.frame_a.max(0) as usize) else {
            return;
        };
        let Some(kf_b) = sequence.get_key_frame(self.frame_b.max(0) as usize) else {
            return;
        };
        let cos = kf_a.rotation().dot(kf_b.rotation()).min(1.0);
        self.theta = cos.acos();
        self.inv_sin = if self.theta == 0.0 {
            0.0
        } else {
            1.0 / self.theta.sin()
        };
    }

    pub fn get_current_translation(
        &self,
        sequence: &CAnimBlendSequence,
        ctx: AnimNodeUpdateContext,
        weight: f32,
    ) -> CVector {
        let blend = ctx.get_blend_amount(weight);
        if blend <= 0.0 || self.frame_a < 0 || self.frame_b < 0 || !sequence.has_translation() {
            return CVector::ZERO;
        }

        let Some(kf_a) = sequence.get_key_frame(self.frame_a as usize) else {
            return CVector::ZERO;
        };
        let Some(kf_b) = sequence.get_key_frame(self.frame_b as usize) else {
            return CVector::ZERO;
        };
        let t = (kf_a.delta_time() - self.remaining_time) / kf_a.delta_time();
        (kf_b.translation() + t * (kf_a.translation() - kf_b.translation())) * blend
    }

    pub fn get_end_translation(
        &self,
        sequence: &CAnimBlendSequence,
        ctx: AnimNodeUpdateContext,
        weight: f32,
    ) -> CVector {
        let blend = ctx.get_blend_amount(weight);
        if blend <= 0.0 || !sequence.has_translation() || sequence.num_frames() == 0 {
            return CVector::ZERO;
        }
        sequence
            .get_key_frame(sequence.num_frames() - 1)
            .map(|frame| frame.translation() * blend)
            .unwrap_or(CVector::ZERO)
    }
}

impl Default for CAnimBlendNode {
    fn default() -> Self {
        Self::new()
    }
}
