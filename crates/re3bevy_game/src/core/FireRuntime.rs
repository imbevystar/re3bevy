//! Live fire control borrows actor owners; SCM retains its native 40 slots.
//! Behavior reference: references/re-GTA/src/core/Fire.cpp.

use std::collections::HashSet;

use bevy::prelude::*;
use re3bevy_core::core::{
    Coordinates::CVector, GameState::GtaAppState, General::CGeneral, Schedules::GtaProcessSet,
    Timer::CTimer,
};

use crate::{
    control::ScriptEntities::ScriptVehicleKind,
    core::{
        Camera::CCamera,
        FileLoader::CFileLoader,
        Fire::{CFire, CFireManager, NUM_FIRES, ScriptFireTarget, apply_script_fire_effects},
        FireActorAccess::FireActorAccess,
    },
    modelinfo::{ModelInfo::ModelInfoSlot, VehicleModelInfo::AuthoredVehiclePosition},
    peds::{
        Ped::{PedState, PedWeaponDamageResult, eMoveState},
        PedType::CPedStatsRuntime,
        PlayerPed::MAX_PLAYERS,
    },
    renderer::{
        Particle::CParticle,
        ParticleMgr::cParticleSystemMgr,
        PointLights::{CPointLights, FOG_NONE, LIGHT_POINT},
        Shadows::CShadows,
    },
    vehicles::Vehicle::VehicleCore,
    weapons::WeaponType::eWeaponType,
};

pub struct GtaFirePlugin;

impl Plugin for GtaFirePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CFireManager>()
            .add_systems(
                Update,
                apply_script_fire_effects
                    // SCM can create an actor and start its fire in one pass.
                    // Materialize the canonical owners before borrowing them.
                    .after(
                        crate::control::ScriptDynamicEntities::apply_script_dynamic_entity_effects,
                    )
                    .in_set(GtaProcessSet::Script)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_script_fires
                    .after(apply_script_fire_effects)
                    .after(
                        crate::control::ScriptDynamicEntities::apply_script_dynamic_entity_effects,
                    )
                    .in_set(GtaProcessSet::Fires)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            // Deferred retirement and manager resets also happen outside gameplay.
            .add_systems(Last, retire_fire_actor_references);
    }
}

#[derive(Clone, Copy)]
enum FireTarget {
    Ped(Entity),
    Vehicle(Entity),
}

fn resolve_target(fire: &CFire, actors: &FireActorAccess) -> Option<FireTarget> {
    match fire.script_target {
        Some(ScriptFireTarget::Char(handle)) => actors.ped_entity(handle).map(FireTarget::Ped),
        Some(ScriptFireTarget::Vehicle(handle)) => {
            actors.vehicle_entity(handle).map(FireTarget::Vehicle)
        }
        None => {
            let entity = fire.target?;
            if actors.ped(entity).is_some() {
                Some(FireTarget::Ped(entity))
            } else {
                actors.vehicle(entity).map(|_| FireTarget::Vehicle(entity))
            }
        }
    }
}

/// Clear weak references without discarding the fire's last position or SCM
/// reservation. Bevy entity generations and pool generations both participate.
pub(crate) fn retire_fire_actor_references(
    mut manager: ResMut<CFireManager>,
    mut actors: FireActorAccess,
) {
    for slot in 0..NUM_FIRES {
        let fire = manager.fires[slot];
        if fire
            .source
            .is_some_and(|source| !actors.live_reference(source))
        {
            manager.fires[slot].source = None;
        }
        if (fire.target.is_some() || fire.script_target.is_some())
            && resolve_target(&fire, &actors).is_none()
        {
            manager.fires[slot].target = None;
            manager.fires[slot].script_target = None;
        }
    }
    actors.clear_attachments(|slot| manager.fires.get(slot).is_none_or(|fire| !fire.active));
}

#[derive(Default)]
pub(crate) struct FireScratch {
    players: Vec<(Entity, Option<Entity>)>,
    peds: Vec<(Entity, CVector)>,
    pending: HashSet<Entity>,
}

pub(crate) fn process_script_fires(
    mut manager: ResMut<CFireManager>,
    mut actors: FireActorAccess,
    timer: Res<CTimer>,
    loader: Res<CFileLoader>,
    camera: Res<CCamera>,
    ped_stats: Res<CPedStatsRuntime>,
    mut general: ResMut<CGeneral>,
    mut particles: ResMut<CParticle>,
    particle_systems: Res<cParticleSystemMgr>,
    mut point_lights: ResMut<CPointLights>,
    mut shadows: ResMut<CShadows>,
    mut scratch: Local<FireScratch>,
) {
    let mut particles = particles.emitter(&particle_systems, camera.position);
    let damage = 1.2 * timer.time_step();
    let now_ms = timer.time_in_milliseconds();
    let mut extinguished = [false; NUM_FIRES];
    scratch.players.clear();
    scratch.peds.clear();
    scratch.pending.clear();
    let scratch = &mut *scratch;

    // Reconcile manager-only ExtinguishPoint calls in one actor pass.
    actors.clear_attachments(|slot| manager.fires.get(slot).is_none_or(|fire| !fire.active));

    for slot in 0..NUM_FIRES {
        if !manager.fires[slot].active {
            continue;
        }
        if manager.fires[slot]
            .source
            .is_some_and(|source| !actors.live_reference(source))
        {
            manager.fires[slot].source = None;
        }
        let snapshot = manager.fires[slot];
        let damaged_by_player = snapshot
            .source
            .is_some_and(|source| actors.is_player(source));
        let target = resolve_target(&snapshot, &actors);
        if target.is_none() && (snapshot.target.is_some() || snapshot.script_target.is_some()) {
            manager.fires[slot].target = None;
            manager.fires[slot].script_target = None;
        }
        let mut flame_position = Some(manager.fires[slot].position);
        match target {
            Some(FireTarget::Ped(entity)) => {
                let ped = actors.ped(entity).expect("resolved live fire ped");
                if ped.vitality.fire != Some(slot) {
                    extinguished[slot] = true;
                    continue;
                }
                let mut position = ped.ped.physical.entity.get_position();
                // Native RUN is distinct from SPRINT here.
                if ped.ped.move_state != eMoveState::PEDMOVE_RUN {
                    position.z -= 1.0;
                }
                let vehicle = ped
                    .relation
                    .map(|relation| relation.vehicle)
                    .filter(|vehicle| actors.vehicle(*vehicle).is_some());
                manager.fires[slot].position = position;
                flame_position = Some(position);
                if let Some(vehicle) = vehicle {
                    if let Some((_, vitality, ScriptVehicleKind::Automobile)) =
                        actors.vehicle_mut(vehicle)
                    {
                        vitality.health = 75.0;
                    }
                } else if let Some(actor) = actors.ped_mut(entity) {
                    let weakness = ped_stats.GetPedStats(actor.ped.ped_stats).m_defend_weakness;
                    let result = actor.ped.inflict_weapon_damage(
                        actor.vitality,
                        eWeaponType::WEAPONTYPE_FLAMETHROWER,
                        damage,
                        weakness,
                        damaged_by_player,
                        actor.can_be_damaged,
                        now_ms,
                        0,
                    );
                    if !actor.is_player && result != PedWeaponDamageResult::Ignored {
                        actor.ped.physical.entity.flags.render_scorched = true;
                    }
                }
            }
            Some(FireTarget::Vehicle(entity)) => {
                let (core, vitality, kind) = actors
                    .vehicle_mut(entity)
                    .expect("resolved live fire vehicle");
                if vitality.car_fire != Some(slot) {
                    extinguished[slot] = true;
                    continue;
                }
                manager.fires[slot].position = core.physical.entity.get_position();
                flame_position = vehicle_flame_position(&loader, core, kind);
                if !snapshot.is_script_fire {
                    vitality.inflict_weapon_damage_from_source(
                        core,
                        eWeaponType::WEAPONTYPE_FLAMETHROWER,
                        damage,
                        damaged_by_player,
                    );
                }
            }
            None => {}
        }

        let position = manager.fires[slot].position;
        let source = manager.fires[slot].source;
        for player_slot in 0..MAX_PLAYERS {
            let Some(player) = actors.player(player_slot) else {
                continue;
            };
            if player.relation.is_some()
                || player.vitality.fire.is_some()
                || player.ped.physical.entity.flags.fire_proof
                || !player.ped.is_ped_in_control(player.vitality)
            {
                continue;
            }
            if distance_sqr(player.ped.physical.entity.get_position(), position) < 2.0
                && scratch.pending.insert(player.entity)
            {
                scratch.players.push((player.entity, source));
            }
        }
        for ped in actors.script_peds() {
            if ped.vitality.fire.is_some()
                || ped.ped.physical.entity.flags.fire_proof
                || !ped.ped.is_ped_in_control(ped.vitality)
            {
                continue;
            }
            if distance_sqr(ped.ped.physical.entity.get_position(), position) < 2.0
                && scratch.pending.insert(ped.entity)
            {
                scratch.peds.push((ped.entity, position));
            }
        }

        let fire = &mut manager.fires[slot];
        if now_ms > fire.next_flame_time_ms {
            fire.next_flame_time_ms = now_ms.wrapping_add(80);
            let vertical_speed =
                general.get_random_number_in_range_f32(0.0125, 0.1) * fire.strength;
            if let Some(position) = flame_position {
                particles.add_particle(
                    crate::renderer::ParticleType::PARTICLE_CARFLAME,
                    position,
                    CVector::new(0.0, 0.0, vertical_speed),
                    None,
                    fire.strength,
                    0,
                    0,
                    0,
                    0,
                );
            }
            // The native three explicit RNG calls remain between flame and smoke,
            // including when missing required authored data prevents emission.
            general.get_random_number();
            general.get_random_number();
            general.get_random_number();
            if let Some(position) = flame_position {
                particles.add_particle(
                    crate::renderer::ParticleType::PARTICLE_CARFLAME_SMOKE,
                    position,
                    CVector::ZERO,
                    None,
                    0.0,
                    0,
                    0,
                    0,
                    0,
                );
            }
        }
        if now_ms < fire.extinguish_time_ms || fire.is_script_fire {
            if now_ms > fire.start_time_ms {
                fire.start_time_ms = now_ms.wrapping_add(400);
            }
            let light_random = general.get_random_number() & 127;
            let intensity = light_random as f32 / 128.0;
            if fire.script_target.is_none() && fire.target.is_none() {
                shadows
                    .static_shadows
                    .push(crate::renderer::Shadows::StaticShadowIntent {
                        key: slot as u32,
                        texture: 3,
                        position: CVector::new(
                            fire.position.x,
                            fire.position.y,
                            fire.position.z + 0.05,
                        ),
                        front: [7.0, 0.0],
                        side: [0.0, -7.0],
                        intensity: 0,
                        color: [(light_random / 2) as u8, (light_random / 2) as u8, 0],
                    });
            }
            point_lights.add_light(
                LIGHT_POINT,
                fire.position,
                CVector::ZERO,
                12.0,
                intensity,
                intensity,
                0.0,
                FOG_NONE,
                false,
                camera.position,
            );
        } else {
            extinguished[slot] = true;
        }
    }

    if extinguished.iter().any(|extinguished| *extinguished) {
        actors.clear_attachments(|slot| extinguished.get(slot).copied().unwrap_or(false));
        for (slot, extinguished) in extinguished.into_iter().enumerate() {
            if extinguished {
                manager.extinguish_slot(slot);
            }
        }
    }
    for (entity, source) in scratch.players.drain(..) {
        let Some(actor) = actors.ped(entity) else {
            continue;
        };
        if !actor.is_player
            || actor.vitality.fire.is_some()
            || !actor.ped.is_ped_in_control(actor.vitality)
        {
            continue;
        }
        let position = actor.ped.physical.entity.get_position();
        let Some(slot) =
            manager.StartFire(position, Some(entity), source, 0.8, true, now_ms, 3_333)
        else {
            continue;
        };
        let actor = actors
            .ped_mut(entity)
            .expect("preflighted player fire owner");
        actor.vitality.fire = Some(slot);
        actor.ped.queued_sound = crate::audio::soundlist::SOUND_PED_BURNING as i16;
    }
    for (entity, origin) in scratch.peds.drain(..) {
        let Some(actor) = actors.ped(entity) else {
            continue;
        };
        if actor.is_player
            || actor.vitality.fire.is_some()
            || !actor.ped.is_ped_in_control(actor.vitality)
        {
            continue;
        }
        let lifetime = general.get_random_number_in_range_f32(10_000.0, 11_000.0) as u32;
        let position = actor.ped.physical.entity.get_position();
        let Some(slot) =
            manager.StartFire(position, Some(entity), None, 0.8, true, now_ms, lifetime)
        else {
            continue;
        };
        let actor = actors.ped_mut(entity).expect("preflighted NPC fire owner");
        actor.vitality.fire = Some(slot);
        if actor.ped.ped_state != PedState::PED_ON_FIRE {
            actor.ped.last_ped_state = actor.ped.ped_state;
            actor.ped.stored_move_state = actor.ped.move_state;
        }
        actor.ped.ped_state = PedState::PED_ON_FIRE;
        actor.ped.move_state = eMoveState::PEDMOVE_SPRINT;
        actor.ped.flee_from_pos_x = origin.x;
        actor.ped.flee_from_pos_y = origin.y;
        actor.ped.flee_timer = now_ms.saturating_add(10_000);
        actor.ped.queued_sound = crate::audio::soundlist::SOUND_PED_BURNING as i16;
    }
}

/// Automobile flames use the resident, preprocessed CAR_POS_HEADLIGHTS dummy.
/// Absent is native zero initialization; invalid/unloaded data is a named wait.
fn vehicle_flame_position(
    loader: &CFileLoader,
    core: &VehicleCore,
    kind: ScriptVehicleKind,
) -> Option<CVector> {
    if kind != ScriptVehicleKind::Automobile {
        return Some(core.physical.entity.get_position());
    }
    let Some(ModelInfoSlot::Vehicle(model)) = usize::try_from(core.physical.entity.model_index)
        .ok()
        .and_then(|model| loader.model_info.get_model_info(model))
    else {
        bevy::log::warn_once!("fire_flame_required_vehicle_model_missing");
        return None;
    };
    let Some(positions) = model.authored_positions() else {
        bevy::log::warn_once!("fire_flame_waiting_resident_vehicle_clump");
        return None;
    };
    let local = match positions.headlights {
        AuthoredVehiclePosition::Position(position) => position,
        AuthoredVehiclePosition::Absent => CVector::ZERO,
        AuthoredVehiclePosition::Invalid => {
            bevy::log::warn_once!("fire_flame_invalid_authored_headlights_chain");
            return None;
        }
    };
    let mut position = core.physical.entity.transform_point(local);
    position.z += 0.15;
    Some(position)
}

fn distance_sqr(a: CVector, b: CVector) -> f32 {
    let delta = CVector::new(a.x - b.x, a.y - b.y, a.z - b.z);
    delta.x * delta.x + delta.y * delta.y + delta.z * delta.z
}
