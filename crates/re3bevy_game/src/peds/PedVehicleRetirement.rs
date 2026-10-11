//! Vehicle-side CPed destructor effects, owned by Bevy actor retirement.
use crate::{
    animation::PedVehicleAnimation::CPedVehicleAnimation,
    peds::{
        Ped::{CPed, PedState, eObjective},
        VehicleRelations::{InVehicle, PedEntity, automobile_door_flag},
    },
    vehicles::Vehicle::{VehicleCore, VehicleEntryState},
};
use bevy::prelude::*;

pub(crate) fn release_entry_reservation(
    entry: &mut VehicleEntryState,
    core: &mut VehicleCore,
    door: usize,
    car_jacked: bool,
) {
    if car_jacked {
        entry.is_being_car_jacked = false;
    }
    if entry.num_getting_in != 0 {
        entry.num_getting_in -= 1;
    }
    entry.getting_in_flags &= !automobile_door_flag(door);
    if core.auto_pilot.cruise_speed == 0 {
        core.auto_pilot.cruise_speed = 17;
    }
}

/// CPed::~CPed / QuitEnteringCar: release only this actor's transient door
/// reservation. A despawn observer remains active during pause and loading.
pub(crate) fn retire_ped_vehicle_reservations(
    event: On<Despawn<PedEntity>>,
    peds: Query<(&CPed, Option<&InVehicle>, Option<&CPedVehicleAnimation>)>,
    mut vehicles: Query<(&mut VehicleEntryState, &mut VehicleCore)>,
) {
    let Ok((ped, occupant, transition)) = peds.get(event.entity) else {
        return;
    };
    if let Some(occupant) = occupant {
        if matches!(
            ped.ped_state,
            PedState::PED_EXIT_CAR | PedState::PED_DRAG_FROM_CAR
        ) && let Ok((mut entry, _)) = vehicles.get_mut(occupant.vehicle)
        {
            entry.getting_out_flags &= !automobile_door_flag(ped.veh_door as usize);
        }
    } else if matches!(
        ped.ped_state,
        PedState::PED_ENTER_CAR | PedState::PED_CARJACK
    ) {
        let Some(transition) = transition else {
            warn_once!("ped_retirement_missing_entry_vehicle");
            return;
        };
        let Ok((mut entry, mut core)) = vehicles.get_mut(transition.vehicle) else {
            return;
        };
        release_entry_reservation(
            &mut entry,
            &mut core,
            transition.door,
            ped.objective == eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                || ped.ped_state == PedState::PED_CARJACK,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{
            AnimBlendAssociation::CAnimBlendAssociation,
            PedVehicleAnimation::PedVehicleAnimationPhase,
        },
        control::ScriptEntities::{
            ScriptVehicleEntity, script_entity_test_world, spawn_script_test_vehicle,
        },
        peds::PedType::ePedType,
        vehicles::{Automobile::CAutomobile, Vehicle::MISSION_VEHICLE},
    };
    use re3bevy_core::core::General::CGeneral;

    #[test]
    fn despawn_releases_entry_then_exit_without_touching_another_door() {
        let mut world = script_entity_test_world();
        world.add_observer(retire_ped_vehicle_reservations);
        let mut general = CGeneral::with_seed(7);
        let car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        let vehicle = spawn_script_test_vehicle(
            &mut world,
            ScriptVehicleEntity::Automobile { handle: 1, car },
        );
        let entry_flag = automobile_door_flag(2);
        let other_flag = automobile_door_flag(3);
        {
            let mut entry = world.get_mut::<VehicleEntryState>(vehicle).unwrap();
            entry.num_getting_in = 2;
            entry.getting_in_flags = entry_flag | other_flag;
            entry.is_being_car_jacked = true;
        }
        world
            .get_mut::<VehicleCore>(vehicle)
            .unwrap()
            .auto_pilot
            .cruise_speed = 0;
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        ped.ped_state = PedState::PED_CARJACK;
        let actor = world
            .spawn((
                PedEntity,
                ped,
                CPedVehicleAnimation {
                    vehicle,
                    door: 2,
                    association: CAnimBlendAssociation::new(),
                    phase: PedVehicleAnimationPhase::Align,
                    enter_as_driver: true,
                    dragged_out_finish_ran: false,
                },
            ))
            .id();
        world.despawn(actor);
        let entry = world.get::<VehicleEntryState>(vehicle).unwrap();
        assert_eq!(entry.num_getting_in, 1);
        assert_eq!(entry.getting_in_flags, other_flag);
        assert!(!entry.is_being_car_jacked);
        assert_eq!(
            world
                .get::<VehicleCore>(vehicle)
                .unwrap()
                .auto_pilot
                .cruise_speed,
            17
        );
        world
            .get_mut::<VehicleEntryState>(vehicle)
            .unwrap()
            .getting_out_flags = entry_flag | other_flag;
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        ped.ped_state = PedState::PED_EXIT_CAR;
        ped.veh_door = 2;
        let actor = world
            .spawn((PedEntity, ped, InVehicle::driver(vehicle)))
            .id();
        world.despawn(actor);
        assert_eq!(
            world
                .get::<VehicleEntryState>(vehicle)
                .unwrap()
                .getting_out_flags,
            other_flag
        );
    }
}
