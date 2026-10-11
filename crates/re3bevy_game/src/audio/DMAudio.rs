//! re3 reference: references/re-GTA/src/audio/DMAudio.cpp
//! re3 reference: references/re-GTA/src/audio/DMAudio.h

use std::collections::{HashMap, HashSet};

use bevy::{
    audio::{AudioSinkPlayback, SpatialAudioSink, SpatialListener, Volume},
    prelude::*,
};
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_core::core::{Coordinates::CVector, Timer::CTimer};

use crate::animation::CutsceneMgr::{CCutsceneMgr, CUTSCENE_AUDIO_TRACKS};
use crate::control::Script::{
    CTheScripts, ScriptCommandEffect, mission_audio_sample_from_key, script_key_to_string,
    script_sound_frontend_sound,
};
use crate::core::FileLoader::CFileLoader;
use crate::core::Pools::CPoolHandle;

use super::AudioManager::{cAudioManager, tSound};
use super::AudioSamples::{
    SFX_AIRPORT_ANNOUNCEMENT_1, SFX_CLUB_1, SFX_CLUB_12, SFX_DOCKS_FOGHORN, SFX_HOME_1,
    SFX_LAUNDERETTE_LOOP, SFX_SAWMILL_LOOP, SFX_SHOP_LOOP, SFX_WORKSHOP_1,
};
use super::AudioScriptObject::{cAudioScriptObject, cAudioScriptObjectPool};
use super::MusicManager::{MusicCommand, MusicServiceContext, cMusicManager};
use super::PolRadio::eCrimeType;
use super::audio_enums::*;
use super::sampman::{SampleMetadata, cSampleManager, parse_sfx_sdt};

/// Requested-voice identity used for CHANNEL_PLAYER_VEHICLE_ENGINE.
pub const PLAYER_ENGINE_VOICE_COUNTER: u32 = 65;

pub fn AEHANDLE_IS_FAILED(handle: i32) -> bool {
    handle < 0
}

pub fn AEHANDLE_IS_OK(handle: i32) -> bool {
    handle >= 0
}

#[derive(Debug, Clone, Default, Resource)]
pub struct cDMAudio {
    pub AudioManager: cAudioManager,
    pub MusicManager: cMusicManager,
    pub SampleManager: cSampleManager,
    pub dynamic_acoustic_modeling_status: bool,
    pub digital_handle_released: bool,
    pub speaker_config: i32,
    pub current_3d_provider: i8,
}

#[derive(Debug, Clone, Default, Resource)]
pub struct GtaAudioFrameIntents {
    pub samples: Vec<tSound>,
    pub voice_updates: Vec<tSound>,
    pub music_commands: Vec<MusicCommand>,
    pub destroyed_entities: Vec<i32>,
    pub stopped_engine_entities: Vec<i32>,
    pub stopped_voice_keys: Vec<(i32, u32)>,
}

/// Bevy backend state for re3's single mission-audio stream (stream slot 1).
/// The SCM-visible state remains owned by `cAudioManager`; this resource only
/// carries the decoded original GTA file and the live Bevy playback entity.
#[derive(Debug, Clone, Default, Resource)]
pub struct GtaMissionAudioPlayback {
    pub sample: Option<u32>,
    pub source: Option<Handle<AudioSource>>,
    pub entity: Option<Entity>,
    pub play_frames: u32,
}

#[derive(Component)]
pub(crate) struct GtaMissionAudioEntity;

/// Bevy backend for re3's primary streamed-audio channel (stream slot 0).
/// MusicManager remains authoritative for what should play; this resource only
/// owns decoded original files and the live Bevy entity/sink.
#[derive(Debug, Clone, Resource)]
pub struct GtaMusicPlayback {
    pub preloaded_track: Option<u8>,
    pub preloaded_source: Option<Handle<AudioSource>>,
    pub active_track: Option<u8>,
    pub entity: Option<Entity>,
    pub volume: u8,
    pub pan: u8,
    pub effects_path: bool,
    pub play_frames: u32,
    pub current_position_ms: u32,
    sources: HashMap<u8, Handle<AudioSource>>,
}

impl Default for GtaMusicPlayback {
    fn default() -> Self {
        Self {
            preloaded_track: None,
            preloaded_source: None,
            active_track: None,
            entity: None,
            volume: 127,
            pan: 63,
            effects_path: false,
            play_frames: 0,
            current_position_ms: 0,
            sources: HashMap::new(),
        }
    }
}

#[derive(Component)]
pub(crate) struct GtaMusicAudioEntity;

#[derive(Debug, Clone, Copy)]
struct LoopingVoice {
    entity: Entity,
    intro_entity: Option<Entity>,
    intro_samples_remaining: f32,
    sample: u32,
    stereo_side: Option<stereo::StereoSide>,
}
#[derive(Resource, Default)]
pub struct GtaSfxPlayback {
    descriptors: Vec<SampleMetadata>,
    raw: Vec<u8>,
    sources: HashMap<u32, Handle<AudioSource>>,
    loop_sources: HashMap<u32, Handle<AudioSource>>,
    stereo_sources: HashMap<stereo::StereoSourceKey, Handle<AudioSource>>,
    looping_voices: HashMap<(i32, u32), LoopingVoice>,
    finite_voices: HashMap<(i32, u32), Entity>,
    loaded: bool,
    pub missing_install_frames: u64,
    pub missing_descriptor_events: u64,
    pub invalid_pcm_events: u64,
    pub original_sdt_failures: u64,
    pub original_raw_failures: u64,
    pub invalid_sdt_failures: u64,
    pub completed_finite_voices: u64,
    pub completed_player_engine_voices: u64,
    pub unsupported_jumbo_pan_events: u64,
    pub stereo_rumble_voice_starts: u64,
}

/// Read the single native player acceleration channel from real Bevy voices.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PlayerEngineChannel<'w, 's> {
    playback: Option<Res<'w, GtaSfxPlayback>>,
    voices: Query<'w, 's, (Option<&'static AudioSink>, Option<&'static SpatialAudioSink>), With<GtaSfxVoice>>,
}
impl PlayerEngineChannel<'_, '_> {
    pub fn is_used(&self, audio_entity: i32) -> Option<bool> {
        let Some(playback) = self.playback.as_deref() else {
            bevy::log::warn_once!("player_engine_missing_sfx_backend");
            return None;
        };
        let Some(entity) = playback.finite_voices.get(&(audio_entity, PLAYER_ENGINE_VOICE_COUNTER)) else { return Some(false); };
        let Ok((sink, spatial_sink)) = self.voices.get(*entity) else { return Some(false); };
        // An accepted, deferred AudioPlayer has no sink yet. It owns the channel
        // until Bevy materializes playback, reports an empty sink, or retires it.
        Some(sink.is_none_or(|sink| !sink.empty()) && spatial_sink.is_none_or(|sink| !sink.empty()))
    }
}

fn sample_pcm<'a>(
    raw: &'a [u8],
    descriptor: SampleMetadata,
    loop_region: bool,
) -> Option<&'a [u8]> {
    let sample_start = descriptor.offset as usize;
    let sample_end = sample_start.checked_add(descriptor.size as usize)?;
    if sample_end > raw.len() {
        return None;
    }
    if loop_region && descriptor.loop_end >= 0 && descriptor.loop_end as u32 > descriptor.loop_start
    {
        let loop_start = sample_start.checked_add(descriptor.loop_start as usize)?;
        // re3 passes loop_start / 2 and loop_end / 2 directly to OpenAL SOFT
        // as sample offsets. loop_end is the point where playback jumps back
        // to loop_start, so the sustain PCM range is end-exclusive.
        let loop_end = sample_start
            .checked_add(descriptor.loop_end as usize)?
            .min(sample_end);
        if loop_start < loop_end && loop_start % 2 == 0 && loop_end % 2 == 0 {
            return raw.get(loop_start..loop_end);
        }
    }
    raw.get(sample_start..sample_end)
}

impl GtaSfxPlayback {
    pub(crate) fn original_sample_available(&self, sample: u32) -> bool {
        self.loaded && self.descriptors.get(sample as usize).is_some_and(|descriptor| {
            descriptor.frequency > 0 && descriptor.size > 0 && descriptor.size % 2 == 0
                && sample_pcm(&self.raw, *descriptor, false).is_some()
        })
    }
    pub(crate) fn load_sfx_data(&mut self, install: &OriginalGtaInstall) {
        if self.loaded {
            return;
        }
        let sdt = match install.read("AUDIO\\SFX.SDT") {
            Ok(bytes) => bytes,
            Err(error) => {
                self.original_sdt_failures += 1;
                bevy::log::warn_once!(%error, "sfx_backend_original_sdt_read_failed");
                return;
            }
        };
        let raw = match install.read("AUDIO\\SFX.RAW") {
            Ok(bytes) => bytes,
            Err(error) => {
                self.original_raw_failures += 1;
                bevy::log::warn_once!(%error, "sfx_backend_original_raw_read_failed");
                return;
            }
        };
        if sdt.is_empty() || sdt.len() % 20 != 0 {
            self.invalid_sdt_failures += 1;
            bevy::log::warn_once!(bytes = sdt.len(), "sfx_backend_invalid_original_sdt");
            return;
        }
        self.descriptors = parse_sfx_sdt(&sdt);
        self.raw = raw;
        self.loaded = true;
    }
}

/// PC re3 `StreamedNameTable` for stream-0 material. Mission dialogue after
/// END uses the separate SCM mission-audio path and is intentionally not
/// resolved here.
fn streamed_track_path(track: u8) -> Option<String> {
    let fixed = match track {
        0 => Some("AUDIO\\HEAD.WAV"),
        1 => Some("AUDIO\\CLASS.WAV"),
        2 => Some("AUDIO\\KJAH.WAV"),
        3 => Some("AUDIO\\RISE.WAV"),
        4 => Some("AUDIO\\LIPS.WAV"),
        5 => Some("AUDIO\\GAME.WAV"),
        6 => Some("AUDIO\\MSX.WAV"),
        7 => Some("AUDIO\\FLASH.WAV"),
        8 => Some("AUDIO\\CHAT.WAV"),
        // re3's USERTRACK slot falls back to HEAD.WAV when using the normal
        // streamed-name table.
        9 => Some("AUDIO\\HEAD.WAV"),
        10 => Some("AUDIO\\POLICE.WAV"),
        11 => Some("AUDIO\\CITY.WAV"),
        12 => Some("AUDIO\\WATER.WAV"),
        13 => Some("AUDIO\\COMOPEN.WAV"),
        14 => Some("AUDIO\\SUBOPEN.WAV"),
        93 => Some("AUDIO\\MISCOM.WAV"),
        94 => Some("AUDIO\\END.MP3"),
        _ => None,
    };
    if let Some(path) = fixed {
        return Some(path.to_owned());
    }

    let track_info = CUTSCENE_AUDIO_TRACKS
        .iter()
        .find(|entry| entry.track_id == i32::from(track))?;
    // The script/cutscene key is A1_SS0 (zero); the shipped PC audio filename
    // is A1_SSO (letter O), exactly as in re3 StreamedNameTable.
    let filename = if track == STREAMED_SOUND_CUTSCENE_ASUKA_1_SSO as u8 {
        "A1_SSO"
    } else {
        track_info.name
    };
    let extension = if matches!(track, 45..=49 | 70..=73) {
        "WAV"
    } else {
        "MP3"
    };
    Some(format!("AUDIO\\{filename}.{extension}"))
}

fn streamed_gain(playback: &GtaMusicPlayback, sample_manager: &cSampleManager) -> Volume {
    let (fade, master) = if playback.effects_path {
        (
            sample_manager.effects_fade_volume,
            sample_manager.effects_master_volume,
        )
    } else {
        (
            sample_manager.music_fade_volume,
            sample_manager.music_master_volume,
        )
    };
    // re3 cSampleManager::SetStreamedVolumeAndPan:
    //   effective = fade * requested * master >> 14
    // then CChannel::SetVolume divides by MAX_VOLUME (127).
    let effective =
        (u32::from(fade) * u32::from(playback.volume.min(127)) * u32::from(master)) >> 14;
    Volume::Linear((effective as f32 / 127.0).clamp(0.0, 1.0))
}

fn load_streamed_source(
    playback: &mut GtaMusicPlayback,
    install: &OriginalGtaInstall,
    audio_sources: &mut Assets<AudioSource>,
    track: u8,
) -> Option<Handle<AudioSource>> {
    if let Some(source) = playback.sources.get(&track) {
        return Some(source.clone());
    }
    let path = streamed_track_path(track)?;
    match install.read(&path) {
        Ok(bytes) => {
            let source = audio_sources.add(AudioSource {
                bytes: bytes.into(),
            });
            playback.sources.insert(track, source.clone());
            Some(source)
        }
        Err(error) => {
            warn!("re3 streamed audio track {track} failed to load '{path}': {error}");
            None
        }
    }
}

#[derive(Component)]
pub struct GtaSfxVoice;

/// Native channel identity retained on the Bevy playback entity for inspection.
#[derive(Component)]
pub struct GtaSfxSample {
    pub sample_index: u32,
    pub audio_entity: i32,
    pub counter: u32,
}

#[derive(Component)]
pub struct GtaAudioListener;

/// Match re3's OpenAL EXTERNAL_3D_SOUND gain path. `m_nEmittingVolume` has
/// already passed through `AdjustSamplesVolume`, including the final-quarter
/// fade; this applies the OpenAL inverse-distance part with reference distance
/// = max / 4.
#[inline]
fn re3_sfx_gain(sound: &tSound) -> f32 {
    if sound.m_bIs2D {
        return f32::from(sound.m_nVolume) / 127.0;
    }
    if sound.m_nEmittingVolume == 0 || sound.m_MaxDistance <= 0.0 {
        return 0.0;
    }

    let reference_distance = sound.m_MaxDistance * 0.25;
    if reference_distance <= 0.0 {
        return f32::from(sound.m_nEmittingVolume) / 127.0;
    }
    let clamped_distance = sound
        .m_fDistance
        .max(reference_distance)
        .min(sound.m_MaxDistance);
    let attenuation = (reference_distance / clamped_distance).clamp(0.0, 1.0);
    f32::from(sound.m_nEmittingVolume) / 127.0 * attenuation
}

/// cSampleManager::SetChannelEmittingVolume -> CChannel::SetVolume. Quantize
/// the requested source gain before OpenAL distance attenuation, including
/// the authored cutscene music exceptions.
struct SfxGainSettings { fade: u8, master: u8, duck: bool }
impl SfxGainSettings {
    fn from_audio(audio: &cDMAudio) -> Self {
        let track = audio.MusicManager.GetNextTrack();
        Self {
            fade: audio.SampleManager.effects_fade_volume,
            master: audio.SampleManager.effects_master_volume,
            duck: audio.MusicManager.GetMusicMode() == MUSICMODE_CUTSCENE as u32
                && track != STREAMED_SOUND_NEWS_INTRO as u8 && track != STREAMED_SOUND_CUTSCENE_SAL4_BDBD as u8,
        }
    }
    fn gain(&self, sound: &tSound) -> f32 {
        let source = if sound.m_bIs2D { sound.m_nVolume } else { sound.m_nEmittingVolume }.min(127);
        if source == 0 { return 0.0; }
        let channel = if self.duck { source / 4 } else { source };
        let effective = (u32::from(self.fade) * u32::from(channel) * u32::from(self.master)) >> 14;
        effective as f32 / f32::from(source) * re3_sfx_gain(sound)
    }
}

#[inline]
fn is_engine_counter(counter: u32) -> bool {
    matches!(counter, 2 | 52 | 61 | 62 | 63 | 64 | 65)
}

fn audio_trace_sample_name(sound: &tSound) -> &'static str {
    if is_engine_counter(sound.m_nCounter) {
        return "vehicle-engine";
    }
    match sound.m_nSampleIndex as i32 {
        SFX_WORKSHOP_1 => "workshop",
        SFX_SAWMILL_LOOP => "sawmill",
        SFX_LAUNDERETTE_LOOP => "launderette",
        SFX_SHOP_LOOP => "shop",
        SFX_AIRPORT_ANNOUNCEMENT_1 => "airport",
        SFX_DOCKS_FOGHORN => "docks",
        SFX_HOME_1 => "home",
        sample if (SFX_CLUB_1..=SFX_CLUB_12).contains(&sample) => "club/party",
        _ => "loop",
    }
}

pub fn sync_audio_listener(
    mut commands: Commands,
    camera_res: Option<Res<crate::core::Camera::CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    cameras: Query<(&Camera, &Transform), Without<GtaAudioListener>>,
    mut listeners: Query<(&mut Transform, Option<&mut GlobalTransform>), With<GtaAudioListener>>,
) {
    let position = camera_res
        .as_ref()
        .map(|camera| camera.position.to_bevy())
        .or_else(|| {
            cameras
                .iter()
                .find(|(camera, _)| camera.is_active)
                .map(|(_, transform)| transform.translation)
        })
        .or_else(|| {
            file_loader
                .as_ref()
                .map(|loader| loader.renderer.camera_position.to_bevy())
        })
        .unwrap_or(Vec3::ZERO);
    // Stereo ears follow the camera orientation as well as its position.
    let rotation = cameras
        .iter()
        .find(|(camera, _)| camera.is_active)
        .map(|(_, transform)| transform.rotation)
        .unwrap_or(Quat::IDENTITY);
    let listener = Transform {
        translation: position,
        rotation,
        scale: Vec3::ONE,
    };
    if listeners.is_empty() {
        commands.spawn((
            SpatialListener::default(),
            listener,
            GlobalTransform::from(listener),
            GtaAudioListener,
            Name::new("re3 GTA audio listener"),
        ));
        return;
    }
    for (mut transform, mut maybe_global) in &mut listeners {
        transform.translation = position;
        transform.rotation = rotation;
        if let Some(ref mut global) = maybe_global {
            **global = GlobalTransform::from(*transform);
        }
    }
}

pub fn service_dmaudio(
    dmaudio: Option<ResMut<cDMAudio>>,
    timer: Option<Res<CTimer>>,
    music_playback: Option<Res<GtaMusicPlayback>>,
    cutscene: Option<ResMut<CCutsceneMgr>>,
    pads: Option<Res<crate::core::Pad::CPads>>,
    replay: Option<Res<crate::control::Replay::CReplay>>,
    player_query: Query<
        (
            &crate::peds::Ped::CPed,
            Option<&crate::peds::VehicleRelations::InVehicle>,
        ),
        With<crate::peds::PlayerPed::PlayerPedState>,
    >,
    mut vehicles: Query<(
        &crate::vehicles::Vehicle::VehicleCore,
        &mut crate::vehicles::Vehicle::VehicleAudioState,
    )>,
    mut frame_intents: ResMut<GtaAudioFrameIntents>,
    original_sfx: Option<Res<GtaSfxPlayback>>,
    mut comments_diagnostics: ResMut<super::PedCommentPlaybackDiagnostics>,
) {
    let Some(mut dmaudio) = dmaudio else {
        frame_intents.samples.clear();
        frame_intents.music_commands.clear();
        return;
    };
    let user_paused = timer
        .as_ref()
        .is_some_and(|timer| timer.get_is_user_paused());
    dmaudio.AudioManager.release_time_step = timer.as_ref().map_or(1.0, |timer| timer.get_time_step_fix());
    dmaudio.AudioManager.SyncUserPauseState(user_paused);
    let mut player_vehicle_entity = None;
    let mut player_alive = true;
    let mut player_in_car = false;
    let mut player_vehicle_radio = crate::audio::audio_enums::HEAD_RADIO as u8;
    let mut player_vehicle_uses_police_radio = false;
    if let Some((player, in_vehicle)) = player_query.iter().find(|(_, relation)| relation.is_some())
        && let Some(in_vehicle) = in_vehicle
        && let Ok((base, audio)) = vehicles.get_mut(in_vehicle.vehicle)
    {
        let model = base.physical.entity.model_index.max(0) as usize;
        player_vehicle_uses_police_radio = matches!(
            model,
            crate::modelinfo::ModelIndices::MI_FBICAR
                | crate::modelinfo::ModelIndices::MI_POLICE
                | crate::modelinfo::ModelIndices::MI_ENFORCER
                | crate::modelinfo::ModelIndices::MI_PREDATOR
                | crate::modelinfo::ModelIndices::MI_RHINO
                | crate::modelinfo::ModelIndices::MI_BARRACKS
        );
        player_in_car = true;
        player_alive = !player.dying_or_dead();
        player_vehicle_radio = audio.radio_station;
        player_vehicle_entity = Some(in_vehicle.vehicle);
    }
    let replay_playback = replay
        .as_deref()
        .is_some_and(|replay| replay.mode == crate::control::Replay::ReplayMode::Playback);
    let change_station_just_down = player_in_car
        && player_alive
        && !replay_playback
        && !player_vehicle_uses_police_radio
        && pads
            .as_deref()
            .is_some_and(|pads| pads.get_pad(0).change_station_just_down());

    let context = MusicServiceContext {
        time_ms_pause_mode: timer
            .as_ref()
            .map(|timer| timer.time_in_milliseconds_pause_mode())
            .unwrap_or(0),
        player_in_car,
        player_vehicle_radio,
        player_vehicle_uses_police_radio,
        mp3_radio_available: dmaudio.IsMP3RadioChannelAvailable(),
        stream_playing: dmaudio.SampleManager.IsStreamPlaying(0),
        streamed_file_position: music_playback
            .as_ref()
            .map_or(0, |playback| playback.current_position_ms),
        change_station_just_down,
        ..default()
    };
    if change_station_just_down {
        let front_end_entity = dmaudio.AudioManager.m_nFrontEndEntity;
        dmaudio.AudioManager.PlayOneShot(
            front_end_entity,
            crate::audio::soundlist::SOUND_FRONTEND_RADIO_CHANGE as u16,
            1.0,
        );
    }
    if let Some(mut cutscene) = cutscene {
        for action in cutscene.drain_audio_actions() {
            match action {
                crate::animation::CutsceneMgr::CutsceneAudioAction::EnterMusicModeAndPreload {
                    track_id,
                } => {
                    dmaudio.ChangeMusicMode(
                        crate::audio::audio_enums::MUSICMODE_CUTSCENE as u8,
                        &context,
                    );
                    if let Some(track_id) = track_id {
                        dmaudio.PreloadCutSceneMusic(track_id);
                    }
                }
                crate::animation::CutsceneMgr::CutsceneAudioAction::PlayPreloaded => {
                    dmaudio.PlayPreloadedCutSceneMusic();
                }
                crate::animation::CutsceneMgr::CutsceneAudioAction::Stop { restore_game_mode } => {
                    dmaudio.StopCutSceneMusic();
                    if restore_game_mode {
                        dmaudio.ChangeMusicMode(
                            crate::audio::audio_enums::MUSICMODE_GAME as u8,
                            &context,
                        );
                    }
                }
            }
        }
    }
    if let Some(timer) = timer.as_deref() {
        let load_context = super::sampman::PedCommentLoadContext {
            code_paused: timer.get_is_code_paused(),
            cutscene_music: dmaudio.MusicManager.IsInitialised() && dmaudio.MusicManager.GetMusicMode() == super::audio_enums::MUSICMODE_CUTSCENE as u32,
            game_completed_music: dmaudio.MusicManager.IsInitialised()
                && dmaudio.MusicManager.GetMusicMode() == super::audio_enums::MUSICMODE_FRONTEND as u32
                && dmaudio.MusicManager.GetNextTrack() == super::audio_enums::STREAMED_SOUND_GAME_COMPLETED as u8,
        };
        if original_sfx.is_none() {
            comments_diagnostics.missing_backend_frames += 1;
            warn_once!("ped_comment_waiting_original_sfx_backend");
        }
        let available = |sample| original_sfx.as_deref().is_some_and(|source| source.original_sample_available(sample));
        let cDMAudio { AudioManager, SampleManager, MusicManager, .. } = &mut *dmaudio;
        AudioManager.service_with_ped_comments(Some(super::PedCommentPlayback::PedCommentService {
            samples: SampleManager, load_context, original_sample_available: &available,
            slow_motion: timer.get_is_slow_motion_active(), diagnostics: &mut comments_diagnostics,
        }));
        MusicManager.Service(&context);
    } else {
        comments_diagnostics.missing_timer_frames += 1;
        warn_once!("ped_comment_waiting_canonical_timer");
        dmaudio.Service(&context);
    }
    if let (Some(vehicle_entity), Some(station)) = (
        player_vehicle_entity,
        dmaudio.MusicManager.take_pending_vehicle_radio_update(),
    ) && let Ok((_, mut audio)) = vehicles.get_mut(vehicle_entity)
    {
        audio.radio_station = station;
    }
    *frame_intents = dmaudio.drain_frame_intents();
}

/// Frontend/pause counterpart of `service_dmaudio`.
///
/// re3 continues servicing the frontend audio entity while menus are active,
/// even though gameplay audio producers are paused. Our normal GTA audio set
/// only runs in `InGame`, so menu one-shots otherwise accumulate on the
/// frontend entity and never reach Bevy's playback backend.
pub(crate) fn service_frontend_dmaudio(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut frame_intents: ResMut<GtaAudioFrameIntents>,
) {
    let Some(dmaudio) = dmaudio.as_deref_mut() else {
        frame_intents.samples.clear();
        return;
    };

    super::AudioLogic::process_front_end_audio(&mut dmaudio.AudioManager);
    dmaudio.AudioManager.Service();
    *frame_intents = dmaudio.drain_frame_intents();
}

/// Pause transition counterpart of re3 `cAudioManager::Service`: gameplay SFX
/// channels are stopped immediately, while streamed audio is paused in place.
/// The normal audio service does not run in our `PauseMenu` app state, so this
/// transition system must perform the first paused audio frame explicitly.
pub(crate) fn enter_audio_pause(
    mut commands: Commands,
    mut timer: Option<ResMut<CTimer>>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut frame_intents: ResMut<GtaAudioFrameIntents>,
    mut sfx_playback: ResMut<GtaSfxPlayback>,
    sfx_entities: Query<Entity, With<GtaSfxVoice>>,
    mut streams: Query<
        &mut AudioSink,
        Or<(With<GtaMusicAudioEntity>, With<GtaMissionAudioEntity>)>,
    >,
) {
    if let Some(timer) = timer.as_deref_mut() {
        timer.start_user_pause();
    }
    if let Some(dmaudio) = dmaudio.as_deref_mut() {
        dmaudio.AudioManager.SyncUserPauseState(true);
    }

    // Generic re3 channels are stopped, not resumed.  AudioLogic will rebuild
    // only still-valid loops on the first gameplay frame after the menu closes.
    for entity in &sfx_entities {
        commands.entity(entity).despawn();
    }
    sfx_playback.looping_voices.clear();
    sfx_playback.finite_voices.clear();
    frame_intents.samples.clear();
    frame_intents.voice_updates.clear();
    frame_intents.music_commands.clear();
    frame_intents.destroyed_entities.clear();
    frame_intents.stopped_engine_entities.clear();
    frame_intents.stopped_voice_keys.clear();

    // PC re3 pauses mission stream 1.  Pausing stream 0 as well is required in
    // this backend because PauseMenu suspends MusicManager service entirely;
    // otherwise the last radio/cutscene buffer keeps playing behind the menu.
    for sink in &mut streams {
        sink.pause();
    }
}

pub(crate) fn exit_audio_pause(
    mut timer: Option<ResMut<CTimer>>,
    mut streams: Query<
        &mut AudioSink,
        Or<(With<GtaMusicAudioEntity>, With<GtaMissionAudioEntity>)>,
    >,
) {
    if let Some(timer) = timer.as_deref_mut() {
        timer.end_user_pause();
    }
    for sink in &mut streams {
        sink.play();
    }
}

pub(crate) fn sync_music_playback(
    mut commands: Commands,
    install: Option<Res<OriginalGtaInstall>>,
    frame_intents: Res<GtaAudioFrameIntents>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut playback: ResMut<GtaMusicPlayback>,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    mut sinks: Query<(Entity, Option<&mut AudioSink>), With<GtaMusicAudioEntity>>,
) {
    let Some(dmaudio) = dmaudio.as_deref_mut() else {
        return;
    };

    let mut stop_requested = false;
    let mut start_request: Option<(u8, Handle<AudioSource>, u32)> = None;
    for command in &frame_intents.music_commands {
        match *command {
            MusicCommand::StopStreamedFile => {
                stop_requested = true;
                start_request = None;
            }
            MusicCommand::PreloadStreamedFile { track } => {
                let source = playback.sources.get(&track).cloned().or_else(|| {
                    install.as_deref().and_then(|install| {
                        load_streamed_source(&mut playback, install, &mut audio_sources, track)
                    })
                });
                playback.preloaded_track = source.as_ref().map(|_| track);
                playback.preloaded_source = source;
            }
            MusicCommand::StartPreloadedStreamedFile => {
                if let (Some(track), Some(source)) =
                    (playback.preloaded_track, playback.preloaded_source.clone())
                {
                    start_request = Some((track, source, 0));
                }
            }
            MusicCommand::StartStreamedFile { track, position } => {
                let source = playback.sources.get(&track).cloned().or_else(|| {
                    install.as_deref().and_then(|install| {
                        load_streamed_source(&mut playback, install, &mut audio_sources, track)
                    })
                });
                if let Some(source) = source {
                    start_request = Some((track, source, position));
                }
            }
            MusicCommand::SetStreamVolumeAndPan {
                volume,
                pan,
                front_end,
            } => {
                playback.volume = volume.min(127);
                playback.pan = pan.min(127);
                playback.effects_path = front_end;
            }
        }
    }

    if stop_requested || start_request.is_some() {
        if let Some(entity) = playback.entity.take() {
            if let Ok((_, Some(sink))) = sinks.get_mut(entity) {
                sink.stop();
            }
            commands.entity(entity).despawn();
        }
        playback.active_track = None;
        playback.play_frames = 0;
        playback.current_position_ms = 0;
        dmaudio.SampleManager.StopStreamedFile(0);
    }

    if let Some((track, source, position)) = start_request {
        let gain = streamed_gain(&playback, &dmaudio.SampleManager);
        let mut settings = PlaybackSettings::ONCE.with_volume(gain);
        if position != 0 {
            settings =
                settings.with_start_position(std::time::Duration::from_millis(u64::from(position)));
        }
        let entity = commands
            .spawn((
                AudioPlayer(source),
                settings,
                GtaMusicAudioEntity,
                Name::new("re3 Streamed Audio 0"),
            ))
            .id();
        playback.entity = Some(entity);
        playback.active_track = Some(track);
        playback.play_frames = 0;
        playback.current_position_ms = position;
        let _ = dmaudio.SampleManager.StartStreamedFile(u32::from(track), 0);
        return;
    }

    let Some(entity) = playback.entity else {
        return;
    };
    let Ok((_, Some(mut sink))) = sinks.get_mut(entity) else {
        return;
    };

    playback.play_frames = playback.play_frames.saturating_add(1);
    playback.current_position_ms = sink.position().as_millis().min(u128::from(u32::MAX)) as u32;
    sink.set_volume(streamed_gain(&playback, &dmaudio.SampleManager));

    if playback.play_frames > 10 && sink.empty() {
        dmaudio.SampleManager.StopStreamedFile(0);
        commands.entity(entity).despawn();
        playback.entity = None;
        playback.active_track = None;
        playback.play_frames = 0;
    }
}

pub fn sync_sfx_playback(
    mut commands: Commands,
    install: Option<Res<OriginalGtaInstall>>,
    frame_intents: Res<GtaAudioFrameIntents>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    time: Option<Res<bevy::prelude::Time>>,
    mut playback: ResMut<GtaSfxPlayback>,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    mut sinks: Query<
        (
            Option<&mut AudioSink>,
            Option<&mut SpatialAudioSink>,
            &mut Transform,
            Option<&mut GlobalTransform>,
        ),
        With<GtaSfxVoice>,
    >,
    voice_entities: Query<Entity, With<GtaSfxVoice>>,
    mut seen: Local<HashSet<(i32, u32)>>,
    mut expired: Local<Vec<(i32, u32)>>,
) {
    if !playback.loaded {
        let Some(install) = install else {
            playback.missing_install_frames += 1;
            bevy::log::warn_once!("sfx_backend_missing_original_install");
            return;
        };
        playback.load_sfx_data(&install);
    }

    let Some(audio_settings) = dmaudio.as_deref() else {
        if !frame_intents.samples.is_empty() { warn_once!("sfx_backend_missing_audio_manager"); }
        return;
    };
    // Gain inputs are invariant for this backend invocation. The narrow
    // projection avoids retaining a mutable gameplay manager borrow below.
    let gain_settings = SfxGainSettings::from_audio(audio_settings);
    let channel_gain = |sound: &tSound| gain_settings.gain(sound);
    if std::env::var_os("RE3BEVY_AUDIO_TRACE").is_some()
        && dmaudio
            .as_ref()
            .is_some_and(|audio| audio.AudioManager.m_FrameCounter % 60 == 0)
    {
        for sound in frame_intents
            .samples
            .iter()
            .filter(|sound| sound.m_nLoopCount == 0 && channel_gain(sound) >= 0.01)
        {
            bevy::log::info!(target: "re3bevy::audio",
                "AUDIO LOOP kind={} entity={} counter={} sample={} pos=({:.1},{:.1},{:.1}) dist={:.1}/{:.1} vol={} emit={} gain={:.3} freq={}",
                audio_trace_sample_name(sound),
                sound.m_nEntityIndex,
                sound.m_nCounter,
                sound.m_nSampleIndex,
                sound.m_vecPos.x,
                sound.m_vecPos.y,
                sound.m_vecPos.z,
                sound.m_fDistance,
                sound.m_MaxDistance,
                sound.m_nVolume,
                sound.m_nEmittingVolume,
                channel_gain(sound),
                sound.m_nFrequency,
            );
        }
    }
    let delta_seconds = time.as_ref().map_or(1.0 / 60.0, |time| time.delta_secs());
    let mut completed_finite = 0;
    let mut completed_player_engine = 0;
    playback
        .finite_voices
        .retain(|&(audio_entity, counter), entity| {
            let sink_finished = sinks.get_mut(*entity).ok().is_some_and(|(sink, spatial, _, _)| {
                sink.as_ref().is_some_and(|sink| sink.empty())
                    || spatial.as_ref().is_some_and(|sink| sink.empty())
            });
            if voice_entities.get(*entity).is_ok() && !sink_finished {
                return true;
            }
            let explicitly_stopped = frame_intents.destroyed_entities.contains(&audio_entity)
                || frame_intents.stopped_engine_entities.contains(&audio_entity)
                || frame_intents.stopped_voice_keys.contains(&(audio_entity, counter));
            if !explicitly_stopped {
                completed_finite += 1;
                if counter == PLAYER_ENGINE_VOICE_COUNTER { completed_player_engine += 1; }
            }
            // A fresh producer request may reuse a wrapping one-shot counter.
            // Only acknowledge completion of the previous, unreplaced channel.
            if !frame_intents
                .samples
                .iter()
                .any(|sound| sound.m_nEntityIndex == audio_entity && sound.m_nCounter == counter)
                && let Some(audio) = dmaudio.as_deref_mut()
            {
                audio.AudioManager.finish_voice(audio_entity, counter);
            }
            false
        });
    playback.completed_finite_voices += completed_finite;
    playback.completed_player_engine_voices += completed_player_engine;
    for &id in &frame_intents.destroyed_entities {
        playback.looping_voices.retain(|&(entity_id, _), voice| {
            if entity_id == id {
                commands.entity(voice.entity).despawn();
                if let Some(intro) = voice.intro_entity {
                    commands.entity(intro).despawn();
                }
                false
            } else {
                true
            }
        });
        playback.finite_voices.retain(|&(entity_id, _), entity| {
            if entity_id == id {
                if voice_entities.get(*entity).is_ok() {
                    commands.entity(*entity).despawn();
                }
                false
            } else {
                true
            }
        });
    }
    for &entity_id in &frame_intents.stopped_engine_entities {
        for cnt in [2, 52, 61, 62, 63, 64, 65] {
            if let Some(voice) = playback.looping_voices.remove(&(entity_id, cnt)) {
                commands.entity(voice.entity).despawn();
                if let Some(intro) = voice.intro_entity {
                    commands.entity(intro).despawn();
                }
            }
            if let Some(entity) = playback.finite_voices.remove(&(entity_id, cnt))
                && voice_entities.get(entity).is_ok()
            {
                commands.entity(entity).despawn();
            }
        }
    }
    for &key in &frame_intents.stopped_voice_keys {
        if let Some(voice) = playback.looping_voices.remove(&key) {
            commands.entity(voice.entity).despawn();
            if let Some(intro) = voice.intro_entity {
                commands.entity(intro).despawn();
            }
        }
        if let Some(entity) = playback.finite_voices.remove(&key)
            && voice_entities.get(entity).is_ok()
        {
            commands.entity(entity).despawn();
        }
    }
    for sound in &frame_intents.voice_updates {
        let key = (sound.m_nEntityIndex, sound.m_nCounter);
        let entity = playback
            .finite_voices
            .get(&key)
            .copied()
            .or_else(|| playback.looping_voices.get(&key).map(|voice| voice.entity));
        let Some(entity) = entity else {
            continue;
        };
        let Some(descriptor) = playback.descriptors.get(sound.m_nSampleIndex as usize) else {
            playback.missing_descriptor_events += 1;
            bevy::log::warn_once!(sample = sound.m_nSampleIndex, "sfx_voice_update_missing_descriptor");
            continue;
        };
        let volume = Volume::Linear(channel_gain(sound).clamp(0.0, 1.0));
        let speed =
            (sound.m_nFrequency.max(1) as f32 / descriptor.frequency.max(1) as f32).clamp(0.1, 4.0);
        let transform = Transform::from_translation(sound.m_vecPos.to_bevy());
        if let Ok((sink_2d, sink_3d, mut voice_transform, mut maybe_global)) = sinks.get_mut(entity)
        {
            if let Some(mut sink) = sink_2d {
                sink.set_volume(volume);
                sink.set_speed(speed);
            }
            if let Some(mut sink) = sink_3d {
                sink.set_volume(volume);
                sink.set_speed(speed);
            }
            *voice_transform = transform;
            if let Some(ref mut global) = maybe_global {
                **global = GlobalTransform::from(transform);
            }
        }
    }
    seen.clear();
    for sound in &frame_intents.samples {
        let sample = sound.m_nSampleIndex;
        let stereo_side = if sample == super::AudioSamples::SFX_JUMBO_RUMBLE as u32 && sound.m_bIs2D {
            let Some(side) = stereo::StereoSide::from_native_pan(sound.m_nPan) else {
                playback.unsupported_jumbo_pan_events += 1;
                bevy::log::warn_once!(pan = sound.m_nPan, "jumbo_audio_invalid_native_pan_endpoint");
                continue;
            };
            Some(side)
        } else { None };
        let Some(descriptor) = playback.descriptors.get(sample as usize).copied() else {
            playback.missing_descriptor_events += 1;
            bevy::log::warn_once!(sample, "sfx_backend_missing_original_sample_descriptor");
            continue;
        };
        let use_loop_region = sound.m_nLoopCount == 0
            && descriptor.loop_end >= 0
            && descriptor.loop_end as u32 > descriptor.loop_start;
        let full_source = if let Some(side) = stereo_side {
            let Some(source) = stereo::stereo_source(&mut playback, &mut audio_sources, sample, descriptor, false, side) else { continue; };
            source
        } else if let Some(source) = playback.sources.get(&sample) {
            source.clone()
        } else {
            let Some(pcm) = sample_pcm(&playback.raw, descriptor, false) else {
                playback.invalid_pcm_events += 1;
                bevy::log::warn_once!(sample, "sfx_backend_invalid_original_pcm");
                continue;
            };
            let wav = pcm16_mono_wav(pcm, descriptor.frequency.max(1));
            let source = audio_sources.add(AudioSource { bytes: wav.into() });
            playback.sources.insert(sample, source.clone());
            source
        };
        let source = if use_loop_region {
            if let Some(side) = stereo_side {
                let Some(source) = stereo::stereo_source(&mut playback, &mut audio_sources, sample, descriptor, true, side) else { continue; };
                source
            } else if let Some(source) = playback.loop_sources.get(&sample) {
                source.clone()
            } else {
                let Some(pcm) = sample_pcm(&playback.raw, descriptor, true) else {
                    playback.invalid_pcm_events += 1;
                    bevy::log::warn_once!(sample, "sfx_backend_invalid_original_loop_pcm");
                    continue;
                };
                let wav = pcm16_mono_wav(pcm, descriptor.frequency.max(1));
                let source = audio_sources.add(AudioSource { bytes: wav.into() });
                playback.loop_sources.insert(sample, source.clone());
                source
            }
        } else {
            full_source.clone()
        };
        let volume = Volume::Linear(channel_gain(sound).clamp(0.0, 1.0));
        let speed = sound.m_nFrequency.max(1) as f32 / descriptor.frequency.max(1) as f32;
        let transform = Transform::from_translation(sound.m_vecPos.to_bevy());
        if sound.m_nLoopCount == 0 {
            let key = (sound.m_nEntityIndex, sound.m_nCounter);
            seen.insert(key);
            let mut spawn_fresh = false;
            if let Some(voice) = playback.looping_voices.get_mut(&key) {
                if voice.sample != sample || voice.stereo_side != stereo_side {
                    commands.entity(voice.entity).despawn();
                    if let Some(intro) = voice.intro_entity.take() {
                        commands.entity(intro).despawn();
                    }
                    spawn_fresh = true;
                } else {
                    // cAudioManager has already applied re3's Doppler formula
                    // and 6000 Hz active-channel slew limit. Do not bend the
                    // pitch a second time in the playback backend.
                    let playback_speed = speed.clamp(0.1, 4.0);
                    let mut intro_finished = false;
                    if voice.intro_entity.is_some() {
                        voice.intro_samples_remaining -=
                            sound.m_nFrequency.max(1) as f32 * delta_seconds.max(0.0);
                        intro_finished = voice.intro_samples_remaining <= 0.0;
                    }
                    if let Ok((sink_2d, sink_3d, mut voice_transform, mut maybe_global)) =
                        sinks.get_mut(voice.entity)
                    {
                        if let Some(mut sink) = sink_2d {
                            sink.set_volume(volume);
                            sink.set_speed(playback_speed);
                            if intro_finished {
                                sink.play();
                            }
                        }
                        if let Some(mut sink) = sink_3d {
                            sink.set_volume(volume);
                            sink.set_speed(playback_speed);
                            if intro_finished {
                                sink.play();
                            }
                        }
                        *voice_transform = transform;
                        if let Some(ref mut global) = maybe_global {
                            **global = GlobalTransform::from(transform);
                        }
                    } else if voice_entities.get(voice.entity).is_err() {
                        spawn_fresh = true;
                    }
                    if let Some(intro) = voice.intro_entity {
                        if let Ok((
                            intro_sink_2d,
                            intro_sink_3d,
                            mut intro_transform,
                            mut intro_global,
                        )) = sinks.get_mut(intro)
                        {
                            if let Some(mut sink) = intro_sink_2d {
                                sink.set_volume(volume);
                                sink.set_speed(playback_speed);
                            }
                            if let Some(mut sink) = intro_sink_3d {
                                sink.set_volume(volume);
                                sink.set_speed(playback_speed);
                            }
                            *intro_transform = transform;
                            if let Some(ref mut global) = intro_global {
                                **global = GlobalTransform::from(transform);
                            }
                        }
                        if intro_finished {
                            if voice_entities.get(intro).is_ok() {
                                commands.entity(intro).despawn();
                            }
                            voice.intro_entity = None;
                        }
                    }
                }
            } else {
                spawn_fresh = true;
            }
            if spawn_fresh {
                if stereo_side.is_some() { playback.stereo_rumble_voice_starts += 1; }
                // re3 does not treat the vehicle audio counters as a mutually
                // exclusive state machine. Reverse gear (61/62), idle (52),
                // rev (2), finger-off (63) and cruising (64) are ordinary
                // requested channels and are allowed to overlap while the
                // generic audio manager releases the old request. The player
                // acceleration sample is the separate dedicated channel and
                // is stopped explicitly through StopVoice(65). Forcibly
                // despawning every other counter here caused audible gaps and
                // restarts on reverse/steering/traction transitions.
                // OpenAL/re3 starts a looping source at sample 0.  The first
                // pass continues through loop_end, then jumps back to
                // loop_start.  Only after that jump do we need the extracted
                // sustain region.  Cutting the intro at loop_start truncates
                // the authored attack/body and creates an audible click.
                let has_intro = use_loop_region && descriptor.loop_start >= 2;
                let intro_entity = if has_intro {
                    Some(
                        commands
                            .spawn((
                                AudioPlayer(full_source),
                                PlaybackSettings::DESPAWN
                                    .with_volume(volume)
                                    .with_speed(speed.clamp(0.1, 4.0))
                                    .with_spatial(!sound.m_bIs2D),
                                transform,
                                GlobalTransform::from(transform),
                                GtaSfxVoice,
                                Name::new("re3 GTA SFX loop attack"),
                                GtaSfxSample { sample_index: sample, audio_entity: sound.m_nEntityIndex, counter: sound.m_nCounter },
                            ))
                            .id(),
                    )
                } else {
                    None
                };
                let playback_settings = if has_intro {
                    PlaybackSettings::LOOP.paused()
                } else {
                    PlaybackSettings::LOOP
                }
                .with_volume(volume)
                .with_speed(speed.clamp(0.1, 4.0))
                .with_spatial(!sound.m_bIs2D);
                let entity = commands
                    .spawn((
                        AudioPlayer(source),
                        playback_settings,
                        transform,
                        GlobalTransform::from(transform),
                        GtaSfxVoice,
                        Name::new("re3 GTA SFX loop"),
                        GtaSfxSample { sample_index: sample, audio_entity: sound.m_nEntityIndex, counter: sound.m_nCounter },
                    ))
                    .id();
                playback.looping_voices.insert(
                    key,
                    LoopingVoice {
                        entity,
                        intro_entity,
                        intro_samples_remaining: if has_intro {
                            descriptor.loop_end.max(0) as f32 * 0.5
                        } else {
                            0.0
                        },
                        sample,
                        stereo_side,
                    },
                );
            }
        } else {
            let key = (sound.m_nEntityIndex, sound.m_nCounter);
            if let Some(entity) = playback.finite_voices.remove(&key)
                && voice_entities.get(entity).is_ok()
            {
                commands.entity(entity).despawn();
            }
            let entity = commands
                .spawn((
                    AudioPlayer(source),
                    PlaybackSettings::DESPAWN
                        .with_volume(volume)
                        .with_speed(speed.clamp(0.1, 4.0))
                        .with_spatial(!sound.m_bIs2D),
                    transform,
                    GlobalTransform::from(transform),
                    GtaSfxVoice,
                    Name::new("re3 GTA SFX one-shot"),
                    GtaSfxSample {
                        sample_index: sample,
                        audio_entity: sound.m_nEntityIndex,
                        counter: sound.m_nCounter,
                    },
                ))
                .id();
            playback.finite_voices.insert(key, entity);
        }
    }
    expired.clear();
    for (key, voice) in playback.looping_voices.iter_mut() {
        if seen.contains(key) {
            continue;
        }
        // re3 ProcessActiveQueues stops an active channel as soon as no
        // matching (entity,counter,sample) request exists in the new queue.
        // Release tails are explicit re-queued samples in the manager, not a
        // backend grace period.
        let _ = voice;
        expired.push(*key);
    }
    for key in expired.iter().copied() {
        if let Some(voice) = playback.looping_voices.remove(&key) {
            commands.entity(voice.entity).despawn();
            if let Some(intro) = voice.intro_entity {
                commands.entity(intro).despawn();
            }
        }
    }
}

#[path = "SfxStereo.rs"]
mod stereo;

fn pcm16_mono_wav(pcm: &[u8], frequency: u32) -> Vec<u8> {
    pcm16_wav(pcm, frequency, 1)
}

fn pcm16_wav(pcm: &[u8], frequency: u32, channels: u16) -> Vec<u8> {
    let block_align = channels * 2;
    let data_len = pcm.len().min(u32::MAX as usize) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36_u32.saturating_add(data_len)).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&frequency.to_le_bytes());
    wav.extend_from_slice(&frequency.saturating_mul(u32::from(block_align)).to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&pcm[..data_len as usize]);
    wav
}

pub(crate) fn sync_mission_audio_playback(
    mut commands: Commands,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut playback: ResMut<GtaMissionAudioPlayback>,
    sinks: Query<(Entity, Option<&AudioSink>), With<GtaMissionAudioEntity>>,
) {
    let Some(dmaudio) = dmaudio.as_deref_mut() else {
        return;
    };
    let status = dmaudio.AudioManager.m_nMissionAudioPlayStatus;
    if status == crate::audio::AudioManager::PLAY_STATUS_PLAYING {
        if playback.entity.is_none() {
            if let Some(source) = playback.source.clone() {
                let volume =
                    Volume::Linear(f32::from(dmaudio.SampleManager.effects_master_volume) / 127.0);
                let entity = commands
                    .spawn((
                        AudioPlayer(source),
                        PlaybackSettings::ONCE.with_volume(volume),
                        GtaMissionAudioEntity,
                        Name::new("re3 Mission Audio Stream 1"),
                    ))
                    .id();
                playback.entity = Some(entity);
                playback.play_frames = 0;
                return;
            } else {
                dmaudio.AudioManager.MarkMissionAudioBackendFinished();
                dmaudio.SampleManager.StopStreamedFile(1);
                playback.entity = None;
                playback.source = None;
                playback.sample = None;
                playback.play_frames = 0;
                return;
            }
        }

        if let Some(entity) = playback.entity
            && let Ok((_, Some(sink))) = sinks.get(entity)
        {
            playback.play_frames = playback.play_frames.saturating_add(1);
            if playback.play_frames > 10 && sink.empty() {
                dmaudio.AudioManager.MarkMissionAudioBackendFinished();
                dmaudio.SampleManager.StopStreamedFile(1);
                commands.entity(entity).despawn();
                playback.entity = None;
                playback.source = None;
                playback.sample = None;
                playback.play_frames = 0;
            } else {
                dmaudio.AudioManager.MarkMissionAudioBackendPlaying();
                let sample = playback.sample.unwrap_or_default();
                let _ = dmaudio.SampleManager.StartStreamedFile(sample, 1);
            }
        }
        return;
    }

    if let Some(entity) = playback.entity.take() {
        if let Ok((_, Some(sink))) = sinks.get(entity) {
            sink.stop();
        }
        commands.entity(entity).despawn();
        dmaudio.SampleManager.StopStreamedFile(1);
    }
    if status == crate::audio::AudioManager::PLAY_STATUS_STOPPED
        && dmaudio.AudioManager.m_nMissionAudioSampleIndex
            >= crate::audio::AudioSamples::TOTAL_AUDIO_SAMPLES as u32
    {
        playback.source = None;
        playback.sample = None;
    }
}

impl cDMAudio {
    pub fn LoadSampleMetadata(&mut self, sdt: &[u8]) {
        self.SampleManager.LoadSampleMetadata(sdt);
        self.AudioManager
            .SetSampleMetadata(self.SampleManager.sample_metadata());
    }

    pub fn Initialise(&mut self) {
        if self.SampleManager.Initialise() {
            self.AudioManager.Initialise();
            let _ = self.MusicManager.Initialise(&[], 0, 0);
        }
    }

    pub fn Terminate(&mut self) {
        self.MusicManager.Terminate(false);
        self.AudioManager.Terminate();
        self.SampleManager.Terminate();
    }

    pub fn Service(&mut self, music_context: &MusicServiceContext) {
        self.AudioManager.Service();
        self.MusicManager.Service(music_context);
    }

    pub fn drain_frame_intents(&mut self) -> GtaAudioFrameIntents {
        GtaAudioFrameIntents {
            samples: std::mem::take(&mut self.AudioManager.emitted_samples),
            voice_updates: std::mem::take(&mut self.AudioManager.voice_updates),
            music_commands: std::mem::take(&mut self.MusicManager.emitted_commands),
            destroyed_entities: std::mem::take(&mut self.AudioManager.destroyed_entities),
            stopped_engine_entities: std::mem::take(&mut self.AudioManager.stopped_engine_entities),
            stopped_voice_keys: std::mem::take(&mut self.AudioManager.stopped_voice_keys),
        }
    }

    pub fn CreateEntity(&mut self, audio_type: eAudioType, uid: Option<i32>) -> i32 {
        self.AudioManager.CreateEntity(audio_type, uid)
    }

    pub fn DestroyEntity(&mut self, audio_entity: i32) {
        self.AudioManager.DestroyEntity(audio_entity);
    }

    pub fn GetEntityStatus(&self, audio_entity: i32) -> bool {
        self.AudioManager.GetEntityStatus(audio_entity)
    }

    pub fn SetEntityStatus(&mut self, audio_entity: i32, status: bool) {
        self.AudioManager.SetEntityStatus(audio_entity, status);
    }

    pub fn PlayOneShot(&mut self, audio_entity: i32, one_shot: u16, volume: f32) {
        self.AudioManager
            .PlayOneShot(audio_entity, one_shot, volume);
    }

    pub fn DestroyAllGameCreatedEntities(&mut self) {
        let ids: Vec<i32> = self.AudioManager.m_aAudioEntityOrderList
            [..self.AudioManager.m_nAudioEntitiesCount]
            .iter()
            .map(|id| *id as i32)
            .collect();
        for id in ids {
            if id >= 0 {
                self.AudioManager.DestroyEntity(id);
            }
        }
    }

    pub fn SetMonoMode(&mut self, mono: bool) {
        self.SampleManager.SetMonoMode(mono);
    }

    pub fn SetEffectsMasterVolume(&mut self, volume: u8) {
        self.SampleManager.SetEffectsMasterVolume(volume);
    }

    pub fn SetMusicMasterVolume(&mut self, volume: u8) {
        self.SampleManager.SetMusicMasterVolume(volume);
    }

    pub fn SetEffectsFadeVol(&mut self, volume: u8) {
        self.SampleManager.SetEffectsFadeVolume(volume);
    }

    pub fn SetMusicFadeVol(&mut self, volume: u8) {
        self.SampleManager.SetMusicFadeVolume(volume);
    }

    pub fn GetNum3DProvidersAvailable(&self) -> u8 {
        1
    }

    pub fn Get3DProviderName(&self, id: u8) -> Option<&'static str> {
        if id == 0 { Some("re3bevy-null") } else { None }
    }

    pub fn GetCurrent3DProviderIndex(&self) -> i8 {
        self.current_3d_provider
    }

    pub fn SetCurrent3DProvider(&mut self, which: u8) -> i8 {
        if which == 0 {
            self.current_3d_provider = 0;
            0
        } else {
            -1
        }
    }

    pub fn SetSpeakerConfig(&mut self, config: i32) {
        self.speaker_config = config;
    }

    pub fn IsMP3RadioChannelAvailable(&self) -> bool {
        true
    }

    pub fn ReleaseDigitalHandle(&mut self) {
        self.digital_handle_released = true;
    }

    pub fn ReacquireDigitalHandle(&mut self) {
        self.digital_handle_released = false;
    }

    pub fn SetDynamicAcousticModelingStatus(&mut self, status: bool) {
        self.dynamic_acoustic_modeling_status = status;
    }

    pub fn CheckForAnAudioFileOnCD(&self) -> bool {
        self.SampleManager.initialised
    }

    pub fn GetCDAudioDriveLetter(&self) -> char {
        '\0'
    }

    pub fn IsAudioInitialised(&self) -> bool {
        self.AudioManager.m_bIsInitialised
    }

    pub fn ResetPoliceRadio(&mut self) {
        self.AudioManager.ResetPoliceRadio();
    }

    pub fn ReportCrime(&mut self, crime: eCrimeType, pos: CVector) {
        self.AudioManager.ReportCrime(crime, pos);
    }

    pub fn CreateLoopingScriptObject(&mut self, script_object: &mut cAudioScriptObject) -> i32 {
        let audio_entity = self
            .AudioManager
            .CreateEntity(AUDIOTYPE_SCRIPTOBJECT, Some(script_object.AudioId as i32));
        if AEHANDLE_IS_OK(audio_entity) {
            script_object.AudioEntity = audio_entity;
            self.AudioManager
                .SetEntityPosition(audio_entity, script_object.Posn);
            self.AudioManager.SetEntityStatus(audio_entity, true);
        }
        audio_entity
    }

    pub fn DestroyLoopingScriptObject(&mut self, audio_entity: i32) {
        self.AudioManager.DestroyEntity(audio_entity);
    }

    pub fn CreateOneShotScriptObject(&mut self, script_object: &mut cAudioScriptObject) {
        let audio_entity = self
            .AudioManager
            .CreateEntity(AUDIOTYPE_SCRIPTOBJECT, Some(script_object.AudioId as i32));
        if AEHANDLE_IS_OK(audio_entity) {
            script_object.AudioEntity = audio_entity;
            self.AudioManager
                .SetEntityPosition(audio_entity, script_object.Posn);
            self.AudioManager.SetEntityStatus(audio_entity, true);
            self.AudioManager
                .PlayOneShot(audio_entity, script_object.AudioId as u16, 0.0);
        }
    }

    pub fn ReportCollision(
        &mut self,
        entity_a: Option<i32>,
        entity_b: Option<i32>,
        surface_type_a: u8,
        surface_type_b: u8,
        collision_power: f32,
        velocity: f32,
        position_a: CVector,
        position_b: CVector,
        entity_a_is_building: bool,
        entity_b_is_building: bool,
        camera_position: CVector,
    ) {
        self.AudioManager.ReportCollision(
            entity_a,
            entity_b,
            surface_type_a,
            surface_type_b,
            collision_power,
            velocity,
            position_a,
            position_b,
            entity_a_is_building,
            entity_b_is_building,
            camera_position,
        );
    }

    pub fn PlayFrontEndSound(&mut self, frontend: u16, volume: u32) {
        self.AudioManager
            .PlayOneShot(self.AudioManager.m_nFrontEndEntity, frontend, volume as f32);
    }

    pub fn PlayRadioAnnouncement(&mut self, announcement: u8) {
        self.MusicManager.PlayAnnouncement(announcement);
    }

    pub fn PlayFrontEndTrack(&mut self, track: u8, frontend_flag: bool) {
        self.MusicManager.PlayFrontEndTrack(track, frontend_flag);
    }

    pub fn StopFrontEndTrack(&mut self) {
        self.MusicManager.StopFrontEndTrack();
    }

    pub fn ResetTimers(&mut self, time: u32) {
        self.AudioManager.m_bTimerJustReset = true;
        self.AudioManager.m_nTimer = time;
        self.MusicManager.ResetTimers(time);
    }

    pub fn ChangeMusicMode(&mut self, mode: u8, ctx: &MusicServiceContext) {
        self.MusicManager.ChangeMusicMode(mode, ctx);
    }

    pub fn PreloadCutSceneMusic(&mut self, track: u8) {
        if self.MusicManager.IsInitialised() && !self.MusicManager.m_bDisabled
            && i32::from(track) < TOTAL_STREAMED_SOUNDS
            && self.MusicManager.m_nMusicMode == MUSICMODE_CUTSCENE as u8
        {
            self.AudioManager.ResetPoliceRadio();
        }
        self.MusicManager
            .PreloadCutSceneMusic(track, self.SampleManager.IsStreamPlaying(0));
    }

    pub fn PlayPreloadedCutSceneMusic(&mut self) {
        self.MusicManager.PlayPreloadedCutSceneMusic();
    }

    pub fn StopCutSceneMusic(&mut self) {
        self.MusicManager.StopCutSceneMusic();
    }

    pub fn PreloadMissionAudio(&mut self, mission_audio_sample: u32) {
        self.AudioManager.PreloadMissionAudio(mission_audio_sample);
    }

    pub fn GetMissionAudioLoadingStatus(&self) -> u8 {
        self.AudioManager.GetMissionAudioLoadingStatus()
    }

    pub fn SetMissionAudioLocation(&mut self, x: f32, y: f32, z: f32) {
        self.AudioManager.SetMissionAudioLocation(x, y, z);
    }

    pub fn PlayLoadedMissionAudio(&mut self) {
        self.AudioManager.PlayLoadedMissionAudio();
    }

    pub fn IsMissionAudioSampleFinished(&self) -> bool {
        self.AudioManager.IsMissionAudioSampleFinished()
    }

    pub fn ClearMissionAudio(&mut self) {
        self.AudioManager.ClearMissionAudio();
    }

    pub fn GetRadioInCar(&self, ctx: &MusicServiceContext) -> u8 {
        self.MusicManager.GetRadioInCar(ctx)
    }

    pub fn SetRadioInCar(&mut self, radio: u32, ctx: &MusicServiceContext) -> Option<u8> {
        self.MusicManager.SetRadioInCar(radio, ctx)
    }

    pub fn SetRadioChannel(&mut self, radio: u8, pos: i32) {
        self.MusicManager.SetRadioChannelByScript(radio, pos);
    }
}

// === FAZ8c-dmaudio: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn apply_script_audio_effects(
    mut commands: Commands,
    mut scripts: ResMut<CTheScripts>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut audio_objects: Option<ResMut<cAudioScriptObjectPool>>,
    install: Option<Res<OriginalGtaInstall>>,
    mut audio_sources: ResMut<Assets<AudioSource>>,
    mut mission_playback: ResMut<GtaMissionAudioPlayback>,
    music_playback: Res<GtaMusicPlayback>,
    timer: Res<CTimer>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::RestoreGameMusicAfterIntro => {
                let Some(dmaudio) = dmaudio.as_deref_mut() else {
                    warn_once!("script_intro_music_waiting_audio_manager");
                    leftover.push(effect);
                    continue;
                };
                let context = script_music_mode_context(dmaudio, &music_playback, &timer);
                dmaudio.ChangeMusicMode(MUSICMODE_GAME as u8, &context);
            }
            ScriptCommandEffect::AddOneOffSound { position, sound_id } => {
                let Some(dmaudio) = dmaudio.as_mut() else {
                    continue;
                };
                if let Some(frontend_sound) = script_sound_frontend_sound(sound_id) {
                    dmaudio.PlayFrontEndSound(frontend_sound as u16, 0);
                } else {
                    let mut object = cAudioScriptObject::new(sound_id as i16, position);
                    dmaudio.CreateOneShotScriptObject(&mut object);
                }
            }
            ScriptCommandEffect::AddContinuousSound {
                handle,
                position,
                sound_id,
            } => {
                let Some(objects) = audio_objects.as_mut() else {
                    leftover.push(ScriptCommandEffect::AddContinuousSound {
                        handle,
                        position,
                        sound_id,
                    });
                    continue;
                };
                let mut object = cAudioScriptObject::new(sound_id as i16, position);
                if let Some(dmaudio) = dmaudio.as_mut() {
                    object.AudioEntity = dmaudio.CreateLoopingScriptObject(&mut object);
                }
                let _ = objects.NewAt(CPoolHandle(handle), object);
            }
            ScriptCommandEffect::RemoveSound { handle } => {
                let Some(objects) = audio_objects.as_mut() else {
                    leftover.push(ScriptCommandEffect::RemoveSound { handle });
                    continue;
                };
                if let Some(object) = objects.Delete(CPoolHandle(handle)) {
                    if let Some(dmaudio) = dmaudio.as_mut() {
                        dmaudio.DestroyLoopingScriptObject(object.AudioEntity);
                    }
                }
            }
            ScriptCommandEffect::LoadMissionAudio { name } => {
                let Some(dmaudio) = dmaudio.as_mut() else {
                    continue;
                };
                if let Some(sample) = mission_audio_sample_from_key(&name) {
                    if let Some(entity) = mission_playback.entity.take() {
                        commands.entity(entity).despawn();
                    }
                    mission_playback.sample = None;
                    mission_playback.source = None;
                    mission_playback.play_frames = 0;
                    let key = script_key_to_string(&name);
                    if let Some(install) = install.as_deref() {
                        let gta_path = format!("AUDIO\\{key}.WAV");
                        match install.read(&gta_path) {
                            Ok(bytes) => {
                                mission_playback.sample = Some(sample);
                                mission_playback.source = Some(audio_sources.add(AudioSource {
                                    bytes: bytes.into(),
                                }));
                            }
                            Err(error) => warn!(
                                "SCM LOAD_MISSION_AUDIO '{}' failed from original GTA install: {error}",
                                key
                            ),
                        }
                    }
                    dmaudio.PreloadMissionAudio(sample);
                    // PC re3 advances NOT_LOADED -> LOADED from AudioManager::Service.
                    scripts.mission_audio_loading_status = dmaudio.GetMissionAudioLoadingStatus();
                    scripts.mission_audio_finished = dmaudio.IsMissionAudioSampleFinished();
                } else {
                    warn!(
                        "SCM LOAD_MISSION_AUDIO unknown '{}'",
                        script_key_to_string(&name)
                    );
                    dmaudio.ClearMissionAudio();
                }
            }
            ScriptCommandEffect::PlayMissionAudio => {
                if let Some(dmaudio) = dmaudio.as_mut() {
                    dmaudio.PlayLoadedMissionAudio();
                    scripts.mission_audio_finished = dmaudio.IsMissionAudioSampleFinished();
                }
            }
            ScriptCommandEffect::SetMissionAudioPosition { position } => {
                if let Some(dmaudio) = dmaudio.as_mut() {
                    dmaudio.SetMissionAudioLocation(position.x, position.y, position.z);
                }
            }
            ScriptCommandEffect::ClearMissionAudio => {
                mission_playback.play_frames = 0;
                if let Some(dmaudio) = dmaudio.as_mut() {
                    dmaudio.ClearMissionAudio();
                    scripts.mission_audio_loading_status = dmaudio.GetMissionAudioLoadingStatus();
                    scripts.mission_audio_finished = dmaudio.IsMissionAudioSampleFinished();
                }
            }
            ScriptCommandEffect::PlayMissionPassedTune { tune_id } => {
                if let Some(dmaudio) = dmaudio.as_mut() {
                    let ctx = script_music_mode_context(dmaudio, &music_playback, &timer);
                    dmaudio.ChangeMusicMode(MUSICMODE_FRONTEND as u8, &ctx);
                    let track = (tune_id + STREAMED_SOUND_MISSION_COMPLETED as i32 - 1)
                        .clamp(0, u8::MAX as i32) as u8;
                    dmaudio.PlayFrontEndTrack(track, false);
                }
            }
            effect @ (ScriptCommandEffect::LoadEndOfGameTune
                | ScriptCommandEffect::PlayEndOfGameTune | ScriptCommandEffect::StopEndOfGameTune) => {
                let Some(dmaudio) = dmaudio.as_deref_mut() else {
                    warn_once!("script_end_tune_waiting_audio_manager");
                    leftover.push(effect);
                    continue;
                };
                let context = script_music_mode_context(dmaudio, &music_playback, &timer);
                match effect {
                    ScriptCommandEffect::LoadEndOfGameTune => {
                        dmaudio.ChangeMusicMode(MUSICMODE_CUTSCENE as u8, &context);
                        dmaudio.PreloadCutSceneMusic(STREAMED_SOUND_GAME_COMPLETED as u8);
                    }
                    ScriptCommandEffect::PlayEndOfGameTune => dmaudio.PlayPreloadedCutSceneMusic(),
                    ScriptCommandEffect::StopEndOfGameTune => {
                        dmaudio.StopCutSceneMusic();
                        dmaudio.ChangeMusicMode(MUSICMODE_GAME as u8, &context);
                    }
                    _ => unreachable!(),
                }
            }
            ScriptCommandEffect::PlayRadioAnnouncement { announcement } => {
                if let Some(dmaudio) = dmaudio.as_mut() {
                    dmaudio.PlayRadioAnnouncement(announcement.clamp(0, u8::MAX as i32) as u8);
                }
            }
            ScriptCommandEffect::SetRadioChannel { channel, position } => {
                if let Some(dmaudio) = dmaudio.as_mut() {
                    dmaudio.SetRadioChannel(channel.clamp(0, u8::MAX as i32) as u8, position);
                }
            }
            other => leftover.push(other),
        }
    }
    if let Some(dmaudio) = dmaudio.as_ref() {
        scripts.mission_audio_loading_status = dmaudio.GetMissionAudioLoadingStatus();
        scripts.mission_audio_finished = dmaudio.IsMissionAudioSampleFinished();
    }
    scripts.pending_effects = leftover;
}

/// ChangeMusicMode borrows these three native stream/timer inputs; player radio
/// inputs are read separately by the normal Service stage.
fn script_music_mode_context(dmaudio: &cDMAudio, playback: &GtaMusicPlayback, timer: &CTimer) -> MusicServiceContext {
    MusicServiceContext {
        stream_playing: dmaudio.SampleManager.IsStreamPlaying(0),
        streamed_file_position: playback.current_position_ms,
        time_ms_pause_mode: timer.time_in_milliseconds_pause_mode(),
        ..Default::default()
    }
}

// FAZ8c-pedtalk: moved to audio/PedTalk.rs verbatim (1 items): fn service_ped_talk

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_end_restores_music_across_streaming_pause_and_missing_audio() {
        use crate::control::ScriptWorldEffects::apply_script_effects_to_world;
        use crate::core::{FileLoader::CFileLoader, Game::ScriptGameplayBridge, Streaming::CStreaming};
        use re3bevy_core::core::GameState::CGameState;
        use re3bevy_formats::formats::Script::ScmFile;

        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        scripts.pending_effects.push(ScriptCommandEffect::SetIntroIsPlaying { playing: false });
        let mut bridge = ScriptGameplayBridge::default();
        bridge.playing_intro = true;
        let mut game = CGameState::default();
        game.playing_intro = true;
        let mut loader = CFileLoader::default();
        loader.model_info.add_simple_model(1000).base.set_model_name("bridgefukb");
        let mut streaming = CStreaming::default();
        apply_script_effects_to_world(
            &mut scripts, &mut bridge, Some(&mut game), None, None,
            Some(&mut streaming), None, None, None, None, None, None,
            Some(&mut loader), 0,
        );
        assert!(!bridge.playing_intro && !game.playing_intro);
        assert!(scripts.pending_effects.iter().any(|effect|
            matches!(effect, ScriptCommandEffect::RestoreGameMusicAfterIntro)));

        let mut app = App::new();
        app.insert_resource(scripts)
            .init_resource::<Assets<AudioSource>>()
            .init_resource::<GtaMissionAudioPlayback>()
            .init_resource::<GtaMusicPlayback>()
            .init_resource::<CTimer>()
            .add_systems(Update, apply_script_audio_effects);
        app.world_mut().resource_mut::<CTimer>().set_code_pause(true);
        app.update();
        assert!(app.world().resource::<CTheScripts>().pending_effects.iter().any(|effect|
            matches!(effect, ScriptCommandEffect::RestoreGameMusicAfterIntro)));

        let mut audio = cDMAudio::default();
        audio.Initialise();
        audio.ChangeMusicMode(MUSICMODE_CUTSCENE as u8, &MusicServiceContext::default());
        app.insert_resource(audio);
        app.update();
        assert_eq!(app.world().resource::<cDMAudio>().MusicManager.GetMusicMode(), MUSICMODE_GAME as u32);
        assert!(app.world().resource::<CTheScripts>().pending_effects.is_empty());
        assert!(app.world().resource::<CTimer>().get_is_code_paused());
    }

    #[test]
    fn streamed_track_paths_match_re3_pc_table() {
        assert_eq!(streamed_track_path(0).as_deref(), Some("AUDIO\\HEAD.WAV"));
        assert_eq!(streamed_track_path(17).as_deref(), Some("AUDIO\\L1_LG.MP3"));
        assert_eq!(
            streamed_track_path(45).as_deref(),
            Some("AUDIO\\A1_SSO.WAV")
        );
        assert_eq!(
            streamed_track_path(70).as_deref(),
            Some("AUDIO\\A6_BAIT.WAV")
        );
        assert_eq!(
            streamed_track_path(93).as_deref(),
            Some("AUDIO\\MISCOM.WAV")
        );
        assert_eq!(streamed_track_path(94).as_deref(), Some("AUDIO\\END.MP3"));
        assert_eq!(streamed_track_path(195), None);
    }

    #[test]
    fn preloaded_cutscene_command_starts_real_stream_zero_backend() {
        let mut app = App::new();
        app.init_resource::<Assets<AudioSource>>();
        app.init_resource::<GtaAudioFrameIntents>();
        app.init_resource::<GtaMusicPlayback>();
        app.add_systems(Update, sync_music_playback);

        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        app.insert_resource(dmaudio);
        let source = {
            let mut assets = app.world_mut().resource_mut::<Assets<AudioSource>>();
            assets.add(AudioSource {
                bytes: vec![0_u8; 16].into(),
            })
        };
        {
            let mut playback = app.world_mut().resource_mut::<GtaMusicPlayback>();
            playback.preloaded_track = Some(17);
            playback.preloaded_source = Some(source);
        }
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .music_commands = vec![
            MusicCommand::SetStreamVolumeAndPan {
                volume: 127,
                pan: 63,
                front_end: true,
            },
            MusicCommand::StartPreloadedStreamedFile,
        ];

        app.update();

        let playback = app.world().resource::<GtaMusicPlayback>();
        assert_eq!(playback.active_track, Some(17));
        assert!(playback.entity.is_some());
        assert!(
            app.world()
                .resource::<cDMAudio>()
                .SampleManager
                .IsStreamPlaying(0)
        );
        let entity = playback.entity.unwrap();
        assert!(app.world().get::<GtaMusicAudioEntity>(entity).is_some());
    }

    #[test]
    fn audio_listener_spawns_once_and_follows_camera() {
        let mut app = App::new();
        app.add_systems(Update, sync_audio_listener);
        app.update();
        app.update();
        let mut spawned = app
            .world_mut()
            .query::<(&GtaAudioListener, &SpatialListener, &Transform)>();
        assert_eq!(spawned.iter(app.world()).count(), 1);

        let mut loader = CFileLoader::default();
        loader.renderer.camera_position = CVector::new(10.0, 20.0, 30.0);
        app.insert_resource(loader);
        app.update();
        let mut positioned = app.world_mut().query::<(&GtaAudioListener, &Transform)>();
        let positions: Vec<Vec3> = positioned
            .iter(app.world())
            .map(|(_, t)| t.translation)
            .collect();
        assert_eq!(positions, vec![CVector::new(10.0, 20.0, 30.0).to_bevy()]);
    }

    #[test]
    fn audio_listener_copies_active_camera_rotation() {
        let mut app = App::new();
        app.add_systems(Update, sync_audio_listener);
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let mut camera = Camera::default();
        camera.is_active = true;
        app.world_mut().spawn((
            camera,
            Transform {
                translation: Vec3::new(1.0, 2.0, 3.0),
                rotation,
                scale: Vec3::ONE,
            },
        ));
        app.update();
        app.update();
        let mut listeners = app.world_mut().query::<(&GtaAudioListener, &Transform)>();
        let rotations: Vec<Quat> = listeners
            .iter(app.world())
            .map(|(_, t)| t.rotation)
            .collect();
        assert_eq!(rotations.len(), 1);
        assert!(
            rotations[0].angle_between(rotation) < 0.001,
            "listener should face where the camera faces for stereo panning"
        );
    }

    fn loop_test_app(frames_to_play: f32) -> App {
        let mut app = App::new();
        app.init_resource::<cDMAudio>();
        app.init_resource::<Assets<AudioSource>>();
        app.add_systems(Update, sync_sfx_playback);
        let raw = vec![0u8; 4410];
        app.insert_resource(GtaSfxPlayback {
            descriptors: vec![SampleMetadata {
                offset: 0,
                size: raw.len() as u32,
                frequency: 22050,
                ..Default::default()
            }],
            raw,
            loaded: true,
            ..Default::default()
        });
        app.insert_resource(GtaAudioFrameIntents {
            samples: vec![tSound {
                m_nEntityIndex: 5,
                m_nCounter: 2,
                m_nSampleIndex: 0,
                m_nFrequency: 22050,
                m_nVolume: 100,
                m_nLoopCount: 0,
                m_nFramesToPlay: frames_to_play,
                ..Default::default()
            }],
            music_commands: Vec::new(),
            ..Default::default()
        });
        app
    }

    fn voice_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<GtaSfxVoice>>()
            .iter(app.world())
            .count()
    }

    #[derive(Default, Resource)]
    struct ChannelObservation(Option<bool>);
    fn observe_player_channel(channel: PlayerEngineChannel, mut observation: ResMut<ChannelObservation>) {
        observation.0 = channel.is_used(5);
    }

    #[test]
    fn player_channel_reads_live_entity_before_backend_cleanup_and_never_guesses_duration() {
        let mut app = loop_test_app(8.0);
        app.init_resource::<ChannelObservation>().add_systems(PreUpdate, observe_player_channel);
        {
            let mut intents = app.world_mut().resource_mut::<GtaAudioFrameIntents>();
            intents.samples[0].m_nCounter = 65;
            intents.samples[0].m_nLoopCount = 1;
        }
        app.update();
        assert_eq!(app.world().resource::<ChannelObservation>().0, Some(false));
        let voice = app.world().resource::<GtaSfxPlayback>().finite_voices[&(5, 65)];
        app.world_mut().resource_mut::<GtaAudioFrameIntents>().samples.clear();
        for _ in 0..100 {
            app.update();
            assert_eq!(app.world().resource::<ChannelObservation>().0, Some(true));
            assert_eq!(app.world().resource::<GtaSfxPlayback>().finite_voices[&(5, 65)], voice);
        }
        app.world_mut().despawn(voice);
        app.update();
        assert_eq!(app.world().resource::<ChannelObservation>().0, Some(false));
        assert!(!app.world().resource::<GtaSfxPlayback>().finite_voices.contains_key(&(5, 65)));
        app.world_mut().resource_mut::<GtaAudioFrameIntents>().samples.push(tSound {
            m_nEntityIndex: 5, m_nCounter: 65, m_nSampleIndex: 0, m_nFrequency: 22050,
            m_nVolume: 100, m_nLoopCount: 1, ..Default::default()
        });
        app.update();
        let replacement = app.world().resource::<GtaSfxPlayback>().finite_voices[&(5, 65)];
        assert_ne!(replacement, voice);
        app.world_mut().resource_mut::<GtaAudioFrameIntents>().samples.clear();
        app.update();
        assert_eq!(app.world().resource::<ChannelObservation>().0, Some(true));
    }

    #[test]
    fn stereo_rumble_preserves_source_routing_cache_live_identity_and_retirement() {
        let mut app = loop_test_app(8.0);
        let sample = super::super::AudioSamples::SFX_JUMBO_RUMBLE as u32;
        {
            let mut playback = app.world_mut().resource_mut::<GtaSfxPlayback>();
            playback.raw = [100_i16, -200, 300, -400, 500, -600, 700, -800]
                .into_iter().flat_map(i16::to_le_bytes).collect();
            let size = playback.raw.len() as u32;
            playback.descriptors.resize(sample as usize + 1, SampleMetadata::default());
            playback.descriptors[sample as usize] = SampleMetadata {
                offset: 0, size, frequency: 22050, loop_start: 4, loop_end: 12,
                ..Default::default()
            };
        }
        {
            let mut intents = app.world_mut().resource_mut::<GtaAudioFrameIntents>();
            let mut left = intents.samples[0];
            left.m_nSampleIndex = sample; left.m_bIs2D = true;
            left.m_nCounter = 5; left.m_nPan = 0;
            let right = tSound { m_nCounter: 6, m_nPan: 127, m_nFrequency: 22250, ..left };
            intents.samples = vec![left, right];
        }
        app.update();
        let left = app.world().resource::<GtaSfxPlayback>().looping_voices[&(5, 5)].entity;
        let right = app.world().resource::<GtaSfxPlayback>().looping_voices[&(5, 6)].entity;
        assert_ne!(left, right);
        for (entity, side) in [(left, stereo::StereoSide::Left), (right, stereo::StereoSide::Right)] {
            let source = &app.world().get::<AudioPlayer>(entity).unwrap().0;
            let bytes = &app.world().resource::<Assets<AudioSource>>().get(source).unwrap().bytes;
            assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 2);
            let original = [300_i16, -400, 500, -600];
            for (frame, expected) in bytes[44..].chunks_exact(4).zip(original) {
                let left_pcm = i16::from_le_bytes(frame[..2].try_into().unwrap());
                let right_pcm = i16::from_le_bytes(frame[2..].try_into().unwrap());
                match side {
                    stereo::StereoSide::Left => { assert_eq!(left_pcm, expected); assert_eq!(right_pcm, 0); }
                    stereo::StereoSide::Right => { assert_eq!(left_pcm, 0); assert_eq!(right_pcm, expected); }
                }
            }
            assert!(!app.world().get::<PlaybackSettings>(entity).unwrap().spatial);
        }
        assert_eq!(app.world().resource::<GtaSfxPlayback>().stereo_sources.len(), 4);
        for _ in 0..20 { app.update(); }
        let playback = app.world().resource::<GtaSfxPlayback>();
        assert_eq!(playback.looping_voices[&(5, 5)].entity, left);
        assert_eq!(playback.looping_voices[&(5, 6)].entity, right);
        assert_eq!(playback.stereo_rumble_voice_starts, 2);
        assert_eq!(playback.unsupported_jumbo_pan_events, 0);
        app.world_mut().resource_mut::<GtaAudioFrameIntents>().destroyed_entities.push(5);
        app.world_mut().resource_mut::<GtaAudioFrameIntents>().samples.clear();
        app.update();
        assert_eq!(voice_count(&mut app), 0);
        assert!(app.world().resource::<GtaSfxPlayback>().looping_voices.is_empty());
        assert_eq!(app.world().resource::<GtaSfxPlayback>().stereo_sources.len(), 4);
    }

    #[test]
    fn jumbo_invalid_pan_is_counted_without_playing_a_centered_mono_substitute() {
        let mut app = loop_test_app(8.0);
        {
            let mut intents = app.world_mut().resource_mut::<GtaAudioFrameIntents>();
            intents.samples[0].m_nSampleIndex = super::super::AudioSamples::SFX_JUMBO_RUMBLE as u32;
            intents.samples[0].m_bIs2D = true;
            intents.samples[0].m_nPan = 63;
        }
        app.update();
        assert_eq!(app.world().resource::<GtaSfxPlayback>().unsupported_jumbo_pan_events, 1);
        assert_eq!(voice_count(&mut app), 0);
    }

    #[test]
    fn invalid_required_backend_sample_is_counted_without_spawning_a_voice() {
        let mut app = loop_test_app(8.0);
        app.world_mut().resource_mut::<GtaSfxPlayback>().raw.clear();
        app.update();
        let playback = app.world().resource::<GtaSfxPlayback>();
        assert_eq!(playback.invalid_pcm_events, 1);
        assert!(playback.finite_voices.is_empty() && playback.looping_voices.is_empty());
    }

    #[test]
    fn finished_backend_one_shot_retires_manager_channel_and_release_buffers() {
        let mut app = loop_test_app(8.0);
        let mut audio = cDMAudio::default();
        audio.Initialise();
        let entity = audio.AudioManager.m_nCollisionEntity;
        let sound = tSound {
            m_nEntityIndex: entity,
            m_nCounter: 28,
            m_nSampleIndex: 0,
            m_nFrequency: 22_050,
            m_nVolume: 40,
            m_nLoopCount: 1,
            m_nFramesToPlay: 100.0,
            ..Default::default()
        };
        audio.AudioManager.m_sQueueSample = sound;
        audio.AudioManager.AddSampleToRequestedQueue();
        audio.AudioManager.ProcessActiveQueues();
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .samples = audio.AudioManager.emitted_samples.clone();
        app.insert_resource(audio);
        app.update();
        let backend = app.world().resource::<GtaSfxPlayback>().finite_voices[&(entity, 28)];
        app.world_mut().despawn(backend);
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .samples
            .clear();
        app.update();
        let mut audio = app.world_mut().resource_mut::<cDMAudio>();
        audio.AudioManager.ProcessActiveQueues();
        assert!(audio.AudioManager.emitted_samples.is_empty());
        assert!(
            audio
                .AudioManager
                .m_asActiveSamples
                .iter()
                .all(|s| s.m_nSampleIndex >= crate::audio::AudioSamples::TOTAL_AUDIO_SAMPLES as u32)
        );
    }

    #[test]
    fn sfx_loop_stops_when_request_disappears() {
        let mut app = loop_test_app(8.0);
        app.update();
        assert_eq!(voice_count(&mut app), 1);
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .samples
            .clear();
        app.update();
        assert_eq!(voice_count(&mut app), 0);
    }

    #[test]
    fn sfx_loop_respawns_after_voice_despawned() {
        let mut app = loop_test_app(8.0);
        app.update();
        assert_eq!(voice_count(&mut app), 1);
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<GtaSfxVoice>>()
            .iter(app.world())
            .next()
            .unwrap();
        app.world_mut().despawn(entity);
        app.update();
        assert_eq!(voice_count(&mut app), 1);
        let live = app
            .world()
            .resource::<GtaSfxPlayback>()
            .looping_voices
            .get(&(5, 2))
            .unwrap()
            .entity;
        assert!(
            app.world().get_entity(live).is_ok(),
            "loop map must point at a live voice, not a dead one"
        );
    }

    #[test]
    fn sfx_authored_loop_plays_attack_before_sustain_region() {
        let mut app = App::new();
        app.init_resource::<cDMAudio>();
        app.init_resource::<Assets<AudioSource>>();
        app.add_systems(Update, sync_sfx_playback);
        let raw = vec![0u8; 2_000];
        app.insert_resource(GtaSfxPlayback {
            descriptors: vec![SampleMetadata {
                offset: 0,
                size: raw.len() as u32,
                frequency: 22_050,
                loop_start: 200,
                loop_end: 600,
            }],
            raw,
            loaded: true,
            ..Default::default()
        });
        app.insert_resource(GtaAudioFrameIntents {
            samples: vec![tSound {
                m_nEntityIndex: 5,
                m_nCounter: 52,
                m_nSampleIndex: 0,
                m_nFrequency: 22_050,
                m_nVolume: 100,
                m_nEmittingVolume: 100,
                m_MaxDistance: 100.0,
                m_nLoopCount: 0,
                ..Default::default()
            }],
            ..Default::default()
        });

        app.update();
        assert_eq!(
            voice_count(&mut app),
            2,
            "attack and paused sustain should coexist initially"
        );
        assert!(
            app.world()
                .resource::<GtaSfxPlayback>()
                .looping_voices
                .get(&(5, 52))
                .is_some_and(|voice| {
                    voice.intro_entity.is_some()
                        && (voice.intro_samples_remaining - 300.0).abs() < f32::EPSILON
                })
        );

        app.update();
        assert_eq!(
            voice_count(&mut app),
            1,
            "attack should hand off to the authored sustain loop"
        );
        assert!(
            app.world()
                .resource::<GtaSfxPlayback>()
                .looping_voices
                .get(&(5, 52))
                .is_some_and(|voice| voice.intro_entity.is_none())
        );
    }

    #[test]
    fn looping_source_uses_sdt_sustain_window() {
        let raw: Vec<u8> = (0..16).collect();
        let descriptor = SampleMetadata {
            offset: 2,
            size: 10,
            frequency: 22_050,
            loop_start: 2,
            loop_end: 6,
        };
        assert_eq!(sample_pcm(&raw, descriptor, false), Some(&raw[2..12]));
        // OpenAL SOFT loop-end is the sample offset where playback jumps back.
        assert_eq!(sample_pcm(&raw, descriptor, true), Some(&raw[4..8]));
    }

    #[test]
    fn re3_3d_gain_uses_emitting_volume_and_inverse_distance() {
        let mut sound = tSound {
            m_nVolume: 12,
            m_nEmittingVolume: 100,
            m_bIs2D: false,
            m_MaxDistance: 50.0,
            m_fDistance: 10.0,
            ..Default::default()
        };
        let full = 100.0 / 127.0;
        assert!((re3_sfx_gain(&sound) - full).abs() < 0.0001);

        sound.m_fDistance = 25.0;
        assert!((re3_sfx_gain(&sound) - full * 0.5).abs() < 0.0001);

        sound.m_fDistance = 49.0;
        assert!((re3_sfx_gain(&sound) - full * (12.5 / 49.0)).abs() < 0.0001);
    }

    #[test]
    fn re3_2d_gain_keeps_software_volume() {
        let sound = tSound {
            m_nVolume: 40,
            m_nEmittingVolume: 120,
            m_bIs2D: true,
            m_MaxDistance: 50.0,
            m_fDistance: 25.0,
            ..Default::default()
        };
        assert!((re3_sfx_gain(&sound) - 40.0 / 127.0).abs() < 0.0001);
    }

    #[test]
    fn native_channel_gain_quantizes_master_fade_and_cutscene_ducking() {
        let sound = tSound { m_bIs2D: true, m_nVolume: 100, ..Default::default() };
        let settings = SfxGainSettings { fade: 127, master: 127, duck: false };
        assert!((settings.gain(&sound) - 98.0 / 127.0).abs() < 0.00001);
        let ducked = SfxGainSettings { duck: true, ..settings };
        assert!((ducked.gain(&sound) - 24.0 / 127.0).abs() < 0.00001);
        assert_eq!(SfxGainSettings { fade: 0, master: 127, duck: false }.gain(&sound), 0.0);
    }

    #[test]
    fn re3_3d_zero_emitting_volume_is_silent() {
        let sound = tSound {
            m_nVolume: 100,
            m_nEmittingVolume: 0,
            m_bIs2D: false,
            m_MaxDistance: 60.0,
            m_fDistance: 10.0,
            ..Default::default()
        };
        assert_eq!(re3_sfx_gain(&sound), 0.0);
    }

    #[test]
    fn sfx_loop_follows_emitter_position_across_frames() {
        let mut app = loop_test_app(8.0);
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .samples[0]
            .m_vecPos = CVector::new(10.0, 20.0, 30.0);
        app.update();

        let live = app
            .world()
            .resource::<GtaSfxPlayback>()
            .looping_voices
            .get(&(5, 2))
            .unwrap()
            .entity;
        let t1 = app.world().get::<Transform>(live).unwrap().translation;
        assert_eq!(t1, CVector::new(10.0, 20.0, 30.0).to_bevy());
        let gt1 = app
            .world()
            .get::<GlobalTransform>(live)
            .unwrap()
            .translation();
        assert_eq!(gt1, CVector::new(10.0, 20.0, 30.0).to_bevy());

        // Car moves forward on next frame:
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .samples = vec![tSound {
            m_nEntityIndex: 5,
            m_nCounter: 2,
            m_nSampleIndex: 0,
            m_nFrequency: 22050,
            m_nVolume: 100,
            m_nLoopCount: 0,
            m_nFramesToPlay: 8.0,
            m_vecPos: CVector::new(50.0, 60.0, 70.0),
            ..Default::default()
        }];
        app.update();

        let t2 = app.world().get::<Transform>(live).unwrap().translation;
        assert_eq!(
            t2,
            CVector::new(50.0, 60.0, 70.0).to_bevy(),
            "looping voice Transform must update to vehicle's new position each frame"
        );
        let gt2 = app
            .world()
            .get::<GlobalTransform>(live)
            .unwrap()
            .translation();
        assert_eq!(
            gt2,
            CVector::new(50.0, 60.0, 70.0).to_bevy(),
            "looping voice GlobalTransform must update to vehicle's new position each frame"
        );
    }

    #[test]
    fn vehicle_audio_counters_can_overlap_like_re3_requested_channels() {
        let mut app = loop_test_app(8.0);
        // Vehicle 5 begins at idle (counter 52):
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .samples[0]
            .m_nCounter = 52;
        app.update();
        assert_eq!(voice_count(&mut app), 1);
        assert!(
            app.world()
                .resource::<GtaSfxPlayback>()
                .looping_voices
                .contains_key(&(5, 52))
        );

        // A second vehicle layer starts. In re3 a different counter is a
        // separate requested channel; both requested channels exist in
        // GtaAudioFrameIntents during transition.
        app.world_mut()
            .resource_mut::<GtaAudioFrameIntents>()
            .samples = vec![
            tSound {
                m_nEntityIndex: 5,
                m_nCounter: 52,
                m_nSampleIndex: 0,
                m_nFrequency: 22050,
                m_nVolume: 100,
                m_nLoopCount: 0,
                m_nFramesToPlay: 8.0,
                ..Default::default()
            },
            tSound {
                m_nEntityIndex: 5,
                m_nCounter: 61,
                m_nSampleIndex: 0,
                m_nFrequency: 22050,
                m_nVolume: 100,
                m_nLoopCount: 0,
                m_nFramesToPlay: 8.0,
                ..Default::default()
            },
        ];
        app.update();

        assert_eq!(voice_count(&mut app), 2);
        assert!(
            app.world()
                .resource::<GtaSfxPlayback>()
                .looping_voices
                .contains_key(&(5, 52)),
            "idle layer must not be force-killed by another vehicle counter"
        );
        assert!(
            app.world()
                .resource::<GtaSfxPlayback>()
                .looping_voices
                .contains_key(&(5, 61)),
            "reverse-gear layer must coexist with the engine layer"
        );
    }
}
