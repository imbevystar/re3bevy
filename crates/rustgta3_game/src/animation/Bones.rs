//! re3 reference: references/re-GTA/src/animation/Bones.cpp
//! re3 reference: references/re-GTA/src/animation/Bones.h
//!
//! Ped skin bone tag helpers.

pub type BoneTag = i32;

pub const BONE_WAIST: BoneTag = 0;
pub const BONE_UPPERLEGR: BoneTag = 1;
pub const BONE_LOWERLEGR: BoneTag = 2;
pub const BONE_FOOTR: BoneTag = 3;
pub const BONE_UPPERLEGL: BoneTag = 4;
pub const BONE_LOWERLEGL: BoneTag = 5;
pub const BONE_FOOTL: BoneTag = 6;
pub const BONE_MID: BoneTag = 7;
pub const BONE_TORSO: BoneTag = 8;
pub const BONE_HEAD: BoneTag = 9;
pub const BONE_UPPERARMR: BoneTag = 10;
pub const BONE_LOWERARMR: BoneTag = 11;
pub const BONE_RHAND: BoneTag = 12;
pub const BONE_UPPERARML: BoneTag = 13;
pub const BONE_LOWERARML: BoneTag = 14;
pub const BONE_LHAND: BoneTag = 15;

pub const PED_TORSO: i32 = 0;
pub const PED_MID: i32 = 1;
pub const PED_HEAD: i32 = 2;
pub const PED_UPPERARML: i32 = 3;
pub const PED_UPPERARMR: i32 = 4;
pub const PED_HANDL: i32 = 5;
pub const PED_HANDR: i32 = 6;
pub const PED_UPPERLEGL: i32 = 7;
pub const PED_UPPERLEGR: i32 = 8;
pub const PED_FOOTL: i32 = 9;
pub const PED_FOOTR: i32 = 10;
pub const PED_LOWERLEGR: i32 = 11;

pub fn convert_ped_node_2_bone_tag(node: i32) -> BoneTag {
    match node {
        PED_TORSO => BONE_WAIST,
        PED_MID => BONE_TORSO,
        PED_HEAD => BONE_HEAD,
        PED_UPPERARML => BONE_UPPERARML,
        PED_UPPERARMR => BONE_UPPERARMR,
        PED_HANDL => BONE_LHAND,
        PED_HANDR => BONE_RHAND,
        PED_UPPERLEGL => BONE_UPPERLEGL,
        PED_UPPERLEGR => BONE_UPPERLEGR,
        PED_FOOTL => BONE_FOOTL,
        PED_FOOTR => BONE_FOOTR,
        PED_LOWERLEGR => BONE_LOWERLEGL,
        _ => -1,
    }
}

pub fn convert_bone_tag_2_bone_name(tag: BoneTag) -> Option<&'static str> {
    match tag {
        BONE_WAIST => Some("Swaist"),
        BONE_UPPERLEGR => Some("Supperlegr"),
        BONE_LOWERLEGR => Some("Slowerlegr"),
        BONE_FOOTR => Some("Sfootr"),
        BONE_UPPERLEGL => Some("Supperlegl"),
        BONE_LOWERLEGL => Some("Slowerlegl"),
        BONE_FOOTL => Some("Sfootl"),
        BONE_MID => Some("Smid"),
        BONE_TORSO => Some("Storso"),
        BONE_HEAD => Some("Shead"),
        BONE_UPPERARMR => Some("Supperarmr"),
        BONE_LOWERARMR => Some("Slowerarmr"),
        BONE_RHAND => Some("SRhand"),
        BONE_UPPERARML => Some("Supperarml"),
        BONE_LOWERARML => Some("Slowerarml"),
        BONE_LHAND => Some("SLhand"),
        _ => None,
    }
}
