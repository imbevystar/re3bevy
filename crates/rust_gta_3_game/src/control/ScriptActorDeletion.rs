//! Native SCM actor retirement through canonical generation-bearing ECS handles.

use super::{
    CarCtrl::CCarCtrl, ScriptEntities::ScriptEntityWorld, ScriptTypes::ScriptCommandEffect,
};
use crate::{
    core::Pools::CPools,
    entities::Entity::eEntityStatus,
    peds::{PedType::ePedType, VehicleRelations::VehicleSeat},
    vehicles::Vehicle::eCarLock,
};

/// COMMAND_DELETE_CHAR and the occupied branch of REMOVE_CHAR_ELEGANTLY.
/// InVehicle is retired with the ped; engine and cruise input remain untouched.
pub(crate) fn delete_script_ped(
    actors: &mut ScriptEntityWorld<'_, '_>,
    pools: &mut CPools,
    handle: i32,
    car_ctrl: Option<&mut CCarCtrl>,
) {
    let Some(ped) = actors.peds.get(&actors.index, &handle) else {
        return;
    };
    let is_cop = ped.ped.ped_type == ePedType::PEDTYPE_COP;
    if let Some(relation) = actors.relation_for_char(handle)
        && relation.seat == VehicleSeat::Driver
        && let Some(vehicle_handle) = actors.vehicle_handle_for_entity(relation.vehicle)
        && let Some(mut vehicle) = actors.vehicles.get_mut(&actors.index, &vehicle_handle)
    {
        let mut base = vehicle.base_mut();
        base.set_status(eEntityStatus::STATUS_ABANDONED);
        if base.entry.door_lock == eCarLock::CARLOCK_LOCKED_INITIALLY {
            base.entry.door_lock = eCarLock::CARLOCK_UNLOCKED;
        }
        if is_cop && base.is_law_enforcement_vehicle() && base.flags.is_law_enforcer {
            base.flags.is_law_enforcer = false;
            if let Some(car_ctrl) = car_ctrl {
                car_ctrl.num_law_enforcer_cars -= 1;
            } else {
                bevy::log::warn_once!("script_delete_driver_missing_car_ctrl");
            }
        }
    }
    actors.remove_char(pools, handle);
}

/// Native DELETE_* null pool lookups complete immediately. Never retain these
/// effects after actor retirement or let an old generation touch a reused slot.
pub(crate) fn apply_script_actor_deletion(
    effect: &ScriptCommandEffect,
    actors: &mut ScriptEntityWorld<'_, '_>,
    pools: &mut CPools,
    car_ctrl: Option<&mut CCarCtrl>,
) -> bool {
    match *effect {
        ScriptCommandEffect::DeleteChar { handle } => {
            delete_script_ped(actors, pools, handle, car_ctrl)
        }
        ScriptCommandEffect::DeleteCar { handle } => actors.remove_car(pools, handle),
        ScriptCommandEffect::DeleteObject { handle } => {
            actors.object_remove(Some(pools), handle);
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        control::ScriptEntities::{
            ScriptEntityIndex, ScriptObjectEntity, ScriptPedEntity, ScriptVehicleEntity,
            script_entity_test_world, spawn_script_test_ped, spawn_script_test_vehicle,
            with_script_entity_test_world,
        },
        control::ScriptTypes::ScriptHandle,
        core::Pools::{CPool, CPoolEntry, CPoolEntryKind},
        modelinfo::ModelIndices::MI_POLICE,
        objects::Object::CObject,
        peds::{
            Ped::{CPed, MISSION_CHAR},
            PedAnimation::PedAnimation,
            PedLocomotion::PedLocomotion,
            PedVitality::PedVitality,
            Population::{CPopulation, refresh_population_counts_from_entities},
            VehicleRelations::InVehicle,
        },
        vehicles::{
            Automobile::CAutomobile,
            Vehicle::{MISSION_VEHICLE, VehicleCore, VehicleDriveState, VehicleEntryState},
        },
    };
    use bevy::{ecs::system::RunSystemOnce, prelude::*};
    use rust_gta_3_core::core::General::CGeneral;

    fn ped(handle: i32, kind: ePedType, general: &mut CGeneral) -> ScriptPedEntity {
        let mut ped = CPed::new(kind, general);
        ped.char_created_by = MISSION_CHAR;
        ScriptPedEntity {
            handle,
            model_id: 0,
            ped,
            vitality: PedVitality::default(),
            locomotion: PedLocomotion::default(),
            animation: PedAnimation::default(),
        }
    }

    #[test]
    fn deleting_cop_driver_and_passenger_preserves_distinct_native_car_effects() {
        let mut world = script_entity_test_world();
        world.insert_resource(CPopulation {
            ped_groups: Vec::new(),
            give_peds_weapons: false,
            all_random_peds_this_type: -1,
            ped_density_multiplier: 1.0,
            total_mission_peds: 0,
            max_number_of_peds_in_use: 25,
            num_civ_male: 0,
            num_civ_female: 0,
            num_cop: 0,
            zone_change_has_happened: false,
            num_emergency: 0,
            countdown_to_peds_at_start: 2,
            gang_counts: [0; 9],
            num_dummy: 0,
            total_peds: 0,
            total_gang_peds: 0,
            total_civ_peds: 0,
            initial_zone_change: None,
            regeneration_point_a: Default::default(),
            regeneration_point_b: Default::default(),
            regeneration_front: Default::default(),
        });
        let mut general = CGeneral::with_seed(11);
        let mut pools = CPools::initialise();
        let driver_handle = pools
            .ped_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Ped))
            .unwrap()
            .0;
        let passenger_handle = pools
            .ped_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Ped))
            .unwrap()
            .0;
        let car_handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))
            .unwrap()
            .0;
        let mut car = CAutomobile::new(
            MI_POLICE as i32,
            MISSION_VEHICLE,
            0,
            &mut general,
            None,
            None,
            false,
        );
        car.vehicle.core.flags.is_law_enforcer = true;
        car.vehicle.core.physical.entity.status = eEntityStatus::STATUS_WRECKED;
        car.vehicle.entry.door_lock = eCarLock::CARLOCK_LOCKED_INITIALLY;
        car.vehicle.drive.engine_on = true;
        car.vehicle.core.auto_pilot.cruise_speed = 23;
        let vehicle = spawn_script_test_vehicle(
            &mut world,
            ScriptVehicleEntity::Automobile {
                handle: car_handle,
                car,
            },
        );
        let driver = spawn_script_test_ped(
            &mut world,
            ped(driver_handle, ePedType::PEDTYPE_COP, &mut general),
        );
        let passenger = spawn_script_test_ped(
            &mut world,
            ped(passenger_handle, ePedType::PEDTYPE_CIVMALE, &mut general),
        );
        world.entity_mut(driver).insert(InVehicle::driver(vehicle));
        world
            .entity_mut(passenger)
            .insert(InVehicle::passenger(vehicle, 0));
        world
            .run_system_once(refresh_population_counts_from_entities)
            .unwrap();
        assert_eq!(world.resource::<CPopulation>().total_mission_peds, 2);
        let mut car_ctrl = CCarCtrl::init();
        car_ctrl.num_law_enforcer_cars = 1;
        with_script_entity_test_world(&mut world, |actors| {
            for _ in 0..2 {
                assert!(apply_script_actor_deletion(
                    &ScriptCommandEffect::DeleteChar {
                        handle: driver_handle
                    },
                    actors,
                    &mut pools,
                    Some(&mut car_ctrl)
                ));
            }
        });
        assert!(world.get_entity(driver).is_err());
        assert_eq!(car_ctrl.num_law_enforcer_cars, 0);
        assert!(
            !world
                .get::<VehicleCore>(vehicle)
                .unwrap()
                .flags
                .is_law_enforcer
        );
        assert_eq!(
            world
                .get::<VehicleCore>(vehicle)
                .unwrap()
                .physical
                .entity
                .status,
            eEntityStatus::STATUS_ABANDONED
        );
        assert_eq!(
            world.get::<VehicleEntryState>(vehicle).unwrap().door_lock,
            eCarLock::CARLOCK_UNLOCKED
        );
        assert!(world.get::<VehicleDriveState>(vehicle).unwrap().engine_on);
        assert_eq!(
            world
                .get::<VehicleCore>(vehicle)
                .unwrap()
                .auto_pilot
                .cruise_speed,
            23
        );
        assert_eq!(
            *world.get::<InVehicle>(passenger).unwrap(),
            InVehicle::passenger(vehicle, 0)
        );
        world
            .run_system_once(refresh_population_counts_from_entities)
            .unwrap();
        assert_eq!(world.resource::<CPopulation>().total_mission_peds, 1);
        world
            .get_mut::<VehicleCore>(vehicle)
            .unwrap()
            .physical
            .entity
            .status = eEntityStatus::STATUS_PHYSICS;
        world
            .get_mut::<VehicleEntryState>(vehicle)
            .unwrap()
            .door_lock = eCarLock::CARLOCK_LOCKED_INITIALLY;
        with_script_entity_test_world(&mut world, |actors| {
            delete_script_ped(actors, &mut pools, passenger_handle, Some(&mut car_ctrl))
        });
        assert_eq!(
            world
                .get::<VehicleCore>(vehicle)
                .unwrap()
                .physical
                .entity
                .status,
            eEntityStatus::STATUS_PHYSICS
        );
        assert_eq!(
            world.get::<VehicleEntryState>(vehicle).unwrap().door_lock,
            eCarLock::CARLOCK_LOCKED_INITIALLY
        );
        world
            .run_system_once(refresh_population_counts_from_entities)
            .unwrap();
        assert_eq!(world.resource::<CPopulation>().total_mission_peds, 0);
    }

    #[test]
    fn repeated_deletes_preserve_reused_generations_and_equal_handles_in_other_pools() {
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(13);
        let mut pools = CPools::initialise();
        pools.ped_pool = CPool::new(1);
        pools.vehicle_pool = CPool::new(1);
        pools.object_pool = CPool::new(1);
        let mut old_handle = None;
        for _ in 0..2 {
            let handle = pools
                .ped_pool
                .new_entry(CPoolEntry::new(CPoolEntryKind::Ped))
                .unwrap()
                .0;
            assert_eq!(
                pools
                    .vehicle_pool
                    .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))
                    .unwrap()
                    .0,
                handle
            );
            assert_eq!(
                pools
                    .object_pool
                    .new_entry(CPoolEntry::new(CPoolEntryKind::Object))
                    .unwrap()
                    .0,
                handle
            );
            let actor = spawn_script_test_ped(
                &mut world,
                ped(handle, ePedType::PEDTYPE_CIVMALE, &mut general),
            );
            let car = CAutomobile::new(
                MI_POLICE as i32,
                MISSION_VEHICLE,
                0,
                &mut general,
                None,
                None,
                false,
            );
            let vehicle = spawn_script_test_vehicle(
                &mut world,
                ScriptVehicleEntity::Automobile { handle, car },
            );
            let object = world
                .spawn((
                    ScriptHandle(handle),
                    ScriptObjectEntity {
                        handle,
                        object: CObject::new(&mut general),
                    }
                    .into_bundle(),
                ))
                .id();
            world
                .resource_mut::<ScriptEntityIndex>()
                .objects
                .insert(handle, object);
            if let Some(stale) = old_handle {
                assert_ne!(handle, stale);
                with_script_entity_test_world(&mut world, |actors| {
                    for effect in [
                        ScriptCommandEffect::DeleteChar { handle: stale },
                        ScriptCommandEffect::DeleteCar { handle: stale },
                        ScriptCommandEffect::DeleteObject { handle: stale },
                    ] {
                        assert!(apply_script_actor_deletion(
                            &effect, actors, &mut pools, None
                        ));
                    }
                });
                for entity in [actor, vehicle, object] {
                    assert!(world.get_entity(entity).is_ok());
                }
            }
            with_script_entity_test_world(&mut world, |actors| {
                assert!(apply_script_actor_deletion(
                    &ScriptCommandEffect::DeleteObject { handle },
                    actors,
                    &mut pools,
                    None
                ));
                assert!(actors.peds.contains_key(&actors.index, &handle));
                assert!(actors.vehicles.contains_key(&actors.index, &handle));
                for _ in 0..3 {
                    for effect in [
                        ScriptCommandEffect::DeleteChar { handle },
                        ScriptCommandEffect::DeleteCar { handle },
                        ScriptCommandEffect::DeleteObject { handle },
                    ] {
                        assert!(apply_script_actor_deletion(
                            &effect, actors, &mut pools, None
                        ));
                    }
                }
            });
            for entity in [actor, vehicle, object] {
                assert!(world.get_entity(entity).is_err());
            }
            old_handle = Some(handle);
        }
    }
}
