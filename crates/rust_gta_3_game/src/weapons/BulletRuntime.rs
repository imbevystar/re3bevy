//! CBulletInfo::Update at the ECS world, damage, event and presentation boundary.
use super::{BulletInfo::*, WeaponRuntime::scaled_vector};
use crate::{
    core::{
        EventList::{CEventList, EEventEntity, EEventType},
        FileLoader::CFileLoader,
        Stats::CStats,
    },
    peds::{
        Ped::CPed,
        PlayerPed::{WeaponRayHit, closest_weapon_ray_hit},
        PlayerRead::CWorldPlayersCombat,
    },
    renderer::{
        Particle::CParticle,
        ParticleType::{PARTICLE_BLOOD_SMALL, PARTICLE_SPARK},
    },
    weapons::WeaponActorAccess::WeaponActorAccess,
};
use bevy::prelude::*;
use rust_gta_3_core::core::{Coordinates::CVector, Timer::CTimer};

pub(crate) fn process_bullet_runtime(
    mut pool: ResMut<CBulletInfoPool>,
    timer: Res<CTimer>,
    loader: Res<CFileLoader>,
    mut world: WeaponActorAccess,
    mut players: CWorldPlayersCombat,
    mut particles: ResMut<CParticle>,
    mut events: ResMut<CEventList>,
    mut stats: ResMut<CStats>,
    mut audio: ResMut<crate::audio::DMAudio::cDMAudio>,
    mut general: ResMut<crate::core::General::CGeneral>,
    mut explosions: ResMut<super::Explosion::CExplosionPool>,
    mut glass: ResMut<crate::renderer::Glass::CGlass>,
    menu: Res<crate::core::Frontend::CMenuManager>,
    ped_stats: Res<crate::peds::PedType::CPedStatsRuntime>,
    camera: Res<crate::core::Camera::CCamera>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
) {
    let mut particles = particles.emitter(&particle_systems, camera.position);
    let now = timer.time_in_milliseconds();
    let intents = pool.Update(
        now,
        timer.time_step(),
        menu.prefs_allow_nasty_game,
        |source| {
            players.iter().any(|p| p.pool_handle as u32 == source)
                || world.ped(source as i32).is_some()
        },
        |trace| {
            let delta = trace.end - trace.start;
            let range = delta.magnitude();
            if range <= f32::EPSILON {
                return None;
            }
            let direction = scaled_vector(delta, 1.0 / range);
            let target = players
                .iter()
                .find(|p| Some(p.pool_handle as u32) != trace.source);
            let hit = closest_weapon_ray_hit(
                trace.start,
                direction,
                range,
                target,
                trace.source.map(|id| id as i32),
                None,
                &world,
                &loader,
            )?;
            let mut result = BulletHit {
                entity: u32::MAX,
                entity_type: BulletHitEntityType::Other,
                point: CVector::ZERO,
                normal: scaled_vector(direction, -1.0),
                piece: 0,
                ped_dying_or_dead: false,
                ped_in_control: false,
                ped_ducking: false,
                ped_los_hit: true,
                ped_is_cop: false,
                ped_position: CVector::ZERO,
                object_infinite_mass: true,
                object_static: true,
                object_uproot_limit: 0.0,
            };
            match hit {
                WeaponRayHit::Player { point, .. } => {
                    let player = target?;
                    result.entity = player.pool_handle as u32;
                    result.entity_type = BulletHitEntityType::Ped;
                    result.point = point;
                    fill_ped_hit(&mut result, &player.ped, player.vitality);
                }
                WeaponRayHit::Ped { handle, point, .. } => {
                    result.entity = handle as u32;
                    result.entity_type = BulletHitEntityType::Ped;
                    result.point = point;
                    let entity = world.ped(handle)?;
                    fill_ped_hit(&mut result, entity.ped, entity.vitality);
                }
                WeaponRayHit::Vehicle {
                    handle,
                    point,
                    normal,
                    ..
                } => {
                    result.entity = handle as u32;
                    result.entity_type = BulletHitEntityType::Vehicle;
                    result.point = point;
                    result.normal = normal;
                }
                WeaponRayHit::Object {
                    handle,
                    point,
                    normal,
                    ..
                } => {
                    result.entity = handle as u32;
                    result.entity_type = BulletHitEntityType::Object;
                    result.point = point;
                    result.normal = normal;
                    let object = world.object(handle)?;
                    result.object_static = object.physical.entity.flags.is_static;
                    result.object_infinite_mass = object.physical.flags.infinite_mass;
                    result.object_uproot_limit = object.uproot_limit;
                }
                WeaponRayHit::Static {
                    entity_index,
                    point,
                    normal,
                    ..
                } => {
                    result.entity = entity_index as u32;
                    result.entity_type = match loader.world.entities.get(entity_index)?.entity_type
                    {
                        crate::core::World::EEntityType::Building => BulletHitEntityType::Building,
                        crate::core::World::EEntityType::Dummy => BulletHitEntityType::Dummy,
                        _ => BulletHitEntityType::Other,
                    };
                    result.point = point;
                    result.normal = normal;
                }
            }
            // TODO(parity): CPed::DoesLOSBulletHitPed body-piece result and CVehicle tyre pieces.
            Some(result)
        },
    );
    for intent in intents {
        match intent {
            BulletUpdateIntent::IncrementInstantHits => stats.instant_hits_hit_by_player += 1,
            BulletUpdateIntent::ClearPedAttackAndPlayFrontHit { ped } => {
                if let Some(mut player) = players.iter_mut().find(|p| p.pool_handle as u32 == ped) {
                    crate::peds::PlayerPed::install_gun_hit_animation(&mut player.ped, 0, now);
                } else if let Some(victim) = world.ped_mut(ped as i32) {
                    crate::peds::PlayerPed::install_gun_hit_animation(&mut *victim.ped, 0, now);
                }
            }
            BulletUpdateIntent::DamagePed {
                ped,
                weapon,
                damage,
                hit_point,
                ..
            } => {
                if let Some(player) = players.iter_mut().find(|p| p.pool_handle as u32 == ped) {
                    let weakness =
                        crate::peds::PlayerPed::ped_defend_weakness(&player.ped, Some(&ped_stats));
                    let direction = player.ped.get_local_direction(
                        player.locomotion.rotation_cur,
                        player.ped.physical.entity.get_position() - hit_point,
                    );
                    player.ped.inflict_weapon_damage(
                        player.vitality,
                        weapon,
                        damage as f32,
                        weakness,
                        true,
                        player.can_be_damaged,
                        now,
                        direction,
                    );
                } else if let Some(victim) = world.ped_mut(ped as i32) {
                    let weakness =
                        crate::peds::PlayerPed::ped_defend_weakness(&victim.ped, Some(&ped_stats));
                    let direction = victim.ped.get_local_direction(
                        victim.locomotion.rotation_cur,
                        victim.ped.physical.entity.get_position() - hit_point,
                    );
                    victim.ped.inflict_weapon_damage(
                        victim.vitality,
                        weapon,
                        damage as f32,
                        weakness,
                        true,
                        true,
                        now,
                        direction,
                    );
                }
            }
            BulletUpdateIntent::RegisterShootEvent { ped, source, cop } => {
                let position = players
                    .iter()
                    .find(|p| p.pool_handle as u32 == ped)
                    .map(|p| p.position())
                    .or_else(|| {
                        world
                            .ped(ped as i32)
                            .map(|p| p.ped.physical.entity.get_position())
                    });
                if let Some(position) = position {
                    events.register_entity_event(
                        if cop {
                            EEventType::ShootCop
                        } else {
                            EEventType::ShootPed
                        },
                        EEventEntity::Ped,
                        ped as i32,
                        source.map(|s| s as i32),
                        position,
                        1000,
                        now,
                    );
                }
            }
            BulletUpdateIntent::DamageVehicle {
                vehicle,
                weapon,
                damage,
                ..
            } => {
                if let Some((core, vitality)) = world.vehicle_damage(vehicle as i32) {
                    vitality.inflict_weapon_damage_from_source(core, weapon, damage as f32, true);
                }
            }
            BulletUpdateIntent::SpawnBlood {
                point,
                direction,
                count,
            }
            | BulletUpdateIntent::SpawnSparks {
                point,
                direction,
                count,
            } => {
                let kind = if matches!(intent, BulletUpdateIntent::SpawnBlood { .. }) {
                    PARTICLE_BLOOD_SMALL
                } else {
                    PARTICLE_SPARK
                };
                for _ in 0..count {
                    particles.add_particle(kind, point, direction, None, 0.0, 0, 0, 0, 0);
                }
            }
            BulletUpdateIntent::UprootObject { object } => {
                if let Some(object) = world.object_mut(object as i32) {
                    object.physical.entity.flags.is_static = false;
                }
            }
            BulletUpdateIntent::ApplyObjectMoveForce { object, force } => {
                if let Some(object) = world.object_mut(object as i32) {
                    object.physical.apply_move_force(scaled_vector(
                        force,
                        crate::entities::Physical::GTA_PHYSICS_HZ,
                    ));
                }
            }
            BulletUpdateIntent::SniperHitSound {
                entity,
                entity_type,
                point,
            } => {
                let position = point;
                match entity_type {
                    BulletHitEntityType::Ped => {
                        let audio_id = players
                            .iter()
                            .find(|p| p.pool_handle as u32 == entity)
                            .map(|p| p.ped.physical.audio_entity_id)
                            .or_else(|| {
                                world
                                    .ped(entity as i32)
                                    .map(|p| p.ped.physical.audio_entity_id)
                            });
                        if let Some(id) = audio_id {
                            audio.PlayOneShot(
                                id,
                                crate::audio::soundlist::SOUND_WEAPON_HIT_PED as u16,
                                1.0,
                            );
                        }
                        if let Some(mut player) =
                            players.iter_mut().find(|p| p.pool_handle as u32 == entity)
                        {
                            queue_bullet_hit_voice(
                                &mut player.ped,
                                true,
                                now,
                                camera.position.z,
                                &mut general,
                            );
                        } else if let Some(victim) = world.ped_mut(entity as i32) {
                            queue_bullet_hit_voice(
                                &mut *victim.ped,
                                false,
                                now,
                                camera.position.z,
                                &mut general,
                            );
                        }
                    }
                    BulletHitEntityType::Vehicle => {
                        if let Some(vehicle) = world.vehicle_core(entity as i32) {
                            audio.PlayOneShot(
                                vehicle.physical.audio_entity_id,
                                crate::audio::soundlist::SOUND_WEAPON_HIT_VEHICLE as u16,
                                1.0,
                            );
                        }
                    }
                    BulletHitEntityType::Building
                    | BulletHitEntityType::Object
                    | BulletHitEntityType::Dummy => {
                        let position = if entity_type == BulletHitEntityType::Object {
                            world
                                .object(entity as i32)
                                .map(|object| object.physical.entity.get_position())
                                .unwrap_or(position)
                        } else {
                            loader
                                .world
                                .entities
                                .get(entity as usize)
                                .map(|entity| {
                                    CVector::new(
                                        entity.position[0],
                                        entity.position[1],
                                        entity.position[2],
                                    )
                                })
                                .unwrap_or(position)
                        };
                        let sound = if entity_type == BulletHitEntityType::Object {
                            crate::audio::soundlist::SCRIPT_SOUND_BULLET_HIT_GROUND_2
                        } else if entity_type == BulletHitEntityType::Dummy {
                            crate::audio::soundlist::SCRIPT_SOUND_BULLET_HIT_GROUND_3
                        } else {
                            crate::audio::soundlist::SCRIPT_SOUND_BULLET_HIT_GROUND_1
                        };
                        let mut object = crate::audio::AudioScriptObject::cAudioScriptObject::new(
                            sound as i16,
                            position,
                        );
                        audio.CreateOneShotScriptObject(&mut object);
                    }
                    BulletHitEntityType::Other => {}
                }
            }
            BulletUpdateIntent::GlassHit { entity, point } => {
                if let Some(object) = world.object_mut(entity as i32) {
                    super::WeaponRuntime::bullet_hit_glass_state(
                        point,
                        object,
                        &loader,
                        &mut glass,
                        &mut general,
                        now,
                    );
                }
            }
            BulletUpdateIntent::BlowUpExplosiveThings { entity } => {
                if let Some(object) = world.object_mut(entity as i32) {
                    super::WeaponRuntime::blow_up_explosive_object_state(
                        entity as i32,
                        players.focused().map(|p| p.pool_handle as u32),
                        object,
                        &loader,
                        &mut explosions,
                        &mut general,
                        now,
                    );
                }
            }
            // Moving bullets are created by FireSniper; FireAreaEffect owns flame fire propagation.
            BulletUpdateIntent::StartVehicleFire { .. } => {
                bevy::log::warn_once!(target: "rust_gta_3::weapons", "flame round entered sniper bullet pool");
            }
        }
    }
}

fn fill_ped_hit(hit: &mut BulletHit, ped: &CPed, vitality: &crate::peds::PedVitality::PedVitality) {
    hit.ped_position = ped.physical.entity.get_position();
    hit.ped_dying_or_dead = ped.dying_or_dead();
    hit.ped_in_control = ped.is_ped_in_control(vitality);
    hit.ped_ducking = ped.flags.is_ducking;
    hit.ped_is_cop = ped.ped_type == crate::peds::PedType::ePedType::PEDTYPE_COP;
}

/// CPed::Say's bullet-hit branch; queued phrases are consumed by ServiceTalking.
fn queue_bullet_hit_voice(
    ped: &mut CPed,
    player: bool,
    now: u32,
    camera_height: f32,
    general: &mut crate::core::General::CGeneral,
) {
    let sound = if player {
        crate::audio::soundlist::SOUND_PED_HIT as i16
    } else {
        crate::audio::soundlist::SOUND_PED_BULLET_HIT as i16
    };
    let timing = &crate::audio::PedTalk::PED_COMMENT_TIMING
        [(sound - crate::audio::soundlist::SOUND_PED_DEATH as i16) as usize];
    if !player && camera_height + 3.0 < ped.physical.entity.get_position().z {
        return;
    }
    if sound >= ped.queued_sound {
        return;
    }
    if sound == ped.last_queued_sound {
        let delay = general.get_random_number_in_range_i32(0, timing.repeat_random_delay) as u32;
        if ped
            .last_sound_start
            .wrapping_add(timing.repeat_delay)
            .wrapping_add(delay)
            > now
        {
            return;
        }
    }
    ped.queued_sound = sound;
}
