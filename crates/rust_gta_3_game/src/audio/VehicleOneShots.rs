//! Native ProcessVehicleOneShots switch and process-wide sequence ownership.
use super::*;

#[derive(Debug, Resource)]
pub struct VehicleOneShotAudioState {
    pub wave_index: u8,
    pub gun_index: u8,
    pub wheel_index: u8,
    pub crunch_offset: u8,
    pub processed_events: u64,
    pub queued_events: u64,
    pub rejected_events: u64,
    pub invalid_model_events: u64,
    pub missing_sample_metadata_events: u64,
    pub speech_requests: u64,
    pub speech_queued: u64,
    pub speech_filtered: u64,
    pub(super) pending_comments: Vec<VehicleCommentRequest>,
}

impl VehicleOneShotAudioState {
    pub const fn new() -> Self {
        Self {
            wave_index: 41,
            gun_index: 53,
            wheel_index: 82,
            crunch_offset: 0,
            processed_events: 0,
            queued_events: 0,
            rejected_events: 0,
            invalid_model_events: 0,
            missing_sample_metadata_events: 0,
            speech_requests: 0,
            speech_queued: 0,
            speech_filtered: 0,
            pending_comments: Vec::new(),
        }
    }
}

impl Default for VehicleOneShotAudioState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(super) struct VehicleCommentRequest {
    pub sound: u16,
    pub audio_entity: i32,
    pub position: CVector,
    pub distance_squared: f32,
    pub distance_calculated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleOneShotOutcome {
    None,
    Queued,
    RequestPedComment(u16),
}

/// Drain re3 audio-entity one-shot events (door open/close, engine start,
/// weapon hits, ...) queued via PlayOneShot, mirroring the
/// ProcessVehicleOneShots tail of re3 ProcessVehicle plus the
/// InterrogateAudioEntities event reset.
pub(super) fn drain_vehicle_one_shots(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    state: &mut VehicleOneShotAudioState,
) {
    let entity = snapshot.audio_entity;
    if entity < 0 {
        return;
    }
    let (events, volumes, count) = match audio.m_asAudioEntities.get(entity as usize) {
        Some(entry) => (entry.m_awAudioEvent, entry.m_afVolume, entry.m_AudioEvents),
        None => {
            bevy::log::warn_once!(entity, "vehicle_one_shots_invalid_audio_entity");
            return;
        }
    };
    for i in 0..(count as usize).min(events.len()) {
        state.processed_events += 1;
        match process_vehicle_one_shot(audio, snapshot, events[i], volumes[i], state) {
            VehicleOneShotOutcome::Queued => state.queued_events += 1,
            VehicleOneShotOutcome::None => state.rejected_events += 1,
            VehicleOneShotOutcome::RequestPedComment(sound) => {
                state.speech_requests += 1;
                state.pending_comments.push(VehicleCommentRequest {
                    sound,
                    audio_entity: entity,
                    position: snapshot.position,
                    distance_squared: snapshot.distance_squared,
                    distance_calculated: snapshot.distance_calculated,
                });
            }
        }
    }
    if let Some(entry) = audio.m_asAudioEntities.get_mut(entity as usize) {
        entry.m_AudioEvents = 0;
    }
}

/// Route native special vehicle speech using the existing PedTalk owner.
pub fn flush_vehicle_comments(
    audio: &mut cAudioManager,
    sequences: &mut VehicleOneShotAudioState,
    speech: &mut crate::audio::PedTalk::PedTalkState,
    loader: &crate::core::FileLoader::CFileLoader,
    listener: CVector,
    time_ms: u32,
) {
    for request in sequences.pending_comments.drain(..) {
        let ctx = crate::audio::PedTalk::PedCommentContext {
            audio_entity: request.audio_entity,
            position: request.position,
            distance_squared: request.distance_squared,
            distance_calculated: request.distance_calculated,
            model_index: None,
            is_player: false,
            special_model_name: None,
            line_of_sight_clear: loader
                .world
                .process_line_of_sight_building_contact(
                    [listener.x, listener.y, listener.z],
                    [request.position.x, request.position.y, request.position.z],
                    &loader.model_info,
                )
                .is_none(),
            pursuit_blocked: false,
            time_ms,
        };
        if crate::audio::PedTalk::setup_ped_comment(audio, speech, &ctx, request.sound) {
            sequences.speech_queued += 1;
        } else {
            sequences.speech_filtered += 1;
        }
    }
}

pub fn process_vehicle_one_shot(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    event: i16,
    event_volume: f32,
    state: &mut VehicleOneShotAudioState,
) -> VehicleOneShotOutcome {
    let mapped = match_vehicle_one_shot(audio, snapshot, event, event_volume, state);
    let Some(mapped) = mapped else {
        return match event {
            x if x == SOUND_PED_HELI_PLAYER_FOUND as i16 => {
                VehicleOneShotOutcome::RequestPedComment(SOUND_PED_HELI_PLAYER_FOUND as u16)
            }
            x if x == SOUND_PED_BODYCAST_HIT as i16 => {
                VehicleOneShotOutcome::RequestPedComment(SOUND_PED_BODYCAST_HIT as u16)
            }
            _ => VehicleOneShotOutcome::None,
        };
    };

    if snapshot.distance_squared >= mapped.max_distance * mapped.max_distance {
        return VehicleOneShotOutcome::None;
    }
    if !audio.has_sample_metadata(mapped.sample) {
        state.missing_sample_metadata_events += 1;
        bevy::log::warn_once!(
            sample = mapped.sample,
            "vehicle_one_shot_missing_original_sample_metadata"
        );
        return VehicleOneShotOutcome::None;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(mapped.emitting_volume, mapped.max_distance, distance);
    if volume == 0 {
        return VehicleOneShotOutcome::None;
    }
    queue_one_shot_sample(audio, snapshot, mapped, volume);
    VehicleOneShotOutcome::Queued
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct VehicleOneShotSample {
    sample: u32,
    counter: u32,
    frequency: u32,
    priority: u32,
    emitting_volume: u8,
    max_distance: f32,
    speed_multiplier: f32,
    looping: bool,
    frames_to_play: f32,
}

#[allow(clippy::too_many_lines)]
fn match_vehicle_one_shot(
    audio: &cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    event: i16,
    event_volume: f32,
    state: &mut VehicleOneShotAudioState,
) -> Option<VehicleOneShotSample> {
    let sample;
    let counter;
    let frequency;
    let priority;
    let emitting_volume;
    let max_distance;
    let mut speed_multiplier = 0.0;
    let mut looping = false;
    let mut frames_to_play = 0.0;

    match event {
        x if x == SOUND_CAR_WINDSHIELD_CRACK as i16 => {
            sample = SFX_GLASS_CRACK as u32;
            counter = 68;
            frequency = audio.sample_base_frequency(SFX_GLASS_CRACK as u32);
            priority = 5;
            emitting_volume =
                random_mod(audio, 1, 30) as u8 + VEHICLE_ONE_SHOT_WINDSHIELD_CRACK_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_WINDSHIELD_CRACK_MAX_DIST;
        }
        x if (SOUND_CAR_DOOR_OPEN_BONNET as i16..=SOUND_CAR_DOOR_OPEN_BACK_RIGHT as i16)
            .contains(&x) =>
        {
            let Some(settings) = A_VEHICLE_SETTINGS.get(snapshot.model_index) else {
                state.invalid_model_events += 1;
                bevy::log::warn_once!(
                    model_index = snapshot.model_index,
                    "vehicle_one_shot_invalid_door_model"
                );
                return None;
            };
            sample = match settings.m_bDoorType {
                OLD_DOOR => SFX_OLD_CAR_DOOR_OPEN as u32,
                TRUCK_DOOR => SFX_TRUCK_DOOR_OPEN as u32,
                BUS_DOOR => SFX_AIR_BRAKES as u32,
                _ => SFX_NEW_CAR_DOOR_OPEN as u32,
            };
            counter = event as u32 + 10;
            frequency = displaced_frequency(
                audio,
                audio.sample_base_frequency(sample) as i32,
                audio.sample_base_frequency(sample) >> 5,
            );
            priority = 3;
            emitting_volume = random_mod(
                audio,
                1,
                (MAX_VOLUME - VEHICLE_ONE_SHOT_DOOR_CLOSE_VOLUME) as u32,
            ) as u8
                + VEHICLE_ONE_SHOT_DOOR_CLOSE_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_DOOR_MAX_DIST;
        }
        x if (SOUND_CAR_DOOR_CLOSE_BONNET as i16..=SOUND_CAR_DOOR_CLOSE_BACK_RIGHT as i16)
            .contains(&x) =>
        {
            let Some(settings) = A_VEHICLE_SETTINGS.get(snapshot.model_index) else {
                state.invalid_model_events += 1;
                bevy::log::warn_once!(
                    model_index = snapshot.model_index,
                    "vehicle_one_shot_invalid_door_model"
                );
                return None;
            };
            sample = match settings.m_bDoorType {
                OLD_DOOR => SFX_OLD_CAR_DOOR_CLOSE as u32,
                TRUCK_DOOR => SFX_TRUCK_DOOR_CLOSE as u32,
                BUS_DOOR => SFX_AIR_BRAKES as u32,
                _ => SFX_NEW_CAR_DOOR_CLOSE as u32,
            };
            counter = event as u32 + 22;
            frequency = displaced_frequency(
                audio,
                audio.sample_base_frequency(sample) as i32,
                audio.sample_base_frequency(sample) >> 5,
            );
            priority = 3;
            emitting_volume = random_mod(
                audio,
                2,
                (MAX_VOLUME - VEHICLE_ONE_SHOT_DOOR_OPEN_VOLUME) as u32,
            ) as u8
                + VEHICLE_ONE_SHOT_DOOR_OPEN_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_DOOR_MAX_DIST;
        }
        x if x == SOUND_CAR_ENGINE_START as i16 => {
            sample = SFX_CAR_STARTER as u32;
            counter = 33;
            frequency = audio.sample_base_frequency(sample);
            priority = 1;
            speed_multiplier = 2.0;
            emitting_volume = VEHICLE_ONE_SHOT_CAR_ENGINE_START_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_CAR_ENGINE_START_MAX_DIST;
        }
        x if x == SOUND_WEAPON_HIT_VEHICLE as i16 => {
            sample = SFX_BULLET_CAR_1 as u32 + random_mod(audio, snapshot.audio_entity as usize, 6);
            counter = 34;
            frequency = displaced_frequency(
                audio,
                audio.sample_base_frequency(sample) as i32,
                audio.sample_base_frequency(sample) >> 5,
            );
            priority = 7;
            emitting_volume =
                random_mod(audio, 3, 20) as u8 + VEHICLE_ONE_SHOT_WEAPON_HIT_VEHICLE_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_WEAPON_HIT_VEHICLE_MAX_DIST;
        }
        x if x == SOUND_BOMB_TIMED_ACTIVATED as i16
            || x == SOUND_55 as i16
            || x == SOUND_BOMB_ONIGNITION_ACTIVATED as i16
            || x == SOUND_BOMB_TICK as i16 =>
        {
            sample = SFX_ARM_BOMB as u32;
            counter = 36;
            frequency = audio.sample_base_frequency(sample);
            priority = 0;
            emitting_volume = VEHICLE_ONE_SHOT_BOMB_ARMED_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_BOMB_ARMED_MAX_DIST;
        }
        x if x == SOUND_CAR_LIGHT_BREAK as i16 => {
            sample = SFX_GLASS_SHARD_1 as u32;
            counter = 37;
            let base = 9 * audio.sample_base_frequency(sample) / 10;
            frequency = displaced_frequency(audio, base as i32, base >> 3);
            priority = 5;
            emitting_volume =
                random_mod(audio, 4, 10) as u8 + VEHICLE_ONE_SHOT_CAR_LIGHT_BREAK_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_WINDSHIELD_CRACK_MAX_DIST;
        }
        x if x == SOUND_PLANE_ON_GROUND as i16 => {
            sample = SFX_JUMBO_LAND_WHEELS as u32;
            counter = 81;
            frequency = audio.sample_base_frequency(sample);
            priority = 2;
            emitting_volume =
                random_mod(audio, 4, 25) as u8 + VEHICLE_ONE_SHOT_PLANE_ON_GROUND_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_PLANE_ON_GROUND_MAX_DIST;
        }
        x if x == SOUND_CAR_JERK as i16 => {
            sample = SFX_SHAG_SUSPENSION as u32;
            counter = 87;
            frequency = displaced_frequency(
                audio,
                audio.sample_base_frequency(sample) as i32,
                audio.sample_base_frequency(sample) >> 3,
            );
            priority = 5;
            emitting_volume =
                random_mod(audio, 1, 15) as u8 + VEHICLE_ONE_SHOT_CAR_HYDRAULIC_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_CAR_HYDRAULIC_MAX_DIST;
        }
        x if x == SOUND_CAR_HYDRAULIC_1 as i16 || x == SOUND_CAR_HYDRAULIC_2 as i16 => {
            sample = SFX_SUSPENSION_FAST_MOVE as u32;
            counter = 51;
            let base = if event == SOUND_CAR_HYDRAULIC_1 as i16 {
                15600
            } else {
                13118
            };
            frequency = displaced_frequency(audio, base, (base as u32) >> 3);
            priority = 5;
            emitting_volume =
                random_mod(audio, 0, 15) as u8 + VEHICLE_ONE_SHOT_CAR_HYDRAULIC_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_CAR_HYDRAULIC_MAX_DIST;
        }
        x if x == SOUND_CAR_HYDRAULIC_3 as i16 => {
            sample = SFX_SUSPENSION_SLOW_MOVE_LOOP as u32;
            counter = 86;
            frequency = audio.sample_base_frequency(sample);
            priority = 5;
            emitting_volume =
                random_mod(audio, 0, 15) as u8 + VEHICLE_ONE_SHOT_CAR_HYDRAULIC_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_CAR_HYDRAULIC_MAX_DIST;
            looping = true;
            frames_to_play = 7.0;
        }
        x if x == SOUND_WATER_FALL as i16 => {
            sample = SFX_SPLASH_1 as u32;
            counter = 15;
            frequency = displaced_frequency(audio, 16000, 1000);
            priority = 1;
            emitting_volume = random_mod(audio, 4, 20) as u8 + VEHICLE_ONE_SHOT_WATER_FALL_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_WATER_FALL_MAX_DIST;
        }
        x if x == SOUND_CAR_BOMB_TICK as i16 => {
            sample = SFX_BOMB_BEEP as u32;
            counter = 80;
            frequency = audio.sample_base_frequency(sample);
            priority = 3;
            emitting_volume = VEHICLE_ONE_SHOT_CAR_BOMB_TICK_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_CAR_BOMB_TICK_MAX_DIST;
        }
        x if x == SOUND_CAR_SPLASH as i16 => {
            let clamped = event_volume.clamp(300.0, 1200.0);
            if event_volume <= 300.0 {
                return None;
            }
            let event_rel_vol = (clamped - 300.0) / 900.0;
            sample = SFX_BOAT_SPLASH_1 as u32 + random_mod(audio, 0, 2);
            counter = state.wave_index as u32;
            state.wave_index += 1;
            if state.wave_index > 46 {
                state.wave_index = 41;
            }
            frequency = (7000.0 * event_rel_vol + 6000.0) as u32;
            priority = 3;
            speed_multiplier = 2.0;
            emitting_volume = (VEHICLE_ONE_SHOT_CAR_SPLASH_VOLUME as f32 * event_rel_vol) as u8;
            max_distance = VEHICLE_ONE_SHOT_CAR_SPLASH_MAX_DIST;
        }
        x if x == SOUND_BOAT_SLOWDOWN as i16 => {
            sample = SFX_POLICE_BOAT_THUMB_OFF as u32;
            counter = 47;
            frequency = displaced_frequency(audio, audio.sample_base_frequency(sample) as i32, 600);
            priority = 2;
            emitting_volume = event_volume.clamp(0.0, MAX_VOLUME as f32) as u8;
            max_distance = VEHICLE_ONE_SHOT_BOAT_SLOWDOWN_MAX_DIST;
        }
        x if x == SOUND_CAR_JUMP as i16 => {
            sample = SFX_TYRE_BUMP as u32;
            counter = state.wheel_index as u32;
            state.wheel_index += 1;
            if state.wheel_index > 85 {
                state.wheel_index = 82;
            }
            let base_frequency = displaced_frequency(
                audio,
                audio.sample_base_frequency(sample) as i32,
                audio.sample_base_frequency(sample) >> 4,
            );
            priority = 6;
            speed_multiplier = 2.0;
            let native_volume = VEHICLE_ONE_SHOT_CAR_JUMP_VOLUME.max((200.0 * event_volume) as u8);
            frequency = if snapshot.model_index == MI_RCBANDIT - MI_LANDSTAL {
                base_frequency << 1
            } else {
                base_frequency
            };
            emitting_volume = if snapshot.model_index == MI_RCBANDIT - MI_LANDSTAL {
                native_volume >> 1
            } else {
                native_volume
            };
            max_distance = VEHICLE_ONE_SHOT_CAR_JUMP_MAX_DIST;
        }
        x if x == SOUND_WEAPON_SHOT_FIRED as i16 => {
            sample = SFX_UZI_LEFT as u32;
            counter = state.gun_index as u32;
            state.gun_index += 1;
            if state.gun_index > 58 {
                state.gun_index = 53;
            }
            frequency = displaced_frequency(
                audio,
                audio.sample_base_frequency(sample) as i32,
                audio.sample_base_frequency(sample) >> 4,
            );
            priority = 3;
            emitting_volume =
                random_mod(audio, 2, 15) as u8 + VEHICLE_ONE_SHOT_WEAPON_SHOT_FIRED_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_WEAPON_SHOT_FIRED_MAX_DIST;
        }
        x if x == SOUND_TRAIN_DOOR_CLOSE as i16 || x == SOUND_TRAIN_DOOR_OPEN as i16 => {
            sample = SFX_AIR_BRAKES as u32;
            counter = 59;
            frequency = displaced_frequency(audio, 11025, 1000);
            priority = 5;
            speed_multiplier = 5.0;
            emitting_volume = random_mod(audio, 1, 20) as u8 + VEHICLE_ONE_SHOT_TRAIN_DOOR_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_TRAIN_DOOR_MAX_DIST;
        }
        x if x == SOUND_SPLATTER as i16 => {
            sample = SFX_PED_CRUNCH_1 as u32 + state.crunch_offset as u32;
            state.crunch_offset = (state.crunch_offset + 1) % 2;
            counter = 48;
            frequency = displaced_frequency(
                audio,
                audio.sample_base_frequency(SFX_PED_CRUNCH_1 as u32) as i32,
                600,
            );
            priority = 1;
            emitting_volume = random_mod(audio, 4, 20) as u8 + VEHICLE_ONE_SHOT_SPLATTER_VOLUME;
            max_distance = VEHICLE_ONE_SHOT_SPLATTER_MAX_DIST;
        }
        x if x == SOUND_CAR_PED_COLLISION as i16 => {
            let event_vol = event_volume.min(20.0);
            emitting_volume = (event_vol / 20.0 * MAX_VOLUME as f32) as u8;
            if emitting_volume == 0 {
                return None;
            }
            sample = SFX_FIGHT_1 as u32 + random_mod(audio, 2, 4);
            counter = 50;
            frequency = audio.sample_base_frequency(sample) >> 1;
            priority = 1;
            max_distance = VEHICLE_ONE_SHOT_CAR_PED_COLLISION_MAX_DIST;
        }
        x if x == SOUND_CAR_TANK_TURRET_ROTATE as i16 => {
            let event_vol = event_volume.min(96.0 / 2500.0);
            sample = SFX_TANK_TURRET as u32;
            counter = 79;
            frequency = (3000.0 * event_vol / (96.0 / 2500.0) + 9000.0) as u32;
            priority = 2;
            speed_multiplier = 2.0;
            frames_to_play = 3.0;
            max_distance = VEHICLE_ONE_SHOT_CAR_TANK_TURRET_MAX_DIST;
            emitting_volume = (37.0 * event_vol / (96.0 / 2500.0)
                + VEHICLE_ONE_SHOT_CAR_TANK_TURRET_VOLUME as f32)
                as u8;
            looping = true;
        }
        _ => return None,
    }

    Some(VehicleOneShotSample {
        sample,
        counter,
        frequency,
        priority,
        emitting_volume,
        max_distance,
        speed_multiplier,
        looping,
        frames_to_play,
    })
}

/// ProcessVehicleOneShots uses authored loop offsets even for finite samples.
fn queue_one_shot_sample(
    audio: &mut cAudioManager,
    source: &VehicleAudioSnapshot,
    sample: VehicleOneShotSample,
    volume: u8,
) {
    let loop_start = audio.sample_loop_start(sample.sample);
    let loop_end = audio.sample_loop_end(sample.sample);
    let output = &mut audio.m_sQueueSample;
    output.m_nEntityIndex = source.audio_entity;
    output.m_nCounter = sample.counter;
    output.m_nSampleIndex = sample.sample;
    output.m_nBankIndex = SFX_BANK_0;
    output.m_bIs2D = false;
    output.m_nPriority = sample.priority;
    output.m_nFrequency = sample.frequency;
    output.m_nVolume = volume;
    output.m_nEmittingVolume = sample.emitting_volume;
    output.m_nLoopCount = if sample.looping { 0 } else { 1 };
    output.m_nLoopStart = loop_start;
    output.m_nLoopEnd = loop_end;
    output.m_fSpeedMultiplier = sample.speed_multiplier;
    output.m_MaxDistance = sample.max_distance;
    output.m_bStatic = !sample.looping;
    output.m_vecPos = source.position;
    output.m_bReverb = true;
    output.m_nFramesToPlay = sample.frames_to_play;
    // TODO(parity): optional AUDIO_REFLECTIONS flag and reflection backend.
    audio.AddSampleToRequestedQueue();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_original_metadata_is_observable_and_never_fabricated() {
        let mut audio = cAudioManager::default();
        audio.Initialise();
        let mut state = VehicleOneShotAudioState::default();
        let source = VehicleAudioSnapshot { audio_entity: 3, ..Default::default() };
        assert_eq!(process_vehicle_one_shot(&mut audio, &source, SOUND_CAR_JUMP as i16, 1.0, &mut state), VehicleOneShotOutcome::None);
        assert_eq!(state.missing_sample_metadata_events, 1);
        assert_eq!(state.wheel_index, 83);
        assert_eq!(audio.m_nRequestedCount[audio.m_nActiveQueue], 0);
    }

    #[test]
    fn shared_sequences_advance_before_distance_filter_and_survive_manager_restart() {
        let mut audio = cAudioManager::default();
        audio.Initialise();
        let mut state = VehicleOneShotAudioState::default();
        let mut source = VehicleAudioSnapshot::default();
        source.distance_squared = f32::MAX;
        for actor in 0..7 {
            source.audio_entity = actor;
            assert_eq!(
                process_vehicle_one_shot(
                    &mut audio,
                    &source,
                    SOUND_CAR_SPLASH as i16,
                    1200.0,
                    &mut state
                ),
                VehicleOneShotOutcome::None
            );
        }
        assert_eq!(state.wave_index, 42);
        audio.Initialise();
        let mapped =
            match_vehicle_one_shot(&audio, &source, SOUND_CAR_SPLASH as i16, 1200.0, &mut state)
                .unwrap();
        assert_eq!(mapped.counter, 42);
        assert_eq!(state.wave_index, 43);
    }

    #[test]
    fn rc_jump_preserves_native_frequency_volume_and_authored_loop_offsets() {
        let mut audio = cAudioManager::default();
        audio.Initialise();
        let mut metadata =
            vec![crate::audio::sampman::SampleMetadata::default(); SFX_TYRE_BUMP as usize + 1];
        metadata[SFX_TYRE_BUMP as usize] = crate::audio::sampman::SampleMetadata {
            frequency: 22050,
            loop_start: 123,
            loop_end: 456,
            ..Default::default()
        };
        audio.SetSampleMetadata(&metadata);
        let mut state = VehicleOneShotAudioState::default();
        let mut source = VehicleAudioSnapshot {
            audio_entity: 3,
            ..Default::default()
        };
        let normal =
            match_vehicle_one_shot(&audio, &source, SOUND_CAR_JUMP as i16, 1.0, &mut state)
                .unwrap();
        source.model_index = MI_RCBANDIT - MI_LANDSTAL;
        let rc = match_vehicle_one_shot(&audio, &source, SOUND_CAR_JUMP as i16, 1.0, &mut state)
            .unwrap();
        assert_eq!(normal.emitting_volume, 200);
        assert_eq!(rc.emitting_volume, 100);
        assert_eq!(rc.frequency, normal.frequency * 2);
        queue_one_shot_sample(&mut audio, &source, rc, 100);
        assert_eq!(audio.m_sQueueSample.m_nLoopStart, 123);
        assert_eq!(audio.m_sQueueSample.m_nLoopEnd, 456);
        assert_eq!(audio.m_sQueueSample.m_nCounter, 83);
    }

    #[test]
    fn drain_preserves_special_speech_identity_and_clears_events_once() {
        let mut audio = cAudioManager::default();
        audio.Initialise();
        let id = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(77));
        audio.SetEntityStatus(id, true);
        audio.PlayOneShot(id, SOUND_PED_HELI_PLAYER_FOUND as u16, 0.0);
        let source = VehicleAudioSnapshot {
            audio_entity: id,
            position: CVector::new(1.0, 2.0, 3.0),
            ..Default::default()
        };
        let mut state = VehicleOneShotAudioState::default();
        drain_vehicle_one_shots(&mut audio, &source, &mut state);
        assert_eq!(state.speech_requests, 1);
        assert_eq!(state.pending_comments[0].audio_entity, id);
        assert_eq!(state.pending_comments[0].position, source.position);
        assert_eq!(
            state.pending_comments[0].sound,
            SOUND_PED_HELI_PLAYER_FOUND as u16
        );
        drain_vehicle_one_shots(&mut audio, &source, &mut state);
        assert_eq!(state.processed_events, 1);
    }
}
