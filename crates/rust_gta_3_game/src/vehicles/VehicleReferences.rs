//! CEntity::ResolveReferences for actor-owned delayed vehicle fire culprits.
use crate::{
    entities::EntityReference::{EntityReferenceAccess, EntityReferencesCleared},
    vehicles::{Automobile::AutomobileState, Boat::BoatState, Vehicle::VehicleVitality},
};
use bevy::prelude::*;

type VehicleReferenceOwners<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut VehicleVitality,
        Option<&'static mut AutomobileState>,
        Option<&'static mut BoatState>,
    ),
>;

fn clear_vehicle_references(
    vehicles: &mut VehicleReferenceOwners,
    references: &EntityReferenceAccess,
    targets: &[Entity],
) {
    for (mut vitality, automobile, boat) in vehicles {
        if vitality
            .blow_up_entity
            .is_some_and(|value| references.registered_should_clear(value, targets))
        {
            vitality.blow_up_entity = None;
        }
        if let Some(mut automobile) = automobile {
            if automobile
                .set_on_fire_entity
                .is_some_and(|value| references.registered_should_clear(value, targets))
            {
                automobile.set_on_fire_entity = None;
            }
        }
        if let Some(mut boat) = boat {
            if boat
                .set_on_fire_entity
                .is_some_and(|value| references.registered_should_clear(value, targets))
            {
                boat.set_on_fire_entity = None;
            }
        }
    }
}

pub(crate) fn retire_vehicle_damage_references(
    mut vehicles: VehicleReferenceOwners,
    references: EntityReferenceAccess,
) {
    clear_vehicle_references(&mut vehicles, &references, &[]);
}

pub(crate) fn clear_vehicle_damage_reference_targets(
    mut vehicles: VehicleReferenceOwners,
    references: EntityReferenceAccess,
    mut cleared: MessageReader<EntityReferencesCleared>,
) {
    let targets: Vec<_> = cleared.read().map(|message| message.0).collect();
    if !targets.is_empty() {
        clear_vehicle_references(&mut vehicles, &references, &targets);
    }
}
