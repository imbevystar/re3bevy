//! Native boat control over the existing script-vehicle ECS component.

use crate::{
    collision::Collision::CCollision,
    control::{
        AutoPilot::eCarMission,
        ScriptEntities::ScriptGameplayBridge,
    },
    core::{FileLoader::CFileLoader, Pad::CPads},
    entities::Entity::eEntityStatus,
    renderer::{WaterLevel::CWaterLevel, Weather::CWeather},
    vehicles::{
        Boat::INVALID_ORIENTATION,
        Floater::process_buoyancy,
        HandlingMgr::cHandlingDataMgr,
    },
};
use bevy::prelude::*;
use re3bevy_core::core::{Coordinates::CVector, Timer::CTimer};

#[derive(Default, Resource)]
pub struct BoatControlDiagnostics {
    pub waiting_for_collision: u64,
    pub waiting_for_handling: u64,
    pub waiting_for_model: u64,
    pub waiting_for_water: u64,
    pub controlled_boats: u64,
}

/// CCarCtrl::SteerAIBoatWithPhysics and HeadingForTarget.
fn steer_ai_boat<BoatCore, BoatVitality, BoatState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(boat: &mut crate::vehicles::Boat::BoatRuntimeView<BoatCore, BoatVitality, BoatState, Drive, Appearance, Entry, Audio>)
where
    BoatCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    BoatVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    BoatState: std::ops::DerefMut<Target = crate::vehicles::Boat::BoatState>,
{
    match boat.vehicle.core.auto_pilot.car_mission {
        eCarMission::MISSION_GOTOCOORDS_ASTHECROWSWIMS => {
            let matrix = boat.vehicle.core.physical.entity.matrix;
            let destination = boat.vehicle.core.auto_pilot.destination_coors;
            let angle_target =
                (destination.y - matrix.position.y).atan2(destination.x - matrix.position.x);
            let angle_forward = matrix.forward.y.atan2(matrix.forward.x);
            let difference = (angle_target - angle_forward + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            boat.steering_left_right = difference.clamp(-0.5, 0.5);
            let speed = boat.vehicle.core.physical.move_speed;
            let current_speed = (speed.x * speed.x + speed.y * speed.y).sqrt();
            let cruise = boat.vehicle.core.auto_pilot.cruise_speed as f32;
            let speed_difference = cruise - current_speed * 60.0;
            boat.accelerate = if speed_difference > 0.0 {
                let remaining = speed_difference / cruise;
                if remaining > 0.25 {
                    1.0
                } else {
                    1.0 - (0.25 - remaining) * 4.0
                }
            } else if speed_difference < -5.0 {
                -0.2
            } else {
                -0.1
            };
            boat.brake = 0.0;
        }
        eCarMission::MISSION_NONE => {
            boat.steering_left_right = 0.0;
            boat.accelerate = 0.0;
            boat.brake = 0.0;
        }
        _ => {}
    }
    boat.vehicle.drive.steer_angle = boat.steering_left_right;
    boat.vehicle.drive.gas_pedal = boat.accelerate;
    boat.vehicle.drive.brake_pedal = boat.brake;
    boat.vehicle.drive.is_handbrake_on = false;
}

/// CBoat::ProcessControl force/control phase, before world collision dispatch.
pub(crate) fn process_boat_control(
    mut vehicles: crate::control::ScriptEntities::ScriptVehicleStore,
    timer: Res<CTimer>,
    pads: Res<CPads>,
    bridge: Res<ScriptGameplayBridge>,
    handling: Res<cHandlingDataMgr>,
    loader: Res<CFileLoader>,
    collision: Res<CCollision>,
    water: Option<Res<CWaterLevel>>,
    weather: Res<CWeather>,
    mut diagnostics: ResMut<BoatControlDiagnostics>,
    mut general: ResMut<re3bevy_core::core::General::CGeneral>,
    mut particles: ResMut<crate::renderer::Particle::CParticle>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    timecycle: Res<crate::renderer::Timecycle::CTimeCycle>,
    camera: Res<crate::core::Camera::CCamera>,
    mut audio: ResMut<crate::audio::DMAudio::cDMAudio>,
) {
    let step = timer.time_step();
    for mut vehicle in vehicles.values_mut() {
        let crate::control::ScriptEntities::ScriptVehicleMut::Boat { boat, .. } = &mut vehicle else {
            continue;
        };
        let zone = boat.vehicle.core.physical.zone_level;
        if zone > crate::collision::ColModel::LEVEL_GENERIC as i8
            && zone != collision.collision_in_memory as i8
        {
            diagnostics.waiting_for_collision += 1;
            continue;
        }
        let Some(data) = boat
            .vehicle.core.handling_id
            .and_then(|id| usize::try_from(id).ok())
            .and_then(|id| handling.get_handling_data(id))
        else {
            diagnostics.waiting_for_handling += 1;
            continue;
        };
        boat.prune_wake_trail(step);
        let colours = crate::vehicles::BoatEffects::boat_water_colours(&timecycle, &mut general);
        boat.vehicle.audio.process_car_alarm(timer.time_step_in_milliseconds());
        let status = boat.vehicle.core.physical.entity.status;
        let on_land = boat.vehicle.core.physical.damage.impulse > 0.0
            && boat.vehicle.core.physical.damage.normal.z > 0.1;
        match status {
            eEntityStatus::STATUS_PLAYER => {
                boat.is_anchored = false;
                boat.orientation = INVALID_ORIENTATION;
                boat.process_control_inputs(pads.get_pad(0), 0, data.steering_lock);
                // TODO(parity): CBoat::ProcessControl / CVehicle::DoFixedMachineGuns.
            }
            eEntityStatus::STATUS_SIMPLE => {
                boat.is_anchored = false;
                boat.orientation = INVALID_ORIENTATION;
                boat.vehicle.core.physical.process_control_pre_physics(step);
                boat.boat_in_water = true;
                boat.propeller_in_water = true;
                boat.vehicle.core.physical.flags.is_in_water = true;
                continue;
            }
            eEntityStatus::STATUS_PHYSICS => {
                boat.is_anchored = false;
                boat.orientation = INVALID_ORIENTATION;
                steer_ai_boat(boat);
            }
            eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED => {
                boat.boat_in_water = true;
                boat.propeller_in_water = true;
                boat.vehicle.core.physical.flags.is_in_water = true;
                boat.vehicle.drive.steer_angle = 0.0;
                boat.vehicle.drive.is_handbrake_on = false;
                boat.vehicle.drive.brake_pedal = 0.5;
                boat.vehicle.drive.gas_pedal = 0.0;
                let position = boat.vehicle.core.physical.entity.get_position();
                let delta = CVector::new(
                    position.x - bridge.player_position.x,
                    position.y - bridge.player_position.y,
                    position.z - bridge.player_position.z,
                );
                if delta.magnitude() > 150.0 {
                    boat.vehicle.core.physical.move_speed = CVector::ZERO;
                    boat.vehicle.core.physical.turn_speed = CVector::ZERO;
                    continue;
                }
            }
            _ => {}
        }
        let collision_damage =
            data.collision_damage_multiplier * boat.vehicle.core.physical.damage.impulse;
        if collision_damage > 25.0
            && status != eEntityStatus::STATUS_WRECKED
            && boat.vehicle.vitality.health >= 150.0
            && !boat.vehicle.core.physical.entity.flags.collision_proof
        {
            let previous_health = boat.vehicle.vitality.health;
            let divisor = match (
                status == eEntityStatus::STATUS_PLAYER,
                boat.vehicle.vitality.take_less_damage,
            ) {
                (true, true) => 6.0,
                (true, false) => 2.0,
                (false, true) => 12.0,
                (false, false) => 4.0,
            };
            boat.vehicle.vitality.health -= (collision_damage - 25.0) / divisor;
            // TODO(parity): CBoat::ProcessControl driver SOUND_PED_ANNOYED_DRIVER
            // and CBoat::BlowUpCar CDarkel::RegisterKillByPlayer/next-process destruction.
            if boat.vehicle.vitality.health <= 0.0 && previous_health > 0.0 {
                boat.vehicle.vitality.health = 1.0;
                boat.set_on_fire_entity = boat.vehicle.core.physical.damage.entity;
            }
        }
        boat.process_damage_particles(
            camera.position,
            timer.time_step_in_milliseconds() as f32,
            timer.time_in_milliseconds(),
            timer.frame_counter(),
            &mut general,
            &mut particles.emitter(&particle_systems, camera.position),
        );
        boat.vehicle.core.physical.process_control_pre_physics(step);
        let model = boat.vehicle.core.physical.entity.model_index.max(0) as usize;
        let Some(col) = loader.model_info.col_model(model) else {
            diagnostics.waiting_for_model += 1;
            continue;
        };
        let Some(water) = water.as_deref() else {
            diagnostics.waiting_for_water += 1;
            continue;
        };
        let minimum = CVector::new(
            col.bounding_box.min.x,
            col.bounding_box.min.y,
            col.bounding_box.min.z,
        );
        let maximum = CVector::new(
            col.bounding_box.max.x,
            col.bounding_box.max.y,
            col.bounding_box.max.z,
        );
        if let Some(result) = process_buoyancy(
            &boat.vehicle.core.physical,
            minimum,
            maximum,
            data.buoyancy,
            water,
            timer.time_in_milliseconds(),
            weather.wind,
            step,
            true,
            None,
        ) {
            let effects = boat.apply_buoyancy_response(
                result,
                data,
                step,
                on_land,
                water,
                timer.time_in_milliseconds(),
                weather.wind,
            );
            crate::vehicles::BoatEffects::emit_boat_water_effects(
                boat,
                effects,
                minimum,
                maximum,
                &mut crate::vehicles::BoatEffects::BoatEffectContext {
                    particles: particles.emitter(&particle_systems, camera.position),
                    general: &mut general,
                    audio: &mut audio,
                    water,
                    colours,
                    camera: &camera,
                    frame: timer.frame_counter(),
                    time_ms: timer.time_in_milliseconds(),
                    wind: weather.wind,
                },
            );
        } else {
            boat.boat_in_water = false;
            boat.vehicle.core.physical.flags.is_in_water = false;
        }
        boat.apply_anchor();
        boat.process_delayed_explosion(step, timer.time_in_milliseconds());
        boat.vehicle.core.physical.apply_move_speed(step);
        boat.vehicle.core.physical.apply_turn_speed(step);
        diagnostics.controlled_boats += 1;
        // The shared ECS lifecycle consumes the boat's pending explosion.
    }
}
