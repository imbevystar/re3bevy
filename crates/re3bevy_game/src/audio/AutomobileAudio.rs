//! Native cAudioManager::ProcessVehicle automobile branch.
//! Actor audio-motion writes are independent of readonly wheel/door/damage inputs.
use super::*;

/// Native UpdateGasPedalAudio advances once per audio service, with fixed
/// attack/release increments rather than frame-time scaling.
pub(super) fn update_gas_pedal_audio(current: f32, gas_pedal: f32) -> f32 {
    let gas_pedal = gas_pedal.abs();
    if current < gas_pedal {
        (current + 0.09).min(gas_pedal)
    } else {
        (current - 0.07).max(gas_pedal)
    }
}

/// Native ProcessVehicle automobile dispatch borrows one actor motion owner
/// and readonly physical/wheel/door/damage inputs before manager service.
pub fn process_single_automobile_audio<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    dmaudio: &mut cDMAudio,
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    handling_data: &crate::vehicles::HandlingMgr::tHandlingData,
    _entity_handle: i32,
    camera_pos: CVector,
    wet_roads: f32,
    rain: f32,
    is_player: bool,
    player_accelerate_state: Option<i16>,
    player_engine_channel_used: Option<bool>,
    one_shots: &mut VehicleOneShotAudioState,
    player_engine: &mut PlayerEngineAudioState,
)
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let drive = &*car.vehicle.drive;
    let core = &*car.vehicle.core;
    let audio = &mut *car.vehicle.audio;
    let state = &*car.state;
    let motion = &mut *car.audio_motion;
    if core.physical.audio_entity_id < 0 {
        bevy::log::warn_once!("audio_vehicle_waiting_for_physical_identity");
        return;
    }
    let position = core.physical.entity.get_position();
    dmaudio.AudioManager.m_sQueueSample.m_nEntityIndex = core.physical.audio_entity_id;
    dmaudio.AudioManager.m_sQueueSample.m_vecPos = position;
    let dx = position.x - camera_pos.x;
    let dy = position.y - camera_pos.y;
    let dz = position.z - camera_pos.z;
    let velocity = core.physical.move_speed;
    let forward = core.physical.entity.get_forward();
    let mut velocity_change =
        velocity.x * forward.x + velocity.y * forward.y + velocity.z * forward.z;
    let previous_velocity = motion.velocity_change_for_audio;
    // re3 UpdateGasPedalAudio tracks throttle with fixed +0.09/-0.07 steps.
    motion.gas_pedal_audio = update_gas_pedal_audio(motion.gas_pedal_audio, drive.gas_pedal);
    let gear =
        usize::from(drive.current_gear).min(handling_data.transmission.gears.len() - 1);
    let Some(audio_model_index) = vehicle_audio_model_index(core, one_shots) else { return; };
    let model_index = core.physical.entity.model_index as usize;
    let status = if is_player {
        VehicleStatusForAudio::Player
    } else {
        match core.physical.entity.status {
            eEntityStatus::STATUS_PLAYER | eEntityStatus::STATUS_PLAYER_DISABLED => {
                VehicleStatusForAudio::Player
            }
            eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED => {
                VehicleStatusForAudio::Abandoned
            }
            eEntityStatus::STATUS_SIMPLE => VehicleStatusForAudio::Simple,
            _ => VehicleStatusForAudio::Other,
        }
    };
    if status == VehicleStatusForAudio::Simple {
        velocity_change = core.auto_pilot.max_traffic_speed * 0.02;
    }
    let mut snapshot = VehicleAudioSnapshot {
        audio_entity: core.physical.audio_entity_id,
        model_index: audio_model_index,
        position,
        distance_squared: dx * dx + dy * dy + dz * dz,
        distance_calculated: false,
        horn_timer: i32::from(audio.car_horn_timer),
        horn_pattern: usize::from(audio.car_horn_pattern),
        siren_or_alarm: audio.siren_or_alarm,
        alarm_on: audio.is_alarm_on(),
        status,
        engine_on: drive.engine_on,
        gas_pedal: drive.gas_pedal,
        gas_pedal_audio: motion.gas_pedal_audio,
        player_accelerate_state,
        previous_velocity_change_for_audio: previous_velocity,
        velocity_change,
        max_velocity: handling_data.transmission.max_velocity,
        current_gear: drive.current_gear,
        gear_shift_down_velocity: handling_data.transmission.gears[gear].shift_down_velocity,
        first_gear_shift_up_velocity: handling_data.transmission.gears[1].shift_up_velocity,
        max_reverse_velocity: handling_data.transmission.max_reverse_velocity,
        wheels_on_ground: state.wheels_on_ground,
        drive_wheels_on_ground: state.drive_wheels_on_ground,
        surface_touched: core.physical.surface_touched,
        wet_roads,
        skid_value: 0.0,
        move_speed_z: core.physical.move_speed.z,
        handbrake_on: drive.is_handbrake_on,
        drive_type: handling_data.transmission.drive_type,
        num_gears: handling_data.transmission.number_of_gears.max(1) as u8,
        wheel_states: state.wheel_state,
        wheel_missing: std::array::from_fn(|wheel| {
            state.damage.get_wheel_status(wheel as u8)
                == crate::vehicles::DamageManager::eWheelStatus::WHEEL_STATUS_MISSING as u32
        }),
        turn_speed_z: core.physical.turn_speed.z,
        player_engine_channel_used,
        doors: std::array::from_fn(|door| DoorAudioSnapshot {
            swinging: state.damage.get_door_status(door as u8)
                == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32,
            open_or_closed: state.doors[door].is_closed() || state.doors[door].is_fully_open(),
            angular_velocity: state.doors[door].angular_velocity,
        }),
        ..Default::default()
    };

    // re3 gives the RC Bandit its own short path: model-car engine and
    // vehicle one-shots only, followed by the common rain pass.
    if model_index == MI_RCBANDIT {
        process_model_car_engine(
            &mut dmaudio.AudioManager,
            &snapshot,
            &mut motion.gas_pedal_audio,
        );
        drain_vehicle_one_shots(
            &mut dmaudio.AudioManager,
            &snapshot,
            one_shots,
        );
        motion.velocity_change_for_audio = velocity_change;
        process_rain_on_vehicle(
            &mut dmaudio.AudioManager,
            &snapshot,
            rain,
            &mut audio.rain_audio_counter,
            &mut audio.rain_samples_counter,
        );
        return;
    }

    if !process_vehicle_road_noise(&mut dmaudio.AudioManager, &snapshot) {
        drain_vehicle_one_shots(
            &mut dmaudio.AudioManager,
            &snapshot,
            one_shots,
        );
        motion.velocity_change_for_audio = velocity_change;
        process_rain_on_vehicle(
            &mut dmaudio.AudioManager,
            &snapshot,
            rain,
            &mut audio.rain_audio_counter,
            &mut audio.rain_samples_counter,
        );
        return;
    }

    if model_index == MI_DODO {
        process_wet_road_noise(&mut dmaudio.AudioManager, &snapshot);
        process_vehicle_skidding(&mut dmaudio.AudioManager, &snapshot);
    } else {
        process_reverse_gear(
            &mut dmaudio.AudioManager,
            &snapshot,
            &mut motion.gas_pedal_audio,
        );
        snapshot.gas_pedal_audio = motion.gas_pedal_audio;
        process_wet_road_noise(&mut dmaudio.AudioManager, &snapshot);
        process_vehicle_skidding(&mut dmaudio.AudioManager, &snapshot);
        prepare_vehicle_horn(&dmaudio.AudioManager, audio, &mut snapshot);
        process_vehicle_horn(&mut dmaudio.AudioManager, &snapshot);
        process_vehicle_siren_or_alarm(&mut dmaudio.AudioManager, &snapshot);
        if uses_reverse_warning(snapshot.model_index) {
            process_vehicle_reverse_warning(&mut dmaudio.AudioManager, &snapshot);
        }
        if has_air_brakes(snapshot.model_index) {
            process_air_brakes(&mut dmaudio.AudioManager, &snapshot);
        }
    }
    process_car_bomb_tick(
        &mut dmaudio.AudioManager,
        &snapshot,
        snapshot.engine_on,
        matches!(
            state.bomb_type,
            crate::vehicles::Automobile::eBombType::CARBOMB_TIMEDACTIVE
        ),
    );
    if is_player {
        player_engine.bind(&mut dmaudio.AudioManager, Some(snapshot.audio_entity));
    }
    if is_player && core.physical.entity.status == eEntityStatus::STATUS_WRECKED {
        dmaudio.AudioManager.StopVoice(snapshot.audio_entity, crate::audio::DMAudio::PLAYER_ENGINE_VOICE_COUNTER);
    } else if snapshot.engine_on {
        if model_index == MI_DODO {
            process_cesna(
                &mut dmaudio.AudioManager,
                &snapshot,
                true,
                is_player,
                drive.gas_pedal > 0.0,
                &mut player_engine.cesna,
            );
        } else if is_player {
            process_players_vehicle_engine(
                &mut dmaudio.AudioManager,
                &snapshot,
                &mut player_engine.engine,
                &mut motion.gas_pedal_audio,
            );
        } else {
            process_vehicle_engine(&mut dmaudio.AudioManager, &snapshot, &mut motion.gas_pedal_audio);
        }
    }
    process_engine_damage(
        &mut dmaudio.AudioManager,
        &snapshot,
        snapshot.engine_on,
        engine_damage_audio_state(state.damage.get_engine_status()),
    );
    process_vehicle_doors(&mut dmaudio.AudioManager, &snapshot);
    drain_vehicle_one_shots(
        &mut dmaudio.AudioManager,
        &snapshot,
        one_shots,
    );
    motion.velocity_change_for_audio = velocity_change;
    process_rain_on_vehicle(
        &mut dmaudio.AudioManager,
        &snapshot,
        rain,
        &mut audio.rain_audio_counter,
        &mut audio.rain_samples_counter,
    );
}

