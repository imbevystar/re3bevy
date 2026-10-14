//! re3 reference: references/re-GTA/src/weapons/Explosion.cpp
//! re3 reference: references/re-GTA/src/weapons/Explosion.h
//!
//! Explosion slot state. Damage/audio/particle consumers read this resource.

use bevy::prelude::{Entity, MessageReader, ParamSet, Res, ResMut, Resource};
use crate::entities::EntityReference::{EntityReference, EntityReferenceAccess, EntityReferencesCleared};
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    weapons::ExplosionActorAccess::ExplosionActorAccess,
    core::{
        EventList::{CEventList, EEventEntity, EEventType},
        Fire::CFireManager,
        config::NUM_EXPLOSIONS,
    },
    entities::Entity::eEntityStatus,
    objects::Object::{CObjectDamageContext, CObjectGlobalState},
    peds::{
        Ped::PedState,
        PedType::CPedStatsRuntime,
        PlayerRead::CWorldPlayersCombat,
    },
    weapons::WeaponType::eWeaponType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eExplosionType {
    EXPLOSION_GRENADE = 0,
    EXPLOSION_MOLOTOV = 1,
    EXPLOSION_ROCKET = 2,
    EXPLOSION_CAR = 3,
    EXPLOSION_CAR_QUICK = 4,
    EXPLOSION_HELI = 5,
    EXPLOSION_MINE = 6,
    EXPLOSION_BARREL = 7,
    EXPLOSION_TANK_GRENADE = 8,
    EXPLOSION_HELI_BOMB = 9,
}

pub(crate) fn script_explosion_type(value: i32) -> Option<eExplosionType> {
    Some(match value {
        0 => eExplosionType::EXPLOSION_GRENADE,
        1 => eExplosionType::EXPLOSION_MOLOTOV,
        2 => eExplosionType::EXPLOSION_ROCKET,
        3 => eExplosionType::EXPLOSION_CAR,
        4 => eExplosionType::EXPLOSION_CAR_QUICK,
        5 => eExplosionType::EXPLOSION_HELI,
        6 => eExplosionType::EXPLOSION_MINE,
        7 => eExplosionType::EXPLOSION_BARREL,
        8 => eExplosionType::EXPLOSION_TANK_GRENADE,
        9 => eExplosionType::EXPLOSION_HELI_BOMB,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CExplosion {
    pub explosion_type: eExplosionType,
    pub position: CVector,
    pub radius: f32,
    pub propagation_rate: f32,
    pub creator_entity: Option<EntityReference>,
    pub victim_entity: Option<EntityReference>,
    pub stop_time: f32,
    pub iteration: u8,
    pub active_counter: u8,
    pub start_time: f32,
    pub particles_expire_time: u32,
    pub power: f32,
    pub is_boat: bool,
    pub z_shift: f32,
    /// Ground result owned by the Molotov update; absence suppresses its burst.
    pub molotov_ground_height: Option<f32>,
    /// Bevy-side render bridge marker. re3 emits the initial particle burst
    /// inside AddExplosion; the ECS renderer consumes it on the next frame.
    pub visual_started: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExplosionUpdateIntent {
    TriggerWorldExplosion {
        position: CVector,
        radius: f32,
        power: f32,
        creator: Option<EntityReference>,
        creates_vehicle_fires: bool,
    },
    SetPedsAndCarsOnFire {
        position: CVector,
        radius: f32,
        creator: Option<EntityReference>,
    },
    StartGroundFire {
        position: CVector,
        size: f32,
        creator: Option<EntityReference>,
    },
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CExplosionPool {
    pub explosions: [Option<CExplosion>; NUM_EXPLOSIONS],
    pub pending_intents: Vec<ExplosionUpdateIntent>,
}

impl Default for CExplosionPool {
    fn default() -> Self {
        Self::Initialise()
    }
}

impl CExplosionPool {
    pub fn Initialise() -> Self {
        Self {
            explosions: [None; NUM_EXPLOSIONS],
            pending_intents: Vec::new(),
        }
    }

    /// re3 shutdown only destroys the shared audio entity; slot memory is not
    /// reinitialised until the next `Initialise` call.
    pub fn Shutdown(&mut self) {}

    pub fn GetExplosionActiveCounter(&self, id: u8) -> i8 {
        self.explosions
            .get(id as usize)
            .and_then(|explosion| explosion.map(|explosion| explosion.active_counter as i8))
            .unwrap_or(-1)
    }

    pub fn ResetExplosionActiveCounter(&mut self, id: u8) {
        if let Some(Some(explosion)) = self.explosions.get_mut(id as usize) {
            explosion.active_counter = 0;
        }
    }

    pub fn GetExplosionType(&self, id: u8) -> Option<eExplosionType> {
        self.explosions
            .get(id as usize)
            .and_then(|explosion| explosion.map(|explosion| explosion.explosion_type))
    }

    pub fn GetExplosionPosition(&self, id: u8) -> Option<CVector> {
        self.explosions
            .get(id as usize)
            .and_then(|explosion| explosion.map(|explosion| explosion.position))
    }

    pub fn AddExplosion(
        &mut self,
        exploding_entity: Option<EntityReference>,
        culprit: Option<EntityReference>,
        explosion_type: eExplosionType,
        pos: CVector,
        lifetime: u32,
        now_ms: u32,
    ) -> bool {
        let Some(slot) = self
            .explosions
            .iter_mut()
            .find(|explosion| explosion.is_none())
        else {
            return false;
        };
        let ExplosionParameters { radius, power, duration_ms: duration } = explosion_parameters(explosion_type);
        let explosion = CExplosion {
            explosion_type,
            position: pos,
            radius,
            propagation_rate: 0.5,
            creator_entity: culprit,
            victim_entity: exploding_entity,
            stop_time: now_ms.wrapping_add(lifetime).wrapping_add(duration) as f32,
            iteration: 1,
            active_counter: 1,
            start_time: if matches!(
                explosion_type,
                eExplosionType::EXPLOSION_CAR
                    | eExplosionType::EXPLOSION_CAR_QUICK
                    | eExplosionType::EXPLOSION_HELI
            ) {
                now_ms as f32
            } else {
                0.0
            },
            particles_expire_time: if lifetime != 0 {
                now_ms.wrapping_add(lifetime)
            } else {
                0
            },
            power,
            is_boat: false,
            z_shift: 0.0,
            molotov_ground_height: None,
            visual_started: false,
        };
        if lifetime == 0 && power != 0.0 {
            self.pending_intents.push(world_explosion_intent(explosion));
        }
        if explosion_type == eExplosionType::EXPLOSION_MOLOTOV {
            self.pending_intents.push(ExplosionUpdateIntent::StartGroundFire {
                position: pos,
                size: 1.8,
                creator: culprit,
            });
        }
        *slot = Some(explosion);
        true
    }

    pub fn Update(&mut self, now_ms: u32, time_step: f32) -> Vec<ExplosionUpdateIntent> {
        let mut intents = std::mem::take(&mut self.pending_intents);
        for slot in &mut self.explosions {
            let Some(explosion) = slot.as_mut() else {
                continue;
            };
            if explosion.particles_expire_time != 0 {
                if now_ms > explosion.particles_expire_time {
                    explosion.particles_expire_time = 0;
                    if explosion.power != 0.0 {
                        intents.push(world_explosion_intent(*explosion));
                    }
                }
                continue;
            }

            explosion.radius += explosion.propagation_rate * time_step;
            if explosion.explosion_type == eExplosionType::EXPLOSION_MOLOTOV {
                intents.push(ExplosionUpdateIntent::SetPedsAndCarsOnFire {
                    position: explosion.position,
                    radius: 6.0,
                    creator: explosion.creator_entity,
                });
            }
            if (explosion.stop_time - now_ms as f32) as i32 > 0 {
                explosion.iteration = explosion.iteration.wrapping_add(1);
                if explosion.iteration == 0 {
                    *slot = None;
                }
            } else {
                // m_nIteration == 0 is the free-slot marker in re3.
                *slot = None;
            }
        }
        intents
    }

    pub fn TestForExplosionInArea(
        &self,
        explosion_type: eExplosionType,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        z1: f32,
        z2: f32,
    ) -> bool {
        self.explosions.iter().flatten().any(|explosion| {
            explosion.explosion_type == explosion_type
                && explosion.position.x >= x1
                && explosion.position.x <= x2
                && explosion.position.y >= y1
                && explosion.position.y <= y2
                && explosion.position.z >= z1
                && explosion.position.z <= z2
        })
    }

    pub fn RemoveAllExplosionsInArea(&mut self, pos: CVector, radius: f32) {
        let radius_sqr = radius * radius;
        for explosion in &mut self.explosions {
            if explosion.is_some_and(|explosion| distance_sqr(explosion.position, pos) < radius_sqr)
            {
                *explosion = None;
            }
        }
    }
}

/// CEntity::ResolveReferences for the native explosion slots and frame intents.
/// Actor retirement is serviced even when the gameplay timer is paused.
pub(crate) fn retire_explosion_references(
    mut pool: ResMut<CExplosionPool>, references: EntityReferenceAccess,
) {
    clear_pool_registered_references(&mut pool, &references, &[]);
}

pub(crate) fn clear_explosion_reference_targets(
    mut pool: ResMut<CExplosionPool>, references: EntityReferenceAccess,
    mut cleared: MessageReader<EntityReferencesCleared>,
) {
    let targets: Vec<_> = cleared.read().map(|message| message.0).collect();
    if !targets.is_empty() { clear_pool_registered_references(&mut pool, &references, &targets); }
}

fn clear_pool_registered_references(pool: &mut CExplosionPool,
    references: &EntityReferenceAccess, targets: &[Entity]) {
    for explosion in pool.explosions.iter_mut().flatten() {
        references.clear_registered(&mut explosion.creator_entity, targets);
        references.clear_registered(&mut explosion.victim_entity, targets);
    }
    for intent in &mut pool.pending_intents {
        let (ExplosionUpdateIntent::TriggerWorldExplosion { creator, .. }
            | ExplosionUpdateIntent::SetPedsAndCarsOnFire { creator, .. }
            | ExplosionUpdateIntent::StartGroundFire { creator, .. }) = intent;
        references.clear_registered(creator, targets);
    }
}

/// Resolve the native culprit before borrowing mutable player components.
/// This projection lives only for one intent; it is not another actor registry.
#[derive(Clone, Copy, Default)]
struct ResolvedExplosionSource {
    reference: Option<EntityReference>,
    entity: Option<Entity>,
    ped_handle: Option<i32>,
    is_player: bool,
}

impl ResolvedExplosionSource {
    fn resolve(creator: Option<EntityReference>, references: &EntityReferenceAccess) -> Self {
        let Some(creator) = creator else { return Self::default(); };
        Self {
            reference: references.resolve(creator).map(|_| creator),
            entity: references.resolve(creator),
            ped_handle: references.ped_handle(creator),
            is_player: references.is_player(creator),
        }
    }
}

pub(crate) fn process_explosion_runtime(
    mut explosions: ResMut<CExplosionPool>,
    timer: Res<CTimer>,
    ped_stats: Res<CPedStatsRuntime>,
    mut actors: ExplosionActorAccess,
    mut players_and_references: ParamSet<(CWorldPlayersCombat, EntityReferenceAccess)>,
    mut fire_manager: ResMut<CFireManager>,
    mut general: ResMut<CGeneral>,
    mut object_globals: ResMut<CObjectGlobalState>,
    mut events: ResMut<CEventList>,
    file_loader: Option<Res<crate::core::FileLoader::CFileLoader>>,
    water: Option<Res<crate::renderer::WaterLevel::CWaterLevel>>,
    mut particles: ResMut<crate::renderer::Particle::CParticle>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    camera: Res<crate::core::Camera::CCamera>,
) {
    let mut particles = particles.emitter(&particle_systems, camera.position);
    let now_ms = timer.time_in_milliseconds();
    for explosion in explosions.explosions.iter_mut().flatten() {
        if explosion.explosion_type != eExplosionType::EXPLOSION_MOLOTOV
            || explosion.particles_expire_time != 0 || explosion.iteration >= 10 { continue; }
        if explosion.iteration == 1 {
            let ground = file_loader.as_deref().and_then(|loader| loader.world
                .process_vertical_line_static([explosion.position.x, explosion.position.y,
                    explosion.position.z + 5.0], -1000.0, &loader.model_info))
                .map(|contact| contact.point.z);
            let Some(ground) = ground else {
                bevy::log::warn_once!(target: "rustgta3::weapons", "Molotov particle burst missing ground COL");
                continue;
            };
            explosion.z_shift = ground;
            explosion.molotov_ground_height = Some(ground);
        }
        let Some(ground_height) = explosion.molotov_ground_height else { continue; };
        let spread = f32::from(explosion.iteration) * 0.55;
        for _ in 0..(5.0 * spread).ceil() as i32 {
            let angle = general.get_random_number() as f32 / 256.0 * 6.28;
            let position = CVector::new(explosion.position.x + spread * angle.sin(),
                explosion.position.y + spread * angle.cos(), ground_height + 0.5);
            particles.add_particle_coloured(crate::renderer::ParticleType::PARTICLE_EXPLOSION_MEDIUM,
                position, CVector::ZERO, None, 0.0,
                crate::renderer::Particle::ParticleColour::default(),
                general.get_random_number_in_range_f32(-3.0, 3.0) as i32,
                general.get_random_number_in_range_f32(-180.0, 180.0) as i32, 0, 0);
        }
    }
    let intents = explosions.Update(now_ms, timer.time_step());
    for intent in intents {
        let (ExplosionUpdateIntent::TriggerWorldExplosion { creator, .. }
            | ExplosionUpdateIntent::SetPedsAndCarsOnFire { creator, .. }
            | ExplosionUpdateIntent::StartGroundFire { creator, .. }) = &intent;
        let source = ResolvedExplosionSource::resolve(*creator, &players_and_references.p1());
        match intent {
            ExplosionUpdateIntent::TriggerWorldExplosion {
                position,
                radius,
                power,
                creator: _,
                creates_vehicle_fires,
            } => apply_world_explosion(
                position,
                radius,
                power,
                source,
                creates_vehicle_fires,
                now_ms,
                timer.frame_counter(),
                &ped_stats,
                &mut actors,
                &mut players_and_references.p0(),
                &mut object_globals,
                &mut general,
                &mut events,
                file_loader.as_deref(),
                &mut explosions,
            ),
            ExplosionUpdateIntent::SetPedsAndCarsOnFire {
                position,
                radius,
                creator: _,
            } => set_peds_and_cars_on_fire(
                position,
                radius,
                source.entity,
                now_ms,
                &mut actors,
                &mut players_and_references.p0(),
                &mut fire_manager,
                &mut general,
            ),
            ExplosionUpdateIntent::StartGroundFire {
                position,
                size,
                creator: _,
            } => {
                let Some(loader) = file_loader.as_deref() else {
                    bevy::log::warn_once!(target: "rustgta3::weapons", "Molotov ground fire missing world data");
                    continue;
                };
                let Some(ground_z) = loader.world.find_ground_z_for_3d_coord(
                        position.x,
                        position.y,
                        position.z + 3.0,
                        &loader.model_info,
                    ) else {
                        bevy::log::warn_once!(target: "rustgta3::weapons", "Molotov ground fire missing ground COL");
                        continue;
                    };
                let submerged = water
                    .as_deref()
                    .and_then(|w| w.get_water_level_no_waves(position.x, position.y, ground_z))
                    .is_some_and(|sample| ground_z < sample.level && sample.level - 6.0 < ground_z);
                if submerged { continue; }
                let ground_pos = CVector::new(position.x, position.y, ground_z);
                fire_manager.StartFire(ground_pos, None, source.entity, size, false, now_ms, 10_000);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_world_explosion(
    position: CVector,
    radius: f32,
    power: f32,
    source: ResolvedExplosionSource,
    process_vehicle_bomb_timer: bool,
    now_ms: u32,
    frame_counter: u32,
    ped_stats: &CPedStatsRuntime,
    actors: &mut ExplosionActorAccess,
    world_players: &mut CWorldPlayersCombat,
    object_globals: &mut CObjectGlobalState,
    general: &mut CGeneral,
    events: &mut CEventList,
    loader: Option<&crate::core::FileLoader::CFileLoader>,
    explosions: &mut CExplosionPool,
) {
    if radius <= 0.0 {
        return;
    }
    let creator_is_player = source.is_player;
    let creator_handle = source.ped_handle;
    let creator_is_ped = creator_handle.is_some();
    for ped_entity in actors.peds_mut() {
        let handle = ped_entity.handle;
        let ped = &mut *ped_entity.ped;
        let vitality = &mut *ped_entity.vitality;
        if ped_entity.seated || ped.physical.entity.flags.explosion_proof {
            continue;
        }
        let entity_position = ped.physical.entity.get_position();
        let Some((multiplier, force)) = explosion_response(
            entity_position,
            position,
            radius,
            power,
            ped.physical.mass,
            false,
        ) else {
            continue;
        };
        ped.physical.apply_move_force(force);
        ped.physical.entity.set_is_static(false);
        ped.flags.is_standing = false;
        ped.physical.apply_move_force(CVector::new(0.0, 0.0, 2.0));
        let direction = ped.get_local_direction(ped_entity.locomotion.rotation_cur, CVector::new(-force.x, -force.y, -force.z));
        let defend_weakness = ped_stats.GetPedStats(ped.ped_stats).m_defend_weakness;
        ped.inflict_weapon_damage(vitality, 
            eWeaponType::WEAPONTYPE_EXPLOSION,
            250.0 * multiplier,
            defend_weakness,
            creator_is_player,
            true,
            now_ms,
            direction,
        );
        if ped.ped_state != PedState::PED_DIE {
            let animation_id =
                crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_FRONT + i32::from(direction);
            let _ = ped.set_fall(vitality, 
                2_000,
                animation_id,
                false,
                now_ms,
                frame_counter,);
        }
        if creator_is_ped {
            let event_type = if ped.ped_type == crate::peds::PedType::ePedType::PEDTYPE_COP {
                EEventType::ShootCop
            } else {
                EEventType::ShootPed
            };
            let _ = events.register_entity_event(
                event_type,
                EEventEntity::Ped,
                handle,
                creator_handle,
                entity_position,
                10_000,
                now_ms,
            );
        }
    }

    for player_index in 0..world_players.slot_count() {
        if world_players.relation(player_index as i32).is_some() {
            continue;
        }
        let Some(player) = world_players.player_mut(player_index as i32) else {
            continue;
        };
        if player.ped.physical.entity.flags.explosion_proof {
            continue;
        }
        let entity_position = player.ped.physical.entity.get_position();
        let Some((multiplier, force)) = explosion_response(
            entity_position,
            position,
            radius,
            power,
            player.ped.physical.mass,
            true,
        ) else {
            continue;
        };
        player.ped.physical.apply_move_force(force);
        player.ped.physical.entity.set_is_static(false);
        player.ped.flags.is_standing = false;
        player
            .ped
            .physical
            .apply_move_force(CVector::new(0.0, 0.0, 2.0));
        let direction = player
            .ped
            .get_local_direction(player.locomotion.rotation_cur, CVector::new(-force.x, -force.y, -force.z));
        player.ped.inflict_weapon_damage(player.vitality, 
            eWeaponType::WEAPONTYPE_EXPLOSION,
            250.0 * multiplier,
            1.0,
            creator_is_player,
            player.can_be_damaged,
            now_ms,
            direction,
        );
        if player.ped.ped_state != PedState::PED_DIE {
            let animation_id =
                crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_FRONT + i32::from(direction);
            let _ = player.ped.set_fall(&player.vitality, 
                2_000,
                animation_id,
                false,
                now_ms,
                frame_counter,);
        }
        if creator_is_ped {
            let event_type = if player.ped.ped_type
                == crate::peds::PedType::ePedType::PEDTYPE_COP
            {
                EEventType::ShootCop
            } else {
                EEventType::ShootPed
            };
            let _ = events.register_entity_event(
                event_type,
                EEventEntity::Ped,
                player.pool_handle,
                creator_handle,
                entity_position,
                10_000,
                now_ms,
            );
        }
    }

    for vehicle in actors.vehicles_mut() {
        let core = &mut *vehicle.core;
        let vitality = &mut *vehicle.vitality;
        if core.physical.entity.flags.explosion_proof {
            continue;
        }
        let entity_position = core.physical.entity.get_position();
        let Some((multiplier, force)) = explosion_response(
            entity_position,
            position,
            radius,
            power,
            core.physical.mass,
            false,
        ) else {
            continue;
        };
        core.physical.apply_move_force(force);
        core.physical.entity.set_is_static(false);
        apply_explosion_turn_force(&mut core.physical, position, force, loader);
        if core.physical.entity.status == eEntityStatus::STATUS_SIMPLE {
            core.physical.entity.status = eEntityStatus::STATUS_PHYSICS;
            core.switch_to_real_physics(now_ms);
        }
        let old_health = vitality.health;
        let _ = vitality.inflict_weapon_damage_from_source(
            core,
            eWeaponType::WEAPONTYPE_EXPLOSION,
            1100.0 * multiplier,
            creator_is_player,
        );
        // CVehicle::InflictDamage schedules fatal blast damage as a delayed
        // explosion. This path is independent of the car's five-second fire.
        if old_health > 0.0 && vitality.health <= 0.0 {
            vitality.bomb_timer = ((1000 + u32::from(general.get_random_number())) & 0x7ff) as i16;
            vitality.blow_up_entity = source.reference;
        }
        // TriggerExplosionSectorList shortens the timer after InflictDamage,
        // including the new timer created by this very blast.
        if process_vehicle_bomb_timer && vitality.bomb_timer != 0 {
            vitality.bomb_timer /= 10;
        }
    }

    let barrel_creator = world_players.focused().map(|player| player.pool_handle as u32);
    for (object_handle, object) in actors.objects_mut() {
        if distance_sqr(object.physical.entity.get_position(), position) < radius * radius
            && let Some(loader) = loader {
            super::WeaponRuntime::blow_up_explosive_object_state(object_handle,
                barrel_creator, object, loader, explosions, general, now_ms);
        }
        if object.physical.entity.flags.explosion_proof {
            continue;
        }
        let entity_position = object.physical.entity.get_position();
        let Some((multiplier, force)) = explosion_response(
            entity_position,
            position,
            radius,
            power,
            object.physical.mass,
            false,
        ) else {
            continue;
        };
        if object.physical.entity.flags.is_static && power > object.uproot_limit {
            object.physical.entity.set_is_static(false);
        }
        if !object.physical.entity.flags.is_static {
            object.physical.apply_move_force(force);
            apply_explosion_turn_force(&mut object.physical, position, force, loader);
        }
        let _ = object.object_damage(
                300.0 * multiplier,
                object_globals,
                CObjectDamageContext {
                    body_cast_model_index: loader
                        .and_then(|loader| loader.model_info.get_model_info_by_name("bodycast"))
                        .map(|(model_id, _)| model_id as i16),
                },
                general,
            );
    }
}

fn apply_explosion_turn_force(physical: &mut crate::entities::Physical::CPhysical,
    explosion_position: CVector, force: CVector,
    loader: Option<&crate::core::FileLoader::CFileLoader>) {
    if physical.entity.flags.ped_physics { return; }
    let bound = loader.and_then(|loader| loader.model_info
        .col_model(physical.entity.model_index.max(0) as usize))
        .map(|col| col.bounding_sphere.radius);
    let Some(radius) = bound else {
        bevy::log::warn_once!(target: "rustgta3::weapons", "explosion torque missing authored COL bound");
        return;
    };
    let height = (explosion_position.z - physical.entity.get_position().z).clamp(-radius, radius);
    physical.apply_turn_force(force, CVector::new(0.0, 0.0, height));
}

fn explosion_response(
    entity_position: CVector,
    explosion_position: CVector,
    radius: f32,
    power: f32,
    mass: f32,
    is_player: bool,
) -> Option<(f32, CVector)> {
    let mut delta = CVector::new(
        entity_position.x - explosion_position.x,
        entity_position.y - explosion_position.y,
        entity_position.z - explosion_position.z,
    );
    let distance = delta.magnitude();
    if distance >= radius {
        return None;
    }
    let multiplier = ((radius - distance) * 2.0 / radius).min(1.0);
    let scale = power * mass / 1400.0 * multiplier / distance.max(0.01);
    delta.x *= scale;
    delta.y *= scale;
    delta.z = (delta.z * scale).max(0.0);
    if is_player {
        delta.z = delta.z.min(1.0);
    }
    Some((multiplier, delta))
}

#[allow(clippy::too_many_arguments)]
fn set_peds_and_cars_on_fire(
    position: CVector,
    radius: f32,
    source: Option<Entity>,
    now_ms: u32,
    actors: &mut ExplosionActorAccess,
    world_players: &mut CWorldPlayersCombat,
    fire_manager: &mut CFireManager,
    general: &mut CGeneral,
) {

    for actor in actors.peds_mut() {
        let ped = actor.ped;
        let vitality = actor.vitality;
        let ped_position = ped.physical.entity.get_position();
        let eligible = vitality.health > 0.0 && vitality.fire.is_none() && !actor.seated
            && !ped.physical.entity.flags.fire_proof
            && (ped_position.z - position.z).abs() < 5.0
            && (ped_position.x - position.x).abs() < radius
            && (ped_position.y - position.y).abs() < radius;
        if !eligible {
            continue;
        }
        let lifetime = general.get_random_number_in_range_i32(10_000, 11_000) as u32;
        let Some(slot) = fire_manager.StartFire(
            ped_position, Some(actor.entity), source, 0.8, true, now_ms, lifetime,
        ) else {
            break;
        };
        ped.last_ped_state = ped.ped_state;
        ped.stored_move_state = ped.move_state;
        vitality.fire = Some(slot);
        ped.ped_state = PedState::PED_ON_FIRE;
        ped.move_state = crate::peds::Ped::eMoveState::PEDMOVE_SPRINT;
        ped.flee_from_pos_x = position.x;
        ped.flee_from_pos_y = position.y;
        ped.flee_timer = now_ms.saturating_add(10_000);
        ped.queued_sound = crate::audio::soundlist::SOUND_PED_BURNING as i16;
    }

    for player_index in 0..world_players.slot_count() {
        if world_players.relation(player_index as i32).is_some() {
            continue;
        }
        let Some(player) = world_players.player(player_index as i32) else {
            continue;
        };
        let player_position = player.ped.physical.entity.get_position();
        if player.vitality.fire.is_some()
            || player.ped.physical.entity.flags.fire_proof
            || (player_position.z - position.z).abs() >= 5.0
            || (player_position.x - position.x).abs() >= radius
            || (player_position.y - position.y).abs() >= radius
        {
            continue;
        }
        let Some(player_entity) = world_players.entity(player_index as i32) else {
            continue;
        };
        let Some(slot) = fire_manager.StartFire(
            player_position,
            Some(player_entity),
            source,
            0.8,
            true,
            now_ms,
            3_333,
        ) else {
            break;
        };
        if let Some(player) = world_players.player_mut(player_index as i32) {
            player.vitality.fire = Some(slot);
        }
    }

    for actor in actors.vehicles_mut() {
        let core = actor.core;
        let vitality = actor.vitality;
        let vehicle_position = core.physical.entity.get_position();
        let blocked_engine = if actor.identity.kind == crate::control::ScriptEntities::ScriptVehicleKind::Automobile {
            let Some(automobile) = actor.automobile else {
                bevy::log::warn_once!("explosion_vehicle_fire_waiting_automobile_owner");
                continue;
            };
            automobile.damage.get_engine_status() >= 225
        } else {
            false
        };
        if core.physical.entity.status == eEntityStatus::STATUS_WRECKED
            || vitality.car_fire.is_some()
            || core.physical.entity.flags.fire_proof
            || blocked_engine
            || (vehicle_position.z - position.z).abs() >= 5.0
            || (vehicle_position.x - position.x).abs() >= radius
            || (vehicle_position.y - position.y).abs() >= radius
        {
            continue;
        }
        let lifetime = general.get_random_number_in_range_i32(4_000, 5_000) as u32;
        let Some(slot) = fire_manager.StartFire(
            vehicle_position, Some(actor.entity), source, 0.8, true, now_ms, lifetime,
        ) else {
            break;
        };
        vitality.car_fire = Some(slot);
    }
}

struct ExplosionParameters {
    radius: f32,
    power: f32,
    duration_ms: u32,
}

impl ExplosionParameters {
    const fn new(radius: f32, power: f32, duration_ms: u32) -> Self {
        Self { radius, power, duration_ms }
    }
}

fn explosion_parameters(explosion_type: eExplosionType) -> ExplosionParameters {
    match explosion_type {
        eExplosionType::EXPLOSION_GRENADE => ExplosionParameters::new(9.0, 300.0, 750),
        eExplosionType::EXPLOSION_MOLOTOV => ExplosionParameters::new(6.0, 0.0, 3_000),
        eExplosionType::EXPLOSION_ROCKET => ExplosionParameters::new(10.0, 300.0, 750),
        eExplosionType::EXPLOSION_CAR | eExplosionType::EXPLOSION_CAR_QUICK => ExplosionParameters::new(9.0, 300.0, 4_250),
        eExplosionType::EXPLOSION_HELI => ExplosionParameters::new(6.0, 300.0, 750),
        eExplosionType::EXPLOSION_MINE => ExplosionParameters::new(10.0, 150.0, 750),
        eExplosionType::EXPLOSION_BARREL => ExplosionParameters::new(7.0, 150.0, 750),
        eExplosionType::EXPLOSION_TANK_GRENADE => ExplosionParameters::new(10.0, 150.0, 750),
        eExplosionType::EXPLOSION_HELI_BOMB => ExplosionParameters::new(8.0, 50.0, 750),
    }
}

fn world_explosion_intent(explosion: CExplosion) -> ExplosionUpdateIntent {
    ExplosionUpdateIntent::TriggerWorldExplosion {
        position: explosion.position,
        radius: explosion.radius,
        power: explosion.power,
        creator: explosion.creator_entity,
        creates_vehicle_fires: matches!(
            explosion.explosion_type,
            eExplosionType::EXPLOSION_ROCKET
                | eExplosionType::EXPLOSION_CAR_QUICK
                | eExplosionType::EXPLOSION_MINE
                | eExplosionType::EXPLOSION_BARREL
                | eExplosionType::EXPLOSION_TANK_GRENADE
                | eExplosionType::EXPLOSION_HELI_BOMB
        ),
    }
}

fn distance_sqr(a: CVector, b: CVector) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_ped_vehicle_handles_retain_kind_and_retire_independently_while_paused() {
        use bevy::{ecs::system::SystemState, prelude::*};
        use crate::control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity, ScriptVehicleIdentity, ScriptVehicleKind};
        let mut app = App::new();
        let handle = 257;
        let ped = app.world_mut().spawn(ScriptPedIdentity { handle, model_id: 0 }).id();
        let vehicle = app.world_mut().spawn(ScriptVehicleIdentity { handle, kind: ScriptVehicleKind::Automobile }).id();
        let mut index = ScriptEntityIndex::default();
        index.peds.insert(handle, ped);
        index.vehicles.insert(handle, vehicle);
        app.insert_resource(index);
        let mut access = SystemState::<EntityReferenceAccess>::new(app.world_mut());
        {
            let access = access.get(app.world()).expect("canonical reference inputs");
            assert_eq!(access.resolve(EntityReference::Ped(handle)), Some(ped));
            assert_eq!(access.resolve(EntityReference::Vehicle(handle)), Some(vehicle));
            assert_eq!(access.ped_handle(EntityReference::Vehicle(handle)), None);
            assert_eq!(access.ped_handle(EntityReference::Ped(handle)), Some(handle));
        }
        let mut pool = CExplosionPool::default();
        assert!(pool.AddExplosion(Some(EntityReference::Ped(handle)), Some(EntityReference::Vehicle(handle)),
            eExplosionType::EXPLOSION_CAR, CVector::ZERO, 0, 100));
        let mut timer = CTimer::initialise();
        timer.start_user_pause();
        app.add_message::<EntityReferencesCleared>().insert_resource(pool).insert_resource(timer)
            .add_systems(Update, retire_explosion_references);
        app.world_mut().despawn(ped);
        app.update();
        let explosion = app.world().resource::<CExplosionPool>().explosions[0].unwrap();
        assert_eq!(explosion.victim_entity, None);
        assert_eq!(explosion.creator_entity, Some(EntityReference::Vehicle(handle)));
        assert!(app.world().resource::<CTimer>().get_is_paused());
        app.world_mut().despawn(vehicle);
        app.update();
        let pool = app.world().resource::<CExplosionPool>();
        assert_eq!(pool.explosions[0].unwrap().creator_entity, None);
        assert!(pool.pending_intents.iter().all(|intent| match intent {
            ExplosionUpdateIntent::TriggerWorldExplosion { creator, .. }
            | ExplosionUpdateIntent::SetPedsAndCarsOnFire { creator, .. }
            | ExplosionUpdateIntent::StartGroundFire { creator, .. } => creator.is_none(),
        }));
    }

    #[test]
    fn molotov_add_explosion_queues_ground_fire_and_sets_peds_on_fire() {
        let mut pool = CExplosionPool::default();
        let pos = CVector::new(10.0, 20.0, 5.0);
        let added = pool.AddExplosion(
            None,
            None,
            eExplosionType::EXPLOSION_MOLOTOV,
            pos,
            0,
            1_000,
        );
        assert!(added);
        assert_eq!(pool.explosions[0].unwrap().z_shift, 0.0, "native ground height is resolved during the explosion update");

        let intents = pool.Update(1_000, 1.0);
        let has_ground_fire = intents.iter().any(|intent| matches!(
            intent,
            ExplosionUpdateIntent::StartGroundFire { position, size, .. }
                if (position.x - 10.0).abs() < 1e-4 && (*size - 1.8).abs() < 1e-4
        ));
        assert!(has_ground_fire, "Molotov AddExplosion must emit StartGroundFire intent with size 1.8");

        let has_set_on_fire = intents.iter().any(|intent| matches!(
            intent,
            ExplosionUpdateIntent::SetPedsAndCarsOnFire { position, radius, .. }
                if (position.x - 10.0).abs() < 1e-4 && (*radius - 6.0).abs() < 1e-4
        ));
        assert!(has_set_on_fire, "Molotov Update must emit SetPedsAndCarsOnFire intent with radius 6.0");
    }
}
