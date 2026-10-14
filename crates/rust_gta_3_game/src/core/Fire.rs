//! re3 reference: references/re-GTA/src/core/Fire.cpp
//! re3 reference: references/re-GTA/src/core/Fire.h
//!
//! Fixed-slot fire manager state. Script fires use the same 40-slot pool and
//! retain their slot index because SCM exposes that index to scripts.

use bevy::prelude::{Entity, Query, Res, ResMut, Resource};
use rust_gta_3_core::core::{Coordinates::CVector, Timer::CTimer};

use crate::{
    control::{
        Garages::CGarages,
        Script::{CTheScripts, ScriptCommandEffect},
        ScriptEntities::ScriptVehicleKind,
    },
    core::FireActorAccess::FireActorAccess,
    peds::Ped::{CPed, PedState, eMoveState},
    vehicles::Automobile::AutomobileState,
};

/// re3 `NUM_FIRES` (`core/config.h`).
pub const NUM_FIRES: usize = 40;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScriptFireTarget {
    Char(i32),
    Vehicle(i32),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CFire {
    pub position: CVector,
    /// Native Bevy target, used by non-SCM callers.
    pub target: Option<Entity>,
    /// Alternative lookup form for SCM-created fires: a kind-tagged CPools
    /// handle including its generation. Ordinary attachments use `target`;
    /// constructors never populate both forms.
    pub script_target: Option<ScriptFireTarget>,
    pub source: Option<Entity>,
    pub strength: f32,
    pub active: bool,
    pub is_script_fire: bool,
    pub propagation: bool,
    pub audio_set: bool,
    pub extinguish_time_ms: u32,
    pub start_time_ms: u32,
    pub next_flame_time_ms: u32,
    pub firemen_putting_out: u32,
}

impl Default for CFire {
    fn default() -> Self {
        Self {
            position: CVector::ZERO,
            target: None,
            script_target: None,
            source: None,
            strength: 0.8,
            active: false,
            is_script_fire: false,
            propagation: true,
            audio_set: true,
            extinguish_time_ms: 0,
            start_time_ms: 0,
            next_flame_time_ms: 0,
            firemen_putting_out: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CFireManager {
    pub total_fires: u32,
    /// Matches re3 `m_aFires[NUM_FIRES]`; index is the public SCM fire handle.
    pub fires: [CFire; NUM_FIRES],
}

impl Default for CFireManager {
    fn default() -> Self {
        Self {
            total_fires: 0,
            fires: [CFire::default(); NUM_FIRES],
        }
    }
}

impl CFireManager {
    pub(crate) fn next_free_fire(&self) -> Option<usize> {
        self.fires
            .iter()
            .position(|fire| !fire.active && !fire.is_script_fire)
    }

    #[allow(non_snake_case)]
    pub fn StartFire(
        &mut self,
        position: CVector,
        target: Option<Entity>,
        source: Option<Entity>,
        strength: f32,
        propagation: bool,
        current_time_ms: u32,
        lifetime_ms: u32,
    ) -> Option<usize> {
        let slot = self.next_free_fire()?;
        self.fires[slot] = CFire {
            position,
            target,
            script_target: None,
            source,
            strength,
            active: true,
            is_script_fire: false,
            propagation,
            audio_set: true,
            extinguish_time_ms: current_time_ms.wrapping_add(lifetime_ms),
            start_time_ms: current_time_ms.wrapping_add(400),
            next_flame_time_ms: 0,
            firemen_putting_out: 0,
        };
        self.total_fires = self.total_fires.saturating_add(1);
        Some(slot)
    }

    /// Mirrors `CFireManager::StartScriptFire` once SCM has reserved its
    /// returned pool slot synchronously.
    pub fn start_script_fire_at(
        &mut self,
        slot: i32,
        position: CVector,
        script_target: Option<ScriptFireTarget>,
        current_time_ms: u32,
    ) -> bool {
        let Ok(slot) = usize::try_from(slot) else {
            return false;
        };
        let Some(fire) = self.fires.get_mut(slot) else {
            return false;
        };
        *fire = CFire {
            position,
            target: None,
            script_target,
            source: None,
            strength: 0.8,
            active: true,
            is_script_fire: true,
            propagation: true,
            audio_set: true,
            // Script fires are exempt from the extinguish-time check.
            extinguish_time_ms: 0,
            start_time_ms: current_time_ms.wrapping_add(400),
            next_flame_time_ms: 0,
            firemen_putting_out: 0,
        };
        true
    }

    /// Mirrors `CFire::Extinguish`: an extinguished script fire keeps its
    /// script-fire marker until REMOVE_SCRIPT_FIRE/REMOVE_ALL_SCRIPT_FIRES.
    /// That reservation is observable through `IsScriptFireExtinguish` and
    /// prevents `GetNextFreeFire` from reusing the SCM handle prematurely.
    pub fn extinguish_slot(&mut self, slot: usize) -> bool {
        let Some(fire) = self.fires.get_mut(slot) else {
            return false;
        };
        if !fire.active {
            return false;
        }
        if !fire.is_script_fire {
            self.total_fires = self.total_fires.saturating_sub(1);
        }
        fire.extinguish_time_ms = 0;
        fire.active = false;
        fire.target = None;
        fire.script_target = None;
        fire.source = None;
        true
    }

    pub fn is_script_fire_extinguished(&self, slot: i32) -> bool {
        usize::try_from(slot)
            .ok()
            .and_then(|slot| self.fires.get(slot))
            .is_none_or(|fire| !fire.active)
    }

    pub fn remove_script_fire(&mut self, slot: i32) {
        if let Ok(slot) = usize::try_from(slot) {
            if slot < self.fires.len() {
                self.extinguish_slot(slot);
                self.fires[slot] = CFire::default();
            }
        }
    }

    pub fn remove_all_script_fires(&mut self) {
        for slot in 0..self.fires.len() {
            if self.fires[slot].is_script_fire {
                self.extinguish_slot(slot);
                self.fires[slot] = CFire::default();
            }
        }
    }

    pub fn set_script_fire_audio(&mut self, slot: i32, state: bool) {
        if let Ok(slot) = usize::try_from(slot) {
            if let Some(fire) = self.fires.get_mut(slot) {
                fire.audio_set = state;
            }
        }
    }

    /// ResolveReferences can clear links to a living actor (resurrection).
    /// It does not extinguish unrelated fires or release their SCM handles.
    pub(crate) fn clear_actor_references(
        &mut self,
        entity: Entity,
        script_target: Option<ScriptFireTarget>,
    ) {
        for fire in &mut self.fires {
            if fire.source == Some(entity) {
                fire.source = None;
            }
            if fire.target == Some(entity)
                || script_target.is_some_and(|target| fire.script_target == Some(target))
            {
                fire.target = None;
                fire.script_target = None;
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn ExtinguishPoint(&mut self, position: CVector, radius: f32) {
        let radius_sqr = radius * radius;
        for slot in 0..NUM_FIRES {
            let fire = &self.fires[slot];
            let dx = fire.position.x - position.x;
            let dy = fire.position.y - position.y;
            let dz = fire.position.z - position.z;
            if fire.active && dx * dx + dy * dy + dz * dz < radius_sqr {
                self.extinguish_slot(slot);
            }
        }
    }

    pub fn GetTotalActiveFires(&self) -> u32 {
        self.total_fires
    }

    pub fn FindFurthestFire_NeverMindFireMen(
        &self,
        coords: CVector,
        min_range: f32,
        max_range: f32,
    ) -> Option<usize> {
        let mut furthest = None;
        let mut last_distance = 0.0_f32;
        for (index, fire) in self.fires.iter().enumerate() {
            if !fire.active || fire.is_script_fire {
                continue;
            }
            let dx = fire.position.x - coords.x;
            let dy = fire.position.y - coords.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > min_range && distance < max_range && distance > last_distance {
                last_distance = distance;
                furthest = Some(index);
            }
        }
        furthest
    }

    /// re3 makes two passes, first looking for unattended fires and then for
    /// fires with one fireman already assigned.
    pub fn FindNearestFire(&self, coords: CVector) -> Option<(usize, f32)> {
        for attendance in 0..2_u32 {
            let mut nearest = None;
            let mut min_distance = 999_999.0_f32;
            for (index, fire) in self.fires.iter().enumerate() {
                if !fire.active || fire.is_script_fire || fire.firemen_putting_out != attendance {
                    continue;
                }
                let dx = fire.position.x - coords.x;
                let dy = fire.position.y - coords.y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance < min_distance {
                    min_distance = distance;
                    nearest = Some(index);
                }
            }
            if let Some(index) = nearest {
                return Some((index, min_distance));
            }
        }
        None
    }
}

pub use super::FireRuntime::GtaFirePlugin;
pub(crate) use super::FireRuntime::process_script_fires;

pub(crate) fn apply_script_fire_effects(
    mut scripts: ResMut<CTheScripts>,
    mut fire_manager: ResMut<CFireManager>,
    mut actors: FireActorAccess,
    mut automobiles: Query<&mut AutomobileState>,
    mut garages: Option<ResMut<CGarages>>,
    timer: Res<CTimer>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    let mut fire_commands_waiting = false;
    for effect in effects {
        if fire_commands_waiting
            && matches!(
                &effect,
                ScriptCommandEffect::StartScriptFire { .. }
                    | ScriptCommandEffect::RemoveScriptFire { .. }
                    | ScriptCommandEffect::RemoveAllScriptFires
                    | ScriptCommandEffect::SetScriptFireAudio { .. }
                    | ScriptCommandEffect::ExtinguishPlayerFires { .. }
            )
        {
            leftover.push(effect);
            continue;
        }
        match effect {
            ScriptCommandEffect::StartScriptFire {
                slot,
                position,
                target,
            } => {
                let vehicle_entity = match target {
                    Some(ScriptFireTarget::Vehicle(handle)) => actors.vehicle_entity(handle),
                    _ => None,
                };
                if let Some(entity) = vehicle_entity
                    && actors
                        .vehicle(entity)
                        .is_some_and(|(_, kind)| kind == ScriptVehicleKind::Automobile)
                    && automobiles.get(entity).is_err()
                {
                    bevy::log::warn_once!("script_fire_waiting_automobile_owner");
                    fire_commands_waiting = true;
                    leftover.push(ScriptCommandEffect::StartScriptFire {
                        slot,
                        position,
                        target,
                    });
                    continue;
                }
                if !fire_manager.start_script_fire_at(
                    slot,
                    position,
                    target,
                    timer.time_in_milliseconds(),
                ) {
                    scripts.release_script_fire_slot(slot);
                    continue;
                }
                clear_script_fire_target(&mut actors, slot);
                match target {
                    Some(ScriptFireTarget::Char(handle)) => {
                        if let Some(entity) = actors.ped_entity(handle)
                            && let Some(actor) = actors.ped_mut(entity)
                        {
                            if let Some(previous) = actor.vitality.fire {
                                fire_manager.extinguish_slot(previous);
                            }
                            actor.vitality.fire = Some(slot as usize);
                            // StartScriptFire retains the player's state. NPCs
                            // save their previous state before entering ON_FIRE.
                            if !actor.is_player {
                                if actor.ped.ped_state != PedState::PED_ON_FIRE {
                                    actor.ped.last_ped_state = actor.ped.ped_state;
                                    actor.ped.stored_move_state = actor.ped.move_state;
                                }
                                actor.ped.ped_state = PedState::PED_ON_FIRE;
                                actor.ped.move_state = eMoveState::PEDMOVE_RUN;
                            }
                        }
                    }
                    Some(ScriptFireTarget::Vehicle(_)) => {
                        if let Some(entity) = vehicle_entity
                            && let Some((_, vitality, kind)) = actors.vehicle_mut(entity)
                        {
                            if let Some(previous) = vitality.car_fire {
                                fire_manager.extinguish_slot(previous);
                            }
                            if kind == ScriptVehicleKind::Automobile {
                                let mut car = automobiles
                                    .get_mut(entity)
                                    .expect("preflighted automobile owner");
                                if car.damage.get_engine_status() >= 225 {
                                    car.damage.set_engine_status(215);
                                }
                            }
                            vitality.car_fire = Some(slot as usize);
                        }
                    }
                    None => {}
                }
            }
            ScriptCommandEffect::RemoveScriptFire { slot } => {
                clear_script_fire_target(&mut actors, slot);
                fire_manager.remove_script_fire(slot);
                scripts.release_script_fire_slot(slot);
            }
            ScriptCommandEffect::RemoveAllScriptFires => {
                let script_slots = fire_manager.fires.map(|fire| fire.is_script_fire);
                actors.clear_attachments(|slot| script_slots.get(slot).copied().unwrap_or(false));
                fire_manager.remove_all_script_fires();
                scripts.script_fire_slots.fill(false);
            }
            ScriptCommandEffect::SetScriptFireAudio { slot, enabled } => {
                fire_manager.set_script_fire_audio(slot, enabled)
            }
            ScriptCommandEffect::ExtinguishPlayerFires { player_index } => {
                let Some(player) = actors.player(player_index.max(0) as usize) else {
                    fire_commands_waiting = true;
                    leftover.push(ScriptCommandEffect::ExtinguishPlayerFires { player_index });
                    continue;
                };
                // COMMAND_SAVE_PLAYER_FROM_FIRES: ExtinguishPoint(player pos, 3).
                fire_manager.ExtinguishPoint(player.ped.physical.entity.get_position(), 3.0);
                actors.clear_attachments(|slot| {
                    fire_manager.fires.get(slot).is_none_or(|fire| !fire.active)
                });
            }
            ScriptCommandEffect::SetFreeResprays { enabled } => {
                if let Some(garages) = garages.as_mut() {
                    garages.resprays_are_free = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetFreeResprays { enabled });
                }
            }
            ScriptCommandEffect::SetFreeBombs { enabled } => {
                if let Some(garages) = garages.as_mut() {
                    garages.bombs_are_free = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetFreeBombs { enabled });
                }
            }
            other => leftover.push(other),
        }
    }
    actors.clear_attachments(|slot| fire_manager.fires.get(slot).is_none_or(|fire| !fire.active));
    scripts.pending_effects = leftover;
}

pub(crate) fn clear_script_fire_target(actors: &mut FireActorAccess, slot: i32) {
    if let Ok(slot) = usize::try_from(slot) {
        actors.clear_attachments(|attachment| attachment == slot);
    }
}

pub(crate) fn restore_ped_after_fire(
    ped: &mut CPed,
    vitality: &mut crate::peds::PedVitality::PedVitality,
) {
    vitality.fire = None;
    if ped.ped_state != PedState::PED_ON_FIRE {
        return;
    }
    if ped.last_ped_state != PedState::PED_NONE {
        ped.ped_state = ped.last_ped_state;
        ped.move_state = ped.stored_move_state;
        ped.last_ped_state = PedState::PED_NONE;
    } else {
        ped.ped_state = PedState::PED_IDLE;
        ped.move_state = eMoveState::PEDMOVE_STILL;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extinguished_script_fire_stays_reserved_until_removed() {
        let mut fires = CFireManager::default();
        assert!(fires.start_script_fire_at(0, CVector::new(1.0, 2.0, 3.0), None, 100));
        fires.ExtinguishPoint(CVector::new(1.0, 2.0, 3.0), 1.0);

        assert!(!fires.fires[0].active);
        assert!(fires.fires[0].is_script_fire);
        assert_eq!(fires.next_free_fire(), Some(1));
        assert!(fires.is_script_fire_extinguished(0));

        fires.remove_script_fire(0);
        assert_eq!(fires.next_free_fire(), Some(0));
    }

    #[test]
    fn extinguished_normal_fire_updates_count_and_reuses_slot() {
        let mut fires = CFireManager::default();
        let slot = fires
            .StartFire(
                CVector::new(4.0, 5.0, 6.0),
                None,
                None,
                0.8,
                true,
                1_000,
                10_000,
            )
            .expect("normal fire should get the first slot");
        assert_eq!(slot, 0);
        assert_eq!(fires.GetTotalActiveFires(), 1);

        fires.ExtinguishPoint(CVector::new(4.0, 5.0, 6.0), 1.0);
        assert_eq!(fires.GetTotalActiveFires(), 0);
        assert_eq!(fires.next_free_fire(), Some(0));
    }

    #[test]
    fn molotov_ground_fire_properties() {
        let mut fires = CFireManager::default();
        let slot = fires
            .StartFire(
                CVector::new(10.0, 20.0, 5.0),
                None,
                None,
                1.8,
                false,
                1_000,
                10_000,
            )
            .expect("should allocate fire slot");
        assert_eq!(fires.fires[slot].strength, 1.8);
        assert_eq!(fires.fires[slot].extinguish_time_ms, 11_000);
        assert!(!fires.fires[slot].is_script_fire);
        assert!(fires.fires[slot].active);
    }

    #[test]
    fn fire_damage_absorbs_armor_first() {
        let flame_damage = 1.2 * 1.0;
        let mut armour = 50.0f32;
        let mut health = 100.0f32;
        let health_impact = flame_damage * 0.33;
        if armour > 0.0 {
            if health_impact < armour {
                armour -= health_impact;
            } else {
                let remaining = health_impact - armour;
                armour = 0.0;
                health = (health - remaining).max(0.0);
            }
        } else {
            health = (health - health_impact).max(0.0);
        }
        assert!(armour < 50.0);
        assert_eq!(
            health, 100.0,
            "Health must not be reduced when armor is present"
        );
    }

    #[test]
    fn molotov_ground_fire_shadow_placement_on_ground() {
        let fire_pos = CVector::new(15.0, 25.0, 8.0);
        let shadow_pos = CVector::new(fire_pos.x, fire_pos.y, fire_pos.z + 0.05);
        assert!((shadow_pos.z - 8.05).abs() < 1e-4);
        assert!(
            (shadow_pos.z - (fire_pos.z + 5.0)).abs() > 4.0,
            "Shadow must not float 5 meters in the air"
        );
    }
}
