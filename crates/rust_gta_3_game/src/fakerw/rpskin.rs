//! re3 reference: references/re-GTA/src/fakerw/rpskin.h
//!
//! Skin plugin metadata facade.

use super::{rphanim::RpHAnimHierarchy, rwplcore::RwMatrix};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RwMatrixWeights {
    pub w0: f32,
    pub w1: f32,
    pub w2: f32,
    pub w3: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpSkin {
    pub num_bones: u32,
    /// RenderWare's compact palette; it is distinct from `num_bones` on
    /// post-3.4 DFFs and required when respecting SkinSplit limits.
    pub used_bones: Vec<u8>,
    pub max_vertex_weights: u8,
    pub weights: Vec<RwMatrixWeights>,
    pub indices: Vec<[u8; 4]>,
    pub inverse_matrices: Vec<RwMatrix>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpSkinAtomicBinding {
    pub hierarchy: Option<RpHAnimHierarchy>,
}

/// Convert the decoded DFF Skin PLG into the small fakerw surface consumed by
/// animation/render code. No asset data is invented: absent/invalid plugins
/// remain `None` at the stream layer.
pub fn RpSkinFromDescriptor(
    descriptor: &rust_gta_3_formats::formats::ClumpRead::RwSkinDescriptor,
) -> RpSkin {
    RpSkin {
        num_bones: descriptor.num_bones as u32,
        used_bones: descriptor.used_bones.clone(),
        max_vertex_weights: descriptor.max_vertex_weights,
        weights: descriptor
            .vertex_bone_weights
            .iter()
            .map(|weights| RwMatrixWeights {
                w0: weights[0],
                w1: weights[1],
                w2: weights[2],
                w3: weights[3],
            })
            .collect(),
        indices: descriptor.vertex_bone_indices.clone(),
        inverse_matrices: descriptor
            .skin_to_bone_matrices
            .iter()
            .map(|matrix| RwMatrix {
                right: super::rwplcore::RwV3d {
                    x: matrix[0][0],
                    y: matrix[0][1],
                    z: matrix[0][2],
                },
                up: super::rwplcore::RwV3d {
                    x: matrix[1][0],
                    y: matrix[1][1],
                    z: matrix[1][2],
                },
                at: super::rwplcore::RwV3d {
                    x: matrix[2][0],
                    y: matrix[2][1],
                    z: matrix[2][2],
                },
                pos: super::rwplcore::RwV3d {
                    x: matrix[3][0],
                    y: matrix[3][1],
                    z: matrix[3][2],
                },
            })
            .collect(),
    }
}

pub fn RpSkinPluginAttach() -> bool {
    true
}

pub fn RpSkinGetNumBones(skin: &RpSkin) -> u32 {
    skin.num_bones
}

pub fn RpSkinGetVertexBoneWeights(skin: &RpSkin) -> &[RwMatrixWeights] {
    &skin.weights
}

pub fn RpSkinGetVertexBoneIndices(skin: &RpSkin) -> &[[u8; 4]] {
    &skin.indices
}

pub fn RpSkinGetSkinToBoneMatrices(skin: &RpSkin) -> &[RwMatrix] {
    &skin.inverse_matrices
}

pub fn RpSkinAtomicSetHAnimHierarchy(
    binding: &mut RpSkinAtomicBinding,
    hierarchy: RpHAnimHierarchy,
) {
    binding.hierarchy = Some(hierarchy);
}
