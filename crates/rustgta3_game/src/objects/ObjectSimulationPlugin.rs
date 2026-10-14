//! System ownership for the object branch of re3 `CGame::Process`.
//! Cross-domain dependencies preserve the production schedule and deferred-command barriers.

use crate::objects::Object::process_script_object_control;
use crate::objects::Object::process_script_vehicle_debris;
use crate::peds::VehicleRelations::process_script_vehicle_doors;
use crate::core::World::world_actor_processing_enabled;
use bevy::prelude::*;
use rustgta3_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

pub struct GtaObjectSimulationPlugin;

impl Plugin for GtaObjectSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            process_script_object_control
                .after(process_script_vehicle_doors)
                .in_set(GtaProcessSet::World)
                .run_if(world_actor_processing_enabled)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            process_script_vehicle_debris
                .after(process_script_object_control)
                .in_set(GtaProcessSet::World)
                .run_if(world_actor_processing_enabled)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            crate::objects::Object::sync_temporary_object_count
                .after(process_script_object_control)
                .after(process_script_vehicle_debris),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::Object::{CObject, CObjectGlobalState, TEMP_OBJECT, sync_temporary_object_count};
    use rustgta3_core::core::{General::CGeneral, Timer::CTimer};

    #[test]
    fn temporary_count_tracks_conversion_and_retirement_through_a_long_pause() {
        let mut app = App::new();
        app.init_resource::<CObjectGlobalState>().init_resource::<CTimer>()
            .add_systems(Update, sync_temporary_object_count);
        app.world_mut().resource_mut::<CTimer>().start_user_pause();
        let mut general = CGeneral::with_seed(5);
        let actor = app.world_mut().spawn(CObject::new(&mut general)).id();
        app.update();
        assert_eq!(app.world().resource::<CObjectGlobalState>().no_temp_objects, 0);
        app.world_mut().get_mut::<CObject>(actor).unwrap().object_created_by = TEMP_OBJECT;
        app.update();
        assert_eq!(app.world().resource::<CObjectGlobalState>().no_temp_objects, 1);
        app.world_mut().despawn(actor);
        for _ in 0..8 { app.update(); }
        assert!(app.world().resource::<CTimer>().get_is_user_paused());
        assert_eq!(app.world().resource::<CObjectGlobalState>().no_temp_objects, 0);
    }
}
