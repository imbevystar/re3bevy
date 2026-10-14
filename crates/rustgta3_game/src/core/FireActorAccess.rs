//! Fire control borrows physical, protection and vitality owners only.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    control::ScriptEntities::{
        ScriptEntityIndex, ScriptObjectIdentity, ScriptPedIdentity, ScriptVehicleIdentity,
        ScriptVehicleKind,
    },
    peds::{
        Ped::CPed,
        PedVitality::PedVitality,
        PlayerPed::{CWorldPlayerState, MAX_PLAYERS, PlayerPedState},
        VehicleRelations::InVehicle,
    },
    vehicles::Vehicle::{VehicleCore, VehicleVitality},
};

pub(crate) struct FirePedRef<'a> {
    pub entity: Entity,
    pub handle: i32,
    pub ped: &'a CPed,
    pub vitality: &'a PedVitality,
    pub relation: Option<InVehicle>,
    pub is_player: bool,
    pub active_in_world: bool,
}

pub(crate) struct FirePedMut<'a> {
    pub ped: &'a mut CPed,
    pub vitality: &'a mut PedVitality,
    pub is_player: bool,
    pub can_be_damaged: bool,
}

#[derive(SystemParam)]
pub(crate) struct FireActorAccess<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    players: Option<Res<'w, CWorldPlayerState>>,
    peds: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static ScriptPedIdentity>,
            Option<&'static PlayerPedState>,
            &'static mut CPed,
            &'static mut PedVitality,
            Option<&'static InVehicle>,
        ),
        Or<(With<ScriptPedIdentity>, With<PlayerPedState>)>,
    >,
    vehicles: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptVehicleIdentity,
            &'static VehicleCore,
            &'static mut VehicleVitality,
        ),
    >,
    references: Query<
        'w,
        's,
        (
            Option<&'static ScriptPedIdentity>,
            Option<&'static ScriptVehicleIdentity>,
            Option<&'static ScriptObjectIdentity>,
            Option<&'static PlayerPedState>,
        ),
    >,
}

fn ped_handle(
    entity: Entity,
    identity: Option<&ScriptPedIdentity>,
    player: Option<&PlayerPedState>,
    index: &ScriptEntityIndex,
    players: Option<&CWorldPlayerState>,
) -> Option<i32> {
    if let Some(player) = player {
        return players?
            .entities
            .contains(&Some(entity))
            .then_some(player.pool_handle);
    }
    let identity = identity?;
    (index.ped(identity.handle) == Some(entity)).then_some(identity.handle)
}

impl FireActorAccess<'_, '_> {
    pub(crate) fn ped(&self, entity: Entity) -> Option<FirePedRef<'_>> {
        let (_, identity, player, ped, vitality, relation) = self.peds.get(entity).ok()?;
        let handle = ped_handle(
            entity,
            identity,
            player,
            &self.index,
            self.players.as_deref(),
        )?;
        Some(FirePedRef {
            entity,
            handle,
            ped,
            vitality,
            relation: relation.copied(),
            is_player: player.is_some(),
            active_in_world: player.is_none_or(|player| player.active_in_world),
        })
    }

    pub(crate) fn ped_mut(&mut self, entity: Entity) -> Option<FirePedMut<'_>> {
        let (_, identity, player, ped, vitality, _) = self.peds.get_mut(entity).ok()?;
        ped_handle(
            entity,
            identity,
            player,
            &self.index,
            self.players.as_deref(),
        )?;
        Some(FirePedMut {
            ped: ped.into_inner(),
            vitality: vitality.into_inner(),
            is_player: player.is_some(),
            can_be_damaged: player.is_none_or(|player| player.can_be_damaged),
        })
    }

    pub(crate) fn ped_control_mut(&mut self, entity: Entity) -> Option<&mut CPed> {
        let (_, identity, player, ped, _, _) = self.peds.get_mut(entity).ok()?;
        ped_handle(
            entity,
            identity,
            player,
            &self.index,
            self.players.as_deref(),
        )?;
        Some(ped.into_inner())
    }

    pub(crate) fn ped_entity(&self, handle: i32) -> Option<Entity> {
        if let Some(entity) = self.index.ped(handle)
            && self
                .ped(entity)
                .is_some_and(|ped| !ped.is_player && ped.handle == handle)
        {
            return Some(entity);
        }
        (0..MAX_PLAYERS).find_map(|slot| {
            let ped = self.player(slot)?;
            (ped.handle == handle).then_some(ped.entity)
        })
    }

    pub(crate) fn player(&self, slot: usize) -> Option<FirePedRef<'_>> {
        self.ped(
            self.players
                .as_ref()?
                .entities
                .get(slot)
                .copied()
                .flatten()?,
        )
        .filter(|ped| ped.is_player)
    }

    pub(crate) fn is_player(&self, entity: Entity) -> bool {
        self.ped(entity).is_some_and(|ped| ped.is_player)
    }

    pub(crate) fn script_peds(&self) -> impl Iterator<Item = FirePedRef<'_>> {
        self.peds
            .iter()
            .filter_map(move |(entity, identity, player, ped, vitality, relation)| {
                if player.is_some() {
                    return None;
                }
                let handle = ped_handle(
                    entity,
                    identity,
                    player,
                    &self.index,
                    self.players.as_deref(),
                )?;
                Some(FirePedRef {
                    entity,
                    handle,
                    ped,
                    vitality,
                    relation: relation.copied(),
                    is_player: false,
                    active_in_world: true,
                })
            })
    }

    /// References can point at any living entity, but indexed actors also have
    /// to retain their original identity during deferred retirement.
    pub(crate) fn live_reference(&self, entity: Entity) -> bool {
        let Ok((ped, vehicle, object, player)) = self.references.get(entity) else {
            return false;
        };
        if player.is_some() {
            return self
                .players
                .as_ref()
                .is_some_and(|players| players.entities.contains(&Some(entity)));
        }
        ped.is_none_or(|ped| self.index.ped(ped.handle) == Some(entity))
            && vehicle.is_none_or(|vehicle| self.index.vehicle(vehicle.handle) == Some(entity))
            && object.is_none_or(|object| self.index.object(object.handle) == Some(entity))
    }

    /// Clear matching attachments in one actor pass. Unaffected components
    /// remain unchanged in Bevy's change tracking.
    pub(crate) fn clear_attachments(&mut self, clear_slot: impl Fn(usize) -> bool) {
        for (entity, identity, player, mut ped, mut vitality, _) in self.peds.iter_mut() {
            if ped_handle(
                entity,
                identity,
                player,
                &self.index,
                self.players.as_deref(),
            )
            .is_none()
            {
                continue;
            }
            if vitality.fire.is_some_and(&clear_slot) {
                super::Fire::restore_ped_after_fire(&mut ped, &mut vitality);
            }
        }
        for (entity, identity, _, mut vitality) in self.vehicles.iter_mut() {
            if self.index.vehicle(identity.handle) == Some(entity)
                && vitality.car_fire.is_some_and(&clear_slot)
            {
                vitality.car_fire = None;
            }
        }
    }

    pub(crate) fn vehicle_entity(&self, handle: i32) -> Option<Entity> {
        let entity = self.index.vehicle(handle)?;
        let (_, identity, _, _) = self.vehicles.get(entity).ok()?;
        (identity.handle == handle).then_some(entity)
    }

    pub(crate) fn vehicle(&self, entity: Entity) -> Option<(&VehicleCore, ScriptVehicleKind)> {
        let (_, identity, core, _) = self.vehicles.get(entity).ok()?;
        (self.index.vehicle(identity.handle) == Some(entity)).then_some((core, identity.kind))
    }

    pub(crate) fn vehicles(
        &self,
    ) -> impl Iterator<Item = (Entity, &VehicleCore, &VehicleVitality, ScriptVehicleKind)> {
        self.vehicles
            .iter()
            .filter_map(move |(entity, identity, core, vitality)| {
                (self.index.vehicle(identity.handle) == Some(entity)).then_some((
                    entity,
                    core,
                    vitality,
                    identity.kind,
                ))
            })
    }

    pub(crate) fn vehicle_mut(
        &mut self,
        entity: Entity,
    ) -> Option<(&VehicleCore, &mut VehicleVitality, ScriptVehicleKind)> {
        let (_, identity, core, vitality) = self.vehicles.get_mut(entity).ok()?;
        (self.index.vehicle(identity.handle) == Some(entity))
            .then(|| (core, vitality.into_inner(), identity.kind))
    }
}
