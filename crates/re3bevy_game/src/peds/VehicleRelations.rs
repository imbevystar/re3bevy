//! ECS relationships between peds and vehicles.
//!
//! The relationship is intentionally stored only on the occupant.  A vehicle
//! does not keep a second ped-slot index; systems that need its occupants query
//! `InVehicle`.  This prevents the two integer links that existed in the
//! original compatibility structs from becoming two competing authorities.

use bevy::{
    ecs::{query::QueryFilter, schedule::IntoScheduleConfigs},
    prelude::*,
};
use re3bevy_core::core::Schedules::GtaProcessSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct PedEntity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct VehicleEntity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleSeat {
    Driver,
    Passenger(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct InVehicle {
    pub vehicle: Entity,
    pub seat: VehicleSeat,
}

impl InVehicle {
    pub const fn driver(vehicle: Entity) -> Self {
        Self {
            vehicle,
            seat: VehicleSeat::Driver,
        }
    }

    pub const fn passenger(vehicle: Entity, seat: u8) -> Self {
        Self {
            vehicle,
            seat: VehicleSeat::Passenger(seat),
        }
    }

    pub const fn is_driver(self) -> bool {
        matches!(self.seat, VehicleSeat::Driver)
    }
}

/// Queue a canonical occupant relation while preserving the one-ped-per-seat
/// invariant.  Callers may keep same-pass transient overlays for deferred
/// command visibility, but the only persisted relationship is `InVehicle`.
pub(crate) fn queue_set_vehicle_relation<F: QueryFilter>(
    commands: &mut Commands,
    relations: &Query<(Entity, Option<&InVehicle>), F>,
    ped: Entity,
    relation: InVehicle,
) -> bool {
    for (other_ped, current) in relations.iter() {
        let Some(current) = current else {
            continue;
        };
        if other_ped != ped && current.vehicle == relation.vehicle && current.seat == relation.seat
        {
            return false;
        }
    }
    commands.entity(ped).insert(relation);
    true
}

/// Queue a one-seat handoff for the narrow re3 `CVehicle::SetDriver` case
/// where a carjacker becomes driver while the previous driver is still in the
/// dragged-out animation. The replaced occupant must currently own this exact
/// seat; every other occupant is still checked against the canonical
/// one-ped-per-seat invariant.
pub(crate) fn queue_replace_vehicle_relation<F: QueryFilter>(
    commands: &mut Commands,
    relations: &Query<(Entity, Option<&InVehicle>), F>,
    ped: Entity,
    relation: InVehicle,
    replaced_ped: Entity,
) -> bool {
    if ped == replaced_ped
        || relations
            .get(replaced_ped)
            .ok()
            .and_then(|(_, current)| current.copied())
            != Some(relation)
    {
        return false;
    }
    for (other_ped, current) in relations.iter() {
        let Some(current) = current else {
            continue;
        };
        if other_ped != ped
            && other_ped != replaced_ped
            && current.vehicle == relation.vehicle
            && current.seat == relation.seat
        {
            return false;
        }
    }
    commands.entity(replaced_ped).remove::<InVehicle>();
    commands.entity(ped).insert(relation);
    true
}

pub(crate) fn queue_clear_vehicle_relation(commands: &mut Commands, ped: Entity) {
    commands.entity(ped).remove::<InVehicle>();
}

pub struct VehicleRelationsPlugin;

impl Plugin for VehicleRelationsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(crate::peds::PedVehicleRetirement::retire_ped_vehicle_reservations);
        app.add_systems(
            bevy::app::Update,
            cleanup_dangling_vehicle_relations.in_set(GtaProcessSet::GameLogic),
        );
    }
}

fn cleanup_dangling_vehicle_relations(
    mut commands: Commands,
    vehicles: Query<(), With<VehicleEntity>>,
    relations: Query<(Entity, &InVehicle), With<PedEntity>>,
) {
    // Despawning a vehicle must not leave a relation pointing at an entity
    // that can never be queried again.  This is derived cleanup; the
    // relation remains the sole authoritative occupant state.
    for (ped, relation) in &relations {
        if vehicles.get(relation.vehicle).is_err() {
            commands.entity(ped).remove::<InVehicle>();
        }
    }
}

/// Keep the gameplay-side CPed transform on the same authored seat matrix the
/// renderer uses. re3 calls SetPedPositionInCar continuously while a ped is
/// seated; without this ECS equivalent only the rendered mesh followed the car
/// while SCM queries, AI and later exit placement still saw the old spawn
/// coordinate.
pub(crate) fn sync_script_vehicle_occupants(
    active_animations: Query<Entity, With<CPedVehicleAnimation>>,
    file_loader: Res<CFileLoader>,
    mut script_entities: ScriptEntityWorld,
) {
    let active = active_animations.iter().collect::<HashSet<_>>();
    let occupants = script_entities.script_relations().collect::<Vec<_>>();

    for (ped_handle, relation) in occupants {
        let Some(ped_entity) = script_entities.ped_entity(ped_handle) else {
            continue;
        };
        // re3 stops calling SetPedPositionInCar once SetDie/SetDead has
        // changed the ped state.  Keep the InVehicle relation (the original
        // does too), but do not keep dragging a corpse with the vehicle.
        if script_entities
            .ped_get(ped_handle)
            .is_some_and(|ped| ped.ped.dying_or_dead())
        {
            continue;
        }
        // Enter/exit associations own the ped root while they are active.
        if active.contains(&ped_entity) {
            continue;
        }
        let Some(vehicle_handle) = script_entities.vehicle_handle_for_entity(relation.vehicle)
        else {
            continue;
        };
        let Some((matrix, move_speed, turn_speed, zone_level)) = script_entities
            .vehicle_get(vehicle_handle)
            .and_then(|vehicle| {
                let base = vehicle.base();
                crate::peds::Ped::vehicle_occupant_matrix(&file_loader, &base, relation.seat).map(
                    |matrix| {
                        (
                            matrix,
                            base.physical.move_speed,
                            base.physical.turn_speed,
                            base.physical.zone_level,
                        )
                    },
                )
            })
        else {
            continue;
        };
        let Some(ped) = script_entities.ped_get_mut(ped_handle) else {
            continue;
        };
        ped.ped.physical.entity.set_matrix(matrix);
        ped.ped.physical.move_speed = move_speed;
        ped.ped.physical.turn_speed = turn_speed;
        ped.ped.physical.zone_level = zone_level;
        ped.ped.physical.entity.flags.uses_collision = false;
        let heading = matrix.right.y.atan2(matrix.right.x);
        ped.locomotion.rotation_cur = heading;
        ped.locomotion.rotation_dest = heading;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seat_relation_keeps_vehicle_and_seat_together() {
        let vehicle = Entity::from_raw_u32(42).expect("non-zero entity id");
        let relation = InVehicle::passenger(vehicle, 3);

        assert_eq!(relation.vehicle, vehicle);
        assert_eq!(relation.seat, VehicleSeat::Passenger(3));
        assert!(!relation.is_driver());
    }

    #[test]
    fn driver_relation_is_explicit() {
        let relation = InVehicle::driver(Entity::from_raw_u32(7).expect("non-zero entity id"));
        assert!(relation.is_driver());
    }

    #[test]
    fn assigning_an_occupied_seat_is_rejected() {
        let mut app = App::new();
        let vehicle = app.world_mut().spawn(VehicleEntity).id();
        app.world_mut()
            .spawn((PedEntity, InVehicle::passenger(vehicle, 0)));
        let second_ped = app.world_mut().spawn(PedEntity).id();

        app.add_systems(
            bevy::app::Update,
            move |mut commands: Commands,
                  relations: Query<(Entity, Option<&InVehicle>), With<PedEntity>>| {
                assert!(!queue_set_vehicle_relation(
                    &mut commands,
                    &relations,
                    second_ped,
                    InVehicle::passenger(vehicle, 0),
                ));
            },
        );

        app.update();

        assert!(app.world().get::<InVehicle>(second_ped).is_none());
    }

    #[test]
    fn driver_seat_can_be_handed_off_from_the_named_replaced_ped() {
        let mut app = App::new();
        let vehicle = app.world_mut().spawn(VehicleEntity).id();
        let old_driver = app
            .world_mut()
            .spawn((PedEntity, InVehicle::driver(vehicle)))
            .id();
        let new_driver = app.world_mut().spawn(PedEntity).id();

        app.add_systems(
            bevy::app::Update,
            move |mut commands: Commands,
                  relations: Query<(Entity, Option<&InVehicle>), With<PedEntity>>| {
                assert!(queue_replace_vehicle_relation(
                    &mut commands,
                    &relations,
                    new_driver,
                    InVehicle::driver(vehicle),
                    old_driver,
                ));
            },
        );

        app.update();

        assert!(app.world().get::<InVehicle>(old_driver).is_none());
        assert_eq!(
            app.world().get::<InVehicle>(new_driver),
            Some(&InVehicle::driver(vehicle))
        );
    }

    #[test]
    fn script_relation_view_includes_non_script_ped_occupants() {
        let mut world = crate::control::Script::script_entity_test_world();
        let vehicle = world.spawn(VehicleEntity).id();
        let player_side_ped = world
            .spawn((PedEntity, InVehicle::driver(vehicle)))
            .id();

        crate::control::Script::with_script_entity_test_world(&mut world, |entities| {
            let occupants = entities.relation_occupants(vehicle).collect::<Vec<_>>();
            assert_eq!(occupants, vec![(player_side_ped, VehicleSeat::Driver)]);
        });
    }

    #[test]
    fn relation_system_removes_vehicle_despawned_relations() {
        let mut app = App::new();
        app.add_plugins(VehicleRelationsPlugin);
        let vehicle = app.world_mut().spawn(VehicleEntity).id();
        let ped = app.world_mut().spawn(PedEntity).id();
        app.world_mut()
            .entity_mut(ped)
            .insert(InVehicle::passenger(vehicle, 0));
        app.world_mut().despawn(vehicle);

        app.update();

        assert!(app.world().get::<InVehicle>(ped).is_none());
    }

    #[test]
    fn ped_behind_car_seeks_nearest_rear_passenger_door() {
        use crate::control::ScriptEntities::ScriptPedEntity;
        use crate::peds::Ped::CPed;

        let mut app = App::new();
        app.init_resource::<crate::control::ScriptEntities::ScriptEntityIndex>();
        app.insert_resource(CAnimManager::default());
        app.add_systems(bevy::app::Update, process_ped_vehicle_objectives);

        let mut general = re3bevy_core::core::General::CGeneral::with_seed(1);
        let vehicle_handle = 1793;
        let vehicle_slot = CPoolHandle(vehicle_handle).index();
        let mut car = CAutomobile::new(90, 1, 0, &mut general, None, None, false);
        car.vehicle.core.physical
            .entity
            .set_position(CVector::new(0.0, 0.0, 0.0));
        car.vehicle.entry.num_max_passengers = 3;

        let ped_handle = 569;
        let mut ped = CPed::new(
            crate::peds::PedType::ePedType::PEDTYPE_CIVMALE,
            &mut general,
        );
        ped.physical
            .entity
            .set_position(CVector::new(0.0, -5.0, 0.0)); // behind car
        ped.objective = eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER;
        ped.car_in_objective = Some(vehicle_slot);
        ped.move_state = eMoveState::PEDMOVE_WALK;
        ped.flags.is_running = false;

        crate::control::ScriptEntities::spawn_script_test_vehicle(
            app.world_mut(),
            ScriptVehicleEntity::Automobile {
                handle: vehicle_handle,
                car,
            },
        );
        crate::control::ScriptEntities::spawn_script_test_ped(
            app.world_mut(),
            ScriptPedEntity {
                vitality: crate::peds::PedVitality::PedVitality::default(),
                locomotion: crate::peds::PedLocomotion::PedLocomotion::default(),
                animation: crate::peds::PedAnimation::PedAnimation::default(),
                handle: ped_handle,
                ped,
                model_id: 0,
            },
        );

        app.update();

        // Ped is ~5m away from car: must seek nearest door, NOT teleport to PED_ENTER_CAR
        let mut ped_query = app.world_mut().query::<&CPed>();
        let spawned_ped = ped_query.iter(app.world()).next().unwrap();
        assert_eq!(spawned_ped.ped_state, PedState::PED_SEEK_CAR);
        assert_eq!(spawned_ped.move_state, eMoveState::PEDMOVE_WALK);
        // Kuruma doors: door 3 (RF) is in front, doors 4/5 (LR/RR) are in the rear.
        // Because ped is at Y = -5.0, nearest door must be rear door (4 or 5), not 3!
        assert!(matches!(spawned_ped.veh_door, 4 | 5));
    }
}

// Ped-vehicle objectives, vehicle animations, door helpers and player
// enter/exit commits. Their schedule is owned by core/Game.rs.

#[allow(unused_imports)]
use crate::{
    animation::{
        AnimManager::CAnimManager,
        PedVehicleAnimation::{CPedVehicleAnimation, PedVehicleAnimationPhase},
    },
    audio::{DMAudio::cDMAudio, audio_enums::AUDIOTYPE_PHYSICAL, soundlist::*},
    control::Script::{CTheScripts, queue_ped_script_say},
    core::{
        FileLoader::CFileLoader,
        Game::{ScriptEntityWorld, ScriptVehicleEntity},
        Pad::{CPad, CPads},
        Pools::CPoolHandle,
    },
    entities::Entity::eEntityStatus,
    peds::{
        Ped::{
            CPedAnimationOffsets, MISSION_CHAR, PedState, RANDOM_CHAR,
            apply_ped_wait_state, apply_script_ped_wander_path, eMoveState, eObjective,
            set_script_ped_idle,
        },
        PedAI::{
            PedVehicleLineUpContext, automobile_ped_door_world_position, automobile_ped_line_up,
        },
        PedType::{CPedStatsRuntime, ePedType},
        PlayerPed::{CWorldPlayers, WorldPlayerAccess},
    },
    vehicles::{
        Automobile::{
            CAutomobile, automobile_door_missing, door_to_debris_component,
            vehicle_component_world_matrix,
        },
        HandlingMgr::cHandlingDataMgr,
        Vehicle::{MISSION_VEHICLE, eCarLock},
    },
};
#[allow(unused_imports)]
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

pub(crate) fn queue_ped_vehicle_animation(
    commands: &mut Commands,
    manager: &CAnimManager,
    ped: Entity,
    vehicle: Entity,
    door: usize,
    animation_id: i32,
    phase: PedVehicleAnimationPhase,
    enter_as_driver: bool,
) -> bool {
    let Some(active) = CPedVehicleAnimation::start_phase(
        manager,
        vehicle,
        door,
        animation_id,
        phase,
        enter_as_driver,
    ) else {
        warn!(
            target: "re3bevy::ped_vehicle",
            animation_id,
            ?phase,
            door,
            "vehicle animation association is unavailable"
        );
        return false;
    };
    commands.entity(ped).insert(active);
    true
}

/// Start real re3 vehicle-animation association chains from the already
/// ported SCM objectives. Warp opcodes intentionally bypass this system, just
/// as their C++ counterparts do.
#[derive(Default)]
pub(crate) struct PedVehicleObjectiveScratch {
    animated_ped_handles: HashSet<i32>,
    vehicle_profiles: HashMap<i32, PedAutomobileProfile>,
    occupied_driver_vehicles: HashSet<i32>,
    occupied_passenger_seats:
        HashMap<i32, [bool; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS]>,
    relation_vehicle_by_ped: HashMap<i32, i32>,
    vehicle_handle_by_slot: HashMap<usize, i32>,
    vehicle_can_enter: HashMap<i32, bool>,
    vehicle_passenger_doors: HashMap<i32, Vec<(usize, CVector)>>,
    vehicle_driver_doors: HashMap<i32, Vec<(usize, CVector)>>,
    starts: Vec<(i32, i32, usize, i32, PedVehicleAnimationPhase, bool)>,
}

pub(crate) fn process_ped_vehicle_objectives(
    active_animations: Query<Entity, With<CPedVehicleAnimation>>,
    manager: Res<CAnimManager>,
    mut script_entities: ScriptEntityWorld,
    file_loader: Option<Res<CFileLoader>>,
    handling_manager: Option<Res<cHandlingDataMgr>>,
    ped_animation_offsets: Option<Res<CPedAnimationOffsets>>,
    timer: Option<Res<CTimer>>,
    mut scratch: Local<PedVehicleObjectiveScratch>,
) {
    crate::profile_scope!("process_ped_vehicle_objectives");
    use crate::animation::AnimationId::{
        ANIM_STD_CAR_ALIGN_DOOR_LHS, ANIM_STD_CAR_ALIGN_DOOR_RHS, ANIM_STD_CAR_ALIGNHI_DOOR_LHS,
        ANIM_STD_CAR_ALIGNHI_DOOR_RHS,
    };
    scratch.animated_ped_handles.clear();
    scratch.vehicle_profiles.clear();
    scratch.occupied_driver_vehicles.clear();
    scratch.occupied_passenger_seats.clear();
    scratch.relation_vehicle_by_ped.clear();
    scratch.vehicle_handle_by_slot.clear();
    scratch.vehicle_can_enter.clear();
    scratch.vehicle_passenger_doors.clear();
    scratch.vehicle_driver_doors.clear();
    scratch.starts.clear();
    let now_ms = timer
        .as_deref()
        .map(CTimer::time_in_milliseconds)
        .unwrap_or(0);

    for entity in active_animations.iter() {
        if let Some(handle) = script_entities.ped_handle_for_entity(entity) {
            scratch.animated_ped_handles.insert(handle);
        }
    }
    for (handle, vehicle) in script_entities.vehicles.iter() {
        scratch
            .vehicle_handle_by_slot
            .insert(CPoolHandle(*handle).index(), *handle);
        scratch
            .vehicle_can_enter
            .insert(*handle, vehicle.base().can_ped_enter_car());
        if let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle {
            scratch
                .vehicle_profiles
                .insert(*handle, PedAutomobileProfile::from_car(&car));
        }
    }

    // Build ped -> vehicle plus global seat occupancy once for door selection.
    // `InVehicle` is canonical for both player and script peds, so a reclaiming
    // NPC must see a player already occupying the driver's seat.
    for (ped_entity, relation) in script_entities.relations.iter() {
        let Some(relation) = relation.copied() else {
            continue;
        };
        let Some(vehicle_handle) = script_entities.vehicle_handle_for_entity(relation.vehicle)
        else {
            continue;
        };
        if let Some(ped_handle) = script_entities.ped_handle_for_entity(ped_entity) {
            scratch
                .relation_vehicle_by_ped
                .insert(ped_handle, vehicle_handle);
        }
        match relation.seat {
            VehicleSeat::Driver => {
                scratch.occupied_driver_vehicles.insert(vehicle_handle);
            }
            VehicleSeat::Passenger(index) => {
                if let Some(slot) = scratch
                    .occupied_passenger_seats
                    .entry(vehicle_handle)
                    .or_default()
                    .get_mut(usize::from(index))
                {
                    *slot = true;
                }
            }
        }
    }

    // Determine door positions and availability for each vehicle
    for (handle, vehicle) in script_entities.vehicles.iter() {
        let base = vehicle.base();
        let has_rear_doors = file_loader.as_deref().map_or(true, |loader| {
            let model_index = base.physical.entity.model_index.max(0) as usize;
            let Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(model)) =
                loader.model_info.get_model_info(model_index)
            else {
                return true;
            };
            let Some(summary) = model.clump.base.clump_summary.as_ref() else {
                return true;
            };
            ["door_lr_dummy", "door_rr_dummy"]
                .into_iter()
                .all(|expected| {
                    summary.frames.iter().any(|frame| {
                        frame
                            .name
                            .as_deref()
                            .is_some_and(|name| name.eq_ignore_ascii_case(expected))
                    })
                })
        });
        let occupied = scratch
            .occupied_passenger_seats
            .get(handle)
            .copied()
            .unwrap_or_default();

        // re3 `CPed::SeekCar`/`GetNearestPassengerDoor` resolve the walk target
        // through `CPed::GetPositionToOpenCarDoor`, which is the authored seat
        // dummy plus handling seat offset minus the get-in animation offset
        // (`GetLocalPositionToOpenCarDoor`), not the DFF door component itself.
        let get_door_pos = |door: usize| -> CVector {
            if let Some(fl) = file_loader.as_deref()
                && let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle
                && let Some(offsets) = ped_animation_offsets.as_deref()
                && let Some(handling) = car
                    .vehicle.core.handling_id
                    .and_then(|id| usize::try_from(id).ok())
                    .and_then(|id| {
                        handling_manager
                            .as_deref()
                            .and_then(|mgr| mgr.get_handling_data(id))
                    })
                && let Some(pos) =
                    automobile_ped_door_world_position(fl, &car, handling, offsets, door, 1.0)
            {
                pos
            } else {
                let (lx, ly) = match door {
                    2 => (-1.0, 0.2),
                    3 => (1.0, 0.2),
                    4 => (-1.0, -0.8),
                    5 => (1.0, -0.8),
                    _ => (1.0, 0.0),
                };
                vehicle
                    .base()
                    .physical
                    .entity
                    .transform_point(CVector::new(lx, ly, 0.0))
            }
        };

        let driver_doors = if scratch.occupied_driver_vehicles.contains(handle) {
            Vec::new()
        } else {
            [2_usize, 3]
                .into_iter()
                .filter(|&door| {
                    base.entry.getting_in_flags & automobile_door_flag(door) == 0
                        && base.entry.getting_out_flags & automobile_door_flag(door) == 0
                })
                .map(|door| (door, get_door_pos(door)))
                .collect()
        };
        scratch.vehicle_driver_doors.insert(*handle, driver_doors);

        let mut pass_doors = Vec::with_capacity(3);
        for (seat, door) in [3_usize, 4, 5].into_iter().enumerate() {
            if seat < usize::from(base.entry.num_max_passengers)
                && (door == 3 || has_rear_doors)
                && !occupied[seat]
                && base.entry.getting_in_flags & automobile_door_flag(door) == 0
                && base.entry.getting_out_flags & automobile_door_flag(door) == 0
            {
                pass_doors.push((door, get_door_pos(door)));
            }
        }
        scratch.vehicle_passenger_doors.insert(*handle, pass_doors);
    }

    let PedVehicleObjectiveScratch {
        animated_ped_handles,
        vehicle_profiles,
        relation_vehicle_by_ped,
        vehicle_handle_by_slot,
        vehicle_can_enter,
        vehicle_passenger_doors,
        vehicle_driver_doors,
        starts,
        ..
    } = &mut *scratch;

    starts.extend(
        script_entities
            .peds
            .iter_mut()
            .filter_map(|(ped_handle, entity)| {
                if animated_ped_handles.contains(&ped_handle) {
                    return None;
                }
                match entity.ped.objective {
                    eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                    | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
                        if !relation_vehicle_by_ped.contains_key(&ped_handle) =>
                    {
                        let vehicle_slot = entity.ped.car_in_objective?;
                        let vehicle_handle = vehicle_handle_by_slot.get(&vehicle_slot).copied()?;
                        let can_enter_vehicle = vehicle_can_enter
                            .get(&vehicle_handle)
                            .copied()
                            .unwrap_or(false);
                        let driver =
                            entity.ped.objective == eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER;

                        let ped_pos = entity.ped.physical.entity.get_position();
                        let nearest_door = |doors: &[(usize, CVector)]| {
                            doors
                                .iter()
                                .enumerate()
                                .min_by(|(_, a), (_, b)| {
                                    let dist_a =
                                        (a.1.x - ped_pos.x).powi(2) + (a.1.y - ped_pos.y).powi(2);
                                    let dist_b =
                                        (b.1.x - ped_pos.x).powi(2) + (b.1.y - ped_pos.y).powi(2);
                                    dist_a.total_cmp(&dist_b)
                                })
                                .map(|(idx, _)| idx)
                        };
                        let (target_door, target_pos) = if driver {
                            let doors = vehicle_driver_doors.get_mut(&vehicle_handle)?;
                            let best_idx = nearest_door(doors)?;
                            doors.remove(best_idx)
                        } else {
                            let doors = vehicle_passenger_doors.get_mut(&vehicle_handle)?;
                            let best_idx = nearest_door(doors)?;
                            doors.remove(best_idx)
                        };

                        let dx = target_pos.x - ped_pos.x;
                        let dy = target_pos.y - ped_pos.y;
                        let dist_sq = dx * dx + dy * dy;

                        let close_enough = dist_sq
                            <= crate::peds::Ped::SEEK_CAR_DONE_DISTANCE
                                * crate::peds::Ped::SEEK_CAR_DONE_DISTANCE;
                        let height_matches =
                            ped_pos.z + 1.5 > target_pos.z && ped_pos.z - 0.5 < target_pos.z;

                        if !close_enough || !height_matches || !can_enter_vehicle {
                            // Ped is still outside door interaction radius: seek toward the door
                            entity.ped.veh_door = target_door as i32;
                            entity.ped.seek_pos = target_pos;
                            entity.ped.ped_state = PedState::PED_SEEK_CAR;
                            if close_enough && (!height_matches || !can_enter_vehicle) {
                                entity.ped.move_state = eMoveState::PEDMOVE_STILL;
                            }
                            None
                        } else {
                            // Arrived at door! Begin get-in animation
                            entity.ped.veh_door = target_door as i32;
                            entity.ped.ped_state = PedState::PED_ENTER_CAR;
                            entity.ped.physical.entity.flags.uses_collision = false;
                            entity.ped.offset_seek = CVector::new(
                                target_pos.x - ped_pos.x,
                                target_pos.y - ped_pos.y,
                                target_pos.z - ped_pos.z,
                            );
                            entity.ped.ped_state_timer = now_ms.wrapping_add(600);
                            let rhs = matches!(target_door, 3 | 5);
                            let high = target_pos.z - ped_pos.z > 4.4;
                            let animation = match (rhs, high) {
                                (true, true) => ANIM_STD_CAR_ALIGNHI_DOOR_RHS,
                                (true, false) => ANIM_STD_CAR_ALIGN_DOOR_RHS,
                                (false, true) => ANIM_STD_CAR_ALIGNHI_DOOR_LHS,
                                (false, false) => ANIM_STD_CAR_ALIGN_DOOR_LHS,
                            };
                            Some((
                                ped_handle,
                                vehicle_handle,
                                target_door,
                                animation,
                                PedVehicleAnimationPhase::Align,
                                driver,
                            ))
                        }
                    }
                    eObjective::OBJECTIVE_LEAVE_CAR
                        if relation_vehicle_by_ped.contains_key(&ped_handle) =>
                    {
                        let vehicle_handle = *relation_vehicle_by_ped.get(&ped_handle)?;
                        let door = entity.ped.veh_door.clamp(2, 5) as usize;
                        entity.ped.ped_state = PedState::PED_EXIT_CAR;
                        let animation = vehicle_profiles
                            .get(&vehicle_handle)
                            .copied()
                            .unwrap_or_default()
                            .exit_animation(door);
                        Some((
                            ped_handle,
                            vehicle_handle,
                            door,
                            animation,
                            PedVehicleAnimationPhase::Exit,
                            false,
                        ))
                    }
                    _ => None,
                }
            }),
    );
    for (ped, vehicle, door, animation, phase, driver) in starts.drain(..) {
        if let Some(mut entity) = script_entities.vehicle_get_mut(vehicle) {
            let base = entity.base_mut();
            match phase {
                PedVehicleAnimationPhase::Align => {
                    base.entry.num_getting_in = base.entry.num_getting_in.saturating_add(1);
                    base.entry.getting_in_flags |= automobile_door_flag(door);
                }
                PedVehicleAnimationPhase::Exit => {
                    base.entry.getting_out_flags |= automobile_door_flag(door);
                }
                _ => {}
            }
        }
        let (Some(ped_entity), Some(vehicle_entity)) = (
            script_entities.ped_entity(ped),
            script_entities.vehicle_entity(vehicle),
        ) else {
            continue;
        };
        queue_ped_vehicle_animation(
            &mut script_entities.commands,
            &manager,
            ped_entity,
            vehicle_entity,
            door,
            animation,
            phase,
            driver,
        );
    }
}

/// Advance persistent re3 `CPed::m_pVehicleAnim` associations and feed their
/// real currentTime into `CAutomobile::ProcessOpenDoor` every game frame.
pub(crate) type FinishedPedVehicleAnimation = (
    Entity,
    i32,
    Entity,
    Option<i32>,
    usize,
    PedVehicleAnimationPhase,
    i32,
    bool,
);

#[derive(Default)]
pub(crate) struct PedVehicleAnimationScratch {
    seated: HashSet<i32>,
    finished: Vec<FinishedPedVehicleAnimation>,
}

pub(crate) fn process_ped_vehicle_animations(
    mut active_animations: Query<(Entity, &mut CPedVehicleAnimation), With<PedEntity>>,
    manager: Res<CAnimManager>,
    mut script_entities: ScriptEntityWorld,
    mut players: CWorldPlayers,
    mut file_loader: Option<ResMut<CFileLoader>>,
    handling_manager: Res<cHandlingDataMgr>,
    ped_animation_offsets: Res<CPedAnimationOffsets>,
    ped_stats: Res<CPedStatsRuntime>,
    scripts: Res<CTheScripts>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut car_ctrl: ResMut<crate::control::CarCtrl::CCarCtrl>,
    mut scratch: Local<PedVehicleAnimationScratch>,
) {
    crate::profile_scope!("process_ped_vehicle_animations");
    let PedVehicleAnimationScratch { seated, finished } = &mut *scratch;
    seated.clear();
    finished.clear();
    seated.extend(script_entities.script_relations().map(|(handle, _)| handle));
    for (ped_entity, mut active) in &mut active_animations {
        // Player peds also carry this component, but their transition callback
        // is processed separately because it has player-control/audio branches.
        let Some(ped_handle) = script_entities.ped_handle_for_entity(ped_entity) else {
            continue;
        };
        // PedDeath owns native SetDie entry cancellation and association fade.
        // A death cannot commit an enter/exit callback while assets are pending.
        if script_entities
            .ped_get(ped_handle)
            .is_some_and(|ped| ped.ped.dying_or_dead())
        {
            continue;
        }
        let vehicle_handle = script_entities.vehicle_handle_for_entity(active.vehicle);
        active
            .association
            .update_time(timer.time_step_in_seconds(), 1.0);
        active
            .association
            .update_blend(timer.time_step_in_seconds());
        let vehicle_exists = match vehicle_handle
            .and_then(|vehicle_handle| script_entities.vehicle_get_mut(vehicle_handle))
        {
            Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) => {
                // Before an animation's authored door window starts,
                // ProcessOpenDoor intentionally does nothing; that must not
                // delete m_pVehicleAnim early.
                car.process_open_door(
                    active.door,
                    active.association.anim_id,
                    active.association.current_time,
                );
                true
            }
            _ => false,
        };
        let line_up = script_entities.ped_get(ped_handle).map(|ped| {
            (
                PedVehicleLineUpContext {
                    current_position: ped.ped.physical.entity.get_position(),
                    current_heading: ped.locomotion.rotation_cur,
                    offset_seek: ped.ped.offset_seek,
                    state_timer_ms: ped.ped.ped_state_timer,
                    now_ms: timer.time_in_milliseconds(),
                },
                ped.ped.physical.move_speed.z,
            )
        });
        if vehicle_exists
            && let Some((line_up, vertical_speed_z)) = line_up
            && let Some(file_loader) = file_loader.as_deref()
            && let Some(vehicle_handle) = vehicle_handle
            && let Some(crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. }) =
                script_entities.vehicle_get(vehicle_handle)
            && let Some(handling) = car
                .vehicle.core.handling_id
                .and_then(|id| usize::try_from(id).ok())
                .and_then(|id| handling_manager.get_handling_data(id))
        {
            if let Some(line_up_result) = automobile_ped_line_up(
                file_loader,
                &car,
                handling,
                &ped_animation_offsets,
                active.door,
                line_up,
                active.phase,
                &active.association,
                vertical_speed_z,
                timer.time_step(),
            ) && let Some(ped) = script_entities.ped_get_mut(ped_handle)
                && (!seated.contains(&ped_handle)
                    || matches!(
                        active.phase,
                        PedVehicleAnimationPhase::Exit
                            | PedVehicleAnimationPhase::CloseExit
                            | PedVehicleAnimationPhase::DraggedOut
                    ))
            {
                ped.ped.physical.entity.set_matrix(line_up_result.matrix);
                ped.ped.physical.move_speed.z = line_up_result.vertical_speed_z;
                let heading = line_up_result
                    .matrix
                    .right
                    .y
                    .atan2(line_up_result.matrix.right.x);
                ped.locomotion.rotation_dest = heading;
                ped.locomotion.rotation_cur = heading;
            }
        }
        if active.phase == PedVehicleAnimationPhase::DraggedOut
            && vehicle_exists
            && !active.dragged_out_finish_ran
            && !active.association.is_running()
            && !active.association.deleted
        {
            if let Some(vehicle_handle) = vehicle_handle {
                finish_script_ped_dragged_out_animation(
                    &mut script_entities,
                    &mut car_ctrl,
                    ped_handle,
                    vehicle_handle,
                    active.door,
                );
                active.association.blend_delta = -1000.0;
                active.association.flags |=
                    crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT;
                active.association.set_delete_callback();
                active.dragged_out_finish_ran = true;
                continue;
            }
        }
        if active.phase == PedVehicleAnimationPhase::DraggedOut
            && active.dragged_out_finish_ran
            && !active.association.deleted
        {
            continue;
        }
        if !vehicle_exists || !active.association.is_running() || active.association.deleted {
            finished.push((
                ped_entity,
                ped_handle,
                active.vehicle,
                vehicle_handle,
                active.door,
                active.phase,
                active.association.anim_id,
                active.enter_as_driver,
            ));
        }
    }
    for (
        ped_entity,
        ped_handle,
        vehicle_entity,
        vehicle_handle,
        door,
        phase,
        animation_id,
        enter_as_driver,
    ) in finished.drain(..)
    {
        script_entities
            .commands
            .entity(ped_entity)
            .remove::<CPedVehicleAnimation>();
        let Some(vehicle_handle) = vehicle_handle else {
            continue;
        };
        let Some((profile, door_missing, door_fully_open, can_open_locks, _engine_on_fire)) =
            script_entities
                .vehicle_get(vehicle_handle)
                .and_then(|vehicle| match vehicle {
                    crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => Some((
                        PedAutomobileProfile::from_car(&car),
                        automobile_door_missing(&car, door),
                        car.doors.get(door).is_some_and(|door| door.is_fully_open()),
                        car.vehicle.can_ped_open_locks(false),
                        car.damage.get_engine_status() >= 225,
                    )),
                    _ => None,
                })
        else {
            continue;
        };
        match phase {
            PedVehicleAnimationPhase::LockedDoor => {
                abort_script_ped_enter_vehicle(
                    &mut script_entities,
                    ped_handle,
                    vehicle_handle,
                    door,
                );
            }
            PedVehicleAnimationPhase::Align => {
                // re3 `PedAnimAlignCB`: a missing door is already open for
                // entry purposes, so skip the door-open animation and go
                // straight to the authored get-in animation.
                let (animation, next_phase) = if door_missing || door_fully_open {
                    (
                        profile.get_in_animation(door),
                        PedVehicleAnimationPhase::GetIn,
                    )
                } else if !can_open_locks {
                    if let Some(ped) = script_entities.ped_get_mut(ped_handle) {
                        ped.ped.flags.cancel_entering_car = true;
                    }
                    (
                        profile.locked_animation(door),
                        PedVehicleAnimationPhase::LockedDoor,
                    )
                } else {
                    (
                        profile.open_animation(door),
                        PedVehicleAnimationPhase::OpenDoor,
                    )
                };
                if !queue_ped_vehicle_animation(
                    &mut script_entities.commands,
                    &manager,
                    ped_entity,
                    vehicle_entity,
                    door,
                    animation,
                    next_phase,
                    enter_as_driver,
                ) {
                    abort_script_ped_enter_vehicle(
                        &mut script_entities,
                        ped_handle,
                        vehicle_handle,
                        door,
                    );
                }
            }
            PedVehicleAnimationPhase::OpenDoor => {
                let vehicle_moving_too_fast = script_entities
                    .vehicle_get(vehicle_handle)
                    .is_some_and(|vehicle| vehicle.base().physical.move_speed.magnitude() > 0.2);
                if vehicle_moving_too_fast {
                    abort_script_ped_enter_vehicle(
                        &mut script_entities,
                        ped_handle,
                        vehicle_handle,
                        door,
                    );
                    let impact = if matches!(door, 3 | 5) {
                        crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_LEFT
                    } else {
                        crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_RIGHT
                    };
                    if let Some(ped) = script_entities.ped_get_mut(ped_handle) {
                        let _ = ped.ped.set_fall(&ped.vitality, 
                            1_000,
                            impact,
                            false,
                            timer.time_in_milliseconds(),
                            timer.frame_counter(),);
                    }
                    continue;
                }
                let animation = profile.get_in_animation(door);
                if !queue_ped_vehicle_animation(
                    &mut script_entities.commands,
                    &manager,
                    ped_entity,
                    vehicle_entity,
                    door,
                    animation,
                    PedVehicleAnimationPhase::GetIn,
                    enter_as_driver,
                ) {
                    abort_script_ped_enter_vehicle(
                        &mut script_entities,
                        ped_handle,
                        vehicle_handle,
                        door,
                    );
                }
            }
            PedVehicleAnimationPhase::PullOut => {
                let animation = profile.get_in_animation(door);
                if !queue_ped_vehicle_animation(
                    &mut script_entities.commands,
                    &manager,
                    ped_entity,
                    vehicle_entity,
                    door,
                    animation,
                    PedVehicleAnimationPhase::GetIn,
                    enter_as_driver,
                ) {
                    abort_script_ped_enter_vehicle(
                        &mut script_entities,
                        ped_handle,
                        vehicle_handle,
                        door,
                    );
                }
            }
            PedVehicleAnimationPhase::DraggedOut => {
                let exit_position = script_entities
                    .ped_get(ped_handle)
                    .map(|ped| {
                        let mut offset = if animation_id
                            == crate::animation::AnimationId::ANIM_STD_QUICKJACKED
                        {
                            ped_animation_offsets.ped_quick_dragged_out_car
                        } else {
                            ped_animation_offsets.ped_dragged_out_car
                        };
                        if matches!(door, 3 | 5) {
                            offset.x = -offset.x;
                            if animation_id != crate::animation::AnimationId::ANIM_STD_QUICKJACKED {
                                offset.y = -offset.y;
                                offset.z = -offset.z;
                            }
                        }
                        let position = ped.ped.physical.entity.get_position();
                        let translated = ped.ped.physical.entity.transform_vector(offset);
                        CVector::new(
                            position.x + translated.x,
                            position.y + translated.y,
                            position.z
                                + translated.z
                                + if animation_id
                                    == crate::animation::AnimationId::ANIM_STD_QUICKJACKED
                                {
                                    0.0
                                } else {
                                    1.0
                                },
                        )
                    })
                    .map(|position| ground_ped_exit_position(file_loader.as_deref(), position));
                complete_script_ped_dragged_out_position(
                    &mut script_entities,
                    file_loader.as_deref_mut(),
                    &mut players,
                    &ped_stats,
                    &scripts,
                    &mut general,
                    timer.time_in_milliseconds(),
                    ped_handle,
                    vehicle_handle,
                    door,
                    animation_id == crate::animation::AnimationId::ANIM_STD_QUICKJACKED,
                    exit_position,
                );
            }
            PedVehicleAnimationPhase::GetIn => {
                if profile.is_bus {
                    complete_script_ped_enter_vehicle(
                        &mut script_entities,
                        &mut car_ctrl,
                        timer.time_in_milliseconds(),
                        ped_handle,
                        vehicle_handle,
                        door,
                        enter_as_driver,
                    );
                    continue;
                }
                if door_missing {
                    if enter_as_driver && door == 3 {
                        let animation = profile.shuffle_animation();
                        if queue_ped_vehicle_animation(
                            &mut script_entities.commands,
                            &manager,
                            ped_entity,
                            vehicle_entity,
                            door,
                            animation,
                            PedVehicleAnimationPhase::Shuffle,
                            true,
                        ) {
                            continue;
                        }
                    }
                    complete_script_ped_enter_vehicle(
                        &mut script_entities,
                        &mut car_ctrl,
                        timer.time_in_milliseconds(),
                        ped_handle,
                        vehicle_handle,
                        door,
                        enter_as_driver,
                    );
                    continue;
                }
                let animation = profile.close_enter_animation(door);
                if !queue_ped_vehicle_animation(
                    &mut script_entities.commands,
                    &manager,
                    ped_entity,
                    vehicle_entity,
                    door,
                    animation,
                    PedVehicleAnimationPhase::CloseEnter,
                    enter_as_driver,
                ) {
                    abort_script_ped_enter_vehicle(
                        &mut script_entities,
                        ped_handle,
                        vehicle_handle,
                        door,
                    );
                }
            }
            PedVehicleAnimationPhase::CloseEnter => {
                if enter_as_driver && door == 3 {
                    if let Some(mut car) = script_entities
                        .vehicle_get_mut(vehicle_handle)
                        .and_then(automobile_mut)
                    {
                        finish_enter_door_close(&mut car, door);
                    }
                    let animation = profile.shuffle_animation();
                    if queue_ped_vehicle_animation(
                        &mut script_entities.commands,
                        &manager,
                        ped_entity,
                        vehicle_entity,
                        door,
                        animation,
                        PedVehicleAnimationPhase::Shuffle,
                        true,
                    ) {
                        continue;
                    }
                }
                complete_script_ped_enter_vehicle(
                    &mut script_entities,
                    &mut car_ctrl,
                    timer.time_in_milliseconds(),
                    ped_handle,
                    vehicle_handle,
                    door,
                    enter_as_driver,
                );
            }
            PedVehicleAnimationPhase::Shuffle => complete_script_ped_enter_vehicle(
                &mut script_entities,
                &mut car_ctrl,
                timer.time_in_milliseconds(),
                ped_handle,
                vehicle_handle,
                door,
                enter_as_driver,
            ),
            PedVehicleAnimationPhase::Exit => {
                // Script peds are not player-controlled, so re3's pad branch
                // never applies; only the vehicle-side conditions remain.
                let close_door = script_entities
                    .vehicle_get(vehicle_handle)
                    .and_then(automobile)
                    .is_some_and(|car| automobile_close_door_after_exit(&car, door, None));
                if !close_door {
                    if !door_missing && !profile.is_bus {
                        if let Some(mut car) = script_entities
                            .vehicle_get_mut(vehicle_handle)
                            .and_then(automobile_mut)
                        {
                            car.damage.set_door_status(
                                door as u8,
                                crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING
                                    as u32,
                            );
                        }
                    }
                    let exit_position = script_entities
                        .ped_get(ped_handle)
                        .map(|ped| ped.ped.physical.entity.get_position())
                        .map(|position| ground_ped_exit_position(file_loader.as_deref(), position));
                    complete_script_ped_exit_vehicle(
                        &mut script_entities,
                        &mut car_ctrl,
                        file_loader.as_deref_mut(),
                        &mut players,
                        &ped_stats,
                        &scripts,
                        &mut general,
                        timer.time_in_milliseconds(),
                        ped_handle,
                        vehicle_handle,
                        door,
                        exit_position,
                    );
                    continue;
                }
                let animation = profile.close_exit_animation(door);
                if !queue_ped_vehicle_animation(
                    &mut script_entities.commands,
                    &manager,
                    ped_entity,
                    vehicle_entity,
                    door,
                    animation,
                    PedVehicleAnimationPhase::CloseExit,
                    false,
                ) {
                    warn!("script ped close-exit animation {animation} is unavailable");
                    let exit_position = script_entities
                        .ped_get(ped_handle)
                        .map(|ped| ped.ped.physical.entity.get_position())
                        .map(|position| ground_ped_exit_position(file_loader.as_deref(), position));
                    complete_script_ped_exit_vehicle(
                        &mut script_entities,
                        &mut car_ctrl,
                        file_loader.as_deref_mut(),
                        &mut players,
                        &ped_stats,
                        &scripts,
                        &mut general,
                        timer.time_in_milliseconds(),
                        ped_handle,
                        vehicle_handle,
                        door,
                        exit_position,
                    );
                }
            }
            PedVehicleAnimationPhase::CloseExit => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicle_get_mut(vehicle_handle)
                {
                    car.open_door(door, 0.0);
                }
                let exit_position = script_entities
                    .ped_get(ped_handle)
                    .map(|ped| ped.ped.physical.entity.get_position())
                    .map(|position| ground_ped_exit_position(file_loader.as_deref(), position));
                complete_script_ped_exit_vehicle(
                    &mut script_entities,
                    &mut car_ctrl,
                    file_loader.as_deref_mut(),
                    &mut players,
                    &ped_stats,
                    &scripts,
                    &mut general,
                    timer.time_in_milliseconds(),
                    ped_handle,
                    vehicle_handle,
                    door,
                    exit_position,
                );
            }
            PedVehicleAnimationPhase::RollingDoorClose => {
                // Script peds do not normally enter this phase (the source
                // branch is `CPlayerPed` only), but keep the shared runtime
                // exhaustive and deterministic if a mission drives the same
                // association explicitly.
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicle_get_mut(vehicle_handle)
                {
                    let animation = if car.vehicle.core.flags.low_vehicle {
                        crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LO_LHS
                    } else {
                        crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS
                    };
                    car.process_open_door(door, animation, 1.0);
                    car.vehicle.entry.getting_out_flags &= !automobile_door_flag(door);
                    if car.damage.get_door_status(door as u8)
                        == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
                    {
                        car.damage.set_door_status(
                            door as u8,
                            crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_OK as u32,
                        );
                    }
                }
            }
            PedVehicleAnimationPhase::External => {}
        }
    }
}

fn abort_script_ped_enter_vehicle(
    world: &mut ScriptEntityWorld,
    ped_handle: i32,
    vehicle_handle: i32,
    door: usize,
) {
    if let Some(mut vehicle) = world.vehicles.get_mut(&world.index, &vehicle_handle) {
        let base = vehicle.base_mut();
        base.entry.num_getting_in = base.entry.num_getting_in.saturating_sub(1);
        base.entry.getting_in_flags &= !automobile_door_flag(door);
    }
    if let Some(ped) = world.peds.get_mut(&world.index, &ped_handle) {
        ped.ped.flags.cancel_entering_car = false;
        ped.ped.seek_target = None;
        ped.ped.physical.entity.flags.uses_collision = true;
        ped.ped.physical.move_speed = CVector::ZERO;
        ped.ped.ped_state = PedState::PED_IDLE;
        ped.ped.move_state = eMoveState::PEDMOVE_STILL;
        ped.ped.restore_previous_objective(None);
    }
}

pub(crate) fn complete_script_ped_enter_vehicle(
    world: &mut ScriptEntityWorld,
    car_ctrl: &mut crate::control::CarCtrl::CCarCtrl,
    now_ms: u32,
    ped_handle: i32,
    vehicle_handle: i32,
    door: usize,
    as_driver: bool,
) {
    // re3 `CPed::PedAnimDoorCloseCB` (PedAI.cpp:2751-2764) finishes CloseEnter
    // with a hard hinge close plus SWINGING→OK, because the authored close
    // window (0.2-0.63 of `CAR_CLOSE_DOOR_LHS`) ends after the real 0.467s
    // PED.IFP animation: without this the door is left visibly ajar.
    // `process_open_door` also keeps the MISSING guard that a raw hinge
    // write would skip.
    if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
        world.vehicles.get_mut(&world.index, &vehicle_handle)
    {
        if !car.vehicle.core.flags.is_bus {
            car.process_open_door(
                door,
                crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_LHS,
                1.0,
            );
        }
        if car.damage.get_door_status(door as u8)
            == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
        {
            car.damage.set_door_status(
                door as u8,
                crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_OK as u32,
            );
        }
    }
    let relation_seat = if as_driver {
        VehicleSeat::Driver
    } else {
        // re3 `PedSetInCarCB`: RF -> passenger 0, LR -> 1, RR -> 2.
        // The entry path already reserves one of doors 3..=5.
        VehicleSeat::Passenger(door.saturating_sub(3).min(2) as u8)
    };
    if !world.set_char_relation(ped_handle, vehicle_handle, relation_seat) {
        if let Some(mut vehicle) = world.vehicles.get_mut(&world.index, &vehicle_handle) {
            let base = vehicle.base_mut();
            base.entry.num_getting_in = base.entry.num_getting_in.saturating_sub(1);
            base.entry.getting_in_flags &= !automobile_door_flag(door);
        }
        if let Some(ped) = world.peds.get_mut(&world.index, &ped_handle) {
            ped.ped.ped_state = PedState::PED_IDLE;
            ped.ped.move_state = eMoveState::PEDMOVE_STILL;
            ped.ped.physical.entity.flags.uses_collision = true;
        }
        return;
    }

    // re3 `PedSetInCarCB`: when an NPC successfully takes the driver's seat
    // through OBJECTIVE_ENTER_CAR_AS_DRIVER, ambient passengers are told to
    // leave. With the one-sided ECS relation this is derived from occupants
    // rather than mirrored vehicle passenger pointers.
    let entering_as_driver_objective = as_driver
        && world.peds.get(&world.index, &ped_handle)
            .is_some_and(|ped| ped.ped.objective == eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER);
    if entering_as_driver_objective
        && let Some(vehicle_entity) = world.vehicle_entity(vehicle_handle)
    {
        let random_passengers = world
            .script_occupant_handles(vehicle_entity)
            .filter_map(|(occupant_handle, seat)| {
                (occupant_handle != ped_handle
                    && matches!(seat, VehicleSeat::Passenger(_))
                    && world.peds.get(&world.index, &occupant_handle)
                        .is_some_and(|occupant| occupant.ped.char_created_by == RANDOM_CHAR))
                .then_some(occupant_handle)
            })
            .collect::<Vec<_>>();
        let vehicle_slot = CPoolHandle(vehicle_handle).index();
        for passenger_handle in random_passengers {
            if let Some(passenger) = world.peds.get_mut(&world.index, &passenger_handle) {
                passenger.ped.flags.script_objective_completed = false;
                passenger.ped.set_objective_entity(eObjective::OBJECTIVE_LEAVE_CAR,
                    vehicle_slot,
                    now_ms,
                    Some(vehicle_slot),
                );
            }
        }
    }

    let entering_cop = as_driver
        && world.peds.get(&world.index, &ped_handle)
            .is_some_and(|ped| ped.ped.ped_type == ePedType::PEDTYPE_COP);
    let Some(mut vehicle) = world.vehicles.get_mut(&world.index, &vehicle_handle) else {
        return;
    };
    let mut base = vehicle.base_mut();
    base.entry.num_getting_in = base.entry.num_getting_in.saturating_sub(1);
    base.entry.getting_in_flags &= !automobile_door_flag(door);
    if as_driver {
        base.drive.engine_on = true;
        base.set_status(crate::entities::Entity::eEntityStatus::STATUS_PHYSICS);
        let model = base.physical.entity.model_index.max(0) as usize;
        let law_vehicle = matches!(
            model,
            crate::modelinfo::ModelIndices::MI_FBICAR
                | crate::modelinfo::ModelIndices::MI_POLICE
                | crate::modelinfo::ModelIndices::MI_ENFORCER
                | crate::modelinfo::ModelIndices::MI_PREDATOR
                | crate::modelinfo::ModelIndices::MI_RHINO
                | crate::modelinfo::ModelIndices::MI_BARRACKS
        );
        if entering_cop && law_vehicle && !base.flags.is_law_enforcer {
            // PedSetInCarCB -> ChangeLawEnforcerState(true). This is not the
            // same as merely being a police-model car: the state means an
            // active cop is driving it and participates in pursuit/budgets.
            base.flags.is_law_enforcer = true;
            car_ctrl.num_law_enforcer_cars += 1;
        }
    }
    if let Some(ped) = world.peds.get_mut(&world.index, &ped_handle) {
        ped.ped.ped_state = PedState::PED_DRIVING;
        ped.ped.physical.entity.flags.uses_collision = false;
        ped.ped.flags.script_objective_completed = true;

        // re3 `CPed::PedSetInCarCB` restores the objective that was active
        // before the entry request, then clears ordinary entry objectives.
        // Leaving `OBJECTIVE_ENTER_CAR_AS_PASSENGER` active makes SCM wait
        // forever on `IS_CHAR_OBJECTIVE_PASSED`, even though the ped is seated.
        let entering = matches!(
            ped.ped.objective,
            eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
        );
        if entering {
            if matches!(
                ped.ped.prev_objective,
                eObjective::OBJECTIVE_RUN_TO_AREA
                    | eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
                    | eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
            ) {
                ped.ped.prev_objective = eObjective::OBJECTIVE_NONE;
            }
            ped.ped
                .restore_previous_objective(Some(CPoolHandle(vehicle_handle).index()));
            if !matches!(
                ped.ped.objective,
                eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
                    | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
                    | eObjective::OBJECTIVE_LEAVE_CAR
                    | eObjective::OBJECTIVE_FOLLOW_CAR_IN_CAR
                    | eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS
                    | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
                    | eObjective::OBJECTIVE_RUN_TO_AREA
            ) {
                ped.ped.objective = eObjective::OBJECTIVE_NONE;
            }
        }
    }
}

pub(crate) fn complete_script_ped_exit_vehicle(
    world: &mut ScriptEntityWorld,
    car_ctrl: &mut crate::control::CarCtrl::CCarCtrl,
    mut file_loader: Option<&mut CFileLoader>,
    players: &mut CWorldPlayers,
    ped_stats: &CPedStatsRuntime,
    scripts: &CTheScripts,
    general: &mut CGeneral,
    now_ms: u32,
    ped_handle: i32,
    vehicle_handle: i32,
    door: usize,
    exit_position: Option<CVector>,
) {
    let was_driver = world
        .relation_for_char(ped_handle)
        .is_some_and(|relation| relation.is_driver());
    let was_cop_driver = was_driver
        && world.peds.get(&world.index, &ped_handle)
            .is_some_and(|ped| ped.ped.ped_type == ePedType::PEDTYPE_COP);
    let objective_target = world.peds.get(&world.index, &ped_handle)
        .and_then(|ped| ped.ped.ped_in_objective);
    let objective_target_is_player = objective_target.is_some_and(|target| {
        (0..players.slot_count()).any(|index| {
            players
                .player(index as i32)
                .is_some_and(|player| CPoolHandle(player.pool_handle).index() == target)
        })
    });
    let _ = world.clear_char_relation(ped_handle);
    let mut cleared_exit_position = exit_position;
    let mut vehicle_position = None;
    if let Some(mut vehicle) = world.vehicles.get_mut(&world.index, &vehicle_handle) {
        let mut base = vehicle.base_mut();
        vehicle_position = Some(base.physical.entity.get_position());
        if was_driver {
            finish_driver_removal(&mut base);
        }
        base.entry.getting_out_flags &= !automobile_door_flag(door);
        let model = base.physical.entity.model_index.max(0) as usize;
        let law_vehicle = matches!(
            model,
            crate::modelinfo::ModelIndices::MI_FBICAR
                | crate::modelinfo::ModelIndices::MI_POLICE
                | crate::modelinfo::ModelIndices::MI_ENFORCER
                | crate::modelinfo::ModelIndices::MI_PREDATOR
                | crate::modelinfo::ModelIndices::MI_RHINO
                | crate::modelinfo::ModelIndices::MI_BARRACKS
        );
        if was_cop_driver && law_vehicle && base.flags.is_law_enforcer {
            // PedSetOutCarCB / PedSetDraggedOutCarCB symmetry.
            base.flags.is_law_enforcer = false;
            car_ctrl.num_law_enforcer_cars = car_ctrl.num_law_enforcer_cars.saturating_sub(1);
        }

        if let Some(position) = exit_position {
            let matrix = base.physical.entity.matrix;
            if let Some(col) = file_loader.as_deref().and_then(|loader| {
                loader
                    .model_info
                    .col_model(base.physical.entity.model_index.max(0) as usize)
            }) {
                let mut local = matrix.multiply_inverse_point(position);
                // PedSetOutCarCB keeps the authored animation endpoint when it
                // is already clear. PositionPedOutOfCollision is only the
                // fallback when that endpoint still intersects collision.
                const PED_RADIUS: f32 = 0.35;
                let overlaps_vehicle = local.x >= col.bounding_box.min.x - PED_RADIUS
                    && local.x <= col.bounding_box.max.x + PED_RADIUS
                    && local.y >= col.bounding_box.min.y - PED_RADIUS
                    && local.y <= col.bounding_box.max.y + PED_RADIUS;
                if overlaps_vehicle {
                    if matches!(door, 3 | 5) {
                        local.x = col.bounding_box.max.x + PED_RADIUS;
                    } else {
                        local.x = col.bounding_box.min.x - PED_RADIUS;
                    }
                    let world_position = matrix.transform_point(local);
                    cleared_exit_position = Some(ground_ped_exit_position(
                        file_loader.as_deref(),
                        world_position,
                    ));
                }
            }
        }
    }
    if let Some(mut ped) = world.peds.get_mut(&world.index, &ped_handle) {
        if let Some(position) = cleared_exit_position {
            ped.ped.physical.entity.set_position(position);
        }
        ped.ped.ped_state = PedState::PED_IDLE;
        ped.ped.physical.entity.flags.uses_collision = true;
        ped.ped.physical.move_speed = CVector::ZERO;
        ped.ped.colliding_entity = None;

        // re3 `PedSetOutCarCB` restores the objective saved by
        // `SetObjective(OBJECTIVE_LEAVE_CAR)`. Without this transition the
        // ped remains in LEAVE_CAR after physically exiting and the following
        // SCM RUN_TO_AREA/GOTO_AREA objective cannot own the state machine.
        let has_followup_objective = ped.ped.objective == eObjective::OBJECTIVE_LEAVE_CAR
            && ped.ped.prev_objective != eObjective::OBJECTIVE_NONE;
        if ped.ped.objective == eObjective::OBJECTIVE_LEAVE_CAR {
            ped.ped.restore_previous_objective(None);
        }
        // Completing the temporary LEAVE_CAR must not mark a queued
        // RUN_TO_AREA/GOTO_AREA objective as already passed. re3's script flag
        // was cleared when that follow-up objective was assigned while the
        // temporary objective was active.
        ped.ped.flags.script_objective_completed = !has_followup_objective;

        // re3 `PedSetOutCarCB` immediately hands ambient occupants back to
        // their pedestrian AI. Without this, RANDOM_CHAR drivers/passengers
        // that leave a car successfully remain parked in PED_IDLE forever.
        if ped.ped.flags.flee_after_exiting_car {
            ped.ped.flags.flee_after_exiting_car = false;
            if let Some(source) = vehicle_position {
                let fear = ped_stats.GetPedStats(ped.ped.ped_stats).m_fear;
                let _ = ped.ped.set_flee_from_position(&ped.vitality, ped.locomotion, source, 12_000, now_ms);
                ped.ped.move_state = if general.get_random_number() & 1 != 0 || fear > 70 {
                    queue_ped_script_say(&mut ped.ped, SOUND_PED_FLEE_SPRINT as i16);
                    eMoveState::PEDMOVE_SPRINT
                } else {
                    queue_ped_script_say(&mut ped.ped, SOUND_PED_FLEE_RUN as i16);
                    eMoveState::PEDMOVE_RUN
                };
                ped.ped.flags.is_running = true;
                ped.ped.last_ped_state = PedState::PED_WANDER_PATH;
            }
        } else if ped.ped.flags.wander_path_after_exiting_car
            || (ped.ped.objective == eObjective::OBJECTIVE_NONE
                && ped.ped.char_created_by != MISSION_CHAR
                && ped.ped.ped_state == PedState::PED_IDLE)
        {
            ped.ped.flags.wander_path_after_exiting_car = false;
            let direction = general.get_random_number_in_range_i32(0, 8);
            if let Some(loader) = file_loader.as_deref_mut() {
                apply_script_ped_wander_path(
                    &mut ped.ped,
                    &ped.vitality,
                    direction,
                    loader,
                    general,
                    now_ms,
                );
            } else {
                // The path resource is optional during teardown/tests. Keep
                // the same ownership/state transition and let the ambient
                // wander system resolve the first node once streaming exists.
                ped.ped.path_dir = direction as i16;
                ped.ped.next_path_node = None;
                ped.ped.ped_state = PedState::PED_WANDER_PATH;
                ped.ped.move_state = eMoveState::PEDMOVE_WALK;
                ped.ped.flags.is_running = false;
            }
            if ped.ped.ped_type == ePedType::PEDTYPE_PROSTITUTE {
                ped.ped.set_objective_timer(now_ms, 30_000);
            }
        } else if ped.ped.flags.gonna_kill_the_car_jacker {
            ped.ped.flags.gonna_kill_the_car_jacker = false;
            if ped.ped.ped_in_objective.is_some() {
                if general.get_random_number() & 1 == 0
                    && ped.ped.ped_type != ePedType::PEDTYPE_COP
                    && (!objective_target_is_player || !scripts.is_player_on_a_mission())
                {
                    ped.ped.clear_objective(&ped.vitality, false, None);
                    ped.ped.set_objective_entity(eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER,
                        CPoolHandle(vehicle_handle).index(),
                        now_ms,
                        None,
                    );
                }
                ped.ped.leave_car_timer = now_ms.wrapping_add(1_500);
            }
            apply_ped_wait_state(&mut ped.ped, ped.locomotion, 15, Some(1_500), now_ms);
            ped.ped.move_state = eMoveState::PEDMOVE_RUN;
            ped.ped.flags.is_running = true;
        }
    }
}

/// re3 `CPed::PedSetDraggedOutCarCB` (`Ped.cpp:4363`). The dragged animation
/// finishing removes the occupant from the vehicle and starts the association
/// fade; placement and post-carjack AI intentionally wait for its delete
/// callback on a later frame.
fn finish_script_ped_dragged_out_animation(
    world: &mut ScriptEntityWorld,
    car_ctrl: &mut crate::control::CarCtrl::CCarCtrl,
    ped_handle: i32,
    vehicle_handle: i32,
    door: usize,
) {
    let was_driver = world
        .relation_for_char(ped_handle)
        .is_some_and(|relation| relation.is_driver());
    let was_cop_driver = was_driver
        && world.peds.get(&world.index, &ped_handle)
            .is_some_and(|ped| ped.ped.ped_type == ePedType::PEDTYPE_COP);
    let _ = world.clear_char_relation(ped_handle);

    if let Some(mut vehicle) = world.vehicles.get_mut(&world.index, &vehicle_handle) {
        let mut base = vehicle.base_mut();
        base.entry.getting_out_flags &= !automobile_door_flag(door);
        if was_driver {
            finish_driver_removal(&mut base);
        }
        let model = base.physical.entity.model_index.max(0) as usize;
        let law_vehicle = matches!(
            model,
            crate::modelinfo::ModelIndices::MI_FBICAR
                | crate::modelinfo::ModelIndices::MI_POLICE
                | crate::modelinfo::ModelIndices::MI_ENFORCER
                | crate::modelinfo::ModelIndices::MI_PREDATOR
                | crate::modelinfo::ModelIndices::MI_RHINO
                | crate::modelinfo::ModelIndices::MI_BARRACKS
        );
        if was_cop_driver && law_vehicle && base.flags.is_law_enforcer {
            base.flags.is_law_enforcer = false;
            car_ctrl.num_law_enforcer_cars = car_ctrl.num_law_enforcer_cars.saturating_sub(1);
        }
    }

    if let Some(ped) = world.peds.get_mut(&world.index, &ped_handle) {
        if ped.ped.ped_state != PedState::PED_ARRESTED {
            ped.ped.last_ped_state = PedState::PED_NONE;
        }
        ped.ped.seek_target = None;
        ped.ped.stored_move_state = eMoveState::PEDMOVE_NONE;
        ped.ped.flags.veh_exit_will_be_instant = false;
    }
}

#[derive(Clone, Copy)]
struct DraggedOutDriver {
    slot: usize,
    is_player: bool,
    is_same_ped: bool,
}

/// re3 `PedSetDraggedOutCarPositionCB` /
/// `PedSetQuickDraggedOutCarPositionCB` (`PedAI.cpp:4319,4414`).
#[allow(clippy::too_many_arguments)]
fn complete_script_ped_dragged_out_position(
    world: &mut ScriptEntityWorld,
    mut file_loader: Option<&mut CFileLoader>,
    players: &mut CWorldPlayers,
    ped_stats: &CPedStatsRuntime,
    scripts: &CTheScripts,
    general: &mut CGeneral,
    now_ms: u32,
    ped_handle: i32,
    vehicle_handle: i32,
    door: usize,
    quick_jack: bool,
    exit_position: Option<CVector>,
) {
    let vehicle_entity = world.vehicle_entity(vehicle_handle);
    let (vehicle_position, vehicle_heading, vehicle_created_by) = world.vehicles.get(&world.index, &vehicle_handle)
        .map(|vehicle| {
            let base = vehicle.base();
            (
                base.physical.entity.get_position(),
                base.physical.entity.get_heading(),
                base.vehicle_created_by,
            )
        })
        .unwrap_or((CVector::ZERO, 0.0, MISSION_VEHICLE));
    let vehicle_exists = world.vehicles.contains_key(&world.index, &vehicle_handle);
    let vehicle_slot = CPoolHandle(vehicle_handle).index();

    let script_driver = script_driver_handle(world, vehicle_handle).map(|driver_handle| {
        DraggedOutDriver {
            slot: CPoolHandle(driver_handle).index(),
            is_player: false,
            is_same_ped: driver_handle == ped_handle,
        }
    });
    let player_driver = vehicle_entity.and_then(|vehicle_entity| {
        (0..players.slot_count()).find_map(|index| {
            players
                .relation(index as i32)
                .filter(|relation| relation.vehicle == vehicle_entity && relation.is_driver())
                .and_then(|_| players.player(index as i32))
                .map(|player| DraggedOutDriver {
                    slot: CPoolHandle(player.pool_handle).index(),
                    is_player: true,
                    is_same_ped: false,
                })
        })
    });
    let current_driver = player_driver.or(script_driver);
    let player_on_mission = scripts.is_player_on_a_mission();

    let cleared_exit_position = exit_position.map(|position| {
        clear_ped_exit_position(
            world,
            file_loader.as_deref(),
            vehicle_handle,
            door,
            position,
        )
    });

    let Some(entity) = world.peds.get_mut(&world.index, &ped_handle) else {
        return;
    };
    let ped = &mut *entity.ped;
        let locomotion = &mut *entity.locomotion;
    let vitality = &mut *entity.vitality;
    ped.physical.entity.flags.uses_collision = true;
    ped.physical.move_speed = CVector::ZERO;
    ped.colliding_entity = None;
    if let Some(position) = cleared_exit_position {
        ped.physical.entity.set_position(position);
    }
    if quick_jack && vehicle_exists {
        let heading = CGeneral::limit_radian_angle(vehicle_heading - std::f32::consts::FRAC_PI_2);
        locomotion.rotation_dest = heading;
        locomotion.rotation_cur = heading;
        ped.physical.entity.set_heading(heading);
    }
    if !ped.can_set_ped_state() {
        return;
    }

    let stats = ped_stats.GetPedStats(ped.ped_stats);
    let temper_exceeds_fear = stats.m_temper > stats.m_fear;
    let fear = stats.m_fear;

    if quick_jack {
        set_script_ped_idle(ped, general, now_ms);
        if vehicle_exists {
            if ped.flags.flee_after_exiting_car {
                ped.flags.flee_after_exiting_car = false;
                let _ = ped.set_flee_from_position(vitality, locomotion, vehicle_position, 14_000, now_ms);
            } else if ped.flags.wander_path_after_exiting_car {
                ped.flags.wander_path_after_exiting_car = false;
                set_script_ped_wander_after_exit(ped, vitality, file_loader.as_deref_mut(), general, now_ms);
            } else if ped.flags.gonna_kill_the_car_jacker {
                ped.flags.gonna_kill_the_car_jacker = false;
                let keep_kill_objective = ped.ped_in_objective.is_some()
                    && general.get_random_number() & 1 != 0;
                if keep_kill_objective {
                    if ped.objective != eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
                        && let Some(target) = ped.ped_in_objective
                    {
                        ped.set_objective_entity(eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT,
                            target,
                            now_ms,
                            None,
                        );
                    }
                } else if current_driver.is_none_or(|driver| {
                    driver.is_same_ped || (driver.is_player && player_on_mission)
                }) {
                    let _ = ped.set_flee_from_position(vitality, locomotion, vehicle_position, 14_000, now_ms);
                } else {
                    ped.clear_objective(vitality, false, None);
                    ped.set_objective_entity(eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER,
                        vehicle_slot,
                        now_ms,
                        None,
                    );
                }
                ped.flags.use_ped_node_seek = true;
                ped.next_path_node = None;
                queue_ped_script_say(ped, SOUND_PED_FLEE_RUN as i16);
            } else if temper_exceeds_fear
                && ped.char_created_by != MISSION_CHAR
                && vehicle_created_by != MISSION_VEHICLE
                && player_driver.is_some()
                && !player_on_mission
            {
                let player_slot = player_driver.expect("checked above").slot;
                if general.get_random_true_false() {
                    ped.set_objective_entity(eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT,
                        player_slot,
                        now_ms,
                        None,
                    );
                } else {
                    ped.set_objective_entity(eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER,
                        vehicle_slot,
                        now_ms,
                        None,
                    );
                }
            } else {
                let _ = ped.set_flee_from_position(vitality, locomotion, vehicle_position, 10_000, now_ms);
                ped.flags.use_ped_node_seek = true;
                ped.next_path_node = None;
                if general.get_random_number() & 1 != 0 || fear > 70 {
                    ped.move_state = eMoveState::PEDMOVE_SPRINT;
                    queue_ped_script_say(ped, SOUND_PED_FLEE_SPRINT as i16);
                } else {
                    ped.move_state = eMoveState::PEDMOVE_RUN;
                    queue_ped_script_say(ped, SOUND_PED_FLEE_RUN as i16);
                }
            }
        }
        if ped.last_ped_state == PedState::PED_IDLE {
            ped.last_ped_state = PedState::PED_WANDER_PATH;
        }
        return;
    }

    if !vehicle_exists {
        set_script_ped_idle(ped, general, now_ms);
    } else if ped.flags.flee_after_exiting_car {
        ped.flags.flee_after_exiting_car = false;
        let _ = ped.set_flee_from_position(vitality, locomotion, vehicle_position, 4_000, now_ms);
    } else if ped.flags.wander_path_after_exiting_car {
        ped.flags.wander_path_after_exiting_car = false;
        set_script_ped_wander_after_exit(ped, vitality, file_loader.as_deref_mut(), general, now_ms);
    } else if ped.flags.gonna_kill_the_car_jacker {
        ped.flags.gonna_kill_the_car_jacker = false;
        let keep_kill_objective = ped.ped_in_objective.is_some()
            && general.get_random_number() & 1 != 0;
        if !keep_kill_objective {
            if current_driver.is_none_or(|driver| {
                driver.is_same_ped || (driver.is_player && player_on_mission)
            }) {
                ped.ped_state = PedState::PED_NONE;
                ped.last_ped_state = PedState::PED_NONE;
                let _ = ped.set_flee_from_position(vitality, locomotion, vehicle_position, 4_000, now_ms);
            } else {
                ped.clear_objective(vitality, false, None);
                ped.set_objective_entity(eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER,
                    vehicle_slot,
                    now_ms,
                    None,
                );
            }
        }
    } else if temper_exceeds_fear
        && ped.char_created_by != MISSION_CHAR
        && vehicle_created_by != MISSION_VEHICLE
        && player_driver.is_some()
        && !player_on_mission
    {
        let player_slot = player_driver.expect("checked above").slot;
        if general.get_random_number() & 1 != 0 {
            ped.set_objective_entity(eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT,
                player_slot,
                now_ms,
                None,
            );
        } else {
            ped.set_objective_entity(eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER,
                vehicle_slot,
                now_ms,
                None,
            );
        }
    } else {
        ped.ped_state = PedState::PED_NONE;
        ped.last_ped_state = PedState::PED_NONE;
        let _ = ped.set_flee_from_position(vitality, locomotion, vehicle_position, 10_000, now_ms);
        ped.flags.use_ped_node_seek = true;
        ped.next_path_node = None;
    }

    start_script_ped_get_up_after_drag(ped);
}

/// State transition from re3 `CPed::SetGetUp` (`Ped.cpp:5240`) used by
/// `PedSetDraggedOutCarPositionCB`. The shared fall/get-up service owns the
/// authored animation and blocked-by-car retry.
fn start_script_ped_get_up_after_drag(ped: &mut crate::peds::Ped::CPed) {
    if !ped.can_set_ped_state() {
        return;
    }
    if ped.ped_state != PedState::PED_GETUP {
        ped.set_stored_state();
        ped.ped_state = PedState::PED_GETUP;
    }
    ped.flags.get_up_anim_started = false;
    ped.impact_animation_id = None;
    ped.seek_target = None;
}

fn set_script_ped_wander_after_exit(
    ped: &mut crate::peds::Ped::CPed, vitality: &crate::peds::PedVitality::PedVitality,
    file_loader: Option<&mut CFileLoader>,
    general: &mut CGeneral,
    now_ms: u32,
) {
    let direction = general.get_random_number_in_range_i32(0, 8);
    if let Some(loader) = file_loader {
        apply_script_ped_wander_path(
            ped, vitality,
            direction,
            loader,
            general,
            now_ms,
        );
    } else {
        ped.path_dir = direction as i16;
        ped.next_path_node = None;
        ped.ped_state = PedState::PED_WANDER_PATH;
        ped.move_state = eMoveState::PEDMOVE_WALK;
        ped.flags.is_running = false;
    }
}

fn clear_ped_exit_position(
    world: &ScriptEntityWorld,
    file_loader: Option<&CFileLoader>,
    vehicle_handle: i32,
    door: usize,
    position: CVector,
) -> CVector {
    let Some(base) = world.vehicles.get(&world.index, &vehicle_handle).map(|vehicle| vehicle.base()) else {
        return position;
    };
    let Some(col) = file_loader.and_then(|loader| {
        loader
            .model_info
            .col_model(base.physical.entity.model_index.max(0) as usize)
    }) else {
        return position;
    };
    let matrix = base.physical.entity.matrix;
    let mut local = matrix.multiply_inverse_point(position);
    const PED_RADIUS: f32 = 0.35;
    let overlaps_vehicle = local.x >= col.bounding_box.min.x - PED_RADIUS
        && local.x <= col.bounding_box.max.x + PED_RADIUS
        && local.y >= col.bounding_box.min.y - PED_RADIUS
        && local.y <= col.bounding_box.max.y + PED_RADIUS;
    if !overlaps_vehicle {
        return position;
    }
    if matches!(door, 3 | 5) {
        local.x = col.bounding_box.max.x + PED_RADIUS;
    } else {
        local.x = col.bounding_box.min.x - PED_RADIUS;
    }
    ground_ped_exit_position(file_loader, matrix.transform_point(local))
}

pub(crate) fn ground_ped_exit_position(
    file_loader: Option<&CFileLoader>,
    position: CVector,
) -> CVector {
    let Some(file_loader) = file_loader else {
        return position;
    };
    let placed = crate::peds::PedPlacement::CPedPlacement::find_z_coor_for_ped(
        position,
        &file_loader.world,
        &file_loader.model_info,
    );
    if placed.found {
        placed.position
    } else {
        position
    }
}

/// re3 `CAutomobile::ProcessSwingingDoor` is called while a live vehicle is
/// rendered. Script-owned automobiles have no RW object to mutate, so advance
/// the same CDoor state in ECS before the renderer snapshots it.
pub(crate) fn process_script_vehicle_doors(mut script_entities: ScriptEntityWorld) {
    for vehicle in script_entities.vehicles.values_mut() {
        let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = vehicle else {
            continue;
        };
        for door in 0..crate::vehicles::Automobile::NUM_CAR_DOORS {
            if car.damage.get_door_status(door as u8)
                == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
            {
                car.process_door(door);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PedAutomobileProfile {
    is_van: bool,
    is_bus: bool,
    is_low: bool,
}

impl PedAutomobileProfile {
    pub const fn from_flags(is_van: bool, is_bus: bool, is_low: bool) -> Self {
        Self {
            is_van,
            is_bus,
            is_low,
        }
    }

    pub fn from_car<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>) -> Self
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
        Self {
            is_van: car.vehicle.core.flags.is_van,
            is_bus: car.vehicle.core.flags.is_bus,
            is_low: car.vehicle.core.flags.low_vehicle,
        }
    }

    fn rear_van_door(self, door: usize) -> bool {
        self.is_van && matches!(door, 4 | 5)
    }

    pub fn open_animation(self, door: usize) -> i32 {
        use crate::animation::AnimationId::*;
        let rhs = matches!(door, 3 | 5);
        match (self.rear_van_door(door), self.is_bus, rhs) {
            (true, _, true) => ANIM_STD_VAN_OPEN_DOOR_REAR_RHS,
            (true, _, false) => ANIM_STD_VAN_OPEN_DOOR_REAR_LHS,
            (_, true, true) => ANIM_STD_COACH_OPEN_RHS,
            (_, true, false) => ANIM_STD_COACH_OPEN_LHS,
            (_, _, true) => ANIM_STD_CAR_OPEN_DOOR_RHS,
            _ => ANIM_STD_CAR_OPEN_DOOR_LHS,
        }
    }

    pub fn locked_animation(self, door: usize) -> i32 {
        if matches!(door, 3 | 5) {
            crate::animation::AnimationId::ANIM_STD_CARDOOR_LOCKED_RHS
        } else {
            crate::animation::AnimationId::ANIM_STD_CARDOOR_LOCKED_LHS
        }
    }

    pub fn get_in_animation(self, door: usize) -> i32 {
        use crate::animation::AnimationId::*;
        let rhs = matches!(door, 3 | 5);
        match (self.rear_van_door(door), self.is_bus, self.is_low, rhs) {
            (true, _, _, true) => ANIM_STD_VAN_GET_IN_REAR_RHS,
            (true, _, _, false) => ANIM_STD_VAN_GET_IN_REAR_LHS,
            (_, true, _, true) => ANIM_STD_COACH_GET_IN_RHS,
            (_, true, _, false) => ANIM_STD_COACH_GET_IN_LHS,
            (_, _, true, true) => ANIM_STD_CAR_GET_IN_LO_RHS,
            (_, _, true, false) => ANIM_STD_CAR_GET_IN_LO_LHS,
            (_, _, _, true) => ANIM_STD_CAR_GET_IN_RHS,
            _ => ANIM_STD_CAR_GET_IN_LHS,
        }
    }

    pub fn pull_out_animation(self, door: usize) -> i32 {
        use crate::animation::AnimationId::*;
        match (self.is_low, matches!(door, 3 | 5)) {
            (true, true) => ANIM_STD_CAR_PULL_OUT_PED_LO_RHS,
            (true, false) => ANIM_STD_CAR_PULL_OUT_PED_LO_LHS,
            (false, true) => ANIM_STD_CAR_PULL_OUT_PED_RHS,
            (false, false) => ANIM_STD_CAR_PULL_OUT_PED_LHS,
        }
    }

    pub fn dragged_out_animation(self, door: usize, quick_jack: bool) -> i32 {
        use crate::animation::AnimationId::*;
        if quick_jack {
            return ANIM_STD_QUICKJACKED;
        }
        match (self.is_low, matches!(door, 3 | 5)) {
            (true, true) => ANIM_STD_JACKEDCAR_LO_RHS,
            (true, false) => ANIM_STD_JACKEDCAR_LO_LHS,
            (false, true) => ANIM_STD_JACKEDCAR_RHS,
            (false, false) => ANIM_STD_JACKEDCAR_LHS,
        }
    }

    pub fn close_enter_animation(self, door: usize) -> i32 {
        use crate::animation::AnimationId::*;
        let rhs = matches!(door, 3 | 5);
        match (self.rear_van_door(door), self.is_low, rhs) {
            (true, _, true) => ANIM_STD_VAN_CLOSE_DOOR_REAR_RHS,
            (true, _, false) => ANIM_STD_VAN_CLOSE_DOOR_REAR_LHS,
            (_, true, true) => ANIM_STD_CAR_CLOSE_DOOR_LO_RHS,
            (_, true, false) => ANIM_STD_CAR_CLOSE_DOOR_LO_LHS,
            (_, _, true) => ANIM_STD_CAR_CLOSE_DOOR_RHS,
            _ => ANIM_STD_CAR_CLOSE_DOOR_LHS,
        }
    }

    pub fn shuffle_animation(self) -> i32 {
        if self.is_low {
            crate::animation::AnimationId::ANIM_STD_CAR_SHUFFLE_LO_RHS
        } else {
            crate::animation::AnimationId::ANIM_STD_CAR_SHUFFLE_RHS
        }
    }

    pub fn exit_animation(self, door: usize) -> i32 {
        use crate::animation::AnimationId::*;
        let rhs = matches!(door, 3 | 5);
        match (self.is_bus, self.rear_van_door(door), self.is_low, rhs) {
            (true, _, _, _) => ANIM_STD_COACH_GET_OUT_LHS,
            (_, true, _, true) => ANIM_STD_VAN_GET_OUT_REAR_RHS,
            (_, true, _, false) => ANIM_STD_VAN_GET_OUT_REAR_LHS,
            (_, _, true, true) => ANIM_STD_GETOUT_LO_RHS,
            (_, _, true, false) => ANIM_STD_GETOUT_LO_LHS,
            (_, _, _, true) => ANIM_STD_GETOUT_RHS,
            _ => ANIM_STD_GETOUT_LHS,
        }
    }

    pub fn close_exit_animation(self, door: usize) -> i32 {
        use crate::animation::AnimationId::{ANIM_STD_CAR_CLOSE_LHS, ANIM_STD_CAR_CLOSE_RHS};
        if matches!(door, 3 | 5) {
            ANIM_STD_CAR_CLOSE_RHS
        } else {
            ANIM_STD_CAR_CLOSE_LHS
        }
    }
}

fn finish_enter_door_close<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>, door: usize)
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    if !car.vehicle.core.flags.is_bus {
        car.process_open_door(
            door,
            crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_LHS,
            1.0,
        );
    }
    if car.damage.get_door_status(door as u8)
        == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
    {
        car.damage.set_door_status(
            door as u8,
            crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_OK as u32,
        );
    }
}

fn vehicle_seat_at_door(door: usize) -> Option<VehicleSeat> {
    match door {
        2 => Some(VehicleSeat::Driver),
        3 => Some(VehicleSeat::Passenger(0)),
        4 => Some(VehicleSeat::Passenger(1)),
        5 => Some(VehicleSeat::Passenger(2)),
        _ => None,
    }
}

fn script_occupant_at_door(
    world: &ScriptEntityWorld,
    vehicle_handle: i32,
    door: usize,
) -> Option<i32> {
    let seat = vehicle_seat_at_door(door)?;
    let vehicle = world.vehicle_entity(vehicle_handle)?;
    world
        .script_occupant_handles(vehicle)
        .find_map(|(handle, occupied_seat)| (occupied_seat == seat).then_some(handle))
}

fn script_driver_handle(world: &ScriptEntityWorld, vehicle_handle: i32) -> Option<i32> {
    let vehicle = world.vehicle_entity(vehicle_handle)?;
    world
        .script_occupant_handles(vehicle)
        .find_map(|(handle, seat)| (seat == VehicleSeat::Driver).then_some(handle))
}

fn start_script_ped_dragged_out(
    world: &mut ScriptEntityWorld,
    manager: &CAnimManager,
    vehicle_handle: i32,
    door: usize,
    ped_handle: i32,
    quick_jack: bool,
) -> bool {
    let Some(vehicle_entity) = world.vehicle_entity(vehicle_handle) else {
        warn!(target: "re3bevy::ped_vehicle", vehicle_handle, "carjack vehicle entity disappeared");
        return false;
    };
    let Some(ped_entity) = world.ped_entity(ped_handle) else {
        warn!(target: "re3bevy::ped_vehicle", ped_handle, "carjack occupant entity disappeared");
        return false;
    };
    let Some(profile) = world.vehicles.get(&world.index, &vehicle_handle)
        .and_then(|vehicle| match vehicle {
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                Some(PedAutomobileProfile::from_car(&car))
            }
            _ => None,
        })
    else {
        warn!(target: "re3bevy::ped_vehicle", vehicle_handle, "carjack target is not an automobile");
        return false;
    };
    let draggable = world.peds.get(&world.index, &ped_handle).is_some_and(|ped| {
        ped.ped.ped_state == PedState::PED_DRIVING
            && (quick_jack || !ped.ped.flags.dont_drag_me_out_car)
    });
    if !draggable {
        return false;
    }
    let animation = profile.dragged_out_animation(door, quick_jack);
    if !queue_ped_vehicle_animation(
        &mut world.commands,
        manager,
        ped_entity,
        vehicle_entity,
        door,
        animation,
        PedVehicleAnimationPhase::DraggedOut,
        false,
    ) {
        return false;
    }
    if let Some(ped) = world.peds.get_mut(&world.index, &ped_handle) {
        ped.ped.last_ped_state = PedState::PED_IDLE;
        ped.ped.ped_state = PedState::PED_DRAG_FROM_CAR;
        ped.ped.move_state = eMoveState::PEDMOVE_NONE;
        ped.ped.physical.entity.flags.uses_collision = false;
        ped.ped.physical.move_speed = CVector::ZERO;
        ped.ped.seek_target = Some(CPoolHandle(vehicle_handle).index());
        ped.ped.veh_door = door as i32;
        ped.ped.flags.changed_seat = false;
        ped.ped.flags.will_be_quick_jacked = quick_jack;
        queue_ped_script_say(&mut *ped.ped, SOUND_PED_CAR_JACKED as i16);
    }
    if let Some(mut vehicle) = world.vehicles.get_mut(&world.index, &vehicle_handle) {
        let mut base = vehicle.base_mut();
        base.entry.getting_out_flags |= automobile_door_flag(door);
        if door == 2 {
            base.set_status(eEntityStatus::STATUS_ABANDONED);
        }
    }
    true
}

fn request_driver_exit_for_right_side(
    world: &mut ScriptEntityWorld,
    manager: &CAnimManager,
    ped_stats: &CPedStatsRuntime,
    players: &mut CWorldPlayers,
    player_index: i32,
    vehicle_handle: i32,
    now_ms: u32,
) {
    let Some(driver_handle) = script_driver_handle(world, vehicle_handle) else {
        return;
    };
    let vehicle_slot = CPoolHandle(vehicle_handle).index();
    let vehicle_created_by = world.vehicles.get(&world.index, &vehicle_handle)
        .map(|vehicle| vehicle.base().vehicle_created_by)
        .unwrap_or(MISSION_VEHICLE);
    let carjacker_slot = players
        .player(player_index)
        .map(|player| CPoolHandle(player.pool_handle).index());
    let mut retaliating_cop = false;
    if let Some(driver) = world.peds.get_mut(&world.index, &driver_handle) {
        if driver.ped.ped_state != PedState::PED_DRIVING {
            return;
        }
        driver.ped.flags.use_ped_node_seek = true;
        driver.ped.next_path_node = None;
        let stats = ped_stats.GetPedStats(driver.ped.ped_stats);
        if stats.m_temper <= stats.m_fear
            || driver.ped.char_created_by == MISSION_CHAR
            || vehicle_created_by == MISSION_VEHICLE
        {
            driver.ped.flags.flee_after_exiting_car = true;
        } else if let Some(carjacker_slot) = carjacker_slot {
            driver.ped.flags.gonna_kill_the_car_jacker = true;
            driver.ped.set_objective_entity(eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT,
                carjacker_slot,
                now_ms,
                Some(vehicle_slot),
            );
            retaliating_cop = driver.ped.ped_type == ePedType::PEDTYPE_COP;
        }
        driver.ped.veh_door = 2;
        driver.ped.set_objective_entity(eObjective::OBJECTIVE_LEAVE_CAR,
            vehicle_slot,
            now_ms,
            Some(vehicle_slot),
        );
        queue_ped_script_say(&mut *driver.ped, SOUND_PED_CAR_JACKED as i16);
    }
    if retaliating_cop && let Some(mut player) = players.player_mut(player_index) {
        player.wanted.SetWantedLevelNoDrop(1);
    }
    let (Some(driver_entity), Some(vehicle_entity)) = (
        world.ped_entity(driver_handle),
        world.vehicle_entity(vehicle_handle),
    ) else {
        warn!(
            target: "re3bevy::ped_vehicle",
            driver_handle,
            vehicle_handle,
            "right-side carjack could not resolve driver/vehicle entities"
        );
        return;
    };
    let Some(profile) = world.vehicles.get(&world.index, &vehicle_handle)
        .and_then(|vehicle| match vehicle {
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                Some(PedAutomobileProfile::from_car(&car))
            }
            _ => None,
        })
    else {
        return;
    };
    if queue_ped_vehicle_animation(
        &mut world.commands,
        manager,
        driver_entity,
        vehicle_entity,
        2,
        profile.exit_animation(2),
        PedVehicleAnimationPhase::Exit,
        false,
    ) {
        if let Some(driver) = world.peds.get_mut(&world.index, &driver_handle) {
            driver.ped.ped_state = PedState::PED_EXIT_CAR;
            driver.ped.physical.entity.flags.uses_collision = false;
        }
        if let Some(mut vehicle) = world.vehicles.get_mut(&world.index, &vehicle_handle) {
            vehicle.base_mut().entry.getting_out_flags |= automobile_door_flag(2);
        }
    }
}

pub fn automobile_door_flag(door: usize) -> i8 {
    door.checked_sub(2)
        .and_then(|shift| 1_i8.checked_shl(shift as u32))
        .unwrap_or(0)
}

/// re3 `GetComponentWorldPosition(CAR_DOOR_*)`, shared by production gameplay
/// and isolated tools. Every authored DFF can place its door dummy differently,
/// so callers must not replace this with a generic vehicle-width offset.
pub fn automobile_door_world_position<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    door: usize,
) -> Option<CVector>
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let component = door_to_debris_component(door)?;
    vehicle_component_world_matrix(
        file_loader,
        car.vehicle.core.physical.entity.model_index.max(0) as usize,
        car.vehicle.core.physical.entity.matrix,
        component,
    )
    .map(|matrix| matrix.position)
}

pub(crate) fn automobile(entity: crate::control::ScriptEntities::ScriptVehicleRef<'_>) -> Option<crate::vehicles::Automobile::AutomobileRef<'_>> {
    match entity {
        crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => Some(car),
        _ => None,
    }
}

pub(crate) fn automobile_mut(entity: crate::control::ScriptEntities::ScriptVehicleMut<'_>) -> Option<crate::vehicles::Automobile::AutomobileMut<'_>> {
    match entity {
        crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } => Some(car),
        _ => None,
    }
}

/// re3 `CPed::PedAnimStepOutCarCB` (`peds/PedAI.cpp:2936-3010`): decide whether
/// the ped shuts the door after stepping out.
///
/// re3 starts from `closeDoor = !veh->IsDoorMissing(door)` and then forces it
/// false when the car is a bus, is burning, has a destroyed engine, or — for a
/// player ped — when the player is holding any movement/target input while
/// controls are enabled and the doors are not locked shut.  GTA III therefore
/// *intentionally* leaves the door open and swinging when you step out while
/// steering; `PedAnimStepOutCarCB` writes `DOOR_STATUS_SWINGING` before handing
/// the ped to `PedSetOutCarCB`.
///
/// `door` is an `eDoors` index in 2..=5 (the Rust runtime already works in
/// `eDoors`, so the C++ `m_vehDoor` -> `eDoors` switch is unnecessary).
pub(crate) fn automobile_close_door_after_exit<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    door: usize,
    pad: Option<&CPad>,
) -> bool
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    use crate::vehicles::DamageManager::eDoorStatus;
    if car.damage.get_door_status(door as u8) == eDoorStatus::DOOR_STATUS_MISSING as u32 {
        return false;
    }
    // `engineIsIntact` in re3 is really "engine on fire": the damage engine
    // status reaches 225 exactly when the vehicle starts burning.
    if car.vehicle.core.flags.is_bus
        || car.vehicle.vitality.car_fire.is_some()
        || car.damage.get_engine_status() >= 225
    {
        return false;
    }
    let Some(pad) = pad else {
        return true;
    };
    // `CPad::GetTarget` (`core/Pad.cpp:2240`) is `NewState.RightShoulder1` in
    // the default control mode; it already returns false when controls are
    // disabled, so re3's `!ArePlayerControlsDisabled()` guard is preserved.
    let target = pad.new_state.right_shoulder1 != 0;
    let movement = pad.new_state.left_stick_x != 0
        || pad.new_state.left_stick_y != 0
        || pad.new_state.dpad_up != 0
        || pad.new_state.dpad_down != 0
        || pad.new_state.dpad_left != 0
        || pad.new_state.dpad_right != 0;
    if !pad.disable_player_controls
        && car.vehicle.entry.door_lock != eCarLock::CARLOCK_FORCE_SHUT_DOORS
        && (target || movement)
    {
        return false;
    }
    true
}

/// Player-owned counterpart of CPed's persistent `m_pVehicleAnim` chain. Both
/// script peds and players keep this timeline on their Bevy ped entity; only
/// the callback/control branches differ.
pub(crate) fn process_player_vehicle_animation(
    mut active_animations: Query<&mut CPedVehicleAnimation, With<PedEntity>>,
    manager: Res<CAnimManager>,
    mut players: CWorldPlayers,
    mut script_entities: ScriptEntityWorld,
    file_loader: Res<CFileLoader>,
    handling_manager: Res<cHandlingDataMgr>,
    ped_animation_offsets: Res<CPedAnimationOffsets>,
    ped_stats: Res<CPedStatsRuntime>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    pads: Option<Res<CPads>>,
) {
    let player_index = players.player_in_focus() as i32;
    let player_seated = players.relation(player_index).is_some();
    let Some(player_entity) = players.player_entity(player_index) else {
        return;
    };
    // re3 CPed::SetDead clears m_pVehicleAnim, and CPlayerPed::ProcessControl
    // returns immediately for PED_DIE/PED_DEAD.  Do this before advancing the
    // association so a death cannot still commit a vehicle callback.
    if players
        .player(player_index)
        .is_some_and(|player| player.ped.dying_or_dead())
    {
        script_entities
            .commands
            .entity(player_entity)
            .remove::<CPedVehicleAnimation>();
        return;
    }
    let mut finished = None;
    if let Ok(mut active) = active_animations.get_mut(player_entity) {
        let vehicle_handle = script_entities.vehicle_handle_for_entity(active.vehicle);
        active
            .association
            .update_time(timer.time_step_in_seconds(), 1.0);
        active
            .association
            .update_blend(timer.time_step_in_seconds());
        let vehicle_exists = match vehicle_handle
            .and_then(|vehicle_handle| script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle))
        {
            Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) => {
                if car.vehicle.core.physical.audio_entity_id < 0
                    && let Some(audio) = dmaudio.as_deref_mut()
                    && let Some(vehicle_handle) = vehicle_handle
                {
                    let entity = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(vehicle_handle));
                    car.vehicle.core.physical.audio_entity_id = entity;
                    if entity >= 0 {
                        audio.SetEntityStatus(entity, true);
                    }
                }
                let was_closed = car.doors[active.door].is_closed();
                car.process_open_door(
                    active.door,
                    active.association.anim_id,
                    active.association.current_time,
                );
                let is_closed = car.doors[active.door].is_closed();
                if was_closed != is_closed
                    && car.vehicle.core.physical.audio_entity_id >= 0
                    && let Some(audio) = dmaudio.as_deref_mut()
                {
                    let sound = if is_closed {
                        SOUND_CAR_DOOR_CLOSE_BONNET
                    } else {
                        SOUND_CAR_DOOR_OPEN_BONNET
                    } + active.door as i32;
                    audio.PlayOneShot(car.vehicle.core.physical.audio_entity_id, sound as u16, 0.0);
                }
                true
            }
            _ => false,
        };
        if let Some(player) = players.player_mut(player_index) {
            if let Some(vehicle_handle) = vehicle_handle
                && let Some(crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. }) =
                    script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                && let Some(handling) = car
                    .vehicle.core.handling_id
                    .and_then(|id| usize::try_from(id).ok())
                    .and_then(|id| handling_manager.get_handling_data(id))
            {
                if let Some(line_up_result) = automobile_ped_line_up(
                    &file_loader,
                    &car,
                    handling,
                    &ped_animation_offsets,
                    active.door,
                    PedVehicleLineUpContext {
                        current_position: player.position(),
                        current_heading: player.locomotion.rotation_cur,
                        offset_seek: player.ped.offset_seek,
                        state_timer_ms: player.ped.ped_state_timer,
                        now_ms: timer.time_in_milliseconds(),
                    },
                    active.phase,
                    &active.association,
                    player.ped.physical.move_speed.z,
                    timer.time_step(),
                ) && (!player_seated
                    || matches!(
                        active.phase,
                        PedVehicleAnimationPhase::Exit | PedVehicleAnimationPhase::CloseExit
                    ))
                {
                    player.ped.physical.entity.set_matrix(line_up_result.matrix);
                    player.ped.physical.move_speed.z = line_up_result.vertical_speed_z;
                    let heading = line_up_result
                        .matrix
                        .right
                        .y
                        .atan2(line_up_result.matrix.right.x);
                    player.locomotion.rotation_dest = heading;
                    player.locomotion.rotation_cur = heading;
                }
            }
        }
        if !vehicle_exists || !active.association.is_running() || active.association.deleted {
            finished = Some((
                active.vehicle,
                vehicle_handle,
                active.door,
                active.phase,
                active.enter_as_driver,
            ));
        }
    }
    let Some((vehicle_entity, vehicle_handle, door, phase, enter_as_driver)) = finished else {
        return;
    };
    script_entities
        .commands
        .entity(player_entity)
        .remove::<CPedVehicleAnimation>();
    let Some(vehicle_handle) = vehicle_handle else {
        return;
    };
    let Some((profile, door_missing, door_fully_open, can_open_locks, _engine_on_fire)) =
        script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
            .and_then(|vehicle| match vehicle {
                crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => Some((
                    PedAutomobileProfile::from_car(&car),
                    automobile_door_missing(&car, door),
                    car.doors.get(door).is_some_and(|door| door.is_fully_open()),
                    car.vehicle.can_ped_open_locks(true),
                    car.damage.get_engine_status() >= 225,
                )),
                _ => None,
            })
    else {
        return;
    };
    match phase {
        PedVehicleAnimationPhase::LockedDoor => {
            abort_player_enter_vehicle(
                &mut players,
                &mut script_entities,
                player_index,
                vehicle_handle,
                door,
            );
        }
        PedVehicleAnimationPhase::Align => {
            let carjacking = players
                .player(player_index)
                .is_some_and(|player| player.ped.ped_state == PedState::PED_CARJACK);
            if carjacking && !profile.is_bus && can_open_locks {
                let victim = script_occupant_at_door(&script_entities, vehicle_handle, door);
                if (door_missing || door_fully_open)
                    && let Some(victim_handle) = victim
                {
                    if start_script_ped_dragged_out(
                        &mut script_entities,
                        &manager,
                        vehicle_handle,
                        door,
                        victim_handle,
                        false,
                    ) {
                        let anim = profile.pull_out_animation(door);
                        if queue_ped_vehicle_animation(
                            &mut script_entities.commands,
                            &manager,
                            player_entity,
                            vehicle_entity,
                            door,
                            anim,
                            PedVehicleAnimationPhase::PullOut,
                            true,
                        ) {
                            return;
                        }
                    }
                    abort_player_enter_vehicle(
                        &mut players,
                        &mut script_entities,
                        player_index,
                        vehicle_handle,
                        door,
                    );
                    return;
                }

                let quick_jack = door == 2
                    && !profile.is_low
                    && victim.is_some_and(|victim_handle| {
                        script_entities.peds.get(&script_entities.index, &victim_handle)
                            .is_some_and(|victim| {
                                victim.ped.char_created_by != MISSION_CHAR
                                    && victim.ped.ped_state == PedState::PED_DRIVING
                            })
                    });
                if quick_jack
                    && let Some(victim_handle) = victim
                    && start_script_ped_dragged_out(
                        &mut script_entities,
                        &manager,
                        vehicle_handle,
                        door,
                        victim_handle,
                        true,
                    )
                {
                    if queue_ped_vehicle_animation(
                        &mut script_entities.commands,
                        &manager,
                        player_entity,
                        vehicle_entity,
                        door,
                        crate::animation::AnimationId::ANIM_STD_QUICKJACK,
                        PedVehicleAnimationPhase::GetIn,
                        true,
                    ) {
                        return;
                    }
                    abort_player_enter_vehicle(
                        &mut players,
                        &mut script_entities,
                        player_index,
                        vehicle_handle,
                        door,
                    );
                    return;
                }
            }
            // re3 `PedAnimAlignCB`: missing doors bypass the open-door
            // animation and begin the get-in animation immediately.
            let (anim, next_phase) = if door_missing || door_fully_open {
                if carjacking {
                    if let Some(player) = players.player_mut(player_index) {
                        player.ped.ped_state = PedState::PED_ENTER_CAR;
                    }
                    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
                        vehicle.base_mut().entry.is_being_car_jacked = false;
                    }
                }
                (
                    profile.get_in_animation(door),
                    PedVehicleAnimationPhase::GetIn,
                )
            } else if !can_open_locks {
                if let Some(player) = players.player_mut(player_index) {
                    player.ped.flags.cancel_entering_car = true;
                }
                (
                    profile.locked_animation(door),
                    PedVehicleAnimationPhase::LockedDoor,
                )
            } else {
                (
                    profile.open_animation(door),
                    PedVehicleAnimationPhase::OpenDoor,
                )
            };
            if !queue_ped_vehicle_animation(
                &mut script_entities.commands,
                &manager,
                player_entity,
                vehicle_entity,
                door,
                anim,
                next_phase,
                enter_as_driver,
            ) {
                abort_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    player_index,
                    vehicle_handle,
                    door,
                );
            }
        }
        PedVehicleAnimationPhase::OpenDoor => {
            let vehicle_moving_too_fast = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                .is_some_and(|vehicle| vehicle.base().physical.move_speed.magnitude() > 0.2);
            if vehicle_moving_too_fast {
                abort_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    player_index,
                    vehicle_handle,
                    door,
                );
                let impact = if matches!(door, 3 | 5) {
                    crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_LEFT
                } else {
                    crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_RIGHT
                };
                if let Some(player) = players.player_mut(player_index) {
                    let _ = player.ped.set_fall(&player.vitality, 
                        1_000,
                        impact,
                        false,
                        timer.time_in_milliseconds(),
                        timer.frame_counter(),);
                }
                return;
            }
            let carjacking = players
                .player(player_index)
                .is_some_and(|player| player.ped.ped_state == PedState::PED_CARJACK);
            if carjacking && !profile.is_bus {
                let Some(victim_handle) =
                    script_occupant_at_door(&script_entities, vehicle_handle, door)
                else {
                    if let Some(player) = players.player_mut(player_index) {
                        player.ped.ped_state = PedState::PED_ENTER_CAR;
                    }
                    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
                        vehicle.base_mut().entry.is_being_car_jacked = false;
                    }
                    let anim = profile.get_in_animation(door);
                    if !queue_ped_vehicle_animation(
                        &mut script_entities.commands,
                        &manager,
                        player_entity,
                        vehicle_entity,
                        door,
                        anim,
                        PedVehicleAnimationPhase::GetIn,
                        enter_as_driver,
                    ) {
                        abort_player_enter_vehicle(
                            &mut players,
                            &mut script_entities,
                            player_index,
                            vehicle_handle,
                            door,
                        );
                    }
                    return;
                };
                if !start_script_ped_dragged_out(
                    &mut script_entities,
                    &manager,
                    vehicle_handle,
                    door,
                    victim_handle,
                    false,
                ) {
                    abort_player_enter_vehicle(
                        &mut players,
                        &mut script_entities,
                        player_index,
                        vehicle_handle,
                        door,
                    );
                    return;
                }
                let anim = profile.pull_out_animation(door);
                if !queue_ped_vehicle_animation(
                    &mut script_entities.commands,
                    &manager,
                    player_entity,
                    vehicle_entity,
                    door,
                    anim,
                    PedVehicleAnimationPhase::PullOut,
                    true,
                ) {
                    abort_player_enter_vehicle(
                        &mut players,
                        &mut script_entities,
                        player_index,
                        vehicle_handle,
                        door,
                    );
                }
                return;
            }
            let anim = profile.get_in_animation(door);
            if !queue_ped_vehicle_animation(
                &mut script_entities.commands,
                &manager,
                player_entity,
                vehicle_entity,
                door,
                anim,
                PedVehicleAnimationPhase::GetIn,
                enter_as_driver,
            ) {
                abort_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    player_index,
                    vehicle_handle,
                    door,
                );
            }
        }
        PedVehicleAnimationPhase::PullOut => {
            let anim = profile.get_in_animation(door);
            if !queue_ped_vehicle_animation(
                &mut script_entities.commands,
                &manager,
                player_entity,
                vehicle_entity,
                door,
                anim,
                PedVehicleAnimationPhase::GetIn,
                enter_as_driver,
            ) {
                abort_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    player_index,
                    vehicle_handle,
                    door,
                );
            }
        }
        PedVehicleAnimationPhase::DraggedOut => {}
        PedVehicleAnimationPhase::GetIn => {
            if enter_as_driver && door != 2 {
                request_driver_exit_for_right_side(
                    &mut script_entities,
                    &manager,
                    &ped_stats,
                    &mut players,
                    player_index,
                    vehicle_handle,
                    timer.time_in_milliseconds(),
                );
            }
            // re3 `CPed::PedAnimGetInCB` (PedAI.cpp:2631-2639): flooring full
            // accelerate/brake while entering via the left-front door skips
            // the close animation; the door stays SWINGING and the rolling
            // close shuts it once driving input pauses.
            let floored = pads.as_deref().is_some_and(|pads| {
                let pad = pads.get_pad(0);
                pad.get_accelerate() >= 255 || pad.get_brake() >= 255
            });
            if door == 2
                && floored
                && let Some(crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. }) =
                    script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                && !automobile_door_missing(&car, door)
            {
                if let Some(mut car) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
                    .and_then(automobile_mut)
                {
                    car.damage.set_door_status(
                        door as u8,
                        crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32,
                    );
                }
                let engine_was_off = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                    .is_some_and(|v| !v.base().drive.engine_on);
                let entered = complete_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    &file_loader,
                    &mut general,
                    timer.time_in_milliseconds(),
                    player_index,
                    vehicle_handle,
                    door,
                    enter_as_driver,
                    false,
                );
                if !entered {
                    abort_player_enter_vehicle(
                        &mut players,
                        &mut script_entities,
                        player_index,
                        vehicle_handle,
                        door,
                    );
                    return;
                }
                if enter_as_driver && engine_was_off {
                    if let Some(audio) = dmaudio.as_deref_mut() {
                        if let Some(player) = players.player(player_index) {
                            audio.PlayOneShot(
                                player.ped.physical.audio_entity_id,
                                crate::audio::soundlist::SOUND_CAR_ENGINE_START as u16,
                                1.0,
                            );
                        }
                    }
                }
                return;
            }
            if profile.is_bus {
                let engine_was_off = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                    .is_some_and(|v| !v.base().drive.engine_on);
                let entered = complete_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    &file_loader,
                    &mut general,
                    timer.time_in_milliseconds(),
                    player_index,
                    vehicle_handle,
                    door,
                    enter_as_driver,
                    true,
                );
                if !entered {
                    abort_player_enter_vehicle(
                        &mut players,
                        &mut script_entities,
                        player_index,
                        vehicle_handle,
                        door,
                    );
                    return;
                }
                if enter_as_driver && engine_was_off {
                    if let Some(audio) = dmaudio.as_deref_mut() {
                        if let Some(player) = players.player(player_index) {
                            audio.PlayOneShot(
                                player.ped.physical.audio_entity_id,
                                crate::audio::soundlist::SOUND_CAR_ENGINE_START as u16,
                                1.0,
                            );
                        }
                    }
                }
                return;
            }
            if door_missing {
                if enter_as_driver && door == 3 {
                    let anim = profile.shuffle_animation();
                    if queue_ped_vehicle_animation(
                        &mut script_entities.commands,
                        &manager,
                        player_entity,
                        vehicle_entity,
                        door,
                        anim,
                        PedVehicleAnimationPhase::Shuffle,
                        true,
                    ) {
                        return;
                    }
                }
                let engine_was_off = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                    .is_some_and(|v| !v.base().drive.engine_on);
                let entered = complete_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    &file_loader,
                    &mut general,
                    timer.time_in_milliseconds(),
                    player_index,
                    vehicle_handle,
                    door,
                    enter_as_driver,
                    false,
                );
                if !entered {
                    abort_player_enter_vehicle(
                        &mut players,
                        &mut script_entities,
                        player_index,
                        vehicle_handle,
                        door,
                    );
                    return;
                }
                if enter_as_driver && engine_was_off {
                    if let Some(audio) = dmaudio.as_deref_mut() {
                        if let Some(player) = players.player(player_index) {
                            audio.PlayOneShot(
                                player.ped.physical.audio_entity_id,
                                crate::audio::soundlist::SOUND_CAR_ENGINE_START as u16,
                                1.0,
                            );
                        }
                    }
                }
                return;
            }
            let anim = profile.close_enter_animation(door);
            if !queue_ped_vehicle_animation(
                &mut script_entities.commands,
                &manager,
                player_entity,
                vehicle_entity,
                door,
                anim,
                PedVehicleAnimationPhase::CloseEnter,
                enter_as_driver,
            ) {
                warn!("player close-enter animation {anim} is unavailable");
                abort_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    player_index,
                    vehicle_handle,
                    door,
                );
            }
        }
        PedVehicleAnimationPhase::CloseEnter => {
            if enter_as_driver && door == 3 {
                if let Some(mut car) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
                    .and_then(automobile_mut)
                {
                    finish_enter_door_close(&mut car, door);
                }
                let anim = profile.shuffle_animation();
                if queue_ped_vehicle_animation(
                    &mut script_entities.commands,
                    &manager,
                    player_entity,
                    vehicle_entity,
                    door,
                    anim,
                    PedVehicleAnimationPhase::Shuffle,
                    true,
                ) {
                    return;
                }
            }
            let engine_was_off = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                .is_some_and(|v| !v.base().drive.engine_on);
            let entered = complete_player_enter_vehicle(
                &mut players,
                &mut script_entities,
                &file_loader,
                &mut general,
                timer.time_in_milliseconds(),
                player_index,
                vehicle_handle,
                door,
                enter_as_driver,
                true,
            );
            if !entered {
                abort_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    player_index,
                    vehicle_handle,
                    door,
                );
                return;
            }
            if enter_as_driver && engine_was_off {
                if let Some(audio) = dmaudio.as_deref_mut() {
                    if let Some(player) = players.player(player_index) {
                        audio.PlayOneShot(
                            player.ped.physical.audio_entity_id,
                            crate::audio::soundlist::SOUND_CAR_ENGINE_START as u16,
                            1.0,
                        );
                    }
                }
            }
        }
        PedVehicleAnimationPhase::Shuffle => {
            let engine_was_off = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                .is_some_and(|v| !v.base().drive.engine_on);
            let entered = complete_player_enter_vehicle(
                &mut players,
                &mut script_entities,
                &file_loader,
                &mut general,
                timer.time_in_milliseconds(),
                player_index,
                vehicle_handle,
                door,
                enter_as_driver,
                false,
            );
            if !entered {
                abort_player_enter_vehicle(
                    &mut players,
                    &mut script_entities,
                    player_index,
                    vehicle_handle,
                    door,
                );
                return;
            }
            if enter_as_driver && engine_was_off {
                if let Some(audio) = dmaudio.as_deref_mut() {
                    if let Some(player) = players.player(player_index) {
                        audio.PlayOneShot(
                            player.ped.physical.audio_entity_id,
                            crate::audio::soundlist::SOUND_CAR_ENGINE_START as u16,
                            1.0,
                        );
                    }
                }
            }
        }
        PedVehicleAnimationPhase::Exit => {
            if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
                vehicle
                    .base_mut()
                    .set_status(eEntityStatus::STATUS_ABANDONED);
            }
            // re3 `CPed::PedAnimStepOutCarCB` decides here whether the ped
            // shuts the door at all; the checks above are the subset re3
            // evaluates before the pad-state branch.
            let close_door = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                .and_then(automobile)
                .is_some_and(|car| {
                    automobile_close_door_after_exit(
                        &car,
                        door,
                        pads.as_deref().map(|pads| pads.get_pad(0)),
                    )
                });
            if !close_door {
                // re3 leaves the door swinging instead of animating it shut.
                if !door_missing && !profile.is_bus {
                    if let Some(mut car) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
                        .and_then(automobile_mut)
                    {
                        car.damage.set_door_status(
                            door as u8,
                            crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING
                                as u32,
                        );
                    }
                }
                complete_player_exit_vehicle(
                    &mut players,
                    &mut script_entities,
                    &file_loader,
                    player_index,
                    vehicle_handle,
                    door,
                );
                return;
            }
            let anim = profile.close_exit_animation(door);
            if !queue_ped_vehicle_animation(
                &mut script_entities.commands,
                &manager,
                player_entity,
                vehicle_entity,
                door,
                anim,
                PedVehicleAnimationPhase::CloseExit,
                false,
            ) {
                warn!("player close-exit animation {anim} is unavailable");
                complete_player_exit_vehicle(
                    &mut players,
                    &mut script_entities,
                    &file_loader,
                    player_index,
                    vehicle_handle,
                    door,
                );
            }
        }
        PedVehicleAnimationPhase::CloseExit => {
            if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
            {
                car.open_door(door, 0.0);
            }
            complete_player_exit_vehicle(
                &mut players,
                &mut script_entities,
                &file_loader,
                player_index,
                vehicle_handle,
                door,
            );
        }
        PedVehicleAnimationPhase::RollingDoorClose => {
            // `CPed::PedAnimDoorCloseRollingCB` (PedAI.cpp:5141-5160)
            // hard-closes the authored left-front hinge, releases the
            // getting-out flag and turns a swinging door back to OK.  This
            // callback is deliberately separate from CloseExit: the player
            // remains seated and keeps the vehicle control state.
            if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
            {
                let animation = if car.vehicle.core.flags.low_vehicle {
                    crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LO_LHS
                } else {
                    crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS
                };
                car.process_open_door(door, animation, 1.0);
                car.vehicle.entry.getting_out_flags &= !automobile_door_flag(door);
                if car.damage.get_door_status(door as u8)
                    == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
                {
                    car.damage.set_door_status(
                        door as u8,
                        crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_OK as u32,
                    );
                }
            }
        }
        PedVehicleAnimationPhase::External => {}
    }
}

pub(crate) fn complete_player_enter_vehicle(
    players: &mut CWorldPlayers,
    script_entities: &mut ScriptEntityWorld,
    file_loader: &CFileLoader,
    _general: &mut CGeneral,
    _now_ms: u32,
    player_index: i32,
    vehicle_handle: i32,
    door: usize,
    enter_as_driver: bool,
    close_door: bool,
) -> bool {
    // re3 `CPed::PedAnimDoorCloseCB` (PedAI.cpp:2751-2764) finishes CloseEnter
    // with a hard hinge close plus SWINGING→OK, because the authored close
    // window (0.2-0.63 of `CAR_CLOSE_DOOR_LHS`) ends after the real 0.467s
    // PED.IFP animation: without this the door is left visibly ajar. The
    // floored-gas quick entry (`PedAnimGetInCB`, PedAI.cpp:2631-2639) skips
    // this and leaves the door SWINGING for the rolling close instead.
    if close_door
        && let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
            script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
    {
        if !car.vehicle.core.flags.is_bus {
            car.process_open_door(
                door,
                crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_LHS,
                1.0,
            );
        }
        if car.damage.get_door_status(door as u8)
            == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
        {
            car.damage.set_door_status(
                door as u8,
                crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_OK as u32,
            );
        }
    }
    let Some(player_entity) = players.player_entity(player_index) else {
        return false;
    };
    let seat = if enter_as_driver {
        VehicleSeat::Driver
    } else {
        VehicleSeat::Passenger(door.saturating_sub(3).min(2) as u8)
    };
    // re3 `CVehicle::SetDriver` overwrites `pDriver` immediately. Quick-jack
    // clips are authored so the carjacker can finish first, while the old
    // driver's `PedSetDraggedOutCarCB` is still pending. Preserve that exact
    // handoff without weakening the general ECS seat invariant.
    let replaced_dragged_driver = enter_as_driver
        .then(|| script_driver_handle(script_entities, vehicle_handle))
        .flatten()
        .filter(|driver_handle| {
            script_entities
                .ped_get(*driver_handle)
                .is_some_and(|driver| driver.ped.ped_state == PedState::PED_DRAG_FROM_CAR)
        })
        .and_then(|driver_handle| script_entities.ped_entity(driver_handle));
    let entered = players.player_mut(player_index).is_some_and(|mut player| {
        script_entities.place_player_in_car_replacing_occupant(
            player_entity,
            &mut player,
            vehicle_handle,
            seat,
            file_loader,
            replaced_dragged_driver,
        )
    });
    if !entered {
        warn!("player enter-car commit failed for vehicle {vehicle_handle}");
        return false;
    }
    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
        let mut base = vehicle.base_mut();
        base.entry.num_getting_in = base.entry.num_getting_in.saturating_sub(1);
        base.entry.getting_in_flags &= !automobile_door_flag(door);
        base.flags.has_been_owned_by_player = true;
        if enter_as_driver {
            base.entry.is_being_car_jacked = false;
            base.set_status(eEntityStatus::STATUS_PLAYER);
            base.drive.engine_on = true;
        }
    }
    if let Some(player) = players.player_mut(player_index) {
        player.ped.ped_state = PedState::PED_DRIVING;
        // re3 `CPed::PedSetInCarCB`: entering the objective vehicle publishes
        // `bScriptObjectiveCompleted` and restores the objective saved by the
        // entry request. Without this the intro mission waits forever on
        // `IS_CHAR_OBJECTIVE_PASSED` even though the player is seated.
        if matches!(
            player.ped.objective,
            eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
        ) {
            player.ped.flags.script_objective_completed = true;
            if matches!(
                player.ped.prev_objective,
                eObjective::OBJECTIVE_RUN_TO_AREA
                    | eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
                    | eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
            ) {
                player.ped.prev_objective = eObjective::OBJECTIVE_NONE;
            }
            player
                .ped
                .restore_previous_objective(Some(CPoolHandle(vehicle_handle).index()));
            if !matches!(
                player.ped.objective,
                eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
                    | eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT
                    | eObjective::OBJECTIVE_LEAVE_CAR
                    | eObjective::OBJECTIVE_FOLLOW_CAR_IN_CAR
                    | eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS
                    | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
                    | eObjective::OBJECTIVE_RUN_TO_AREA
            ) {
                player.ped.objective = eObjective::OBJECTIVE_NONE;
            }
        }
    }
    true
}

pub(crate) fn abort_player_enter_vehicle(
    players: &mut CWorldPlayers,
    script_entities: &mut ScriptEntityWorld,
    player_index: i32,
    vehicle_handle: i32,
    door: usize,
) {
    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
        let base = vehicle.base_mut();
        base.entry.num_getting_in = base.entry.num_getting_in.saturating_sub(1);
        base.entry.getting_in_flags &= !automobile_door_flag(door);
        base.entry.is_being_car_jacked = false;
    }
    if let Some(mut player) = players.player_mut(player_index) {
        player.ped.flags.cancel_entering_car = false;
        player.ped.seek_target = None;
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        player.ped.physical.entity.flags.uses_collision = true;
        player.ped.physical.move_speed = CVector::ZERO;
        player.move_speed = 0.0;
        player.ped.restore_previous_objective(None);
    }
}

pub(crate) fn complete_player_exit_vehicle(
    players: &mut CWorldPlayers,
    script_entities: &mut ScriptEntityWorld,
    file_loader: &CFileLoader,
    player_index: i32,
    vehicle_handle: i32,
    door: usize,
) {
    let vehicle_entity = script_entities.vehicle_entity(vehicle_handle);
    let was_driver = players.relation(player_index).is_some_and(|relation| {
        Some(relation.vehicle) == vehicle_entity && relation.seat == VehicleSeat::Driver
    });
    let _ = players.clear_relation(player_index);
    let Some(mut player) = players.player_mut(player_index) else {
        return;
    };

    let position =
        ground_ped_exit_position(Some(file_loader), player.ped.physical.entity.get_position());
    player.set_position(position);
    player.ped.physical.entity.flags.uses_collision = true;
    player.ped.ped_state = PedState::PED_IDLE;
    player.ped.move_state = eMoveState::PEDMOVE_STILL;
    player.ped.physical.move_speed = CVector::ZERO;
    player.move_speed = 0.0;
    player.ped.flags.is_in_the_air = false;
    player.ped.flags.is_standing = true;
    // re3 `PedSetOutCarCB` restores the objective saved by the leave-car
    // request before SCM can assign the next on-foot objective.
    if player.ped.objective == eObjective::OBJECTIVE_LEAVE_CAR {
        player.ped.restore_previous_objective(None);
    }

    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
        let mut base = vehicle.base_mut();
        base.entry.getting_out_flags &= !automobile_door_flag(door);
        if was_driver {
            finish_driver_removal(&mut base);
        }
    }
}

/// `CVehicle::RemoveDriver` and the driver-only lock branch of
/// `CPed::PedSetOutCarCB` / `PedSetDraggedOutCarCB`. The ECS caller removes
/// InVehicle; this function owns the vehicle-side release effects.
fn finish_driver_removal<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>)
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    if vehicle.physical.entity.status != eEntityStatus::STATUS_WRECKED {
        vehicle.set_status(eEntityStatus::STATUS_ABANDONED);
    }
    if vehicle.entry.door_lock == eCarLock::CARLOCK_LOCKED_INITIALLY {
        vehicle.entry.door_lock = eCarLock::CARLOCK_UNLOCKED;
    }
}
