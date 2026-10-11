//! Blast/fire access to live physical and vitality owners.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    control::ScriptEntities::{
        ScriptEntityIndex, ScriptObjectIdentity, ScriptPedIdentity, ScriptVehicleIdentity,
    },
    objects::Object::CObject,
    peds::{
        Ped::CPed, PedLocomotion::PedLocomotion, PedVitality::PedVitality,
        PlayerPed::PlayerPedState, VehicleRelations::InVehicle,
    },
    vehicles::{
        Automobile::AutomobileState,
        Vehicle::{VehicleCore, VehicleVitality},
    },
};

pub(crate) struct ExplosionPedMut<'a> {
    pub entity: Entity,
    pub handle: i32,
    pub ped: &'a mut CPed,
    pub vitality: &'a mut PedVitality,
    pub locomotion: &'a PedLocomotion,
    pub seated: bool,
}

pub(crate) struct ExplosionVehicleMut<'a> {
    pub entity: Entity,
    pub identity: &'a ScriptVehicleIdentity,
    pub core: &'a mut VehicleCore,
    pub vitality: &'a mut VehicleVitality,
    pub automobile: Option<&'a AutomobileState>,
}

#[derive(SystemParam)]
pub(crate) struct ExplosionActorAccess<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    peds: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptPedIdentity,
            &'static mut CPed,
            &'static mut PedVitality,
            &'static PedLocomotion,
            Has<InVehicle>,
        ),
        Without<PlayerPedState>,
    >,
    vehicles: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptVehicleIdentity,
            &'static mut VehicleCore,
            &'static mut VehicleVitality,
            Option<&'static AutomobileState>,
        ),
    >,
    objects: Query<'w, 's, (Entity, &'static ScriptObjectIdentity, &'static mut CObject)>,
}

impl ExplosionActorAccess<'_, '_> {
    pub(crate) fn ped_entity(&self, handle: i32) -> Option<Entity> {
        let entity = self.index.ped(handle)?;
        let (_, identity, _, _, _, _) = self.peds.get(entity).ok()?;
        (identity.handle == handle).then_some(entity)
    }

    pub(crate) fn vehicle_entity(&self, handle: i32) -> Option<Entity> {
        let entity = self.index.vehicle(handle)?;
        let (_, identity, _, _, _) = self.vehicles.get(entity).ok()?;
        (identity.handle == handle).then_some(entity)
    }

    pub(crate) fn peds_mut(&mut self) -> impl Iterator<Item = ExplosionPedMut<'_>> {
        let index = &self.index;
        self.peds.iter_mut().filter_map(
            move |(entity, identity, ped, vitality, locomotion, seated)| {
                (index.ped(identity.handle) == Some(entity)).then(|| ExplosionPedMut {
                    entity,
                    handle: identity.handle,
                    ped: ped.into_inner(),
                    vitality: vitality.into_inner(),
                    locomotion,
                    seated,
                })
            },
        )
    }

    pub(crate) fn vehicles_mut(&mut self) -> impl Iterator<Item = ExplosionVehicleMut<'_>> {
        let index = &self.index;
        self.vehicles.iter_mut().filter_map(
            move |(entity, identity, core, vitality, automobile)| {
                (index.vehicle(identity.handle) == Some(entity)).then(|| ExplosionVehicleMut {
                    entity,
                    identity,
                    core: core.into_inner(),
                    vitality: vitality.into_inner(),
                    automobile,
                })
            },
        )
    }

    pub(crate) fn objects_mut(&mut self) -> impl Iterator<Item = (i32, &mut CObject)> {
        let index = &self.index;
        self.objects
            .iter_mut()
            .filter_map(move |(entity, identity, object)| {
                (index.object(identity.handle) == Some(entity))
                    .then(|| (identity.handle, object.into_inner()))
            })
    }
}
