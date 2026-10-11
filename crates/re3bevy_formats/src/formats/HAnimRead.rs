use crate::formats::ClumpRead::read_chunk_header;

pub const RW_ID_HANIMANIMATION: u32 = 0x1B;
pub const RP_HANIM_STD_KEYFRAME_STREAM_SIZE: usize = 36;

#[derive(Debug, Clone, PartialEq)]
pub struct RwHAnimAnimationSummary {
    pub chunk_id: u32,
    pub chunk_length: u32,
    pub chunk_version: u32,
    pub type_id: u32,
    pub flags: u32,
    pub num_keyframes: u32,
    pub custom_data_size: u32,
    pub duration_seconds: f32,
    pub keyframe_size: usize,
    pub keyframe_bytes: usize,
    pub payload_bytes: usize,
    pub expected_payload_bytes: usize,
    pub payload_size_matches: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwHAnimKeyFrameSample {
    pub index: usize,
    pub time_bits: u32,
    pub time_seconds: f32,
    pub quaternion_bits: [u32; 4],
    pub translation_bits: [u32; 3],
    pub frame_tail_raw: u32,
}

/// Decoded `RpHAnimStdKeyFrame`: time, quaternion, translation and stream tail.
/// The 36-byte layout is the one consumed by re3 `RpHAnimAnimationStreamRead`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwHAnimKeyFrame {
    pub time_seconds: f32,
    pub rotation: [f32; 4],
    pub translation: [f32; 3],
    pub frame_tail_raw: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwHAnimAnimation {
    pub summary: RwHAnimAnimationSummary,
    pub keyframes: Vec<RwHAnimKeyFrame>,
    /// Keyframe indices grouped by the serialized `prevFrame` links. RenderWare
    /// stores all hierarchy nodes in one keyframe array; it is not one global
    /// chronological track.
    pub node_tracks: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwHAnimNodePose {
    pub node_index: usize,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

impl RwHAnimAnimation {
    /// Equivalent of the standard HAnim interpolator at an explicit animation
    /// time. Each hierarchy node follows its own `prevFrame` chain.
    pub fn sample_nodes(&self, time_seconds: f32) -> Vec<RwHAnimNodePose> {
        let duration = self.summary.duration_seconds;
        // `AnimInterpolator::addTime` keeps the exact duration sample and only
        // resets after currentTime becomes greater than duration.
        let time = if duration > 0.0 && time_seconds > duration {
            time_seconds.rem_euclid(duration)
        } else {
            time_seconds.max(0.0)
        };
        self.node_tracks
            .iter()
            .enumerate()
            .filter_map(|(node_index, track)| {
                let first = *self.keyframes.get(*track.first()?)?;
                let (left, right) = track
                    .windows(2)
                    .find_map(|pair| {
                        let left = *self.keyframes.get(pair[0])?;
                        let right = *self.keyframes.get(pair[1])?;
                        (time >= left.time_seconds && time < right.time_seconds)
                            .then_some((left, right))
                    })
                    .unwrap_or_else(|| {
                        let last = track
                            .last()
                            .and_then(|index| self.keyframes.get(*index))
                            .copied()
                            .unwrap_or(first);
                        if time < first.time_seconds {
                            (first, first)
                        } else {
                            (last, last)
                        }
                    });
                let span = right.time_seconds - left.time_seconds;
                let alpha = if span > 0.0 {
                    ((time - left.time_seconds) / span).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                Some(RwHAnimNodePose {
                    node_index,
                    translation: lerp3(left.translation, right.translation, alpha),
                    rotation: slerp_quaternion(left.rotation, right.rotation, alpha),
                })
            })
            .collect()
    }
}

pub fn read_hanim_animation(bytes: &[u8]) -> Result<RwHAnimAnimationSummary, String> {
    let chunk = read_chunk_header(bytes, 0).ok_or("missing RW chunk header")?;
    if chunk.id != RW_ID_HANIMANIMATION {
        return Err(format!(
            "expected rwID_HANIMANIMATION ({RW_ID_HANIMANIMATION}), found {}",
            chunk.id
        ));
    }
    if chunk.data_end > bytes.len() {
        return Err(format!(
            "HAnim chunk end {} exceeds byte length {}",
            chunk.data_end,
            bytes.len()
        ));
    }

    let payload = &bytes[chunk.data_start..chunk.data_end];
    if payload.len() < 20 {
        return Err(format!("HAnim payload too short: {}", payload.len()));
    }

    let type_id = read_u32_le(payload, 0)?;
    let flags = read_u32_le(payload, 4)?;
    let num_keyframes = read_u32_le(payload, 8)?;
    let custom_data_size = read_u32_le(payload, 12)?;
    let duration_seconds = f32::from_bits(read_u32_le(payload, 16)?);
    let keyframe_bytes = payload.len().saturating_sub(20);
    let expected_payload_bytes =
        20usize.saturating_add(num_keyframes as usize * RP_HANIM_STD_KEYFRAME_STREAM_SIZE);

    Ok(RwHAnimAnimationSummary {
        chunk_id: chunk.id,
        chunk_length: chunk.length,
        chunk_version: chunk.version,
        type_id,
        flags,
        num_keyframes,
        custom_data_size,
        duration_seconds,
        keyframe_size: RP_HANIM_STD_KEYFRAME_STREAM_SIZE,
        keyframe_bytes,
        payload_bytes: payload.len(),
        expected_payload_bytes,
        payload_size_matches: payload.len() == expected_payload_bytes,
    })
}

/// Decode an HAnim animation into runtime keyframes without retaining CUTS.IMG bytes.
pub fn decode_hanim_animation(bytes: &[u8]) -> Result<RwHAnimAnimation, String> {
    let summary = read_hanim_animation(bytes)?;
    if !summary.payload_size_matches {
        return Err(format!(
            "HAnim keyframe payload mismatch: expected {}, got {}",
            summary.expected_payload_bytes, summary.payload_bytes
        ));
    }
    let mut keyframes = Vec::with_capacity(summary.num_keyframes as usize);
    for index in 0..summary.num_keyframes as usize {
        let frame = read_hanim_keyframe(bytes, index)?;
        keyframes.push(RwHAnimKeyFrame {
            time_seconds: frame.time_seconds,
            rotation: frame.quaternion_bits.map(f32::from_bits),
            translation: frame.translation_bits.map(f32::from_bits),
            frame_tail_raw: frame.frame_tail_raw,
        });
    }
    let mut node_tracks: Vec<Vec<usize>> = Vec::new();
    let mut frame_nodes = vec![usize::MAX; keyframes.len()];
    for (index, frame) in keyframes.iter().enumerate() {
        let previous = usize::try_from(frame.frame_tail_raw)
            .ok()
            .filter(|offset| offset % RP_HANIM_STD_KEYFRAME_STREAM_SIZE == 0)
            .map(|offset| offset / RP_HANIM_STD_KEYFRAME_STREAM_SIZE)
            .filter(|previous| *previous < index);
        let node = previous
            .and_then(|previous| frame_nodes.get(previous).copied())
            .filter(|node| *node != usize::MAX)
            .unwrap_or_else(|| {
                node_tracks.push(Vec::new());
                node_tracks.len() - 1
            });
        frame_nodes[index] = node;
        node_tracks[node].push(index);
    }
    Ok(RwHAnimAnimation {
        summary,
        keyframes,
        node_tracks,
    })
}

fn lerp3(left: [f32; 3], right: [f32; 3], alpha: f32) -> [f32; 3] {
    [
        left[0] + (right[0] - left[0]) * alpha,
        left[1] + (right[1] - left[1]) * alpha,
        left[2] + (right[2] - left[2]) * alpha,
    ]
}

fn slerp_quaternion(mut left: [f32; 4], mut right: [f32; 4], alpha: f32) -> [f32; 4] {
    normalize_quaternion(&mut left);
    normalize_quaternion(&mut right);
    let mut dot = left[0] * right[0] + left[1] * right[1] + left[2] * right[2] + left[3] * right[3];
    if dot < 0.0 {
        right = right.map(|component| -component);
        dot = -dot;
    }
    let mut result = if dot > 0.9995 {
        [
            left[0] + (right[0] - left[0]) * alpha,
            left[1] + (right[1] - left[1]) * alpha,
            left[2] + (right[2] - left[2]) * alpha,
            left[3] + (right[3] - left[3]) * alpha,
        ]
    } else {
        let angle = dot.clamp(-1.0, 1.0).acos();
        let denominator = angle.sin();
        let left_weight = ((1.0 - alpha) * angle).sin() / denominator;
        let right_weight = (alpha * angle).sin() / denominator;
        [
            left[0] * left_weight + right[0] * right_weight,
            left[1] * left_weight + right[1] * right_weight,
            left[2] * left_weight + right[2] * right_weight,
            left[3] * left_weight + right[3] * right_weight,
        ]
    };
    normalize_quaternion(&mut result);
    result
}

fn normalize_quaternion(quaternion: &mut [f32; 4]) {
    let length = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if length > 1.0e-8 {
        for value in quaternion {
            *value /= length;
        }
    } else {
        *quaternion = [0.0, 0.0, 0.0, 1.0];
    }
}

pub fn read_hanim_keyframe(bytes: &[u8], index: usize) -> Result<RwHAnimKeyFrameSample, String> {
    let chunk = read_chunk_header(bytes, 0).ok_or("missing RW chunk header")?;
    if chunk.id != RW_ID_HANIMANIMATION {
        return Err(format!(
            "expected rwID_HANIMANIMATION ({RW_ID_HANIMANIMATION}), found {}",
            chunk.id
        ));
    }
    if chunk.data_end > bytes.len() {
        return Err(format!(
            "HAnim chunk end {} exceeds byte length {}",
            chunk.data_end,
            bytes.len()
        ));
    }

    let payload = &bytes[chunk.data_start..chunk.data_end];
    let frame_start = 20usize.saturating_add(index * RP_HANIM_STD_KEYFRAME_STREAM_SIZE);
    let frame_end = frame_start.saturating_add(RP_HANIM_STD_KEYFRAME_STREAM_SIZE);
    if frame_end > payload.len() {
        return Err(format!(
            "HAnim keyframe {index} exceeds payload length {}",
            payload.len()
        ));
    }

    let time_bits = read_u32_le(payload, frame_start)?;
    Ok(RwHAnimKeyFrameSample {
        index,
        time_bits,
        time_seconds: f32::from_bits(time_bits),
        quaternion_bits: [
            read_u32_le(payload, frame_start + 4)?,
            read_u32_le(payload, frame_start + 8)?,
            read_u32_le(payload, frame_start + 12)?,
            read_u32_le(payload, frame_start + 16)?,
        ],
        translation_bits: [
            read_u32_le(payload, frame_start + 20)?,
            read_u32_le(payload, frame_start + 24)?,
            read_u32_le(payload, frame_start + 28)?,
        ],
        frame_tail_raw: read_u32_le(payload, frame_start + 32)?,
    })
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset.saturating_add(4);
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| format!("not enough bytes to read u32 at {offset}"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
