//! Live ECS pool save adapters. Native layouts: Pools.cpp, Vehicle.cpp,
//! AutoPilot.cpp, Entity.cpp, Automobile.cpp, Boat.cpp and Matrix.cpp.

use bevy::prelude::*;
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};
use crate::{
    control::{AutoPilot::{eCarDrivingStyle, eCarTempAction, script_car_mission},
        ScriptEntities::{ScriptEntityWorld, ScriptVehicleEntity, ScriptObjectEntity, ScriptObjectIdentity, spawn_script_object}},
    core::{FileLoader::CFileLoader, Pools::*,},
    entities::Entity::{CEntity, script_entity_status},
    math::Matrix::CMatrix,
    modelinfo::{ModelInfo::ModelInfoSlot, VehicleModelInfo::choose_vehicle_optional_components},
    objects::Object::{CObject, MISSION_OBJECT},
    peds::VehicleRelations::InVehicle,
    vehicles::{Automobile::CAutomobile, Boat::{CBoat, CBoatHandlingSnapshot},
        DamageManager::CDamageManager, HandlingMgr::cHandlingDataMgr,
        Vehicle::{MISSION_VEHICLE, RANDOM_VEHICLE, script_car_lock}},
};
use super::{GenericGameStorage::{SaveBlock, SaveBlockKind}, SaveBuf::SaveBufWriter};

fn write_vector(writer: &mut SaveBufWriter, vector: CVector) {
    writer.write(vector.x);
    writer.write(vector.y);
    writer.write(vector.z);
}

/// CEntity::SaveEntityFlags: named fields in native bit order.
fn write_entity_flags(writer: &mut SaveBufWriter, entity: &CEntity) {
    let flags = &entity.flags;
    let word1_flags = [flags.uses_collision, flags.collision_processed, flags.is_static,
        flags.has_contacted, flags.ped_physics, flags.is_stuck, flags.is_in_safe_position,
        flags.use_collision_records, flags.was_postponed, flags.explosion_proof,
        flags.is_visible, flags.has_collided, flags.render_scorched, flags.has_blip,
        flags.is_big_building, flags.render_damaged, flags.bullet_proof, flags.fire_proof,
        flags.collision_proof, flags.melee_proof, flags.only_damaged_by_player,
        flags.streaming_dont_delete, flags.zone_culled, flags.zone_culled2];
    let word2_flags = [flags.remove_from_world, flags.has_hit_wall, flags.im_being_rendered,
        flags.touching_water, flags.is_subway, flags.draw_last, flags.no_bright_head_lights,
        flags.do_not_render, flags.distance_fade, flags.flag_e2];
    let mut word1 = (entity.entity_type as u32 & 7) | ((entity.status as u32 & 31) << 3);
    for (index, flag) in word1_flags.into_iter().enumerate() {
        word1 |= u32::from(flag) << (index + 8);
    }
    let mut word2 = 0u32;
    for (index, flag) in word2_flags.into_iter().enumerate() {
        word2 |= u32::from(flag) << index;
    }
    writer.write(word1);
    writer.write(word2);
}

fn write_vehicle_base<VehCore, VehVitality, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(writer: &mut SaveBufWriter, vehicle: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>)
where
    VehCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let matrix = &vehicle.physical.entity.matrix;
    writer.zero(4);
    write_vector(writer, matrix.right);
    writer.zero(4);
    write_vector(writer, matrix.forward);
    writer.zero(4);
    write_vector(writer, matrix.up);
    writer.zero(4);
    write_vector(writer, matrix.position);
    writer.zero(16);
    write_entity_flags(writer, &vehicle.physical.entity);
    writer.zero(212);
    let pilot = &vehicle.auto_pilot;
    writer.write(pilot.current_route_node);
    writer.write(pilot.next_route_node);
    writer.write(pilot.prev_route_node);
    writer.write(pilot.time_entered_curve);
    writer.write(pilot.time_to_spend_on_current_curve);
    writer.write(pilot.current_path_node_info);
    writer.write(pilot.next_path_node_info);
    writer.write(pilot.previous_path_node_info);
    writer.write(pilot.anti_reverse_timer);
    writer.write(pilot.time_to_start_mission);
    writer.write(pilot.previous_direction);
    writer.write(pilot.current_direction);
    writer.write(pilot.next_direction);
    writer.write(pilot.current_lane);
    writer.write(pilot.next_lane);
    writer.write(pilot.driving_style as u8);
    writer.write(pilot.car_mission as u8);
    writer.write(pilot.temp_action as u8);
    writer.write(pilot.time_temp_action);
    writer.write(pilot.max_traffic_speed);
    writer.write(pilot.cruise_speed);
    writer.write(u8::from(pilot.slowed_down_because_of_cars)
        | (u8::from(pilot.slowed_down_because_of_peds) << 1)
        | (u8::from(pilot.stay_in_current_level) << 2)
        | (u8::from(pilot.stay_in_fast_lane) << 3)
        | (u8::from(pilot.ignore_pathfinding) << 4));
    writer.zero(2);
    write_vector(writer, pilot.destination_coors);
    writer.zero(32);
    writer.write(pilot.path_find_nodes_count);
    writer.zero(6);
    writer.write(vehicle.appearance.current_colour1);
    writer.write(vehicle.appearance.current_colour2);
    writer.zero(2);
    writer.write(vehicle.audio.alarm_state);
    writer.zero(43);
    writer.write(vehicle.entry.num_max_passengers);
    writer.zero(2);
    for value in vehicle.field_1d0 { writer.write(value); }
    writer.zero(8);
    writer.write(vehicle.drive.steer_angle);
    writer.write(vehicle.drive.gas_pedal);
    writer.write(vehicle.drive.brake_pedal);
    writer.write(vehicle.vehicle_created_by);
    writer.write(u8::from(vehicle.flags.is_law_enforcer)
        | (u8::from(vehicle.flags.is_locked) << 3)
        | (u8::from(vehicle.drive.engine_on) << 4)
        | (u8::from(vehicle.drive.is_handbrake_on) << 5)
        | (u8::from(vehicle.appearance.lights_on) << 6)
        | (u8::from(vehicle.flags.freebies) << 7));
    writer.zero(10);
    writer.write(vehicle.vitality.health);
    writer.write(vehicle.drive.current_gear);
    writer.zero(3);
    writer.write(vehicle.drive.change_gear_time);
    writer.zero(4);
    writer.write(vehicle.vitality.time_of_death);
    writer.zero(2);
    writer.write(vehicle.vitality.bomb_timer);
    writer.zero(12);
    writer.write(vehicle.entry.door_lock as i32);
    writer.zero(96);
}

fn write_damage(writer: &mut SaveBufWriter, damage: &CDamageManager) {
    writer.write(damage.wheel_damage_effect);
    writer.write(damage.engine_status);
    for status in damage.wheel_status { writer.write(status); }
    for status in damage.door_status { writer.write(status); }
    writer.zero(1);
    writer.write(damage.light_status);
    writer.write(damage.panel_status);
    writer.write(damage.field_18);
    writer.zero(3);
}

pub(crate) fn save_live_vehicle_pool(
    vehicles: &crate::control::ScriptEntities::VehicleReadStore,
    occupants: &Query<&InVehicle>,
) -> SaveBlock {
    let occupied: std::collections::HashSet<Entity> = occupants.iter().map(|relation| relation.vehicle).collect();
    let mut records = Vec::new();
    let mut cars = 0i32;
    let mut boats = 0i32;
    for item in vehicles.entities.iter() {
        let entity=item.0;
        let Some(vehicle)=crate::control::ScriptEntities::vehicle_ref(item) else {continue;};
        if vehicle.base().vehicle_created_by != MISSION_VEHICLE || occupied.contains(&entity) { continue; }
        let mut payload = SaveBufWriter::new();
        write_vehicle_base(&mut payload, &vehicle.base());
        debug_assert_eq!(payload.offset(), COMPATIBLE_VEHICLE_BASE_SAVE_SIZE);
        let kind = match vehicle {
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                write_damage(&mut payload, &car.damage);
                payload.zero(COMPATIBLE_AUTOMOBILE_TRAILING_SAVE_SIZE - COMPATIBLE_DAMAGE_MANAGER_SAVE_SIZE);
                cars += 1;
                VEHICLE_TYPE_CAR
            }
            crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. } => {
                payload.zero(COMPATIBLE_BOAT_TRAILING_SAVE_SIZE);
                boats += 1;
                VEHICLE_TYPE_BOAT
            }
            _ => continue,
        };
        records.push((kind, vehicle.base().physical.entity.model_index, vehicle.handle(), payload.into_inner()));
    }
    records.sort_by_key(|record| CPoolHandle(record.2).index());
    let mut writer = SaveBufWriter::new();
    writer.write(cars);
    writer.write(boats);
    for (kind, model, handle, payload) in records {
        writer.write(kind);
        writer.write(model);
        writer.write(handle);
        writer.write_bytes(&payload);
    }
    SaveBlock { kind: SaveBlockKind::VehiclePool, data: writer.into_inner() }
}

fn write_compressed_matrix(writer: &mut SaveBufWriter, matrix: &CMatrix) {
    write_vector(writer, matrix.position);
    for value in [matrix.right.x, matrix.right.y, matrix.right.z,
        matrix.forward.x, matrix.forward.y, matrix.forward.z] {
        writer.write((127.0 * value) as i8);
    }
    writer.zero(6);
}

pub(crate) fn save_live_object_pool(objects: &Query<(Entity, &ScriptObjectIdentity, &CObject)>) -> SaveBlock {
    let mut records = Vec::new();
    for (_, identity, state) in objects.iter() {
        if state.object_created_by != MISSION_OBJECT { continue; }
        let mut payload = SaveBufWriter::new();
        write_compressed_matrix(&mut payload, &state.physical.entity.matrix);
        payload.write(state.uproot_limit);
        write_compressed_matrix(&mut payload, &state.object_matrix);
        payload.write(state.object_created_by);
        for flag in [state.flags.is_pickup, state.flags.pickup_obj_with_message,
            state.flags.out_of_stock, state.flags.glass_cracked, state.flags.glass_broken,
            state.flags.has_been_damaged, state.flags.use_vehicle_colours] {
            payload.write(u8::from(flag));
        }
        payload.write(state.collision_damage_multiplier);
        payload.write(state.collision_damage_effect as i8);
        payload.write(state.special_collision_response_cases as i8);
        payload.write(state.end_of_life_time);
        write_entity_flags(&mut payload, &state.physical.entity);
        debug_assert_eq!(payload.offset(), COMPATIBLE_OBJECT_SAVE_RECORD_SIZE - 6);
        records.push((state.physical.entity.model_index, identity.handle, payload.into_inner()));
    }
    records.sort_by_key(|record| CPoolHandle(record.1).index());
    let mut writer = SaveBufWriter::new();
    writer.write(records.len() as i32);
    for (model, handle, payload) in records {
        writer.write(model);
        writer.write(handle);
        writer.write_bytes(&payload);
    }
    SaveBlock { kind: SaveBlockKind::ObjectPool, data: writer.into_inner() }
}

fn apply_vehicle_base<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>, saved: &CompatibleVehicleBaseSave) -> Result<(), &'static str>
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let entity = &mut vehicle.physical.entity;
    entity.matrix.right = saved.matrix.right;
    entity.matrix.forward = saved.matrix.forward;
    entity.matrix.up = saved.matrix.up;
    entity.matrix.position = saved.matrix.position;
    entity.flags = saved.entity_flags.flags;
    entity.status = script_entity_status(saved.entity_flags.status_bits as i32)
        .ok_or("saved vehicle entity status is invalid")?;
    let pilot = &mut vehicle.auto_pilot;
    let data = &saved.autopilot;
    pilot.current_route_node = data.current_route_node;
    pilot.next_route_node = data.next_route_node;
    pilot.prev_route_node = data.prev_route_node;
    pilot.time_entered_curve = data.time_entered_curve;
    pilot.time_to_spend_on_current_curve = data.time_to_spend_on_current_curve;
    pilot.current_path_node_info = data.current_path_node_info;
    pilot.next_path_node_info = data.next_path_node_info;
    pilot.previous_path_node_info = data.previous_path_node_info;
    pilot.anti_reverse_timer = data.anti_reverse_timer;
    pilot.time_to_start_mission = data.time_to_start_mission;
    pilot.previous_direction = data.previous_direction;
    pilot.current_direction = data.current_direction;
    pilot.next_direction = data.next_direction;
    pilot.current_lane = data.current_lane;
    pilot.next_lane = data.next_lane;
    pilot.driving_style = match data.driving_style {
        0 => eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS,
        1 => eCarDrivingStyle::DRIVINGSTYLE_SLOW_DOWN_FOR_CARS,
        2 => eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS,
        3 => eCarDrivingStyle::DRIVINGSTYLE_PLOUGH_THROUGH,
        4 => eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS_IGNORE_LIGHTS,
        _ => return Err("saved autopilot driving style is invalid"),
    };
    if data.car_mission > 19 { return Err("saved autopilot mission is invalid"); }
    pilot.car_mission = script_car_mission(data.car_mission as i32);
    pilot.temp_action = match data.temp_action {
        0 => eCarTempAction::TEMPACT_NONE,
        1 => eCarTempAction::TEMPACT_WAIT,
        2 => eCarTempAction::TEMPACT_REVERSE,
        3 => eCarTempAction::TEMPACT_HANDBRAKETURNLEFT,
        4 => eCarTempAction::TEMPACT_HANDBRAKETURNRIGHT,
        5 => eCarTempAction::TEMPACT_HANDBRAKESTRAIGHT,
        6 => eCarTempAction::TEMPACT_TURNLEFT,
        7 => eCarTempAction::TEMPACT_TURNRIGHT,
        8 => eCarTempAction::TEMPACT_GOFORWARD,
        9 => eCarTempAction::TEMPACT_SWERVELEFT,
        10 => eCarTempAction::TEMPACT_SWERVERIGHT,
        _ => return Err("saved autopilot temporary action is invalid"),
    };
    pilot.time_temp_action = data.time_temp_action;
    pilot.max_traffic_speed = data.max_traffic_speed;
    pilot.cruise_speed = data.cruise_speed;
    pilot.slowed_down_because_of_cars = data.slowed_down_because_of_cars;
    pilot.slowed_down_because_of_peds = data.slowed_down_because_of_peds;
    pilot.stay_in_current_level = data.stay_in_current_level;
    pilot.stay_in_fast_lane = data.stay_in_fast_lane;
    pilot.ignore_pathfinding = data.ignore_pathfinding;
    pilot.destination_coors = data.destination_coors;
    pilot.path_find_nodes_count = data.path_find_nodes_count;
    vehicle.appearance.current_colour1 = saved.current_colour1;
    vehicle.appearance.current_colour2 = saved.current_colour2;
    vehicle.audio.alarm_state = saved.alarm_state;
    vehicle.entry.num_max_passengers = saved.num_max_passengers;
    vehicle.field_1d0 = saved.field_1d0;
    vehicle.drive.steer_angle = saved.steer_angle;
    vehicle.drive.gas_pedal = saved.gas_pedal;
    vehicle.drive.brake_pedal = saved.brake_pedal;
    vehicle.vehicle_created_by = saved.vehicle_created_by;
    vehicle.flags.is_law_enforcer = saved.is_law_enforcer;
    vehicle.flags.is_locked = saved.is_locked;
    vehicle.drive.engine_on = saved.engine_on;
    vehicle.drive.is_handbrake_on = saved.is_handbrake_on;
    vehicle.appearance.lights_on = saved.lights_on;
    vehicle.flags.freebies = saved.freebies;
    vehicle.vitality.health = saved.health;
    vehicle.drive.current_gear = saved.current_gear;
    vehicle.drive.change_gear_time = saved.change_gear_time;
    vehicle.vitality.time_of_death = saved.time_of_death;
    vehicle.vitality.bomb_timer = saved.bomb_timer;
    if !(0..=6).contains(&saved.door_lock) { return Err("saved door lock is invalid"); }
    vehicle.entry.door_lock = script_car_lock(saved.door_lock);
    Ok(())
}

fn decompress_matrix(saved: &CompatibleCompressedMatrix) -> CMatrix {
    let mut matrix = CMatrix::default();
    matrix.right = CVector::new(saved.right[0] as f32 / 127.0, saved.right[1] as f32 / 127.0, saved.right[2] as f32 / 127.0);
    // Native compressed `m_upX/Y/Z` actually stores the forward vector.
    matrix.forward = CVector::new(saved.up[0] as f32 / 127.0, saved.up[1] as f32 / 127.0, saved.up[2] as f32 / 127.0);
    matrix.up = matrix.right.cross(matrix.forward);
    matrix.position = saved.position;
    matrix.reorthogonalise();
    matrix
}

pub(crate) fn restore_live_vehicle_and_object_pools(
    pools: &mut CPools,
    entities: &mut ScriptEntityWorld,
    file_loader: &CFileLoader,
    handling: &cHandlingDataMgr,
    general: &mut CGeneral,
    time_ms: u32,
    all_taxi_lights: bool,
) -> Result<(), &'static str> {
    let mut vehicles = Vec::new();
    for (_, record) in pools.compatible_vehicle_records() {
        let model = file_loader.model_info.get_model_info(record.model_index as usize)
            .ok_or("saved vehicle model metadata is unavailable")?;
        let ModelInfoSlot::Vehicle(vehicle_model) = model else { return Err("saved vehicle model has the wrong type") };
        let handling_id = cHandlingDataMgr::get_handling_id(&vehicle_model.handling_id)
            .ok_or("saved vehicle HANDLING.CFG identity is unavailable")?;
        let snapshot = handling.automobile_snapshot(handling_id)
            .ok_or("saved vehicle handling data is unavailable")?;
        let colours = Some((record.base.current_colour1, record.base.current_colour2));
        let mut vehicle = match &record.specific {
            CompatibleVehicleSpecificSave::Automobile { damage, .. } => {
                let mut car = CAutomobile::new(record.model_index as i32, RANDOM_VEHICLE, time_ms,
                    general, Some(snapshot), colours, all_taxi_lights);
                car.vehicle.appearance.extras = choose_vehicle_optional_components(Some(model), general, false);
                if let Some(data) = handling.get_handling_data(handling_id) {
                    car.setup_suspension_from_model(vehicle_model, data);
                }
                car.damage = CDamageManager {
                    wheel_damage_effect: damage.wheel_damage_effect,
                    engine_status: damage.engine_status,
                    wheel_status: damage.wheel_status,
                    door_status: damage.door_status,
                    light_status: damage.light_status,
                    panel_status: damage.panel_status,
                    field_18: damage.field_18,
                };
                // SetDoorDamage/SetPanelDamage/SetBumperDamage are projections
                // of this shared manager state in Rust: the renderer selects
                // the matching authored `_ok` / `_dam` atomics, and the next
                // vehicle debris stage consumes newly-missing transitions.
                ScriptVehicleEntity::Automobile { handle: record.handle.0, car }
            }
            CompatibleVehicleSpecificSave::Boat { .. } => {
                let boat = CBoat::new(record.model_index as i32, RANDOM_VEHICLE, time_ms, general,
                    Some(CBoatHandlingSnapshot { mass: snapshot.mass, turn_mass: snapshot.turn_mass,
                        centre_of_mass: snapshot.centre_of_mass, dimension: snapshot.dimension,
                        buoyancy: snapshot.buoyancy, handling_id: snapshot.handling_id }), colours);
                ScriptVehicleEntity::Boat { handle: record.handle.0, boat }
            }
        };
        apply_vehicle_base(vehicle.base_mut(), &record.base)?;
        vehicle.base_mut().physical.zone_level = file_loader.world.zones.get_level_from_position(
            [record.base.matrix.position.x, record.base.matrix.position.y, record.base.matrix.position.z]) as i8;
        vehicles.push(vehicle);
    }
    let mut objects = Vec::new();
    for (_, record) in pools.compatible_object_records() {
        let _model = file_loader.model_info.get_model_info(record.model_index as usize)
            .ok_or("saved object model metadata is unavailable")?;
        let mut object = CObject::new_with_model(record.model_index as i32, false, general);
        object.initialize_world_state(file_loader);
        object.physical.entity.matrix = decompress_matrix(&record.matrix);
        object.physical.entity.flags = record.entity_flags.flags;
        object.physical.entity.status = script_entity_status(record.entity_flags.status_bits as i32)
            .ok_or("saved object entity status is invalid")?;
        object.physical.entity.flags.has_collided = false;
        object.uproot_limit = record.uproot_limit;
        object.object_matrix = decompress_matrix(&record.object_matrix);
        object.object_created_by = record.object_created_by;
        object.flags.is_pickup = record.is_pickup;
        object.flags.pickup_obj_with_message = record.pickup_obj_with_message;
        object.flags.out_of_stock = record.out_of_stock;
        object.flags.glass_cracked = record.glass_cracked;
        object.flags.glass_broken = record.glass_broken;
        object.flags.has_been_damaged = record.has_been_damaged;
        object.flags.use_vehicle_colours = record.use_vehicle_colours;
        object.collision_damage_multiplier = record.collision_damage_multiplier;
        object.collision_damage_effect = record.collision_damage_effect as u8;
        object.special_collision_response_cases = record.special_collision_response_cases as u8;
        object.end_of_life_time = record.end_of_life_time;
        objects.push(ScriptObjectEntity { handle: record.handle.0, object });
    }
    // All records and installation-backed data are resolved before ECS mutation.
    let old_vehicles: Vec<_> = entities.index.vehicles.keys().copied().collect();
    for handle in old_vehicles { entities.despawn_vehicle(handle); }
    let old_objects: Vec<_> = entities.index.objects.drain().map(|(_, entity)| entity).collect();
    for entity in old_objects { entities.commands.entity(entity).despawn(); }
    for vehicle in vehicles { entities.spawn_vehicle(vehicle); }
    for object in objects { spawn_script_object(&mut entities.commands, &mut entities.index, object); }
    for slot in 0..pools.vehicle_pool.get_size() {
        if let Some(entry) = pools.vehicle_pool.get_slot_mut(slot) { entry.compatible_save = None; }
    }
    for slot in 0..pools.object_pool.get_size() {
        if let Some(entry) = pools.object_pool.get_slot_mut(slot) { entry.compatible_save = None; }
    }
    Ok(())
}

#[cfg(test)]
mod drive_save_tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    use crate::{control::ScriptEntities::{ScriptEntityIndex, VehicleReadStore, spawn_script_test_vehicle},
        vehicles::Vehicle::{VehicleDriveState, VehicleAppearance, VehicleEntryState, eCarLock}};

    #[test]
    fn live_drive_state_preserves_native_vehicle_bytes_and_restored_component() {
        let mut world = World::new();
        world.init_resource::<ScriptEntityIndex>();
        let mut general = CGeneral::with_seed(11);
        let handle = 0x501;
        let car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        let entity = spawn_script_test_vehicle(&mut world, ScriptVehicleEntity::Automobile { handle, car });
        {
            let mut drive = world.get_mut::<VehicleDriveState>(entity).unwrap();
            drive.steer_angle = -0.25;
            drive.gas_pedal = 0.5;
            drive.brake_pedal = 0.75;
            drive.current_gear = 3;
            drive.change_gear_time = 12.5;
            drive.engine_on = false;
            drive.is_handbrake_on = true;
        }
        {
            let mut appearance = world.get_mut::<VehicleAppearance>(entity).unwrap();
            appearance.current_colour1 = 12;
            appearance.current_colour2 = 27;
            appearance.lights_on = true;
        }
        {
            let mut entry = world.get_mut::<VehicleEntryState>(entity).unwrap();
            entry.num_max_passengers = 3;
            entry.door_lock = eCarLock::CARLOCK_LOCKOUT_PLAYER_ONLY;
        }
        world.get_mut::<crate::vehicles::Vehicle::VehicleAudioState>(entity).unwrap().alarm_state = 1234;
        let mut reader = SystemState::<(VehicleReadStore, Query<&InVehicle>)>::new(&mut world);
        let (vehicles, occupants) = reader.get(&world).unwrap();
        let block = save_live_vehicle_pool(&vehicles, &occupants);
        assert_eq!(block.data.len(), 8 + 10 + COMPATIBLE_AUTOMOBILE_SAVE_SIZE);
        let mut pools = CPools::initialise();
        pools.load_vehicle_pool_compatible(&block.data).unwrap();
        let (_, saved) = pools.compatible_vehicle_records().next().unwrap();
        assert_eq!(saved.base.alarm_state, 1234);
        assert_eq!(saved.base.steer_angle, -0.25);
        assert_eq!(saved.base.gas_pedal, 0.5);
        assert_eq!(saved.base.brake_pedal, 0.75);
        assert_eq!(saved.base.current_gear, 3);
        assert_eq!(saved.base.change_gear_time, 12.5);
        assert!(!saved.base.engine_on);
        assert!(saved.base.is_handbrake_on);
        assert_eq!(saved.base.current_colour1, 12);
        assert_eq!(saved.base.current_colour2, 27);
        assert!(saved.base.lights_on);
        assert_eq!(saved.base.num_max_passengers, 3);
        assert_eq!(saved.base.door_lock, eCarLock::CARLOCK_LOCKOUT_PLAYER_ONLY as i32);
        assert_eq!(pools.save_vehicle_pool_compatible(), block.data);

        let mut restored = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        apply_vehicle_base(&mut restored.vehicle, &saved.base).unwrap();
        world.despawn(entity);
        let restored_entity = spawn_script_test_vehicle(&mut world,
            ScriptVehicleEntity::Automobile { handle, car: restored });
        assert_eq!(world.get::<crate::vehicles::Vehicle::VehicleAudioState>(restored_entity).unwrap().alarm_state, 1234);
        let drive = world.get::<VehicleDriveState>(restored_entity).unwrap();
        assert_eq!(drive.current_gear, 3);
        assert_eq!(drive.brake_pedal, 0.75);
        assert!(!drive.engine_on);
        assert!(drive.is_handbrake_on);
        let appearance = world.get::<VehicleAppearance>(restored_entity).unwrap();
        assert_eq!(appearance.current_colour1, 12);
        assert_eq!(appearance.current_colour2, 27);
        assert!(appearance.lights_on);
        let entry = world.get::<VehicleEntryState>(restored_entity).unwrap();
        assert_eq!(entry.num_max_passengers, 3);
        assert_eq!(entry.door_lock, eCarLock::CARLOCK_LOCKOUT_PLAYER_ONLY);
    }
}
