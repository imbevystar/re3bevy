//! re3 reference: references/re-GTA/src/audio/AudioLogic.cpp GetPedCommentSfx/Get*TalkSfx phrase resolver.
#![allow(dead_code)]
use bevy::prelude::{Res, ResMut, Resource};
use re3bevy_core::core::Timer::CTimer;

use super::AudioLogic::{
    PED_COMMENT_MAX_DIST, PED_COMMENT_POLICE_HELI_MAX_DIST, PED_COMMENT_VOLUME,
    PED_COMMENT_VOLUME_BEHIND_WALL,
};
use super::AudioManager::{cAudioManager, tPedComment};
use super::AudioSamples::*;
use super::audio_enums::AUDIOTYPE_PHYSICAL;
use super::soundlist::*;
use re3bevy_core::core::Coordinates::CVector;

use crate::core::Camera::CCamera;
use crate::modelinfo::ModelIndices::{
    MI_ARMY,
    MI_B_MAN1,
    MI_B_MAN2,
    MI_B_MAN3,
    MI_B_WOM1,
    MI_B_WOM2,
    MI_B_WOM3,
    MI_CAS_MAN,
    MI_CAS_WOM,
    MI_CONST1,
    MI_CONST2,
    MI_COP,
    MI_CRIMINAL01,
    MI_CRIMINAL02,
    MI_CT_MAN1,
    MI_CT_MAN2,
    MI_CT_WOM1,
    MI_CT_WOM2,
    MI_DOCKER1,
    MI_DOCKER2,
    MI_FAN_MAN1,
    MI_FAN_MAN2,
    MI_FAN_WOM,
    MI_FATFEMALE01,
    MI_FATFEMALE02,
    MI_FATMALE01,
    MI_FATMALE02,
    MI_FBI,
    MI_FEMALE01,
    MI_FEMALE02,
    MI_FEMALE03,
    MI_FIREMAN,
    MI_GANG01,
    MI_GANG02,
    MI_GANG03,
    MI_GANG04,
    MI_GANG05,
    MI_GANG06,
    MI_GANG07,
    MI_GANG08,
    MI_GANG09,
    MI_GANG10,
    MI_GANG11,
    MI_GANG12,
    MI_GANG13,
    MI_GANG14,
    MI_HOS_MAN,
    MI_HOS_WOM,
    MI_LI_MAN1,
    MI_LI_MAN2,
    MI_LI_WOM1,
    MI_LI_WOM2,
    MI_MALE01,
    MI_MALE02,
    MI_MALE03,
    MI_MEDIC,
    MI_MOD_MAN,
    MI_MOD_WOM,
    MI_PIMP,
    MI_PROSTITUTE,
    MI_PROSTITUTE2,
    MI_P_MAN1,
    MI_P_MAN2,
    MI_P_WOM1,
    MI_P_WOM2,
    MI_SCUM_MAN,
    MI_SCUM_WOM,
    MI_SHOPPER1,
    MI_SHOPPER2,
    MI_SHOPPER3,
    MI_SPECIAL01,
    MI_SPECIAL02,
    MI_SPECIAL03,
    MI_SPECIAL04,
    MI_STUD_MAN,
    MI_STUD_WOM,
    MI_ST_MAN,
    MI_ST_WOM,
    MI_SWAT,
    MI_TAXI_D,
    MI_WORKER1,
    MI_WORKER2
};
use crate::peds::Ped::CPed;
use super::AudioActorAccess::{AudioPedSpeech, AudioActorDiagnostics};

use super::DMAudio::cDMAudio;

const NO_SAMPLE_U32: u32 = u32::MAX;

/// PedChat.cpp CommentWaitTime, ordered SOUND_PED_DEATH through TAXI_CALL.
pub(crate) struct PedCommentTiming {
    pub playback_delay: u32,
    pub playback_random_delay: i32,
    pub repeat_delay: u32,
    pub repeat_random_delay: i32,
}

impl PedCommentTiming {
    const fn new(playback_delay: u32, playback_random_delay: i32,
        repeat_delay: u32, repeat_random_delay: i32) -> Self {
        Self { playback_delay, playback_random_delay, repeat_delay, repeat_random_delay }
    }
}

pub(crate) const PED_COMMENT_TIMING: [PedCommentTiming; 39] = [
    PedCommentTiming::new(500,800,500,2), PedCommentTiming::new(500,800,500,2),
    PedCommentTiming::new(500,800,500,2), PedCommentTiming::new(500,800,500,2),
    PedCommentTiming::new(100,2,100,2), PedCommentTiming::new(700,500,1000,500),
    PedCommentTiming::new(700,500,1000,500), PedCommentTiming::new(5000,2000,15000,3000),
    PedCommentTiming::new(5000,2000,15000,3000), PedCommentTiming::new(5000,2000,15000,3000),
    PedCommentTiming::new(6000,6000,6000,6000), PedCommentTiming::new(1000,1000,2000,2000),
    PedCommentTiming::new(1000,500,2000,1500), PedCommentTiming::new(1000,500,2000,1500),
    PedCommentTiming::new(800,200,1000,500), PedCommentTiming::new(800,200,1000,500),
    PedCommentTiming::new(800,400,2000,1000), PedCommentTiming::new(800,400,2000,1000),
    PedCommentTiming::new(400,300,2000,1000), PedCommentTiming::new(2000,1000,2500,1500),
    PedCommentTiming::new(200,200,200,200), PedCommentTiming::new(6000,3000,5000,6000),
    PedCommentTiming::new(6000,3000,9000,5000), PedCommentTiming::new(6000,3000,9000,5000),
    PedCommentTiming::new(6000,3000,9000,5000), PedCommentTiming::new(400,300,4000,1000),
    PedCommentTiming::new(400,300,4000,1000), PedCommentTiming::new(400,300,4000,1000),
    PedCommentTiming::new(1000,500,3000,1000), PedCommentTiming::new(1000,500,1000,1000),
    PedCommentTiming::new(3000,2000,3000,2000), PedCommentTiming::new(1000,500,3000,6000),
    PedCommentTiming::new(1000,500,2000,4000), PedCommentTiming::new(1000,500,2000,5000),
    PedCommentTiming::new(1000,500,3000,2000), PedCommentTiming::new(1600,1000,2000,2000),
    PedCommentTiming::new(3000,2000,5000,3000), PedCommentTiming::new(1000,1000,1000,1000),
    PedCommentTiming::new(1000,1000,5000,5000),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PedTalkFamily {
    Player,
    Cop,
    Swat,
    FBI,
    Army,
    Medic,
    Fireman,
    BusinessMaleOld,
    BusinessMaleYoung,
    Mafia,
    Triad,
    Diablo,
    Yakuza,
    Yardie,
    Columbian,
    Hood,
    BlackCriminal,
    WhiteCriminal,
    CasualMaleOld,
    CasualMaleYoung,
    BlackCasualFemale,
    WhiteCasualFemale,
    FemaleNo3,
    WhiteBusinessFemale,
    BlackFatFemale,
    WhiteFatMale,
    BlackFatMale,
    WhiteFatFemale,
    BlackFemaleProstitute,
    WhiteFemaleProstitute,
    BlackProjectMale,
    BlackProjectFemaleOld,
    BlackProjectFemaleYoung,
    ChinatownMaleOld,
    ChinatownMaleYoung,
    ChinatownFemaleOld,
    ChinatownFemaleYoung,
    LittleItalyMale,
    LittleItalyFemaleOld,
    LittleItalyFemaleYoung,
    WhiteDockerMale,
    BlackDockerMale,
    ScumMale,
    ScumFemale,
    WhiteWorkerMale,
    BlackWorkerMale,
    BlackBusinessFemale,
    SupermodelMale,
    SupermodelFemale,
    StewardMale,
    StewardFemale,
    FanMale,
    FanFemale,
    HospitalMale,
    HospitalFemale,
    WhiteConstructionWorker,
    BlackConstructionWorker,
    ShopperFemale,
    StudentMale,
    StudentFemale,
    SpecialCharacter,
    EightBall,
    Salvatore,
    Misty,
    OldJap,
    Catalina,
    Bomber,
    SecurityGuard,
    Chunky,
    AsianTaxiDriver,
    Pimp,
    NormalMale,
    GenericMale,
    GenericFemale,
}

pub const PED_TALK_FAMILY_COUNT: usize = 74;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct PedTalkState {
    pub last_sfx: [u32; PED_TALK_FAMILY_COUNT],
    pub next_cry_time_ms: u32,
}

impl Default for PedTalkState {
    fn default() -> Self {
        Self {
            last_sfx: [NO_SAMPLE_U32; PED_TALK_FAMILY_COUNT],
            next_cry_time_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PedCommentContext<'a> {
    pub audio_entity: i32,
    pub position: CVector,
    pub distance_squared: f32,
    pub distance_calculated: bool,
    pub model_index: Option<usize>,
    pub is_player: bool,
    pub special_model_name: Option<&'a str>,
    pub line_of_sight_clear: bool,
    pub pursuit_blocked: bool,
    pub time_ms: u32,
}

/// Bevy backend for re3 `CPed::ServiceTalking`: translate the CPed queued
/// phrase into the audio manager request while keeping the authored CPed
/// talking timestamps/flags on the ped itself.
pub(crate) fn service_single_ped_talk(
    audio: &mut cAudioManager,
    talk_state: &mut PedTalkState,
    ped: &mut CPed, vitality: &crate::peds::PedVitality::PedVitality,
    model_index: Option<usize>,
    uid: i32,
    camera_position: CVector,
    time_ms: u32,
    is_player: bool,
    general: &mut crate::core::General::CGeneral,
    special_model_name: Option<&str>,
    loader: &crate::core::FileLoader::CFileLoader,
) {
    if ped.ped_state == crate::peds::Ped::PedState::PED_DEAD
        && ped.queued_sound != SOUND_PED_DEATH as i16 {
        return;
    }
    if ped.flags.body_part_just_came_off && vitality.body_part_bleeding == 2 { return; }
    if special_model_name.is_some_and(|name| name.eq_ignore_ascii_case("bomber")) {
        ped.queued_sound = SOUND_PED_BOMBER as i16;
    } else if ped.ped_state == crate::peds::Ped::PedState::PED_ON_FIRE {
        ped.queued_sound = SOUND_PED_BURNING as i16;
    }
    if ped.queued_sound == crate::peds::Ped::SOUND_NO_SOUND {
        return;
    }
    if ped.queued_sound == SOUND_PED_DEATH as i16 {
        ped.sound_start = time_ms.wrapping_sub(1);
    }
    if time_ms <= ped.sound_start { return; }
    let timing = (ped.queued_sound as u16).checked_sub(SOUND_PED_DEATH as u16)
        .and_then(|index| PED_COMMENT_TIMING.get(index as usize));
    let welcome = matches!(ped.queued_sound,
        value if value == SOUND_AMMUNATION_WELCOME_1 as i16
            || value == SOUND_AMMUNATION_WELCOME_2 as i16
            || value == SOUND_AMMUNATION_WELCOME_3 as i16);
    if timing.is_none() && !welcome {
        bevy::log::warn_once!(target: "re3bevy::audio", "queued ped phrase has no native timing entry");
        ped.queued_sound = crate::peds::Ped::SOUND_NO_SOUND;
        ped.flags.is_talking = false;
        return;
    }
    if ped.physical.audio_entity_id < 0 {
        ped.physical.audio_entity_id = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(uid));
        if ped.physical.audio_entity_id >= 0 {
            audio.SetEntityStatus(ped.physical.audio_entity_id, true);
        }
    }
    if ped.physical.audio_entity_id < 0 {
        bevy::log::warn_once!(target: "re3bevy::audio", "ped speech waiting for audio entity capacity");
        return;
    }
    let position = ped.physical.entity.get_position();
    let dx = camera_position.x - position.x;
    let dy = camera_position.y - position.y;
    let dz = camera_position.z - position.z;
    let sound = ped.queued_sound as u16;
    let ctx = PedCommentContext {
        audio_entity: ped.physical.audio_entity_id,
        position,
        distance_squared: dx * dx + dy * dy + dz * dz,
        distance_calculated: false,
        model_index,
        is_player,
        special_model_name,
        line_of_sight_clear: loader.world.process_line_of_sight_building_contact(
            [camera_position.x, camera_position.y, camera_position.z],
            [position.x, position.y, position.z], &loader.model_info).is_none(),
        pursuit_blocked: false,
        time_ms,
    };
    setup_ped_comment(audio, talk_state, &ctx, sound);
    ped.last_sound_start = time_ms;
    if let Some(timing) = timing {
        ped.sound_start = time_ms.wrapping_add(timing.playback_delay).wrapping_add(
            general.get_random_number_in_range_i32(0, timing.playback_random_delay) as u32);
    }
    ped.last_queued_sound = ped.queued_sound;
    ped.queued_sound = crate::peds::Ped::SOUND_NO_SOUND;
    ped.flags.is_talking = false;
}

pub fn setup_ped_comment(
    audio: &mut cAudioManager,
    state: &mut PedTalkState,
    ctx: &PedCommentContext<'_>,
    sound: u16,
) -> bool {
    let (sample, max_dist) = match resolve_setup_sample(audio, state, ctx, sound) {
        Some(v) => v,
        None => return false,
    };
    if ctx.distance_squared >= max_dist * max_dist {
        return false;
    }
    let distance = if ctx.distance_calculated {
        ctx.distance_squared
    } else {
        ctx.distance_squared.sqrt()
    };
    let emitting = match sound {
        x if x == SOUND_AMMUNATION_WELCOME_1 as u16
            || x == SOUND_AMMUNATION_WELCOME_2 as u16
            || x == SOUND_AMMUNATION_WELCOME_3 as u16 =>
        {
            PED_COMMENT_VOLUME
        }
        _ if ctx.line_of_sight_clear => PED_COMMENT_VOLUME,
        _ => PED_COMMENT_VOLUME_BEHIND_WALL,
    };
    let volume = audio.ComputeVolume(emitting, max_dist, distance);
    if volume == 0 {
        return false;
    }
    audio.m_sPedComments.Add(tPedComment {
        m_nSampleIndex: sample,
        m_nEntityIndex: ctx.audio_entity,
        m_vecPos: ctx.position,
        m_fDistance: distance,
        m_nVolume: volume,
        m_nLoadingTimeout: 10,
        m_nEmittingVolume: emitting,
    });
    true
}

fn resolve_setup_sample(
    audio: &cAudioManager,
    state: &mut PedTalkState,
    ctx: &PedCommentContext<'_>,
    sound: u16,
) -> Option<(u32, f32)> {
    match sound {
        x if x == SOUND_AMMUNATION_WELCOME_1 as u16 => {
            return Some((SFX_AMMU_D as u32, PED_COMMENT_MAX_DIST));
        }
        x if x == SOUND_AMMUNATION_WELCOME_2 as u16 => {
            return Some((SFX_AMMU_E as u32, PED_COMMENT_MAX_DIST));
        }
        x if x == SOUND_AMMUNATION_WELCOME_3 as u16 => {
            return Some((SFX_AMMU_F as u32, PED_COMMENT_MAX_DIST));
        }
        _ => {}
    }
    if ctx.model_index.is_none() {
        let table = (ctx.audio_entity.max(0) as usize) & 3;
        let sample = match sound {
            x if x == SOUND_PED_HELI_PLAYER_FOUND as u16 => {
                SFX_POLICE_HELI_1 as u32 + (audio.m_anRandomTable[table].unsigned_abs() % 29)
            }
            x if x == SOUND_PED_BODYCAST_HIT as u16 => {
                if ctx.time_ms <= state.next_cry_time_ms {
                    return None;
                }
                state.next_cry_time_ms = ctx.time_ms.wrapping_add(500);
                SFX_PLASTER_BLOKE_1 as u32 + (audio.m_anRandomTable[table].unsigned_abs() % 4)
            }
            x if x == SOUND_INJURED_PED_MALE_OUCH as u16
                || x == SOUND_INJURED_PED_MALE_PRISON as u16 =>
            {
                SFX_GENERIC_MALE_GRUNT_1 as u32 + (audio.m_anRandomTable[table].unsigned_abs() % 15)
            }
            x if x == SOUND_INJURED_PED_FEMALE as u16 => {
                SFX_GENERIC_FEMALE_GRUNT_1 as u32
                    + (audio.m_anRandomTable[table].unsigned_abs() % 11)
            }
            _ => return None,
        };
        let max_dist = if sound == SOUND_PED_HELI_PLAYER_FOUND as u16 {
            PED_COMMENT_POLICE_HELI_MAX_DIST
        } else {
            PED_COMMENT_MAX_DIST
        };
        return Some((sample, max_dist));
    }
    resolve_ped_comment_sfx(audio, state, ctx, sound).map(|sample| (sample, PED_COMMENT_MAX_DIST))
}

pub fn resolve_ped_comment_sfx(
    audio: &cAudioManager,
    state: &mut PedTalkState,
    ctx: &PedCommentContext<'_>,
    sound: u16,
) -> Option<u32> {
    let family = ped_talk_family(ctx)?;
    resolve_family(audio, state, ctx, family, sound)
}

pub fn ped_talk_family(ctx: &PedCommentContext<'_>) -> Option<PedTalkFamily> {
    if ctx.is_player {
        return Some(PedTalkFamily::Player);
    }
    let model = ctx.model_index?;
    Some(match model {
        MI_COP => PedTalkFamily::Cop,
        MI_SWAT => PedTalkFamily::Swat,
        MI_FBI => PedTalkFamily::FBI,
        MI_ARMY => PedTalkFamily::Army,
        MI_MEDIC => PedTalkFamily::Medic,
        MI_FIREMAN => PedTalkFamily::Fireman,
        MI_MALE01 => PedTalkFamily::NormalMale,
        MI_TAXI_D => PedTalkFamily::AsianTaxiDriver,
        MI_PIMP => PedTalkFamily::Pimp,
        MI_GANG01 => PedTalkFamily::Mafia,
        MI_GANG02 => PedTalkFamily::Mafia,
        MI_GANG03 => PedTalkFamily::Triad,
        MI_GANG04 => PedTalkFamily::Triad,
        MI_GANG05 => PedTalkFamily::Diablo,
        MI_GANG06 => PedTalkFamily::Diablo,
        MI_GANG07 => PedTalkFamily::Yakuza,
        MI_GANG08 => PedTalkFamily::Yakuza,
        MI_GANG09 => PedTalkFamily::Yardie,
        MI_GANG10 => PedTalkFamily::Yardie,
        MI_GANG11 => PedTalkFamily::Columbian,
        MI_GANG12 => PedTalkFamily::Columbian,
        MI_GANG13 => PedTalkFamily::Hood,
        MI_GANG14 => PedTalkFamily::Hood,
        MI_CRIMINAL01 => PedTalkFamily::BlackCriminal,
        MI_CRIMINAL02 => PedTalkFamily::WhiteCriminal,
        MI_SPECIAL01 => PedTalkFamily::SpecialCharacter,
        MI_SPECIAL02 => PedTalkFamily::SpecialCharacter,
        MI_SPECIAL03 => PedTalkFamily::SpecialCharacter,
        MI_SPECIAL04 => PedTalkFamily::SpecialCharacter,
        MI_MALE02 => PedTalkFamily::CasualMaleOld,
        MI_MALE03 => PedTalkFamily::BlackProjectMale,
        MI_FATMALE01 => PedTalkFamily::WhiteFatMale,
        MI_FATMALE02 => PedTalkFamily::BlackFatMale,
        MI_FEMALE01 => PedTalkFamily::BlackCasualFemale,
        MI_FEMALE02 => PedTalkFamily::WhiteCasualFemale,
        MI_FEMALE03 => PedTalkFamily::FemaleNo3,
        MI_FATFEMALE01 => PedTalkFamily::BlackFatFemale,
        MI_FATFEMALE02 => PedTalkFamily::WhiteFatFemale,
        MI_PROSTITUTE => PedTalkFamily::BlackFemaleProstitute,
        MI_PROSTITUTE2 => PedTalkFamily::WhiteFemaleProstitute,
        MI_P_MAN1 => PedTalkFamily::BlackProjectMale,
        MI_P_MAN2 => PedTalkFamily::BlackProjectMale,
        MI_P_WOM1 => PedTalkFamily::BlackProjectFemaleOld,
        MI_P_WOM2 => PedTalkFamily::BlackProjectFemaleYoung,
        MI_CT_MAN1 => PedTalkFamily::ChinatownMaleOld,
        MI_CT_MAN2 => PedTalkFamily::ChinatownMaleYoung,
        MI_CT_WOM1 => PedTalkFamily::ChinatownFemaleOld,
        MI_CT_WOM2 => PedTalkFamily::ChinatownFemaleYoung,
        MI_LI_MAN1 => PedTalkFamily::LittleItalyMale,
        MI_LI_MAN2 => PedTalkFamily::LittleItalyMale,
        MI_LI_WOM1 => PedTalkFamily::LittleItalyFemaleOld,
        MI_LI_WOM2 => PedTalkFamily::LittleItalyFemaleYoung,
        MI_DOCKER1 => PedTalkFamily::WhiteDockerMale,
        MI_DOCKER2 => PedTalkFamily::BlackDockerMale,
        MI_SCUM_MAN => PedTalkFamily::ScumMale,
        MI_SCUM_WOM => PedTalkFamily::ScumFemale,
        MI_WORKER1 => PedTalkFamily::WhiteWorkerMale,
        MI_WORKER2 => PedTalkFamily::BlackWorkerMale,
        MI_B_MAN1 => PedTalkFamily::BusinessMaleYoung,
        MI_B_MAN2 => PedTalkFamily::BusinessMaleOld,
        MI_B_MAN3 => PedTalkFamily::BusinessMaleYoung,
        MI_B_WOM1 => PedTalkFamily::WhiteBusinessFemale,
        MI_B_WOM2 => PedTalkFamily::WhiteBusinessFemale,
        MI_B_WOM3 => PedTalkFamily::BlackBusinessFemale,
        MI_MOD_MAN => PedTalkFamily::SupermodelMale,
        MI_MOD_WOM => PedTalkFamily::SupermodelFemale,
        MI_ST_MAN => PedTalkFamily::StewardMale,
        MI_ST_WOM => PedTalkFamily::StewardFemale,
        MI_FAN_MAN1 => PedTalkFamily::FanMale,
        MI_FAN_MAN2 => PedTalkFamily::FanMale,
        MI_FAN_WOM => PedTalkFamily::FanFemale,
        MI_HOS_MAN => PedTalkFamily::HospitalMale,
        MI_HOS_WOM => PedTalkFamily::HospitalFemale,
        MI_CONST1 => PedTalkFamily::WhiteConstructionWorker,
        MI_CONST2 => PedTalkFamily::BlackConstructionWorker,
        MI_SHOPPER1 => PedTalkFamily::ShopperFemale,
        MI_SHOPPER2 => PedTalkFamily::ShopperFemale,
        MI_SHOPPER3 => PedTalkFamily::ShopperFemale,
        MI_STUD_MAN => PedTalkFamily::StudentMale,
        MI_STUD_WOM => PedTalkFamily::StudentFemale,
        MI_CAS_MAN => PedTalkFamily::CasualMaleYoung,
        MI_CAS_WOM => PedTalkFamily::WhiteCasualFemale,
        _ => PedTalkFamily::GenericMale,
    })
}

fn special_character_family(name: Option<&str>) -> Option<PedTalkFamily> {
    let name = name?.to_ascii_lowercase();
    Some(match name.as_str() {
        "eight" | "eight2" => PedTalkFamily::EightBall,
        "frankie" => PedTalkFamily::Salvatore,
        "misty" => PedTalkFamily::Misty,
        "ojg" | "ojg_p" => PedTalkFamily::OldJap,
        "cat" => PedTalkFamily::Catalina,
        "bomber" => PedTalkFamily::Bomber,
        "s_guard" => PedTalkFamily::SecurityGuard,
        "chunky" => PedTalkFamily::Chunky,
        "asuka" | "maria" => PedTalkFamily::GenericFemale,
        _ => PedTalkFamily::GenericMale,
    })
}

fn resolve_family(
    audio: &cAudioManager,
    state: &mut PedTalkState,
    ctx: &PedCommentContext<'_>,
    family: PedTalkFamily,
    sound: u16,
) -> Option<u32> {
    if matches!(sound, x if x == SOUND_PED_PURSUIT_COP as u16 || x == SOUND_PED_PURSUIT_SWAT as u16 || x == SOUND_PED_PURSUIT_FBI as u16 || x == SOUND_PED_PURSUIT_ARMY as u16)
        && ctx.pursuit_blocked
    {
        return None;
    }
    let family = if family == PedTalkFamily::SpecialCharacter {
        special_character_family(ctx.special_model_name).unwrap_or(PedTalkFamily::GenericMale)
    } else {
        family
    };
    let idx = family as usize;
    let mut sfx = match family {
        PedTalkFamily::Player => match sound {
            x if x == SOUND_PED_DAMAGE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CLAUDE_HIGH_DAMAGE_GRUNT_1 as u32,
                11,
            )),
            x if x == SOUND_PED_HIT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CLAUDE_LOW_DAMAGE_GRUNT_1 as u32,
                10,
            )),
            x if x == SOUND_PED_LAND as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CLAUDE_HIT_GROUND_GRUNT_1 as u32,
                6,
            )),
            _ => None,
        },
        PedTalkFamily::Cop => match sound {
            x if x == SOUND_PED_ARREST_COP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COP_VOICE_1_ARREST_1 as u32,
                6,
            )),
            x if x == SOUND_PED_PURSUIT_COP as u16 => None,
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Swat => match sound {
            x if x == SOUND_PED_ARREST_SWAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SWAT_VOICE_1_CHASE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_PURSUIT_SWAT as u16 => None,
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::FBI => match sound {
            x if x == SOUND_PED_ARREST_FBI as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FBI_VOICE_1_CHASE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_PURSUIT_FBI as u16 => None,
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Army => match sound {
            x if x == SOUND_PED_PURSUIT_ARMY as u16 => None,
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Medic => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MEDIC_VOICE_1_GUN_PANIC_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MEDIC_VOICE_1_CARJACKED_1 as u32,
                5,
            )),
            x if x == SOUND_PED_HEALING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MEDIC_VOICE_1_AT_VICTIM_1 as u32,
                12,
            )),
            x if x == SOUND_PED_LEAVE_VEHICLE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MEDIC_VOICE_1_GET_OUT_VAN_CHAT_1 as u32,
                9,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MEDIC_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Fireman => match sound {
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BusinessMaleOld => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_MRUN_FROM_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_OLD_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BusinessMaleYoung => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BUSINESS_MALE_YOUNG_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Mafia => match sound {
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MAFIA_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MAFIA_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MAFIA_MALE_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MAFIA_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MAFIA_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MAFIA_MALE_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MAFIA_MALE_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Triad => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_GUN_COOL_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_TRIAD_MALE_VOICE_1_CHAT_1 as u32,
                8,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Diablo => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_GUN_COOL_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_EYING_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_DIABLO_MALE_VOICE_1_CHAT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_HANDS_COWER as u16 => {
                resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound)
            }
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Yakuza => match sound {
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YAKUZA_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YAKUZA_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YAKUZA_MALE_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YAKUZA_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YAKUZA_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YAKUZA_MALE_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Yardie => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_EYING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_YARDIE_MALE_VOICE_1_CHAT_1 as u32,
                8,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Columbian => match sound {
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COLUMBIAN_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COLUMBIAN_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COLUMBIAN_MALE_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COLUMBIAN_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COLUMBIAN_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COLUMBIAN_MALE_VOICE_1_EYING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_COLUMBIAN_MALE_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Hood => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_GUN_COOL_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_CARJACKING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_EYING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOOD_MALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackCriminal => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CRIMINAL_VOICE_1_GUN_COOL_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CRIMINAL_VOICE_1_MUGGING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_MUGGING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CRIMINAL_VOICE_1_MUGGING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CRIMINAL_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CRIMINAL_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CRIMINAL_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::WhiteCriminal => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CRIMINAL_VOICE_1_GUN_COOL_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CRIMINAL_VOICE_1_MUGGING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_MUGGING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CRIMINAL_VOICE_1_MUGGING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CRIMINAL_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CRIMINAL_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CRIMINAL_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::CasualMaleOld => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CASUAL_MALE_OLD_VOICE_1_CARJACKED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CASUAL_MALE_OLD_VOICE_1_MUGGED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CASUAL_MALE_OLD_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CASUAL_MALE_OLD_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CASUAL_MALE_OLD_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CASUAL_MALE_OLD_VOICE_1_EYING_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CASUAL_MALE_OLD_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::CasualMaleYoung => match sound {
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackCasualFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_GUN_PANIC_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_MUGGED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_1_VOICE_1_CHAT_1 as u32,
                8,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::WhiteCasualFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_DODGE_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_DODGE_1 as u32,
                3,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                8,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_SHOCKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_CASUAL_FEMALE_VOICE_1_CHAT_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::FemaleNo3 => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_GUN_PANIC_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_CARJACKED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_MUGGED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FEMALE_3_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::WhiteBusinessFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_BUSINESS_FEMALE_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::BlackFatFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_FEMALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_FEMALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_FEMALE_VOICE_1_SHOCKED_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_FEMALE_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::WhiteFatMale => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_MALE_VOICE_1_CARJACKED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_MALE_VOICE_1_MUGGED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_MALE_VOICE_1_DODGE_1 as u32,
                9,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                9,
            )),
            x if x == SOUND_PED_WAIT_DOUBLEBACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_MALE_VOICE_1_LOST_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_MALE_VOICE_1_CHAT_1 as u32,
                9,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackFatMale => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_MALE_VOICE_1_CARJACKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_MALE_VOICE_1_MUGGED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_MALE_VOICE_1_DODGE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_WAIT_DOUBLEBACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_MALE_VOICE_1_LOST_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_FAT_MALE_VOICE_1_CHAT_1 as u32,
                8,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::WhiteFatFemale => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_FEMALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_FEMALE_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                8,
            )),
            x if x == SOUND_PED_WAIT_DOUBLEBACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_FEMALE_VOICE_1_LOST_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_FEMALE_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_FAT_FEMALE_VOICE_1_CHAT_1 as u32,
                8,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::BlackFemaleProstitute => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROSTITUTE_VOICE_1_GUN_COOL_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROSTITUTE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROSTITUTE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROSTITUTE_VOICE_1_DODGE_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROSTITUTE_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_SOLICIT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROSTITUTE_VOICE_1_SOLICIT_1 as u32,
                8,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROSTITUTE_VOICE_1_CHAT_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::WhiteFemaleProstitute => match sound {
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_PROSTITUTE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_PROSTITUTE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_PROSTITUTE_VOICE_1_DODGE_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_PROSTITUTE_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_SOLICIT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_PROSTITUTE_VOICE_1_SOLICIT_1 as u32,
                8,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_PROSTITUTE_VOICE_1_CHAT_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::BlackProjectMale => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_GUN_COOL_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_MALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackProjectFemaleOld => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_OLD_VOICE_1_CARJACKED_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_OLD_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_OLD_VOICE_1_DODGE_1 as u32,
                10,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_OLD_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_OLD_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_OLD_VOICE_1_SHOCKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_OLD_VOICE_1_CHAT_1 as u32,
                10,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::BlackProjectFemaleYoung => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_YOUNG_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_YOUNG_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_YOUNG_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_YOUNG_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_YOUNG_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_YOUNG_VOICE_1_SHOCKED_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_PROJECT_FEMALE_YOUNG_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::ChinatownMaleOld => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_OLD_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_OLD_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_OLD_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_OLD_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_OLD_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_OLD_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_OLD_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::ChinatownMaleYoung => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_YOUNG_VOICE_1_GUN_PANIC_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_YOUNG_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_YOUNG_VOICE_1_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_YOUNG_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_YOUNG_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_YOUNG_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_MALE_YOUNG_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::ChinatownFemaleOld => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_OLD_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_OLD_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_OLD_FEMALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_OLD_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_OLD_FEMALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_OLD_FEMALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::ChinatownFemaleYoung => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_YOUNG_FEMALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_YOUNG_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_YOUNG_FEMALE_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_YOUNG_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_YOUNG_FEMALE_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHINATOWN_YOUNG_FEMALE_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::LittleItalyMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_MALE_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_MALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_MALE_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_MALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::LittleItalyFemaleOld => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_OLD_FEMALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_OLD_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_OLD_FEMALE_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_OLD_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_OLD_FEMALE_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_OLD_FEMALE_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::LittleItalyFemaleYoung => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_YOUNG_FEMALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_YOUNG_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_YOUNG_FEMALE_VOICE_1_DODGE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_YOUNG_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_YOUNG_FEMALE_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_LITTLE_ITALY_YOUNG_FEMALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::WhiteDockerMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_DOCKER_MALE_VOICE_1_GUN_PANIC_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_DOCKER_MALE_VOICE_1_FIGHT_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_DOCKER_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_DOCKER_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_DOCKER_MALE_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_DOCKER_MALE_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackDockerMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_DOCKER_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_DOCKER_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_DOCKER_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_DOCKER_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_DOCKER_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_DOCKER_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::ScumMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_GUN_PANIC_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_FIGHT_1 as u32,
                10,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_FIGHT_1 as u32,
                10,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_WAIT_DOUBLEBACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_LOST_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_EYING_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_MALE_VOICE_1_CHAT_1 as u32,
                9,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::ScumFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_FEMALE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_FEMALE_VOICE_1_DODGE_1 as u32,
                8,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SCUM_FEMALE_VOICE_1_CHAT_1 as u32,
                13,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::WhiteWorkerMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_WORKER_MALE_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_WORKER_MALE_VOICE_1_FIGHT_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_WORKER_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_WORKER_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_WORKER_MALE_VOICE_1_EYING_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_WORKER_MALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackWorkerMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_WORKER_MALE_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_WORKER_MALE_VOICE_1_FIGHT_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_WORKER_MALE_VOICE_1_DODGE_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_WORKER_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_WORKER_MALE_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_WORKER_MALE_VOICE_1_CHAT_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackBusinessFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_CARAJACKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_MUGGED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_RUN_FROM_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_BUSINESS_FEMALE_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::SupermodelMale => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_MALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_MALE_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_MALE_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_MALE_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_MALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::SupermodelFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_FEMALE_VOICE_1_MUGGED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_FEMALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_FEMALE_VOICE_1_SHOCKED_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MODEL_FEMALE_VOICE_1_CHAT_1 as u32,
                8,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::StewardMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_MALE_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_MALE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_MALE_VOICE_1_DODGE_1 as u32,
                3,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_MALE_VOICE_1_CHAT_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::StewardFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_FEMALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STEWARD_FEMALE_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::FanMale => match sound {
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_MALE_VOICE_1_FIGHT_1 as u32,
                3,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_MALE_VOICE_1_SHOCKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_MALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::FanFemale => match sound {
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_FEMALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_FEMALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_FEMALE_VOICE_1_SHOCKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_FOOTBALL_FEMALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::HospitalMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_MALE_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_MALE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_MALE_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::HospitalFemale => match sound {
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_FEMALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_HOSPITAL_FEMALE_VOICE_1_CHAT_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::WhiteConstructionWorker => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_MALE_CONSTRUCTION_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_MALE_CONSTRUCTION_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_MALE_CONSTRUCTION_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_MALE_CONSTRUCTION_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_MALE_CONSTRUCTION_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_MALE_CONSTRUCTION_VOICE_1_EYING_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_WHITE_MALE_CONSTRUCTION_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::BlackConstructionWorker => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CONSTRUCTION_MALE_VOICE_1_GUN_PANIC_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CONSTRUCTION_MALE_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CONSTRUCTION_MALE_VOICE_1_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CONSTRUCTION_MALE_VOICE_1_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CONSTRUCTION_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CONSTRUCTION_MALE_VOICE_1_EYING_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BLACK_CONSTRUCTION_MALE_VOICE_1_CHAT_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::ShopperFemale => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SHOPPER_VOICE_1_CARJACKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SHOPPER_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SHOPPER_VOICE_1_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SHOPPER_VOICE_1_DRIVER_ABUSE_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SHOPPER_VOICE_1_SHOCKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SHOPPER_VOICE_1_CHAT_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::StudentMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_MALE_VOICE_1_GUN_PANIC_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_MALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_MALE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_MALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_MALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_MALE_VOICE_1_SHOCKED_1 as u32,
                3,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_MALE_VOICE_1_CHAT_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::StudentFemale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_FEMALE_VOICE_1_GUN_PANIC_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_FEMALE_VOICE_1_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_FEMALE_VOICE_1_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_FEMALE_VOICE_1_DODGE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32,
                4,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_FEMALE_VOICE_1_SHOCKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_STUDENT_FEMALE_VOICE_1_CHAT_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::SpecialCharacter => match sound {
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::EightBall => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_8BALL_GUN_COOL_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_8BALL_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_8BALL_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_8BALL_DODGE_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Salvatore => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SALVATORE_GUN_COOL_1 as u32,
                4,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SALVATORE_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SALVATORE_FIGHT_1 as u32,
                6,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SALVATORE_DODGE_1 as u32,
                3,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Misty => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MISTY_GUN_COOL_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ROBBED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MISTY_MUGGED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MISTY_FIGHT_1 as u32,
                4,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MISTY_DODGE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_TAXI_CALL as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_MISTY_HERE_1 as u32,
                4,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::OldJap => match sound {
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Catalina => match sound {
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericFemale, sound),
        },
        PedTalkFamily::Bomber => match sound {
            x if x == SOUND_PED_BOMBER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_BOMBERMAN_1 as u32,
                7,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::SecurityGuard => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SECURITY_GUARD_VOICE_1_GUN_COOL_1 as u32,
                2,
            )),
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SECURITY_GUARD_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SECURITY_GUARD_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SECURITY_GUARD_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ATTACK as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SECURITY_GUARD_VOICE_1_FIGHT_1 as u32,
                2,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_SECURITY_GUARD_VOICE_1_DRIVER_ABUSE_1 as u32,
                12,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Chunky => match sound {
            x if x == SOUND_PED_DEATH as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHUNKY_RUN_1 as u32,
                5,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_CHUNKY_RUN_1 as u32,
                5,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::AsianTaxiDriver => match sound {
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_ASIAN_TAXI_DRIVER_VOICE_1_CARJACKED_1 as u32,
                7,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_ASIAN_TAXI_DRIVER_VOICE_1_DRIVER_ABUSE_1 as u32,
                6,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::Pimp => match sound {
            x if x == SOUND_PED_HANDS_UP as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_PIMP_GUN_COOL_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_PIMP_CARJACKED_1 as u32,
                4,
            )),
            x if x == SOUND_PED_DEFEND as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_PIMP_FIGHT_1 as u32,
                9,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_PIMP_DODGE_1 as u32,
                6,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_PIMP_DRIVER_ABUSE_1 as u32,
                5,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_PIMP_SHOCKED_1 as u32,
                2,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_PIMP_CHAT_1 as u32,
                17,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::NormalMale => match sound {
            x if x == SOUND_PED_HANDS_COWER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_GUN_PANIC_1 as u32,
                7,
            )),
            x if x == SOUND_PED_CAR_JACKED as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_CARJACKED_1 as u32,
                7,
            )),
            x if x == SOUND_PED_EVADE as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_DODGE_1 as u32,
                9,
            )),
            x if x == SOUND_PED_FLEE_RUN as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_RUN_FROM_FIGHT_1 as u32,
                5,
            )),
            x if x == SOUND_PED_ANNOYED_DRIVER as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_DRIVER_ABUSE_1 as u32,
                12,
            )),
            x if x == SOUND_PED_CHAT_SEXY as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_EYING_1 as u32,
                8,
            )),
            x if x == SOUND_PED_CHAT_EVENT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_SHOCKED_1 as u32,
                10,
            )),
            x if x == SOUND_PED_CHAT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_NORMAL_MALE_CHAT_1 as u32,
                25,
            )),
            _ => resolve_family(audio, state, ctx, PedTalkFamily::GenericMale, sound),
        },
        PedTalkFamily::GenericMale => match sound {
            x if x == SOUND_PED_DEATH as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_MALE_DEATH_1 as u32,
                8,
            )),
            x if x == SOUND_PED_BULLET_HIT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_MALE_GRUNT_1 as u32,
                15,
            )),
            x if x == SOUND_PED_DEFEND as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_MALE_GRUNT_1 as u32,
                15,
            )),
            x if x == SOUND_PED_BURNING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_MALE_FIRE_1 as u32,
                8,
            )),
            x if x == SOUND_PED_FLEE_SPRINT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_MALE_PANIC_1 as u32,
                6,
            )),
            _ => None,
        },
        PedTalkFamily::GenericFemale => match sound {
            x if x == SOUND_PED_DEATH as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_FEMALE_DEATH_1 as u32,
                10,
            )),
            x if x == SOUND_PED_BULLET_HIT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_FEMALE_GRUNT_1 as u32,
                11,
            )),
            x if x == SOUND_PED_DEFEND as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_FEMALE_GRUNT_1 as u32,
                11,
            )),
            x if x == SOUND_PED_BURNING as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_FEMALE_FIRE_1 as u32,
                9,
            )),
            x if x == SOUND_PED_FLEE_SPRINT as u16 => Some(get_phrase(
                audio,
                &mut state.last_sfx[idx],
                SFX_GENERIC_FEMALE_PANIC_1 as u32,
                8,
            )),
            _ => None,
        },
    }?;
    sfx = apply_family_variant(audio, ctx, family, sfx);
    Some(sfx)
}

fn get_phrase(audio: &cAudioManager, previous: &mut u32, sample: u32, max_offset: u32) -> u32 {
    let table = (audio.m_sQueueSample.m_nEntityIndex.max(0) as usize) & 3;
    let mut phrase = sample + audio.m_anRandomTable[table].unsigned_abs() % max_offset;
    if phrase == *previous {
        phrase += 1;
        if phrase >= sample + max_offset {
            phrase = sample;
        }
    }
    *previous = phrase;
    phrase
}

fn apply_family_variant(
    audio: &cAudioManager,
    ctx: &PedCommentContext<'_>,
    family: PedTalkFamily,
    mut sfx: u32,
) -> u32 {
    let entity = audio.m_sQueueSample.m_nEntityIndex.max(0) as u32;
    match family {
        PedTalkFamily::Cop => {
            sfx +=
                (SFX_COP_VOICE_2_ARREST_1 as u32 - SFX_COP_VOICE_1_ARREST_1 as u32) * (entity % 5)
        }
        PedTalkFamily::Swat => {
            sfx +=
                (SFX_SWAT_VOICE_2_CHASE_1 as u32 - SFX_SWAT_VOICE_1_CHASE_1 as u32) * (entity % 4)
        }
        PedTalkFamily::FBI => {
            sfx += (SFX_FBI_VOICE_2_CHASE_1 as u32 - SFX_FBI_VOICE_1_CHASE_1 as u32) * (entity % 3)
        }
        PedTalkFamily::Army => {
            sfx +=
                (SFX_ARMY_VOICE_2_CHASE_1 as u32 - SFX_ARMY_VOICE_1_CHASE_1 as u32) * (entity % 2)
        }
        PedTalkFamily::Medic => {
            sfx += (SFX_MEDIC_VOICE_2_GUN_PANIC_1 as u32 - SFX_MEDIC_VOICE_1_GUN_PANIC_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::BusinessMaleYoung if ctx.model_index == Some(MI_B_MAN3) => {
            sfx += SFX_BUSINESS_MALE_YOUNG_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_BUSINESS_MALE_YOUNG_VOICE_1_DRIVER_ABUSE_1 as u32
        }
        PedTalkFamily::Mafia => {
            sfx += (SFX_MAFIA_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_MAFIA_MALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 3)
        }
        PedTalkFamily::Diablo => {
            sfx += (SFX_DIABLO_MALE_VOICE_2_CHAT_1 as u32 - SFX_DIABLO_MALE_VOICE_1_CHAT_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::Yakuza => {
            sfx += (SFX_YAKUZA_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_YAKUZA_MALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::Yardie => {
            sfx += (SFX_YARDIE_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_YARDIE_MALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::Columbian => {
            sfx += (SFX_COLUMBIAN_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_COLUMBIAN_MALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::Hood => {
            sfx += (SFX_HOOD_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_HOOD_MALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::WhiteBusinessFemale if ctx.model_index == Some(MI_B_WOM2) => {
            sfx += SFX_WHITE_BUSINESS_FEMALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_WHITE_BUSINESS_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32
        }
        PedTalkFamily::BlackFemaleProstitute => {
            sfx += (SFX_BLACK_PROSTITUTE_VOICE_2_CHAT_1 as u32
                - SFX_BLACK_PROSTITUTE_VOICE_1_CHAT_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::WhiteFemaleProstitute => {
            sfx += (SFX_WHITE_PROSTITUTE_VOICE_2_CHAT_1 as u32
                - SFX_WHITE_PROSTITUTE_VOICE_1_CHAT_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::BlackProjectMale if ctx.model_index == Some(MI_P_MAN2) => {
            sfx += SFX_BLACK_PROJECT_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_BLACK_PROJECT_MALE_VOICE_1_DRIVER_ABUSE_1 as u32
        }
        PedTalkFamily::LittleItalyMale => {
            sfx += (SFX_LITTLE_ITALY_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_LITTLE_ITALY_MALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::StewardFemale => {
            sfx += (SFX_STEWARD_FEMALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_STEWARD_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::FanMale if ctx.model_index == Some(MI_FAN_MAN2) => {
            sfx += SFX_FOOTBALL_MALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_FOOTBALL_MALE_VOICE_1_DRIVER_ABUSE_1 as u32
        }
        PedTalkFamily::FanFemale => {
            sfx += (SFX_FOOTBALL_FEMALE_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_FOOTBALL_FEMALE_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::AsianTaxiDriver => {
            sfx += (SFX_ASIAN_TAXI_DRIVER_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_ASIAN_TAXI_DRIVER_VOICE_1_DRIVER_ABUSE_1 as u32)
                * (entity % 2)
        }
        PedTalkFamily::ShopperFemale if ctx.model_index == Some(MI_SHOPPER2) => {
            sfx += SFX_SHOPPER_VOICE_2_DRIVER_ABUSE_1 as u32
                - SFX_SHOPPER_VOICE_1_DRIVER_ABUSE_1 as u32
        }
        PedTalkFamily::ShopperFemale if ctx.model_index == Some(MI_SHOPPER3) => {
            sfx += SFX_SHOPPER_VOICE_3_DRIVER_ABUSE_1 as u32
                - SFX_SHOPPER_VOICE_1_DRIVER_ABUSE_1 as u32
        }
        _ => {}
    }
    sfx
}

// CPed::ServiceTalking over the live ECS ped and player queries.

pub(crate) fn service_ped_talk(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut talk_state: ResMut<PedTalkState>,
    mut actors: AudioPedSpeech,
    camera: Option<Res<CCamera>>,
    timer: Res<CTimer>,
    mut general: ResMut<crate::core::General::CGeneral>,
    loader: Res<crate::core::FileLoader::CFileLoader>,
    mut diagnostics: ResMut<AudioActorDiagnostics>,
) {
    if !diagnostics.audio_manager_available(dmaudio.is_some()) { return; }
    let Some(dmaudio) = dmaudio.as_deref_mut() else { return; };
    let Some(camera_position) = diagnostics.listener_position(camera.as_deref(), Some(&loader)) else { return; };
    let time_ms = timer.time_in_milliseconds();
    actors.for_each_script_ped(|handle, ped, vitality| {
        let model_index = ped.physical.entity.model_index.max(0) as usize;
        service_single_ped_talk(&mut dmaudio.AudioManager, &mut talk_state,
            ped, vitality, Some(model_index), handle, camera_position, time_ms,
            false, &mut general, loader.model_info.get_model_info(model_index)
                .map(|model| model.base().name.as_str()), &loader);
        if ped.physical.audio_entity_id >= 0 || ped.queued_sound != crate::peds::Ped::SOUND_NO_SOUND {
            diagnostics.observe_actor_audio_id(ped.physical.audio_entity_id, dmaudio.AudioManager.m_asAudioEntities.len());
        }
    });
    actors.for_each_player(|handle, ped, vitality| {
        let model_index = ped.physical.entity.model_index.max(0) as usize;
        service_single_ped_talk(&mut dmaudio.AudioManager, &mut talk_state,
            ped, vitality, Some(model_index), handle, camera_position, time_ms,
            true, &mut general, loader.model_info.get_model_info(model_index)
                .map(|model| model.base().name.as_str()), &loader);
        // No queued phrase means an unallocated audio identity is expected.
        if ped.physical.audio_entity_id >= 0 || ped.queued_sound != crate::peds::Ped::SOUND_NO_SOUND {
            diagnostics.observe_actor_audio_id(ped.physical.audio_entity_id, dmaudio.AudioManager.m_asAudioEntities.len());
        }
    });
}
