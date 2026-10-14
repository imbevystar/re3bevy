//! re3 reference: references/re-GTA/src/weapons/ShotInfo.cpp
//! re3 reference: references/re-GTA/src/weapons/ShotInfo.h
//!
//! Persistent area-effect shots (the flamethrower gameplay volume). Visual
//! particles are deliberately not represented here; re3 uses these records to
//! move/expand the damaging volume and to emit ped/vehicle fire side effects.

use bevy::prelude::{Entity, Local, Query, Res, ResMut, Resource};
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    control::ScriptEntities::ScriptVehicleKind,
    core::{Fire::CFireManager, FireActorAccess::FireActorAccess},
    peds::{
        Ped::{PedState, eMoveState},
        PlayerPed::MAX_PLAYERS,
    },
    vehicles::Automobile::AutomobileState,
};

use super::{
    WeaponInfo::{
        CWeaponInfoStore, WEAPONFLAG_EXPANDS, WEAPONFLAG_RAND_SPEED, WEAPONFLAG_SLOWS_DOWN,
    },
    WeaponType::eWeaponType,
};

pub const NUM_SHOTS: usize = 32;
pub const RAND_TABLE_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CShotInfo {
    pub weapon: eWeaponType,
    pub start_pos: CVector,
    /// Direction multiplied by weapon speed, exactly `m_areaAffected` in re3.
    pub area_affected: CVector,
    pub radius: f32,
    pub source_entity: Option<u32>,
    /// Absolute `CTimer::GetTimeInMilliseconds()` expiry time.
    pub timeout: f32,
    pub in_use: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotNearPed {
    pub entity: u32,
    pub position: CVector,
    pub pointer_valid: bool,
    pub in_control: bool,
    pub is_player: bool,
    pub fire_proof: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotUpdateIntent {
    FleeAndSprint {
        ped: u32,
        source: u32,
        duration_ms: u32,
    },
    StartPedFire {
        ped: u32,
        source: u32,
    },
    SetCarsOnFire {
        source: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CShotInfoPool {
    pub shots: [CShotInfo; NUM_SHOTS],
    pub rand_table: [f32; RAND_TABLE_SIZE],
}

impl Default for CShotInfo {
    fn default() -> Self {
        Self {
            weapon: eWeaponType::WEAPONTYPE_COLT45,
            start_pos: CVector::ZERO,
            area_affected: CVector::ZERO,
            radius: 1.0,
            source_entity: None,
            timeout: 0.0,
            in_use: false,
        }
    }
}

impl Default for CShotInfoPool {
    fn default() -> Self {
        Self::Initialise()
    }
}

impl CShotInfoPool {
    pub fn Initialise() -> Self {
        let mut next = -0.05_f32;
        let rand_table = std::array::from_fn(|_| {
            let value = next;
            next += 0.005;
            value
        });
        Self {
            shots: [CShotInfo::default(); NUM_SHOTS],
            rand_table,
        }
    }

    /// re3 shutdown owns no allocation and therefore does not reset the array.
    pub fn Shutdown(&mut self) {}

    /// `CShotInfo::AddShot`. Radius, spread, speed and lifespan come only from
    /// `WEAPON.DAT`; callers do not provide substitute gameplay constants.
    pub fn AddShot(
        &mut self,
        source: Option<u32>,
        weapon: eWeaponType,
        start_pos: CVector,
        end_pos: CVector,
        current_time_ms: u32,
        weapon_info: &CWeaponInfoStore,
        general: &mut CGeneral,
    ) -> bool {
        let Some(info) = weapon_info.GetWeaponInfo(weapon) else {
            return false;
        };
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.in_use) else {
            return false;
        };

        let mut direction = CVector::new(
            end_pos.x - start_pos.x,
            end_pos.y - start_pos.y,
            end_pos.z - start_pos.z,
        );
        if info.spread != 0.0 {
            let random = |general: &mut CGeneral, table: &[f32; RAND_TABLE_SIZE]| {
                table[usize::from(general.get_random_number()) % RAND_TABLE_SIZE]
            };
            direction.x += random(general, &self.rand_table) * info.spread;
            direction.y += random(general, &self.rand_table) * info.spread;
            // re3 intentionally does not multiply Z by m_fSpread.
            direction.z += random(general, &self.rand_table);
        }
        let magnitude = direction.magnitude();
        if magnitude > 0.0 {
            direction.x /= magnitude;
            direction.y /= magnitude;
            direction.z /= magnitude;
        }
        let speed = if info.IsFlagSet(WEAPONFLAG_RAND_SPEED) {
            self.rand_table[usize::from(general.get_random_number()) % RAND_TABLE_SIZE] + info.speed
        } else {
            info.speed
        };
        direction.x *= speed;
        direction.y *= speed;
        direction.z *= speed;

        *slot = CShotInfo {
            weapon,
            start_pos,
            area_affected: direction,
            radius: info.radius,
            source_entity: source,
            timeout: current_time_ms as f32 + info.lifespan,
            in_use: true,
        };
        true
    }

    /// CShotInfo::Update queries authoritative ECS targets through the caller.
    pub fn Update<I: IntoIterator<Item = ShotNearPed>>(
        &mut self,
        current_time_ms: u32,
        time_step: f32,
        frame_counter: u32,
        weapon_info: &CWeaponInfoStore,
        source_pointer_valid: impl Fn(u32) -> bool,
        near_peds: impl Fn(u32) -> I,
    ) -> Vec<(CVector, ShotUpdateIntent)> {
        let mut intents = Vec::new();
        for (slot_index, shot) in self.shots.iter_mut().enumerate() {
            if let Some(source) = shot.source_entity {
                if !source_pointer_valid(source) {
                    shot.source_entity = None;
                }
            }
            if !shot.in_use {
                continue;
            }
            let Some(info) = weapon_info.GetWeaponInfo(shot.weapon) else {
                continue;
            };
            if current_time_ms as f32 > shot.timeout {
                shot.in_use = false;
                // Original code still performs this frame's movement/effects.
            }
            if info.IsFlagSet(WEAPONFLAG_SLOWS_DOWN) {
                let damping = 0.96_f32.powf(time_step);
                shot.area_affected.x *= damping;
                shot.area_affected.y *= damping;
                shot.area_affected.z *= damping;
            }
            if info.IsFlagSet(WEAPONFLAG_EXPANDS) {
                shot.radius += 0.075 * time_step;
            }
            shot.start_pos.x += time_step * shot.area_affected.x;
            shot.start_pos.y += time_step * shot.area_affected.y;
            shot.start_pos.z += time_step * shot.area_affected.z;

            if let Some(source) = shot.source_entity {
                let radius = shot.radius.max(1.0);
                for ped in near_peds(source) {
                    let delta = CVector::new(
                        ped.position.x - shot.start_pos.x,
                        ped.position.y - shot.start_pos.y,
                        ped.position.z - shot.start_pos.z,
                    );
                    // re3 compares MagnitudeSqr against `radius`, not radius².
                    if ped.entity != source
                        && ped.pointer_valid
                        && ped.in_control
                        && delta.magnitude_sqr() < radius
                        && !ped.fire_proof
                    {
                        if !ped.is_player {
                            intents.push((
                                shot.start_pos,
                                ShotUpdateIntent::FleeAndSprint {
                                    ped: ped.entity,
                                    source,
                                    duration_ms: 10_000,
                                },
                            ));
                        }
                        intents.push((
                            shot.start_pos,
                            ShotUpdateIntent::StartPedFire {
                                ped: ped.entity,
                                source,
                            },
                        ));
                    }
                }
            }
            if ((frame_counter + slot_index as u32) & 3) == 0 {
                intents.push((
                    shot.start_pos,
                    ShotUpdateIntent::SetCarsOnFire {
                        source: shot.source_entity,
                    },
                ));
            }
        }
        intents
    }
}

/// World application of re3 `CShotInfo::Update` for persistent area-effect
/// weapons (primarily the flamethrower).
pub(crate) fn process_shot_runtime(
    mut shots: ResMut<CShotInfoPool>,
    timer: Res<CTimer>,
    weapon_info: Res<CWeaponInfoStore>,
    mut actors: FireActorAccess,
    automobiles: Query<&AutomobileState>,
    mut fire_manager: ResMut<CFireManager>,
    mut general: ResMut<CGeneral>,
    mut nearby_vehicles: Local<Vec<Entity>>,
) {
    let now_ms = timer.time_in_milliseconds();
    let intents = {
        let actors = &actors;
        shots.Update(
            now_ms,
            timer.time_step(),
            timer.frame_counter(),
            &weapon_info,
            move |source| actors.ped_entity(source as i32).is_some(),
            move |_| {
                actors
                    .script_peds()
                    .map(|actor| ShotNearPed {
                        entity: actor.handle as u32,
                        position: actor.ped.physical.entity.get_position(),
                        pointer_valid: true,
                        in_control: actor.ped.is_ped_in_control(actor.vitality),
                        is_player: false,
                        fire_proof: actor.ped.physical.entity.flags.fire_proof,
                    })
                    .chain(
                        (0..MAX_PLAYERS)
                            .filter_map(move |slot| actors.player(slot))
                            .map(|actor| ShotNearPed {
                                entity: actor.handle as u32,
                                position: actor.ped.physical.entity.get_position(),
                                pointer_valid: actor.active_in_world,
                                in_control: actor.ped.is_ped_in_control(actor.vitality),
                                is_player: true,
                                fire_proof: actor.ped.physical.entity.flags.fire_proof,
                            }),
                    )
            },
        )
    };
    for (position, intent) in intents {
        match intent {
            ShotUpdateIntent::FleeAndSprint { ped, .. } => {
                if let Some(entity) = actors.ped_entity(ped as i32)
                    && !actors.is_player(entity)
                    && let Some(target) = actors.ped_control_mut(entity)
                {
                    target.ped_state = PedState::PED_FLEE_ENTITY;
                    target.move_state = eMoveState::PEDMOVE_SPRINT;
                }
            }
            ShotUpdateIntent::StartPedFire { ped, source } => {
                let Some(entity) = actors.ped_entity(ped as i32) else {
                    continue;
                };
                let Some(target) = actors.ped(entity) else {
                    continue;
                };
                if target.vitality.fire.is_some() || !target.ped.is_ped_in_control(target.vitality)
                {
                    continue;
                }
                let target_position = target.ped.physical.entity.get_position();
                let is_player = target.is_player;
                let source_entity = actors
                    .ped_entity(source as i32)
                    .or_else(|| actors.vehicle_entity(source as i32));
                let lifetime_ms = if is_player {
                    3_333
                } else {
                    general.get_random_number_in_range_i32(10_000, 11_000) as u32
                };
                let Some(slot) = fire_manager.StartFire(
                    target_position,
                    Some(entity),
                    source_entity,
                    0.8,
                    true,
                    now_ms,
                    lifetime_ms,
                ) else {
                    continue;
                };
                if let Some(target) = actors.ped_mut(entity) {
                    target.vitality.fire = Some(slot);
                    if !is_player {
                        if target.ped.ped_state != PedState::PED_ON_FIRE {
                            target.ped.last_ped_state = target.ped.ped_state;
                            target.ped.stored_move_state = target.ped.move_state;
                        }
                        target.ped.ped_state = PedState::PED_ON_FIRE;
                    }
                }
            }
            ShotUpdateIntent::SetCarsOnFire { source } => {
                let source_entity = source.and_then(|source| {
                    actors
                        .ped_entity(source as i32)
                        .or_else(|| actors.vehicle_entity(source as i32))
                });
                nearby_vehicles.clear();
                for (entity, core, vitality, kind) in actors.vehicles() {
                    let vpos = core.physical.entity.get_position();
                    let in_range = (vpos.x - position.x).abs() < 4.0
                        && (vpos.y - position.y).abs() < 4.0
                        && (vpos.z - position.z).abs() < 5.0;
                    if !in_range
                        || core.physical.entity.status
                            == crate::entities::Entity::eEntityStatus::STATUS_WRECKED
                        || vitality.car_fire.is_some()
                        || core.physical.entity.flags.fire_proof
                    {
                        continue;
                    }
                    if kind == ScriptVehicleKind::Automobile {
                        let Ok(car) = automobiles.get(entity) else {
                            bevy::log::warn_once!("shot_vehicle_fire_waiting_automobile_owner");
                            continue;
                        };
                        if car.damage.get_engine_status() >= 225 {
                            continue;
                        }
                    }
                    nearby_vehicles.push(entity);
                }
                // Scratch is only intent identity; read each current position at
                // mutation time and leave unrelated vehicle owners untouched.
                for entity in nearby_vehicles.drain(..) {
                    let Some((core, _)) = actors.vehicle(entity) else {
                        continue;
                    };
                    let vpos = core.physical.entity.get_position();
                    let lifetime_ms = general.get_random_number_in_range_i32(4_000, 5_000) as u32;
                    if let Some(slot) = fire_manager.StartFire(
                        vpos,
                        Some(entity),
                        source_entity,
                        0.8,
                        true,
                        now_ms,
                        lifetime_ms,
                    ) && let Some((_, vitality, _)) = actors.vehicle_mut(entity)
                    {
                        vitality.car_fire = Some(slot);
                    }
                }
            }
        }
    }
}
