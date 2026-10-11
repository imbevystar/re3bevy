//! Arena CPed audio identity, native event processing and unguarded retirement.
use crate::{ArenaCamera, ArenaCoopPlayerTwo, ArenaPlayer};
use bevy::prelude::*;
use re3bevy_core::core::Coordinates::CVector;
use re3bevy_game::{
    audio::{DMAudio::cDMAudio, audio_enums::AUDIOTYPE_PHYSICAL},
    peds::Ped::CPed,
};
use std::collections::{HashMap, HashSet};

type ArenaPedFilter = Or<(With<ArenaPlayer>, With<ArenaCoopPlayerTwo>)>;

pub(crate) fn process_arena_ped_audio(
    mut peds: Query<(Entity, &mut CPed), ArenaPedFilter>,
    dmaudio: Option<ResMut<cDMAudio>>,
    cameras: Query<(&Camera, &Transform), With<ArenaCamera>>,
) {
    let Some(mut dmaudio) = dmaudio else {
        if !peds.is_empty() {
            warn_once!("arena_ped_audio_missing_manager");
        }
        return;
    };
    let Some(listener) = cameras
        .iter()
        .find(|(camera, _)| camera.is_active)
        .map(|(_, transform)| CVector::from_bevy(transform.translation))
    else {
        if !peds.is_empty() {
            warn_once!("arena_ped_audio_missing_listener");
        }
        return;
    };
    for (entity, mut ped) in &mut peds {
        let owner = entity.to_bits() as i32;
        let id = ped.physical.audio_entity_id;
        if id >= 0
            && !dmaudio
                .AudioManager
                .m_asAudioEntities
                .get(id as usize)
                .is_some_and(|slot| {
                    slot.m_bIsUsed
                        && slot.m_nType == AUDIOTYPE_PHYSICAL
                        && slot.m_pEntity == Some(owner)
                })
        {
            warn_once!("arena_ped_audio_stale_slot_binding");
            ped.physical.audio_entity_id = -1;
        }
        if ped.physical.audio_entity_id < 0 {
            let id = dmaudio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(owner));
            ped.physical.audio_entity_id = id;
            if id < 0 {
                warn_once!("arena_ped_audio_waiting_for_physical_identity");
                continue;
            }
            dmaudio.SetEntityStatus(id, true);
        }
        re3bevy_game::audio::AudioLogic::process_single_ped_audio(
            &mut dmaudio,
            &mut ped,
            true,
            listener,
            owner,
        );
    }
}

/// Runs even after arena close/session disable. Derived bindings contain audio
/// identity only; canonical gameplay state remains on the live actor.
pub(crate) fn retire_arena_ped_audio(
    peds: Query<(Entity, &CPed), ArenaPedFilter>,
    dmaudio: Option<ResMut<cDMAudio>>,
    mut bindings: Local<HashMap<Entity, i32>>,
    mut live_slots: Local<HashSet<i32>>,
) {
    let Some(mut dmaudio) = dmaudio else {
        if !bindings.is_empty() {
            warn_once!("arena_ped_audio_retirement_missing_manager");
        }
        return;
    };
    live_slots.clear();
    for (_, ped) in &peds {
        if ped.physical.audio_entity_id >= 0 {
            live_slots.insert(ped.physical.audio_entity_id);
        }
    }
    bindings.retain(|entity, id| {
        if peds
            .get(*entity)
            .is_ok_and(|(_, ped)| ped.physical.audio_entity_id == *id)
        {
            return true;
        }
        if !live_slots.contains(id)
            && dmaudio
                .AudioManager
                .m_asAudioEntities
                .get(*id as usize)
                .is_some_and(|slot| {
                    slot.m_bIsUsed
                        && slot.m_nType == AUDIOTYPE_PHYSICAL
                        && slot.m_pEntity == Some(entity.to_bits() as i32)
                })
        {
            dmaudio.DestroyEntity(*id);
        }
        false
    });
    for (entity, ped) in &peds {
        if ped.physical.audio_entity_id >= 0 {
            bindings.insert(entity, ped.physical.audio_entity_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use re3bevy_game::peds::{PedAnimation::PedAnimation, PedVitality::PedVitality};

    #[test]
    fn both_actors_drain_footsteps_without_vehicle_inputs_and_retire_when_inactive() {
        for reused_owner in [false, true] {
            let mut app = App::new();
            let mut audio = cDMAudio::default();
            audio.Initialise();
            app.insert_resource(audio)
                .add_systems(Update, process_arena_ped_audio)
                .add_systems(PostUpdate, retire_arena_ped_audio);
            app.world_mut().spawn((
                Camera::default(),
                Transform::default(),
                ArenaCamera { local_player: 0 },
            ));
            let mut random = re3bevy_core::core::General::CGeneral::default();
            let mut actors = Vec::new();
            for player in 0..2 {
                let mut dto = re3bevy_game::peds::PlayerPed::CPlayerPed::new(&mut random, None);
                dto.ped.queued_footstep =
                    Some(re3bevy_game::audio::soundlist::SOUND_STEP_START as u16);
                let entity = app.world_mut().spawn(dto.into_bundle()).id();
                if player == 0 {
                    app.world_mut().entity_mut(entity).insert(ArenaPlayer {
                        horizontal_velocity: Vec2::ZERO,
                        vertical_velocity: 0.0,
                        grounded: true,
                        previous_position: Vec3::ZERO,
                    });
                } else {
                    app.world_mut()
                        .entity_mut(entity)
                        .insert(ArenaCoopPlayerTwo {
                            pad: Default::default(),
                            horizontal_velocity: Vec2::ZERO,
                            vertical_velocity: 0.0,
                            grounded: true,
                            previous_position: Vec3::ZERO,
                            camera_yaw: 0.0,
                            camera_pitch: 0.0,
                            camera_distance: 8.0,
                            controlled_vehicle: None,
                            vehicle_transition: None,
                        });
                }
                actors.push(entity);
            }
            app.update();
            let ids = actors
                .iter()
                .map(|entity| {
                    let ped = app.world().get::<CPed>(*entity).unwrap();
                    assert!(ped.queued_footstep.is_none());
                    assert!(app.world().get::<PedAnimation>(*entity).is_some());
                    assert_eq!(
                        app.world().get::<PedVitality>(*entity).unwrap().health,
                        100.0
                    );
                    assert!(ped.physical.audio_entity_id >= 0);
                    ped.physical.audio_entity_id
                })
                .collect::<Vec<_>>();
            assert_ne!(ids[0], ids[1]);
            if reused_owner {
                app.world_mut()
                    .resource_mut::<cDMAudio>()
                    .AudioManager
                    .m_asAudioEntities[ids[0] as usize]
                    .m_pEntity = Some(123456);
            }
            for entity in actors {
                app.world_mut().despawn(entity);
            }
            app.update();
            let audio = app.world().resource::<cDMAudio>();
            assert_eq!(
                audio.AudioManager.m_asAudioEntities[ids[0] as usize].m_bIsUsed, reused_owner,
                "retirement must preserve a slot now owned by another physical"
            );
            assert!(!audio.AudioManager.m_asAudioEntities[ids[1] as usize].m_bIsUsed);
        }
    }
}
