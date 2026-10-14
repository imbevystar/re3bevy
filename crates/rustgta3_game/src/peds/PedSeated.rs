//! CPed::PedSetInCarCB / StopNonPartialAnims on the canonical actor clump.
use bevy::prelude::*;
use rustgta3_core::core::Timer::CTimer;
use crate::{
    animation::{AnimManager::CAnimManager, PedVehicleAnimation::CPedVehicleAnimation},
    vehicles::Vehicle::VehicleCore,
};
use super::{
    Ped::{CPed, eMoveState},
    PedAnimation::PedAnimation,
    VehicleRelations::{InVehicle, PedEntity},
};

pub(crate) fn process_seated_ped_animations(
    mut peds: Query<(&mut CPed, &mut PedAnimation, &InVehicle), (With<PedEntity>, Without<CPedVehicleAnimation>)>,
    vehicles: Query<&VehicleCore>,
    manager: Res<CAnimManager>,
    timer: Res<CTimer>,
) {
    for (mut ped, mut animation, relation) in &mut peds {
        let Ok(vehicle) = vehicles.get(relation.vehicle) else {
            bevy::log::warn_once!("seated_ped_missing_live_vehicle");
            continue;
        };
        if !animation.install_seated_pose(&manager, !relation.is_driver(), vehicle.flags.low_vehicle) {
            bevy::log::warn_once!("seated_ped_missing_original_association");
            continue;
        }
        // A later exit must select locomotion again rather than keep a stopped
        // WALK/RUN association whose stored state still matches the request.
        ped.stored_move_state = eMoveState::PEDMOVE_NONE;
        animation.movement_anim_clump.associations.retain_mut(|association| {
            association.update_blend(timer.time_step_in_seconds())
        });
    }
}
