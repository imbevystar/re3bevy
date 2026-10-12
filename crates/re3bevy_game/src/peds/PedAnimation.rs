//! One actor-owned RpAnimBlend clump shared by gameplay and rendering.
//! re3 references: CPed::SetMoveAnim, CPlayerPed::SetRealMoveAnim,
//! CPed::CalculateNewVelocity and RpAnimBlendClumpUpdateAnimations.

use crate::animation::AnimBlendClumpData::CAnimBlendClumpData;
use bevy::prelude::Component;

/// One pending native animation sound, retired with its actor.
#[derive(Component)]
pub(crate) struct PedAnimationSound(pub u16);

#[derive(Debug, Clone, PartialEq, Component)]
pub struct PedAnimation {
    pub movement_anim_clump: CAnimBlendClumpData,
}

impl Default for PedAnimation {
    fn default() -> Self {
        Self {
            movement_anim_clump: CAnimBlendClumpData::new(),
        }
    }
}

impl PedAnimation {
    /// RpAnimBlendClumpInitNotSkinned: visit descendants of the original clump
    /// root, preserving frame names/local reset positions in the actor owner.
    pub(crate) fn ensure_original_frames(
        &mut self,
        model: usize,
        loader: &crate::core::FileLoader::CFileLoader,
    ) -> bool {
        use crate::animation::AnimBlendClumpData::{AnimBlendFrameData, VELOCITY_EXTRACTION};
        let clump = &mut self.movement_anim_clump;
        if clump
            .frames
            .iter()
            .any(|frame| !frame.frame_name.is_empty())
        {
            return true;
        }
        let Some(summary) = loader
            .model_info
            .get_model_info(model)
            .and_then(|model| model.base().clump_summary.as_ref())
        else {
            bevy::log::warn_once!(model, "ped_animation_waiting_installed_dff_frames");
            return false;
        };
        let Some(root) = summary
            .frames
            .iter()
            .position(|frame| frame.parent_index < 0)
        else {
            bevy::log::warn_once!(model, "ped_animation_invalid_original_dff_root");
            return false;
        };
        let mut children = vec![Vec::new(); summary.frames.len()];
        for (index, frame) in summary.frames.iter().enumerate() {
            if frame.parent_index >= 0 {
                let Some(parent) = children.get_mut(frame.parent_index as usize) else {
                    bevy::log::warn_once!(model, "ped_animation_invalid_original_dff_parent");
                    return false;
                };
                parent.push(index);
            }
        }
        let mut pending: Vec<_> = children[root].iter().rev().copied().collect();
        let mut frames = Vec::new();
        let mut visited = vec![false; summary.frames.len()];
        while let Some(index) = pending.pop() {
            if visited[index] {
                bevy::log::warn_once!(model, "ped_animation_invalid_original_dff_cycle");
                return false;
            }
            visited[index] = true;
            let frame = &summary.frames[index];
            if !frame
                .position
                .iter()
                .all(|coordinate| coordinate.is_finite())
            {
                bevy::log::warn_once!(model, "ped_animation_invalid_original_reset_position");
                return false;
            }
            frames.push(AnimBlendFrameData {
                flag: 0,
                reset_pos: re3bevy_core::core::Coordinates::CVector::new(
                    frame.position[0],
                    frame.position[1],
                    frame.position[2],
                ),
                frame_name: frame.name.clone().unwrap_or_default(),
                node_id: frame.hanim_id,
            });
            pending.extend(children[index].iter().rev().copied());
        }
        let Some(first) = frames.first_mut() else {
            bevy::log::warn_once!(model, "ped_animation_missing_original_child_frames");
            return false;
        };
        first.flag |= VELOCITY_EXTRACTION;
        clump.num_frames = frames.len();
        clump.frames = frames;
        clump.velocity2d = Some((0.0, 0.0));
        true
    }

    pub(crate) fn start_landing(
        &mut self,
        manager: &crate::animation::AnimManager::CAnimManager,
    ) -> Option<u16> {
        use crate::animation::{AnimAssocDefinitions::ASSOCGRP_STD, AnimationId::*};
        let falling = self
            .movement_anim_clump
            .associations
            .iter()
            .any(|association| association.anim_id == ANIM_STD_FALL);
        let id = if falling {
            ANIM_STD_FALL_COLLAPSE
        } else {
            ANIM_STD_FALL_LAND
        };
        let mut landing = manager.create_anim_association(ASSOCGRP_STD, id as usize)?;
        for association in &mut self.movement_anim_clump.associations {
            if association.is_partial() {
                association.blend_delta = -1000.0;
            }
        }
        landing.start(0.0);
        landing.set_finish_callback();
        self.movement_anim_clump.associations.insert(0, landing);
        Some(if falling {
            crate::audio::soundlist::SOUND_FALL_COLLAPSE as u16
        } else {
            crate::audio::soundlist::SOUND_FALL_LAND as u16
        })
    }

    /// CPed::PedSetInCarCB blends the authored seat association and then
    /// StopNonPartialAnims freezes its timeline, including CAR_SIT_P.
    pub(crate) fn install_seated_pose(
        &mut self,
        manager: &crate::animation::AnimManager::CAnimManager,
        passenger: bool,
        low_vehicle: bool,
    ) -> bool {
        use crate::animation::{AnimAssocDefinitions::ASSOCGRP_STD, AnimationId::*};
        let id = match (passenger, low_vehicle) {
            (true, true) => ANIM_STD_CAR_SIT_P_LO,
            (true, false) => ANIM_STD_CAR_SIT_P,
            (false, true) => ANIM_STD_CAR_SIT_LO,
            (false, false) => ANIM_STD_CAR_SIT,
        };
        if !self
            .movement_anim_clump
            .associations
            .iter()
            .any(|assoc| assoc.anim_id == id && assoc.blend_delta >= 0.0)
            && super::Ped::blend_ped_movement_animation(
                &mut self.movement_anim_clump,
                manager,
                ASSOCGRP_STD,
                id,
                100.0,
            )
            .is_none()
        {
            return false;
        }
        for association in &mut self.movement_anim_clump.associations {
            if !association.is_partial() {
                association.flags &= !crate::animation::AnimBlendAssociation::ASSOC_RUNNING;
                association.time_step = 0.0;
            }
        }
        true
    }

    /// CPed::PedSetOutCarCB / PedSetDraggedOutCarCB restart non-partial
    /// associations when the occupant leaves the seat. The vehicle get-out
    /// sequence lives in CPedVehicleAnimation; its former CAR_SIT association
    /// must no longer contribute to the on-foot clump after that callback.
    pub(crate) fn finish_vehicle_exit(&mut self) {
        use crate::animation::AnimationId::{
            ANIM_STD_CAR_SIT, ANIM_STD_CAR_SIT_LO, ANIM_STD_CAR_SIT_P,
            ANIM_STD_CAR_SIT_P_LO,
        };
        let clump = &mut self.movement_anim_clump;
        clump.associations.retain(|association| {
            let is_seated = matches!(
                association.anim_id,
                ANIM_STD_CAR_SIT | ANIM_STD_CAR_SIT_LO
                    | ANIM_STD_CAR_SIT_P | ANIM_STD_CAR_SIT_P_LO
            );
            let is_stale_fade = !association.is_partial()
                && association.blend_delta < 0.0
                && association.flags
                    & crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT != 0;
            !is_seated && !is_stale_fade
        });
        for association in &mut clump.associations {
            if !association.is_partial() {
                association.set_run();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peds::{Ped::CPed, PedType::ePedType};
    use bevy::prelude::World;
    use re3bevy_core::core::General::CGeneral;

    #[test]
    fn ped_materialization_requires_animation_and_retains_explicit_staging_state() {
        let mut world = World::new();
        let mut general = CGeneral::with_seed(1);
        let bare = world
            .spawn(CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general))
            .id();
        assert!(world.get::<PedAnimation>(bare).is_some());

        let mut animation = PedAnimation::default();
        animation.movement_anim_clump.velocity2d = Some((2.0, 3.0));
        let staged = world
            .spawn((
                CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general),
                animation,
            ))
            .id();
        assert_eq!(
            world
                .get::<PedAnimation>(staged)
                .unwrap()
                .movement_anim_clump
                .velocity2d,
            Some((2.0, 3.0))
        );
        assert!(world.despawn(staged));
        assert!(world.get::<PedAnimation>(staged).is_none());
    }

    fn manager_with_association(
        id: i32,
        flags: i32,
    ) -> crate::animation::AnimManager::CAnimManager {
        use crate::animation::{
            AnimBlendAssocGroup::CAnimBlendAssocGroup, AnimBlendAssociation::CAnimBlendAssociation,
            AnimBlendHierarchy::CAnimBlendHierarchy,
        };
        let mut manager = crate::animation::AnimManager::CAnimManager::initialise();
        let mut hierarchy = CAnimBlendHierarchy::new();
        hierarchy.total_length = 0.25;
        let mut association = CAnimBlendAssociation::new();
        association.init_with_hierarchy(hierarchy, 0);
        association.anim_id = id;
        association.flags = flags;
        manager.anim_assoc_groups.push(CAnimBlendAssocGroup {
            assoc_list: vec![association],
        });
        manager
    }

    #[test]
    fn passenger_pose_stops_at_entry_and_reuses_a_fading_seat_after_reentry() {
        use crate::animation::{
            AnimBlendAssociation::ASSOC_RUNNING, AnimationId::ANIM_STD_CAR_SIT_P,
        };
        let manager = manager_with_association(ANIM_STD_CAR_SIT_P, ASSOC_RUNNING);
        let mut animation = PedAnimation::default();
        assert!(animation.install_seated_pose(&manager, true, false));
        for _ in 0..90 {
            assert!(animation.install_seated_pose(&manager, true, false));
            animation.movement_anim_clump.associations[0].update_time(1.0 / 30.0, 1.0);
        }
        let seat = &mut animation.movement_anim_clump.associations[0];
        assert_eq!(
            seat.current_time, 0.0,
            "seated passenger must not replay close motion"
        );
        assert!(!seat.is_running());
        seat.blend_amount = 0.5;
        seat.blend_delta = -4.0;
        assert!(animation.install_seated_pose(&manager, true, false));
        let seat = &animation.movement_anim_clump.associations[0];
        assert!(
            seat.blend_delta > 0.0,
            "reentry revives the original seat association"
        );
        assert_eq!(animation.movement_anim_clump.associations.len(), 1);
    }

    #[test]
    fn vehicle_exit_retires_frozen_seat_and_restarts_on_foot_animation() {
        use crate::animation::{
            AnimBlendAssociation::{ASSOC_DELETEFADEDOUT, ASSOC_RUNNING},
            AnimationId::{ANIM_STD_CAR_SIT_P, ANIM_STD_IDLE, ANIM_STD_WALK},
        };
        let manager = manager_with_association(ANIM_STD_CAR_SIT_P, ASSOC_RUNNING);
        let mut animation = PedAnimation::default();
        assert!(animation.install_seated_pose(&manager, true, false));
        let mut idle = animation.movement_anim_clump.associations[0].clone();
        idle.anim_id = ANIM_STD_IDLE;
        idle.blend_amount = 0.35;
        idle.blend_delta = -4.0;
        idle.flags |= ASSOC_DELETEFADEDOUT;
        let mut walk = idle.clone();
        walk.anim_id = ANIM_STD_WALK;
        walk.blend_amount = 0.5;
        walk.blend_delta = 0.0;
        walk.flags &= !ASSOC_DELETEFADEDOUT;
        animation.movement_anim_clump.associations.extend([idle, walk]);

        animation.finish_vehicle_exit();

        assert_eq!(animation.movement_anim_clump.associations.len(), 1);
        let walk = &animation.movement_anim_clump.associations[0];
        assert_eq!(walk.anim_id, ANIM_STD_WALK);
        assert!(walk.is_running());
    }

    #[test]
    fn landing_callback_releases_control_on_the_same_actor_clump() {
        use crate::animation::{
            AnimBlendAssociation::ASSOC_PARTIAL, AnimationId::ANIM_STD_FALL_LAND,
        };
        let manager = manager_with_association(ANIM_STD_FALL_LAND, ASSOC_PARTIAL);
        let mut animation = PedAnimation::default();
        // This isolated callback test supplies its own synthetic clump;
        // production frame construction requires installed DFF descriptors.
        animation.movement_anim_clump.set_number_of_frames(1);
        animation.movement_anim_clump.frames[0].frame_name = "test_root".to_owned();
        let mut general = CGeneral::with_seed(3);
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion::default();
        let vitality = crate::peds::PedVitality::PedVitality::default();
        let loader = crate::core::FileLoader::CFileLoader::default();
        assert!(animation.start_landing(&manager).is_some());
        ped.flags.is_landing = true;
        for _ in 0..12 {
            crate::peds::Ped::update_non_player_movement_animation(
                &mut ped,
                &mut locomotion,
                &vitality,
                &mut animation,
                &manager,
                &loader,
                1.0 / 30.0,
                false,
                None,
            );
        }
        assert!(
            !ped.flags.is_landing,
            "PedLandCB must release the landing control gate"
        );
    }
}
