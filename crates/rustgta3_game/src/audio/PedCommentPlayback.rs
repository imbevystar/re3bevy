//! cPedComments::Process and the native PC sample-manager load boundary.
use super::{
    AudioLogic::{PED_COMMENT_MAX_DIST, PED_COMMENT_POLICE_HELI_MAX_DIST},
    AudioManager::{NUM_PED_COMMENTS_SLOTS, cAudioManager, tSound},
    AudioSamples::{SFX_AMMU_D, SFX_AMMU_F, SFX_POLICE_HELI_1, SFX_POLICE_HELI_29},
    sampman::{PedCommentLoadContext, SFX_BANK_PED_COMMENTS, cSampleManager},
};
use bevy::prelude::*;

#[derive(Default, Resource)]
pub struct PedCommentPlaybackDiagnostics {
    pub missing_backend_frames: u64,
    pub missing_original_samples: u64,
    pub missing_timer_frames: u64,
    pub admitted_phrases: u64,
    pub expired_phrases: u64,
    pub code_pause_load_wait_frames: u64,
    pub cutscene_music_load_wait_frames: u64,
    pub completion_music_load_wait_frames: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{
        AudioManager::tPedComment,
        sampman::{MAX_PEDSFX, SampleMetadata},
    };
    use rustgta3_core::core::Coordinates::CVector;

    fn load_context() -> PedCommentLoadContext {
        PedCommentLoadContext {
            code_paused: false,
            cutscene_music: false,
            game_completed_music: false,
        }
    }

    #[test]
    fn native_comment_queue_consumes_only_loudest_and_rotates_pending_timeout() {
        let mut audio = cAudioManager::default();
        audio.Initialise();
        let entity = audio.CreateEntity(crate::audio::audio_enums::AUDIOTYPE_PHYSICAL, Some(1));
        audio.SetEntityStatus(entity, true);
        let mut samples = cSampleManager::default();
        let descriptor = SampleMetadata {
            offset: 0,
            size: 1000,
            frequency: 22050,
            ..Default::default()
        };
        let mut bytes = Vec::new();
        for _ in 0..2 {
            bytes.extend_from_slice(&descriptor.offset.to_le_bytes());
            bytes.extend_from_slice(&descriptor.size.to_le_bytes());
            bytes.extend_from_slice(&descriptor.frequency.to_le_bytes());
            bytes.extend_from_slice(&descriptor.loop_start.to_le_bytes());
            bytes.extend_from_slice(&descriptor.loop_end.to_le_bytes());
        }
        samples.LoadSampleMetadata(&bytes);
        let comment = tPedComment {
            m_nEntityIndex: entity,
            m_nSampleIndex: 0,
            m_nVolume: 50,
            m_nEmittingVolume: 75,
            m_nLoadingTimeout: 10,
            m_vecPos: CVector::new(10.0, 20.0, 30.0),
            m_fDistance: 12.0,
        };
        audio.m_sPedComments.Add(comment);
        audio.m_sPedComments.Add(tPedComment {
            m_nSampleIndex: 1,
            m_nVolume: 75,
            ..comment
        });
        let mut diagnostics = PedCommentPlaybackDiagnostics::default();
        process_ped_comments(
            &mut audio,
            PedCommentService {
                samples: &mut samples,
                load_context: load_context(),
                original_sample_available: &|_| true,
                slow_motion: false,
                diagnostics: &mut diagnostics,
            },
        );
        assert_eq!(diagnostics.admitted_phrases, 1);
        assert_eq!(audio.m_sQueueSample.m_nSampleIndex, 1);
        assert_eq!(audio.m_sQueueSample.m_vecPos, comment.m_vecPos);
        assert!((21300..=22800).contains(&audio.m_sQueueSample.m_nFrequency));
        assert_eq!(audio.m_sPedComments.m_nActiveQueue, 1);
        assert_eq!(audio.m_sPedComments.m_nPedCommentCount, [0, 1]);
        assert_eq!(
            audio.m_sPedComments.m_aPedCommentQueue[1][0].m_nLoadingTimeout,
            9
        );
        audio.m_bIsPaused = true;
        process_ped_comments(
            &mut audio,
            PedCommentService {
                samples: &mut samples,
                load_context: load_context(),
                original_sample_available: &|_| true,
                slow_motion: false,
                diagnostics: &mut diagnostics,
            },
        );
        assert_eq!(audio.m_sPedComments.m_nActiveQueue, 1);
        assert_eq!(
            audio.m_sPedComments.m_aPedCommentQueue[1][0].m_nLoadingTimeout,
            9
        );
    }

    #[test]
    fn sample_manager_remembers_last_three_with_native_seven_slot_wrap_and_load_gates() {
        let mut samples = cSampleManager::default();
        for sample in 0..MAX_PEDSFX as u32 + 2 {
            assert!(samples.load_ped_comment(sample, load_context(), true));
        }
        assert!(samples.is_ped_comment_loaded(8));
        assert!(samples.is_ped_comment_loaded(7));
        assert!(samples.is_ped_comment_loaded(6));
        assert!(!samples.is_ped_comment_loaded(5));
        assert!(!samples.load_ped_comment(
            9,
            PedCommentLoadContext {
                code_paused: true,
                ..load_context()
            },
            true
        ));
        assert!(!samples.load_ped_comment(
            9,
            PedCommentLoadContext {
                cutscene_music: true,
                ..load_context()
            },
            true
        ));
        assert!(!samples.load_ped_comment(
            9,
            PedCommentLoadContext {
                game_completed_music: true,
                ..load_context()
            },
            true
        ));
        assert!(!samples.load_ped_comment(9, load_context(), false));
        assert!(samples.is_ped_comment_loaded(8));
    }
}

pub(crate) struct PedCommentService<'a> {
    pub samples: &'a mut cSampleManager,
    pub load_context: PedCommentLoadContext,
    pub original_sample_available: &'a dyn Fn(u32) -> bool,
    pub slow_motion: bool,
    pub diagnostics: &'a mut PedCommentPlaybackDiagnostics,
}

pub(crate) fn process_ped_comments(audio: &mut cAudioManager, service: PedCommentService<'_>) {
    if audio.m_bIsPaused {
        return;
    }
    let queue = audio.m_sPedComments.m_nActiveQueue;
    if audio.m_sPedComments.m_nPedCommentCount[queue] > 0 {
        let index = audio.m_sPedComments.m_aPedCommentOrderList[queue][0];
        let comment = audio.m_sPedComments.m_aPedCommentQueue[queue][index];
        let sample = comment.m_nSampleIndex;
        let metadata = service
            .samples
            .sample_metadata()
            .get(sample as usize)
            .copied()
            .filter(|sample| sample.frequency > 0 && sample.size > 0);
        let pcm_available = (service.original_sample_available)(sample) && metadata.is_some();
        let loaded = pcm_available
            && (service.samples.is_ped_comment_loaded(sample)
                || service
                    .samples
                    .load_ped_comment(sample, service.load_context, pcm_available));
        if !pcm_available {
            service.diagnostics.missing_original_samples += 1;
            warn_once!(sample, "ped_comment_waiting_original_sdt_pcm");
        }
        if pcm_available && !loaded {
            let context = service.load_context;
            service.diagnostics.code_pause_load_wait_frames += u64::from(context.code_paused);
            service.diagnostics.cutscene_music_load_wait_frames +=
                u64::from(context.cutscene_music);
            service.diagnostics.completion_music_load_wait_frames +=
                u64::from(context.game_completed_music);
        }
        if loaded {
            let police_heli =
                (SFX_POLICE_HELI_1 as u32..=SFX_POLICE_HELI_29 as u32).contains(&sample);
            let ammunation = (SFX_AMMU_D as u32..=SFX_AMMU_F as u32).contains(&sample);
            let base = metadata
                .expect("validated original SDT descriptor")
                .frequency;
            let mut frequency = (i64::from(base) + i64::from(audio.RandomDisplacement(750))) as u32;
            if service.slow_motion {
                frequency >>= 1;
            }
            audio.m_sQueueSample = tSound {
                m_nEntityIndex: comment.m_nEntityIndex,
                m_nCounter: 0,
                m_nSampleIndex: sample,
                m_nBankIndex: SFX_BANK_PED_COMMENTS as u8,
                m_nPriority: 3,
                m_nFrequency: frequency,
                m_nVolume: comment.m_nVolume,
                m_nEmittingVolume: comment.m_nEmittingVolume,
                m_fDistance: comment.m_fDistance,
                m_nLoopCount: 1,
                m_nLoopStart: 0,
                m_nLoopEnd: -1,
                m_fSpeedMultiplier: 3.0,
                m_MaxDistance: if police_heli {
                    PED_COMMENT_POLICE_HELI_MAX_DIST
                } else {
                    PED_COMMENT_MAX_DIST
                },
                m_bStatic: true,
                m_vecPos: comment.m_vecPos,
                m_bReverb: !ammunation,
                m_bIs2D: false,
                ..Default::default()
            };
            audio.m_sPedComments.m_aPedCommentQueue[queue][index].m_nLoadingTimeout = -1;
            audio.AddSampleToRequestedQueue();
            service.diagnostics.admitted_phrases += 1;
        }
    }
    audio.m_sPedComments.m_nActiveQueue = 1 - queue;
    let count = audio.m_sPedComments.m_nPedCommentCount[queue];
    for ordered in 0..count {
        let index = audio.m_sPedComments.m_aPedCommentOrderList[queue][ordered];
        let mut comment = audio.m_sPedComments.m_aPedCommentQueue[queue][index];
        if comment.m_nLoadingTimeout > 0 {
            comment.m_nLoadingTimeout -= 1;
            audio.m_sPedComments.Add(comment);
        } else if comment.m_nLoadingTimeout == 0 {
            service.diagnostics.expired_phrases += 1;
        }
    }
    audio.m_sPedComments.m_aPedCommentOrderList[queue] =
        [NUM_PED_COMMENTS_SLOTS; NUM_PED_COMMENTS_SLOTS];
    audio.m_sPedComments.m_nPedCommentCount[queue] = 0;
}
