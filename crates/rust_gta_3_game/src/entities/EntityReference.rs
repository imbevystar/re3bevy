//! Native CEntity references retain pool kind/generation or the actual ECS Entity.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    control::ScriptEntities::{ScriptEntityIndex, ScriptObjectIdentity, ScriptPedIdentity, ScriptVehicleIdentity},
    core::FileLoader::CFileLoader,
    peds::PlayerPed::{CWorldPlayerState, PlayerPedState},
};

/// Numeric handles from different pools are distinct identities. World indices
/// refer to parsed static map entities, never to an actor pool or ECS row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityReference {
    Ped(i32),
    Vehicle(i32),
    Object(i32),
    World(usize),
    Entity(Entity),
}

impl EntityReference {
    pub fn ped_handle(self) -> Option<i32> {
        match self { Self::Ped(handle) => Some(handle), _ => None }
    }
}

/// ResolveReferences can clear pointers to an actor that remains alive, such
/// as the player and previous vehicle during resurrection. This is a Bevy
/// lifecycle notification, not another reference table or actor snapshot.
#[derive(Message, Clone, Copy)]
pub(crate) struct EntityReferencesCleared(pub Entity);

#[derive(SystemParam)]
pub(crate) struct EntityReferenceAccess<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    players: Option<Res<'w, CWorldPlayerState>>,
    loader: Option<Res<'w, CFileLoader>>,
    identities: Query<'w, 's, (
        Option<&'static ScriptPedIdentity>, Option<&'static ScriptVehicleIdentity>,
        Option<&'static ScriptObjectIdentity>, Option<&'static PlayerPedState>,
    )>,
}

impl EntityReferenceAccess<'_, '_> {
    pub(crate) fn clear_registered(
        &self, reference: &mut Option<EntityReference>, cleared: &[Entity],
    ) {
        let Some(value) = *reference else { return; };
        if self.registered_should_clear(value, cleared) {
            *reference = None;
        }
    }
    pub(crate) fn registered_should_clear(&self, value: EntityReference, cleared: &[Entity]) -> bool {
        if matches!(value, EntityReference::World(_)) {
            // Native CEntity::RegisterReference skips permanent buildings.
            !self.is_live(value)
        } else {
            self.resolve(value).is_none_or(|entity| cleared.contains(&entity))
        }
    }
    pub(crate) fn membership_changed(&self) -> bool {
        self.index.is_changed() || self.players.as_ref().is_some_and(|players| players.is_changed())
    }
    fn live_entity(&self, entity: Entity) -> bool {
        let Ok((ped, vehicle, object, player)) = self.identities.get(entity) else { return false; };
        if player.is_some() {
            return self.players.as_ref().is_some_and(|players| players.entities.contains(&Some(entity)));
        }
        ped.is_none_or(|ped| self.index.ped(ped.handle) == Some(entity))
            && vehicle.is_none_or(|vehicle| self.index.vehicle(vehicle.handle) == Some(entity))
            && object.is_none_or(|object| self.index.object(object.handle) == Some(entity))
    }

    pub(crate) fn resolve(&self, reference: EntityReference) -> Option<Entity> {
        match reference {
            EntityReference::Entity(entity) => self.live_entity(entity).then_some(entity),
            EntityReference::Ped(handle) => {
                if let Some(entity) = self.index.ped(handle)
                    && let Ok((Some(ped), _, _, None)) = self.identities.get(entity)
                    && ped.handle == handle
                { return Some(entity); }
                self.players.as_ref()?.entities.iter().flatten().copied().find(|entity| {
                    self.identities.get(*entity).ok().and_then(|(_, _, _, player)| player)
                        .is_some_and(|player| player.pool_handle == handle)
                })
            }
            EntityReference::Vehicle(handle) => {
                let entity = self.index.vehicle(handle)?;
                let (_, vehicle, _, _) = self.identities.get(entity).ok()?;
                (vehicle?.handle == handle).then_some(entity)
            }
            EntityReference::Object(handle) => {
                let entity = self.index.object(handle)?;
                let (_, _, object, _) = self.identities.get(entity).ok()?;
                (object?.handle == handle).then_some(entity)
            }
            EntityReference::World(_) => None,
        }
    }

    pub(crate) fn is_live(&self, reference: EntityReference) -> bool {
        match reference {
            EntityReference::World(index) => self.loader.as_ref().is_some_and(|loader| loader.world.entities.get(index).is_some()),
            _ => self.resolve(reference).is_some(),
        }
    }

    pub(crate) fn is_player(&self, reference: EntityReference) -> bool {
        self.resolve(reference).and_then(|entity| self.identities.get(entity).ok())
            .is_some_and(|(_, _, _, player)| player.is_some())
    }

    /// Events whose native culprit is CPed receive a ped handle only after the
    /// referenced entity has actually resolved as a live ped/player.
    pub(crate) fn ped_handle(&self, reference: EntityReference) -> Option<i32> {
        let entity = self.resolve(reference)?;
        let (ped, _, _, player) = self.identities.get(entity).ok()?;
        player.map(|player| player.pool_handle).or_else(|| ped.map(|ped| ped.handle))
    }
}
