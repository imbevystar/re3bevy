//! Combat access to canonical actor components without script-world mutation.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::WeaponRaycast::WeaponRaycastActors;
use crate::{
    control::ScriptEntities::{
        ScriptEntityIndex, ScriptObjectIdentity, ScriptPedIdentity, ScriptPedMut, ScriptPedRef,
        ScriptVehicleIdentity,
    },
    entities::Entity::CEntity,
    objects::Object::CObject,
    peds::{
        Ped::CPed, PedAnimation::PedAnimation, PedLocomotion::PedLocomotion,
        PedVitality::PedVitality, PlayerPed::PlayerPedState,
    },
    vehicles::Vehicle::{VehicleCore, VehicleVitality},
};

#[derive(SystemParam)]
pub(crate) struct WeaponActorAccess<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    peds: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptPedIdentity,
            &'static mut CPed,
            &'static mut PedVitality,
            &'static mut PedLocomotion,
            &'static mut PedAnimation,
        ),
        Without<PlayerPedState>,
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
    objects: Query<'w, 's, (Entity, &'static ScriptObjectIdentity, &'static mut CObject)>,
}

impl WeaponActorAccess<'_, '_> {
    pub(crate) fn ped_handle_for_slot(&self, slot: usize) -> Option<i32> {
        self.index.ped_handle_for_slot(slot)
    }

    pub(crate) fn ped(&self, handle: i32) -> Option<ScriptPedRef<'_>> {
        let entity = self.index.ped(handle)?;
        let (_, identity, ped, vitality, locomotion, animation) = self.peds.get(entity).ok()?;
        (identity.handle == handle).then_some(ScriptPedRef {
            handle,
            model_id: identity.model_id,
            ped,
            vitality,
            locomotion,
            animation,
        })
    }

    pub(crate) fn ped_mut(&mut self, handle: i32) -> Option<ScriptPedMut<'_>> {
        let entity = self.index.ped(handle)?;
        let (_, identity, ped, vitality, locomotion, animation) = self.peds.get_mut(entity).ok()?;
        (identity.handle == handle).then(|| ScriptPedMut {
            handle,
            model_id: identity.model_id,
            ped: ped.into_inner(),
            vitality: vitality.into_inner(),
            locomotion: locomotion.into_inner(),
            animation: animation.into_inner(),
        })
    }

    pub(crate) fn peds(&self) -> impl Iterator<Item = (i32, ScriptPedRef<'_>)> {
        self.peds.iter().filter_map(
            move |(entity, identity, ped, vitality, locomotion, animation)| {
                (self.index.ped(identity.handle) == Some(entity)).then_some((
                    identity.handle,
                    ScriptPedRef {
                        handle: identity.handle,
                        model_id: identity.model_id,
                        ped,
                        vitality,
                        locomotion,
                        animation,
                    },
                ))
            },
        )
    }

    pub(crate) fn peds_mut(&mut self) -> impl Iterator<Item = (i32, ScriptPedMut<'_>)> {
        let index = &self.index;
        self.peds.iter_mut().filter_map(
            move |(entity, identity, ped, vitality, locomotion, animation)| {
                (index.ped(identity.handle) == Some(entity)).then(|| {
                    (
                        identity.handle,
                        ScriptPedMut {
                            handle: identity.handle,
                            model_id: identity.model_id,
                            ped: ped.into_inner(),
                            vitality: vitality.into_inner(),
                            locomotion: locomotion.into_inner(),
                            animation: animation.into_inner(),
                        },
                    )
                })
            },
        )
    }

    pub(crate) fn vehicle_damage(
        &mut self,
        handle: i32,
    ) -> Option<(&VehicleCore, &mut VehicleVitality)> {
        let entity = self.index.vehicle(handle)?;
        let (_, identity, core, vitality) = self.vehicles.get_mut(entity).ok()?;
        (identity.handle == handle).then(|| (core, vitality.into_inner()))
    }

    pub(crate) fn vehicle_core(&self, handle: i32) -> Option<&VehicleCore> {
        let entity = self.index.vehicle(handle)?;
        let (_, identity, core, _) = self.vehicles.get(entity).ok()?;
        (identity.handle == handle).then_some(core)
    }

    pub(crate) fn object(&self, handle: i32) -> Option<&CObject> {
        let entity = self.index.object(handle)?;
        let (_, identity, object) = self.objects.get(entity).ok()?;
        (identity.handle == handle).then_some(object)
    }

    pub(crate) fn object_mut(&mut self, handle: i32) -> Option<&mut CObject> {
        let entity = self.index.object(handle)?;
        let (_, identity, object) = self.objects.get_mut(entity).ok()?;
        (identity.handle == handle).then(|| object.into_inner())
    }
}

impl WeaponRaycastActors for WeaponActorAccess<'_, '_> {
    fn ray_peds(&self) -> impl Iterator<Item = (i32, &CPed, &PedAnimation)> {
        self.peds()
            .map(|(handle, actor)| (handle, actor.ped, actor.animation))
    }

    fn ray_vehicles(&self) -> impl Iterator<Item = (i32, &CEntity)> {
        self.vehicles
            .iter()
            .filter_map(move |(entity, identity, core, _)| {
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
