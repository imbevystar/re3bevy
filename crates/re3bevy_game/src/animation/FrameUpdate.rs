//! re3 reference: references/re-GTA/src/animation/FrameUpdate.cpp
//!
//! Frame update callbacks represented as transform intents.

use crate::{
    animation::{
        AnimBlendAssociation::CAnimBlendAssociation,
        AnimBlendClumpData::{
            AnimBlendFrameData, CAnimBlendClumpData, IGNORE_ROTATION, IGNORE_TRANSLATION,
            VELOCITY_EXTRACTION, VELOCITY_EXTRACTION_3D,
        },
        AnimBlendNode::AnimNodeUpdateContext,
    },
    math::{Quaternion::CQuaternion, Vector::CVector},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimFrameTransform {
    pub translation: CVector,
    pub rotation: CQuaternion,
    pub velocity2d: Option<(f32, f32)>,
    pub velocity3d: Option<CVector>,
    pub looped: bool,
}

impl Default for AnimFrameTransform {
    fn default() -> Self {
        Self {
            translation: CVector::ZERO,
            rotation: CQuaternion::ZERO,
            velocity2d: None,
            velocity3d: None,
            looped: false,
        }
    }
}

pub fn frame_update_callback_non_skinned(
    clump_data: &mut CAnimBlendClumpData,
    frame_index: usize,
    partials_affect_weight: bool,
) -> AnimFrameTransform {
    let Some(frame) = clump_data.frames.get(frame_index).cloned() else {
        return AnimFrameTransform::default();
    };

    if frame.flag & VELOCITY_EXTRACTION != 0 && clump_data.velocity2d.is_some() {
        if frame.flag & VELOCITY_EXTRACTION_3D != 0 {
            return frame_update_callback_with_3d_velocity_extraction_non_skinned(
                clump_data,
                frame_index,
                partials_affect_weight,
            );
        }
        return frame_update_callback_with_velocity_extraction_non_skinned(
            clump_data,
            frame_index,
            partials_affect_weight,
        );
    }

    let total_blend_amount =
        total_partial_blend_for_velocity_extraction(clump_data, partials_affect_weight);
    let mut pos = CVector::ZERO;
    let mut rot = CQuaternion::ZERO;

    for assoc in &mut clump_data.associations {
        let ctx = AnimNodeUpdateContext {
            running: assoc.is_running(),
            repeating: assoc.is_repeating(),
            partial: assoc.is_partial(),
            blend_amount: assoc.blend_amount,
            time_step: assoc.time_step,
        };
        for node in &mut assoc.nodes {
            let Some(sequence_index) = node.sequence_index else {
                continue;
            };
            let Some(sequence) = assoc.hierarchy.sequences.get(sequence_index) else {
                continue;
            };
            let result = node.update(sequence, ctx, 1.0 - total_blend_amount);
            if sequence.has_translation() {
                pos += result.translation;
            }
            add_rotation_fixbugs(&mut rot, result.rotation);
        }
    }

    finish_transform(frame, pos, rot, None, None, false)
}

pub fn frame_update_callback_with_velocity_extraction_non_skinned(
    clump_data: &mut CAnimBlendClumpData,
    frame_index: usize,
    partials_affect_weight: bool,
) -> AnimFrameTransform {
    let Some(frame) = clump_data.frames.get(frame_index).cloned() else {
        return AnimFrameTransform::default();
    };
    let total_blend_amount = total_partial_blend(clump_data, partials_affect_weight);
    let mut pos = CVector::ZERO;
    let mut rot = CQuaternion::ZERO;
    let mut transx = 0.0;
    let mut transy = 0.0;
    let mut curx = 0.0;
    let mut cury = 0.0;
    let mut endx = 0.0;
    let mut endy = 0.0;
    let mut looped = false;

    for assoc in &clump_data.associations {
        let ctx = AnimNodeUpdateContext {
            running: assoc.is_running(),
            repeating: assoc.is_repeating(),
            partial: assoc.is_partial(),
            blend_amount: assoc.blend_amount,
            time_step: assoc.time_step,
        };
        for node in &assoc.nodes {
            let Some(sequence_index) = node.sequence_index else {
                continue;
            };
            let Some(sequence) = assoc.hierarchy.sequences.get(sequence_index) else {
                continue;
            };
            if sequence.has_translation() && assoc.has_translation() {
                let cur = node.get_current_translation(sequence, ctx, 1.0 - total_blend_amount);
                cury += cur.y;
                if assoc.has_x_translation() {
                    curx += cur.x;
                }
            }
        }
    }

    for assoc in &mut clump_data.associations {
        let ctx = AnimNodeUpdateContext {
            running: assoc.is_running(),
            repeating: assoc.is_repeating(),
            partial: assoc.is_partial(),
            blend_amount: assoc.blend_amount,
            time_step: assoc.time_step,
        };
        let assoc_has_translation = assoc.has_translation();
        let assoc_has_x_translation = assoc.has_x_translation();
        for node in &mut assoc.nodes {
            let Some(sequence_index) = node.sequence_index else {
                continue;
            };
            let Some(sequence) = assoc.hierarchy.sequences.get(sequence_index) else {
                continue;
            };
            let result = node.update(sequence, ctx, 1.0 - total_blend_amount);
            add_rotation_fixbugs(&mut rot, result.rotation);
            if sequence.has_translation() {
                pos += result.translation;
                if assoc_has_translation {
                    transy += result.translation.y;
                    if assoc_has_x_translation {
                        transx += result.translation.x;
                    }
                    looped |= result.looped;
                    if result.looped {
                        let end = node.get_end_translation(sequence, ctx, 1.0 - total_blend_amount);
                        endy += end.y;
                        if assoc_has_x_translation {
                            endx += end.x;
                        }
                    }
                }
            }
        }
    }

    let mut adjusted_pos = CVector::new(pos.x - transx, pos.y - transy, pos.z);
    if adjusted_pos.z >= -0.8 {
        if adjusted_pos.z < -0.4 {
            adjusted_pos.z += (2.5 * adjusted_pos.z + 2.0) * frame.reset_pos.z;
        } else {
            adjusted_pos.z += frame.reset_pos.z;
        }
    }
    adjusted_pos.x += frame.reset_pos.x;
    adjusted_pos.y += frame.reset_pos.y;

    let velocity = (
        transx - curx + if looped { endx } else { 0.0 },
        transy - cury + if looped { endy } else { 0.0 },
    );
    clump_data.velocity2d = Some(velocity);
    finish_transform_without_reset(frame, adjusted_pos, rot, Some(velocity), None, looped)
}

/// re3 keeps jump/fall/landing partial associations in the same RpAnimBlend
/// clump as locomotion. The Rust player owns that association separately, so
/// feed it into the velocity-extraction callback without moving ownership or
/// advancing association time a second time. Node state is advanced exactly
/// once here, while `CAnimBlendAssociation::update_time` remains owned by the
/// normal jump-animation lifecycle.
pub fn frame_update_callback_with_velocity_extraction_overlay_non_skinned(
    clump_data: &mut CAnimBlendClumpData,
    frame_index: usize,
    partials_affect_weight: bool,
    mut overlay: Option<&mut CAnimBlendAssociation>,
) -> AnimFrameTransform {
    let Some(frame) = clump_data.frames.get(frame_index).cloned() else {
        return AnimFrameTransform::default();
    };
    let mut total_blend_amount =
        total_partial_blend_for_velocity_extraction(clump_data, partials_affect_weight);
    if partials_affect_weight
        && let Some(association) = overlay.as_deref()
        && association.is_partial()
        && velocity_extraction_sequence_index(association).is_some()
    {
        total_blend_amount += association.blend_amount;
    }

    let mut pos = CVector::ZERO;
    let mut rot = CQuaternion::ZERO;
    let mut transx = 0.0;
    let mut transy = 0.0;
    let mut curx = 0.0;
    let mut cury = 0.0;
    let mut endx = 0.0;
    let mut endy = 0.0;
    let mut looped = false;

    for assoc in &clump_data.associations {
        accumulate_current_velocity_translation(
            assoc,
            1.0 - total_blend_amount,
            &mut curx,
            &mut cury,
        );
    }
    if let Some(assoc) = overlay.as_deref() {
        accumulate_current_velocity_translation(
            assoc,
            1.0 - total_blend_amount,
            &mut curx,
            &mut cury,
        );
    }

    for assoc in &mut clump_data.associations {
        accumulate_updated_velocity_translation(
            assoc,
            1.0 - total_blend_amount,
            &mut pos,
            &mut rot,
            &mut transx,
            &mut transy,
            &mut endx,
            &mut endy,
            &mut looped,
        );
    }
    if let Some(assoc) = overlay.as_deref_mut() {
        accumulate_updated_velocity_translation(
            assoc,
            1.0 - total_blend_amount,
            &mut pos,
            &mut rot,
            &mut transx,
            &mut transy,
            &mut endx,
            &mut endy,
            &mut looped,
        );
    }

    let mut adjusted_pos = CVector::new(pos.x - transx, pos.y - transy, pos.z);
    if adjusted_pos.z >= -0.8 {
        if adjusted_pos.z < -0.4 {
            adjusted_pos.z += (2.5 * adjusted_pos.z + 2.0) * frame.reset_pos.z;
        } else {
            adjusted_pos.z += frame.reset_pos.z;
        }
    }
    adjusted_pos.x += frame.reset_pos.x;
    adjusted_pos.y += frame.reset_pos.y;

    let velocity = (
        transx - curx + if looped { endx } else { 0.0 },
        transy - cury + if looped { endy } else { 0.0 },
    );
    clump_data.velocity2d = Some(velocity);
    finish_transform_without_reset(frame, adjusted_pos, rot, Some(velocity), None, looped)
}

fn accumulate_current_velocity_translation(
    assoc: &CAnimBlendAssociation,
    weight: f32,
    curx: &mut f32,
    cury: &mut f32,
) {
    let Some(sequence_index) = velocity_extraction_sequence_index(assoc) else {
        return;
    };
    let ctx = AnimNodeUpdateContext {
        running: assoc.is_running(),
        repeating: assoc.is_repeating(),
        partial: assoc.is_partial(),
        blend_amount: assoc.blend_amount,
        time_step: assoc.time_step,
    };
    let Some(node) = assoc.nodes.get(sequence_index) else {
        return;
    };
    let Some(sequence) = assoc.hierarchy.sequences.get(sequence_index) else {
        return;
    };
    let cur = node.get_current_translation(sequence, ctx, weight);
    if !assoc.has_translation() {
        return;
    }
    *cury += cur.y;
    if assoc.has_x_translation() {
        *curx += cur.x;
    }
}

#[allow(clippy::too_many_arguments)]
fn accumulate_updated_velocity_translation(
    assoc: &mut CAnimBlendAssociation,
    weight: f32,
    pos: &mut CVector,
    rot: &mut CQuaternion,
    transx: &mut f32,
    transy: &mut f32,
    endx: &mut f32,
    endy: &mut f32,
    looped: &mut bool,
) {
    let Some(sequence_index) = velocity_extraction_sequence_index(assoc) else {
        return;
    };
    let ctx = AnimNodeUpdateContext {
        running: assoc.is_running(),
        repeating: assoc.is_repeating(),
        partial: assoc.is_partial(),
        blend_amount: assoc.blend_amount,
        time_step: assoc.time_step,
    };
    let assoc_has_translation = assoc.has_translation();
    let assoc_has_x_translation = assoc.has_x_translation();
    let Some(sequence) = assoc.hierarchy.sequences.get(sequence_index) else {
        return;
    };
    let Some(node) = assoc.nodes.get_mut(sequence_index) else {
        return;
    };
    let result = node.update(sequence, ctx, weight);
    add_rotation_fixbugs(rot, result.rotation);
    *pos += result.translation;
    if assoc_has_translation {
        *transy += result.translation.y;
        if assoc_has_x_translation {
            *transx += result.translation.x;
        }
        *looped |= result.looped;
        if result.looped {
            let end = node.get_end_translation(sequence, ctx, weight);
            *endy += end.y;
            if assoc_has_x_translation {
                *endx += end.x;
            }
        }
    }
}

fn velocity_extraction_sequence_index(assoc: &CAnimBlendAssociation) -> Option<usize> {
    // In re3 the velocity-extraction callback runs on the clump root frame and
    // therefore sees exactly one node from each association. Our lightweight
    // player clump doesn't retain the RenderWare frame->node mapping; the same
    // Ped Swaist-track partials consume locomotion weight even without
    // ASSOC_HAS_TRANSLATION (e.g. WEAPON_AK_body). That flag controls physical
    // extraction, not participation in the root pose blend.
    assoc
        .hierarchy
        .sequences
        .iter()
        .position(|sequence| sequence.name.eq_ignore_ascii_case("Swaist")
            || sequence.name.eq_ignore_ascii_case("Root"))
}

pub fn frame_update_callback_with_3d_velocity_extraction_non_skinned(
    clump_data: &mut CAnimBlendClumpData,
    frame_index: usize,
    partials_affect_weight: bool,
) -> AnimFrameTransform {
    let Some(frame) = clump_data.frames.get(frame_index).cloned() else {
        return AnimFrameTransform::default();
    };
    let total_blend_amount =
        total_partial_blend_for_velocity_extraction(clump_data, partials_affect_weight);
    let mut pos = CVector::ZERO;
    let mut rot = CQuaternion::ZERO;
    let mut trans = CVector::ZERO;
    let mut cur = CVector::ZERO;
    let mut end = CVector::ZERO;
    let mut looped = false;

    for assoc in &clump_data.associations {
        let ctx = AnimNodeUpdateContext {
            running: assoc.is_running(),
            repeating: assoc.is_repeating(),
            partial: assoc.is_partial(),
            blend_amount: assoc.blend_amount,
            time_step: assoc.time_step,
        };
        for node in &assoc.nodes {
            let Some(sequence_index) = node.sequence_index else {
                continue;
            };
            let Some(sequence) = assoc.hierarchy.sequences.get(sequence_index) else {
                continue;
            };
            if sequence.has_translation() && assoc.has_translation() {
                cur += node.get_current_translation(sequence, ctx, 1.0 - total_blend_amount);
            }
        }
    }

    for assoc in &mut clump_data.associations {
        let ctx = AnimNodeUpdateContext {
            running: assoc.is_running(),
            repeating: assoc.is_repeating(),
            partial: assoc.is_partial(),
            blend_amount: assoc.blend_amount,
            time_step: assoc.time_step,
        };
        let assoc_has_translation = assoc.has_translation();
        for node in &mut assoc.nodes {
            let Some(sequence_index) = node.sequence_index else {
                continue;
            };
            let Some(sequence) = assoc.hierarchy.sequences.get(sequence_index) else {
                continue;
            };
            let result = node.update(sequence, ctx, 1.0 - total_blend_amount);
            add_rotation_fixbugs(&mut rot, result.rotation);
            if sequence.has_translation() {
                pos += result.translation;
                if assoc_has_translation {
                    trans += result.translation;
                    looped |= result.looped;
                    if result.looped {
                        end += node.get_end_translation(sequence, ctx, 1.0 - total_blend_amount);
                    }
                }
            }
        }
    }

    let velocity = trans - cur + if looped { end } else { CVector::ZERO };
    clump_data.velocity3d = Some(velocity);
    let adjusted_pos = pos - trans + frame.reset_pos;
    finish_transform_without_reset(frame, adjusted_pos, rot, None, Some(velocity), looped)
}

fn total_partial_blend(clump_data: &CAnimBlendClumpData, enabled: bool) -> f32 {
    if !enabled {
        return 0.0;
    }
    clump_data
        .associations
        .iter()
        .filter(|assoc| assoc.is_partial())
        .map(|assoc| assoc.blend_amount)
        .sum()
}

fn total_partial_blend_for_velocity_extraction(
    clump_data: &CAnimBlendClumpData,
    enabled: bool,
) -> f32 {
    if !enabled {
        return 0.0;
    }
    // Native FrameUpdate.cpp computes partial weight from the node list of the
    // frame currently being updated. The player's extracted velocity belongs
    // to the clump root frame, so an upper-body-only partial (weapon/punch)
    // must not consume root-motion weight. Our lightweight clump does not keep
    // RenderWare's frame->node pointer table; `velocity_extraction_sequence_index`
    // is the existing root-sequence approximation used by this path.
    clump_data
        .associations
        .iter()
        .filter(|assoc| assoc.is_partial() && velocity_extraction_sequence_index(assoc).is_some())
        .map(|assoc| assoc.blend_amount)
        .sum()
}

fn add_rotation_fixbugs(rot: &mut CQuaternion, q: CQuaternion) {
    if rot.dot(q) < 0.0 {
        *rot -= q;
    } else {
        *rot += q;
    }
}

fn finish_transform(
    frame: AnimBlendFrameData,
    pos: CVector,
    mut rot: CQuaternion,
    velocity2d: Option<(f32, f32)>,
    velocity3d: Option<CVector>,
    looped: bool,
) -> AnimFrameTransform {
    let translation = if frame.flag & IGNORE_TRANSLATION == 0 {
        pos + frame.reset_pos
    } else {
        CVector::ZERO
    };
    if frame.flag & IGNORE_ROTATION == 0 {
        rot.normalise();
    } else {
        rot = CQuaternion::ZERO;
    }
    AnimFrameTransform {
        translation,
        rotation: rot,
        velocity2d,
        velocity3d,
        looped,
    }
}

fn finish_transform_without_reset(
    frame: AnimBlendFrameData,
    translation: CVector,
    mut rot: CQuaternion,
    velocity2d: Option<(f32, f32)>,
    velocity3d: Option<CVector>,
    looped: bool,
) -> AnimFrameTransform {
    let translation = if frame.flag & IGNORE_TRANSLATION == 0 {
        translation
    } else {
        CVector::ZERO
    };
    if frame.flag & IGNORE_ROTATION == 0 {
        rot.normalise();
    } else {
        rot = CQuaternion::ZERO;
    }
    AnimFrameTransform {
        translation,
        rotation: rot,
        velocity2d,
        velocity3d,
        looped,
    }
}
