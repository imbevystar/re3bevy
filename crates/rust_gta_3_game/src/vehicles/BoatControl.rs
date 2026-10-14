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
use rust_gta_3_core::core::{Coordinates::CVector, Timer::CTimer};

/// re3 `CVehicle::DoFixedMachineGuns` queues the two rounds during boat
/// control. Collision and damage are resolved after the mutable vehicle pass.
#[derive(Clone, Copy)]
struct PredatorRound {
    boat_handle: i32,
    source: CVector,
    target: CVector,
}

/// Keep the nearest collision, recording the actual player slot only when the
/// player is closer than the world/vehicle/ped hit in front of them.
fn keep_closer_predator_player_hit(
    hit: &mut Option<crate::weapons::WeaponRaycast::WeaponRayHit>,
    hit_player_slot: &mut Option<i32>,
    slot: i32,
    distance: f32,
    point: CVector,
) {
    use crate::weapons::WeaponRaycast::WeaponRayHit;
    if hit.is_none_or(|previous| distance < previous.distance()) {
        *hit = Some(WeaponRayHit::Player { distance, point });
        *hit_player_slot = Some(slot);
    }
}

/// Native 150ms cadence and 20-round magazine with a 1400ms reload delay.
/// `gun_firing_time` is also the reload gate in the original vehicle state.
fn predator_gun_trigger(ammo: &mut u8, next_fire: &mut u32, disabled: bool, pressed: bool, now: u32) -> bool {
    if pressed && !disabled {
        if now > next_fire.wrapping_add(150) {
            *next_fire = now;
            *ammo = ammo.saturating_sub(1);
            if *ammo == 0 {
                *ammo = 20;
                *next_fire = now.wrapping_add(1400);
            }
            return true;
        }
    } else if now > next_fire.wrapping_add(1400) {
        *ammo = 20;
    }
    false
}

/// CWeapon::DoTankDoomAiming: aim the fixed horizontal gun towards the
/// nearest eligible vehicle's height, retaining its original horizontal line.
fn predator_doom_aiming(
    shooter: i32,
    source: CVector,
    mut target: CVector,
    actors: &impl crate::weapons::WeaponRaycast::WeaponRaycastActors,
    loader: &CFileLoader,
) -> CVector {
    let length = (target - source).magnitude();
    if length <= 0.0 { return target; }
    let slope = (target.z - source.z) / length;
    let source_vehicle_position = actors.ray_vehicles().find_map(|(handle, entity)|
        (handle == shooter).then_some(entity.get_position()));
    let Some(shooter_position) = source_vehicle_position else { return target; };
    let dx = target.x - source.x;
    let dy = target.y - source.y;
    let horizontal_length_sq = dx * dx + dy * dy;
    if horizontal_length_sq <= 0.0 { return target; }
    let mut best_distance = 9000.0_f32;
    let mut best_height = None;
    for (handle, entity) in actors.ray_vehicles() {
        if handle == shooter || matches!(entity.status,
            eEntityStatus::STATUS_TRAIN_MOVING | eEntityStatus::STATUS_TRAIN_NOT_MOVING
            | eEntityStatus::STATUS_HELI | eEntityStatus::STATUS_PLANE) { continue; }
        let position = entity.get_position();
        let planar = ((position.x - shooter_position.x).powi(2)
            + (position.y - shooter_position.y).powi(2)).sqrt();
        if planar > length { continue; }
        let vertical = (shooter_position.z - (planar * slope + position.z)).abs();
        if 3.0 * vertical >= planar { continue; }
        let Some(col) = loader.model_info.col_model(entity.model_index.max(0) as usize) else { continue; };
        let along = ((position.x - source.x) * dx + (position.y - source.y) * dy)
            / horizontal_length_sq;
        let t = along.clamp(0.0, 1.0);
        let offset_x = position.x - (source.x + dx * t);
        let offset_y = position.y - (source.y + dy * t);
        if (offset_x * offset_x + offset_y * offset_y).sqrt() >= col.bounding_sphere.radius * 3.0 { continue; }
        let distance = (planar * planar + vertical * vertical).sqrt();
        if distance < best_distance {
            best_distance = distance;
            best_height = Some(position);
        }
    }
    if let Some(position) = best_height {
        let dist_to_source = ((position.x - source.x).powi(2)
            + (position.y - source.y).powi(2)).sqrt();
        if dist_to_source > 0.0 {
            let dist_to_target = horizontal_length_sq.sqrt();
            target.z = (dist_to_target / dist_to_source) * (0.3 + position.z - source.z) + source.z;
        }
    }
    target
}

#[derive(Default, Resource)]
pub struct BoatControlDiagnostics {
    pub waiting_for_collision: u64,
    pub waiting_for_handling: u64,
    pub waiting_for_model: u64,
    pub waiting_for_water: u64,
    pub controlled_boats: u64,
    predator_rounds: Vec<PredatorRound>,
    driver_collision_speech: Vec<i32>,
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
    mut general: ResMut<rust_gta_3_core::core::General::CGeneral>,
    mut particles: ResMut<crate::renderer::Particle::CParticle>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    timecycle: Res<crate::renderer::Timecycle::CTimeCycle>,
    camera: Res<crate::core::Camera::CCamera>,
    mut audio: ResMut<crate::audio::DMAudio::cDMAudio>,
) {
    let step = timer.time_step();
    diagnostics.predator_rounds.clear();
    diagnostics.driver_collision_speech.clear();
    for (handle, mut vehicle) in vehicles.iter_mut() {
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
                if boat.vehicle.core.physical.entity.model_index as usize
                    == crate::modelinfo::ModelIndices::MI_PREDATOR
                {
                    let core = &mut boat.vehicle.core;
                    if predator_gun_trigger(
                        &mut core.ammo_in_clip, &mut core.gun_firing_time,
                        core.flags.gun_switched_off, pads.get_pad(0).get_car_gun_fired(),
                        timer.time_in_milliseconds(),
                    ) {
                        let entity = &core.physical.entity;
                        let forward = entity.get_forward();
                        let length = (forward.x * forward.x + forward.y * forward.y).sqrt().max(0.1);
                        let direction = CVector::new(forward.x / length, forward.y / length, 0.0);
                        for x in [2.0_f32, -2.0] {
                            let source = entity.transform_point(CVector::new(x, 2.5, 1.0));
                            let target = source + direction * 60.0 + CVector::new(
                                ((general.get_random_number() & 255) as i32 - 128) as f32 * 0.015,
                                ((general.get_random_number() & 255) as i32 - 128) as f32 * 0.015,
                                ((general.get_random_number() & 255) as i32 - 128) as f32 * 0.02,
                            );
                            diagnostics.predator_rounds.push(PredatorRound { boat_handle: handle, source, target });
                        }
                        audio.PlayOneShot(core.physical.audio_entity_id, crate::audio::soundlist::SOUND_WEAPON_SHOT_FIRED as u16, 0.0);
                    }
                }
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
            // re3 CBoat::ProcessControl: a non-player boat's driver complains
            // on strong impacts, before the damage/fire transition. The
            // canonical InVehicle relation is resolved after this boat pass.
            if status != eEntityStatus::STATUS_PLAYER && collision_damage > 60.0 {
                diagnostics.driver_collision_speech.push(handle);
            }
            // TODO(parity): CBoat::BlowUpCar CDarkel::RegisterKillByPlayer/next-process destruction.
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

/// Deliver collision reactions through the normal CPed::Say priority queue,
/// using the live driver relationship rather than copying occupant handles
/// into vehicle state.
pub(crate) fn process_boat_driver_collision_speech(
    mut diagnostics: ResMut<BoatControlDiagnostics>,
    index: Res<crate::control::ScriptEntities::ScriptEntityIndex>,
    mut drivers: Query<
        (&mut crate::peds::Ped::CPed, &crate::peds::VehicleRelations::InVehicle),
        Without<crate::peds::PlayerPed::PlayerPedState>,
    >,
) {
    use crate::audio::soundlist::SOUND_PED_ANNOYED_DRIVER;
    for handle in diagnostics.driver_collision_speech.drain(..) {
        let Some(vehicle) = index.vehicle(handle) else { continue; };
        for (mut ped, relation) in &mut drivers {
            if relation.vehicle == vehicle && relation.is_driver() {
                crate::control::ScriptSnapshots::queue_ped_script_say(
                    &mut ped, SOUND_PED_ANNOYED_DRIVER as i16,
                );
                break;
            }
        }
    }
}

/// FireOneInstantHitRound: resolve Predator gun rounds against the same live
/// collision actors used by other weapons, outside the mutable boat pass.
#[allow(clippy::too_many_arguments)]
pub(crate) fn process_predator_gun_rounds(
    mut diagnostics: ResMut<BoatControlDiagnostics>,
    mut actors: crate::weapons::WeaponActorAccess::WeaponActorAccess,
    mut players: crate::peds::PlayerRead::CWorldPlayersCombat,
    occupants: Query<
        (
            &crate::peds::VehicleRelations::InVehicle,
            Option<&crate::control::ScriptEntities::ScriptPedIdentity>,
        ),
        With<crate::peds::VehicleRelations::PedEntity>,
    >,
    loader: Res<CFileLoader>,
    ped_stats: Res<crate::peds::PedType::CPedStatsRuntime>,
    timer: Res<CTimer>,
    mut particles: ResMut<crate::renderer::Particle::CParticle>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    mut general: ResMut<rust_gta_3_core::core::General::CGeneral>,
    mut audio: ResMut<crate::audio::DMAudio::cDMAudio>,
    mut point_lights: Option<ResMut<crate::renderer::PointLights::CPointLights>>,
    camera: Res<crate::core::Camera::CCamera>,
) {
    use crate::peds::PlayerPed::{
        apply_ped_gun_hit_reaction, apply_player_gun_hit_reaction, emit_bullet_surface_impact,
        emit_weapon_blood, ped_defend_weakness,
    };
    use crate::renderer::ParticleType::{PARTICLE_GUNFLASH, PARTICLE_HELI_ATTACK};
    use crate::weapons::{
        WeaponRaycast::{closest_weapon_ray_hit, weapon_ped_model_hit, WeaponRayHit},
        WeaponType::eWeaponType,
    };
    use crate::audio::soundlist::{SOUND_WEAPON_HIT_PED, SOUND_WEAPON_HIT_VEHICLE};

    let now = timer.time_in_milliseconds();
    let frame = timer.frame_counter();
    for round in diagnostics.predator_rounds.drain(..) {
        let mut emitter = particles.emitter(&particle_systems, camera.position);
        emitter.add_particle(PARTICLE_GUNFLASH, round.source, CVector::ZERO, None, 0.0, 0, 0, 0, 0);
        emitter.add_particle(PARTICLE_HELI_ATTACK, round.source, (round.target - round.source) * 0.15, None, 0.0, 0, 0, 0, 0);
        if let Some(lights) = point_lights.as_deref_mut() {
            lights.add_light(
                crate::renderer::PointLights::LIGHT_POINT, round.source, CVector::ZERO,
                5.0, 1.0, 0.8, 0.0, crate::renderer::PointLights::FOG_NONE,
                false, camera.position,
            );
        }
        let target = predator_doom_aiming(round.boat_handle, round.source, round.target, &actors, &loader);
        let delta = target - round.source;
        let range = delta.magnitude();
        if range <= 0.0 { continue; }
        // re3 ProcessLineOfSight includes the world player as well as NPCs.
        // Trace the world once, then compare the canonical player hit models
        // (including co-op) against that nearest collision. Exclude occupants
        // of the firing boat, which the original shooter/driver filter ignores.
        let mut hit_player_slot = None;
        let firing_vehicle = actors.vehicle_entity(round.boat_handle);
        let firing_driver = firing_vehicle.and_then(|vehicle| {
            occupants.iter().find_map(|(relation, identity)| {
                (relation.vehicle == vehicle && relation.is_driver())
                    .then(|| identity.map(|identity| identity.handle))
                    .flatten()
            })
        });
        let mut hit = closest_weapon_ray_hit(
            round.source, delta * (1.0 / range), range, None, firing_driver,
            Some(round.boat_handle), &actors, &loader,
        );
        for slot in 0..players.slot_count() {
            let slot = slot as i32;
            if players.relation(slot).is_some_and(|relation|
                Some(relation.vehicle) == firing_vehicle)
            {
                continue;
            }
            let Some(player) = players.player(slot) else { continue; };
            let model = if player.ped.physical.entity.model_index <= 0 {
                crate::modelinfo::ModelIndices::MI_PLAYER
            } else {
                player.ped.physical.entity.model_index as usize
            };
            if let Some((distance, point, _)) = weapon_ped_model_hit(
                round.source, target, player.ped, player.animation, model, &loader,
            ) {
                keep_closer_predator_player_hit(
                    &mut hit, &mut hit_player_slot, slot, distance, point,
                );
            }
        }
        match hit {
            Some(WeaponRayHit::Ped { handle, point, .. }) => {
                if let Some(victim) = actors.ped_mut(handle) {
                    if victim.ped.dying_or_dead() { continue; }
                    let position = victim.ped.physical.entity.get_position();
                    let direction = apply_ped_gun_hit_reaction(
                        &mut *victim.ped, victim.locomotion.rotation_cur, &*victim.vitality,
                        eWeaponType::WEAPONTYPE_UZI, round.source, now, frame,
                    );
                    let weakness = ped_defend_weakness(&*victim.ped, Some(&ped_stats));
                    victim.ped.inflict_weapon_damage(
                        victim.vitality, eWeaponType::WEAPONTYPE_UZI, 15.0,
                        weakness, false, true, now, direction,
                    );
                    emit_weapon_blood(&mut emitter, point, round.source, position, 4);
                    audio.PlayOneShot(victim.ped.physical.audio_entity_id, SOUND_WEAPON_HIT_PED as u16, 1.0);
                }
            }
            Some(WeaponRayHit::Player { point, .. }) => {
                if let Some(mut victim) = hit_player_slot.and_then(|slot| players.player_mut(slot)) {
                    if victim.ped.dying_or_dead() { continue; }
                    let position = victim.position();
                    let weakness = ped_defend_weakness(&*victim.ped, Some(&ped_stats));
                    let direction = apply_player_gun_hit_reaction(
                        &mut victim, eWeaponType::WEAPONTYPE_UZI, round.source, now, frame,
                    );
                    victim.ped.inflict_weapon_damage(
                        victim.vitality, eWeaponType::WEAPONTYPE_UZI, 15.0,
                        weakness, false, victim.can_be_damaged, now, direction,
                    );
                    emit_weapon_blood(&mut emitter, point, round.source, position, 4);
                    audio.PlayOneShot(victim.ped.physical.audio_entity_id, SOUND_WEAPON_HIT_PED as u16, 1.0);
                }
            }
            Some(WeaponRayHit::Vehicle { handle, point, normal, .. }) => {
                if let Some((core, vitality)) = actors.vehicle_damage(handle) {
                    vitality.inflict_weapon_damage_from_source(
                        core, eWeaponType::WEAPONTYPE_UZI, 15.0, false,
                    );
                    emit_bullet_surface_impact(&mut emitter, point, normal, 4, &mut general);
                    audio.PlayOneShot(core.physical.audio_entity_id, SOUND_WEAPON_HIT_VEHICLE as u16, 1.0);
                }
            }
            Some(WeaponRayHit::Object { point, normal, .. })
            | Some(WeaponRayHit::Static { point, normal, .. }) => {
                emit_bullet_surface_impact(&mut emitter, point, normal, 4, &mut general);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod predator_gun_tests {
    use super::*;

    use crate::weapons::WeaponRaycast::WeaponRayHit;

    #[test]
    fn hard_boat_collision_queues_only_the_affected_npc_driver_voice() {
        let mut app = App::new();
        app.init_resource::<BoatControlDiagnostics>()
            .init_resource::<crate::control::ScriptEntities::ScriptEntityIndex>()
            .add_systems(Update, process_boat_driver_collision_speech);

        let boat = app.world_mut().spawn_empty().id();
        let other_boat = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<crate::control::ScriptEntities::ScriptEntityIndex>()
            .vehicles
            .insert(1, boat);

        let mut random = rust_gta_3_core::core::General::CGeneral::with_seed(1);
        let driver = app.world_mut().spawn((
            crate::peds::Ped::CPed::new(
                crate::peds::PedType::ePedType::PEDTYPE_CIVMALE, &mut random,
            ),
            crate::peds::VehicleRelations::InVehicle::driver(boat),
        )).id();
        let unrelated = app.world_mut().spawn((
            crate::peds::Ped::CPed::new(
                crate::peds::PedType::ePedType::PEDTYPE_CIVMALE, &mut random,
            ),
            crate::peds::VehicleRelations::InVehicle::driver(other_boat),
        )).id();

        app.world_mut()
            .resource_mut::<BoatControlDiagnostics>()
            .driver_collision_speech
            .push(1);
        app.update();
        assert_eq!(
            app.world().get::<crate::peds::Ped::CPed>(driver).unwrap().queued_sound,
            crate::audio::soundlist::SOUND_PED_ANNOYED_DRIVER as i16,
        );
        assert_eq!(
            app.world().get::<crate::peds::Ped::CPed>(unrelated).unwrap().queued_sound,
            crate::peds::Ped::SOUND_NO_SOUND,
        );
        assert!(app.world().resource::<BoatControlDiagnostics>().driver_collision_speech.is_empty());
    }

    #[test]
    fn predator_round_hits_nearest_player_only_if_no_earlier_world_contact() {
        let point = CVector::new(0.0, 0.0, 0.0);
        let mut hit = Some(WeaponRayHit::Static {
            entity_index: 0,
            distance: 8.0,
            point,
            normal: CVector::new(0.0, 0.0, 1.0),
        });
        let mut player_slot = None;
        keep_closer_predator_player_hit(&mut hit, &mut player_slot, 1, 12.0, point);
        assert!(matches!(hit, Some(WeaponRayHit::Static { .. })));
        assert_eq!(player_slot, None);
        keep_closer_predator_player_hit(&mut hit, &mut player_slot, 0, 5.0, point);
        assert_eq!(player_slot, Some(0));
        keep_closer_predator_player_hit(&mut hit, &mut player_slot, 1, 6.0, point);
        assert_eq!(player_slot, Some(0));
        keep_closer_predator_player_hit(&mut hit, &mut player_slot, 1, 3.0, point);
        assert_eq!(player_slot, Some(1));
        assert!(matches!(hit, Some(WeaponRayHit::Player { distance: 3.0, .. })));
    }

    #[test]
    fn predator_gun_waits_for_burst_and_reload_then_refills_after_idle() {
        let mut ammo = 20;
        let mut next_fire = 0;
        let mut trigger = |pressed, now, disabled| predator_gun_trigger(&mut ammo, &mut next_fire, disabled, pressed, now);
        assert!(!trigger(true, 150, false));
        assert!(trigger(true, 151, false));
        assert!(!trigger(true, 301, false));
        assert!(trigger(true, 302, false));
        ammo = 1;
        assert!(predator_gun_trigger(&mut ammo, &mut next_fire, false, true, 453));
        assert_eq!(ammo, 20);
        assert!(!predator_gun_trigger(&mut ammo, &mut next_fire, false, true, 2003));
        assert!(predator_gun_trigger(&mut ammo, &mut next_fire, false, true, 2004));
        ammo = 9;
        assert!(!predator_gun_trigger(&mut ammo, &mut next_fire, true, true, 4000));
        assert_eq!(ammo, 20);
    }
}
