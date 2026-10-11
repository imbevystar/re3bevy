//! Immutable collision access for projectiles and other weapon line queries.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::WeaponRaycast::WeaponRaycastActors;
use crate::{
    control::ScriptEntities::{
        ScriptEntityIndex, ScriptObjectIdentity, ScriptPedIdentity, ScriptVehicleIdentity,
    },
    entities::Entity::CEntity,
    objects::Object::CObject,
    peds::{Ped::CPed, PedAnimation::PedAnimation, PlayerPed::PlayerPedState},
    vehicles::Vehicle::VehicleCore,
};

#[derive(SystemParam)]
pub(crate) struct WeaponRaycastAccess<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    peds: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptPedIdentity,
            &'static CPed,
            &'static PedAnimation,
        ),
        Without<PlayerPedState>,
    >,
    vehicles: Query<'w, 's, (Entity, &'static ScriptVehicleIdentity, &'static VehicleCore)>,
    objects: Query<'w, 's, (Entity, &'static ScriptObjectIdentity, &'static CObject)>,
}

impl WeaponRaycastActors for WeaponRaycastAccess<'_, '_> {
    fn ray_peds(&self) -> impl Iterator<Item = (i32, &CPed, &PedAnimation)> {
        self.peds
            .iter()
            .filter_map(move |(entity, identity, ped, animation)| {
                (self.index.ped(identity.handle) == Some(entity)).then_some((
                    identity.handle,
                    ped,
                    animation,
                ))
            })
    }

    fn ray_vehicles(&self) -> impl Iterator<Item = (i32, &CEntity)> {
        self.vehicles
            .iter()
            .filter_map(move |(entity, identity, core)| {
                (self.index.vehicle(identity.handle) == Some(entity))
                    .then_some((identity.handle, &core.physical.entity))
            })
    }

    fn ray_objects(&self) -> impl Iterator<Item = (i32, &CEntity)> {
        self.objects
            .iter()
            .filter_map(move |(entity, identity, object)| {
                (self.index.object(identity.handle) == Some(entity))
                    .then_some((identity.handle, &object.physical.entity))
            })
    }
}
