//! System ownership for the audio branch of re3 `CGame::Process`.
//! Cross-domain dependencies preserve the production schedule and deferred-command barriers.

use crate::audio::AudioLogic::process_explosion_audio;
use crate::audio::AudioLogic::process_fire_audio;
use crate::audio::AudioLogic::process_ped_audio;
use crate::audio::AudioLogic::process_script_audio;
use crate::audio::AudioLogic::process_vehicle_audio;
use crate::audio::AudioLogic::process_weather_audio;
use crate::audio::DMAudio::service_dmaudio;
use crate::audio::DMAudio::service_frontend_dmaudio;
use crate::audio::DMAudio::sync_audio_listener;
use crate::audio::DMAudio::sync_mission_audio_playback;
use crate::audio::DMAudio::sync_music_playback;
use crate::audio::DMAudio::sync_sfx_playback;
use crate::audio::PedTalk::service_ped_talk;
use crate::core::Game::frontend_or_pause_active;
use crate::objects::Object::process_script_vehicle_debris;
use crate::peds::PedChat::process_ambient_ped_chat;
use bevy::prelude::*;
use re3bevy_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

pub struct GtaAudioPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
enum SfxBackendSet {
    Playback,
}

impl Plugin for GtaAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::audio::AudioActorAccess::AudioActorDiagnostics>()
            .init_resource::<crate::audio::PedCommentPlaybackDiagnostics>()
            .init_resource::<crate::audio::PhysicalAudioLifecycle::PhysicalAudioBindings>()
            .init_resource::<crate::audio::AudioSpatial::SfxSpatialDiagnostics>()
            .init_resource::<crate::audio::AudioLogic::VehicleOneShotAudioState>()
            .init_resource::<crate::audio::AudioLogic::JumboAudioState>()
            .init_resource::<crate::audio::PlayerEngineAudio::PlayerEngineAudioState>()
            .add_systems(Update, crate::audio::PhysicalAudioLifecycle::retire_physical_audio
                .after(GtaProcessSet::World).before(GtaProcessSet::Audio))
            .add_systems(
                Update,
                sync_sfx_playback
                    .after(service_frontend_dmaudio)
                    .in_set(SfxBackendSet::Playback)
                    .run_if(frontend_or_pause_active),
            )
            .add_systems(
                Update,
                service_ped_talk
                    .after(process_script_vehicle_debris)
                    .after(process_ambient_ped_chat)
                    .in_set(GtaProcessSet::World)
                    .run_if(crate::core::World::world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                crate::audio::AudioActorAccess::allocate_vehicle_audio_entities
                    .before(process_vehicle_audio)
                    .in_set(GtaProcessSet::Audio),
            )
            .add_systems(Update, process_vehicle_audio.in_set(GtaProcessSet::Audio))
            .add_systems(Update, crate::audio::PhysicalAudioLifecycle::allocate_ped_audio_entities
                // CPed's constructor allocates before control/attack/talk can emit.
                // This lifecycle phase also runs while gameplay is paused.
                .after(crate::peds::Population::process_ambient_ped_generation)
                .before(crate::peds::PlayerPed::update_player_on_foot_control)
                .before(crate::peds::PedAI::process_script_ped_weapon_attacks)
                .before(service_ped_talk)
                .before(crate::audio::AudioLogic::queue_ped_animation_audio)
                .before(process_ped_audio))
            .add_systems(Update, crate::audio::AudioLogic::queue_ped_animation_audio
                .before(process_ped_audio).in_set(GtaProcessSet::Audio))
            .add_systems(Update, process_ped_audio.in_set(GtaProcessSet::Audio))
            .add_systems(Update, process_weather_audio.in_set(GtaProcessSet::Audio))
            .add_systems(Update, process_explosion_audio.in_set(GtaProcessSet::Audio))
            .add_systems(Update, process_fire_audio.in_set(GtaProcessSet::Audio))
            .add_systems(Update, process_script_audio.in_set(GtaProcessSet::Audio))
            .add_systems(
                Update,
                service_dmaudio
                    .after(process_vehicle_audio)
                    .after(process_ped_audio)
                    .after(process_weather_audio)
                    .after(process_explosion_audio)
                    .after(process_fire_audio)
                    .after(process_script_audio)
                    .in_set(GtaProcessSet::Audio),
            )
            .add_systems(
                Update,
                sync_audio_listener
                    .after(service_dmaudio)
                    .in_set(GtaProcessSet::Audio),
            )
            .add_systems(
                Update,
                sync_music_playback
                    .after(service_dmaudio)
                    .in_set(GtaProcessSet::Audio),
            )
            .add_systems(
                Update,
                sync_sfx_playback
                    .after(sync_audio_listener)
                    .in_set(SfxBackendSet::Playback)
                    .in_set(GtaProcessSet::Audio),
            )
            .add_systems(
                Update,
                sync_mission_audio_playback
                    .after(service_dmaudio)
                    .in_set(GtaProcessSet::Audio),
            )
            .add_systems(Update, crate::audio::AudioSpatial::normalize_sfx_spatial_backend
                .after(SfxBackendSet::Playback).after(sync_audio_listener));
    }
}
