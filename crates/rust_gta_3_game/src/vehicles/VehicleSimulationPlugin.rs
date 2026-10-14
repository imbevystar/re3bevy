//! System ownership for the vehicle branch of re3 `CGame::Process`.
//! Cross-domain dependencies preserve the production schedule and deferred-command barriers.

use crate::audio::PedTalk::service_ped_talk;
use crate::control::CarCtrl::process_emergency_service_generation;
use crate::control::CarCtrl::process_physics_automobile_ai_control;
use crate::control::CarCtrl::process_simple_automobile_control;
use crate::control::CarCtrl::process_siren_traffic_response;
use crate::control::CarCtrl::process_traffic_car_generation;
use crate::control::CarCtrl::process_traffic_vehicle_cleanup;
use crate::control::CarCtrl::update_ambient_vehicle_and_zone_streaming;
use crate::control::Remote::process_remote_vehicle;
use crate::control::RoadBlocks::process_roadblock_cop_spawns;
use crate::control::RoadBlocks::process_roadblocks;
use crate::peds::PlayerPed::update_player_on_foot_control;
use crate::peds::Population::refresh_population_counts_from_entities;
use crate::peds::VehicleRelations::sync_script_vehicle_occupants;
use crate::renderer::Rubbish::stir_rubbish_from_automobiles;
use crate::renderer::WaterCannon::process_firetruck_water_cannon_control;
use crate::vehicles::Automobile::process_automobile_damage_lifecycle;
use crate::vehicles::Automobile::process_passive_automobile_physics;
use crate::vehicles::Heli::update_heli_runtime;
use crate::vehicles::Plane::initialise_plane_entities;
use crate::vehicles::Plane::update_plane_runtime;
use crate::core::World::world_actor_processing_enabled;
use bevy::prelude::*;
use rust_gta_3_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

pub struct GtaVehicleSimulationPlugin;

impl Plugin for GtaVehicleSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::entities::EntityReference::EntityReferencesCleared>()
            .init_resource::<crate::vehicles::Automobile::AutomobilePhysicsDiagnostics>()
            .add_systems(Update, super::retire_vehicle_damage_references
                .after(GtaProcessSet::Script)
                .before(GtaProcessSet::World)
                .before(process_automobile_damage_lifecycle))
            .add_systems(Update, super::clear_vehicle_damage_reference_targets
                .after(GtaProcessSet::GameLogic).before(GtaProcessSet::Bridge))
            .init_resource::<crate::vehicles::VehicleConstruction::GtaSuspensionDiagnostics>()
            .add_systems(Update, crate::vehicles::VehicleConstruction::invalidate_automobile_suspension_sources
                .before(update_player_on_foot_control)
                .before(process_simple_automobile_control)
                .before(process_remote_vehicle)
                .before(process_passive_automobile_physics)
                .before(crate::vehicles::VehicleConstruction::VehicleConstructionSet::Suspension))
            .add_systems(Update, crate::vehicles::VehicleConstruction::initialise_automobile_suspension
                .in_set(GtaProcessSet::RenderPrep)
                .in_set(crate::vehicles::VehicleConstruction::VehicleConstructionSet::Suspension)
                .after(crate::renderer::BevyRender::NativeRenderPreparationSet::WorldRequests)
                .before(crate::renderer::BevyRender::NativeRenderPreparationSet::Audit)
                .before(crate::vehicles::AutomobileEngineEffects::AutomobilePreRenderSet::Particles)
                .run_if(in_state(GtaAppState::InGame)))
            .init_resource::<crate::control::SirenTraffic::AutomobileSpatialIndex>()
            .init_resource::<crate::vehicles::VehicleAudio::VehicleAlarmDiagnostics>()
            .add_systems(Update, crate::vehicles::AutomobileEngineEffects::register_automobile_prerender_particles
                .in_set(GtaProcessSet::RenderPrep)
                .in_set(crate::vehicles::AutomobileEngineEffects::AutomobilePreRenderSet::Particles)
                .after(crate::renderer::Particle::ParticleCreationSet::BeforeRender)
                .before(crate::renderer::Particle::NativeParticleRenderSet::Consume)
                .run_if(in_state(GtaAppState::InGame)))
            .add_systems(Update, crate::vehicles::VehicleAudio::process_automobile_audio_control
                .before(update_player_on_foot_control)
                .before(process_simple_automobile_control)
                .before(process_remote_vehicle)
                .before(process_passive_automobile_physics)
                .in_set(GtaProcessSet::World)
                .run_if(world_actor_processing_enabled)
                .run_if(in_state(GtaAppState::InGame)))
            .add_systems(OnEnter(GtaAppState::InGame), initialise_plane_entities)
            .add_systems(
                Update,
                update_plane_runtime
                    .in_set(GtaProcessSet::Script)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_heli_runtime
                    .after(update_plane_runtime)
                    .in_set(GtaProcessSet::Script)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_ambient_vehicle_and_zone_streaming
                    .after(refresh_population_counts_from_entities)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_traffic_car_generation
                    .after(update_ambient_vehicle_and_zone_streaming)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_roadblocks
                    .after(process_traffic_car_generation)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_emergency_service_generation
                    .after(process_roadblocks)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_remote_vehicle
                    .after(service_ped_talk)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                stir_rubbish_from_automobiles
                    .after(update_player_on_foot_control)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_simple_automobile_control
                    .after(stir_rubbish_from_automobiles)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_passive_automobile_physics
                    .after(process_physics_automobile_ai_control)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                crate::vehicles::BoatControl::process_boat_control
                    .after(process_passive_automobile_physics)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                crate::vehicles::BoatControl::process_predator_gun_rounds
                    .after(crate::vehicles::BoatControl::process_boat_control)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                crate::vehicles::BoatControl::process_boat_driver_collision_speech
                    .after(crate::vehicles::BoatControl::process_boat_control)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_automobile_damage_lifecycle
                    .after(sync_script_vehicle_occupants)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_physics_automobile_ai_control
                    .after(process_simple_automobile_control)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_traffic_vehicle_cleanup
                    .after(process_emergency_service_generation)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_roadblock_cop_spawns
                    .after(process_traffic_vehicle_cleanup)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                crate::control::SirenTraffic::sync_automobile_spatial_index
                    .after(process_physics_automobile_ai_control)
                    .before(process_siren_traffic_response),
            )
            .add_systems(
                Update,
                process_siren_traffic_response
                    .after(process_physics_automobile_ai_control)
                    .before(process_passive_automobile_physics)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_firetruck_water_cannon_control
                    .after(process_physics_automobile_ai_control)
                    .before(process_passive_automobile_physics)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            );
    }
}
