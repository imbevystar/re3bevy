//! Native player engine function-static history and dedicated channel lifetime.
use super::*;
use crate::audio::DMAudio::PLAYER_ENGINE_VOICE_COUNTER;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCesnaAudioState {
    pub accel_counter: u8,
}

impl PlayerCesnaAudioState {
    pub const fn new() -> Self {
        Self { accel_counter: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerVehicleAudioState {
    pub last_accel: i16,
    pub last_brake: i16,
    pub current_pretend_gear: u8,
    pub lost_traction_last_frame: bool,
    pub handbrake_on_last_frame: bool,
    pub cruising: u32,
    pub accel_sample_stopped: bool,
    pub player_just_entered_car: bool,
}

impl Default for PlayerVehicleAudioState {
    fn default() -> Self {
        Self {
            last_accel: 0,
            last_brake: 0,
            current_pretend_gear: 1,
            lost_traction_last_frame: false,
            handbrake_on_last_frame: false,
            cruising: 0,
            accel_sample_stopped: true,
            player_just_entered_car: true,
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct PlayerEngineAudioState {
    pub engine: PlayerVehicleAudioState,
    pub cesna: PlayerCesnaAudioState,
    pub bound_audio_entity: Option<i32>,
    pub channel_rebindings: u64,
    pub bound_actor: Option<Entity>,
}
impl Default for PlayerCesnaAudioState {
    fn default() -> Self {
        Self::new()
    }
}
impl PlayerEngineAudioState {
    /// ECS identity detects replacement even if the audio manager reuses a slot.
    pub fn bind_actor(
        &mut self,
        audio: &mut cAudioManager,
        actor: Option<Entity>,
        entity: Option<i32>,
    ) {
        if self.bound_actor != actor {
            self.bind(audio, None);
            self.bound_actor = actor;
        }
        self.bind(audio, entity);
    }
    /// The dedicated player channel has one emitter. Entry resets engine
    /// history; native ProcessCesna's nAccel survives vehicle changes.
    pub fn bind(&mut self, audio: &mut cAudioManager, entity: Option<i32>) {
        if self.bound_audio_entity == entity {
            return;
        }
        if let Some(previous) = self.bound_audio_entity {
            audio.StopVoice(previous, PLAYER_ENGINE_VOICE_COUNTER);
        }
        self.stop_channel();
        self.bound_audio_entity = entity;
        self.channel_rebindings += 1;
        if entity.is_some() {
            self.engine.player_just_entered_car = true;
        }
    }
    pub fn stop_channel(&mut self) {
        self.engine.accel_sample_stopped = true;
        self.engine.cruising = 0;
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct VehicleAudioGlobals<'w, 's> {
    pub one_shots: ResMut<'w, super::VehicleOneShotAudioState>,
    pub player_engine: ResMut<'w, PlayerEngineAudioState>,
    pub jumbo: ResMut<'w, super::JumboAudioState>,
    pub plane_runtime: Option<Res<'w, crate::vehicles::Plane::CPlaneRuntime>>,
    pub channel: super::super::DMAudio::PlayerEngineChannel<'w, 's>,
}

const GEAR_FREQ_ADJ: [i32; 6] = [6000, 6000, 3400, 1200, 0, -1000];

#[allow(clippy::too_many_arguments)]
pub fn add_player_car_sample(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    vol: u8,
    freq: u32,
    sample: u32,
    _bank: u8,
    counter: u32,
    looping: bool,
) {
    if !audio.has_sample_metadata(sample) {
        bevy::log::warn_once!(sample, "player_engine_missing_original_sample_metadata");
        return;
    }
    let loop_start = if counter == PLAYER_ENGINE_VOICE_COUNTER {
        0
    } else {
        audio.sample_loop_start(sample)
    };
    let loop_end = if counter == PLAYER_ENGINE_VOICE_COUNTER {
        -1
    } else {
        audio.sample_loop_end(sample)
    };
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(vol, VEHICLE_ENGINE_MAX_DIST, distance);
    if volume > 0 {
        audio.m_sQueueSample.m_nEntityIndex = snapshot.audio_entity;
        audio.m_sQueueSample.m_nCounter = counter;
        audio.m_sQueueSample.m_nSampleIndex = sample;
        audio.m_sQueueSample.m_nBankIndex = SFX_BANK_0;
        audio.m_sQueueSample.m_bIs2D = false;
        audio.m_sQueueSample.m_nPriority = 0;
        audio.m_sQueueSample.m_nFrequency = freq;
        audio.m_sQueueSample.m_nVolume = volume.min(MAX_VOLUME);
        audio.m_sQueueSample.m_nEmittingVolume = vol.min(MAX_VOLUME);
        audio.m_sQueueSample.m_nLoopCount = if looping { 0 } else { 1 };
        audio.m_sQueueSample.m_nLoopStart = loop_start;
        audio.m_sQueueSample.m_nLoopEnd = loop_end;
        audio.m_sQueueSample.m_fSpeedMultiplier = 6.0;
        audio.m_sQueueSample.m_MaxDistance = VEHICLE_ENGINE_MAX_DIST;
        audio.m_sQueueSample.m_bStatic = false;
        audio.m_sQueueSample.m_vecPos = snapshot.position;
        audio.m_sQueueSample.m_bReverb = true;
        audio.m_sQueueSample.m_nFramesToPlay = if looping { 8.0 } else { 0.0 };
        if counter == PLAYER_ENGINE_VOICE_COUNTER {
            audio.StartDedicatedVoice(audio.m_sQueueSample);
        } else {
            audio.AddSampleToRequestedQueue();
        }
    }
}

fn update_player_car_sample(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    vol: u8,
    freq: u32,
    sample: u32,
    counter: u32,
) {
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(vol, VEHICLE_ENGINE_MAX_DIST, distance);
    audio.UpdateVoice(tSound {
        m_nEntityIndex: snapshot.audio_entity,
        m_nCounter: counter,
        m_nSampleIndex: sample,
        m_nBankIndex: SFX_BANK_0,
        m_bIs2D: false,
        m_nPriority: 0,
        m_nFrequency: freq,
        m_nVolume: volume.min(MAX_VOLUME),
        m_nEmittingVolume: vol.min(MAX_VOLUME),
        m_fDistance: distance,
        m_nLoopCount: 1,
        m_fSpeedMultiplier: 6.0,
        m_MaxDistance: VEHICLE_ENGINE_MAX_DIST,
        m_bStatic: false,
        m_vecPos: snapshot.position,
        m_bReverb: true,
        ..Default::default()
    });
}

pub fn process_players_vehicle_engine(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    state: &mut PlayerVehicleAudioState,
    gas_pedal_audio: &mut f32,
) -> bool {
    if snapshot.distance_squared >= VEHICLE_ENGINE_MAX_DIST * VEHICLE_ENGINE_MAX_DIST {
        // ProcessVehicleEngine filters this radius without changing the
        // dedicated channel or native history; the live voice finishes itself.
        return false;
    }
    if !snapshot.engine_on {
        return true;
    }
    if state.player_just_entered_car {
        state.accel_sample_stopped = true;
        state.player_just_entered_car = false;
        state.cruising = 0;
        state.last_accel = 0;
        state.lost_traction_last_frame = false;
        state.last_brake = 0;
        state.handbrake_on_last_frame = false;
        state.current_pretend_gear = 1;
    }

    // TODO(parity): cAudioManager::ProcessPlayersVehicleEngine — CReplay and
    // FIX_BUGS PED_EXIT_CAR input selection at the player/pad boundary.
    let accelerate_state = snapshot
        .player_accelerate_state
        .unwrap_or_else(|| (snapshot.gas_pedal.clamp(0.0, 1.0) * 255.0) as i16);
    let slowing_down = snapshot.velocity_change < -0.001;
    let relative_velocity_change = if snapshot.max_velocity > 0.0 {
        2.0 * snapshot.velocity_change / snapshot.max_velocity
    } else {
        0.0
    };
    let acceleration_multiplier = relative_velocity_change.clamp(0.0, 1.0);
    // re3 keeps this as a local. `automobile->m_fGasPedalAudio` is the
    // separately smoothed persistent value from UpdateGasPedalAudio.
    let mut local_gas_pedal_audio = acceleration_multiplier;
    let current_gear = snapshot.current_gear;

    let mut lost_traction = false;
    match snapshot.drive_type {
        '4' => {
            let mut wheel_in_use_counter = 0;
            for s in snapshot.wheel_states {
                if s != crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL {
                    wheel_in_use_counter += 1;
                }
            }
            if wheel_in_use_counter > 2 {
                lost_traction = true;
            }
        }
        'F' => {
            if (snapshot.wheel_states[crate::vehicles::Automobile::CARWHEEL_FRONT_LEFT]
                != crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL
                || snapshot.wheel_states[crate::vehicles::Automobile::CARWHEEL_FRONT_RIGHT]
                    != crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL)
                && (snapshot.wheel_states[crate::vehicles::Automobile::CARWHEEL_REAR_LEFT]
                    != crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL
                    || snapshot.wheel_states[crate::vehicles::Automobile::CARWHEEL_REAR_RIGHT]
                        != crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL)
            {
                lost_traction = true;
            }
        }
        'R' => {
            if snapshot.wheel_states[crate::vehicles::Automobile::CARWHEEL_REAR_LEFT]
                != crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL
                || snapshot.wheel_states[crate::vehicles::Automobile::CARWHEEL_REAR_RIGHT]
                    != crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL
            {
                lost_traction = true;
            }
        }
        _ => {}
    }

    let mut freq_modifier = if snapshot.velocity_change != 0.0 {
        let time = snapshot.move_speed_z / snapshot.velocity_change;
        let mut modifier = if time > 0.0 {
            -(time.min(0.2) * 3000.0 * 5.0) as i32
        } else {
            -(time.max(-0.2) * 3000.0 * 5.0) as i32
        };
        if slowing_down {
            modifier = -modifier;
        }
        modifier
    } else {
        0
    };

    let Some(settings) = A_VEHICLE_SETTINGS.get(snapshot.model_index).copied() else {
        bevy::log::warn_once!(
            model_index = snapshot.model_index,
            "player_engine_invalid_audio_model"
        );
        audio.StopVoice(snapshot.audio_entity, PLAYER_ENGINE_VOICE_COUNTER);
        return false;
    };
    let engine_sound_type = settings.m_nBank;
    let sound_offset = 3 * (engine_sound_type.saturating_sub(CAR_SFX_BANKS_OFFSET) as u32);
    let accel_sample = sound_offset + SFX_CAR_ACCEL_1 as u32;

    let Some(mut channel_used) = snapshot.player_engine_channel_used else {
        bevy::log::warn_once!("player_engine_waiting_for_backend_channel_state");
        return false;
    };

    if accelerate_state > 0 {
        if state.cruising > 0 {
            state.accel_sample_stopped = true;
            let top_gear = snapshot.num_gears.saturating_sub(1).max(1);
            if accelerate_state < 150
                || snapshot.wheels_on_ground == 0
                || snapshot.handbrake_on
                || lost_traction
                || current_gear < top_gear
            {
                state.cruising = 0;
            } else {
                if accelerate_state < 220
                    || snapshot.velocity_change + 0.001
                        < snapshot.previous_velocity_change_for_audio
                {
                    if state.cruising > 3 {
                        state.cruising -= 1;
                    }
                } else if state.cruising < 800 {
                    state.cruising += 1;
                }
                let mut freq = (27 * state.cruising as i32 + freq_modifier + 22050).max(1000);
                if engine_sound_type == SFX_BANK_TRUCK {
                    freq >>= 1;
                }
                add_player_car_sample(
                    audio,
                    snapshot,
                    PLAYER_VEHICLE_ENGINE_VOLUME,
                    freq as u32,
                    sound_offset + SFX_CAR_AFTER_ACCEL_1 as u32,
                    engine_sound_type,
                    64,
                    true,
                );
            }
        } else {
            let spin_condition = accelerate_state < 150
                || snapshot.wheels_on_ground == 0
                || snapshot.handbrake_on
                || lost_traction
                || (current_gear < 2
                    && (snapshot.velocity_change - snapshot.previous_velocity_change_for_audio)
                        < 0.01);

            if spin_condition {
                let (base_freq, vol) =
                    if snapshot.wheels_on_ground == 0 || snapshot.handbrake_on || lost_traction {
                        if (snapshot.wheels_on_ground == 0 && snapshot.drive_wheels_on_ground > 0)
                            || ((snapshot.handbrake_on && !state.handbrake_on_last_frame
                                || lost_traction && !state.lost_traction_last_frame)
                                && snapshot.wheels_on_ground > 0)
                        {
                            *gas_pedal_audio *= 0.6;
                        }
                        freq_modifier = 0;
                        (
                            (15000.0 * *gas_pedal_audio + 14000.0) as i32,
                            ((25.0 * *gas_pedal_audio + 60.0).clamp(0.0, 127.0)) as u8,
                        )
                    } else {
                        *gas_pedal_audio = acceleration_multiplier;
                        (
                            (8000.0 * acceleration_multiplier + 16000.0) as i32,
                            ((25.0 * acceleration_multiplier + 60.0).clamp(0.0, 127.0)) as u8,
                        )
                    };
                let mut freq = (freq_modifier + base_freq).max(1000);
                if engine_sound_type == SFX_BANK_TRUCK {
                    freq >>= 1;
                }
                if channel_used {
                    audio.StopVoice(snapshot.audio_entity, PLAYER_ENGINE_VOICE_COUNTER);
                    state.accel_sample_stopped = true;
                    channel_used = false;
                }
                add_player_car_sample(
                    audio,
                    snapshot,
                    vol,
                    freq as u32,
                    (engine_sound_type.saturating_sub(CAR_SFX_BANKS_OFFSET) as u32)
                        + SFX_CAR_REV_1 as u32,
                    SFX_BANK_0,
                    2,
                    true,
                );
            } else {
                let mut processed_accel_sample_stopped = false;
                if state.accel_sample_stopped {
                    if state.current_pretend_gear != 1 || current_gear != 2 {
                        state.current_pretend_gear = current_gear.saturating_sub(1).max(1);
                    }
                    processed_accel_sample_stopped = true;
                    state.accel_sample_stopped = false;
                }

                let top_gear = snapshot.num_gears.saturating_sub(1).max(1);
                if !channel_used {
                    if !processed_accel_sample_stopped {
                        if state.current_pretend_gear < top_gear {
                            state.current_pretend_gear += 1;
                        } else if current_gear >= top_gear {
                            state.cruising = 1;
                        }
                    }
                }

                if state.cruising > 0 {
                    state.accel_sample_stopped = true;
                    let mut freq = (27 * state.cruising as i32 + freq_modifier + 22050).max(1000);
                    if engine_sound_type == SFX_BANK_TRUCK {
                        freq >>= 1;
                    }
                    add_player_car_sample(
                        audio,
                        snapshot,
                        PLAYER_VEHICLE_ENGINE_VOLUME,
                        freq as u32,
                        sound_offset + SFX_CAR_AFTER_ACCEL_1 as u32,
                        engine_sound_type,
                        64,
                        true,
                    );
                } else {
                    let pretend_gear =
                        (state.current_pretend_gear as usize).min(GEAR_FREQ_ADJ.len() - 1);
                    let mut freq = (GEAR_FREQ_ADJ[pretend_gear] + freq_modifier + 22050).max(1000);
                    if engine_sound_type == SFX_BANK_TRUCK {
                        freq >>= 1;
                    }
                    if !channel_used {
                        if !audio.has_sample_metadata(accel_sample)
                            || audio.sample_length(accel_sample) == 0
                        {
                            bevy::log::warn_once!(
                                sample = accel_sample,
                                "player_engine_missing_acceleration_sample"
                            );
                            return false;
                        }
                        add_player_car_sample(
                            audio,
                            snapshot,
                            PLAYER_VEHICLE_ENGINE_VOLUME,
                            freq as u32,
                            accel_sample,
                            SFX_BANK_0,
                            PLAYER_ENGINE_VOICE_COUNTER,
                            false,
                        );
                    } else {
                        update_player_car_sample(
                            audio,
                            snapshot,
                            PLAYER_VEHICLE_ENGINE_VOLUME,
                            freq as u32,
                            accel_sample,
                            PLAYER_ENGINE_VOICE_COUNTER,
                        );
                    }
                }
            }
        }
    } else {
        if slowing_down {
            if channel_used {
                audio.StopVoice(snapshot.audio_entity, PLAYER_ENGINE_VOICE_COUNTER);
                state.accel_sample_stopped = true;
                channel_used = false;
            }
            local_gas_pedal_audio =
                if snapshot.wheels_on_ground == 0 || snapshot.handbrake_on || lost_traction {
                    *gas_pedal_audio
                } else if snapshot.max_reverse_velocity != 0.0 {
                    (snapshot.velocity_change / snapshot.max_reverse_velocity)
                        .min(1.0)
                        .max(0.0)
                } else {
                    0.0
                };
            *gas_pedal_audio = local_gas_pedal_audio;
        } else if state.last_accel > 0 {
            if channel_used {
                audio.StopVoice(snapshot.audio_entity, PLAYER_ENGINE_VOICE_COUNTER);
                state.accel_sample_stopped = true;
                channel_used = false;
            }
            state.cruising = 0;
            if snapshot.wheels_on_ground == 0
                || snapshot.handbrake_on
                || lost_traction
                || (snapshot.velocity_change < 0.01 && *gas_pedal_audio > 0.2)
            {
                *gas_pedal_audio *= 0.6;
                local_gas_pedal_audio = *gas_pedal_audio;
            }
            if local_gas_pedal_audio > 0.05 {
                let norm = (local_gas_pedal_audio - 0.05) / 0.95;
                let mut freq = ((5000.0 * norm) as i32 + 19000).max(1000);
                if engine_sound_type == SFX_BANK_TRUCK {
                    freq >>= 1;
                }
                let vol = ((25.0 * norm) as u8 + 40).clamp(0, 127);
                add_player_car_sample(
                    audio,
                    snapshot,
                    vol,
                    freq as u32,
                    sound_offset + SFX_CAR_FINGER_OFF_ACCEL_1 as u32,
                    engine_sound_type,
                    63,
                    false,
                );
            }
        }

        let mut freq = ((10000.0 * local_gas_pedal_audio) as i32 + 22050).max(1000);
        if engine_sound_type == SFX_BANK_TRUCK {
            freq >>= 1;
        }
        let vol = (110.0 - 40.0 * local_gas_pedal_audio).clamp(0.0, 127.0) as u8;
        add_player_car_sample(
            audio,
            snapshot,
            vol,
            freq as u32,
            (engine_sound_type.saturating_sub(CAR_SFX_BANKS_OFFSET) as u32) + SFX_CAR_IDLE_1 as u32,
            SFX_BANK_0,
            52,
            true,
        );

        state.current_pretend_gear = current_gear.max(1);
    }

    let _ = channel_used;

    state.last_accel = accelerate_state;
    state.handbrake_on_last_frame = snapshot.handbrake_on;
    state.lost_traction_last_frame = lost_traction;
    true
}

pub fn process_cesna(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    is_dodo: bool,
    is_player_vehicle: bool,
    accelerate_pressed: bool,
    player_state: &mut PlayerCesnaAudioState,
) -> bool {
    if is_player_vehicle {
        if !is_dodo {
            return false;
        }
        if accelerate_pressed {
            player_state.accel_counter = player_state.accel_counter.saturating_add(1).min(60);
        } else {
            player_state.accel_counter = player_state.accel_counter.saturating_sub(1);
        }
        let accel = u32::from(player_state.accel_counter);
        let idle_volume = (u32::from(PLAYER_VEHICLE_ENGINE_VOLUME) * (60 - accel) / 60 + 20) as u8;
        let rev_volume = (u32::from(PLAYER_VEHICLE_ENGINE_VOLUME) * accel / 60 + 20) as u8;
        let frequency = 8500 * accel / 60 + 17000;
        add_player_car_sample(
            audio,
            snapshot,
            idle_volume,
            frequency,
            SFX_CESNA_IDLE as u32,
            SFX_BANK_0,
            52,
            true,
        );
        add_player_car_sample(
            audio,
            snapshot,
            rev_volume,
            frequency,
            SFX_CESNA_REV as u32,
            SFX_BANK_0,
            2,
            true,
        );
        return true;
    }
    if is_dodo {
        add_player_car_sample(
            audio,
            snapshot,
            PLAYER_VEHICLE_ENGINE_VOLUME + 20,
            17000,
            SFX_CESNA_IDLE as u32,
            SFX_BANK_0,
            52,
            true,
        );
        return true;
    }
    if snapshot.distance_squared >= CESNA_IDLE_MAX_DIST * CESNA_IDLE_MAX_DIST {
        return false;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    for voice in [
        CesnaLoopVoice {
            sample: SFX_CESNA_IDLE as u32,
            counter: 52,
            frequency: 12500,
            max_distance: CESNA_IDLE_MAX_DIST,
            frames: 8.0,
        },
        CesnaLoopVoice {
            sample: SFX_CESNA_REV as u32,
            counter: 2,
            frequency: 25000,
            max_distance: CESNA_REV_MAX_DIST,
            frames: 4.0,
        },
    ] {
        if distance < voice.max_distance {
            let volume = audio.ComputeVolume(CESNA_VOLUME, voice.max_distance, distance);
            if volume > 0 {
                queue_cesna_loop(audio, snapshot, voice, volume);
            }
        }
    }
    true
}

struct CesnaLoopVoice {
    sample: u32,
    counter: u32,
    frequency: u32,
    max_distance: f32,
    frames: f32,
}

/// ProcessCesna's non-Dodo positional loops use authored SDT offsets.
fn queue_cesna_loop(
    audio: &mut cAudioManager,
    source: &VehicleAudioSnapshot,
    voice: CesnaLoopVoice,
    volume: u8,
) {
    if !audio.has_sample_metadata(voice.sample) {
        bevy::log::warn_once!(
            sample = voice.sample,
            "cesna_audio_missing_original_sample_metadata"
        );
        return;
    }
    let loop_start = audio.sample_loop_start(voice.sample);
    let loop_end = audio.sample_loop_end(voice.sample);
    let output = &mut audio.m_sQueueSample;
    output.m_nEntityIndex = source.audio_entity;
    output.m_nCounter = voice.counter;
    output.m_nSampleIndex = voice.sample;
    output.m_nBankIndex = SFX_BANK_0;
    output.m_bIs2D = false;
    output.m_nPriority = 0;
    output.m_nFrequency = voice.frequency;
    output.m_nVolume = volume;
    output.m_nEmittingVolume = CESNA_VOLUME;
    output.m_nLoopCount = 0;
    output.m_nLoopStart = loop_start;
    output.m_nLoopEnd = loop_end;
    output.m_fSpeedMultiplier = 8.0;
    output.m_MaxDistance = voice.max_distance;
    output.m_bStatic = false;
    output.m_vecPos = source.position;
    output.m_bReverb = true;
    output.m_nFramesToPlay = voice.frames;
    audio.AddSampleToRequestedQueue();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioLogic::{
        PLAYER_VEHICLE_ENGINE_VOLUME, VehicleAudioSnapshot, process_cesna,
    };

    #[test]
    fn channel_binding_stops_previous_voice_without_resetting_cesna_static() {
        let mut audio = cAudioManager::default();
        audio.Initialise();
        let mut shared = PlayerEngineAudioState::default();
        shared.bind(&mut audio, Some(7));
        shared.cesna.accel_counter = 38;
        shared.engine.cruising = 99;
        shared.engine.player_just_entered_car = false;
        shared.bind(&mut audio, Some(8));
        assert!(audio.stopped_voice_keys.contains(&(7, PLAYER_ENGINE_VOICE_COUNTER)));
        assert_eq!(shared.cesna.accel_counter, 38);
        assert_eq!(shared.engine.cruising, 0);
        assert!(shared.engine.player_just_entered_car);
        shared.bind(&mut audio, Some(8));
        assert_eq!(shared.channel_rebindings, 2);
        shared.bind(&mut audio, None);
        assert!(audio.stopped_voice_keys.contains(&(8, PLAYER_ENGINE_VOICE_COUNTER)));
    }

    #[test]
    fn replacing_focused_entity_resets_engine_even_when_audio_slot_is_reused() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let mut audio = cAudioManager::default();
        audio.Initialise();
        let mut shared = PlayerEngineAudioState::default();
        shared.bind_actor(&mut audio, Some(first), Some(7));
        shared.engine.player_just_entered_car = false;
        shared.cesna.accel_counter = 30;
        world.despawn(first);
        let replacement = world.spawn_empty().id();
        shared.bind_actor(&mut audio, Some(replacement), Some(7));
        assert!(shared.engine.player_just_entered_car);
        assert!(audio.stopped_voice_keys.contains(&(7, PLAYER_ENGINE_VOICE_COUNTER)));
        assert_eq!(shared.cesna.accel_counter, 30);
        assert_eq!(shared.bound_actor, Some(replacement));
    }

    #[test]
    fn cesna_full_native_ramp_survives_player_vehicle_switch_and_npc_processing() {
        let mut audio = cAudioManager::default();
        audio.Initialise();
        audio.SetSampleMetadata(&vec![
            crate::audio::sampman::SampleMetadata {
                frequency: 22050,
                loop_start: 13,
                loop_end: 456,
                ..Default::default()
            };
            crate::audio::AudioSamples::TOTAL_AUDIO_SAMPLES
                as usize
        ]);
        let mut shared = PlayerEngineAudioState::default();
        let mut source = VehicleAudioSnapshot {
            audio_entity: 7,
            ..Default::default()
        };
        shared.bind(&mut audio, Some(7));
        for expected in 1..=60 {
            audio.m_nRequestedCount[audio.m_nActiveQueue] = 0;
            assert!(process_cesna(
                &mut audio,
                &source,
                true,
                true,
                true,
                &mut shared.cesna
            ));
            assert_eq!(shared.cesna.accel_counter, expected);
            let output = &audio.m_sQueueSample;
            assert_eq!(output.m_nFrequency, 8500 * u32::from(expected) / 60 + 17000);
            assert_eq!(
                output.m_nVolume,
                (u32::from(PLAYER_VEHICLE_ENGINE_VOLUME) * u32::from(expected) / 60 + 20) as u8
            );
        }
        shared.bind(&mut audio, Some(8));
        source.audio_entity = 8;
        process_cesna(&mut audio, &source, false, false, false, &mut shared.cesna);
        assert_eq!(shared.cesna.accel_counter, 60);
        audio.Initialise();
        for expected in (0..60).rev() {
            audio.m_nRequestedCount[audio.m_nActiveQueue] = 0;
            process_cesna(&mut audio, &source, true, true, false, &mut shared.cesna);
            assert_eq!(shared.cesna.accel_counter, expected);
        }
    }
}
