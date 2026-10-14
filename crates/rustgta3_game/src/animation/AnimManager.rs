//! re3 reference: references/re-GTA/src/animation/AnimManager.cpp
//! re3 reference: references/re-GTA/src/animation/AnimManager.h
//!
//! `CAnimManager` runtime metadata state.

use bevy::prelude::Resource;
use rustgta3_assets::assets::AnimationAssets::CAnimationAssets;
use rustgta3_formats::formats::AnimManager as Ifp;

use crate::{
    animation::{
        AnimAssocDefinitions::{AnimAssocDefinition, MS_A_ANIM_ASSOC_DEFINITIONS},
        AnimBlendAssocGroup::CAnimBlendAssocGroup,
        AnimBlendAssociation::CAnimBlendAssociation,
        AnimBlendHierarchy::CAnimBlendHierarchy,
        AnimBlendSequence::{AnimKeyFrame, CAnimBlendSequence, KeyFrame, KeyFrameTrans},
    },
    math::{Quaternion::CQuaternion, Vector::CVector},
};

#[derive(Debug, Clone, Resource)]
pub struct CAnimManager {
    pub anim_blocks: Vec<CAnimBlock>,
    pub animations: Vec<CAnimBlendHierarchy>,
    pub anim_assoc_groups: Vec<CAnimBlendAssocGroup>,
    pub anim_cache: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CAnimBlock {
    pub name: String,
    pub first_index: usize,
    pub num_anims: usize,
}

impl CAnimManager {
    pub fn initialise() -> Self {
        Self {
            anim_blocks: Vec::new(),
            animations: Vec::new(),
            anim_assoc_groups: Vec::new(),
            anim_cache: Vec::new(),
        }
    }

    pub fn load_anim_files(animation_assets: &CAnimationAssets) -> Self {
        let mut manager = Self::initialise();
        manager.load_anim_file_blocks(&animation_assets.ped_ifp.file);
        // re3 `CAnimManager::LoadAnimFiles`: after PED.IFP is resident, build
        // every association template group from `ms_aAnimAssocDefinitions`.
        manager.create_anim_assoc_groups();
        manager
    }

    fn load_anim_file_blocks(&mut self, file: &Ifp::IfpFile) {
        for block in &file.blocks {
            let first_index = self.animations.len();
            self.anim_blocks.push(CAnimBlock {
                name: block.name.clone(),
                first_index,
                num_anims: block.animations.len(),
            });
            for animation in &block.animations {
                self.animations.push(hierarchy_from_ifp(animation));
            }
        }
    }

    pub fn load_anim_file(&mut self, file: &Ifp::IfpFile) {
        self.load_anim_file_blocks(file);
        self.create_anim_assoc_groups();
    }

    fn create_anim_assoc_groups(&mut self) {
        self.anim_assoc_groups.clear();
        self.anim_assoc_groups
            .reserve(MS_A_ANIM_ASSOC_DEFINITIONS.len());
        for definition in MS_A_ANIM_ASSOC_DEFINITIONS {
            let block_hierarchies = self
                .get_animation_block(definition.block_name)
                .map(|block| {
                    self.animations
                        .iter()
                        .skip(block.first_index)
                        .take(block.num_anims)
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mut group = CAnimBlendAssocGroup::new();
            // `CreateAssociations` accepts absent IFP names, but an absent name
            // must not shift every following AnimationId/flag pair. Build each
            // descriptor atomically, as re3's association table does.
            for (name, descriptor) in definition.anim_names.iter().zip(definition.anim_descs) {
                let Some(hierarchy) = block_hierarchies
                    .iter()
                    .find(|hierarchy| hierarchy.name.eq_ignore_ascii_case(name))
                else {
                    continue;
                };
                let mut association = CAnimBlendAssociation::new();
                association.init_with_hierarchy(hierarchy.clone(), hierarchy.sequences.len());
                association.anim_id = descriptor.anim_id;
                association.flags = descriptor.flags;
                group.assoc_list.push(association);
            }
            self.anim_assoc_groups.push(group);
        }
    }

    /// re3 `GetAnimationGroup`: IDE PED.DAT names address the static
    /// `ms_aAnimAssocDefinitions` table, not the order models happened to load.
    pub fn assoc_group_id_by_name(&self, name: &str) -> Option<usize> {
        MS_A_ANIM_ASSOC_DEFINITIONS
            .iter()
            .position(|definition| definition.name.eq_ignore_ascii_case(name))
    }

    pub fn shutdown(&mut self) {
        self.anim_assoc_groups.clear();
        for animation in &mut self.animations {
            animation.shutdown();
        }
        self.animations.clear();
        self.anim_blocks.clear();
        self.anim_cache.clear();
    }

    pub fn uncompress_animation(&mut self, animation_index: usize) {
        if let Some(animation) = self.animations.get_mut(animation_index) {
            animation.uncompress();
            if !self.anim_cache.contains(&animation_index) {
                self.anim_cache.push(animation_index);
            }
        }
    }

    pub fn get_animation_block(&self, name: &str) -> Option<&CAnimBlock> {
        self.anim_blocks
            .iter()
            .find(|block| block.name.eq_ignore_ascii_case(name))
    }

    pub fn get_animation(&self, index: usize) -> Option<&CAnimBlendHierarchy> {
        self.animations.get(index)
    }

    pub fn get_animation_by_name(
        &self,
        name: &str,
        anim_block: &CAnimBlock,
    ) -> Option<&CAnimBlendHierarchy> {
        self.animations
            .iter()
            .skip(anim_block.first_index)
            .take(anim_block.num_anims)
            .find(|hierarchy| hierarchy.name.eq_ignore_ascii_case(name))
    }

    pub fn get_anim_assoc_definition(
        &self,
        group_id: usize,
    ) -> Option<&'static AnimAssocDefinition> {
        MS_A_ANIM_ASSOC_DEFINITIONS.get(group_id)
    }

    pub fn get_anim_group_name(&self, group_id: usize) -> Option<&'static str> {
        self.get_anim_assoc_definition(group_id)
            .map(|definition| definition.name)
    }

    pub fn remove_last_anim_file(&mut self) {
        if let Some(block) = self.anim_blocks.pop() {
            self.animations.truncate(block.first_index);
            self.anim_cache.retain(|index| *index < block.first_index);
        }
    }

    pub fn create_anim_association(
        &self,
        group_id: usize,
        anim_id: usize,
    ) -> Option<CAnimBlendAssociation> {
        self.anim_assoc_groups
            .get(group_id)?
            .copy_animation(anim_id)
    }

    pub fn get_anim_association(
        &self,
        group_id: usize,
        anim_id: usize,
    ) -> Option<&CAnimBlendAssociation> {
        self.anim_assoc_groups.get(group_id)?.get_animation(anim_id)
    }
}

impl Default for CAnimManager {
    fn default() -> Self {
        Self::initialise()
    }
}

pub(crate) fn hierarchy_from_ifp(animation: &Ifp::IfpAnimation) -> CAnimBlendHierarchy {
    let mut hierarchy = CAnimBlendHierarchy::new();
    hierarchy.set_name(&animation.name);
    hierarchy.sequences = animation.sequences.iter().map(sequence_from_ifp).collect();
    hierarchy.remove_quaternion_flips();
    hierarchy.calc_total_time();
    hierarchy
}

fn sequence_from_ifp(sequence: &Ifp::IfpSequence) -> CAnimBlendSequence {
    let mut anim_sequence = CAnimBlendSequence::new();
    anim_sequence.set_name(&sequence.name);
    anim_sequence.bone_tag = sequence.bone_tag.unwrap_or(-1) as i16;
    let has_translation = sequence
        .frames
        .iter()
        .any(|frame| frame.translation.is_some());
    anim_sequence.set_num_frames(sequence.frames.len(), has_translation);

    for (index, frame) in sequence.frames.iter().enumerate() {
        let rotation = CQuaternion::new(
            frame.rotation[0],
            frame.rotation[1],
            frame.rotation[2],
            frame.rotation[3],
        );
        if has_translation {
            let translation = frame.translation.unwrap_or([0.0, 0.0, 0.0]);
            anim_sequence.set_key_frame(
                index,
                AnimKeyFrame::Trans(KeyFrameTrans {
                    rotation,
                    delta_time: frame.delta_time,
                    translation: CVector::new(translation[0], translation[1], translation[2]),
                }),
            );
        } else {
            anim_sequence.set_key_frame(
                index,
                AnimKeyFrame::Rot(KeyFrame {
                    rotation,
                    delta_time: frame.delta_time,
                }),
            );
        }
    }

    anim_sequence
}
