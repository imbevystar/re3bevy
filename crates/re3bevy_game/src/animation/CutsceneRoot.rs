//! Native cutscene placement and clump-root velocity are different inputs.
use super::{AnimBlendHierarchy::CAnimBlendHierarchy, RpAnimBlend::animation_frame_names_match};
use crate::math::Vector::CVector;
use re3bevy_formats::formats::ClumpRead::RwFrameDescriptor;

/// Immutable association binding, invalidated with its original DFF/IFP.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CutsceneAnimationRoot {
    pub sequence_index: usize,
    placement: CVector,
    start_translation: CVector,
}

impl CutsceneAnimationRoot {
    pub fn resolve(frames: &[RwFrameDescriptor], hierarchy: &CAnimBlendHierarchy) -> Option<Self> {
        let clump_root = frames.iter().position(|frame| frame.parent_index < 0)?;
        // RpAnimBlendClumpInitNotSkinned enumerates children, not the clump
        // frame itself. The first child owns VELOCITY_EXTRACTION_3D.
        let root = frames
            .iter()
            .find(|frame| frame.parent_index == clump_root as i32)?;
        let sequence_index = hierarchy
            .sequences
            .iter()
            .enumerate()
            .filter(|(_, sequence)| {
                sequence.num_frames() > 0
                    && (root
                        .name
                        .as_deref()
                        .is_some_and(|name| animation_frame_names_match(&sequence.name, name))
                        || (sequence.bone_tag >= 0
                            && root.hanim_id == Some(i32::from(sequence.bone_tag))))
            })
            .map(|(index, _)| index)
            .last()?;
        let placement_sequence = hierarchy.sequences.first()?;
        let root_sequence = &hierarchy.sequences[sequence_index];
        if !placement_sequence.has_translation() || !root_sequence.has_translation() {
            return None;
        }
        Some(Self {
            sequence_index,
            placement: placement_sequence.get_key_frame(0)?.translation(),
            start_translation: root_sequence.get_key_frame(0)?.translation(),
        })
    }

    pub fn position(
        &self,
        hierarchy: &CAnimBlendHierarchy,
        offset: CVector,
        time: f32,
    ) -> Option<CVector> {
        let sequence = hierarchy.sequences.get(self.sequence_index)?;
        let translation = sequence.get_key_frame_at_time(time)?.translation();
        // SetupCutsceneToStart places from hierarchy sequence zero. Thereafter
        // CCutsceneObject applies only the bound root's extracted delta. An
        // unmatched 'follow' sequence must not become an animated object root.
        Some(offset + self.placement + translation - self.start_translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::AnimBlendSequence::{AnimKeyFrame, CAnimBlendSequence, KeyFrameTrans},
        math::Quaternion::CQuaternion,
    };

    #[test]
    fn unmatched_follow_places_the_gun_but_bound_child_drives_its_motion() {
        let frame = |name: &str, parent_index| RwFrameDescriptor {
            name: Some(name.into()),
            parent_index,
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            at: [0.0, 0.0, 1.0],
            position: [0.0; 3],
            matrix_flags: 0,
            hanim_id: None,
        };
        let frames = [frame("colt1", -1), frame("colt45_lo", 0)];
        let sequence = |name: &str, start: f32, end: f32| {
            let mut sequence = CAnimBlendSequence::new();
            sequence.name = name.into();
            sequence.set_num_frames(2, true);
            sequence.key_frames = [start, end]
                .into_iter()
                .enumerate()
                .map(|(index, x)| {
                    AnimKeyFrame::Trans(KeyFrameTrans {
                        translation: CVector::new(x, 0.0, 0.0),
                        rotation: CQuaternion::IDENTITY,
                        delta_time: index as f32,
                    })
                })
                .collect();
            sequence
        };
        let mut hierarchy = CAnimBlendHierarchy::new();
        hierarchy.sequences = vec![
            sequence("follow", 100.0, 200.0),
            sequence("colt45_lo", 10.0, 20.0),
        ];
        let root = CutsceneAnimationRoot::resolve(&frames, &hierarchy).unwrap();
        assert_eq!(root.sequence_index, 1);
        assert_eq!(
            root.position(&hierarchy, CVector::new(2.0, 3.0, 4.0), 0.0),
            Some(CVector::new(102.0, 3.0, 4.0))
        );
        assert_eq!(
            root.position(&hierarchy, CVector::new(2.0, 3.0, 4.0), 1.0),
            Some(CVector::new(112.0, 3.0, 4.0))
        );
    }
}
