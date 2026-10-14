//! re3 reference: references/re-GTA/src/animation/AnimBlendHierarchy.cpp
//! re3 reference: references/re-GTA/src/animation/AnimBlendHierarchy.h
//!
//! Collection of animation sequences.

use crate::animation::AnimBlendSequence::CAnimBlendSequence;

#[derive(Debug, Clone, PartialEq)]
pub struct CAnimBlendHierarchy {
    pub name: String,
    pub sequences: Vec<CAnimBlendSequence>,
    pub compressed: i16,
    pub total_length: f32,
    pub linked_in_cache: bool,
}

impl CAnimBlendHierarchy {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            sequences: Vec::new(),
            compressed: 0,
            total_length: 0.0,
            linked_in_cache: false,
        }
    }

    pub fn shutdown(&mut self) {
        self.remove_anim_sequences();
        self.compressed = 0;
        self.linked_in_cache = false;
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.chars().take(24).collect();
    }

    pub fn calc_total_time(&mut self) {
        self.total_length = 0.0;
        for sequence in &self.sequences {
            let seq_time = sequence
                .key_frames
                .iter()
                .map(|frame| frame.delta_time())
                .sum::<f32>();
            self.total_length = self.total_length.max(seq_time);
        }
    }

    pub fn remove_quaternion_flips(&mut self) {
        for sequence in &mut self.sequences {
            sequence.remove_quaternion_flips();
        }
    }

    pub fn remove_anim_sequences(&mut self) {
        self.sequences.clear();
    }

    pub fn uncompress(&mut self) {
        for sequence in &mut self.sequences {
            sequence.uncompress();
        }
        if self.total_length == 0.0 {
            self.calc_total_time();
        }
        self.compressed = 0;
    }

    pub fn remove_uncompressed_data(&mut self) {
        for sequence in &mut self.sequences {
            sequence.remove_uncompressed_data();
        }
        self.compressed = 1;
    }
}

impl Default for CAnimBlendHierarchy {
    fn default() -> Self {
        Self::new()
    }
}
