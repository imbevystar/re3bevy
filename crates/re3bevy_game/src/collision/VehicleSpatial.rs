//! CPhysical sector membership for production vehicle contacts.
use crate::core::World::world_actor_processing_enabled;
use crate::{
    control::ScriptEntities::ScriptVehicleIdentity,
    core::{EntitySectorIndex::EntitySectorIndex, FileLoader::CFileLoader},
    vehicles::Vehicle::VehicleCore,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use re3bevy_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

/// Cross-domain ordering around live sector membership and body response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum ActorCollisionSet {
    Membership,
    Response,
}

#[derive(Default, Resource)]
pub(crate) struct VehicleCollisionSpatial {
    pub grid: EntitySectorIndex,
    pub missing_bounds: u64,
    pub candidate_visits: u64,
}

impl VehicleCollisionSpatial {
    pub(crate) fn refresh(&mut self, entity: Entity, core: &VehicleCore, loader: &CFileLoader) {
        let model = usize::try_from(core.physical.entity.model_index)
            .ok()
            .and_then(|model| loader.model_info.col_model(model));
        if !model.is_some_and(|model| {
            self.grid
                .update_rect(entity, core.physical.get_bound_rect(model))
        }) {
            self.grid.remove(entity);
            self.missing_bounds += 1;
            warn_once!("vehicle_collision_waiting_original_bounds");
        }
    }
}

/// Both the pedestrian and post-vehicle passes maintain the same sector index.
/// Each has its own change/removal cursor, so the later pass also sees vehicles
/// moved by control and physics after the pedestrian pass.
#[derive(SystemParam)]
pub(crate) struct VehicleCollisionMembership<'w, 's> {
    spatial: ResMut<'w, VehicleCollisionSpatial>,
    loader: Res<'w, CFileLoader>,
    changed: Query<
        'w,
        's,
        (Entity, &'static VehicleCore),
        (With<ScriptVehicleIdentity>, Or<(Changed<VehicleCore>, Added<ScriptVehicleIdentity>)>),
    >,
    removed_core: RemovedComponents<'w, 's, VehicleCore>,
    removed_identity: RemovedComponents<'w, 's, ScriptVehicleIdentity>,
}

impl VehicleCollisionMembership<'_, '_> {
    fn sync(&mut self) {
        for entity in self.removed_core.read().chain(self.removed_identity.read()) {
            self.spatial.grid.remove(entity);
        }
        for (entity, core) in &self.changed {
            self.spatial.refresh(entity, core, &self.loader);
        }
    }
}

/// Refresh membership for pedestrian obstacle queries before the actor control
/// chain reaches vehicle physics; otherwise the dependency would form a cycle.
pub(crate) fn sync_vehicle_collision_spatial_for_peds(mut membership: VehicleCollisionMembership) {
    membership.sync();
}

/// Always drain removals, including loading, restart and pause. Collision
/// response is guarded separately and uses the post-control vehicle positions.
pub(crate) fn sync_vehicle_collision_spatial(mut membership: VehicleCollisionMembership) {
    membership.sync();
}

pub(crate) struct GtaCollisionPlugin;
impl Plugin for GtaCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::entities::EntityReference::EntityReferencesCleared>()
            .init_resource::<VehicleCollisionSpatial>()
            .init_resource::<super::VehiclePair::VehiclePairDiagnostics>()
            .add_systems(
                Update,
                crate::entities::PhysicalReferences::retire_physical_damage_references
                    .after(GtaProcessSet::Script)
                    .before(GtaProcessSet::World),
            )
            .add_systems(
                Update,
                crate::entities::PhysicalReferences::clear_physical_reference_targets
                    .after(GtaProcessSet::GameLogic)
                    .before(GtaProcessSet::Bridge),
            )
            .add_systems(
                Update,
                sync_vehicle_collision_spatial_for_peds
                    .after(crate::peds::PedAI::process_script_ped_weapon_attacks),
            )
            .add_systems(
                Update,
                sync_vehicle_collision_spatial
                    .in_set(ActorCollisionSet::Membership)
                    .after(crate::vehicles::BoatControl::process_boat_control)
                    .before(super::Collision::process_script_entity_collisions),
            )
            .add_systems(
                Update,
                super::Collision::process_script_entity_collisions
                    .in_set(ActorCollisionSet::Response)
                    .after(crate::vehicles::BoatControl::process_boat_control)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collision::ColModel::{CColBox, CColModel, CColSphere, CColVector},
        control::ScriptEntities::{
            ScriptEntityIndex, ScriptVehicleEntity, spawn_script_test_vehicle,
        },
        vehicles::{Automobile::CAutomobile, Vehicle::MISSION_VEHICLE},
    };
    use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};

    #[test]
    fn pedestrian_and_collision_passes_use_vehicle_positions_from_their_own_stage() {
        fn move_vehicle_after_pedestrians(mut vehicles: Query<&mut VehicleCore>) {
            for mut core in &mut vehicles {
                core.physical.entity.set_position(CVector::new(200.0, 0.0, 0.0));
            }
        }

        let mut app = App::new();
        let mut loader = CFileLoader::new();
        let center = CColVector::new(0.0, 0.0, 0.0);
        loader
            .model_info
            .add_vehicle_model(90)
            .clump
            .base
            .set_col_model(
            CColModel::new(
                CColSphere {
                    center,
                    radius: 1.0,
                    surface: 0,
                    piece: 0,
                },
                CColBox::new(center, center, 0, 0),
                Vec::new(),
            ),
            true,
        );
        app.insert_resource(loader)
            .init_resource::<ScriptEntityIndex>()
            .init_resource::<VehicleCollisionSpatial>()
            .add_systems(
                Update,
                (
                    sync_vehicle_collision_spatial_for_peds,
                    move_vehicle_after_pedestrians,
                    sync_vehicle_collision_spatial,
                )
                    .chain(),
            );

        let mut random = CGeneral::with_seed(9);
        let car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut random, None, None, false);
        let actor = spawn_script_test_vehicle(
            app.world_mut(),
            ScriptVehicleEntity::Automobile { handle: 0x501, car },
        );
        app.update();

        let mut candidates = Vec::new();
        let grid = &app.world().resource::<VehicleCollisionSpatial>().grid;
        assert!(grid.candidates(CVector::new(200.0, 0.0, 0.0), 3.0, &mut candidates));
        assert!(candidates.contains(&actor), "post-control membership must include the moved car");
        assert!(grid.candidates(CVector::ZERO, 3.0, &mut candidates));
        assert!(!candidates.contains(&actor), "old pedestrian-pass membership must be retired");
    }

    #[test]
    fn membership_retirement_continues_during_cutscene_only_world_processing() {
        let mut app = App::new();
        let mut loader = CFileLoader::new();
        loader.world.process_cutscene_only = true;
        let center = CColVector::new(0.0, 0.0, 0.0);
        loader
            .model_info
            .add_vehicle_model(90)
            .clump
            .base
            .set_col_model(
                CColModel::new(
                    CColSphere {
                        center,
                        radius: 1.0,
                        surface: 0,
                        piece: 0,
                    },
                    CColBox::new(center, center, 0, 0),
                    Vec::new(),
                ),
                true,
            );
        app.insert_resource(loader)
            .init_resource::<ScriptEntityIndex>()
            .init_resource::<VehicleCollisionSpatial>()
            .add_systems(Update, sync_vehicle_collision_spatial);
        let mut random = CGeneral::with_seed(9);
        let car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut random, None, None, false);
        let actor = spawn_script_test_vehicle(
            app.world_mut(),
            ScriptVehicleEntity::Automobile { handle: 0x501, car },
        );
        app.update();
        assert!(
            app.world()
                .resource::<VehicleCollisionSpatial>()
                .grid
                .contains(&actor)
        );
        app.world_mut().despawn(actor);
        for _ in 0..8 {
            app.update();
        }
        assert!(
            !app.world()
                .resource::<VehicleCollisionSpatial>()
                .grid
                .contains(&actor)
        );
    }
}
