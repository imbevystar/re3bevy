//! Registered physical damage-pointer lifetime on the actual ECS owners.
use super::EntityReference::{EntityReferenceAccess, EntityReferencesCleared};
use crate::{
    control::ScriptEntities::{ScriptObjectIdentity, ScriptPedIdentity, ScriptVehicleIdentity},
    objects::Object::CObject,
    peds::{Ped::CPed, PlayerPed::PlayerPedState},
    vehicles::Vehicle::VehicleCore,
};
use bevy::{ecs::query::QueryFilter, prelude::*};

fn clear_owners<P: QueryFilter, V: QueryFilter, O: QueryFilter>(
    peds: &mut Query<&mut CPed, P>,
    vehicles: &mut Query<&mut VehicleCore, V>,
    objects: &mut Query<&mut CObject, O>,
    references: &EntityReferenceAccess,
    targets: &[Entity],
) {
    for mut ped in peds {
        if ped
            .physical
            .damage
            .entity
            .is_some_and(|value| references.registered_should_clear(value, targets))
        {
            ped.physical.damage.entity = None;
        }
    }
    for mut vehicle in vehicles {
        if vehicle
            .physical
            .damage
            .entity
            .is_some_and(|value| references.registered_should_clear(value, targets))
        {
            vehicle.physical.damage.entity = None;
        }
    }
    for mut object in objects {
        if object
            .physical
            .damage
            .entity
            .is_some_and(|value| references.registered_should_clear(value, targets))
        {
            object.physical.damage.entity = None;
        }
    }
}

/// No actor poses or pool contents are copied. Collision records are native
/// unregistered history; only SetDamagedPieceRecord's registered pointer clears.
pub(crate) fn retire_physical_damage_references(
    mut owners: ParamSet<(
        (
            Query<&mut CPed, Changed<CPed>>,
            Query<&mut VehicleCore, Changed<VehicleCore>>,
            Query<&mut CObject, Changed<CObject>>,
        ),
        (
            Query<&mut CPed>,
            Query<&mut VehicleCore>,
            Query<&mut CObject>,
        ),
    )>,
    references: EntityReferenceAccess,
    mut removed_peds: RemovedComponents<ScriptPedIdentity>,
    mut removed_vehicles: RemovedComponents<ScriptVehicleIdentity>,
    mut removed_objects: RemovedComponents<ScriptObjectIdentity>,
    mut removed_players: RemovedComponents<PlayerPedState>,
) {
    let retired = removed_peds.read().count()
        + removed_vehicles.read().count()
        + removed_objects.read().count()
        + removed_players.read().count();
    if retired != 0 || references.membership_changed() {
        let (mut peds, mut vehicles, mut objects) = owners.p1();
        clear_owners(&mut peds, &mut vehicles, &mut objects, &references, &[]);
    } else {
        let (mut peds, mut vehicles, mut objects) = owners.p0();
        clear_owners(&mut peds, &mut vehicles, &mut objects, &references, &[]);
    }
}

pub(crate) fn clear_physical_reference_targets(
    mut peds: Query<&mut CPed>,
    mut vehicles: Query<&mut VehicleCore>,
    mut objects: Query<&mut CObject>,
    references: EntityReferenceAccess,
    mut cleared: MessageReader<EntityReferencesCleared>,
) {
    let targets: Vec<_> = cleared.read().map(|message| message.0).collect();
    if !targets.is_empty() {
        clear_owners(
            &mut peds,
            &mut vehicles,
            &mut objects,
            &references,
            &targets,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        control::ScriptEntities::{ScriptEntityIndex, ScriptVehicleKind},
        core::FileLoader::CFileLoader,
        entities::EntityReference::EntityReference,
        peds::PedType::ePedType,
        vehicles::{
            Automobile::{AutomobileState, CAutomobile},
            Vehicle::VehicleVitality,
        },
        weapons::Explosion::{
            CExplosionPool, clear_explosion_reference_targets, eExplosionType,
            retire_explosion_references,
        },
    };
    use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};
    use rust_gta_3_formats::formats::FileLoader::GtaIplInstance;

    #[test]
    fn explicit_live_target_clear_preserves_pool_kind_actor_lifetime_and_unregistered_history() {
        let mut app = App::new();
        app.add_message::<EntityReferencesCleared>().add_systems(
            Update,
            (
                retire_physical_damage_references,
                clear_physical_reference_targets,
                crate::vehicles::retire_vehicle_damage_references,
                crate::vehicles::clear_vehicle_damage_reference_targets,
                retire_explosion_references,
                clear_explosion_reference_targets,
            ),
        );
        let mut timer = CTimer::initialise();
        timer.start_user_pause();
        app.insert_resource(timer);
        let mut loader = CFileLoader::default();
        loader.model_info.add_simple_model(1000);
        loader
            .world
            .load_object_instance(
                &GtaIplInstance {
                    id: 1000,
                    name: "unit_permanent_building".into(),
                    position: [0.0; 3],
                    scale: [1.0; 3],
                    rotation_axis: [0.0; 3],
                    angle: 1.0,
                },
                &loader.model_info,
                &Default::default(),
            )
            .unwrap();
        app.insert_resource(loader);
        let handle = 257;
        let mut general = CGeneral::with_seed(7);
        let mut ped_owner = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        ped_owner.physical.damage.entity = Some(EntityReference::Vehicle(handle));
        ped_owner.physical.damage.impulse = 23.0;
        ped_owner.physical.collision_records = vec![
            EntityReference::Ped(handle),
            EntityReference::Vehicle(handle),
        ];
        let ped = app
            .world_mut()
            .spawn((
                ped_owner,
                ScriptPedIdentity {
                    handle,
                    model_id: 0,
                },
            ))
            .id();
        let mut car = CAutomobile::new(90, 1, 0, &mut general, None, None, false);
        car.vehicle.core.physical.damage.entity = Some(EntityReference::Ped(handle));
        car.vehicle.vitality.blow_up_entity = Some(EntityReference::Ped(handle));
        car.state.set_on_fire_entity = Some(EntityReference::Vehicle(handle));
        let (core, vitality, _, _, _, _) = car.vehicle.into_parts();
        let vehicle = app
            .world_mut()
            .spawn((
                core,
                vitality,
                car.state.0,
                ScriptVehicleIdentity {
                    handle,
                    kind: ScriptVehicleKind::Automobile,
                },
            ))
            .id();
        let mut object_owner = CObject::new(&mut general);
        object_owner.physical.damage.entity = Some(EntityReference::Entity(ped));
        let object = app
            .world_mut()
            .spawn((object_owner, ScriptObjectIdentity { handle }))
            .id();
        let mut permanent_owner = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        permanent_owner.physical.damage.entity = Some(EntityReference::World(0));
        let permanent = app.world_mut().spawn(permanent_owner).id();
        let mut index = ScriptEntityIndex::default();
        index.peds.insert(handle, ped);
        index.vehicles.insert(handle, vehicle);
        index.objects.insert(handle, object);
        app.insert_resource(index);
        let mut explosions = CExplosionPool::Initialise();
        assert!(explosions.AddExplosion(
            Some(EntityReference::Vehicle(handle)),
            Some(EntityReference::Ped(handle)),
            eExplosionType::EXPLOSION_CAR,
            CVector::ZERO,
            0,
            100
        ));
        app.insert_resource(explosions);
        app.update();
        app.world_mut().write_message(EntityReferencesCleared(ped));
        app.update();
        assert!(app.world().get_entity(ped).is_ok() && app.world().get_entity(vehicle).is_ok());
        assert_eq!(
            app.world().get::<CPed>(ped).unwrap().physical.damage.entity,
            Some(EntityReference::Vehicle(handle)),
            "equal numeric vehicle reference must survive"
        );
        assert_eq!(
            app.world()
                .get::<VehicleCore>(vehicle)
                .unwrap()
                .physical
                .damage
                .entity,
            None
        );
        assert_eq!(
            app.world()
                .get::<VehicleVitality>(vehicle)
                .unwrap()
                .blow_up_entity,
            None
        );
        assert_eq!(
            app.world()
                .get::<AutomobileState>(vehicle)
                .unwrap()
                .set_on_fire_entity,
            Some(EntityReference::Vehicle(handle))
        );
        assert_eq!(
            app.world()
                .get::<CObject>(object)
                .unwrap()
                .physical
                .damage
                .entity,
            None
        );
        assert_eq!(
            app.world()
                .get::<CPed>(permanent)
                .unwrap()
                .physical
                .damage
                .entity,
            Some(EntityReference::World(0))
        );
        let explosion = app.world().resource::<CExplosionPool>().explosions[0].unwrap();
        assert_eq!(explosion.creator_entity, None);
        assert_eq!(
            explosion.victim_entity,
            Some(EntityReference::Vehicle(handle))
        );
        assert!(
            app.world()
                .resource::<CExplosionPool>()
                .pending_intents
                .iter()
                .all(|intent| {
                    let (crate::weapons::Explosion::ExplosionUpdateIntent::TriggerWorldExplosion {
                        creator,
                        ..
                    }
                    | crate::weapons::Explosion::ExplosionUpdateIntent::SetPedsAndCarsOnFire {
                        creator,
                        ..
                    }
                    | crate::weapons::Explosion::ExplosionUpdateIntent::StartGroundFire {
                        creator,
                        ..
                    }) = intent;
                    creator.is_none()
                })
        );
        app.world_mut()
            .write_message(EntityReferencesCleared(vehicle));
        app.update();
        assert_eq!(
            app.world().get::<CPed>(ped).unwrap().physical.damage.entity,
            None
        );
        assert_eq!(
            app.world()
                .get::<CPed>(ped)
                .unwrap()
                .physical
                .damage
                .impulse,
            23.0
        );
        assert_eq!(
            app.world()
                .get::<CPed>(ped)
                .unwrap()
                .physical
                .collision_records,
            vec![
                EntityReference::Ped(handle),
                EntityReference::Vehicle(handle)
            ]
        );
        assert_eq!(
            app.world()
                .get::<AutomobileState>(vehicle)
                .unwrap()
                .set_on_fire_entity,
            None
        );
        assert_eq!(
            app.world().resource::<CExplosionPool>().explosions[0]
                .unwrap()
                .victim_entity,
            None
        );
        assert!(
            app.world().get_entity(vehicle).is_ok(),
            "ResolveReferences never retires the target"
        );
        app.world_mut()
            .get_mut::<CPed>(ped)
            .unwrap()
            .physical
            .damage
            .entity = Some(EntityReference::Vehicle(handle));
        app.world_mut().despawn(vehicle);
        app.update();
        assert_eq!(
            app.world().get::<CPed>(ped).unwrap().physical.damage.entity,
            None
        );
    }
}
