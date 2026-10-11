//! System ownership for the ped branch of re3 `CGame::Process`.
//! Cross-domain dependencies preserve the production schedule and deferred-command barriers.

use crate::collision::Collision::process_script_entity_collisions;
use crate::control::Remote::process_remote_vehicle;
use crate::control::RoadBlocks::process_roadblock_cop_spawns;
use crate::control::ScriptDomainEffects::process_chase_scene;
use crate::core::PlayerInfo::process_player_info_runtime;
use crate::peds::CopPed::process_cop_ai;
use crate::peds::PedRecovery::process_script_ped_fall_getup;
use crate::peds::PedDeath::process_script_ped_deaths;
use crate::peds::PedAI::process_script_ped_on_foot_objectives;
use crate::peds::PedAI::process_script_ped_weapon_attacks;
use crate::peds::PedSeated::process_seated_ped_animations;
use crate::peds::PedNearby::{PedThreatReactionRange, process_near_ped_lists};
use crate::peds::PlayerPed::update_player_on_foot_control;
use crate::peds::PlayerPed::update_player_wanted;
use crate::peds::Population::process_abandoned_zone_entities;
use crate::peds::PedChat::process_ambient_ped_chat;
use crate::peds::PedPopulationLifecycle::process_ambient_ped_cleanup;
use crate::peds::PedEvasionTraffic::process_ambient_ped_evasion;
use crate::peds::Population::process_ambient_ped_generation;
use crate::peds::Population::AmbientPedSpawnDiagnostics;
use crate::peds::PedWander::process_ambient_ped_wander;
use crate::peds::Population::process_map_object_collision_lifecycle;
use crate::peds::Population::refresh_population_counts_from_entities;
use crate::peds::VehicleRelations::process_ped_vehicle_animations;
use crate::peds::VehicleRelations::process_ped_vehicle_objectives;
use crate::peds::VehicleRelations::process_player_vehicle_animation;
use crate::peds::VehicleRelations::process_script_vehicle_doors;
use crate::peds::VehicleRelations::sync_script_vehicle_occupants;
use crate::vehicles::Automobile::process_automobile_damage_lifecycle;
use crate::vehicles::Automobile::process_passive_automobile_physics;
use crate::core::World::world_actor_processing_enabled;
use bevy::prelude::*;
use re3bevy_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

pub struct GtaPedSimulationPlugin;

impl Plugin for GtaPedSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PedThreatReactionRange>()
            .init_resource::<super::PedNearby::PedNearbySpatial>()
            .init_resource::<super::PedNearby::PedNearbyDiagnostics>()
            .add_systems(Update, (
                super::PedNearby::retire_near_ped_references,
                super::PedNearby::sync_near_ped_spatial,
            ).chain().after(process_ambient_ped_generation).before(process_near_ped_lists))
            .init_resource::<super::PedCorpse::PedCorpseDiagnostics>()
            .init_resource::<AmbientPedSpawnDiagnostics>()
            .init_resource::<crate::peds::PedEvasionTraffic::PedEvasionDiagnostics>()
            .add_systems(Update, super::PedRecovery::process_pending_fall_animations
                .after(GtaProcessSet::World)
                .before(GtaProcessSet::Audio)
                .run_if(world_actor_processing_enabled)
                .run_if(in_state(GtaAppState::InGame)))
            .add_systems(
                Update,
                process_near_ped_lists
                    .after(process_ambient_ped_generation)
                    .before(process_ped_vehicle_objectives)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_map_object_collision_lifecycle
                    .in_set(GtaProcessSet::Population)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                refresh_population_counts_from_entities
                    .after(process_chase_scene)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_ambient_ped_generation
                    .after(process_ambient_ped_cleanup)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_ped_vehicle_objectives
                    .after(process_ambient_ped_generation)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_script_ped_fall_getup
                    .after(process_ambient_ped_wander)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_script_ped_deaths
                    .after(process_script_ped_fall_getup)
                    .before(crate::audio::PedTalk::service_ped_talk)
                    .before(process_ped_vehicle_animations)
                    .before(process_script_entity_collisions)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(Update, super::PedDeath::process_player_death_associations
                .after(process_script_ped_deaths)
                .before(crate::audio::PedTalk::service_ped_talk)
                .before(process_player_vehicle_animation)
                .before(update_player_on_foot_control)
                .in_set(GtaProcessSet::World)
                .run_if(world_actor_processing_enabled)
                .run_if(in_state(GtaAppState::InGame)))
            .add_systems(Update, super::PedCorpse::process_corpse_tyre_contacts
                .after(process_script_ped_deaths)
                .after(crate::collision::VehicleSpatial::sync_vehicle_collision_spatial)
                .before(process_script_entity_collisions)
                .in_set(GtaProcessSet::World)
                .run_if(world_actor_processing_enabled)
                .run_if(in_state(GtaAppState::InGame)))
            .add_systems(
                Update,
                process_script_ped_on_foot_objectives
                    .after(process_cop_ai)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_script_ped_weapon_attacks
                    .after(process_script_ped_on_foot_objectives)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_ambient_ped_wander
                    .after(process_script_ped_weapon_attacks)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_ambient_ped_chat
                    .after(process_ambient_ped_wander)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_ped_vehicle_animations
                    .after(process_script_ped_weapon_attacks)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_script_vehicle_doors
                    .after(process_ped_vehicle_animations)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_player_vehicle_animation
                    .after(process_remote_vehicle)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_player_on_foot_control
                    .after(process_player_vehicle_animation)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                sync_script_vehicle_occupants
                    .after(process_script_entity_collisions)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_seated_ped_animations
                    .after(sync_script_vehicle_occupants)
                    .after(update_player_on_foot_control)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_player_wanted
                    .after(process_automobile_damage_lifecycle)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_player_info_runtime
                    .after(update_player_wanted)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_cop_ai
                    .after(process_ped_vehicle_objectives)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_abandoned_zone_entities
                    .after(process_roadblock_cop_spawns)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_ambient_ped_cleanup
                    .after(process_abandoned_zone_entities)
                    .before(process_ambient_ped_generation)
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                process_ambient_ped_evasion
                    .after(crate::control::SirenTraffic::sync_automobile_spatial_index)
                    .before(process_passive_automobile_physics)
                    .in_set(GtaProcessSet::World)
                    .run_if(world_actor_processing_enabled)
                    .run_if(in_state(GtaAppState::InGame)),
            );
    }
}
