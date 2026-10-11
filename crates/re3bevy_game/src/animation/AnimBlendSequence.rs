//! re3 reference: references/re-GTA/src/animation/AnimBlendSequence.cpp
//! re3 reference: references/re-GTA/src/animation/AnimBlendSequence.h
//!
//! Keyframe sequence for one animated node.

use crate::{math::Quaternion::CQuaternion, math::Vector::CVector};

pub const KF_ROT: i32 = 1;
pub const KF_TRANS: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyFrame {
    pub rotation: CQuaternion,
    pub delta_time: f32,
}

impl Default for KeyFrame {
    fn default() -> Self {
        Self {
            rotation: CQuaternion::IDENTITY,
            delta_time: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyFrameTrans {
    pub rotation: CQuaternion,
    pub delta_time: f32,
    pub translation: CVector,
}

impl Default for KeyFrameTrans {
    fn default() -> Self {
        Self {
            rotation: CQuaternion::IDENTITY,
            delta_time: 0.0,
            translation: CVector::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyFrameCompressed {
    pub rot: [i16; 4],
    pub delta_time: i16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyFrameTransCompressed {
    pub rot: [i16; 4],
    pub delta_time: i16,
    pub trans: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimKeyFrame {
    Rot(KeyFrame),
    Trans(KeyFrameTrans),
}

impl AnimKeyFrame {
    pub fn rotation(self) -> CQuaternion {
        match self {
            Self::Rot(frame) => frame.rotation,
            Self::Trans(frame) => frame.rotation,
        }
    }

    pub fn delta_time(self) -> f32 {
        match self {
            Self::Rot(frame) => frame.delta_time,
            Self::Trans(frame) => frame.delta_time,
        }
    }

    pub fn translation(self) -> CVector {
        match self {
            Self::Rot(_) => CVector::ZERO,
            Self::Trans(frame) => frame.translation,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CAnimBlendSequence {
    pub type_flags: i32,
    pub name: String,
    pub bone_tag: i16,
    pub key_frames: Vec<AnimKeyFrame>,
    pub key_frames_compressed: Vec<KeyFrameTransCompressed>,
}

impl CAnimBlendSequence {
    pub fn new() -> Self {
        Self {
            type_flags: 0,
            name: String::new(),
            bone_tag: -1,
            key_frames: Vec::new(),
            key_frames_compressed: Vec::new(),
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.chars().take(24).collect();
    }

    pub fn set_num_frames(&mut self, num_frames: usize, translation: bool) {
        if translation {
            self.type_flags |= KF_ROT | KF_TRANS;
            self.key_frames = vec![AnimKeyFrame::Trans(KeyFrameTrans::default()); num_frames];
        } else {
            self.type_flags |= KF_ROT;
            self.key_frames = vec![AnimKeyFrame::Rot(KeyFrame::default()); num_frames];
        }
    }

    pub fn num_frames(&self) -> usize {
        if self.key_frames.is_empty() {
            self.key_frames_compressed.len()
        } else {
            self.key_frames.len()
        }
    }

    pub fn has_translation(&self) -> bool {
        self.type_flags & KF_TRANS != 0
    }

    pub fn get_key_frame(&self, index: usize) -> Option<AnimKeyFrame> {
        self.key_frames.get(index).copied()
    }

    /// Sample using the exact A/B keyframe convention of re3
    /// `CAnimBlendNode::FindKeyFrame` / `Update`: a frame's delta is the
    /// duration of the interval ending at that frame, so interpolation is from
    /// the preceding frame (B) to the active frame (A).
    pub fn sample_at_time(&self, mut time: f32) -> Option<AnimKeyFrame> {
        let count = self.key_frames.len();
        if count == 0 {
            return None;
        }
        if count == 1 {
            return self.key_frames.first().copied();
        }
        time = time.max(0.0);
        let mut frame_a = 1usize;
        let mut frame_b = 0usize;
        // `FindKeyFrame` loops only for repeating associations. Callers pass a
        // modulo hierarchy time for repeats, hence wrapping here is correct.
        for _ in 0..=count {
            let duration = self.key_frames[frame_a].delta_time().max(0.0);
            if time <= duration || duration == 0.0 {
                let t = if duration > 0.0 { time / duration } else { 0.0 };
                let a = self.key_frames[frame_a];
                let b = self.key_frames[frame_b];
                let mut to = a.rotation();
                let from = b.rotation();
                if from.dot(to) < 0.0 {
                    to = -to;
                }
                let dot = from.dot(to).clamp(-1.0, 1.0);
                let rotation = if dot > 0.9995 {
                    (from * (1.0 - t) + to * t).normalised()
                } else {
                    let theta = dot.acos();
                    CQuaternion::slerp(from, to, theta, theta.sin().recip(), t).normalised()
                };
                let translation = b.translation() + (a.translation() - b.translation()) * t;
                return Some(if self.has_translation() {
                    AnimKeyFrame::Trans(KeyFrameTrans {
                        rotation,
                        delta_time: duration,
                        translation,
                    })
                } else {
                    AnimKeyFrame::Rot(KeyFrame {
                        rotation,
                        delta_time: duration,
                    })
                });
            }
            time -= duration;
            frame_b = frame_a;
            frame_a = (frame_a + 1) % count;
        }
        self.key_frames.get(frame_b).copied()
    }

    /// Legacy nearest-key access retained for stream diagnostics. Runtime
    /// animation must use [`Self::sample_at_time`].
    pub fn get_key_frame_at_time(&self, time: f32) -> Option<AnimKeyFrame> {
        self.sample_at_time(time)
    }

    pub fn set_key_frame(&mut self, index: usize, frame: AnimKeyFrame) {
        if let Some(slot) = self.key_frames.get_mut(index) {
            *slot = frame;
        }
    }

    pub fn remove_quaternion_flips(&mut self) {
        if self.key_frames.len() < 2 {
            return;
        }

        let mut last = self.key_frames[0].rotation();
        for frame in self.key_frames.iter_mut().skip(1) {
            match frame {
                AnimKeyFrame::Rot(kf) => {
                    if last.dot(kf.rotation) < 0.0 {
                        kf.rotation = -kf.rotation;
                    }
                    last = kf.rotation;
                }
                AnimKeyFrame::Trans(kf) => {
                    if last.dot(kf.rotation) < 0.0 {
                        kf.rotation = -kf.rotation;
                    }
                    last = kf.rotation;
                }
            }
        }
    }

    pub fn uncompress(&mut self) {
        if self.key_frames_compressed.is_empty() {
            return;
        }

        let rot_scale = 1.0 / 4096.0;
        let time_scale = 1.0 / 60.0;
        let trans_scale = 1.0 / 128.0;
        self.key_frames = self
            .key_frames_compressed
            .iter()
            .map(|compressed| {
                let rotation = CQuaternion::new(
                    compressed.rot[0] as f32 * rot_scale,
                    compressed.rot[1] as f32 * rot_scale,
                    compressed.rot[2] as f32 * rot_scale,
                    compressed.rot[3] as f32 * rot_scale,
                );
                let delta_time = compressed.delta_time as f32 * time_scale;
                if self.has_translation() {
                    AnimKeyFrame::Trans(KeyFrameTrans {
                        rotation,
                        delta_time,
                        translation: CVector::new(
                            compressed.trans[0] as f32 * trans_scale,
                            compressed.trans[1] as f32 * trans_scale,
                            compressed.trans[2] as f32 * trans_scale,
                        ),
                    })
                } else {
                    AnimKeyFrame::Rot(KeyFrame {
                        rotation,
                        delta_time,
                    })
                }
            })
            .collect();
        self.key_frames_compressed.clear();
    }

    pub fn compress_keyframes(&mut self) {
        if self.key_frames.is_empty() {
            return;
        }

        let rot_scale = 4096.0;
        let time_scale = 60.0;
        let trans_scale = 128.0;
        self.key_frames_compressed = self
            .key_frames
            .iter()
            .map(|frame| {
                let rotation = frame.rotation();
                let translation = frame.translation();
                KeyFrameTransCompressed {
                    rot: [
                        (rotation.x * rot_scale) as i16,
                        (rotation.y * rot_scale) as i16,
                        (rotation.z * rot_scale) as i16,
                        (rotation.w * rot_scale) as i16,
                    ],
                    delta_time: (frame.delta_time() * time_scale + 0.5) as i16,
                    trans: [
                        (translation.x * trans_scale) as i16,
                        (translation.y * trans_scale) as i16,
                        (translation.z * trans_scale) as i16,
                    ],
                }
            })
            .collect();
    }

    pub fn remove_uncompressed_data(&mut self) {
        if self.key_frames.is_empty() {
            return;
        }
        self.compress_keyframes();
        self.key_frames.clear();
    }
}

impl Default for CAnimBlendSequence {
    fn default() -> Self {
        Self::new()
    }
}
