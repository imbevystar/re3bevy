//! re3 reference: references/re-GTA/src/animation/RpAnimBlend.cpp
//! re3 reference: references/re-GTA/src/animation/RpAnimBlend.h
//!
//! RenderWare animation blend helpers expressed over `CAnimBlendClumpData`.

use crate::animation::{
    AnimBlendAssociation::CAnimBlendAssociation,
    AnimBlendClumpData::{AnimBlendFrameData, CAnimBlendClumpData, VELOCITY_EXTRACTION},
};

pub fn rp_anim_blend_allocate_data() -> CAnimBlendClumpData {
    CAnimBlendClumpData::new()
}

pub fn rp_anim_blend_get_next_association(
    clump_data: &CAnimBlendClumpData,
    current_index: usize,
) -> Option<&CAnimBlendAssociation> {
    clump_data.associations.get(current_index + 1)
}

pub fn rp_anim_blend_get_next_association_with_mask(
    clump_data: &CAnimBlendClumpData,
    current_index: usize,
    mask: i32,
) -> Option<&CAnimBlendAssociation> {
    clump_data
        .associations
        .iter()
        .skip(current_index + 1)
        .find(|assoc| assoc.flags & mask != 0)
}

pub fn rp_anim_blend_clump_set_blend_deltas(
    clump_data: &mut CAnimBlendClumpData,
    mask: i32,
    delta: f32,
) {
    for assoc in &mut clump_data.associations {
        if mask == 0 || assoc.flags & mask != 0 {
            assoc.blend_delta = delta;
        }
    }
}

pub fn rp_anim_blend_clump_remove_all_associations(clump_data: &mut CAnimBlendClumpData) {
    rp_anim_blend_clump_remove_associations(clump_data, 0);
}

pub fn rp_anim_blend_clump_remove_associations(clump_data: &mut CAnimBlendClumpData, mask: i32) {
    if mask == 0 {
        clump_data.associations.clear();
    } else {
        clump_data
            .associations
            .retain(|association| association.flags & mask == 0);
    }
}

pub fn rp_anim_blend_clump_init(frame_names: &[String]) -> CAnimBlendClumpData {
    let mut clump_data = CAnimBlendClumpData::new();
    clump_data.set_number_of_frames(frame_names.len());
    for (frame, name) in clump_data.frames.iter_mut().zip(frame_names) {
        frame.frame_name = name.clone();
    }
    if let Some(root) = clump_data.frames.first_mut() {
        root.flag |= VELOCITY_EXTRACTION;
    }
    clump_data
}

pub fn rp_anim_blend_clump_is_initialized(clump_data: &CAnimBlendClumpData) -> bool {
    clump_data.num_frames != 0
}

pub fn rp_anim_blend_clump_fill_frame_array(
    clump_data: &CAnimBlendClumpData,
) -> Vec<AnimBlendFrameData> {
    clump_data.frames.clone()
}

pub fn rp_anim_blend_clump_find_frame<'a>(
    clump_data: &'a CAnimBlendClumpData,
    name: &str,
) -> Option<&'a AnimBlendFrameData> {
    clump_data
        .frames
        .iter()
        .find(|frame| frame.frame_name.eq_ignore_ascii_case(name))
}

pub fn rp_anim_blend_clump_get_association(
    clump_data: &CAnimBlendClumpData,
    id: i32,
) -> Option<&CAnimBlendAssociation> {
    clump_data
        .associations
        .iter()
        .find(|assoc| assoc.anim_id == id)
}

pub fn rp_anim_blend_clump_get_main_association(
    clump_data: &CAnimBlendClumpData,
) -> (
    Option<&CAnimBlendAssociation>,
    Option<&CAnimBlendAssociation>,
    f32,
) {
    let mut main_assoc = None;
    let mut second_assoc = None;
    let mut main_blend = 0.0;
    let mut second_blend = 0.0;

    for assoc in &clump_data.associations {
        if assoc.is_partial() {
            continue;
        }
        if assoc.blend_amount > main_blend {
            second_blend = main_blend;
            second_assoc = main_assoc;
            main_blend = assoc.blend_amount;
            main_assoc = Some(assoc);
        } else if assoc.blend_amount > second_blend {
            second_blend = assoc.blend_amount;
            second_assoc = Some(assoc);
        }
    }

    let remaining = (1.0 - main_blend).max(0.0);
    (main_assoc, second_assoc, remaining)
}

pub fn rp_anim_blend_clump_get_main_partial_association(
    clump_data: &CAnimBlendClumpData,
) -> Option<&CAnimBlendAssociation> {
    clump_data
        .associations
        .iter()
        .filter(|assoc| assoc.is_partial())
        .max_by(|left, right| left.blend_amount.total_cmp(&right.blend_amount))
}

pub fn rp_anim_blend_clump_get_first_association(
    clump_data: &CAnimBlendClumpData,
    mask: i32,
) -> Option<&CAnimBlendAssociation> {
    clump_data
        .associations
        .iter()
        .find(|assoc| mask == 0 || assoc.flags & mask != 0)
}

pub fn rp_anim_blend_clump_update_animations(
    clump_data: &mut CAnimBlendClumpData,
    time_delta: f32,
    rel_speed: f32,
) {
    let mut index = 0;
    while index < clump_data.associations.len() {
        clump_data.associations[index].update_time(time_delta, rel_speed);
        if clump_data.associations[index].update_blend(time_delta) {
            index += 1;
        } else {
            clump_data.associations.remove(index);
        }
    }
}

pub(crate) fn animation_frame_names_match(animation_name: &str, rw_frame_name: &str) -> bool {
    if animation_name.eq_ignore_ascii_case(rw_frame_name) {
        return true;
    }
    // GTA III PC non-skinned ped DFF frames use an `S` prefix (`SRhand`,
    // `Shead`, ...), while PED.IFP/CUTS.IMG sequences use the canonical bone
    // names (`Rhand`, `head`). Resolve this authored naming boundary alongside
    // the HAnim bone-tag matching used by RpAnimBlend associations.
    rw_frame_name
        .strip_prefix(['S', 's'])
        .is_some_and(|name| animation_name.eq_ignore_ascii_case(name))
        || animation_name
            .strip_prefix(['S', 's'])
            .is_some_and(|name| name.eq_ignore_ascii_case(rw_frame_name))
}
