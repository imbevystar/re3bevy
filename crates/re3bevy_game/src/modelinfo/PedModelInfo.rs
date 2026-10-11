//! re3 reference: references/re-GTA/src/modelinfo/PedModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/PedModelInfo.h
//!
//! Ped model metadata facade.

pub use super::ModelInfo::CPedModelInfo;
use crate::peds::PedVitality::PedDamagePiece;

/// Resident DFF node indices for CPedModelInfo::m_pColNodeInfos. Centres are
/// sampled from the actor's canonical clump when a weapon query needs them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PedHitSphere {
    pub frame_index: usize,
    pub offset_x: f32,
    pub offset_z: f32,
    pub radius: f32,
    pub piece: PedDamagePiece,
}

impl CPedModelInfo {
    pub(crate) fn resolve_hit_spheres(summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary) -> Vec<PedHitSphere> {
        use crate::animation::Bones::*;
        use PedDamagePiece::*;
        struct Node { name: &'static str, bone: i32, x: f32, z: f32, radius: f32, piece: PedDamagePiece }
        // Executable-authored parameters from PedModelInfo.cpp, not GTA file data.
        let nodes = [
            Node { name: "Shead", bone: BONE_HEAD, x: 0.0, z: 0.05, radius: 0.2, piece: Head },
            Node { name: "Storso", bone: convert_ped_node_2_bone_tag(PED_TORSO), x: 0.0, z: 0.15, radius: 0.2, piece: Torso },
            Node { name: "Storso", bone: convert_ped_node_2_bone_tag(PED_TORSO), x: 0.0, z: -0.05, radius: 0.3, piece: Torso },
            Node { name: "Smid", bone: convert_ped_node_2_bone_tag(PED_MID), x: 0.0, z: -0.07, radius: 0.3, piece: Mid },
            Node { name: "Supperarml", bone: BONE_UPPERARML, x: 0.07, z: -0.1, radius: 0.2, piece: LeftArm },
            Node { name: "Supperarmr", bone: BONE_UPPERARMR, x: -0.07, z: -0.1, radius: 0.2, piece: RightArm },
            Node { name: "Slowerlegl", bone: convert_ped_node_2_bone_tag(PED_TORSO), x: 0.0, z: 0.07, radius: 0.25, piece: LeftLeg },
            Node { name: "Slowerlegr", bone: convert_ped_node_2_bone_tag(PED_LOWERLEGR), x: 0.0, z: 0.07, radius: 0.25, piece: RightLeg },
        ];
        let skinned = summary.geometries.iter().any(|geometry| geometry.skin.is_some());
        let resolved: Option<Vec<_>> = nodes.iter().map(|node| {
            let frame_index = summary.frames.iter().position(|frame| {
                if skinned { frame.hanim_id == Some(node.bone) }
                else { frame.name.as_deref().is_some_and(|name| name.eq_ignore_ascii_case(node.name)) }
            });
            let Some(frame_index) = frame_index else {
                bevy::log::warn_once!("ped_weapon_hit_model_missing_original_node: {}", node.name);
                return None;
            };
            let radius = if !skinned && node.piece == Head {
                // CreateHitColModel initialises radius from m_pColNodeInfos;
                // FindHeadRadiusCB replaces it when the frame owns an atomic.
                summary.atomics.iter().filter(|atomic| atomic.frame_index == frame_index as i32)
                    .filter_map(|atomic| summary.geometries.get(atomic.geometry_index as usize))
                    .filter_map(|geometry| geometry.decoded.as_ref()?.bounding_sphere)
                    .map(|sphere| sphere[3]).last().unwrap_or(node.radius)
            } else { node.radius };
            Some(PedHitSphere { frame_index, offset_x: node.x, offset_z: node.z, radius, piece: node.piece })
        }).collect();
        resolved.unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PedNode {
    PED_TORSO = 0,
    PED_MID = 1,
    PED_HEAD = 2,
    PED_UPPERARML = 3,
    PED_UPPERARMR = 4,
    PED_HANDL = 5,
    PED_HANDR = 6,
    PED_UPPERLEGL = 7,
    PED_UPPERLEGR = 8,
    PED_FOOTL = 9,
    PED_FOOTR = 10,
    PED_LOWERLEGR = 11,
    PED_NODE_MAX = 12,
}

impl CPedModelInfo {
    pub fn DeleteRwObject(&mut self) {
        self.clump.DeleteRwObject();
    }

    pub fn GetPedTypeName(&self) -> &str {
        &self.ped_type
    }

    pub fn SetPedTypeName(&mut self, ped_type: impl Into<String>) {
        self.ped_type = ped_type.into();
    }

    pub fn GetPedStatsName(&self) -> &str {
        &self.ped_stats
    }

    pub fn SetPedStatsName(&mut self, ped_stats: impl Into<String>) {
        self.ped_stats = ped_stats.into();
    }

    pub fn GetAnimGroupName(&self) -> &str {
        &self.anim_group
    }

    pub fn SetAnimGroupName(&mut self, anim_group: impl Into<String>) {
        self.anim_group = anim_group.into();
    }

    pub fn SetCarsCanDriveMask(&mut self, mask: u32) {
        self.cars_can_drive = mask;
    }
}
