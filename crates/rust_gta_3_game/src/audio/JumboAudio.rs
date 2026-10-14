//! Native ProcessJumbo and its sample helpers, borrowing canonical plane paths.
use super::*;
use crate::vehicles::Plane::CPlaneRuntime;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JumboAudioPhase {
    Taxi,
    Accel,
    TakeOff,
    Flying,
    Landing,
    Decel,
}

#[derive(Debug, Clone, Copy)]
pub struct JumboPathInput {
    pub position: f32,
    pub take_off_point: f32,
    pub landing_point: f32,
}

/// Native gJumboVolOffsetPercentage has process lifetime, independent of actors.
#[derive(Debug, Default, Resource)]
pub struct JumboAudioState {
    pub volume_offset_percentage: u8,
    pub phases: HashMap<JumboAudioPhase, u64>,
    pub missing_path_events: u64,
    pub invalid_path_events: u64,
    pub missing_metadata_events: u64,
    pub unsupported_model_events: u64,
}
impl JumboAudioState {
    pub fn path_input(
        &mut self,
        runtime: Option<&CPlaneRuntime>,
        plane_id: i16,
    ) -> Option<JumboPathInput> {
        let Some(runtime) = runtime else {
            self.missing_path_events += 1;
            bevy::log::warn_once!("jumbo_audio_missing_plane_runtime");
            return None;
        };
        let position = usize::try_from(plane_id)
            .ok()
            .and_then(|id| runtime.plane_path_position.get(id))
            .copied();
        let Some(position) = position.filter(|position| position.is_finite()) else {
            self.invalid_path_events += 1;
            bevy::log::warn_once!(plane_id, "jumbo_audio_invalid_plane_identity");
            return None;
        };
        if runtime.path_nodes.is_empty()
            || !runtime.take_off_point.is_finite()
            || !runtime.landing_point.is_finite()
            || runtime.take_off_point < 0.0
            || runtime.landing_point <= runtime.take_off_point
        {
            self.invalid_path_events += 1;
            bevy::log::warn_once!("jumbo_audio_invalid_original_flight_path");
            return None;
        }
        Some(JumboPathInput {
            position,
            take_off_point: runtime.take_off_point,
            landing_point: runtime.landing_point,
        })
    }
}

pub fn jumbo_phase(path: JumboPathInput, speed: f32) -> JumboAudioPhase {
    if path.position > path.take_off_point {
        if path.take_off_point + 300.0 < path.position {
            if path.landing_point - 350.0 < path.position {
                if path.position > path.landing_point {
                    if speed > 0.103344 {
                        JumboAudioPhase::Decel
                    } else {
                        JumboAudioPhase::Taxi
                    }
                } else {
                    JumboAudioPhase::Landing
                }
            } else {
                JumboAudioPhase::Flying
            }
        } else {
            JumboAudioPhase::TakeOff
        }
    } else if speed > 0.103344 {
        JumboAudioPhase::Accel
    } else {
        JumboAudioPhase::Taxi
    }
}

pub fn process_jumbo(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    path: JumboPathInput,
    state: &mut JumboAudioState,
) -> bool {
    if snapshot.distance_squared >= JUMBO_MAX_DIST * JUMBO_MAX_DIST {
        return false;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    // Native random table is nonnegative; consume it without drawing new RNG.
    if audio.m_FrameCounter % (audio.m_anRandomTable[0].unsigned_abs() % 6 + 3) == 0 {
        state.volume_offset_percentage = (audio.m_anRandomTable[1].unsigned_abs() % 60) as u8;
    }
    let phase = jumbo_phase(path, snapshot.jumbo_speed);
    *state.phases.entry(phase).or_default() += 1;
    let mut voice = JumboSamples {
        audio,
        snapshot,
        distance,
        state,
    };
    match phase {
        JumboAudioPhase::Taxi => {
            if voice.fly(20) && voice.taxi(75) {
                voice.whine(18, 29500);
            }
        }
        JumboAudioPhase::Accel => {
            if voice.fly(20) {
                let modifier = ((snapshot.jumbo_speed - 0.103344) * 1.6760077).min(1.0);
                if voice.rumble((MAX_VOLUME as f32 * modifier) as u8)
                    && voice.taxi(((1.0 - modifier) * 75.0) as u8)
                {
                    let frequency_multiplier = if modifier >= 0.2 { 1.0 } else { modifier * 5.0 };
                    let volume = if modifier >= 0.2 {
                        MAX_VOLUME
                    } else {
                        (frequency_multiplier * MAX_VOLUME as f32) as u8
                    };
                    let frequency = if modifier >= 0.2 {
                        22050
                    } else {
                        (frequency_multiplier * 6050.0 + 16000.0) as u32
                    };
                    voice.engine(volume, frequency);
                    voice.whine(18, (14600.0 * frequency_multiplier + 29500.0) as u32);
                }
            }
        }
        JumboAudioPhase::TakeOff => {
            let modifier = (path.position - path.take_off_point) / 300.0;
            if voice.fly((107.0 * modifier + 20.0) as u8)
                && voice.rumble((MAX_VOLUME as f32 * (1.0 - modifier)) as u8)
                && voice.engine(MAX_VOLUME, 22050)
            {
                voice.whine((18.0 * (1.0 - modifier)) as u8, 44100);
            }
        }
        JumboAudioPhase::Flying => {
            if voice.fly(MAX_VOLUME) {
                voice.engine(63, 22050);
            }
        }
        JumboAudioPhase::Landing => {
            let modifier = (path.landing_point - path.position) / 350.0;
            if voice.fly((107.0 * modifier + 20.0) as u8)
                && voice.taxi((75.0 * (1.0 - modifier)) as u8)
            {
                voice.engine(MAX_VOLUME, 22050);
                voice.whine(
                    (18.0 * (1.0 - modifier)) as u8,
                    (14600.0 * modifier + 29500.0) as u32,
                );
            }
        }
        JumboAudioPhase::Decel => {
            if voice.fly(20) && voice.taxi(75) {
                let modifier = ((snapshot.jumbo_speed - 0.103344) * 1.6760077).min(1.0);
                voice.engine(
                    (MAX_VOLUME as f32 * modifier) as u8,
                    (6050.0 * modifier + 16000.0) as u32,
                );
                voice.whine(18, 29500);
            }
        }
    }
    true
}

struct JumboSamples<'a> {
    audio: &'a mut cAudioManager,
    snapshot: &'a VehicleAudioSnapshot,
    distance: f32,
    state: &'a mut JumboAudioState,
}
impl JumboSamples<'_> {
    fn submit(
        &mut self,
        sample: u32,
        counter: u32,
        volume: u8,
        frequency: u32,
        max_distance: f32,
        frames: f32,
        pan: Option<u8>,
    ) -> bool {
        if self.distance >= max_distance {
            return false;
        }
        if !self.audio.has_sample_metadata(sample) {
            self.state.missing_metadata_events += 1;
            bevy::log::warn_once!(sample, "jumbo_audio_missing_original_sample");
            return false;
        }
        let audible = self
            .audio
            .ComputeVolume(volume, max_distance, self.distance);
        if audible > 0 {
            let sound = tSound {
                m_nEntityIndex: self.snapshot.audio_entity,
                m_nCounter: counter,
                m_nSampleIndex: sample,
                m_nBankIndex: SFX_BANK_0,
                m_bIs2D: pan.is_some(),
                m_nPan: pan.unwrap_or(63),
                m_nPriority: 1,
                m_nFrequency: frequency,
                m_nVolume: audible,
                m_nEmittingVolume: volume,
                m_nLoopCount: 0,
                m_nLoopStart: self.audio.sample_loop_start(sample),
                m_nLoopEnd: self.audio.sample_loop_end(sample),
                m_fSpeedMultiplier: 4.0,
                m_MaxDistance: max_distance,
                m_bStatic: false,
                m_vecPos: self.snapshot.position,
                m_bReverb: true,
                m_nFramesToPlay: frames,
                ..Default::default()
            };
            self.audio.m_sQueueSample = sound;
            self.audio.AddSampleToRequestedQueue();
        }
        true
    }
    fn fly(&mut self, volume: u8) -> bool {
        self.submit(
            SFX_JUMBO_DIST_FLY as u32,
            0,
            volume,
            self.audio.sample_base_frequency(SFX_JUMBO_DIST_FLY as u32),
            JUMBO_MAX_DIST,
            5.0,
            None,
        )
    }
    fn taxi(&mut self, volume: u8) -> bool {
        let half = volume >> 1;
        let mut emitting =
            (half as f32 + half as f32 * self.distance / JUMBO_ENGINE_SOUND_MAX_DIST) as u8;
        if self.distance / JUMBO_ENGINE_SOUND_MAX_DIST < 0.7 {
            emitting -=
                (u32::from(emitting) * u32::from(self.state.volume_offset_percentage) / 100) as u8;
        }
        self.submit(
            SFX_JUMBO_TAXI as u32,
            1,
            emitting,
            ((1.0 / 180.0) * 10950.0 * self.distance + 22050.0) as u32,
            JUMBO_ENGINE_SOUND_MAX_DIST,
            4.0,
            None,
        )
    }
    fn whine(&mut self, volume: u8, frequency: u32) -> bool {
        self.submit(
            SFX_JUMBO_WHINE as u32,
            2,
            volume,
            frequency,
            JUMBO_WHINE_SOUND_MAX_DIST,
            4.0,
            None,
        )
    }
    fn engine(&mut self, volume: u8, frequency: u32) -> bool {
        // Preserve native integer division; the original subtracts offset/100.
        let emitting = volume - self.state.volume_offset_percentage / 100;
        self.submit(
            SFX_JUMBO_ENGINE as u32,
            3,
            emitting,
            frequency,
            JUMBO_ENGINE_SOUND_MAX_DIST,
            4.0,
            None,
        )
    }
    fn rumble(&mut self, volume: u8) -> bool {
        let frequency = self.audio.sample_base_frequency(SFX_JUMBO_RUMBLE as u32);
        if !self.submit(
            SFX_JUMBO_RUMBLE as u32,
            5,
            volume,
            frequency,
            JUMBO_RUMBLE_SOUND_MAX_DIST,
            12.0,
            Some(0),
        ) {
            return false;
        }
        self.submit(
            SFX_JUMBO_RUMBLE as u32,
            6,
            volume,
            frequency + 200,
            JUMBO_RUMBLE_SOUND_MAX_DIST,
            12.0,
            Some(127),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn path(position: f32) -> JumboPathInput {
        JumboPathInput {
            position,
            take_off_point: 1000.0,
            landing_point: 3000.0,
        }
    }
    fn manager() -> cAudioManager {
        let mut manager = cAudioManager::default();
        manager.Initialise();
        manager.SetSampleMetadata(&vec![
            crate::audio::sampman::SampleMetadata {
                frequency: 22050,
                size: 44100,
                loop_start: 100,
                loop_end: 1000,
                ..Default::default()
            };
            TOTAL_AUDIO_SAMPLES as usize
        ]);
        manager.m_FrameCounter = 1;
        manager
    }
    fn requested(manager: &cAudioManager, counter: u32) -> Option<tSound> {
        let queue = manager.m_nActiveQueue;
        manager.m_aRequestedQueue[queue][..manager.m_nRequestedCount[queue]]
            .iter()
            .find(|sound| sound.m_nCounter == counter)
            .copied()
    }
    #[test]
    fn native_phase_boundaries_use_path_even_at_identical_speed() {
        assert_eq!(jumbo_phase(path(1000.0), 0.103344), JumboAudioPhase::Taxi);
        assert_eq!(jumbo_phase(path(1000.0), 0.2), JumboAudioPhase::Accel);
        assert_eq!(jumbo_phase(path(1300.0), 0.2), JumboAudioPhase::TakeOff);
        assert_eq!(jumbo_phase(path(1300.01), 0.2), JumboAudioPhase::Flying);
        assert_eq!(jumbo_phase(path(2650.0), 0.2), JumboAudioPhase::Flying);
        assert_eq!(jumbo_phase(path(2650.01), 0.2), JumboAudioPhase::Landing);
        assert_eq!(jumbo_phase(path(3000.0), 0.2), JumboAudioPhase::Landing);
        assert_eq!(jumbo_phase(path(3000.01), 0.2), JumboAudioPhase::Decel);
        assert_eq!(jumbo_phase(path(3000.01), 0.1), JumboAudioPhase::Taxi);
    }
    #[test]
    fn native_takeoff_rumble_is_stereo_and_uses_original_loop_descriptors() {
        let mut manager = manager();
        let mut state = JumboAudioState::default();
        let snapshot = VehicleAudioSnapshot {
            audio_entity: manager.m_nFrontEndEntity,
            jumbo_speed: 0.2,
            ..Default::default()
        };
        assert!(process_jumbo(
            &mut manager,
            &snapshot,
            path(1150.0),
            &mut state
        ));
        let left = requested(&manager, 5).unwrap();
        let right = requested(&manager, 6).unwrap();
        assert!(left.m_bIs2D && right.m_bIs2D);
        assert_eq!(left.m_nPan, 0);
        assert_eq!(right.m_nPan, 127);
        assert_eq!(right.m_nFrequency - left.m_nFrequency, 200);
        assert_eq!(left.m_nLoopStart, 100);
        assert_eq!(left.m_nLoopEnd, 1000);
        assert_eq!(left.m_nEmittingVolume, 63);
        assert_eq!(requested(&manager, 0).unwrap().m_nEmittingVolume, 73);
        assert_eq!(requested(&manager, 2).unwrap().m_nEmittingVolume, 9);
        assert_eq!(requested(&manager, 2).unwrap().m_nFrequency, 44100);
    }
    #[test]
    fn native_volume_history_and_distance_gates_preserve_short_circuiting() {
        let mut manager = manager();
        let mut state = JumboAudioState {
            volume_offset_percentage: 50,
            ..Default::default()
        };
        let mut snapshot = VehicleAudioSnapshot {
            audio_entity: manager.m_nFrontEndEntity,
            ..Default::default()
        };
        process_jumbo(&mut manager, &snapshot, path(900.0), &mut state);
        assert_eq!(requested(&manager, 1).unwrap().m_nEmittingVolume, 19);
        manager.ClearRequestedQueue();
        snapshot.distance_squared = 200.0 * 200.0;
        process_jumbo(&mut manager, &snapshot, path(900.0), &mut state);
        assert!(requested(&manager, 0).is_some());
        assert!(requested(&manager, 1).is_none() && requested(&manager, 2).is_none());
        manager.ClearRequestedQueue();
        snapshot.distance_squared = JUMBO_MAX_DIST * JUMBO_MAX_DIST;
        let frames = state.phases.values().sum::<u64>();
        assert!(!process_jumbo(
            &mut manager,
            &snapshot,
            path(900.0),
            &mut state
        ));
        assert_eq!(state.phases.values().sum::<u64>(), frames);
        assert_eq!(state.volume_offset_percentage, 50);
    }
    #[test]
    fn missing_path_or_sample_is_observable_without_substitute_sound() {
        let mut state = JumboAudioState::default();
        assert!(state.path_input(None, 0).is_none());
        assert_eq!(state.missing_path_events, 1);
        let mut manager = manager();
        manager.SetSampleMetadata(&[]);
        let snapshot = VehicleAudioSnapshot {
            audio_entity: manager.m_nFrontEndEntity,
            ..Default::default()
        };
        process_jumbo(&mut manager, &snapshot, path(900.0), &mut state);
        assert_eq!(state.missing_metadata_events, 1);
        assert_eq!(manager.m_nRequestedCount[manager.m_nActiveQueue], 0);
    }
    #[test]
    fn native_shared_volume_survives_manager_restart_and_off_cadence_frames() {
        let mut manager = manager();
        let mut state = JumboAudioState::default();
        manager.m_FrameCounter = 6;
        manager.m_anRandomTable[0] = 0;
        manager.m_anRandomTable[1] = 47;
        let mut snapshot = VehicleAudioSnapshot {
            audio_entity: manager.m_nFrontEndEntity,
            ..Default::default()
        };
        process_jumbo(&mut manager, &snapshot, path(900.0), &mut state);
        assert_eq!(state.volume_offset_percentage, 47);
        manager.Terminate();
        manager.Initialise();
        manager.m_FrameCounter = 1;
        snapshot.audio_entity = manager.m_nFrontEndEntity;
        process_jumbo(&mut manager, &snapshot, path(900.0), &mut state);
        assert_eq!(state.volume_offset_percentage, 47);
    }
}
