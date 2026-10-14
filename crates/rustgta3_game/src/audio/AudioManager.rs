//! re3 reference: references/re-GTA/src/audio/AudioManager.cpp
//! re3 reference: references/re-GTA/src/audio/AudioManager.h

use rustgta3_core::core::Coordinates::CVector;
use std::sync::atomic::{AtomicU32, Ordering};

static MISSION_AUDIO_PRETEND_FRAME: AtomicU32 = AtomicU32::new(1);

use crate::core::SurfaceTable::eSurfaceType;

use super::AudioCollision::*;
use super::AudioSamples::*;
use super::AudioScriptObject::NUM_SCRIPT_MAX_ENTITIES;
use super::PolRadio::*;
use super::audio_enums::*;
use super::sampman::SampleMetadata;
use super::soundlist::*;

pub const SPEED_OF_SOUND: f32 = 343.0;
pub const TIME_SPENT: u8 = 50;
pub const MAX_VOLUME: u8 = 127;
pub const NUM_AUDIOENTITY_EVENTS: usize = 4;
pub const NUM_PED_COMMENTS_SLOTS: usize = 20;
pub const NUM_SOUND_QUEUES: usize = 2;
pub const NUM_AUDIOENTITIES: usize = 200;
pub const NUM_CHANNELS_GENERIC: usize = 27;
pub const SFX_BANK_0: u8 = 0;
pub const SFX_BANK_PED_COMMENTS: u8 = 1;

pub const PLAY_STATUS_STOPPED: u8 = 0;
pub const PLAY_STATUS_PLAYING: u8 = 1;
pub const PLAY_STATUS_FINISHED: u8 = 2;
pub const LOADING_STATUS_NOT_LOADED: u8 = 0;
pub const LOADING_STATUS_LOADED: u8 = 1;
pub const LOADING_STATUS_LOADING: u8 = 2;

#[inline]
fn clamp_towards_u32(target: u32, current: u32, radius: u32) -> u32 {
    if target > current {
        target.min(current.saturating_add(radius))
    } else {
        target.max(current.saturating_sub(radius))
    }
}

#[inline]
fn clamp_towards_u8(target: u8, current: u8, radius: u8) -> u8 {
    clamp_towards_u32(target.into(), current.into(), radius.into()) as u8
}

const ONE_SHOT_PRIORITY: &[u8] = &[
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 5, 5, 5, 3, 5, 2, 2, 1, 1, 3, 1, 3, 3, 1, 1, 1, 4, 4, 3,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 1, 1, 3, 2, 2, 2, 2, 0, 0, 6, 6, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0,
    0, 3, 1, 1, 1, 9, 2, 2, 0, 0, 0, 0, 3, 3, 5, 1, 1, 1, 1, 3, 4, 7, 6, 6, 6, 6, 1, 3, 4, 3, 4, 2,
    1, 3, 5, 4, 6, 6, 1, 3, 1, 1, 1, 0, 0, 0, 0, 0, 0, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct tSound {
    pub m_nEntityIndex: i32,
    pub m_nCounter: u32,
    pub m_nSampleIndex: u32,
    pub m_nBankIndex: u8,
    pub m_bIs2D: bool,
    pub m_nPriority: u32,
    pub m_nFrequency: u32,
    pub m_nVolume: u8,
    /// re3/OpenAL EXTERNAL_3D_SOUND source gain before distance attenuation.
    /// m_nVolume remains the software-computed audible volume used for
    /// priority/cutoff; the backend must use this field for live 3D gain.
    pub m_nEmittingVolume: u8,
    pub m_fDistance: f32,
    pub m_nLoopCount: u32,
    pub m_nLoopStart: u32,
    pub m_nLoopEnd: i32,
    pub m_fSpeedMultiplier: f32,
    pub m_MaxDistance: f32,
    pub m_bStatic: bool,
    pub m_vecPos: CVector,
    pub m_bReverb: bool,
    pub m_nPan: u8,
    pub m_nFramesToPlay: f32,
    pub m_bIsBeingPlayed: bool,
    pub m_bIsPlayingFinished: bool,
    pub m_nFinalPriority: u32,
    pub m_nVolumeChange: i8,
    pub m_nEmittingVolumeChange: i8,
}

impl Default for tSound {
    fn default() -> Self {
        Self {
            m_nEntityIndex: AEHANDLE_NONE,
            m_nCounter: 0,
            m_nSampleIndex: TOTAL_AUDIO_SAMPLES as u32,
            m_nBankIndex: SFX_BANK_0,
            m_bIs2D: false,
            m_nPriority: 0,
            m_nFrequency: 0,
            m_nVolume: 0,
            m_nEmittingVolume: 0,
            m_fDistance: 0.0,
            m_nLoopCount: 1,
            m_nLoopStart: 0,
            m_nLoopEnd: -1,
            m_fSpeedMultiplier: 0.0,
            m_MaxDistance: 0.0,
            m_bStatic: false,
            m_vecPos: CVector::ZERO,
            m_bReverb: false,
            m_nPan: 63,
            m_nFramesToPlay: 0.0,
            m_bIsBeingPlayed: false,
            m_bIsPlayingFinished: false,
            m_nFinalPriority: 0,
            m_nVolumeChange: 0,
            m_nEmittingVolumeChange: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct tAudioEntity {
    /// Allocation identity for Bevy actor bindings; survives numeric slot reuse.
    pub allocation_generation: u64,
    pub m_nType: eAudioType,
    pub m_pEntity: Option<i32>,
    pub m_bIsUsed: bool,
    pub m_bStatus: bool,
    pub m_awAudioEvent: [i16; NUM_AUDIOENTITY_EVENTS],
    pub m_afVolume: [f32; NUM_AUDIOENTITY_EVENTS],
    pub m_AudioEvents: u8,
    pub m_vecPos: CVector,
}

impl Default for tAudioEntity {
    fn default() -> Self {
        Self {
            allocation_generation: 0,
            m_nType: AUDIOTYPE_PHYSICAL,
            m_pEntity: None,
            m_bIsUsed: false,
            m_bStatus: false,
            m_awAudioEvent: [SOUND_NO_SOUND as i16; NUM_AUDIOENTITY_EVENTS],
            m_afVolume: [0.0; NUM_AUDIOENTITY_EVENTS],
            m_AudioEvents: 0,
            m_vecPos: CVector::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct tPedComment {
    pub m_nSampleIndex: u32,
    pub m_nEntityIndex: i32,
    pub m_vecPos: CVector,
    pub m_fDistance: f32,
    pub m_nVolume: u8,
    pub m_nLoadingTimeout: i8,
    pub m_nEmittingVolume: u8,
}

impl Default for tPedComment {
    fn default() -> Self {
        Self {
            m_nSampleIndex: TOTAL_AUDIO_SAMPLES as u32,
            m_nEntityIndex: AEHANDLE_NONE,
            m_vecPos: CVector::ZERO,
            m_fDistance: 0.0,
            m_nVolume: 0,
            m_nLoadingTimeout: -1,
            m_nEmittingVolume: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct cPedComments {
    pub m_aPedCommentQueue: [[tPedComment; NUM_PED_COMMENTS_SLOTS]; NUM_SOUND_QUEUES],
    pub m_aPedCommentOrderList: [[usize; NUM_PED_COMMENTS_SLOTS]; NUM_SOUND_QUEUES],
    pub m_nPedCommentCount: [usize; NUM_SOUND_QUEUES],
    pub m_nActiveQueue: usize,
}

impl Default for cPedComments {
    fn default() -> Self {
        Self {
            m_aPedCommentQueue: [[tPedComment::default(); NUM_PED_COMMENTS_SLOTS];
                NUM_SOUND_QUEUES],
            m_aPedCommentOrderList: [[NUM_PED_COMMENTS_SLOTS; NUM_PED_COMMENTS_SLOTS];
                NUM_SOUND_QUEUES],
            m_nPedCommentCount: [0; NUM_SOUND_QUEUES],
            m_nActiveQueue: 0,
        }
    }
}

impl cPedComments {
    pub fn Add(&mut self, comment: tPedComment) {
        let queue = self.m_nActiveQueue;
        let index = if self.m_nPedCommentCount[queue] >= NUM_PED_COMMENTS_SLOTS {
            let index = self.m_aPedCommentOrderList[queue][NUM_PED_COMMENTS_SLOTS - 1];
            if self.m_aPedCommentQueue[queue][index].m_nVolume > comment.m_nVolume {
                return;
            }
            index
        } else {
            let index = self.m_nPedCommentCount[queue];
            self.m_nPedCommentCount[queue] += 1;
            index
        };

        self.m_aPedCommentQueue[queue][index] = comment;
        let mut i = 0usize;
        if index != 0 {
            while i < index {
                let ordered = self.m_aPedCommentOrderList[queue][i];
                if ordered >= NUM_PED_COMMENTS_SLOTS
                    || self.m_aPedCommentQueue[queue][ordered].m_nVolume
                        < self.m_aPedCommentQueue[queue][index].m_nVolume
                {
                    break;
                }
                i += 1;
            }
            if i < index {
                for j in (i + 1..NUM_PED_COMMENTS_SLOTS).rev() {
                    self.m_aPedCommentOrderList[queue][j] =
                        self.m_aPedCommentOrderList[queue][j - 1];
                }
            }
        }
        self.m_aPedCommentOrderList[queue][i] = index;
    }

    pub fn clear_active_queue(&mut self) {
        let queue = self.m_nActiveQueue;
        self.m_aPedCommentOrderList[queue] = [NUM_PED_COMMENTS_SLOTS; NUM_PED_COMMENTS_SLOTS];
        self.m_nPedCommentCount[queue] = 0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct cAudioScriptObjectManager {
    pub m_anScriptObjectEntityIndices: [i32; NUM_SCRIPT_MAX_ENTITIES],
    pub m_nScriptObjectEntityTotal: usize,
}

impl Default for cAudioScriptObjectManager {
    fn default() -> Self {
        Self {
            m_anScriptObjectEntityIndices: [AEHANDLE_NONE; NUM_SCRIPT_MAX_ENTITIES],
            m_nScriptObjectEntityTotal: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct cAudioManager {
    pub m_bIsInitialised: bool,
    pub m_bIsSurround: bool,
    pub m_bReduceReleasingPriority: bool,
    pub m_nActiveSamples: usize,
    pub m_bDoubleVolume: bool,
    pub m_fSpeedOfSound: f32,
    pub m_bTimerJustReset: bool,
    pub m_nTimer: u32,
    pub m_sQueueSample: tSound,
    pub m_nActiveQueue: usize,
    pub m_aRequestedQueue: [[tSound; NUM_CHANNELS_GENERIC]; NUM_SOUND_QUEUES],
    pub m_aRequestedOrderList: [[usize; NUM_CHANNELS_GENERIC]; NUM_SOUND_QUEUES],
    pub m_nRequestedCount: [usize; NUM_SOUND_QUEUES],
    pub m_asActiveSamples: [tSound; NUM_CHANNELS_GENERIC],
    pub m_asAudioEntities: [tAudioEntity; NUM_AUDIOENTITIES],
    pub m_aAudioEntityOrderList: [usize; NUM_AUDIOENTITIES],
    pub m_nAudioEntitiesCount: usize,
    pub m_sAudioScriptObjectManager: cAudioScriptObjectManager,
    pub m_sPedComments: cPedComments,
    pub m_nFireAudioEntity: i32,
    pub m_nWaterCannonEntity: i32,
    pub m_nPoliceChannelEntity: i32,
    pub m_sPoliceRadioQueue: cPoliceRadioQueue,
    pub m_aCrimes: [cAMCrime; 10],
    pub m_nFrontEndEntity: i32,
    pub m_nCollisionEntity: i32,
    pub m_sCollisionManager: cAudioCollisionManager,
    pub m_nProjectileEntity: i32,
    pub m_nBridgeEntity: i32,
    pub m_nWeatherAudioEntity: i32,
    pub m_vecMissionAudioPosition: CVector,
    pub m_bIsMissionAudio2D: bool,
    pub m_nMissionAudioSampleIndex: u32,
    pub m_nMissionAudioLoadingStatus: u8,
    pub m_nMissionAudioPlayStatus: u8,
    pub m_bIsMissionAudioPlaying: bool,
    pub m_nMissionAudioFramesToPlay: i32,
    pub m_bIsMissionAudioAllowedToPlay: bool,
    pub m_anRandomTable: [i32; 5],
    random_displacement_positive: bool,
    random_displacement_adjustment: u32,
    collision_one_shot_counter: u32,
    pub m_nTimeSpent: u8,
    pub m_FrameCounter: u32,
    pub m_bIsPaused: bool,
    pub m_bWasPaused: bool,
    pub mission_police_audio_sfx: u32,
    pub mission_police_audio_status: i8,
    pub min_time_to_next_report: [u32; NUM_CRIME_TYPES],
    pub emitted_samples: Vec<tSound>,
    pub voice_updates: Vec<tSound>,
    dedicated_voice_starts: Vec<tSound>,
    pub destroyed_entities: Vec<i32>,
    pub stopped_engine_entities: Vec<i32>,
    pub stopped_voice_keys: Vec<(i32, u32)>,
    pub m_nPickupNextFrame: u32,
    pub m_nPartMisComNextFrame: u32,
    pub m_nFrontEndSoundCounter: u8,
    /// cAudioManager::ProcessPedOneShots keeps iSound across ped/frame calls.
    pub(crate) ped_one_shot_state: super::AudioLogic::PedOneShotAudioState,
    sample_metadata: Vec<SampleMetadata>,
    /// CTimer::GetTimeStepFix for the current audio service; defaults to one
    /// native 30 Hz frame for standalone manager calls.
    pub(crate) release_time_step: f32,
    next_entity_generation: u64,
}

impl Default for cAudioManager {
    fn default() -> Self {
        let mut manager = Self {
            m_bIsInitialised: false,
            m_bIsSurround: true,
            m_bReduceReleasingPriority: false,
            m_nActiveSamples: NUM_CHANNELS_GENERIC,
            m_bDoubleVolume: false,
            m_fSpeedOfSound: SPEED_OF_SOUND / TIME_SPENT as f32,
            m_bTimerJustReset: false,
            m_nTimer: 0,
            m_sQueueSample: tSound::default(),
            m_nActiveQueue: 0,
            m_aRequestedQueue: [[tSound::default(); NUM_CHANNELS_GENERIC]; NUM_SOUND_QUEUES],
            m_aRequestedOrderList: [[NUM_CHANNELS_GENERIC; NUM_CHANNELS_GENERIC]; NUM_SOUND_QUEUES],
            m_nRequestedCount: [0; NUM_SOUND_QUEUES],
            m_asActiveSamples: [tSound::default(); NUM_CHANNELS_GENERIC],
            m_asAudioEntities: [tAudioEntity::default(); NUM_AUDIOENTITIES],
            m_aAudioEntityOrderList: [NUM_AUDIOENTITIES; NUM_AUDIOENTITIES],
            m_nAudioEntitiesCount: 0,
            m_sAudioScriptObjectManager: cAudioScriptObjectManager::default(),
            m_sPedComments: cPedComments::default(),
            m_nFireAudioEntity: AEHANDLE_NONE,
            m_nWaterCannonEntity: AEHANDLE_NONE,
            m_nPoliceChannelEntity: AEHANDLE_NONE,
            m_sPoliceRadioQueue: cPoliceRadioQueue::default(),
            m_aCrimes: [cAMCrime::default(); 10],
            m_nFrontEndEntity: AEHANDLE_NONE,
            m_nCollisionEntity: AEHANDLE_NONE,
            m_sCollisionManager: cAudioCollisionManager::default(),
            m_nProjectileEntity: AEHANDLE_NONE,
            m_nBridgeEntity: AEHANDLE_NONE,
            m_nWeatherAudioEntity: AEHANDLE_NONE,
            m_vecMissionAudioPosition: CVector::ZERO,
            m_bIsMissionAudio2D: true,
            m_nMissionAudioSampleIndex: TOTAL_AUDIO_SAMPLES as u32,
            m_nMissionAudioLoadingStatus: LOADING_STATUS_NOT_LOADED,
            m_nMissionAudioPlayStatus: PLAY_STATUS_STOPPED,
            m_bIsMissionAudioPlaying: false,
            m_nMissionAudioFramesToPlay: 0,
            m_bIsMissionAudioAllowedToPlay: false,
            m_anRandomTable: [0; 5],
            random_displacement_positive: true,
            random_displacement_adjustment: 0,
            collision_one_shot_counter: 28,
            m_nTimeSpent: TIME_SPENT,
            m_FrameCounter: 0,
            m_bIsPaused: false,
            m_bWasPaused: false,
            mission_police_audio_sfx: TOTAL_AUDIO_SAMPLES as u32,
            mission_police_audio_status: PLAY_STATUS_FINISHED as i8,
            min_time_to_next_report: [0; NUM_CRIME_TYPES],
            emitted_samples: Vec::new(),
            voice_updates: Vec::new(),
            dedicated_voice_starts: Vec::new(),
            destroyed_entities: Vec::new(),
            stopped_engine_entities: Vec::new(),
            stopped_voice_keys: Vec::new(),
            m_nPickupNextFrame: 0,
            m_nPartMisComNextFrame: 0,
            m_nFrontEndSoundCounter: 0,
            ped_one_shot_state: super::AudioLogic::PedOneShotAudioState::new(),
            sample_metadata: Vec::new(),
            release_time_step: 1.0,
            next_entity_generation: 0,
        };
        manager.ClearRequestedQueue();
        manager.m_nActiveQueue = 1;
        manager.ClearRequestedQueue();
        manager.m_nActiveQueue = 0;
        manager.ClearActiveSamples();
        manager.GenerateIntegerRandomNumberTable();
        manager
    }
}

impl cAudioManager {
    /// re3 updates these two flags at the start of every audio service.  On the
    /// first user-pause frame it also kills every generic channel and clears
    /// both requested queues so a stale loop cannot survive behind the menu.
    pub fn SyncUserPauseState(&mut self, paused: bool) -> bool {
        self.m_bWasPaused = self.m_bIsPaused;
        self.m_bIsPaused = paused;
        if !self.m_bIsPaused || self.m_bWasPaused {
            return false;
        }

        let active_queue = self.m_nActiveQueue;
        self.ClearRequestedQueue();
        self.m_nActiveQueue = 1 - active_queue;
        self.ClearRequestedQueue();
        self.m_nActiveQueue = active_queue;
        self.ClearActiveSamples();
        self.emitted_samples.clear();
        self.voice_updates.clear();
        self.dedicated_voice_starts.clear();
        true
    }

    pub fn Initialise(&mut self) {
        if !self.m_bIsInitialised {
            self.m_bIsInitialised = true;
            self.m_nPickupNextFrame = 0;
            self.m_nPartMisComNextFrame = 0;
            self.m_nFrontEndSoundCounter = 0;
            self.PostInitialiseGameSpecificSetup();
            self.InitialisePoliceRadio();
        }
    }

    pub fn Terminate(&mut self) {
        if self.m_bIsInitialised {
            for i in 0..NUM_AUDIOENTITIES {
                self.m_asAudioEntities[i] = tAudioEntity::default();
                self.m_aAudioEntityOrderList[i] = NUM_AUDIOENTITIES;
            }
            self.m_nAudioEntitiesCount = 0;
            self.m_sAudioScriptObjectManager.m_nScriptObjectEntityTotal = 0;
            self.destroyed_entities.clear();
            self.stopped_engine_entities.clear();
            self.stopped_voice_keys.clear();
            self.voice_updates.clear();
            self.dedicated_voice_starts.clear();
            self.m_nPickupNextFrame = 0;
            self.m_nPartMisComNextFrame = 0;
            self.m_nFrontEndSoundCounter = 0;
            self.m_bIsInitialised = false;
        }
    }

    pub fn Service(&mut self) {
        self.service_with_ped_comments(None);
    }

    pub(crate) fn service_with_ped_comments(&mut self, comments: Option<super::PedCommentPlayback::PedCommentService<'_>>) {
        if self.m_bIsInitialised {
            self.GenerateIntegerRandomNumberTable();
            self.m_bReduceReleasingPriority = self.m_FrameCounter % 5 == 0;
            if self.m_bTimerJustReset {
                self.m_bTimerJustReset = false;
            }
            if let Some(comments) = comments {
                super::PedCommentPlayback::process_ped_comments(self, comments);
            }
            self.ProcessMissionAudio();
            self.ServiceCollisions();
            self.ProcessActiveQueues();
            self.m_FrameCounter = self.m_FrameCounter.wrapping_add(1);
        }
    }

    pub fn PostInitialiseGameSpecificSetup(&mut self) {
        self.m_nFireAudioEntity = self.create_builtin_entity(AUDIOTYPE_FIRE);
        self.m_nCollisionEntity = self.create_builtin_entity(AUDIOTYPE_COLLISION);
        self.m_nFrontEndEntity = self.create_builtin_entity(AUDIOTYPE_FRONTEND);
        self.m_nProjectileEntity = self.create_builtin_entity(AUDIOTYPE_PROJECTILE);
        self.m_nWaterCannonEntity = self.create_builtin_entity(AUDIOTYPE_WATERCANNON);
        self.m_nPoliceChannelEntity = self.create_builtin_entity(AUDIOTYPE_POLICERADIO);
        self.m_nBridgeEntity = self.create_builtin_entity(AUDIOTYPE_BRIDGE);
        self.m_nWeatherAudioEntity = self.create_builtin_entity(AUDIOTYPE_WEATHER);
    }

    fn create_builtin_entity(&mut self, audio_type: eAudioType) -> i32 {
        let handle = self.CreateEntity(audio_type, Some(1));
        if handle >= 0 {
            self.SetEntityStatus(handle, true);
        }
        handle
    }

    pub fn CreateEntity(&mut self, audio_type: eAudioType, entity: Option<i32>) -> i32 {
        if !self.m_bIsInitialised {
            return AEHANDLE_ERROR_NOAUDIOSYS;
        }
        if entity.is_none() {
            return AEHANDLE_ERROR_NOENTITY;
        }
        if audio_type >= TOTAL_AUDIO_TYPES {
            return AEHANDLE_ERROR_BADAUDIOTYPE;
        }
        for i in 0..NUM_AUDIOENTITIES {
            if !self.m_asAudioEntities[i].m_bIsUsed {
                self.next_entity_generation = self.next_entity_generation.wrapping_add(1).max(1);
                self.m_asAudioEntities[i].allocation_generation = self.next_entity_generation;
                self.m_asAudioEntities[i].m_bIsUsed = true;
                self.m_asAudioEntities[i].m_bStatus = false;
                self.m_asAudioEntities[i].m_nType = audio_type;
                self.m_asAudioEntities[i].m_pEntity = entity;
                self.m_asAudioEntities[i].m_awAudioEvent =
                    [SOUND_NO_SOUND as i16; NUM_AUDIOENTITY_EVENTS];
                self.m_asAudioEntities[i].m_afVolume = [0.0; NUM_AUDIOENTITY_EVENTS];
                self.m_asAudioEntities[i].m_AudioEvents = 0;
                self.m_asAudioEntities[i].m_vecPos = CVector::ZERO;
                self.m_aAudioEntityOrderList[self.m_nAudioEntitiesCount] = i;
                self.m_nAudioEntitiesCount += 1;
                return i as i32;
            }
        }
        AEHANDLE_ERROR_NOFREESLOT
    }

    pub fn DestroyEntity(&mut self, id: i32) {
        if self.entity_index(id).is_none() {
            return;
        }
        let id_usize = id as usize;
        self.m_asAudioEntities[id_usize].m_bIsUsed = false;
        self.m_asAudioEntities[id_usize].m_bStatus = false;
        self.m_asAudioEntities[id_usize].m_AudioEvents = 0;
        self.m_asAudioEntities[id_usize].m_vecPos = CVector::ZERO;
        self.dedicated_voice_starts.retain(|sound| sound.m_nEntityIndex != id);
        self.voice_updates.retain(|sound| sound.m_nEntityIndex != id);
        self.destroyed_entities.push(id);
        if let Some(pos) = self.m_aAudioEntityOrderList[..self.m_nAudioEntitiesCount]
            .iter()
            .position(|&entry| entry == id_usize)
        {
            for i in pos..self.m_nAudioEntitiesCount - 1 {
                self.m_aAudioEntityOrderList[i] = self.m_aAudioEntityOrderList[i + 1];
            }
            self.m_nAudioEntitiesCount -= 1;
            self.m_aAudioEntityOrderList[self.m_nAudioEntitiesCount] = NUM_AUDIOENTITIES;
        }
    }

    pub fn SetEntityPosition(&mut self, id: i32, pos: CVector) {
        if let Some(index) = self.entity_index(id) {
            self.m_asAudioEntities[index].m_vecPos = pos;
        }
    }

    pub fn GetEntityPosition(&self, id: i32) -> Option<CVector> {
        self.entity_index(id)
            .map(|index| self.m_asAudioEntities[index].m_vecPos)
    }

    pub fn StopEngineLoop(&mut self, id: i32) {
        if id >= 0 {
            for counter in [2, 52, 61, 62, 63, 64, 65] {
                self.clear_voice_state(id, counter);
            }
            self.stopped_engine_entities.push(id);
        }
    }

    /// Stop one re3 channel identity immediately. Used for the dedicated
    /// player-vehicle acceleration channel, whose finite sample is outside the
    /// generic requested-queue lifetime in the original game.
    pub fn StopVoice(&mut self, id: i32, counter: u32) {
        if id < 0 {
            return;
        }
        self.clear_voice_state(id, counter);
        self.stopped_voice_keys.push((id, counter));
    }

    /// Bevy's finished finite voice is re3's GetChannelUsedFlag == false.
    /// Retire both request buffers so AddReleasingSounds cannot revive it.
    pub(crate) fn finish_voice(&mut self, id: i32, counter: u32) {
        self.clear_voice_state(id, counter);
    }

    fn clear_voice_state(&mut self, id: i32, counter: u32) {
        self.dedicated_voice_starts
            .retain(|sound| sound.m_nEntityIndex != id || sound.m_nCounter != counter);
        for active in &mut self.m_asActiveSamples[..self.m_nActiveSamples] {
            if active.m_nEntityIndex == id && active.m_nCounter == counter {
                *active = tSound::default();
            }
        }
        for queue in 0..NUM_SOUND_QUEUES {
            for order in 0..self.m_nRequestedCount[queue] {
                let index = self.m_aRequestedOrderList[queue][order];
                if index >= NUM_CHANNELS_GENERIC {
                    continue;
                }
                let sample = &mut self.m_aRequestedQueue[queue][index];
                if sample.m_nEntityIndex == id && sample.m_nCounter == counter {
                    sample.m_bIsPlayingFinished = true;
                    sample.m_nFramesToPlay = 0.0;
                }
            }
        }
    }

    /// Native dedicated channels bypass generic priority and release lifetimes.
    pub(crate) fn StartDedicatedVoice(&mut self, sound: tSound) {
        if !self.m_bIsInitialised || self.m_bIsPaused {
            return;
        }
        if !self.audio_entity_is_used(sound.m_nEntityIndex) {
            bevy::log::warn_once!(audio_entity = sound.m_nEntityIndex, "dedicated_voice_invalid_audio_entity");
            return;
        }
        self.dedicated_voice_starts.retain(|pending| {
            pending.m_nEntityIndex != sound.m_nEntityIndex || pending.m_nCounter != sound.m_nCounter
        });
        self.dedicated_voice_starts.push(sound);
    }

    pub fn UpdateVoice(&mut self, sound: tSound) {
        self.voice_updates.push(sound);
    }

    pub fn GetEntityStatus(&self, id: i32) -> bool {
        self.entity_index(id)
            .map(|index| self.m_asAudioEntities[index].m_bStatus)
            .unwrap_or(false)
    }

    pub fn SetEntityStatus(&mut self, id: i32, status: bool) {
        if let Some(index) = self.entity_index(id) {
            self.m_asAudioEntities[index].m_bStatus = status;
        }
    }

    pub fn PlayOneShot(&mut self, index: i32, sound: u16, vol: f32) {
        if !self.m_bIsInitialised || sound as i32 >= SOUND_TOTAL_SOUNDS {
            return;
        }
        let Some(entity_index) = self.entity_index(index) else {
            return;
        };
        let entity = &mut self.m_asAudioEntities[entity_index];
        if entity.m_nType == AUDIOTYPE_SCRIPTOBJECT {
            if self.m_sAudioScriptObjectManager.m_nScriptObjectEntityTotal < NUM_SCRIPT_MAX_ENTITIES
            {
                entity.m_awAudioEvent[0] = sound as i16;
                entity.m_AudioEvents = 1;
                let total = self.m_sAudioScriptObjectManager.m_nScriptObjectEntityTotal;
                self.m_sAudioScriptObjectManager
                    .m_anScriptObjectEntityIndices[total] = index;
                self.m_sAudioScriptObjectManager.m_nScriptObjectEntityTotal += 1;
            }
            return;
        }

        let new_priority = one_shot_priority(sound as usize);
        let mut i = 0usize;
        while i < entity.m_AudioEvents as usize {
            let old = entity.m_awAudioEvent[i].max(0) as usize;
            if one_shot_priority(old) > new_priority {
                break;
            }
            i += 1;
        }
        if i >= entity.m_AudioEvents as usize {
            if (entity.m_AudioEvents as usize) < NUM_AUDIOENTITY_EVENTS {
                entity.m_awAudioEvent[i] = sound as i16;
                entity.m_afVolume[i] = vol;
                entity.m_AudioEvents += 1;
            }
            return;
        }
        if i < NUM_AUDIOENTITY_EVENTS - 1 {
            for j in (i + 1..NUM_AUDIOENTITY_EVENTS).rev() {
                entity.m_awAudioEvent[j] = entity.m_awAudioEvent[j - 1];
                entity.m_afVolume[j] = entity.m_afVolume[j - 1];
            }
        }
        entity.m_awAudioEvent[i] = sound as i16;
        entity.m_afVolume[i] = vol;
        if (entity.m_AudioEvents as usize) < NUM_AUDIOENTITY_EVENTS {
            entity.m_AudioEvents += 1;
        }
    }

    pub fn ClearRequestedQueue(&mut self) {
        self.m_aRequestedOrderList[self.m_nActiveQueue] =
            [NUM_CHANNELS_GENERIC; NUM_CHANNELS_GENERIC];
        self.m_nRequestedCount[self.m_nActiveQueue] = 0;
    }

    pub fn ClearActiveSamples(&mut self) {
        self.m_asActiveSamples = [tSound::default(); NUM_CHANNELS_GENERIC];
    }

    pub fn ComputeVolume(&self, emitting_volume: u8, max_distance: f32, distance: f32) -> u8 {
        if max_distance <= 0.0 || distance >= max_distance {
            return 0;
        }
        let min_distance = max_distance / 5.0;
        if distance > min_distance {
            let scale = (max_distance - distance) / (max_distance - min_distance);
            (scale * scale * emitting_volume as f32).clamp(0.0, MAX_VOLUME as f32) as u8
        } else {
            emitting_volume
        }
    }

    /// re3 `ComputeEmittingVolume` for the PC external-3D-sound path.
    /// OpenAL handles the inverse-distance curve, while re3 additionally
    /// fades the source gain to zero over the last quarter of MaxDistance.
    pub fn ComputeEmittingVolume(
        &self,
        emitting_volume: u8,
        max_distance: f32,
        distance: f32,
    ) -> u8 {
        if max_distance <= 0.0 || distance >= max_distance {
            return 0;
        }
        let min_distance = max_distance * 0.25;
        let fade_start = max_distance - min_distance;
        if distance > fade_start {
            let remaining = max_distance - distance;
            return (remaining * f32::from(emitting_volume) / min_distance)
                .clamp(0.0, f32::from(emitting_volume)) as u8;
        }
        emitting_volume
    }

    pub fn AdjustSamplesVolume(&mut self) {
        let queue = self.m_nActiveQueue;
        for order in 0..self.m_nRequestedCount[queue] {
            let index = self.m_aRequestedOrderList[queue][order];
            if index >= NUM_CHANNELS_GENERIC {
                continue;
            }
            let sample = self.m_aRequestedQueue[queue][index];
            if sample.m_bIs2D {
                continue;
            }
            let adjusted = self.ComputeEmittingVolume(
                sample.m_nEmittingVolume,
                sample.m_MaxDistance,
                sample.m_fDistance,
            );
            self.m_aRequestedQueue[queue][index].m_nEmittingVolume = adjusted;
        }
    }

    /// re3 `ComputeDopplerEffectedFrequency`: apply radial source velocity to
    /// the requested frequency before the active-channel slew limiter.  The
    /// Bevy backend must not apply a second Doppler pass on top of this value.
    pub fn ComputeDopplerEffectedFrequency(
        &self,
        old_freq: u32,
        position1: f32,
        position2: f32,
        speed_multiplier: f32,
    ) -> u32 {
        if speed_multiplier == 0.0 || self.m_nTimeSpent == 0 {
            return old_freq;
        }
        let dist = position2 - position1;
        if dist == 0.0 {
            return old_freq;
        }
        let mut speed_of_source = (dist / f32::from(self.m_nTimeSpent)) * speed_multiplier;
        if self.m_fSpeedOfSound <= speed_of_source.abs() {
            return old_freq;
        }
        speed_of_source = speed_of_source.clamp(-1.5, 1.5);
        ((old_freq as f32 * self.m_fSpeedOfSound) / (speed_of_source + self.m_fSpeedOfSound)) as u32
    }

    pub fn AddSampleToRequestedQueue(&mut self) {
        if self.m_sQueueSample.m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32 {
            return;
        }
        let final_priority =
            self.m_sQueueSample.m_nPriority * (MAX_VOLUME - self.m_sQueueSample.m_nVolume) as u32;
        let queue = self.m_nActiveQueue;
        let sample_index = if self.m_nRequestedCount[queue] >= self.m_nActiveSamples {
            let index = self.m_aRequestedOrderList[queue][self.m_nActiveSamples - 1];
            if self.m_aRequestedQueue[queue][index].m_nFinalPriority <= final_priority {
                return;
            }
            index
        } else {
            let index = self.m_nRequestedCount[queue];
            self.m_nRequestedCount[queue] += 1;
            index
        };

        self.m_sQueueSample.m_nFinalPriority = final_priority;
        self.m_sQueueSample.m_bIsPlayingFinished = false;
        self.m_aRequestedQueue[queue][sample_index] = self.m_sQueueSample;
        self.AddDetailsToRequestedOrderList(sample_index);
    }

    pub fn AddDetailsToRequestedOrderList(&mut self, sample: usize) {
        let queue = self.m_nActiveQueue;
        let mut i = 0usize;
        if sample != 0 {
            while i < sample {
                let ordered = self.m_aRequestedOrderList[queue][i];
                if ordered >= NUM_CHANNELS_GENERIC
                    || self.m_aRequestedQueue[queue][ordered].m_nFinalPriority
                        > self.m_aRequestedQueue[queue][sample].m_nFinalPriority
                {
                    break;
                }
                i += 1;
            }
            if i < sample {
                for j in (i + 1..NUM_CHANNELS_GENERIC).rev() {
                    self.m_aRequestedOrderList[queue][j] = self.m_aRequestedOrderList[queue][j - 1];
                }
            }
        }
        self.m_aRequestedOrderList[queue][i] = sample;
    }

    fn audio_entity_is_used(&self, id: i32) -> bool {
        id >= 0
            && (id as usize) < NUM_AUDIOENTITIES
            && self.m_asAudioEntities[id as usize].m_bIsUsed
    }

    pub fn sample_length(&self, sample: u32) -> u32 {
        self.sample_metadata
            .get(sample as usize)
            .map(|sample| sample.size / 2)
            .unwrap_or(0)
    }

    /// re3 `AddReleasingSounds`: a sound that disappeared from the newly
    /// built request queue can keep its existing channel only by being
    /// explicitly re-queued here. Infinite loops fade over their authored
    /// `m_nFramesToPlay`; finite one-shots retain the channel bookkeeping long
    /// enough for the already-started sample to finish without being started
    /// again by the backend.
    fn AddReleasingSounds(&mut self) {
        let current = self.m_nActiveQueue;
        let previous = 1 - current;
        let mut releasing = Vec::new();

        for order in 0..self.m_nRequestedCount[previous] {
            let index = self.m_aRequestedOrderList[previous][order];
            if index >= NUM_CHANNELS_GENERIC {
                continue;
            }
            let mut sample = self.m_aRequestedQueue[previous][index];
            if sample.m_bIsPlayingFinished || !self.audio_entity_is_used(sample.m_nEntityIndex) {
                continue;
            }

            let replaced = (0..self.m_nRequestedCount[current]).any(|current_order| {
                let current_index = self.m_aRequestedOrderList[current][current_order];
                current_index < NUM_CHANNELS_GENERIC
                    && self.m_aRequestedQueue[current][current_index].m_nEntityIndex
                        == sample.m_nEntityIndex
                    && self.m_aRequestedQueue[current][current_index].m_nCounter
                        == sample.m_nCounter
            });
            if replaced || sample.m_nFramesToPlay <= 0.0 {
                continue;
            }

            if sample.m_nLoopCount == 0 {
                if sample.m_nVolumeChange <= 0 {
                    sample.m_nVolumeChange = (f32::from(sample.m_nVolume)
                        / sample.m_nFramesToPlay.max(1.0))
                    .clamp(1.0, i8::MAX as f32) as i8;
                }
                let change = f32::from(sample.m_nVolumeChange.max(1)) * self.release_time_step;
                if f32::from(sample.m_nVolume) <= change {
                    continue;
                }
                sample.m_nVolume = (f32::from(sample.m_nVolume) - change) as u8;

                // OpenAL/re3 fades the pre-distance source gain separately
                // from the software-computed priority/cutoff volume.
                if !sample.m_bIs2D && sample.m_nEmittingVolume > 0 {
                    if sample.m_nEmittingVolumeChange <= 0 {
                        sample.m_nEmittingVolumeChange =
                            (f32::from(sample.m_nEmittingVolume) / sample.m_nFramesToPlay.max(1.0))
                                .clamp(1.0, i8::MAX as f32) as i8;
                    }
                    let emitting_change = f32::from(sample.m_nEmittingVolumeChange.max(1)) * self.release_time_step;
                    if f32::from(sample.m_nEmittingVolume) <= emitting_change {
                        continue;
                    }
                    sample.m_nEmittingVolume = (f32::from(sample.m_nEmittingVolume) - emitting_change) as u8;
                }
            }

            sample.m_nFramesToPlay = (sample.m_nFramesToPlay - self.release_time_step).max(0.0);
            if self.m_bReduceReleasingPriority && sample.m_nPriority < 20 {
                sample.m_nPriority += 1;
            }
            sample.m_bStatic = false;
            releasing.push(sample);
        }

        for sample in releasing {
            self.m_sQueueSample = sample;
            self.AddSampleToRequestedQueue();
        }
    }

    pub fn ProcessActiveQueues(&mut self) {
        self.AddReleasingSounds();
        self.AdjustSamplesVolume();
        let queue = self.m_nActiveQueue;
        self.emitted_samples.clear();

        for active in &mut self.m_asActiveSamples[..self.m_nActiveSamples] {
            active.m_bIsBeingPlayed = false;
        }
        for order in 0..self.m_nRequestedCount[queue] {
            let index = self.m_aRequestedOrderList[queue][order];
            if index < NUM_CHANNELS_GENERIC {
                self.m_aRequestedQueue[queue][index].m_bIsBeingPlayed = false;
            }
        }

        // Keep already-running channels. Looping channels are emitted every
        // frame because the Bevy sink needs position/volume/pitch updates;
        // finite samples stay on their existing backend voice and are not
        // spawned again merely because release bookkeeping re-queued them.
        for order in 0..self.m_nRequestedCount[queue] {
            let index = self.m_aRequestedOrderList[queue][order];
            if index >= NUM_CHANNELS_GENERIC {
                continue;
            }
            let mut sample = self.m_aRequestedQueue[queue][index];
            if sample.m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32 {
                continue;
            }
            if let Some(active_index) = self.m_asActiveSamples[..self.m_nActiveSamples]
                .iter()
                .position(|active| {
                    active.m_nSampleIndex == sample.m_nSampleIndex
                        && active.m_nEntityIndex == sample.m_nEntityIndex
                        && active.m_nCounter == sample.m_nCounter
                })
            {
                let previous = self.m_asActiveSamples[active_index];
                sample.m_bIsBeingPlayed = true;
                sample.m_nVolumeChange = -1;
                sample.m_nEmittingVolumeChange = -1;

                // re3 does not snap a live 3D channel straight to a newly
                // requested pitch/volume.  It first applies Doppler, then
                // limits frequency movement to 6000 Hz and volume movement to
                // 10 units per audio service.  Without this, vehicle loops can
                // click/crackle and pump between very loud/quiet values.
                let mut active = sample;
                if sample.m_bStatic {
                    active = previous;
                    active.m_bIsBeingPlayed = true;
                } else if !sample.m_bIs2D {
                    sample.m_nFrequency = self.ComputeDopplerEffectedFrequency(
                        sample.m_nFrequency,
                        previous.m_fDistance,
                        sample.m_fDistance,
                        sample.m_fSpeedMultiplier,
                    );
                    active = sample;
                    active.m_nFrequency =
                        clamp_towards_u32(sample.m_nFrequency, previous.m_nFrequency, 6000);
                    // With EXTERNAL_3D_SOUND re3 slews emitting volume, not
                    // the already-distance-attenuated m_nVolume.
                    if sample.m_nEmittingVolume > 0 || previous.m_nEmittingVolume > 0 {
                        active.m_nEmittingVolume = clamp_towards_u8(
                            sample.m_nEmittingVolume,
                            previous.m_nEmittingVolume,
                            10,
                        );
                    }
                }
                self.m_asActiveSamples[active_index] = active;
                self.m_aRequestedQueue[queue][index] = sample;
                if sample.m_nLoopCount == 0 {
                    self.emitted_samples.push(active);
                }
            }
        }

        // An active channel with no matching request is stopped immediately;
        // any intended fade is present above as an explicit release request.
        for active in &mut self.m_asActiveSamples[..self.m_nActiveSamples] {
            if active.m_nSampleIndex < TOTAL_AUDIO_SAMPLES as u32 && !active.m_bIsBeingPlayed {
                *active = tSound::default();
            }
        }

        // Requests are already priority-sorted by AddSampleToRequestedQueue.
        // Fill free channels in that order, matching re3's 27-channel budget.
        for order in 0..self.m_nRequestedCount[queue] {
            let index = self.m_aRequestedOrderList[queue][order];
            if index >= NUM_CHANNELS_GENERIC {
                continue;
            }
            let mut sample = self.m_aRequestedQueue[queue][index];
            if sample.m_bIsBeingPlayed
                || sample.m_bIsPlayingFinished
                || sample.m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32
                || !self.audio_entity_is_used(sample.m_nEntityIndex)
            {
                continue;
            }
            let Some(active_index) = self.m_asActiveSamples[..self.m_nActiveSamples]
                .iter()
                .position(|active| active.m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32)
            else {
                break;
            };

            if sample.m_nLoopCount > 0 && sample.m_nFramesToPlay <= 0.0 {
                let samples_per_frame = sample.m_nFrequency / u32::from(self.m_nTimeSpent.max(1));
                let samples_to_play = sample
                    .m_nLoopCount
                    .saturating_mul(self.sample_length(sample.m_nSampleIndex));
                if samples_per_frame > 0 {
                    sample.m_nFramesToPlay =
                        (samples_to_play / samples_per_frame).saturating_add(1) as f32;
                }
            }
            sample.m_bIsBeingPlayed = true;
            sample.m_nVolumeChange = -1;
            self.m_asActiveSamples[active_index] = sample;
            self.m_aRequestedQueue[queue][index] = sample;
            self.emitted_samples.push(sample);
        }

        self.emitted_samples.append(&mut self.dedicated_voice_starts);

        // Preserve this frame's queue as the previous queue for release-tail
        // generation on the next frame. The other queue becomes the empty
        // producer queue used by AudioLogic.
        self.m_nActiveQueue = 1 - queue;
        self.ClearRequestedQueue();
    }

    pub fn ReportCollision(
        &mut self,
        entity1: Option<i32>,
        entity2: Option<i32>,
        surface1: u8,
        surface2: u8,
        collision_power: f32,
        velocity: f32,
        position1: CVector,
        position2: CVector,
        entity1_is_building: bool,
        entity2_is_building: bool,
        camera_position: CVector,
    ) {
        if !self.m_bIsInitialised
            || self.m_nCollisionEntity < 0
            || self.m_bIsPaused
            || (velocity < 0.0016 && collision_power < 0.01)
        {
            return;
        }
        let (v1, v2) = if entity1_is_building {
            (position2, position2)
        } else if entity2_is_building {
            (position1, position1)
        } else {
            (position1, position2)
        };
        let pos = CVector::new(
            (v1.x + v2.x) * 0.5,
            (v1.y + v2.y) * 0.5,
            (v1.z + v2.z) * 0.5,
        );
        let dist_squared = distance_squared(pos, camera_position);
        if dist_squared < COLLISION_MAX_DIST * COLLISION_MAX_DIST {
            self.m_sCollisionManager.m_sQueue.m_pEntity1 = entity1;
            self.m_sCollisionManager.m_sQueue.m_pEntity2 = entity2;
            self.m_sCollisionManager.m_sQueue.m_bSurface1 = surface1;
            self.m_sCollisionManager.m_sQueue.m_bSurface2 = surface2;
            self.m_sCollisionManager.m_sQueue.m_fIntensity1 = collision_power;
            self.m_sCollisionManager.m_sQueue.m_fIntensity2 = velocity;
            self.m_sCollisionManager.m_sQueue.m_vecPosition = pos;
            self.m_sCollisionManager.m_sQueue.m_fDistance = dist_squared;
            self.m_sCollisionManager.AddCollisionToRequestedQueue();
        }
    }

    pub fn ServiceCollisions(&mut self) {
        self.m_sQueueSample.m_nEntityIndex = self.m_nCollisionEntity;
        let mut repeated1 = [false; NUMAUDIOCOLLISIONS];
        let mut repeated2 = [false; NUMAUDIOCOLLISIONS];

        for i in 0..self.m_sCollisionManager.m_bCollisionsInQueue {
            let index = self.m_sCollisionManager.m_bIndicesTable[i];
            for j in 0..NUMAUDIOCOLLISIONS {
                if collisions_match(
                    self.m_sCollisionManager.m_asCollisions1[index],
                    self.m_sCollisionManager.m_asCollisions2[j],
                ) {
                    repeated1[index] = true;
                    repeated2[j] = true;
                    self.m_sCollisionManager.m_asCollisions2[j].m_nBaseVolume += 1;
                    self.m_sCollisionManager.m_asCollisions1[index].m_nBaseVolume =
                        self.m_sCollisionManager.m_asCollisions2[j].m_nBaseVolume;
                    self.SetUpLoopingCollisionSound(
                        self.m_sCollisionManager.m_asCollisions1[index],
                        j as u8,
                    );
                    break;
                }
            }
        }

        for (i, repeated) in repeated2.iter().enumerate() {
            if !*repeated {
                self.m_sCollisionManager.m_asCollisions2[i].Reset();
            }
        }

        for i in 0..self.m_sCollisionManager.m_bCollisionsInQueue {
            let index = self.m_sCollisionManager.m_bIndicesTable[i];
            if !repeated1[index] {
                let mut free_index = 0usize;
                for j in 0..NUMAUDIOCOLLISIONS {
                    if !repeated2[j] {
                        free_index = j;
                        self.m_sCollisionManager.m_asCollisions2[j].m_nBaseVolume = 1;
                        self.m_sCollisionManager.m_asCollisions2[j].m_pEntity1 =
                            self.m_sCollisionManager.m_asCollisions1[index].m_pEntity1;
                        self.m_sCollisionManager.m_asCollisions2[j].m_pEntity2 =
                            self.m_sCollisionManager.m_asCollisions1[index].m_pEntity2;
                        self.m_sCollisionManager.m_asCollisions2[j].m_bSurface1 =
                            self.m_sCollisionManager.m_asCollisions1[index].m_bSurface1;
                        self.m_sCollisionManager.m_asCollisions2[j].m_bSurface2 =
                            self.m_sCollisionManager.m_asCollisions1[index].m_bSurface2;
                        break;
                    }
                }
                self.SetUpOneShotCollisionSound(self.m_sCollisionManager.m_asCollisions1[index]);
                self.SetUpLoopingCollisionSound(
                    self.m_sCollisionManager.m_asCollisions1[index],
                    free_index as u8,
                );
            }
        }
        self.m_sCollisionManager.reset_requested_queue();
    }

    pub fn SetUpOneShotCollisionSound(&mut self, col: cAudioCollision) {
        for pass in 0..2 {
            let (mut s1, s2) = if pass == 1 {
                (col.m_bSurface2, col.m_bSurface1)
            } else {
                (col.m_bSurface1, col.m_bSurface2)
            };
            let mut ratio = GetCollisionOneShotRatio(s1 as u32, col.m_fIntensity1);
            if s1 == eSurfaceType::SURFACE_CAR as u8 && s2 == eSurfaceType::SURFACE_PED as u8 {
                ratio /= 4.0;
            }
            if s1 == eSurfaceType::SURFACE_CAR as u8 && ratio < 0.6 {
                s1 = eSurfaceType::SURFACE_CAR_PANEL as u8;
                ratio = (2.0 * ratio).min(1.0);
            }
            let emitting_vol = (40.0 * ratio) as u8;
            if emitting_vol == 0 {
                continue;
            }
            self.m_sQueueSample.m_fDistance = col.m_fDistance.sqrt();
            self.m_sQueueSample.m_nVolume = self.ComputeVolume(
                emitting_vol,
                COLLISION_MAX_DIST,
                self.m_sQueueSample.m_fDistance,
            );
            if self.m_sQueueSample.m_nVolume == 0 {
                continue;
            }
            self.m_sQueueSample.m_nSampleIndex = G_ONE_SHOT_COL
                .get(s1 as usize)
                .copied()
                .unwrap_or(SFX_COL_TARMAC_1)
                as u32;
            self.m_sQueueSample.m_nSampleIndex += match self.m_sQueueSample.m_nSampleIndex as i32 {
                SFX_COL_TARMAC_1 => self.m_anRandomTable[3].rem_euclid(5) as u32,
                SFX_COL_CAR_PANEL_1 => self.m_anRandomTable[0].rem_euclid(6) as u32,
                SFX_COL_LAMP_POST_1 => self.m_anRandomTable[1].rem_euclid(2) as u32,
                SFX_COL_METAL_CHAIN_FENCE_1 => self.m_anRandomTable[3].rem_euclid(4) as u32,
                SFX_COL_PED_1 => self.m_anRandomTable[4].rem_euclid(5) as u32,
                SFX_COL_WOOD_CRATES_1 => self.m_anRandomTable[4].rem_euclid(4) as u32,
                SFX_COL_WOOD_BENCH_1 => self.m_anRandomTable[1].rem_euclid(4) as u32,
                SFX_COL_VEG_1 => self.m_anRandomTable[2].rem_euclid(5) as u32,
                SFX_COL_NEWS_VENDOR_1 => self.m_anRandomTable[2].rem_euclid(3) as u32,
                SFX_COL_CAR_1 => self.m_anRandomTable[1].rem_euclid(5) as u32,
                SFX_COL_CARDBOARD_1 => self.m_anRandomTable[3].rem_euclid(2) as u32,
                _ => 0,
            };
            let base_frequency = match s1 {
                x if x == eSurfaceType::SURFACE_GLASS as u8 => 13_500,
                x if x == eSurfaceType::SURFACE_GIRDER as u8 => 8_819,
                x if x == eSurfaceType::SURFACE_WATER as u8 => {
                    2 * self.sample_base_frequency(self.m_sQueueSample.m_nSampleIndex)
                }
                x if x == eSurfaceType::SURFACE_RUBBER as u8 => 6_000,
                x if x == eSurfaceType::SURFACE_PLASTIC as u8 => 8_000,
                _ => self.sample_base_frequency(self.m_sQueueSample.m_nSampleIndex),
            };
            self.m_sQueueSample.m_nFrequency = (base_frequency as i32
                + self.RandomDisplacement((base_frequency / 16).max(1)))
            .max(0) as u32;
            self.m_sQueueSample.m_nCounter = self.collision_one_shot_counter;
            self.collision_one_shot_counter += 1;
            if self.collision_one_shot_counter >= 255 {
                self.collision_one_shot_counter = 28;
            }
            self.m_sQueueSample.m_vecPos = col.m_vecPosition;
            self.m_sQueueSample.m_nBankIndex = SFX_BANK_0;
            self.m_sQueueSample.m_bIs2D = false;
            self.m_sQueueSample.m_nPriority = 11;
            self.m_sQueueSample.m_nLoopCount = 1;
            self.m_sQueueSample.m_nEmittingVolume = emitting_vol;
            self.m_sQueueSample.m_nLoopStart = 0;
            self.m_sQueueSample.m_nLoopEnd = -1;
            self.m_sQueueSample.m_fSpeedMultiplier = 4.0;
            self.m_sQueueSample.m_MaxDistance = COLLISION_MAX_DIST;
            self.m_sQueueSample.m_bStatic = true;
            self.m_sQueueSample.m_bReverb = true;
            self.AddSampleToRequestedQueue();
        }
    }

    pub fn SetUpLoopingCollisionSound(&mut self, col: cAudioCollision, counter: u8) {
        if col.m_fIntensity2 <= 0.0016 {
            return;
        }
        let emitting_vol = self.SetLoopingCollisionRequestedSfxFreqAndGetVol(col);
        if emitting_vol == 0 {
            return;
        }
        self.m_sQueueSample.m_fDistance = col.m_fDistance.sqrt();
        self.m_sQueueSample.m_nVolume = self.ComputeVolume(
            emitting_vol,
            COLLISION_MAX_DIST,
            self.m_sQueueSample.m_fDistance,
        );
        if self.m_sQueueSample.m_nVolume == 0 {
            return;
        }
        self.m_sQueueSample.m_nCounter = counter as u32;
        self.m_sQueueSample.m_vecPos = col.m_vecPosition;
        self.m_sQueueSample.m_nBankIndex = SFX_BANK_0;
        self.m_sQueueSample.m_bIs2D = false;
        self.m_sQueueSample.m_nPriority = 7;
        self.m_sQueueSample.m_nLoopCount = 0;
        self.m_sQueueSample.m_nEmittingVolume = emitting_vol;
        self.m_sQueueSample.m_nLoopStart =
            self.sample_loop_start(self.m_sQueueSample.m_nSampleIndex);
        self.m_sQueueSample.m_nLoopEnd = self.sample_loop_end(self.m_sQueueSample.m_nSampleIndex);
        self.m_sQueueSample.m_fSpeedMultiplier = 4.0;
        self.m_sQueueSample.m_MaxDistance = COLLISION_MAX_DIST;
        self.m_sQueueSample.m_bStatic = false;
        self.m_sQueueSample.m_nFramesToPlay = 5.0;
        self.m_sQueueSample.m_bReverb = true;
        self.AddSampleToRequestedQueue();
    }

    pub fn SetLoopingCollisionRequestedSfxFreqAndGetVol(
        &mut self,
        audio_collision: cAudioCollision,
    ) -> u8 {
        let surface1 = audio_collision.m_bSurface1;
        let surface2 = audio_collision.m_bSurface2;
        let ratio = GetCollisionRatio(audio_collision.m_fIntensity2, 0.0001, 0.09, 0.0899);
        let has = |s: eSurfaceType| surface1 == s as u8 || surface2 == s as u8;
        let mut vol;
        if has(eSurfaceType::SURFACE_GRASS) || has(eSurfaceType::SURFACE_HEDGE) {
            self.m_sQueueSample.m_nSampleIndex = SFX_RAIN as u32;
            self.m_sQueueSample.m_nFrequency = (13000.0 * ratio + 35000.0) as u32;
            vol = (50.0 * ratio) as i32;
        } else if has(eSurfaceType::SURFACE_WATER) {
            self.m_sQueueSample.m_nSampleIndex = SFX_BOAT_WATER_LOOP as u32;
            self.m_sQueueSample.m_nFrequency = (6050.0 * ratio + 16000.0) as u32;
            vol = (30.0 * ratio) as i32;
        } else if has(eSurfaceType::SURFACE_GRAVEL)
            || has(eSurfaceType::SURFACE_MUD_DRY)
            || has(eSurfaceType::SURFACE_SAND)
        {
            self.m_sQueueSample.m_nSampleIndex = SFX_GRAVEL_SKID as u32;
            self.m_sQueueSample.m_nFrequency = (6000.0 * ratio + 10000.0) as u32;
            vol = (50.0 * ratio) as i32;
        } else if has(eSurfaceType::SURFACE_PED) {
            return 0;
        } else {
            self.m_sQueueSample.m_nSampleIndex = SFX_SCRAPE_CAR_1 as u32;
            self.m_sQueueSample.m_nFrequency = (10000.0 * ratio + 10000.0) as u32;
            vol = (40.0 * ratio) as i32;
        }
        if audio_collision.m_nBaseVolume < 2 {
            vol = audio_collision.m_nBaseVolume * vol / 2;
        }
        vol.clamp(0, MAX_VOLUME as i32) as u8
    }

    pub fn InitialisePoliceRadio(&mut self) {
        self.m_sPoliceRadioQueue.Reset();
        for crime in &mut self.m_aCrimes {
            crime.r#type = CRIME_NONE;
            crime.timer = 0;
        }
        self.mission_police_audio_sfx = TOTAL_AUDIO_SAMPLES as u32;
        self.mission_police_audio_status = PLAY_STATUS_FINISHED as i8;
        self.min_time_to_next_report = [self.m_FrameCounter; NUM_CRIME_TYPES];
    }

    pub fn ResetPoliceRadio(&mut self) {
        if self.m_bIsInitialised {
            self.InitialisePoliceRadio();
        }
    }

    pub fn SetMissionScriptPoliceAudio(&mut self, sfx: u32) {
        if self.m_bIsInitialised && self.mission_police_audio_status != PLAY_STATUS_PLAYING as i8 {
            self.mission_police_audio_status = PLAY_STATUS_STOPPED as i8;
            self.mission_police_audio_sfx = sfx;
        }
    }

    pub fn GetMissionScriptPoliceAudioPlayingStatus(&self) -> i8 {
        self.mission_police_audio_status
    }

    pub fn ReportCrime(&mut self, crime_type: eCrimeType, pos: CVector) {
        if !self.m_bIsInitialised
            || crime_type <= CRIME_NONE
            || crime_type as usize >= NUM_CRIME_TYPES
        {
            return;
        }
        if self.m_FrameCounter < self.min_time_to_next_report[crime_type as usize] {
            return;
        }
        if let Some(slot) = self
            .m_aCrimes
            .iter_mut()
            .find(|crime| crime.r#type == CRIME_NONE)
        {
            slot.r#type = crime_type;
            slot.position = pos;
            slot.timer = 0;
            self.min_time_to_next_report[crime_type as usize] = self.m_FrameCounter + 180;
        }
    }

    pub fn PreloadMissionAudio(&mut self, mission_audio_sample: u32) {
        if !self.m_bIsInitialised {
            return;
        }
        self.m_nMissionAudioSampleIndex = mission_audio_sample;
        self.m_nMissionAudioLoadingStatus = LOADING_STATUS_NOT_LOADED;
        self.m_nMissionAudioPlayStatus = PLAY_STATUS_STOPPED;
        self.m_bIsMissionAudioPlaying = false;
        self.m_bIsMissionAudioAllowedToPlay = false;
        self.m_bIsMissionAudio2D = true;
    }

    pub fn MarkMissionAudioLoaded(&mut self) {
        if !self.m_bIsInitialised {
            return;
        }
        if matches!(
            self.m_nMissionAudioLoadingStatus,
            LOADING_STATUS_NOT_LOADED | LOADING_STATUS_LOADING
        ) {
            self.m_nMissionAudioLoadingStatus = LOADING_STATUS_LOADED;
        }
    }

    pub fn GetMissionAudioLoadingStatus(&self) -> u8 {
        if self.m_bIsInitialised {
            self.m_nMissionAudioLoadingStatus
        } else {
            LOADING_STATUS_LOADED
        }
    }

    pub fn SetMissionAudioLocation(&mut self, x: f32, y: f32, z: f32) {
        if !self.m_bIsInitialised {
            return;
        }
        self.m_vecMissionAudioPosition = CVector::new(x, y, z);
        self.m_bIsMissionAudio2D = false;
    }

    pub fn PlayLoadedMissionAudio(&mut self) {
        if !self.m_bIsInitialised {
            return;
        }
        if self.m_nMissionAudioSampleIndex < TOTAL_AUDIO_SAMPLES as u32
            && self.m_nMissionAudioLoadingStatus == LOADING_STATUS_LOADED
            && self.m_nMissionAudioPlayStatus == PLAY_STATUS_STOPPED
        {
            self.m_bIsMissionAudioAllowedToPlay = true;
        }
    }

    pub fn IsMissionAudioSampleFinished(&self) -> bool {
        if self.m_bIsInitialised {
            self.m_nMissionAudioPlayStatus == PLAY_STATUS_FINISHED
        } else {
            MISSION_AUDIO_PRETEND_FRAME.fetch_add(1, Ordering::Relaxed) & 63 == 0
        }
    }

    pub fn MarkMissionAudioBackendPlaying(&mut self) {
        if self.m_nMissionAudioPlayStatus == PLAY_STATUS_PLAYING {
            self.m_bIsMissionAudioPlaying = true;
        }
    }

    pub fn MarkMissionAudioBackendFinished(&mut self) {
        // re3 only accepts stream completion after IsStreamPlaying has first
        // proved that the backend really started (`m_bIsMissionAudioPlaying`).
        if self.m_nMissionAudioPlayStatus == PLAY_STATUS_PLAYING && self.m_bIsMissionAudioPlaying {
            self.m_nMissionAudioPlayStatus = PLAY_STATUS_FINISHED;
            self.m_nMissionAudioSampleIndex = TOTAL_AUDIO_SAMPLES as u32;
            self.m_bIsMissionAudioPlaying = false;
            self.m_nMissionAudioFramesToPlay = 0;
        }
    }

    pub fn ClearMissionAudio(&mut self) {
        if !self.m_bIsInitialised {
            return;
        }
        self.m_nMissionAudioSampleIndex = TOTAL_AUDIO_SAMPLES as u32;
        self.m_nMissionAudioLoadingStatus = LOADING_STATUS_NOT_LOADED;
        self.m_nMissionAudioPlayStatus = PLAY_STATUS_STOPPED;
        self.m_bIsMissionAudioPlaying = false;
        self.m_bIsMissionAudioAllowedToPlay = false;
        self.m_bIsMissionAudio2D = true;
        self.m_nMissionAudioFramesToPlay = 0;
    }

    /// PC re3 mission-audio state transition core. Backend stream completion
    /// is represented by the immediate PC NOT_LOADED -> LOADED preload step.
    pub fn ProcessMissionAudio(&mut self) {
        if !self.m_bIsInitialised || self.m_nMissionAudioSampleIndex >= TOTAL_AUDIO_SAMPLES as u32 {
            return;
        }
        if self.m_nMissionAudioLoadingStatus == LOADING_STATUS_NOT_LOADED {
            self.m_nMissionAudioLoadingStatus = LOADING_STATUS_LOADED;
            return;
        }
        if self.m_nMissionAudioLoadingStatus == LOADING_STATUS_LOADED
            && self.m_bIsMissionAudioAllowedToPlay
            && self.m_nMissionAudioPlayStatus == PLAY_STATUS_STOPPED
        {
            self.m_nMissionAudioPlayStatus = PLAY_STATUS_PLAYING;
            self.m_bIsMissionAudioPlaying = false;
        }
    }

    pub fn GenerateIntegerRandomNumberTable(&mut self) {
        self.m_anRandomTable = [
            self.m_FrameCounter
                .wrapping_mul(1103515245)
                .wrapping_add(12345) as i32,
            self.m_FrameCounter
                .wrapping_mul(1103515245)
                .wrapping_add(23456) as i32,
            self.m_FrameCounter
                .wrapping_mul(1103515245)
                .wrapping_add(34567) as i32,
            self.m_FrameCounter
                .wrapping_mul(1103515245)
                .wrapping_add(45678) as i32,
            self.m_FrameCounter
                .wrapping_mul(1103515245)
                .wrapping_add(56789) as i32,
        ];
    }

    pub fn SetSampleMetadata(&mut self, metadata: &[SampleMetadata]) {
        self.sample_metadata.clear();
        self.sample_metadata.extend_from_slice(metadata);
    }

    pub fn sample_base_frequency(&self, sample: u32) -> u32 {
        self.sample_metadata
            .get(sample as usize)
            .map(|sample| sample.frequency)
            .unwrap_or_else(|| {
                (sample < TOTAL_AUDIO_SAMPLES as u32)
                    .then_some(22_050)
                    .unwrap_or(0)
            })
    }

    pub(crate) fn has_sample_metadata(&self, sample: u32) -> bool {
        self.sample_metadata.get(sample as usize).is_some()
    }

    pub(crate) fn sample_loop_start(&self, sample: u32) -> u32 {
        self.sample_metadata
            .get(sample as usize)
            .map_or(0, |sample| sample.loop_start)
    }

    pub(crate) fn sample_loop_end(&self, sample: u32) -> i32 {
        self.sample_metadata
            .get(sample as usize)
            .map_or(-1, |sample| sample.loop_end)
    }

    /// re3 `cAudioManager::RandomDisplacement`. The sign and table slot are
    /// intentionally stateful across calls; many vehicle/collision one-shots
    /// rely on this sequence for their small pitch variations.
    pub fn RandomDisplacement(&mut self, seed: u32) -> i32 {
        if seed == 0 {
            return 0;
        }
        let index = self.random_displacement_adjustment.wrapping_add(seed) % 5;
        // `seed` is uint32 in re3, so the signed table entry is converted to
        // uint32 before `% seed` by C++'s usual arithmetic conversions.
        let mut value = ((self.m_anRandomTable[index as usize] as u32) % seed) as i32;
        self.random_displacement_adjustment = self
            .random_displacement_adjustment
            .wrapping_add(value as u32);
        if value % 2 != 0 {
            self.random_displacement_positive = !self.random_displacement_positive;
        }
        if !self.random_displacement_positive {
            value = -value;
        }
        value
    }

    fn entity_index(&self, id: i32) -> Option<usize> {
        if self.m_bIsInitialised
            && id >= 0
            && (id as usize) < NUM_AUDIOENTITIES
            && self.m_asAudioEntities[id as usize].m_bIsUsed
        {
            Some(id as usize)
        } else {
            None
        }
    }
}

fn one_shot_priority(sound: usize) -> u8 {
    ONE_SHOT_PRIORITY.get(sound).copied().unwrap_or(0)
}

fn distance_squared(a: CVector, b: CVector) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

fn collisions_match(a: cAudioCollision, b: cAudioCollision) -> bool {
    a.m_pEntity1 == b.m_pEntity1
        && a.m_pEntity2 == b.m_pEntity2
        && a.m_bSurface1 == b.m_bSurface1
        && a.m_bSurface2 == b.m_bSurface2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dedicated_sample(manager: &cAudioManager) -> tSound {
        tSound {
            m_nEntityIndex: manager.m_nFrontEndEntity,
            m_nCounter: crate::audio::DMAudio::PLAYER_ENGINE_VOICE_COUNTER,
            m_nSampleIndex: 0,
            m_nFrequency: 22_050,
            m_nVolume: 100,
            m_nLoopCount: 1,
            ..Default::default()
        }
    }

    #[test]
    fn dedicated_channel_bypasses_full_generic_queue_and_has_no_release_ledger() {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        for counter in 100..100 + NUM_CHANNELS_GENERIC as u32 {
            manager.m_sQueueSample = tSound {
                m_nCounter: counter,
                m_nPriority: 0,
                m_nVolume: MAX_VOLUME,
                ..dedicated_sample(&manager)
            };
            manager.AddSampleToRequestedQueue();
        }
        let queue = manager.m_nActiveQueue;
        assert_eq!(manager.m_nRequestedCount[queue], NUM_CHANNELS_GENERIC);
        let sample = dedicated_sample(&manager);
        manager.StartDedicatedVoice(sample);
        assert_eq!(manager.m_nRequestedCount[queue], NUM_CHANNELS_GENERIC);
        manager.ProcessActiveQueues();
        assert!(manager.emitted_samples.iter().any(|sound| sound.m_nCounter == sample.m_nCounter));
        assert!(!manager.m_asActiveSamples.iter().any(|sound| sound.m_nCounter == sample.m_nCounter));
        manager.ProcessActiveQueues();
        assert!(!manager.emitted_samples.iter().any(|sound| sound.m_nCounter == sample.m_nCounter));
        manager.finish_voice(sample.m_nEntityIndex, sample.m_nCounter);
        manager.StartDedicatedVoice(sample);
        manager.ProcessActiveQueues();
        assert_eq!(manager.emitted_samples.iter().filter(|sound| sound.m_nCounter == sample.m_nCounter).count(), 1);
    }

    #[test]
    fn dedicated_start_is_cancelled_by_stop_pause_termination_and_retirement() {
        enum Cancellation { Stop, Pause, Terminate, Retire }
        for cancel in [Cancellation::Stop, Cancellation::Pause, Cancellation::Terminate, Cancellation::Retire] {
            let mut manager = cAudioManager::default();
            manager.Initialise();
            let sample = dedicated_sample(&manager);
            manager.StartDedicatedVoice(sample);
            match cancel {
                Cancellation::Stop => manager.StopVoice(sample.m_nEntityIndex, sample.m_nCounter),
                Cancellation::Pause => { manager.SyncUserPauseState(true); }
                Cancellation::Terminate => manager.Terminate(),
                Cancellation::Retire => manager.DestroyEntity(sample.m_nEntityIndex),
            }
            assert!(manager.dedicated_voice_starts.is_empty());
            manager.ProcessActiveQueues();
            assert!(!manager.emitted_samples.iter().any(|sound| sound.m_nCounter == sample.m_nCounter));
        }
    }

    #[test]
    fn test_compute_volume_attenuation_and_cutoff() {
        let manager = cAudioManager::default();
        let max_dist = 50.0;
        let emitting = 100u8;

        // At or below min_distance (50 / 5 = 10), volume is 100%
        assert_eq!(manager.ComputeVolume(emitting, max_dist, 0.0), emitting);
        assert_eq!(manager.ComputeVolume(emitting, max_dist, 10.0), emitting);

        // Intermediate distances attenuate
        let v20 = manager.ComputeVolume(emitting, max_dist, 20.0);
        let v30 = manager.ComputeVolume(emitting, max_dist, 30.0);
        let v40 = manager.ComputeVolume(emitting, max_dist, 40.0);
        assert!(emitting > v20);
        assert!(v20 > v30);
        assert!(v30 > v40);
        assert!(v40 > 0);

        // At or beyond max_distance, volume must be 0 (never wrap/invert!)
        assert_eq!(manager.ComputeVolume(emitting, max_dist, 50.0), 0);
        assert_eq!(manager.ComputeVolume(emitting, max_dist, 50.1), 0);
        assert_eq!(manager.ComputeVolume(emitting, max_dist, 100.0), 0);
        assert_eq!(manager.ComputeVolume(emitting, max_dist, 500.0), 0);

        // Zero or negative max_distance returns 0
        assert_eq!(manager.ComputeVolume(emitting, 0.0, 10.0), 0);
        assert_eq!(manager.ComputeVolume(emitting, -10.0, 10.0), 0);
    }

    #[test]
    fn entering_user_pause_clears_both_queues_and_active_channels() {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        manager.m_sQueueSample = tSound {
            m_nEntityIndex: manager.m_nFrontEndEntity,
            m_nCounter: 9,
            m_nSampleIndex: 0,
            m_nFrequency: 22_050,
            m_nVolume: 100,
            m_nLoopCount: 0,
            ..Default::default()
        };
        manager.AddSampleToRequestedQueue();
        manager.ProcessActiveQueues();
        assert!(
            manager
                .m_asActiveSamples
                .iter()
                .any(|sample| sample.m_nSampleIndex < TOTAL_AUDIO_SAMPLES as u32)
        );

        manager.m_sQueueSample.m_nCounter = 10;
        manager.AddSampleToRequestedQueue();
        assert!(manager.SyncUserPauseState(true));

        assert!(manager.m_bIsPaused);
        assert!(!manager.m_bWasPaused);
        assert_eq!(manager.m_nRequestedCount, [0, 0]);
        assert!(
            manager
                .m_asActiveSamples
                .iter()
                .all(|sample| sample.m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32)
        );
        assert!(manager.emitted_samples.is_empty());
        assert!(manager.voice_updates.is_empty());

        assert!(!manager.SyncUserPauseState(true));
        manager.SyncUserPauseState(false);
        assert!(!manager.m_bIsPaused);
        assert!(manager.m_bWasPaused);
    }

    #[test]
    fn sample_metadata_drives_pitch_and_one_shot_lifetime_without_retriggering() {
        let mut manager = cAudioManager::default();
        manager.SetSampleMetadata(&[SampleMetadata {
            size: 44_100,
            frequency: 44_100,
            ..Default::default()
        }]);
        manager.Initialise();
        assert_eq!(manager.sample_base_frequency(0), 44_100);

        manager.m_sQueueSample = tSound {
            m_nEntityIndex: manager.m_nFrontEndEntity,
            m_nCounter: 7,
            m_nSampleIndex: 0,
            m_nFrequency: 44_100,
            m_nVolume: 100,
            m_nLoopCount: 1,
            ..Default::default()
        };
        manager.AddSampleToRequestedQueue();
        manager.ProcessActiveQueues();
        assert_eq!(manager.emitted_samples.len(), 1);
        assert!(manager.m_asActiveSamples[0].m_nFramesToPlay > 1.0);

        // No fresh request: re3 carries finite-sample channel bookkeeping via
        // AddReleasingSounds, but does not start the one-shot a second time.
        manager.ProcessActiveQueues();
        assert!(manager.emitted_samples.is_empty());
        assert_eq!(manager.m_asActiveSamples[0].m_nSampleIndex, 0);
    }

    #[test]
    fn collision_scrape_uses_its_own_gain_and_authored_loop_points() {
        let mut manager = cAudioManager::default();
        let mut metadata = vec![SampleMetadata::default(); SFX_SCRAPE_CAR_1 as usize + 1];
        metadata[SFX_SCRAPE_CAR_1 as usize] = SampleMetadata {
            frequency: 22_050,
            loop_start: 1_234,
            loop_end: 5_678,
            ..Default::default()
        };
        manager.SetSampleMetadata(&metadata);

        // m_sQueueSample is intentionally reused by re3. A previous loud 3D
        // sound must not leak its emitting volume into a collision scrape.
        manager.m_sQueueSample.m_nEmittingVolume = MAX_VOLUME;
        manager.SetUpLoopingCollisionSound(
            cAudioCollision {
                m_fIntensity2: 0.09,
                m_fDistance: 1.0,
                m_nBaseVolume: 2,
                ..Default::default()
            },
            0,
        );

        assert_eq!(
            manager.m_sQueueSample.m_nSampleIndex,
            SFX_SCRAPE_CAR_1 as u32
        );
        assert_eq!(manager.m_sQueueSample.m_nEmittingVolume, 40);
        assert_eq!(manager.m_sQueueSample.m_nLoopStart, 1_234);
        assert_eq!(manager.m_sQueueSample.m_nLoopEnd, 5_678);
    }

    #[test]
    fn collision_one_shot_overwrites_stale_emitting_volume() {
        let mut manager = cAudioManager::default();
        manager.m_sQueueSample.m_nEmittingVolume = MAX_VOLUME;
        manager.SetUpOneShotCollisionSound(cAudioCollision {
            m_bSurface1: eSurfaceType::SURFACE_TARMAC as u8,
            m_bSurface2: eSurfaceType::SURFACE_TARMAC as u8,
            m_fIntensity1: 60.0,
            m_fDistance: 1.0,
            ..Default::default()
        });

        assert_eq!(manager.m_sQueueSample.m_nEmittingVolume, 40);
    }

    #[test]
    fn external_3d_gain_fades_to_zero_in_last_quarter() {
        let manager = cAudioManager::default();
        assert_eq!(manager.ComputeEmittingVolume(100, 60.0, 45.0), 100);
        assert_eq!(manager.ComputeEmittingVolume(100, 60.0, 52.5), 50);
        assert_eq!(manager.ComputeEmittingVolume(100, 60.0, 59.0), 6);
        assert_eq!(manager.ComputeEmittingVolume(100, 60.0, 60.0), 0);
    }

    #[test]
    fn finite_collision_channel_does_not_renew_its_last_release_frame() {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        manager.m_sQueueSample = tSound {
            m_nEntityIndex: manager.m_nCollisionEntity,
            m_nCounter: 28,
            m_nSampleIndex: SFX_COL_CAR_1 as u32,
            m_nFrequency: 22_050,
            m_nVolume: 40,
            m_nLoopCount: 1,
            m_nFramesToPlay: 2.0,
            ..Default::default()
        };
        manager.AddSampleToRequestedQueue();
        manager.ProcessActiveQueues();
        for _ in 0..4 {
            manager.ProcessActiveQueues();
        }
        assert!(
            manager
                .m_asActiveSamples
                .iter()
                .all(|s| s.m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32)
        );
        assert!(manager.m_nRequestedCount.iter().all(|count| *count == 0));
    }

    #[test]
    fn scrape_contact_stops_after_native_release_tail() {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        for _ in 0..3 {
            manager.ReportCollision(
                Some(7),
                Some(9),
                eSurfaceType::SURFACE_CAR as u8,
                eSurfaceType::SURFACE_TARMAC as u8,
                20.0,
                0.04,
                CVector::ZERO,
                CVector::ZERO,
                false,
                true,
                CVector::ZERO,
            );
            manager.Service();
        }
        assert!(
            manager
                .emitted_samples
                .iter()
                .any(|s| s.m_nSampleIndex == SFX_SCRAPE_CAR_1 as u32 && s.m_nLoopCount == 0)
        );
        for _ in 0..8 {
            manager.Service();
        }
        assert!(manager.emitted_samples.is_empty());
        assert!(
            manager
                .m_asActiveSamples
                .iter()
                .all(|s| s.m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32)
        );
        assert_eq!(manager.m_sCollisionManager.m_bCollisionsInQueue, 0);
    }

    #[test]
    fn looping_release_is_explicit_and_then_stops() {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        manager.m_sQueueSample = tSound {
            m_nEntityIndex: manager.m_nFrontEndEntity,
            m_nCounter: 8,
            m_nSampleIndex: 0,
            m_nFrequency: 22_050,
            m_nVolume: 100,
            m_nLoopCount: 0,
            m_nFramesToPlay: 2.0,
            ..Default::default()
        };
        manager.AddSampleToRequestedQueue();
        manager.ProcessActiveQueues();
        assert_eq!(manager.emitted_samples.len(), 1);

        manager.ProcessActiveQueues();
        assert_eq!(manager.emitted_samples.len(), 1);
        assert!(manager.emitted_samples[0].m_nVolume < 100);

        manager.ProcessActiveQueues();
        assert!(manager.emitted_samples.is_empty());
        assert!(manager.m_asActiveSamples[0].m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32);
    }

    #[test]
    fn active_3d_loop_slews_volume_and_frequency_like_re3() {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        let entity = manager.m_nFrontEndEntity;
        manager.m_sQueueSample = tSound {
            m_nEntityIndex: entity,
            m_nCounter: 2,
            m_nSampleIndex: 0,
            m_nFrequency: 10_000,
            m_nVolume: 10,
            m_nEmittingVolume: 10,
            m_nLoopCount: 0,
            m_fDistance: 10.0,
            m_MaxDistance: 80.0,
            m_fSpeedMultiplier: 0.0,
            m_bStatic: false,
            m_bIs2D: false,
            m_nFramesToPlay: 8.0,
            ..Default::default()
        };
        manager.AddSampleToRequestedQueue();
        manager.ProcessActiveQueues();

        manager.m_sQueueSample = tSound {
            m_nEntityIndex: entity,
            m_nCounter: 2,
            m_nSampleIndex: 0,
            m_nFrequency: 30_000,
            m_nVolume: 100,
            m_nEmittingVolume: 100,
            m_nLoopCount: 0,
            m_fDistance: 10.0,
            m_MaxDistance: 80.0,
            m_fSpeedMultiplier: 0.0,
            m_bStatic: false,
            m_bIs2D: false,
            m_nFramesToPlay: 8.0,
            ..Default::default()
        };
        manager.AddSampleToRequestedQueue();
        manager.ProcessActiveQueues();

        assert_eq!(manager.emitted_samples.len(), 1);
        assert_eq!(manager.emitted_samples[0].m_nFrequency, 16_000);
        assert_eq!(manager.emitted_samples[0].m_nVolume, 100);
        assert_eq!(manager.emitted_samples[0].m_nEmittingVolume, 20);
    }

    #[test]
    fn doppler_is_applied_once_in_audio_manager() {
        let manager = cAudioManager::default();
        let approach = manager.ComputeDopplerEffectedFrequency(22_050, 40.0, 30.0, 6.0);
        let recede = manager.ComputeDopplerEffectedFrequency(22_050, 30.0, 40.0, 6.0);
        assert!(approach > 22_050);
        assert!(recede < 22_050);
        assert_eq!(
            manager.ComputeDopplerEffectedFrequency(22_050, 40.0, 30.0, 0.0),
            22_050
        );
        assert_eq!(
            manager.ComputeDopplerEffectedFrequency(22_050, 10.0, 1000.0, 6.0),
            22_050
        );
    }

    #[test]
    fn explicit_voice_stop_cannot_be_resurrected_by_release_queue() {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        let entity = manager.m_nFrontEndEntity;
        manager.m_sQueueSample = tSound {
            m_nEntityIndex: entity,
            m_nCounter: 65,
            m_nSampleIndex: 0,
            m_nFrequency: 22_050,
            m_nVolume: 100,
            m_nLoopCount: 0,
            m_nFramesToPlay: 8.0,
            ..Default::default()
        };
        manager.AddSampleToRequestedQueue();
        manager.ProcessActiveQueues();
        assert_eq!(manager.emitted_samples.len(), 1);

        manager.StopVoice(entity, 65);
        manager.ProcessActiveQueues();
        assert!(manager.emitted_samples.is_empty());
        assert!(manager.m_asActiveSamples[0].m_nSampleIndex >= TOTAL_AUDIO_SAMPLES as u32);
    }

    #[test]
    fn release_tail_uses_native_elapsed_frames_at_both_render_rates() {
        for (step, services) in [(1.0, 3), (0.5, 6)] {
            let mut manager = cAudioManager::default();
            manager.Initialise();
            manager.release_time_step = step;
            let entity = manager.m_nFrontEndEntity;
            manager.m_sQueueSample = tSound { m_nEntityIndex: entity, m_nSampleIndex: 0,
                m_nCounter: 19, m_nFrequency: 22050, m_nVolume: 100,
                m_nLoopCount: 1, m_nFramesToPlay: 8.0, m_bStatic: false, ..Default::default() };
            manager.AddSampleToRequestedQueue();
            manager.ProcessActiveQueues();
            for _ in 0..services { manager.ProcessActiveQueues(); }
            let sample = manager.m_aRequestedQueue[1 - manager.m_nActiveQueue].iter()
                .find(|sound| sound.m_nSampleIndex == 0 && sound.m_nCounter == 19).expect("live native release tail");
            assert_eq!(sample.m_nFramesToPlay, 5.0);
        }
    }
}
