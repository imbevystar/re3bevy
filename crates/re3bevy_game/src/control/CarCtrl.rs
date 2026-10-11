//! re3 reference: references/re-GTA/src/control/CarCtrl.cpp
//! re3 reference: references/re-GTA/src/control/CarCtrl.h
//!
//! `CCarCtrl::Init` state.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub const MAX_CARS_TO_KEEP: usize = 2;
pub const MAX_CAR_MODELS_IN_ARRAY: usize = 256;
pub const TOTAL_CUSTOM_CLASSES: usize = 7;
/// re3 `CIniFile::LoadIniFile` default before an optional INI multiplier is
/// applied.  GTA III uses this global budget in addition to the separate
/// `NumRandomCars < 30` guard.
pub const DEFAULT_MAX_NUMBER_OF_CARS: i32 = 12;

/// re3 `apCarsToKeep/aCarsToKeepTime`, owned by the vehicle entity.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct VehicleOfInterest {
    slot: usize,
    registered_ms: u32,
}

pub(crate) fn queue_register_vehicle_of_interest(commands: &mut Commands, vehicle: Entity) {
    commands.queue(move |world: &mut World| {
        if world.get_entity(vehicle).is_err() {
            warn!(target: "re3bevy::traffic", ?vehicle, "interest registration lost its vehicle");
            return;
        }
        let now_ms = world.resource::<CTimer>().time_in_milliseconds();
        if let Some(mut interest) = world.get_mut::<VehicleOfInterest>(vehicle) {
            interest.registered_ms = now_ms;
            return;
        }
        let mut occupied = [false; MAX_CARS_TO_KEEP];
        let mut oldest: Option<(Entity, VehicleOfInterest)> = None;
        let mut query = world.query::<(Entity, &VehicleOfInterest)>();
        for (entity, interest) in query.iter(world) {
            occupied[interest.slot] = true;
            if oldest.is_none_or(|(_, current)| {
                (interest.registered_ms, interest.slot) < (current.registered_ms, current.slot)
            }) {
                oldest = Some((entity, *interest));
            }
        }
        let slot = if let Some(slot) = occupied.iter().position(|occupied| !occupied) {
            slot
        } else if let Some((entity, interest)) = oldest {
            world.entity_mut(entity).remove::<VehicleOfInterest>();
            interest.slot
        } else {
            warn!(target: "re3bevy::traffic", "interest slots are inconsistent");
            return;
        };
        world.entity_mut(vehicle).insert(VehicleOfInterest {
            slot,
            registered_ms: now_ms,
        });
    });
}

#[derive(Debug, Clone, Resource)]
pub struct CCarCtrl {
    pub num_random_cars: i32,
    pub num_law_enforcer_cars: i32,
    pub num_mission_cars: i32,
    pub num_parked_cars: i32,
    pub num_permanent_cars: i32,
    pub max_number_of_cars_in_use: i32,
    pub last_time_law_enforcer_created: u32,
    pub num_ambulances_on_duty: i32,
    pub num_firetrucks_on_duty: i32,
    pub last_time_fire_truck_created: u32,
    pub last_time_ambulance_created: u32,
    pub cars_generated_around_camera: bool,
    pub countdown_to_cars_at_start: i32,
    pub car_density_multiplier: f32,
    /// re3 `CIniFile::CarNumberMultiplier` from the second `gta3.ini` line.
    pub car_number_multiplier: f32,
    pub car_arrays: [[i32; MAX_CAR_MODELS_IN_ARRAY]; TOTAL_CUSTOM_CLASSES],
    pub next_car_of_rating: [i32; TOTAL_CUSTOM_CLASSES],
    pub total_num_of_cars_of_rating: [i32; TOTAL_CUSTOM_CLASSES],
}

impl CCarCtrl {
    pub fn init() -> Self {
        Self {
            num_random_cars: 0,
            num_law_enforcer_cars: 0,
            num_mission_cars: 0,
            num_parked_cars: 0,
            num_permanent_cars: 0,
            max_number_of_cars_in_use: DEFAULT_MAX_NUMBER_OF_CARS,
            last_time_law_enforcer_created: 0,
            num_ambulances_on_duty: 0,
            num_firetrucks_on_duty: 0,
            last_time_fire_truck_created: 0,
            last_time_ambulance_created: 0,
            cars_generated_around_camera: false,
            countdown_to_cars_at_start: 2,
            car_density_multiplier: 1.0,
            car_number_multiplier: 1.0,
            car_arrays: [[0; MAX_CAR_MODELS_IN_ARRAY]; TOTAL_CUSTOM_CLASSES],
            next_car_of_rating: [0; TOTAL_CUSTOM_CLASSES],
            total_num_of_cars_of_rating: [0; TOTAL_CUSTOM_CLASSES],
        }
    }

    /// Exact pool-pressure sum used by `GenerateOneRandomCar` and
    /// `CPopulation::AddToPopulation`. Permanent cars are intentionally absent:
    /// the original does not include `NumPermanentCars` in this gate.
    pub fn num_cars_in_use_for_population(&self) -> i32 {
        self.num_firetrucks_on_duty
            + self.num_ambulances_on_duty
            + self.num_parked_cars
            + self.num_mission_cars
            + self.num_law_enforcer_cars
            + self.num_random_cars
    }

    pub fn population_car_budget_full(&self) -> bool {
        self.num_cars_in_use_for_population() >= self.max_number_of_cars_in_use
    }

    /// re3 `CCarCtrl::ChooseCarModel`. IDE frequency is already expanded in
    /// `CarArrays`, so this advances the authored per-class cursor exactly as
    /// the original does instead of sampling the bucket uniformly.
    pub fn choose_car_model(&mut self, class: usize, general: &mut CGeneral) -> Option<i32> {
        let total = *self.total_num_of_cars_of_rating.get(class)?;
        if total <= 0 {
            return None;
        }
        let total = total as usize;
        let next = self.next_car_of_rating[class].max(0) as usize % total;
        let model = self.car_arrays[class][next];
        let advance = general.get_random_number_in_range_i32(1, total as i32) as usize;
        self.next_car_of_rating[class] = ((next + advance) % total) as i32;
        Some(model)
    }
}

// === FAZ3a: moved from core/Game.rs (verbatim, parity-preserving) ===
// Traffic control systems: car generation and STATUS_SIMPLE control.
// Rail math lives in control/CarRails.rs. Schedule stays in core/Game.rs.

#[allow(unused_imports)]
use crate::animation::CutsceneMgr::CCutsceneMgr;
#[allow(unused_imports)]
use crate::collision::Collision::CCollision;
#[allow(unused_imports)]
use crate::control::AutoPilot::{LANE_WIDTH, eCarDrivingStyle, eCarMission, eCarTempAction};
#[allow(unused_imports)]
use crate::control::CarAI::{
    CCarAI, CarAiStepInput, GAME_SPEED_TO_CARAI_SPEED,
};
#[allow(unused_imports)]
use crate::control::CarRails::{
    TrafficPedScan, TrafficSpatialIndex, TrafficVehicleScan, drag_car_to_point,
    find_maximum_speed_for_this_car_in_traffic, join_car_with_road_system_goto_coors,
    modify_speed_on_rails, one_way_lane_offset, pick_next_node_according_to_strategy,
    place_ambient_traffic_car_at_creation_curve, rails_update_car_on_rails,
    slow_car_on_rails_down_for_traffic_and_lights,
};
#[allow(unused_imports)]
use crate::control::Garages::{CGarages, EGarageType};
use crate::control::Remote::CRemote;
#[allow(unused_imports)]
use crate::control::Script::{
    ScriptCreateCarRequest, ScriptCreateCharRequest, remove_script_vehicle_and_occupants,
    script_vehicle_base_can_be_deleted,
};
#[allow(unused_imports)]
use crate::control::ScriptEntities::ScriptPedEntity;
#[allow(unused_imports)]
use crate::control::TrafficLights::{CTrafficLights, CTrafficVehicleState};
#[allow(unused_imports)]
use crate::core::Accident::{CAccidentManager, CAccidentVictimState};
#[allow(unused_imports)]
use crate::core::Camera::CCamera;
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Fire::CFireManager;
#[allow(unused_imports)]
use crate::core::Game::ScriptGameplayBridge;
#[allow(unused_imports)]
use crate::core::Game::{
    GameplayAuditTimer, ScriptEntityWorld, ScriptTaxiLightState, ScriptVehicleEntity,
    join_car_with_road_system,
};
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPoolHandle, CPools};
use crate::core::Streaming::StreamLoadState;
#[allow(unused_imports)]
use crate::core::Streaming::{
    CStreaming, DESIRED_NUM_VEHICLES_LOADED, STREAMFLAGS_DEPENDENCY, STREAMFLAGS_DONT_REMOVE,
};
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::{choose_vehicle_optional_components, eVehicleType};
#[allow(unused_imports)]
use crate::peds::Gangs::CGangs;
#[allow(unused_imports)]
use crate::peds::Ped::{CPed, PedState, RANDOM_CHAR, eObjective, give_ped_weapon};
#[allow(unused_imports)]
use crate::peds::PedType::{ePedType, script_ped_type};
#[allow(unused_imports)]
use crate::peds::PlayerRead::CWorldPlayersRead;
#[allow(unused_imports)]
use crate::peds::Population::{
    AmbientCarClass, AmbientSpawnCollisionCandidate, AmbientTrafficGenerationScratch,
    AmbientTrafficModels, CPopulation, add_ped_to_population_counts,
    ambient_car_spawn_has_nearby_entity, ambient_car_spawn_is_blocked,
    ambient_car_spawn_is_blocked_with_extra_radius, ambient_car_spawn_position,
    ambient_driver_profile, ambient_zone_info, choose_ambient_car_model, choose_police_car_model,
};
#[allow(unused_imports)]
use crate::peds::VehicleRelations::{
    PedAutomobileProfile, VehicleSeat, queue_ped_vehicle_animation,
};
#[allow(unused_imports)]
use crate::vehicles::Automobile::CAutomobile;
use crate::vehicles::Cranes::CCranes;
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::{CVehicle, RANDOM_VEHICLE};
#[allow(unused_imports)]
use re3bevy_core::core::Clock::CClock;
use re3bevy_core::core::GameState::{CGameState, ELevelName};
#[allow(unused_imports)]
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

/// re3 `CCarCtrl::RemoveDistantCars/PossiblyRemoveVehicle`.
///
/// The original runs this after random-car generation every frame. Keeping
/// the lifecycle is gameplay-significant: without it the random-car counter
/// reaches its cap and traffic never repopulates around the player after a
/// long drive. Visible distant vehicles fade through their gameplay-owned
/// clump alpha before deletion, matching the two-stage re3 lifetime path.
pub(crate) fn process_traffic_vehicle_cleanup(
    interesting: Query<(), With<VehicleOfInterest>>,
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut car_ctrl: ResMut<CCarCtrl>,
    mut population: Option<ResMut<CPopulation>>,
    players: CWorldPlayersRead,
    camera: Option<Res<CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    garages: Option<Res<CGarages>>,
    cranes: Option<Res<CCranes>>,
    timer: Res<CTimer>,
) {
    // `CAutomobile::ProcessControl`: ambient clumps fade in by 16 and a car
    // marked bFadeOut fades by 8. Keep this state on the gameplay entity so
    // renderer and lifetime decisions observe the same value.
    for mut vehicle in script_entities.vehicles.values_mut() {
        let mut base = vehicle.base_mut();
        base.physical.entity.clump_alpha = if base.appearance.fade_out {
            base.physical.entity.clump_alpha.saturating_sub(8)
        } else {
            base.physical.entity.clump_alpha.saturating_add(16)
        };
    }
    let mut random_cars = 0;
    let mut mission_cars = 0;
    let mut parked_cars = 0;
    let mut permanent_cars = 0;
    let mut law_enforcer_cars = 0;
    let mut ambulances_on_duty = 0;
    let mut firetrucks_on_duty = 0;
    for vehicle in script_entities.vehicles.values() {
        let base = vehicle.base();
        match base.vehicle_created_by {
            RANDOM_VEHICLE => random_cars += 1,
            crate::vehicles::Vehicle::MISSION_VEHICLE => mission_cars += 1,
            crate::vehicles::Vehicle::PARKED_VEHICLE => parked_cars += 1,
            crate::vehicles::Vehicle::PERMANENT_VEHICLE => permanent_cars += 1,
            _ => {}
        }
        law_enforcer_cars += i32::from(base.flags.is_law_enforcer);
        ambulances_on_duty += i32::from(base.flags.is_ambulance_on_duty);
        firetrucks_on_duty += i32::from(base.flags.is_fire_truck_on_duty);
    }
    car_ctrl.num_random_cars = random_cars;
    car_ctrl.num_mission_cars = mission_cars;
    car_ctrl.num_parked_cars = parked_cars;
    car_ctrl.num_permanent_cars = permanent_cars;
    car_ctrl.num_law_enforcer_cars = law_enforcer_cars;
    car_ctrl.num_ambulances_on_duty = ambulances_on_duty;
    car_ctrl.num_firetrucks_on_duty = firetrucks_on_duty;
    let (Some(player), Some(camera), Some(file_loader)) =
        (players.focused(), camera.as_deref(), file_loader.as_deref())
    else {
        bevy::log::warn_once!(target: "re3bevy::traffic_cleanup",
            "required player/camera/original model inputs are absent");
        return;
    };
    let player_position = player.position();
    let now_ms = timer.time_in_milliseconds();

    let mut doomed = Vec::new();
    let mut fading = Vec::new();
    for (&handle, vehicle_entity) in script_entities.vehicles.iter() {
        let base = vehicle_entity.base();
        let crane_targeted = cranes.as_deref().is_some_and(|cranes| {
            cranes.is_vehicle_targeted_by_any_crane(CPoolHandle(handle).index() as i32)
        });
        if base.flags.is_locked
            || crane_targeted
            || script_entities
                .vehicle_entity(handle)
                .is_some_and(|entity| interesting.contains(entity))
            || !script_vehicle_base_can_be_deleted(&script_entities, handle, &base)
        {
            continue;
        }

        let position = base.physical.entity.get_position();
        let in_hideout_garage = garages.as_deref().is_some_and(|garages| {
            garages.garages[..(garages.num_garages as usize).min(garages.garages.len())]
                .iter()
                .filter(|garage| {
                    matches!(
                        garage.garage_type,
                        EGarageType::HideoutOne
                            | EGarageType::HideoutTwo
                            | EGarageType::HideoutThree
                    )
                })
                .any(|garage| {
                    position.x >= garage.x1.min(garage.x2)
                        && position.x <= garage.x1.max(garage.x2)
                        && position.y >= garage.y1.min(garage.y2)
                        && position.y <= garage.y1.max(garage.y2)
                        && position.z >= garage.z1.min(garage.z2)
                        && position.z <= garage.z1.max(garage.z2)
                })
        });
        if in_hideout_garage {
            continue;
        }
        let dx = position.x - player_position.x;
        let dy = position.y - player_position.y;
        let distance = (dx * dx + dy * dy).sqrt();
        let dz = position.z - player_position.z;
        let radius = file_loader
            .model_info
            .col_model(base.physical.entity.model_index.max(0) as usize)
            .map_or(2.0, |col| col.bounding_sphere.radius);
        let on_screen = camera.is_sphere_visible(position, radius);
        let status = base.physical.entity.status;
        let cull_zone_visible = if crate::renderer::Renderer::zone_cull_disabled(
            &file_loader.renderer,
        ) {
            true
        } else if base.physical.entity.flags.zone_culled {
            false
        } else if matches!(
            status,
            eEntityStatus::STATUS_SIMPLE
                | eEntityStatus::STATUS_PHYSICS
                | eEntityStatus::STATUS_ABANDONED
                | eEntityStatus::STATUS_WRECKED
        ) {
            base.current_ground_entity
                .and_then(|index| file_loader.world.entities.get(index))
                .is_none_or(|ground| !ground.zone_culled2)
        } else {
            true
        };
        let model_id = base.physical.entity.model_index.max(0) as usize;

        if base.appearance.fade_out && base.physical.entity.clump_alpha == 0 {
            doomed.push(handle);
            continue;
        }

        let camera_looking_away = camera.cams.get(camera.active_cam).is_some_and(|cam| {
            cam.direction_was_looking != crate::core::Cam::LOOKING_FORWARD
        });
        let mut threshold = if on_screen
            || camera_looking_away
            || base.vehicle_created_by == crate::vehicles::Vehicle::PARKED_VEHICLE
            || base.flags.is_law_enforcer
            || base.flags.is_car_park_vehicle
            || matches!(
                model_id,
                crate::modelinfo::ModelIndices::MI_AMBULAN
                    | crate::modelinfo::ModelIndices::MI_FIRETRUCK
            ) {
            130.0 * camera.generation_dist_multiplier
        } else {
            50.0
        };
        if base.flags.extended_range {
            threshold *= 1.5;
        }

        if distance > threshold {
            if on_screen && cull_zone_visible {
                // re3 keeps a visible distant car alive until its clump has
                // faded to zero, avoiding the hard pop at the removal radius.
                fading.push(handle);
            } else {
                doomed.push(handle);
            }
            continue;
        }

        // re3's second reclamation path is intentionally much closer than
        // the normal generation radius: an ordinary traffic car that has
        // existed for five seconds can be removed once it is off-screen and
        // more than 25 m away, unless a light/bridge is actively holding it.
        let ordinary_traffic_status = status == eEntityStatus::STATUS_SIMPLE
            || (status == eEntityStatus::STATUS_PHYSICS
                && base.auto_pilot.driving_style == eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS);
        if ordinary_traffic_status
            && now_ms.wrapping_sub(base.auto_pilot.time_to_start_mission) > 5_000
            && !on_screen
            && distance > 25.0
        {
            let traffic_state = CTrafficVehicleState {
                position: [position.x, position.y, position.z],
                next_path_node_info: base.auto_pilot.next_path_node_info as usize,
                current_path_node_info: base.auto_pilot.current_path_node_info as usize,
                previous_path_node_info: base.auto_pilot.previous_path_node_info as usize,
                next_route_node: base.auto_pilot.next_route_node as i16,
                current_route_node: base.auto_pilot.current_route_node as i16,
                prev_route_node: base.auto_pilot.prev_route_node as i16,
                next_direction: base.auto_pilot.next_direction,
                current_direction: base.auto_pilot.current_direction,
                previous_direction: base.auto_pilot.previous_direction,
                status_physics: status == eEntityStatus::STATUS_PHYSICS,
            };
            let stopped_for_light = CTrafficLights::should_car_stop_for_light(
                &file_loader.path_find,
                &traffic_state,
                true,
                now_ms,
            );
            let stopped_for_bridge =
                CTrafficLights::should_car_stop_for_bridge(&file_loader.path_find, &traffic_state);
            if !stopped_for_light && !stopped_for_bridge {
                doomed.push(handle);
                continue;
            }
        }

        // Wrecks persist for one minute, then become reclaimable off-screen
        // once the player is at least 7.5 m away (`m_nTimeOfDeath`).
        if status == eEntityStatus::STATUS_WRECKED
            && base.vitality.time_of_death != 0
            && now_ms > base.vitality.time_of_death.wrapping_add(60_000)
            && !(on_screen && cull_zone_visible)
            && dx * dx + dy * dy + dz * dz > 7.5 * 7.5
        {
            doomed.push(handle);
        }
    }

    for handle in fading {
        if let Some(mut vehicle) = script_entities.vehicle_get_mut(handle) {
            vehicle.base_mut().appearance.fade_out = true;
        }
    }
    for handle in doomed {
        remove_script_vehicle_and_occupants(
            &mut script_entities,
            &mut pools,
            handle,
            population.as_deref_mut(),
            Some(&mut car_ctrl),
        );
    }
}

#[derive(SystemParam)]
pub(crate) struct AmbientTrafficGenerationResources<'w> {
    weapon_info: Res<'w, crate::weapons::WeaponInfo::CWeaponInfoStore>,
    clock: Res<'w, CClock>,
    population: Res<'w, CPopulation>,
    gangs: Option<Res<'w, CGangs>>,
    remote: Option<Res<'w, CRemote>>,
}

fn service_ambient_vehicle_and_zone_streaming(
    streaming: &mut CStreaming,
    file_loader: &CFileLoader,
    car_ctrl: &mut CCarCtrl,
    population: &CPopulation,
    gangs: Option<&CGangs>,
    zone_info: crate::core::World::ZonePedInfo,
    current_level: i32,
    wanted: &crate::core::Wanted::CWanted,
    general: &mut CGeneral,
) {
    use crate::modelinfo::ModelIndices::{
        MI_ARMY, MI_BARRACKS, MI_CHOPPER, MI_ENFORCER, MI_FBI, MI_FBICAR, MI_MALE01, MI_RHINO,
        MI_SWAT,
    };

    // re3 `CStreaming::StreamVehiclesAndPeds`: wanted-level assets stay pinned
    // only while their corresponding response tier is active.
    if wanted.are_swat_required() {
        streaming.request_model_with_adapter(MI_ENFORCER, STREAMFLAGS_DONT_REMOVE, file_loader);
        streaming.request_model_with_adapter(MI_SWAT, STREAMFLAGS_DONT_REMOVE, file_loader);
    } else {
        streaming.set_model_is_deletable(MI_ENFORCER, file_loader);
        if !streaming.has_model_loaded(MI_ENFORCER) {
            streaming.set_model_is_deletable(MI_SWAT, file_loader);
        }
    }
    if wanted.are_fbi_required() {
        streaming.request_model_with_adapter(MI_FBICAR, STREAMFLAGS_DONT_REMOVE, file_loader);
        streaming.request_model_with_adapter(MI_FBI, STREAMFLAGS_DONT_REMOVE, file_loader);
    } else {
        streaming.set_model_is_deletable(MI_FBICAR, file_loader);
        if !streaming.has_model_loaded(MI_FBICAR) {
            streaming.set_model_is_deletable(MI_FBI, file_loader);
        }
    }
    if wanted.are_army_required() {
        streaming.request_model_with_adapter(MI_RHINO, STREAMFLAGS_DONT_REMOVE, file_loader);
        streaming.request_model_with_adapter(MI_BARRACKS, STREAMFLAGS_DONT_REMOVE, file_loader);
        streaming.request_model_with_adapter(MI_ARMY, STREAMFLAGS_DONT_REMOVE, file_loader);
    } else {
        streaming.set_model_is_deletable(MI_RHINO, file_loader);
        streaming.set_model_is_deletable(MI_BARRACKS, file_loader);
        if !streaming.has_model_loaded(MI_RHINO) && !streaming.has_model_loaded(MI_BARRACKS) {
            streaming.set_model_is_deletable(MI_ARMY, file_loader);
        }
    }
    if wanted.num_of_helis_required() > 0 {
        streaming.request_model_with_adapter(MI_CHOPPER, STREAMFLAGS_DONT_REMOVE, file_loader);
    } else {
        streaming.set_model_is_deletable(MI_CHOPPER, file_loader);
    }

    if streaming.time_before_next_vehicle_load >= 0 {
        streaming.time_before_next_vehicle_load -= 1;
    } else if streaming.num_vehicles_loaded <= DESIRED_NUM_VEHICLES_LOADED {
        for _ in 0..10 {
            let class = streaming.model_quality_class;
            streaming.model_quality_class =
                (streaming.model_quality_class + 1) % TOTAL_CUSTOM_CLASSES;
            let Some(model_id) = car_ctrl.choose_car_model(class, general) else {
                continue;
            };
            let model_id = model_id.max(0) as usize;
            let belongs_to_level = match file_loader.model_info.get_model_info(model_id) {
                Some(ModelInfoSlot::Vehicle(model)) if current_level > 0 => {
                    model.level & (1 << (current_level - 1)) != 0
                }
                _ => false,
            };
            let is_not_loaded =
                streaming.info_for_model[model_id].load_state == StreamLoadState::NotLoaded;
            if is_not_loaded && belongs_to_level {
                bevy::log::debug!(
                    target: "re3bevy::traffic",
                    class,
                    model_id,
                    "service_ambient_vehicle_and_zone_streaming requested vehicle model"
                );
                streaming.request_model_with_adapter(model_id, STREAMFLAGS_DEPENDENCY, file_loader);
                streaming.time_before_next_vehicle_load = 500;
                break;
            }
        }
    }

    // re3 `StreamZoneModels`: swap the current pedestrian group and keep the
    // always-present fallback male resident.
    let new_group = zone_info.ped_group as usize;
    if streaming.current_ped_grp != new_group as i32 {
        if let Some(old_group) = usize::try_from(streaming.current_ped_grp)
            .ok()
            .and_then(|index| population.ped_groups.get(index))
        {
            for &model_id in &old_group.models {
                if model_id < 0 {
                    continue;
                }
                let model_id = model_id as usize;
                streaming.set_model_is_deletable(model_id, file_loader);
                streaming.set_model_txd_is_deletable(model_id, file_loader);
            }
        }
        streaming.current_ped_grp = new_group as i32;
        if let Some(group) = population.ped_groups.get(new_group) {
            for &model_id in &group.models {
                if model_id >= 0 {
                    streaming.request_model_with_adapter(
                        model_id as usize,
                        STREAMFLAGS_DONT_REMOVE,
                        file_loader,
                    );
                }
            }
        }
    }
    streaming.request_model_with_adapter(MI_MALE01, STREAMFLAGS_DONT_REMOVE, file_loader);

    let mut gangs_to_load = 0_u16;
    let mut gang_cars_to_load = 0_u16;
    for gang in 0..9 {
        let bit = 1_u16 << gang;
        if zone_info.gang_density[gang] != 0 {
            gangs_to_load |= bit;
        }
        let previous_threshold = if gang == 0 {
            zone_info.cop_density as i16
        } else {
            zone_info.gang_threshold[gang - 1]
        };
        if zone_info.gang_threshold[gang] != previous_threshold {
            gang_cars_to_load |= bit;
        }
    }
    if gangs_to_load == streaming.loaded_gangs && gang_cars_to_load == streaming.loaded_gang_cars {
        return;
    }
    gangs_to_load |= gang_cars_to_load;

    for gang in 0..9 {
        let bit = 1_u16 << gang;
        let first_ped = 10 + gang * 2;
        if gangs_to_load & bit != 0 && streaming.loaded_gangs & bit == 0 {
            streaming.request_model_with_adapter(first_ped, STREAMFLAGS_DONT_REMOVE, file_loader);
            streaming.request_model_with_adapter(
                first_ped + 1,
                STREAMFLAGS_DONT_REMOVE,
                file_loader,
            );
            streaming.loaded_gangs |= bit;
        } else if gangs_to_load & bit == 0 && streaming.loaded_gangs & bit != 0 {
            for ped_model in [first_ped, first_ped + 1] {
                streaming.set_model_is_deletable(ped_model, file_loader);
                streaming.set_model_txd_is_deletable(ped_model, file_loader);
            }
            streaming.loaded_gangs &= !bit;
        }

        let gang_vehicle = gangs
            .and_then(|gangs| gangs.gangs.get(gang))
            .map(|info| info.vehicle_model)
            .filter(|model| *model >= 0)
            .map(|model| model as usize);
        if gang_cars_to_load & bit != 0 && streaming.loaded_gang_cars & bit == 0 {
            if let Some(model_id) = gang_vehicle {
                streaming.request_model_with_adapter(
                    model_id,
                    STREAMFLAGS_DONT_REMOVE,
                    file_loader,
                );
            }
        } else if gang_cars_to_load & bit == 0
            && streaming.loaded_gang_cars & bit != 0
            && let Some(model_id) = gang_vehicle
        {
            streaming.set_model_is_deletable(model_id, file_loader);
            streaming.set_model_txd_is_deletable(model_id, file_loader);
        }
    }
    streaming.loaded_gang_cars = gang_cars_to_load;
}

fn ambient_traffic_target_position(
    car_ctrl: &CCarCtrl,
    player_position: CVector,
    player_vehicle_position: Option<CVector>,
    remote_vehicle_position: Option<CVector>,
    camera: Option<&CCamera>,
) -> (CVector, &'static str) {
    // re3 `FindPlayerCentreOfWorld`: script can deliberately make population
    // follow the camera; RC vehicles take precedence over the normal player car.
    if car_ctrl.cars_generated_around_camera {
        if let Some(camera) = camera {
            let position = camera
                .cams
                .get(camera.active_cam)
                .map_or(camera.position, |cam| cam.source);
            return (position, "camera");
        }
    }
    if let Some(position) = remote_vehicle_position {
        return (position, "remote");
    }
    if let Some(position) = player_vehicle_position {
        return (position, "vehicle");
    }
    (player_position, "ped")
}

/// re3 `CStreaming::StreamVehiclesAndPeds` + `StreamZoneModels`. Keep this
/// cadence independent from whether `GenerateRandomCars` can actually spawn a
/// car this frame; residency is world-streaming state, not a spawn side effect.
pub(crate) fn update_ambient_vehicle_and_zone_streaming(
    mut car_ctrl: ResMut<CCarCtrl>,
    mut traffic_models: ResMut<AmbientTrafficModels>,
    mut streaming: Option<ResMut<CStreaming>>,
    file_loader: Option<Res<CFileLoader>>,
    players: CWorldPlayersRead,
    vehicles: crate::control::ScriptEntities::VehicleCoreRead,
    camera: Option<Res<CCamera>>,
    remote: Option<Res<CRemote>>,
    clock: Res<CClock>,
    game_state: Res<CGameState>,
    population: Res<CPopulation>,
    gangs: Option<Res<CGangs>>,
    mut general: ResMut<CGeneral>,
    cutscene: Option<Res<CCutsceneMgr>>,
    bridge: Option<Res<ScriptGameplayBridge>>,
) {
    if cutscene.as_deref().is_some_and(|c| c.is_running())
        || game_state.playing_intro
        || bridge.as_deref().is_some_and(|b| !b.player_spawned)
        || game_state.curr_level == ELevelName::Generic
    {
        return;
    }
    let (Some(streaming), Some(file_loader), Some(player)) = (
        streaming.as_deref_mut(),
        file_loader.as_deref(),
        players.focused(),
    ) else {
        bevy::log::warn_once!(target: "re3bevy::ambient_streaming",
            missing_streaming = streaming.is_none(),
            missing_file_loader = file_loader.is_none(),
            missing_player = players.focused().is_none(),
            "required ambient streaming inputs are absent");
        return;
    };
    if streaming.num_models_requested >= 5 || file_loader.renderer.loading_priority {
        return;
    }
    if !traffic_models.built {
        traffic_models.build(file_loader, &mut car_ctrl);
    }
    if traffic_models.car_models.is_empty() {
        bevy::log::warn_once!(target: "re3bevy::ambient_streaming",
            "required ambient vehicle model table is empty");
        return;
    }

    let player_vehicle_position = players
        .relation(players.player_in_focus() as i32)
        .and_then(|relation| vehicles.get_entity(relation.vehicle))
        .map(|vehicle| vehicle.physical.entity.get_position());
    let remote_vehicle_position = remote
        .as_deref()
        .and_then(|remote| remote.controlled_vehicle_handle)
        .and_then(|handle| vehicles.get(handle))
        .map(|vehicle| vehicle.physical.entity.get_position());
    let (position, _) = ambient_traffic_target_position(
        &car_ctrl,
        player.position(),
        player_vehicle_position,
        remote_vehicle_position,
        camera.as_deref(),
    );
    let zone_info = ambient_zone_info(file_loader, position, &clock);
    service_ambient_vehicle_and_zone_streaming(
        streaming,
        file_loader,
        &mut car_ctrl,
        &population,
        gangs.as_deref(),
        zone_info,
        game_state.curr_level as i32,
        &player.wanted,
        &mut general,
    );
}

#[derive(Debug, Clone, Copy)]
struct AmbientSpawnParams {
    direction_x: f32,
    direction_y: f32,
    preferred_distance: f32,
    angle_limit: f32,
    invert_angle_limit_test: bool,
    test_for_collision: bool,
}

/// re3 `CCarCtrl::GenerateRandomCars` (`control/CarCtrl.cpp:91`).
///
/// The zone class ladder, `NextCarOfRating` cursors and the vehicle/ped zone
/// streaming that feeds `ChooseModel` are kept together here so generation
/// observes the same resident model set as re3.
#[derive(Default, Debug)]
pub(crate) struct TrafficSpawnDiagnostics {
    calls: u64,
    density_gate: u64,
    pool_gate: u64,
    model_none: u64,
    unloaded: u64,
    no_spawn_coors: u64,
    blocked: u64,
    not_close: u64,
    insert_fail: u64,
    rails_fail: u64,
    placement_fail: u64,
    final_reject: u64,
    success: u64,
    cutscene_gate: u64,
    streaming_missing: u64,
    loader_missing: u64,
    models_empty: u64,
    player_missing: u64,
    random_car_cap: u64,
}

pub(crate) fn process_traffic_car_generation(
    mut car_ctrl: ResMut<CCarCtrl>,
    mut traffic_models: ResMut<AmbientTrafficModels>,
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut general: ResMut<CGeneral>,
    mut file_loader: Option<ResMut<CFileLoader>>,
    handling_manager: Res<cHandlingDataMgr>,
    players: CWorldPlayersRead,
    camera: Option<Res<CCamera>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    streaming: Option<ResMut<CStreaming>>,
    timer: Res<CTimer>,
    taxi_lights: Res<ScriptTaxiLightState>,
    ambient: AmbientTrafficGenerationResources,
    mut scratch: Local<AmbientTrafficGenerationScratch>,
    mut debug_counts: Local<TrafficSpawnDiagnostics>,
) {
    crate::profile_scope!("process_traffic_car_generation");
    let _audit_timer = GameplayAuditTimer::start("process_traffic_car_generation");
    debug_counts.calls = debug_counts.calls.saturating_add(1);
    if debug_counts.calls % 300 == 0 {
        info!(target: "re3bevy::traffic", diagnostics = ?*debug_counts,
            random_cars = car_ctrl.num_random_cars,
            mission_cars = car_ctrl.num_mission_cars,
            density_multiplier = car_ctrl.car_density_multiplier,
            "traffic generation gates (cumulative)");
    }
    // `if (CCutsceneMgr::IsRunning()) return;`
    if cutscene.is_some_and(|cutscene| cutscene.running) {
        debug_counts.cutscene_gate += 1;
        return;
    }
    if streaming.is_none() {
        debug_counts.streaming_missing += 1;
        return;
    }
    if !traffic_models.built {
        let Some(loader) = file_loader.as_deref() else {
            debug_counts.loader_missing += 1;
            return;
        };
        traffic_models.build(loader, &mut car_ctrl);
    }
    if traffic_models.car_models.is_empty() {
        debug_counts.models_empty += 1;
        return;
    }
    let Some(player) = players.focused() else {
        debug_counts.player_missing += 1;
        return;
    };
    let wanted_level = player.wanted.get_wanted_level();
    let player_vehicle = players
        .relation(players.player_in_focus() as i32)
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle))
        .and_then(|handle| {
            script_entities.vehicle_get(handle).map(|vehicle| {
                (
                    vehicle.base().physical.entity.get_position(),
                    vehicle.base().physical.move_speed,
                )
            })
        });
    let remote_vehicle = ambient
        .remote
        .as_deref()
        .and_then(|remote| remote.controlled_vehicle_handle)
        .and_then(|handle| {
            script_entities.vehicle_get(handle).map(|vehicle| {
                (
                    vehicle.base().physical.entity.get_position(),
                    vehicle.base().physical.move_speed,
                )
            })
        });
    let (position, _target_source) = ambient_traffic_target_position(
        &car_ctrl,
        player.position(),
        player_vehicle.map(|(position, _)| position),
        remote_vehicle.map(|(position, _)| position),
        camera.as_deref(),
    );
    let target_move_speed = player_vehicle
        .map(|(_, speed)| speed)
        .unwrap_or(player.ped.physical.move_speed);
    let zone_info = file_loader
        .as_deref()
        .map(|loader| ambient_zone_info(loader, position, &ambient.clock))
        .unwrap_or_default();
    if car_ctrl.num_random_cars >= 30 {
        debug_counts.random_car_cap += 1;
        return;
    }

    let camera_front = camera
        .as_deref()
        .and_then(|camera| camera.cams.get(camera.active_cam).map(|cam| cam.front))
        .unwrap_or_else(|| player.ped.physical.entity.get_forward());
    let camera_xy_len = (camera_front.x * camera_front.x + camera_front.y * camera_front.y).sqrt();
    let camera_dir = if camera_xy_len > f32::EPSILON {
        (
            camera_front.x / camera_xy_len,
            camera_front.y / camera_xy_len,
        )
    } else {
        let forward = player.ped.physical.entity.get_forward();
        let len = (forward.x * forward.x + forward.y * forward.y)
            .sqrt()
            .max(f32::EPSILON);
        (forward.x / len, forward.y / len)
    };
    let generation_dist_multiplier = camera
        .as_deref()
        .map_or(1.0, |camera| camera.generation_dist_multiplier);
    let far_distance = 120.0 * generation_dist_multiplier;
    let spawn_params = if camera_front.z < -0.9 {
        // re3 top-down camera: direction is irrelevant and every bearing is
        // accepted at the close 40 m ring.
        AmbientSpawnParams {
            direction_x: 0.707,
            direction_y: 0.707,
            preferred_distance: 40.0,
            angle_limit: -1.0,
            invert_angle_limit_test: true,
            test_for_collision: false,
        }
    } else if let Some((_, speed)) = player_vehicle {
        let speed_2d = (speed.x * speed.x + speed.y * speed.y).sqrt();
        if speed_2d > 0.4 {
            let direction = (speed.x / speed_2d, speed.y / speed_2d);
            match timer.frame_counter() & 3 {
                0 | 1 => AmbientSpawnParams {
                    direction_x: direction.0,
                    direction_y: direction.1,
                    preferred_distance: far_distance,
                    angle_limit: 0.85,
                    invert_angle_limit_test: true,
                    test_for_collision: false,
                },
                2 => AmbientSpawnParams {
                    direction_x: direction.0,
                    direction_y: direction.1,
                    preferred_distance: far_distance,
                    angle_limit: 0.707,
                    invert_angle_limit_test: true,
                    test_for_collision: false,
                },
                _ => AmbientSpawnParams {
                    direction_x: direction.0,
                    direction_y: direction.1,
                    preferred_distance: 40.0,
                    angle_limit: 0.707,
                    invert_angle_limit_test: false,
                    test_for_collision: false,
                },
            }
        } else if speed_2d > 0.1 {
            let direction = (speed.x / speed_2d, speed.y / speed_2d);
            match timer.frame_counter() & 3 {
                0 => AmbientSpawnParams {
                    direction_x: direction.0,
                    direction_y: direction.1,
                    preferred_distance: far_distance,
                    angle_limit: 0.85,
                    invert_angle_limit_test: true,
                    test_for_collision: false,
                },
                1 => AmbientSpawnParams {
                    direction_x: direction.0,
                    direction_y: direction.1,
                    preferred_distance: far_distance,
                    angle_limit: 0.707,
                    invert_angle_limit_test: true,
                    test_for_collision: false,
                },
                _ => AmbientSpawnParams {
                    direction_x: direction.0,
                    direction_y: direction.1,
                    preferred_distance: 40.0,
                    angle_limit: 0.707,
                    invert_angle_limit_test: false,
                    test_for_collision: false,
                },
            }
        } else if timer.frame_counter() & 1 == 0 {
            AmbientSpawnParams {
                direction_x: camera_dir.0,
                direction_y: camera_dir.1,
                preferred_distance: far_distance,
                angle_limit: 0.707,
                invert_angle_limit_test: true,
                test_for_collision: true,
            }
        } else {
            AmbientSpawnParams {
                direction_x: camera_dir.0,
                direction_y: camera_dir.1,
                preferred_distance: 40.0,
                angle_limit: 0.707,
                invert_angle_limit_test: false,
                test_for_collision: true,
            }
        }
    } else if timer.frame_counter() & 1 == 0 {
        AmbientSpawnParams {
            direction_x: camera_dir.0,
            direction_y: camera_dir.1,
            preferred_distance: far_distance,
            angle_limit: 0.707,
            invert_angle_limit_test: true,
            test_for_collision: true,
        }
    } else {
        AmbientSpawnParams {
            direction_x: camera_dir.0,
            direction_y: camera_dir.1,
            preferred_distance: 40.0,
            angle_limit: 0.707,
            invert_angle_limit_test: false,
            test_for_collision: true,
        }
    };

    let mut to_spawn = 0usize;
    if car_ctrl.countdown_to_cars_at_start == 0 {
        to_spawn = 1;
    } else {
        car_ctrl.countdown_to_cars_at_start -= 1;
        if car_ctrl.countdown_to_cars_at_start == 0 {
            // re3 `CCarCtrl::GenerateRandomCars` performs 50 generation attempts
            // on the initial burst frame. Keep the attempt count: failed road,
            // collision, and closeness gates are expected and otherwise make
            // ambient traffic visibly sparse.
            to_spawn = 50;
        }
    }
    if to_spawn == 0 {
        return;
    }

    let Some(loader) = file_loader.as_deref() else {
        return;
    };
    scratch.collision_candidates.clear();
    scratch
        .collision_candidates
        .extend(script_entities.vehicles.values().filter_map(|vehicle| {
            let base = vehicle.base();
            if !base.physical.entity.flags.uses_collision {
                return None;
            }
            let model_id = base.physical.entity.model_index.max(0) as usize;
            let col = loader.model_info.col_model(model_id)?;
            Some(AmbientSpawnCollisionCandidate {
                matrix: base.physical.entity.matrix,
                radius: col.bounding_sphere.radius,
                is_vehicle: true,
            })
        }));
    scratch
        .collision_candidates
        .extend(script_entities.peds.iter().filter_map(|(&handle, entity)| {
            let ped = &entity.ped;
            let vitality = &entity.vitality;
            if script_entities.relation_for_char(handle).is_some()
                || !ped.physical.entity.flags.uses_collision
                || vitality.health <= 0.0
            {
                return None;
            }
            let model_id = ped.physical.entity.model_index.max(0) as usize;
            let col = loader.model_info.col_model(model_id)?;
            Some(AmbientSpawnCollisionCandidate {
                matrix: ped.physical.entity.matrix,
                radius: col.bounding_sphere.radius,
                is_vehicle: false,
            })
        }));
    scratch
        .collision_candidates
        .extend(players.iter().filter_map(|player| {
            if !player.active_in_world || !player.ped.physical.entity.flags.uses_collision {
                return None;
            }
            let model_id = player.ped.physical.entity.model_index.max(0) as usize;
            let col = loader.model_info.col_model(model_id)?;
            Some(AmbientSpawnCollisionCandidate {
                matrix: player.ped.physical.entity.matrix,
                radius: col.bounding_sphere.radius,
                is_vehicle: false,
            })
        }));

    let time_ms = timer.time_in_milliseconds();
    let road_density = file_loader
        .as_deref()
        .map(|loader| loader.path_find.calc_road_density(position.x, position.y))
        .unwrap_or(1.0)
        .clamp(0.4, 1.45);
    // re3 stores `m_fRoadDensity * zone.carDensity` in the player's int16
    // traffic multiplier before applying `CarDensityMultiplier`.  Preserve
    // that truncation as well as PlayerInfo's 0.4..1.45 road-density clamp;
    // using the raw density here can produce a zero traffic budget in valid
    // sparse-road sectors and permanently suppress ambient cars.
    let traffic_multiplier = (road_density * f32::from(zone_info.car_density.max(0))) as i16;
    let traffic_limit = f32::from(traffic_multiplier)
        * car_ctrl.car_density_multiplier
        * car_ctrl.car_number_multiplier;
    if car_ctrl.num_random_cars as f32 >= traffic_limit {
        debug_counts.density_gate = debug_counts.density_gate.saturating_add(1);
        return;
    }
    for _ in 0..to_spawn {
        if car_ctrl.num_random_cars >= 30 || car_ctrl.population_car_budget_full() {
            debug_counts.pool_gate = debug_counts.pool_gate.saturating_add(1);
            break;
        }
        let wanted_info = &player.wanted;
        let police_spawn_due = wanted_level > 1
            && car_ctrl.num_law_enforcer_cars < i32::from(wanted_info.m_MaximumLawEnforcerVehicles)
            && wanted_info.m_CurrentCops < wanted_info.m_MaxCops
            && (wanted_level > 3
                || (wanted_level > 2
                    && time_ms > car_ctrl.last_time_law_enforcer_created.wrapping_add(5_000))
                || (wanted_level > 1
                    && time_ms > car_ctrl.last_time_law_enforcer_created.wrapping_add(8_000)));
        let (model_id, chosen_class) = if police_spawn_due {
            (
                choose_police_car_model(
                    wanted_info,
                    streaming.as_deref().expect("streaming checked above"),
                    &mut general,
                ),
                AmbientCarClass::Cops,
            )
        } else {
            let Some(choice) = choose_ambient_car_model(
                &mut car_ctrl,
                zone_info,
                ambient.gangs.as_deref(),
                wanted_info,
                streaming.as_deref().expect("streaming checked above"),
                &mut general,
            ) else {
                debug_counts.model_none = debug_counts.model_none.saturating_add(1);
                break;
            };
            (choice.model_id, choice.class)
        };
        // re3 discards a police-class car chosen by the ordinary random
        // traffic roll while wanted-level pursuit cars are managed separately.
        if !police_spawn_due && wanted_level > 0 && chosen_class == AmbientCarClass::Cops {
            continue;
        }
        let model_index = model_id.max(0) as usize;
        let is_law_enforcer = matches!(
            model_index,
            crate::modelinfo::ModelIndices::MI_POLICE
                | crate::modelinfo::ModelIndices::MI_ENFORCER
                | crate::modelinfo::ModelIndices::MI_FBICAR
                | crate::modelinfo::ModelIndices::MI_RHINO
                | crate::modelinfo::ModelIndices::MI_BARRACKS
        );
        // `ChooseModel` only returns resident models. Forced police generation
        // can still fall back to the initially requested police car while a
        // higher wanted-tier model finishes streaming.
        if !streaming
            .as_deref()
            .is_some_and(|streaming| streaming.has_model_loaded(model_index))
        {
            debug_counts.unloaded = debug_counts.unloaded.saturating_add(1);
            continue;
        }
        // `NewGenerateCarCreationCoors` receives the player centre directly;
        // it chooses a path crossing the requested 40/120 m ring itself (CarCtrl.cpp:241).
        let Some(spawn) = file_loader.as_deref_mut().and_then(|loader| {
            ambient_car_spawn_position(
                loader,
                position,
                spawn_params.direction_x,
                spawn_params.direction_y,
                spawn_params.preferred_distance,
                spawn_params.angle_limit,
                spawn_params.invert_angle_limit_test,
                &mut general,
            )
        }) else {
            debug_counts.no_spawn_coors = debug_counts.no_spawn_coors.saturating_add(1);
            continue;
        };
        // re3 performs this broad 10 m vehicle/ped clearance test before the
        // path-closeness search and before allocating the new vehicle.
        if ambient_car_spawn_is_blocked(spawn.position, &scratch.collision_candidates) {
            debug_counts.blocked = debug_counts.blocked.saturating_add(1);
            continue;
        }
        // `GenerateOneRandomCar` rejects a spawn whose authored road network
        // cannot reach the player's centre (`PathFind.cpp:244-249`). Without
        // this gate, a valid-looking node on another disconnected/indoor path
        // is accepted and rails movement carries the car through map walls.
        if let Some(loader) = file_loader.as_deref_mut()
            && !loader.path_find.test_coors_closeness(
                position,
                crate::control::PathFind::PATH_CAR,
                spawn.position,
            )
        {
            debug_counts.not_close = debug_counts.not_close.saturating_add(1);
            continue;
        }
        let Some(loader) = file_loader.as_deref() else {
            break;
        };
        let (vehicle_type, handling_name, colours) =
            match loader.model_info.get_model_info(model_index) {
                Some(ModelInfoSlot::Vehicle(vehicle)) => {
                    let colours = if vehicle.num_colours > 0 {
                        let idx = (general.get_random_number() as usize) % vehicle.num_colours;
                        Some((vehicle.colours1[idx], vehicle.colours2[idx]))
                    } else {
                        None
                    };
                    (vehicle.vehicle_type, vehicle.handling_id.as_str(), colours)
                }
                _ => (eVehicleType::VEHICLE_TYPE_CAR as i32, "", None),
            };
        if vehicle_type != eVehicleType::VEHICLE_TYPE_CAR as i32 {
            continue;
        }

        let handling_id = cHandlingDataMgr::get_handling_id(handling_name);
        let handling = handling_id.and_then(|id| handling_manager.automobile_snapshot(id));
        let handling_data = handling_id
            .and_then(|id| handling_manager.get_handling_data(id))
            .copied();
        let mut car = CAutomobile::new(
            model_id,
            RANDOM_VEHICLE,
            time_ms,
            &mut general,
            handling,
            colours,
            taxi_lights.all_enabled,
        );
        car.vehicle.appearance.extras = choose_vehicle_optional_components(
            loader.model_info.get_model_info(model_index),
            &mut general,
            false,
        );
        if let (Some(ModelInfoSlot::Vehicle(vehicle_model)), Some(handling_data_ref)) = (
            loader.model_info.get_model_info(model_index),
            handling_data.as_ref(),
        )
        {
            car.setup_suspension_from_model(vehicle_model, handling_data_ref);
        }

        let traffic_class = match loader.model_info.get_model_info(model_index) {
            Some(ModelInfoSlot::Vehicle(vehicle)) => vehicle.vehicle_class.as_str(),
            _ => "",
        };
        let (speed_min, speed_max) = match (model_id as usize, traffic_class) {
            (crate::modelinfo::ModelIndices::MI_POLICE, _) => (12, 16),
            (_, "executive") => (12, 18),
            (_, "poorfamily" | "special") => (7, 10),
            _ => (9, 14),
        };
        let long_vehicle = traffic_class == "big"
            || loader
                .model_info
                .col_model(model_index)
                .is_some_and(|col| col.bounding_box.max.y - col.bounding_box.min.y > 10.0);
        let mut cruise_speed = general.get_random_number_in_range_i32(speed_min, speed_max) as u8;
        if long_vehicle {
            cruise_speed = cruise_speed.saturating_mul(3) / 4;
        }

        car.vehicle.core.vehicle_created_by = RANDOM_VEHICLE;
        car.vehicle.core.physical.entity.clump_alpha = 0;
        car.vehicle.core.physical.zone_level = crate::collision::ColModel::LEVEL_GENERIC as i8;
        car.vehicle.drive.engine_on = true;
        car.vehicle.core.flags.is_law_enforcer = is_law_enforcer;
        car.vehicle.core.auto_pilot.car_mission = if is_law_enforcer && wanted_level > 0 {
            CCarAI::FindPoliceCarMissionForWantedLevel(
                wanted_level.clamp(0, u8::MAX as i32) as u8,
                &mut general,
            )
        } else {
            eCarMission::MISSION_CRUISE
        };
        if is_law_enforcer && wanted_level > 0 {
            let transmission_max_velocity =
                handling_data.map_or(0.0, |handling| handling.transmission.max_velocity);
            let speed = CCarAI::FindPoliceCarSpeedForWantedLevel(
                wanted_level.clamp(0, u8::MAX as i32) as u8,
                transmission_max_velocity,
                &mut general,
            );
            car.vehicle.core.auto_pilot.cruise_speed = speed.clamp(0, u8::MAX as i32) as u8;
            car.vehicle.core.auto_pilot.max_traffic_speed = speed as f32 * 0.5;
            car.vehicle.core.auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
        } else {
            car.vehicle.core.auto_pilot.cruise_speed = cruise_speed;
            car.vehicle.core.auto_pilot.max_traffic_speed = cruise_speed as f32;
            car.vehicle.core.auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS;
        }
        car.vehicle.core.auto_pilot.anti_reverse_timer = time_ms;
        car.vehicle.core.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
        car.vehicle.core.auto_pilot.time_entered_curve = time_ms as i32;

        if model_index == crate::modelinfo::ModelIndices::MI_MRWHOOP {
            car.vehicle.audio.siren_or_alarm = true;
        }
        if model_index == crate::modelinfo::ModelIndices::MI_FBICAR && wanted_level > 0 {
            car.vehicle.appearance.current_colour1 = 0;
            car.vehicle.appearance.current_colour2 = 0;
        }

        let mut spawn_on_road = spawn;
        let car_length = loader
            .model_info
            .col_model(model_index)
            .map(|col| 1.0 + (col.bounding_box.max.y - col.bounding_box.min.y) * 0.5)
            .unwrap_or(1.0);
        let rails_initialised = crate::control::CarRails::initialise_ambient_traffic_car(
            &loader.path_find,
            &mut car.vehicle,
            &mut spawn_on_road,
            car_length,
            time_ms,
            &mut general,
        );
        if !rails_initialised {
            debug_counts.rails_fail = debug_counts.rails_fail.saturating_add(1);
            continue;
        }

        let curve_placed =
            place_ambient_traffic_car_at_creation_curve(&mut car, loader, spawn_on_road, time_ms);
        if !curve_placed {
            debug_counts.placement_fail = debug_counts.placement_fail.saturating_add(1);
            continue;
        }

        let final_position = car.vehicle.core.physical.entity.get_position();
        let Some(col) = loader.model_info.col_model(model_index) else {
            continue;
        };
        let radius = col.bounding_sphere.radius;
        let dx = final_position.x - position.x;
        let dy = final_position.y - position.y;
        let target_distance = (dx * dx + dy * dy).sqrt();
        let on_screen = camera
            .as_deref()
            .is_some_and(|camera| camera.is_sphere_visible(final_position, radius));
        if !on_screen {
            if target_distance > 50.0 {
                debug_counts.final_reject = debug_counts.final_reject.saturating_add(1);
                continue;
            }
        } else {
            if target_distance > generation_dist_multiplier * 130.0
                || target_distance < generation_dist_multiplier * 110.0
            {
                debug_counts.final_reject = debug_counts.final_reject.saturating_add(1);
                continue;
            }
            if let Some(camera) = camera.as_deref() {
                let camera_dx = final_position.x - camera.position.x;
                let camera_dy = final_position.y - camera.position.y;
                let minimum_camera_distance = 90.0 * generation_dist_multiplier;
                if camera_dx * camera_dx + camera_dy * camera_dy
                    < minimum_camera_distance * minimum_camera_distance
                {
                    debug_counts.final_reject = debug_counts.final_reject.saturating_add(1);
                    continue;
                }
            }
        }
        if ambient_car_spawn_is_blocked_with_extra_radius(
            final_position,
            model_index,
            &scratch.collision_candidates,
            loader,
            0.0,
        ) || (spawn_params.test_for_collision
            && ambient_car_spawn_has_nearby_entity(
                final_position,
                model_index,
                &scratch.collision_candidates,
                loader,
                20.0,
            ))
        {
            debug_counts.final_reject = debug_counts.final_reject.saturating_add(1);
            continue;
        }
        let relative_speed = CVector::new(
            car.vehicle.core.physical.move_speed.x - target_move_speed.x,
            car.vehicle.core.physical.move_speed.y - target_move_speed.y,
            0.0,
        );
        if relative_speed.x * dx + relative_speed.y * dy >= 0.0 {
            debug_counts.final_reject = debug_counts.final_reject.saturating_add(1);
            continue;
        }

        let spawn_with_real_physics = general.get_random_number() & 0x3F == 0;
        if is_law_enforcer && car.vehicle.core.auto_pilot.car_mission != eCarMission::MISSION_CRUISE {
            car.vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
        } else if spawn_with_real_physics {
            car.vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
            car.vehicle.core.auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
            car.vehicle.core.auto_pilot.cruise_speed =
                car.vehicle.core.auto_pilot.cruise_speed.saturating_add(10);
            car.vehicle.core.auto_pilot.max_traffic_speed = car.vehicle.core.auto_pilot.cruise_speed as f32;
        } else {
            car.vehicle.set_status(eEntityStatus::STATUS_SIMPLE);
        }
        car.vehicle.drive.engine_on = true;
        car.vehicle.core.physical.entity.clump_alpha = 0;

        let police_occupants_generated = matches!(
            model_index,
            crate::modelinfo::ModelIndices::MI_FBICAR
                | crate::modelinfo::ModelIndices::MI_ENFORCER
                | crate::modelinfo::ModelIndices::MI_POLICE
                | crate::modelinfo::ModelIndices::MI_RHINO
                | crate::modelinfo::ModelIndices::MI_BARRACKS
        );
        if police_occupants_generated {
            car.vehicle.core.flags.occupants_have_been_generated = true;
        }

        let car_matrix = car.vehicle.core.physical.entity.matrix;
        let uses_collision = car.vehicle.core.physical.entity.flags.uses_collision;

        let Some(pool_handle) = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))
        else {
            debug_counts.insert_fail = debug_counts.insert_fail.saturating_add(1);
            continue;
        };
        let handle = pool_handle.0;

        script_entities.spawn_vehicle(ScriptVehicleEntity::Automobile { handle, car });

        let Some(streaming_ref) = streaming.as_deref() else {
            continue;
        };
        let (driver_ped_type, driver_model_id) = ambient_driver_profile(
            model_index,
            &mut general,
            streaming_ref,
            loader,
            &ambient.population,
            ambient.gangs.as_deref(),
            zone_info.ped_group as usize,
        );
        let mut driver_ped = CPed::new(script_ped_type(driver_ped_type), &mut general);
        let mut driver_vitality = crate::peds::PedVitality::PedVitality::default();
        driver_ped
            .physical
            .entity
            .set_model_index_no_create(driver_model_id.max(0) as usize);
        if driver_ped.ped_type == ePedType::PEDTYPE_COP {
            let cop_type = crate::peds::CopPed::cop_type_for_model(driver_model_id.max(0) as usize);
            crate::peds::CopPed::initialise_cop_ped_with_info(&mut driver_ped, &mut driver_vitality, cop_type, &ambient.weapon_info);
        }
        driver_ped.char_created_by = RANDOM_CHAR;
        driver_ped.ped_state = PedState::PED_DRIVING;
        driver_ped.physical.entity.flags.uses_collision = false;
        driver_ped.physical.entity.clump_alpha = 0;
        driver_ped.physical.entity.set_position(final_position);
        driver_ped.physical.zone_level = crate::collision::ColModel::LEVEL_GENERIC as i8;
        driver_ped.flags.responds_to_threats = true;
        driver_ped.flags.allow_medics_to_revive_me = true;
        driver_ped.flags.render_ped_in_car = true;
        if driver_ped.ped_type != ePedType::PEDTYPE_COP {
            let colt = crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_COLT45;
            if driver_ped.weapons[colt as usize].weapon_type == colt {
                driver_ped.current_weapon = colt;
            }
        }
        if let Some(driver_pool_handle) = pools
            .ped_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Ped))
        {
            let driver_handle = driver_pool_handle.0;
            let driver_entity = ScriptPedEntity {
            locomotion: Default::default(),
                vitality: driver_vitality,
                animation: crate::peds::PedAnimation::PedAnimation::default(),
                handle: driver_handle,
                ped: driver_ped,
                model_id: driver_model_id,
            };
            script_entities.spawn_ped(driver_entity);
            script_entities.set_char_relation(driver_handle, handle, VehicleSeat::Driver);
        }

        let passenger_seats: &[i32] = match model_index {
            crate::modelinfo::ModelIndices::MI_FBICAR
            | crate::modelinfo::ModelIndices::MI_ENFORCER => &[0, 1, 2],
            crate::modelinfo::ModelIndices::MI_POLICE
            | crate::modelinfo::ModelIndices::MI_RHINO
            | crate::modelinfo::ModelIndices::MI_BARRACKS
                if wanted_level > 1 =>
            {
                &[0]
            }
            crate::modelinfo::ModelIndices::MI_FIRETRUCK => &[0],
            crate::modelinfo::ModelIndices::MI_AMBULAN => &[1],
            _ => &[],
        };
        for &passenger_seat in passenger_seats {
            let (ped_type, ped_model_id) = ambient_driver_profile(
                model_index,
                &mut general,
                streaming_ref,
                loader,
                &ambient.population,
                ambient.gangs.as_deref(),
                zone_info.ped_group as usize,
            );
            let mut passenger_ped = CPed::new(script_ped_type(ped_type), &mut general);
        let mut passenger_vitality = crate::peds::PedVitality::PedVitality::default();
            passenger_ped
                .physical
                .entity
                .set_model_index_no_create(ped_model_id.max(0) as usize);
            if passenger_ped.ped_type == ePedType::PEDTYPE_COP {
                let cop_type =
                    crate::peds::CopPed::cop_type_for_model(ped_model_id.max(0) as usize);
                crate::peds::CopPed::initialise_cop_ped_with_info(&mut passenger_ped, &mut passenger_vitality, cop_type, &ambient.weapon_info);
            }
            passenger_ped.char_created_by = RANDOM_CHAR;
            passenger_ped.ped_state = PedState::PED_DRIVING;
            passenger_ped.physical.entity.flags.uses_collision = false;
            passenger_ped.physical.entity.clump_alpha = 0;
            passenger_ped.physical.entity.set_position(final_position);
            passenger_ped.physical.zone_level = crate::collision::ColModel::LEVEL_GENERIC as i8;
            passenger_ped.flags.responds_to_threats = true;
            passenger_ped.flags.allow_medics_to_revive_me = true;
            passenger_ped.flags.render_ped_in_car = true;
            if passenger_ped.ped_type != ePedType::PEDTYPE_COP {
                let colt = crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_COLT45;
                if passenger_ped.weapons[colt as usize].weapon_type == colt {
                    passenger_ped.current_weapon = colt;
                }
            }
            if let Some(passenger_pool_handle) = pools
                .ped_pool
                .new_entry(CPoolEntry::new(CPoolEntryKind::Ped))
            {
                let passenger_handle = passenger_pool_handle.0;
                let passenger_entity = ScriptPedEntity {
            locomotion: Default::default(),
                vitality: passenger_vitality,
                    animation: crate::peds::PedAnimation::PedAnimation::default(),
                    handle: passenger_handle,
                    ped: passenger_ped,
                    model_id: ped_model_id,
                };
                script_entities.spawn_ped(passenger_entity);
                script_entities.set_char_relation(
                    passenger_handle,
                    handle,
                    VehicleSeat::Passenger(passenger_seat as u8),
                );
            }
        }

        if uses_collision {
            scratch
                .collision_candidates
                .push(AmbientSpawnCollisionCandidate {
                    matrix: car_matrix,
                    radius: col.bounding_sphere.radius,
                    is_vehicle: true,
                });
        }
        car_ctrl.num_random_cars += 1;
        debug_counts.success = debug_counts.success.saturating_add(1);
        if is_law_enforcer {
            car_ctrl.num_law_enforcer_cars += 1;
            car_ctrl.last_time_law_enforcer_created = time_ms;
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct EmergencyServiceGenerationScratch {
    collision_candidates: Vec<AmbientSpawnCollisionCandidate>,
}

#[derive(Debug, Clone, Copy)]
struct EmergencyServiceSpawn {
    vehicle_model: usize,
    ped_model: usize,
    ped_type: ePedType,
    passenger_seat: i32,
    destination: CVector,
    ambulance: bool,
}

/// re3 `CCarCtrl::GenerateEmergencyServicesCar` and
/// `GenerateOneEmergencyServicesCar` (`control/CarCtrl.cpp`).  The original
/// services this from `GenerateRandomCars` whenever the timer crosses a 4.096s
/// boundary; keeping it as a separate Bevy system preserves the cadence while
/// leaving ambient traffic generation ECS-owned.
pub(crate) fn process_emergency_service_generation(
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut car_ctrl: ResMut<CCarCtrl>,
    mut population: ResMut<CPopulation>,
    accidents: Res<CAccidentManager>,
    fires: Res<CFireManager>,
    players: CWorldPlayersRead,
    cutscene: Option<Res<CCutsceneMgr>>,
    mut file_loader: ResMut<CFileLoader>,
    handling_manager: Res<cHandlingDataMgr>,
    mut streaming: ResMut<CStreaming>,
    taxi_lights: Res<ScriptTaxiLightState>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut scratch: Local<EmergencyServiceGenerationScratch>,
) {
    if cutscene
        .as_deref()
        .is_some_and(CCutsceneMgr::is_cutscene_processing)
    {
        return;
    }
    let now_ms = timer.time_in_milliseconds();
    let previous_ms = timer.previous_time_in_milliseconds();
    if (now_ms & 0xffff_f000) == (previous_ms & 0xffff_f000) {
        return;
    }
    let Some(player) = players.focused() else {
        bevy::log::warn_once!(target: "re3bevy::emergency_generation",
            "required materialized player input is absent");
        return;
    };
    if player.wanted.get_wanted_level() > 3 {
        return;
    }

    car_ctrl.num_ambulances_on_duty = script_entities
        .vehicles
        .values()
        .filter(|vehicle| vehicle.base().flags.is_ambulance_on_duty)
        .count() as i32;
    car_ctrl.num_firetrucks_on_duty = script_entities
        .vehicles
        .values()
        .filter(|vehicle| vehicle.base().flags.is_fire_truck_on_duty)
        .count() as i32;
    if car_ctrl.num_cars_in_use_for_population() > car_ctrl.max_number_of_cars_in_use {
        return;
    }

    let player_position = players
        .relation(players.player_in_focus() as i32)
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle))
        .and_then(|handle| script_entities.vehicle_get(handle))
        .map(|vehicle| vehicle.base().physical.entity.get_position())
        .unwrap_or_else(|| player.position());

    let mut requests = Vec::with_capacity(2);
    let ambulance_count = script_entities
        .vehicles
        .values()
        .filter(|vehicle| {
            vehicle.base().physical.entity.model_index.max(0) as usize
                == crate::modelinfo::ModelIndices::MI_AMBULAN
        })
        .count();
    if car_ctrl.num_ambulances_on_duty == 0
        && accidents.count_active_accidents() >= 2
        && ambulance_count < 2
        && now_ms > car_ctrl.last_time_ambulance_created.wrapping_add(30_000)
        && let Some((slot, distance)) = accidents.find_nearest_accident(player_position, |victim| {
            script_entities.peds.values().find_map(|entity| {
                (CPoolHandle(entity.handle).index() == victim).then_some(CAccidentVictimState {
                    position: entity.ped.physical.entity.get_position(),
                    char_created_by: entity.ped.char_created_by,
                    health: entity.vitality.health,
                })
            })
        })
        && distance < 30.0
        && let Some(victim_slot) = accidents
            .accidents
            .get(slot)
            .and_then(|accident| accident.victim)
        && let Some(destination) = script_entities.peds.values().find_map(|entity| {
            (CPoolHandle(entity.handle).index() == victim_slot)
                .then_some(entity.ped.physical.entity.get_position())
        })
    {
        requests.push(EmergencyServiceSpawn {
            vehicle_model: crate::modelinfo::ModelIndices::MI_AMBULAN,
            ped_model: crate::modelinfo::ModelIndices::MI_MEDIC,
            ped_type: ePedType::PEDTYPE_EMERGENCY,
            passenger_seat: 1,
            destination,
            ambulance: true,
        });
    }

    let firetruck_count = script_entities
        .vehicles
        .values()
        .filter(|vehicle| {
            vehicle.base().physical.entity.model_index.max(0) as usize
                == crate::modelinfo::ModelIndices::MI_FIRETRUCK
        })
        .count();
    if car_ctrl.num_firetrucks_on_duty == 0
        && fires.GetTotalActiveFires() >= 3
        && firetruck_count < 2
        && now_ms > car_ctrl.last_time_fire_truck_created.wrapping_add(35_000)
        && let Some((slot, distance)) = fires.FindNearestFire(player_position)
        && distance < 30.0
        && let Some(fire) = fires.fires.get(slot)
    {
        requests.push(EmergencyServiceSpawn {
            vehicle_model: crate::modelinfo::ModelIndices::MI_FIRETRUCK,
            ped_model: crate::modelinfo::ModelIndices::MI_FIREMAN,
            ped_type: ePedType::PEDTYPE_FIREMAN,
            passenger_seat: 0,
            destination: fire.position,
            ambulance: false,
        });
    }

    for request in requests {
        streaming.request_model_with_adapter(
            request.vehicle_model,
            STREAMFLAGS_DEPENDENCY,
            &*file_loader,
        );
        streaming.request_model_with_adapter(
            request.ped_model,
            STREAMFLAGS_DONT_REMOVE,
            &*file_loader,
        );
        if !streaming.has_model_loaded(request.vehicle_model)
            || !streaming.has_model_loaded(request.ped_model)
        {
            continue;
        }

        scratch.collision_candidates.clear();
        scratch
            .collision_candidates
            .extend(script_entities.vehicles.values().filter_map(|vehicle| {
                let base = vehicle.base();
                if !base.physical.entity.flags.uses_collision {
                    return None;
                }
                let model_id = base.physical.entity.model_index.max(0) as usize;
                let col = file_loader.model_info.col_model(model_id)?;
                Some(AmbientSpawnCollisionCandidate {
                    matrix: base.physical.entity.matrix,
                    radius: col.bounding_sphere.radius,
                    is_vehicle: true,
                })
            }));
        scratch
            .collision_candidates
            .extend(script_entities.peds.iter().filter_map(|(&handle, entity)| {
                let ped = &entity.ped;
                let vitality = &entity.vitality;
                if script_entities.relation_for_char(handle).is_some()
                    || !ped.physical.entity.flags.uses_collision
                    || vitality.health <= 0.0
                {
                    return None;
                }
                let model_id = ped.physical.entity.model_index.max(0) as usize;
                let col = file_loader.model_info.col_model(model_id)?;
                Some(AmbientSpawnCollisionCandidate {
                    matrix: ped.physical.entity.matrix,
                    radius: col.bounding_sphere.radius,
                    is_vehicle: false,
                })
            }));
        scratch
            .collision_candidates
            .extend(players.iter().filter_map(|player| {
                if !player.active_in_world || !player.ped.physical.entity.flags.uses_collision {
                    return None;
                }
                let model_id = player.ped.physical.entity.model_index.max(0) as usize;
                let col = file_loader.model_info.col_model(model_id)?;
                Some(AmbientSpawnCollisionCandidate {
                    matrix: player.ped.physical.entity.matrix,
                    radius: col.bounding_sphere.radius,
                    is_vehicle: false,
                })
            }));

        let mut creation = None;
        for _ in 0..5 {
            let Some(candidate) = ambient_car_spawn_position(
                &mut file_loader,
                player_position,
                0.707,
                0.707,
                120.0,
                -1.0,
                true,
                &mut general,
            ) else {
                continue;
            };
            if !ambient_car_spawn_is_blocked(candidate.position, &scratch.collision_candidates) {
                creation = Some(candidate);
                break;
            }
        }
        let Some(creation) = creation else {
            continue;
        };

        let Some(handle) = script_entities.insert_script_car(
            &mut pools,
            ScriptCreateCarRequest {
                model_id: request.vehicle_model as i32,
                position: creation.position,
                time_ms: now_ms,
                clear_space_for_mission_entity: false,
            },
            file_loader.model_info.get_model_info(request.vehicle_model),
            Some(&handling_manager),
            &mut general,
            &file_loader,
            taxi_lights.all_enabled,
        ) else {
            continue;
        };
        if let Some(mut vehicle) = script_entities.vehicle_get_mut(handle) {
            let mut base = vehicle.base_mut();
            base.vehicle_created_by = RANDOM_VEHICLE;
            base.physical.zone_level = crate::collision::ColModel::LEVEL_GENERIC as i8;
        }

        let driver = script_entities.insert_script_char(
            &mut pools,
            ScriptCreateCharRequest {
                ped_type: request.ped_type as i32,
                model_id: request.ped_model as i32,
                position: creation.position,
                vehicle_handle: Some(handle),
                passenger_seat: None,
            },
            &mut general,
            None,
            &file_loader,
        );
        let Some(driver_handle) = driver else {
            remove_script_vehicle_and_occupants(
                &mut script_entities,
                &mut pools,
                handle,
                None,
                None,
            );
            continue;
        };
        for ped_handle in [
            Some(driver_handle),
            script_entities.insert_script_char(
                &mut pools,
                ScriptCreateCharRequest {
                    ped_type: request.ped_type as i32,
                    model_id: request.ped_model as i32,
                    position: creation.position,
                    vehicle_handle: Some(handle),
                    passenger_seat: Some(request.passenger_seat),
                },
                &mut general,
                None,
                &file_loader,
            ),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(entity) = script_entities.ped_get_mut(ped_handle) {
                entity.ped.char_created_by = RANDOM_CHAR;
                entity.ped.physical.zone_level = crate::collision::ColModel::LEVEL_GENERIC as i8;
                entity.ped.flags.responds_to_threats = true;
                entity.ped.flags.allow_medics_to_revive_me = true;
                entity.ped.flags.render_ped_in_car = true;
                add_ped_to_population_counts(&mut population, entity.ped.ped_type);
            }
        }

        let mut initialised = false;
        if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
            script_entities.vehicle_get_mut(handle)
        {
            let mut creation_on_road = creation;
            let car_length = file_loader
                .model_info
                .col_model(car.vehicle.core.physical.entity.model_index.max(0) as usize)
                .map(|col| 1.0 + (col.bounding_box.max.y - col.bounding_box.min.y) * 0.5)
                .unwrap_or(1.0);
            initialised = crate::control::CarRails::initialise_ambient_traffic_car(
                &file_loader.path_find,
                &mut car.vehicle,
                &mut creation_on_road,
                car_length,
                now_ms,
                &mut general,
            ) && place_ambient_traffic_car_at_creation_curve(
                &mut car,
                &file_loader,
                creation_on_road,
                now_ms,
            );
            if initialised {
                let start = car.vehicle.core.physical.entity.get_position();
                let straight = join_car_with_road_system_goto_coors(
                    &mut file_loader.path_find,
                    &mut car.vehicle,
                    request.destination,
                    &mut general,
                );
                car.vehicle.core.auto_pilot.car_mission = if straight {
                    eCarMission::MISSION_GOTOCOORDS_STRAIGHT
                } else {
                    eCarMission::MISSION_GOTOCOORDS
                };
                car.vehicle.core.auto_pilot.cruise_speed = 25;
                car.vehicle.core.auto_pilot.max_traffic_speed = 25.0;
                car.vehicle.core.auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
                car.vehicle.core.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                car.vehicle.core.auto_pilot.anti_reverse_timer = now_ms;
                car.vehicle.core.physical.move_speed = CVector::ZERO;
                car.vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
                car.vehicle.audio.siren_or_alarm = true;
                car.vehicle.core.flags.is_ambulance_on_duty = request.ambulance;
                car.vehicle.core.flags.is_fire_truck_on_duty = !request.ambulance;
                let direction = CVector::new(
                    request.destination.x - start.x,
                    request.destination.y - start.y,
                    0.0,
                );
                let length = (direction.x * direction.x + direction.y * direction.y).sqrt();
                if length > f32::EPSILON {
                    let forward = CVector::new(direction.x / length, direction.y / length, 0.0);
                    car.vehicle.core.physical.entity.matrix.forward = forward;
                    car.vehicle.core.physical.entity.matrix.right =
                        CVector::new(forward.y, -forward.x, 0.0);
                    car.vehicle.core.physical.entity.matrix.up = CVector::new(0.0, 0.0, 1.0);
                }
            }
        }
        if !initialised {
            remove_script_vehicle_and_occupants(
                &mut script_entities,
                &mut pools,
                handle,
                None,
                None,
            );
            continue;
        }

        car_ctrl.num_random_cars += 1;
        if request.ambulance {
            car_ctrl.num_ambulances_on_duty += 1;
            car_ctrl.last_time_ambulance_created = now_ms;
        } else {
            car_ctrl.num_firetrucks_on_duty += 1;
            car_ctrl.last_time_fire_truck_created = now_ms;
        }
    }
}

#[derive(Default)]
pub(crate) struct SimpleAutomobileControlScratch {
    vehicle_scans: Vec<TrafficVehicleScan>,
    ped_scans: Vec<TrafficPedScan>,
    traffic_index: TrafficSpatialIndex,
    simple_handles: Vec<i32>,
    occupants_to_leave: Vec<(i32, usize)>,
    occupant_handles: Vec<i32>,
    vehicle_position_by_slot: HashMap<usize, CVector>,
    driver_kill_player_any_means: HashSet<i32>,
}

/// re3 `CAutomobile::ProcessControl` for `STATUS_SIMPLE` traffic cars
/// (`vehicles/Automobile.cpp:374-393`) plus the pre-switch centre-of-mass and
/// slowed-flag resets every status shares (`Automobile.cpp:321-330`):
/// `CCarAI::UpdateCarAI` → `CPhysical::ProcessControl` (simple early-out) →
/// `CCarCtrl::UpdateCarOnRails` → simple gear/wheel update.
pub(crate) fn process_simple_automobile_control(
    mut script_entities: ScriptEntityWorld,
    mut file_loader: ResMut<CFileLoader>,
    collision: Res<CCollision>,
    mut handling_manager: ResMut<cHandlingDataMgr>,
    world_players: CWorldPlayersRead,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut scratch: Local<SimpleAutomobileControlScratch>,
) {
    crate::profile_scope!("process_simple_automobile_control");
    let _audit_timer = GameplayAuditTimer::start("process_simple_automobile_control");
    let time_ms = timer.time_in_milliseconds();
    let time_step = timer.time_step().max(0.0);

    let Some(player) = world_players.focused() else {
        bevy::log::warn_once!(target: "re3bevy::simple_vehicle_control",
            "required materialized player input is absent");
        return;
    };
    let wanted_level = player.wanted.get_wanted_level();
    let ignored_by_everyone = player.wanted.m_bIgnoredByEveryone;
    let ignored_by_cops = player.wanted.m_bIgnoredByCops;
    let player_position = player.position();
    let player_move_speed = player.ped.physical.move_speed;
    let player_forward = player.ped.physical.entity.get_forward();
    let player_vehicle_handle = world_players
        .relation(world_players.player_in_focus() as i32)
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle));
    let player_vehicle_slot = player_vehicle_handle.map(|handle| CPoolHandle(handle).index());
    let target_player_position = player_vehicle_handle
        .and_then(|handle| script_entities.vehicle_get(handle))
        .map_or(player_position, |vehicle| {
            vehicle.base().physical.entity.get_position()
        });
    let player_vehicle_state = player_vehicle_handle
        .and_then(|handle| script_entities.vehicle_get(handle))
        .map(|vehicle| {
            let base = vehicle.base();
            (
                base.physical.move_speed.magnitude(),
                base.physical.entity.get_up().z < 0.0,
                base.physical.entity.flags.has_collided,
            )
        });

    scratch.vehicle_scans.clear();
    scratch.ped_scans.clear();
    scratch.traffic_index.begin_frame();
    scratch.simple_handles.clear();
    scratch.occupants_to_leave.clear();
    scratch.occupant_handles.clear();
    scratch.vehicle_position_by_slot.clear();
    scratch.driver_kill_player_any_means.clear();

    // re3 CCarAI reads pVehicle->pDriver->m_objective directly. Occupancy is
    // relation-owned in ECS, so snapshot the driver objective before taking
    // mutable vehicle borrows below.
    for (ped_handle, relation) in script_entities.script_relations() {
        if !relation.is_driver() {
            continue;
        }
        let kill_any_means = script_entities.peds.get(&script_entities.index, &ped_handle)
            .is_some_and(|ped| ped.ped.objective == eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS);
        if !kill_any_means {
            continue;
        }
        if let Some(vehicle_handle) = script_entities.vehicle_handle_for_entity(relation.vehicle) {
            scratch.driver_kill_player_any_means.insert(vehicle_handle);
        }
    }

    // Snapshots for the traffic/ped slow-down scans. Retain allocation across
    // frames and cache slot -> position for UpdateCarAI target lookups.
    for (&handle, vehicle) in script_entities.vehicles.iter() {
        let base = vehicle.base();
        let zone_resident = base.physical.zone_level
            <= crate::collision::ColModel::LEVEL_GENERIC as i8
            || base.physical.zone_level == collision.collision_in_memory as i8;
        if base.physical.entity.status == eEntityStatus::STATUS_SIMPLE && zone_resident {
            scratch.simple_handles.push(handle);
        }
        let model_id = base.physical.entity.model_index.max(0) as usize;
        let Some(col) = file_loader.model_info.col_model(model_id) else {
            continue;
        };
        let slot = CPoolHandle(handle).index();
        let position = base.physical.entity.get_position();
        scratch.vehicle_position_by_slot.insert(slot, position);
        let scan_index = scratch.vehicle_scans.len();
        scratch.vehicle_scans.push(TrafficVehicleScan {
            slot,
            position,
            forward: base.physical.entity.get_forward(),
            move_speed: base.physical.move_speed,
            bbox_min: CVector::new(
                col.bounding_box.min.x,
                col.bounding_box.min.y,
                col.bounding_box.min.z,
            ),
            bbox_max: CVector::new(
                col.bounding_box.max.x,
                col.bounding_box.max.y,
                col.bounding_box.max.z,
            ),
            is_player_vehicle: Some(slot) == player_vehicle_slot,
            time_to_start_mission: base.auto_pilot.time_to_start_mission,
        });
        scratch.traffic_index.insert_vehicle(scan_index, position);
    }
    for entity in script_entities.peds.values() {
        if entity.ped.physical.entity.flags.uses_collision && entity.vitality.health > 0.0 {
            let scan_index = scratch.ped_scans.len();
            let position = entity.ped.physical.entity.get_position();
            scratch.ped_scans.push(TrafficPedScan { position });
            scratch.traffic_index.insert_ped(scan_index, position);
        }
    }

    // Deterministic per-entity processing order.
    scratch
        .simple_handles
        .sort_by_key(|handle| CPoolHandle(*handle).index());

    let SimpleAutomobileControlScratch {
        vehicle_scans,
        ped_scans,
        traffic_index,
        simple_handles,
        occupants_to_leave,
        occupant_handles,
        vehicle_position_by_slot,
        driver_kill_player_any_means,
    } = &mut *scratch;

    // `CCarAI::UpdateCarAI` can ask every occupant to leave. Queue those
    // objectives while the vehicle borrow is live, then apply them after the
    // per-car pass so the ECS map is not mutably borrowed twice.
    for handle in simple_handles.iter().copied() {
        let self_slot = CPoolHandle(handle).index();
        // Rails movement also supplies same-frame history to ped/body collision.
        script_entities.vehicles.capture_collision_pose(&script_entities.index, handle);
        let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
            script_entities.vehicle_get_mut(handle)
        else {
            continue;
        };
        let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
        let Some(col) = file_loader.model_info.col_model(model_id) else {
            // re3 `CAutomobile::ProcessControl` early-outs when the col model
            // for the current level is not in memory (Automobile.cpp:238-239).
            continue;
        };
        let bbox_min = CVector::new(
            col.bounding_box.min.x,
            col.bounding_box.min.y,
            col.bounding_box.min.z,
        );
        let bbox_max = CVector::new(
            col.bounding_box.max.x,
            col.bounding_box.max.y,
            col.bounding_box.max.z,
        );
        // bWarnedPeds = false (Automobile.cpp:235).
        car.vehicle.core.flags.warned_peds = false;
        // AutoPilot.m_bSlowedDownBecauseOf{Cars,Peds} = false
        // (Automobile.cpp:321-322).
        car.vehicle.core.auto_pilot.slowed_down_because_of_cars = false;
        car.vehicle.core.auto_pilot.slowed_down_because_of_peds = false;

        let handling_id = car.vehicle.core.handling_id.or_else(|| {
            match file_loader.model_info.get_model_info(model_id) {
                Some(ModelInfoSlot::Vehicle(model)) => {
                    cHandlingDataMgr::get_handling_id(&model.handling_id)
                        .and_then(|id| i32::try_from(id).ok())
                }
                _ => None,
            }
        });
        let Some(handling_id) = handling_id else {
            continue;
        };
        car.vehicle.core.handling_id = Some(handling_id);
        let Ok(handling_index) = usize::try_from(handling_id) else {
            continue;
        };

        // Suspension setup supplies `m_fHeightAboveRoad` for DragCarToPoint
        // (C++ computes it in SetupSuspensionData after SetModelIndex).
        if let Some(ModelInfoSlot::Vehicle(model)) =
                file_loader.model_info.get_model_info(model_id)

            && let Some(handling) = handling_manager.get_handling_data_mut(handling_index)
        {
            car.setup_suspension_from_model(model, handling);
        }
        // Centre of mass: default branch (Automobile.cpp:329-330).
        if let Some(handling) = handling_manager.get_handling_data(handling_index) {
            car.vehicle.core.physical.centre_of_mass.z = handling.centre_of_mass.z;
        }

        // ---- CCarAI::UpdateCarAI (control/CarAI.cpp).
        let up = car.vehicle.core.physical.entity.get_up();
        let ai_input = CarAiStepInput {
            status: car.vehicle.core.physical.entity.status,
            position: car.vehicle.core.physical.entity.get_position(),
            move_speed: car.vehicle.core.physical.move_speed,
            forward: car.vehicle.core.physical.entity.get_forward(),
            up_z: up.z,
            is_law_enforcer: car.vehicle.core.flags.is_law_enforcer,
            uses_siren: crate::vehicles::Vehicle::vehicle_uses_siren(model_id),
            siren_or_alarm: car.vehicle.audio.siren_or_alarm,
            is_rhino: model_id == crate::modelinfo::ModelIndices::MI_RHINO as usize,
            is_ambulance_on_duty: car.vehicle.core.flags.is_ambulance_on_duty,
            is_fire_truck_on_duty: car.vehicle.core.flags.is_fire_truck_on_duty,
            created_by: car.vehicle.core.vehicle_created_by,
            random_seed: car.vehicle.core.physical.entity.random_seed,
            last_collision_ms: car.vehicle.core.physical.last_time_collided,
            time_blocked_ms: u32::from(car.vehicle.core.time_blocked),
            now_ms: time_ms,
            previous_ms: time_ms
                .wrapping_sub((timer.time_step_in_seconds().max(0.0) * 1_000.0) as u32),
            time_step_ms: time_step,
            frame_counter: timer.frame_counter(),
            player_position,
            player_vehicle_speed: player_vehicle_state.map(|state| state.0),
            player_move_speed,
            player_forward,
            player_vehicle_upside_down: player_vehicle_state.is_some_and(|state| state.1),
            player_vehicle_collided: player_vehicle_state.is_some_and(|state| state.2),
            driver_kill_player_any_means: driver_kill_player_any_means.contains(&handle),
            player_ignored_by_everyone: ignored_by_everyone,
            player_ignored_by_cops: ignored_by_cops,
            wanted_level: wanted_level.clamp(0, u8::MAX as i32) as u8,
            // `CCarAI::UpdateCarAI` consults `CCullZones::NoPolice()` before
            // every pursuit/roadblock branch.  The previous hard-coded false
            // made police traffic keep chasing the player inside authored
            // no-police volumes (and also ignored the bridge/indoor cull
            // state refreshed by `update_cull_zone_runtime`).
            no_police_zone: file_loader.world.cull_zones.player_no_police(),
            // TODO(parity): CTrafficLights::ShouldCarStopForBridge admission.
            bridge_requires_stop: false,
            target_car_position: car
                .vehicle.core.auto_pilot
                .target_car
                .and_then(|slot| vehicle_position_by_slot.get(&slot).copied()),
            target_car_is_player_vehicle: car
                .vehicle.core.auto_pilot
                .target_car
                .is_some_and(|slot| Some(slot) == player_vehicle_slot),
            target_car_collided: false,
        };
        let transmission_max_velocity = handling_manager
            .get_handling_data(handling_index)
            .map(|handling| handling.transmission.max_velocity)
            .unwrap_or(0.0);
        let ai_result = CCarAI::UpdateCarAI(
            &mut car.vehicle.core.auto_pilot,
            ai_input,
            transmission_max_velocity,
            &mut general,
        );
        // Apply the intents the C++ core performs through entity pointers.
        if ai_result.promote_to_physics {
            car.vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
        }
        if ai_result.switch_to_real_physics {
            car.vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
        }
        if ai_result.join_road_system {
            join_car_with_road_system(&mut file_loader.path_find, &mut car.vehicle, &mut general);
        }
        if ai_result.join_goto_target {
            let mission_after_update = car.vehicle.core.auto_pilot.car_mission;
            let target = match mission_after_update {
                eCarMission::MISSION_RAMPLAYER_FARAWAY
                | eCarMission::MISSION_BLOCKPLAYER_FARAWAY => target_player_position,
                _ => car.vehicle.core.auto_pilot.destination_coors,
            };
            let straight = join_car_with_road_system_goto_coors(
                &mut file_loader.path_find,
                &mut car.vehicle,
                target,
                &mut general,
            );
            car.vehicle.core.auto_pilot.car_mission = match mission_after_update {
                eCarMission::MISSION_RAMPLAYER_FARAWAY if straight => {
                    eCarMission::MISSION_RAMPLAYER_CLOSE
                }
                eCarMission::MISSION_BLOCKPLAYER_FARAWAY if straight => {
                    eCarMission::MISSION_BLOCKPLAYER_CLOSE
                }
                eCarMission::MISSION_GOTOCOORDS if straight => {
                    eCarMission::MISSION_GOTOCOORDS_STRAIGHT
                }
                eCarMission::MISSION_GOTOCOORDS_ACCURATE if straight => {
                    eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE
                }
                _ => mission_after_update,
            };
        }
        if ai_result.occupants_leave {
            occupants_to_leave.push((handle, self_slot));
        }
        if ai_result.siren == Some(true) {
            car.vehicle.audio.siren_or_alarm = true;
        } else if ai_result.siren == Some(false) {
            car.vehicle.audio.siren_or_alarm = false;
        }
        if let Some(horn_timer) = ai_result.horn_timer {
            car.vehicle.audio.car_horn_timer = horn_timer.min(u32::from(u8::MAX)) as u8;
        }
        car.vehicle.core.time_blocked = u16::try_from(ai_result.time_blocked_ms).unwrap_or(u16::MAX);

        // Still simple? UpdateCarAI may have switched us to physics.
        if car.vehicle.core.physical.entity.status != eEntityStatus::STATUS_SIMPLE {
            continue;
        }

        // ---- CPhysical::ProcessControl simple early-out
        // (entities/Physical.cpp:353-364).
        {
            let physical = &mut car.vehicle.core.physical;
            physical.flags.is_in_water = false;
            physical.entity.flags.has_contacted = false;
            physical.entity.flags.is_in_safe_position = false;
            physical.entity.flags.was_postponed = false;
            physical.entity.flags.has_hit_wall = false;
        }

        // ---- CCarCtrl::UpdateCarOnRails (control/CarCtrl.cpp:806-854).
        rails_update_car_on_rails(
            &mut car,
            &mut file_loader,
            self_slot,
            &vehicle_scans,
            &ped_scans,
            traffic_index,
            player_position,
            player_vehicle_slot,
            bbox_min,
            bbox_max,
            time_ms,
            time_step,
            &mut general,
        );

        // Still on rails? A U-turn pick or the standing-car annoyance can
        // hand the car to real physics.
        if car.vehicle.core.physical.entity.status != eEntityStatus::STATUS_SIMPLE {
            continue;
        }

        // m_nWheelsOnGround = 4 (Automobile.cpp:379-381) is implicit in the
        // rails step; the gear and wheel-rotation parts remain.
        let max_traffic_speed = car.vehicle.core.auto_pilot.max_traffic_speed;
        if let Some(handling) = handling_manager.get_handling_data_mut(handling_index) {
            handling.transmission.calculate_gear_for_simple_car(
                max_traffic_speed / 50.0,
                &mut car.vehicle.drive.current_gear,
            );
        }
        let forward = car.vehicle.core.physical.entity.get_forward();
        let move_speed = car.vehicle.core.physical.move_speed;
        let wheel_rot = CVehicle::process_wheel_rotation(
            crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL,
            forward,
            move_speed,
            0.35,
            time_step,
        );
        for rotation in &mut car.wheel_rotation {
            *rotation += wheel_rot;
        }
        car.vehicle.audio.reduce_horn_counter();
        car.vehicle.core.flags.vehicle_col_processed = false;
    }

    for (vehicle_handle, vehicle_slot) in occupants_to_leave.iter().copied() {
        occupant_handles.clear();
        if let Some(vehicle_entity) = script_entities.vehicle_entity(vehicle_handle) {
            occupant_handles.extend(
                script_entities
                    .script_occupant_handles(vehicle_entity)
                    .map(|(ped_handle, _)| ped_handle),
            );
        }
        for occupant_handle in occupant_handles.iter().copied() {
            if let Some(entity) = script_entities.ped_get_mut(occupant_handle) {
                entity.ped.flags.script_objective_completed = false;
                entity.ped.set_objective_entity(eObjective::OBJECTIVE_LEAVE_CAR,
                    vehicle_slot,
                    time_ms,
                    Some(vehicle_slot),
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PhysicsAiVehicleSnapshot {
    position: CVector,
    move_speed: CVector,
    up_z: f32,
    collided: bool,
}

#[derive(Default)]
pub(crate) struct PhysicsAutomobileAiScratch {
    handles: Vec<i32>,
    vehicles_by_slot: HashMap<usize, PhysicsAiVehicleSnapshot>,
    occupants_to_leave: Vec<(i32, usize)>,
    occupant_handles: Vec<i32>,
    driver_kill_player_any_means: HashSet<i32>,
}

pub(crate) use super::SirenTraffic::process_siren_traffic_response;

#[derive(Debug, Clone, Copy, Default)]
struct PhysicsAiControls {
    steer: f32,
    accel: f32,
    brake: f32,
    handbrake: bool,
}

fn physics_ai_speed_multiplier(angle_change: f32) -> f32 {
    const MIN_ANGLE: f32 = 0.4;
    const MAX_ANGLE: f32 = 1.2;
    const MIN_COEF: f32 = 0.4;
    let angle = crate::control::CarRails::limit_radian_angle(angle_change).abs();
    let n = (angle - MIN_ANGLE).max(0.0);
    let d = MAX_ANGLE - MIN_ANGLE;
    if n > d {
        MIN_COEF
    } else {
        1.0 - n / d * (1.0 - MIN_COEF)
    }
}

fn physics_ai_heading_controls<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    target: CVector,
    max_steer: f32,
) -> PhysicsAiControls
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let position = car.vehicle.core.physical.entity.get_position();
    let forward = car.vehicle.core.physical.entity.get_forward();
    let angle_to_target = CGeneral::get_atan_of_xy(target.x - position.x, target.y - position.y);
    let angle_forward = CGeneral::get_atan_of_xy(forward.x, forward.y);
    let raw_steer = crate::control::CarRails::limit_radian_angle(angle_to_target - angle_forward);
    let steer = raw_steer.clamp(-max_steer, max_steer);
    let current_speed = car.vehicle.core.physical.move_speed.magnitude() * GAME_SPEED_TO_CARAI_SPEED;
    let speed_target =
        f32::from(car.vehicle.core.auto_pilot.cruise_speed) * physics_ai_speed_multiplier(raw_steer);
    let speed_diff = speed_target - current_speed;
    let (accel, brake) = if speed_diff <= 0.0 {
        (0.0, (-speed_diff * 0.05).min(0.5))
    } else if current_speed < 25.0 {
        ((speed_diff * 0.1).min(1.0), 0.0)
    } else {
        (1.0, 0.0)
    };
    PhysicsAiControls {
        steer,
        accel,
        brake,
        handbrake: car.vehicle.core.physical.move_speed.magnitude() > 0.3 && raw_steer.abs() > 0.7,
    }
}

fn physics_ai_path_target<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &mut CFileLoader,
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    player_position: CVector,
    target_car_position: Option<CVector>,
    general: &mut CGeneral,
) -> Option<CVector>
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let position = car.vehicle.core.physical.entity.get_position();
    let current_index = car.vehicle.core.auto_pilot.current_path_node_info as usize;
    let mut current_link = file_loader
        .path_find
        .car_path_links
        .get(current_index)
        .copied()?;
    let mut current_forward = CVector::new(
        current_link.dir[0] * f32::from(car.vehicle.core.auto_pilot.current_direction),
        current_link.dir[1] * f32::from(car.vehicle.core.auto_pilot.current_direction),
        0.0,
    );
    let mut lane =
        f32::from(car.vehicle.core.auto_pilot.current_lane) + one_way_lane_offset(&current_link);
    let mut target = CVector::new(
        current_link.pos[0] + lane * LANE_WIDTH * current_forward.y,
        current_link.pos[1] - lane * LANE_WIDTH * current_forward.x,
        position.z,
    );
    let dx = target.x - position.x;
    let dy = target.y - position.y;
    if dx * dx + dy * dy < 5.0 * 5.0 {
        let result = pick_next_node_according_to_strategy(
            &mut file_loader.path_find,
            &mut car.vehicle.core.auto_pilot,
            position,
            player_position,
            target_car_position,
            general,
        );
        if result.restart_path_search {
            car.vehicle.core.auto_pilot.car_mission = match car.vehicle.core.auto_pilot.car_mission {
                eCarMission::MISSION_GOTOCOORDS => eCarMission::MISSION_GOTOCOORDS_STRAIGHT,
                eCarMission::MISSION_GOTOCOORDS_ACCURATE => {
                    eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE
                }
                mission => mission,
            };
            return Some(car.vehicle.core.auto_pilot.destination_coors);
        }
        current_link = *file_loader
            .path_find
            .car_path_links
            .get(car.vehicle.core.auto_pilot.current_path_node_info as usize)?;
        current_forward = CVector::new(
            current_link.dir[0] * f32::from(car.vehicle.core.auto_pilot.current_direction),
            current_link.dir[1] * f32::from(car.vehicle.core.auto_pilot.current_direction),
            0.0,
        );
        lane = f32::from(car.vehicle.core.auto_pilot.current_lane) + one_way_lane_offset(&current_link);
        target = CVector::new(
            current_link.pos[0] + lane * LANE_WIDTH * current_forward.y,
            current_link.pos[1] - lane * LANE_WIDTH * current_forward.x,
            position.z,
        );
    }
    Some(target)
}

fn physics_ai_controls_for_mission<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &mut CFileLoader,
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    now_ms: u32,
    player_position: CVector,
    player_speed: CVector,
    target_car: Option<PhysicsAiVehicleSnapshot>,
    general: &mut CGeneral,
) -> PhysicsAiControls
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let temp = car.vehicle.core.auto_pilot.temp_action;
    if temp != eCarTempAction::TEMPACT_NONE {
        let expired = now_ms > car.vehicle.core.auto_pilot.time_temp_action;
        let controls = match temp {
            eCarTempAction::TEMPACT_WAIT => PhysicsAiControls {
                brake: 0.2,
                ..Default::default()
            },
            eCarTempAction::TEMPACT_REVERSE => {
                let target = physics_ai_path_target(
                    file_loader,
                    car,
                    player_position,
                    target_car.map(|target| target.position),
                    general,
                )
                .unwrap_or(car.vehicle.core.auto_pilot.destination_coors);
                let mut controls = physics_ai_heading_controls(car, target, 0.5);
                controls.steer = -controls.steer;
                controls.handbrake = false;
                if car
                    .vehicle.core.physical
                    .move_speed
                    .dot(car.vehicle.core.physical.entity.get_forward())
                    > 0.04
                {
                    controls.accel = 0.0;
                    controls.brake = 0.5;
                } else {
                    controls.accel = -0.5;
                    controls.brake = 0.0;
                }
                controls
            }
            eCarTempAction::TEMPACT_HANDBRAKETURNLEFT => PhysicsAiControls {
                steer: -1.0,
                handbrake: true,
                ..Default::default()
            },
            eCarTempAction::TEMPACT_HANDBRAKETURNRIGHT => PhysicsAiControls {
                steer: 1.0,
                handbrake: true,
                ..Default::default()
            },
            eCarTempAction::TEMPACT_HANDBRAKESTRAIGHT => PhysicsAiControls {
                handbrake: true,
                ..Default::default()
            },
            eCarTempAction::TEMPACT_TURNLEFT => PhysicsAiControls {
                steer: 1.0,
                accel: 1.0,
                ..Default::default()
            },
            eCarTempAction::TEMPACT_TURNRIGHT => PhysicsAiControls {
                steer: -1.0,
                accel: 1.0,
                ..Default::default()
            },
            eCarTempAction::TEMPACT_GOFORWARD => PhysicsAiControls {
                accel: 0.5,
                ..Default::default()
            },
            eCarTempAction::TEMPACT_SWERVELEFT | eCarTempAction::TEMPACT_SWERVERIGHT => {
                let mut steer = if temp == eCarTempAction::TEMPACT_SWERVERIGHT {
                    0.15
                } else {
                    -0.15
                };
                if now_ms
                    > car
                        .vehicle.core.auto_pilot
                        .time_temp_action
                        .saturating_sub(1_000)
                {
                    steer = -steer;
                }
                PhysicsAiControls {
                    steer,
                    brake: 0.001,
                    ..Default::default()
                }
            }
            eCarTempAction::TEMPACT_NONE => Default::default(),
        };
        if expired {
            car.vehicle.core.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
            car.vehicle.core.auto_pilot.anti_reverse_timer = now_ms;
            car.vehicle.core.auto_pilot.time_temp_action = now_ms;
        }
        return controls;
    }

    let max_steer = if car.vehicle.core.physical.entity.model_index.max(0) as usize
        == crate::modelinfo::ModelIndices::MI_ENFORCER
    {
        0.7
    } else {
        0.5
    };
    match car.vehicle.core.auto_pilot.car_mission {
        eCarMission::MISSION_NONE
        | eCarMission::MISSION_EMERGENCYVEHICLE_STOP
        | eCarMission::MISSION_STOP_FOREVER
        | eCarMission::MISSION_WAITFORDELETION => PhysicsAiControls {
            brake: 0.5,
            handbrake: true,
            ..Default::default()
        },
        eCarMission::MISSION_GOTOCOORDS_STRAIGHT
        | eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE => {
            physics_ai_heading_controls(car, car.vehicle.core.auto_pilot.destination_coors, max_steer)
        }
        eCarMission::MISSION_RAMPLAYER_CLOSE => {
            physics_ai_heading_controls(car, player_position, max_steer)
        }
        eCarMission::MISSION_BLOCKPLAYER_CLOSE => {
            let mut offset = CVector::new(player_speed.x, player_speed.y, 0.0);
            let len = (offset.x * offset.x + offset.y * offset.y).sqrt();
            if len > 0.13 {
                let scale = 0.13 / len;
                offset.x *= scale;
                offset.y *= scale;
            }
            let target = CVector::new(
                player_position.x + offset.x * GAME_SPEED_TO_CARAI_SPEED,
                player_position.y + offset.y * GAME_SPEED_TO_CARAI_SPEED,
                player_position.z,
            );
            let controls = physics_ai_heading_controls(car, target, max_steer);
            let position = car.vehicle.core.physical.entity.get_position();
            let dx = target.x - position.x;
            let dy = target.y - position.y;
            if dx * dx + dy * dy < 25.0 {
                car.vehicle.core.auto_pilot.car_mission = eCarMission::MISSION_BLOCKPLAYER_HANDBRAKESTOP;
            }
            controls
        }
        eCarMission::MISSION_BLOCKPLAYER_HANDBRAKESTOP => {
            let position = car.vehicle.core.physical.entity.get_position();
            let dx = player_position.x - position.x;
            let dy = player_position.y - position.y;
            if dx * dx + dy * dy > 100.0 {
                car.vehicle.core.auto_pilot.car_mission = eCarMission::MISSION_BLOCKPLAYER_CLOSE;
            }
            PhysicsAiControls {
                brake: 1.0,
                handbrake: true,
                ..Default::default()
            }
        }
        eCarMission::MISSION_RAMCAR_CLOSE => target_car.map_or_else(
            || PhysicsAiControls {
                brake: 0.5,
                ..Default::default()
            },
            |target| physics_ai_heading_controls(car, target.position, max_steer),
        ),
        eCarMission::MISSION_BLOCKCAR_CLOSE => target_car.map_or_else(
            || PhysicsAiControls {
                brake: 0.5,
                ..Default::default()
            },
            |target_car| {
                let mut offset =
                    CVector::new(target_car.move_speed.x, target_car.move_speed.y, 0.0);
                let len = (offset.x * offset.x + offset.y * offset.y).sqrt();
                if len > 0.13 {
                    let scale = 0.13 / len;
                    offset.x *= scale;
                    offset.y *= scale;
                }
                let target = CVector::new(
                    target_car.position.x + offset.x * GAME_SPEED_TO_CARAI_SPEED,
                    target_car.position.y + offset.y * GAME_SPEED_TO_CARAI_SPEED,
                    target_car.position.z,
                );
                let controls = physics_ai_heading_controls(car, target, max_steer);
                let position = car.vehicle.core.physical.entity.get_position();
                let dx = target.x - position.x;
                let dy = target.y - position.y;
                if dx * dx + dy * dy < 25.0 {
                    car.vehicle.core.auto_pilot.car_mission =
                        eCarMission::MISSION_BLOCKCAR_HANDBRAKESTOP;
                }
                controls
            },
        ),
        eCarMission::MISSION_BLOCKCAR_HANDBRAKESTOP => {
            if let Some(target) = target_car {
                let position = car.vehicle.core.physical.entity.get_position();
                let dx = target.position.x - position.x;
                let dy = target.position.y - position.y;
                if dx * dx + dy * dy > 100.0 {
                    car.vehicle.core.auto_pilot.car_mission = eCarMission::MISSION_BLOCKCAR_CLOSE;
                }
            }
            PhysicsAiControls {
                brake: 1.0,
                handbrake: true,
                ..Default::default()
            }
        }
        eCarMission::MISSION_CRUISE
        | eCarMission::MISSION_RAMPLAYER_FARAWAY
        | eCarMission::MISSION_BLOCKPLAYER_FARAWAY
        | eCarMission::MISSION_GOTOCOORDS
        | eCarMission::MISSION_GOTOCOORDS_ACCURATE
        | eCarMission::MISSION_RAMCAR_FARAWAY
        | eCarMission::MISSION_BLOCKCAR_FARAWAY => {
            let target = physics_ai_path_target(
                file_loader,
                car,
                player_position,
                target_car.map(|target| target.position),
                general,
            )
            .unwrap_or(car.vehicle.core.auto_pilot.destination_coors);
            physics_ai_heading_controls(car, target, max_steer)
        }
        eCarMission::MISSION_GOTOCOORDS_ASTHECROWSWIMS => {
            physics_ai_heading_controls(car, car.vehicle.core.auto_pilot.destination_coors, max_steer)
        }
    }
}

/// Physics-mode half of re3 `CAutomobile::ProcessControl`:
/// `CCarAI::UpdateCarAI -> CCarCtrl::SteerAICarWithPhysics`.
///
/// Rail traffic already runs through `process_simple_automobile_control`.
/// This system owns only `STATUS_PHYSICS` AI cars, which is the state mission
/// cars and police pursuits enter when they need real wheel/collision physics.
pub(crate) fn process_physics_automobile_ai_control(
    mut script_entities: ScriptEntityWorld,
    mut file_loader: ResMut<CFileLoader>,
    collision: Res<CCollision>,
    handling_manager: Res<cHandlingDataMgr>,
    world_players: CWorldPlayersRead,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut scratch: Local<PhysicsAutomobileAiScratch>,
) {
    let Some(player) = world_players.focused() else {
        bevy::log::warn_once!(target: "re3bevy::physics_vehicle_ai",
            "required materialized player input is absent");
        return;
    };
    let player_position = player.position();
    let player_move_speed = player.ped.physical.move_speed;
    let player_forward = player.ped.physical.entity.get_forward();
    let wanted_level = player.wanted.get_wanted_level();
    let ignored_by_everyone = player.wanted.m_bIgnoredByEveryone;
    let ignored_by_cops = player.wanted.m_bIgnoredByCops;
    let focus = world_players.player_in_focus() as i32;
    let player_vehicle_handle = world_players
        .relation(focus)
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle));

    scratch.handles.clear();
    scratch.vehicles_by_slot.clear();
    scratch.occupants_to_leave.clear();
    scratch.occupant_handles.clear();
    scratch.driver_kill_player_any_means.clear();
    for (ped_handle, relation) in script_entities.script_relations() {
        if !relation.is_driver() {
            continue;
        }
        if script_entities.peds.get(&script_entities.index, &ped_handle)
            .is_some_and(|ped| ped.ped.objective == eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS)
            && let Some(vehicle_handle) =
                script_entities.vehicle_handle_for_entity(relation.vehicle)
        {
            scratch.driver_kill_player_any_means.insert(vehicle_handle);
        }
    }
    for (&handle, vehicle) in script_entities.vehicles.iter() {
        let base = vehicle.base();
        let slot = CPoolHandle(handle).index();
        scratch.vehicles_by_slot.insert(
            slot,
            PhysicsAiVehicleSnapshot {
                position: base.physical.entity.get_position(),
                move_speed: base.physical.move_speed,
                up_z: base.physical.entity.get_up().z,
                collided: base.physical.entity.flags.has_collided,
            },
        );
        if base.physical.entity.status == eEntityStatus::STATUS_PHYSICS
            && (base.physical.zone_level <= crate::collision::ColModel::LEVEL_GENERIC as i8
                || base.physical.zone_level == collision.collision_in_memory as i8)
            && (base.auto_pilot.car_mission != eCarMission::MISSION_NONE
                || base.flags.is_law_enforcer)
        {
            scratch.handles.push(handle);
        }
    }
    scratch
        .handles
        .sort_by_key(|handle| CPoolHandle(*handle).index());

    let player_vehicle_slot = player_vehicle_handle.map(|handle| CPoolHandle(handle).index());
    let player_vehicle =
        player_vehicle_slot.and_then(|slot| scratch.vehicles_by_slot.get(&slot).copied());
    let target_player_position = player_vehicle.map_or(player_position, |vehicle| vehicle.position);
    let target_player_speed =
        player_vehicle.map_or(player_move_speed, |vehicle| vehicle.move_speed);
    let now_ms = timer.time_in_milliseconds();
    let previous_ms = now_ms.wrapping_sub((timer.time_step_in_seconds().max(0.0) * 1_000.0) as u32);

    let PhysicsAutomobileAiScratch {
        handles,
        vehicles_by_slot,
        occupants_to_leave,
        occupant_handles,
        driver_kill_player_any_means,
    } = &mut *scratch;

    for handle in handles.iter().copied() {
        let self_slot = CPoolHandle(handle).index();
        let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
            script_entities.vehicle_get_mut(handle)
        else {
            continue;
        };
        if car.vehicle.core.physical.entity.status != eEntityStatus::STATUS_PHYSICS {
            continue;
        }
        car.vehicle.core.auto_pilot.slowed_down_because_of_cars = false;
        car.vehicle.core.auto_pilot.slowed_down_because_of_peds = false;
        let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
        let handling_id = car.vehicle.core.handling_id.or_else(|| {
            match file_loader.model_info.get_model_info(model_id) {
                Some(ModelInfoSlot::Vehicle(model)) => {
                    cHandlingDataMgr::get_handling_id(&model.handling_id)
                        .and_then(|id| i32::try_from(id).ok())
                }
                _ => None,
            }
        });
        let transmission_max_velocity = handling_id
            .and_then(|id| usize::try_from(id).ok())
            .and_then(|id| handling_manager.get_handling_data(id))
            .map(|handling| handling.transmission.max_velocity)
            .unwrap_or_default();
        let target_car = car
            .vehicle.core.auto_pilot
            .target_car
            .and_then(|slot| vehicles_by_slot.get(&slot).copied());
        let input = CarAiStepInput {
            status: car.vehicle.core.physical.entity.status,
            position: car.vehicle.core.physical.entity.get_position(),
            move_speed: car.vehicle.core.physical.move_speed,
            forward: car.vehicle.core.physical.entity.get_forward(),
            up_z: car.vehicle.core.physical.entity.get_up().z,
            is_law_enforcer: car.vehicle.core.flags.is_law_enforcer,
            uses_siren: crate::vehicles::Vehicle::vehicle_uses_siren(model_id),
            siren_or_alarm: car.vehicle.audio.siren_or_alarm,
            is_rhino: model_id == crate::modelinfo::ModelIndices::MI_RHINO,
            is_ambulance_on_duty: car.vehicle.core.flags.is_ambulance_on_duty,
            is_fire_truck_on_duty: car.vehicle.core.flags.is_fire_truck_on_duty,
            created_by: car.vehicle.core.vehicle_created_by,
            random_seed: car.vehicle.core.physical.entity.random_seed,
            last_collision_ms: car.vehicle.core.physical.last_time_collided,
            time_blocked_ms: u32::from(car.vehicle.core.time_blocked),
            now_ms,
            previous_ms,
            time_step_ms: timer.time_step().max(0.0),
            frame_counter: timer.frame_counter(),
            player_position: target_player_position,
            player_vehicle_speed: player_vehicle.map(|vehicle| vehicle.move_speed.magnitude()),
            player_move_speed: target_player_speed,
            player_forward,
            player_vehicle_upside_down: player_vehicle.is_some_and(|vehicle| vehicle.up_z < 0.0),
            player_vehicle_collided: player_vehicle.is_some_and(|vehicle| vehicle.collided),
            driver_kill_player_any_means: driver_kill_player_any_means.contains(&handle),
            player_ignored_by_everyone: ignored_by_everyone,
            player_ignored_by_cops: ignored_by_cops,
            wanted_level: wanted_level.clamp(0, u8::MAX as i32) as u8,
            no_police_zone: file_loader.world.cull_zones.player_no_police(),
            // TODO(parity): CTrafficLights::ShouldCarStopForBridge admission.
            bridge_requires_stop: false,
            target_car_position: target_car.map(|target| target.position),
            target_car_is_player_vehicle: car
                .vehicle.core.auto_pilot
                .target_car
                .is_some_and(|slot| Some(slot) == player_vehicle_slot),
            target_car_collided: target_car.is_some_and(|target| target.collided),
        };
        let ai_result = CCarAI::UpdateCarAI(
            &mut car.vehicle.core.auto_pilot,
            input,
            transmission_max_velocity,
            &mut general,
        );
        if ai_result.join_road_system {
            join_car_with_road_system(&mut file_loader.path_find, &mut car.vehicle, &mut general);
        }
        if ai_result.join_goto_target {
            let mission_after_update = car.vehicle.core.auto_pilot.car_mission;
            let target = match mission_after_update {
                eCarMission::MISSION_RAMPLAYER_FARAWAY
                | eCarMission::MISSION_BLOCKPLAYER_FARAWAY => target_player_position,
                _ => car.vehicle.core.auto_pilot.destination_coors,
            };
            let straight = join_car_with_road_system_goto_coors(
                &mut file_loader.path_find,
                &mut car.vehicle,
                target,
                &mut general,
            );
            car.vehicle.core.auto_pilot.car_mission = match mission_after_update {
                eCarMission::MISSION_RAMPLAYER_FARAWAY if straight => {
                    eCarMission::MISSION_RAMPLAYER_CLOSE
                }
                eCarMission::MISSION_BLOCKPLAYER_FARAWAY if straight => {
                    eCarMission::MISSION_BLOCKPLAYER_CLOSE
                }
                eCarMission::MISSION_GOTOCOORDS if straight => {
                    eCarMission::MISSION_GOTOCOORDS_STRAIGHT
                }
                eCarMission::MISSION_GOTOCOORDS_ACCURATE if straight => {
                    eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE
                }
                _ => mission_after_update,
            };
        }
        if ai_result.occupants_leave {
            occupants_to_leave.push((handle, self_slot));
        }
        if let Some(siren) = ai_result.siren {
            car.vehicle.audio.siren_or_alarm = siren;
        }
        if let Some(horn_timer) = ai_result.horn_timer {
            car.vehicle.audio.car_horn_timer = horn_timer.min(u32::from(u8::MAX)) as u8;
        }
        car.vehicle.core.time_blocked = u16::try_from(ai_result.time_blocked_ms).unwrap_or(u16::MAX);

        let controls = physics_ai_controls_for_mission(
            &mut file_loader,
            &mut car,
            now_ms,
            target_player_position,
            target_player_speed,
            target_car,
            &mut general,
        );
        car.vehicle.drive.steer_angle = controls.steer;
        car.vehicle.drive.steer_input = if controls.steer == 0.0 {
            0.0
        } else {
            controls.steer.signum() * (controls.steer.abs() / 0.7).sqrt().clamp(0.0, 1.0)
        };
        car.vehicle.drive.gas_pedal = controls.accel.clamp(-1.0, 1.0);
        car.vehicle.drive.brake_pedal = controls.brake.clamp(0.0, 1.0);
        car.vehicle.drive.is_handbrake_on = controls.handbrake;
    }

    for (vehicle_handle, vehicle_slot) in occupants_to_leave.iter().copied() {
        occupant_handles.clear();
        if let Some(vehicle_entity) = script_entities.vehicle_entity(vehicle_handle) {
            occupant_handles.extend(
                script_entities
                    .script_occupant_handles(vehicle_entity)
                    .map(|(ped_handle, _)| ped_handle),
            );
        }
        for occupant_handle in occupant_handles.iter().copied() {
            if let Some(entity) = script_entities.ped_get_mut(occupant_handle) {
                entity.ped.flags.script_objective_completed = false;
                entity.ped.set_objective_entity(eObjective::OBJECTIVE_LEAVE_CAR,
                    vehicle_slot,
                    now_ms,
                    Some(vehicle_slot),
                );
            }
        }
    }
}

#[cfg(test)]
mod vehicle_interest_tests {
    use super::*;

    #[test]
    fn recent_two_cars_survive_interest_eviction_and_reentry_refreshes_age() {
        let mut world = World::new();
        world.insert_resource(CTimer::initialise());
        let cars = [
            world.spawn_empty().id(),
            world.spawn_empty().id(),
            world.spawn_empty().id(),
        ];
        for (time, car) in [(10, cars[0]), (20, cars[1]), (30, cars[0]), (40, cars[2])] {
            world
                .resource_mut::<CTimer>()
                .set_time_in_milliseconds(time);
            queue_register_vehicle_of_interest(&mut world.commands(), car);
            world.flush();
        }
        assert!(world.get::<VehicleOfInterest>(cars[0]).is_some());
        assert!(world.get::<VehicleOfInterest>(cars[1]).is_none());
        assert!(world.get::<VehicleOfInterest>(cars[2]).is_some());
        world.despawn(cars[0]);
        let replacement = world.spawn_empty().id();
        queue_register_vehicle_of_interest(&mut world.commands(), replacement);
        world.flush();
        assert_eq!(world.get::<VehicleOfInterest>(replacement).unwrap().slot, 0);
    }
}
