//! re3 reference: references/re-GTA/src/control/RoadBlocks.cpp
//! re3 reference: references/re-GTA/src/control/RoadBlocks.h
//!
//! `CRoadBlocks::Init` state.

use bevy::prelude::*;
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    control::{
        AutoPilot::{eCarMission, eCarTempAction},
        CarRails::join_car_with_road_system,
        PathFind::{CPathMapObject, OBJECT_EAST_WEST},
        Script::{ScriptCreateCarRequest, ScriptCreateCharRequest},
    },
    core::{
        Camera::CCamera,
        FileLoader::CFileLoader,
        Game::{ScriptEntityWorld, ScriptTaxiLightState},
        Pools::{CPoolHandle, CPools},
        Streaming::CStreaming,
    },
    entities::Entity::eEntityStatus,
    math::Matrix::CMatrix,
    modelinfo::ModelIndices::{
        MI_ARMY, MI_BARRACKS, MI_COP, MI_ENFORCER, MI_FBI, MI_FBICAR, MI_POLICE, MI_SWAT,
    },
    peds::{
        Ped::{RANDOM_CHAR, set_script_ped_idle},
        PedPlacement::CPedPlacement,
        PedType::ePedType,
        PlayerPed::CWorldPlayers,
    },
    vehicles::{
        HandlingMgr::cHandlingDataMgr,
        Vehicle::{RANDOM_VEHICLE, eCarLock},
    },
    weapons::WeaponType::eWeaponType,
};

pub const NUMROADBLOCKS: usize = 600;
const ROADBLOCK_DISTANCE: f32 = 80.0;
const DISTANCE_TO_SPAWN_ROADBLOCK_PEDS: f32 = 51.0;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct RoadblockCop {
    pub roadblock_node: i16,
    pub vehicle_handle: i32,
}

#[derive(Debug, Clone, Resource)]
pub struct CRoadBlocks {
    pub num_road_blocks: i16,
    pub road_block_objects: Vec<i16>,
    pub in_or_out: Vec<bool>,
}

impl CRoadBlocks {
    pub fn init_from_path_objects(path_object_flags: impl IntoIterator<Item = bool>) -> Self {
        let mut road_block_objects = Vec::with_capacity(NUMROADBLOCKS);
        let mut in_or_out = Vec::with_capacity(NUMROADBLOCKS);

        for (index, use_in_roadblock) in path_object_flags.into_iter().enumerate() {
            if use_in_roadblock {
                if road_block_objects.len() >= NUMROADBLOCKS {
                    break;
                }
                in_or_out.push(true);
                road_block_objects.push(index as i16);
            }
        }

        Self {
            num_road_blocks: road_block_objects.len() as i16,
            road_block_objects,
            in_or_out,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_roadblocks(
    mut roadblocks: ResMut<CRoadBlocks>,
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut general: ResMut<CGeneral>,
    file_loader: Res<CFileLoader>,
    handling_manager: Res<cHandlingDataMgr>,
    players: CWorldPlayers,
    camera: Res<CCamera>,
    streaming: Res<CStreaming>,
    timer: Res<CTimer>,
    taxi_lights: Res<ScriptTaxiLightState>,
) {
    let Some(player) = players.focused() else {
        return;
    };
    let player_position = player.position();
    let roadblock_density = player.wanted.m_RoadblockDensity;
    let army_required = player.wanted.are_army_required();
    let fbi_required = player.wanted.are_fbi_required();
    let swat_required = player.wanted.are_swat_required();
    let player_in_vehicle = players.relation(players.player_in_focus() as i32).is_some();

    let frame = (timer.frame_counter() & 0xF) as usize;
    let start = NUMROADBLOCKS * frame / 16;
    let end = NUMROADBLOCKS * (frame + 1) / 16;
    let count = (roadblocks.num_road_blocks.max(0) as usize)
        .min(roadblocks.road_block_objects.len())
        .min(roadblocks.in_or_out.len());

    for roadblock_node in start..end.min(count) {
        let map_object_index = roadblocks.road_block_objects[roadblock_node].max(0) as usize;
        let Some(map_object) = file_loader
            .path_find
            .map_objects
            .get(map_object_index)
            .and_then(|object| *object)
        else {
            continue;
        };

        let dx = player_position.x - map_object.position.x;
        let dy = player_position.y - map_object.position.y;
        let inside = dx > -ROADBLOCK_DISTANCE
            && dx < ROADBLOCK_DISTANCE
            && dy > -ROADBLOCK_DISTANCE
            && dy < ROADBLOCK_DISTANCE
            && dx * dx + dy * dy < ROADBLOCK_DISTANCE * ROADBLOCK_DISTANCE;
        if !inside {
            roadblocks.in_or_out[roadblock_node] = false;
            continue;
        }
        if roadblocks.in_or_out[roadblock_node] {
            continue;
        }
        roadblocks.in_or_out[roadblock_node] = true;

        if !player_in_vehicle || (general.get_random_number() & 0x7F) as i16 >= roadblock_density {
            continue;
        }

        let mut vehicle_model = if army_required {
            MI_BARRACKS
        } else if fbi_required {
            MI_FBICAR
        } else if swat_required {
            MI_ENFORCER
        } else {
            MI_POLICE
        };
        if !streaming.has_model_loaded(vehicle_model) {
            vehicle_model = MI_POLICE;
        }

        let Some(map_col) = file_loader.model_info.col_model(map_object.model_index) else {
            continue;
        };
        let Some(vehicle_col) = file_loader.model_info.col_model(vehicle_model) else {
            continue;
        };
        let map_object_radius = 2.0 * map_col.bounding_box.max.x;
        let vehicle_spacing = 2.0 * vehicle_col.bounding_sphere.radius + 0.25;
        let vehicles_across = (map_object_radius / vehicle_spacing) as i16;
        if vehicles_across <= 0 || vehicles_across >= 6 {
            continue;
        }

        let map_matrix = map_object_matrix(map_object);
        let camera_delta = CVector::new(
            camera.position.x - map_object.position.x,
            camera.position.y - map_object.position.y,
            0.0,
        );
        let camera_dot =
            camera_delta.x * map_matrix.forward.x + camera_delta.y * map_matrix.forward.y;
        let offset = 0.5 * vehicle_spacing * f32::from(vehicles_across - 1);
        let east_west = file_loader
            .path_find
            .object_flags
            .get(map_object_index)
            .is_some_and(|flags| flags & OBJECT_EAST_WEST != 0);

        for index in 0..vehicles_across {
            let mut roadblock_type = u8::from(camera_dot < 0.0);
            let rotate_positive = general.get_random_number() & 1 != 0;
            let jitter = (f32::from((general.get_random_number() & 0xFF) as u8) - 128.0) * 0.003;
            let angle = if rotate_positive {
                jitter + std::f32::consts::FRAC_PI_2
            } else {
                roadblock_type ^= 1;
                jitter - std::f32::consts::FRAC_PI_2
            };

            let mut local = CMatrix::unity();
            local.set_rotate_z(angle);
            local.position = if east_west {
                CVector::new(0.0, f32::from(index) * vehicle_spacing - offset, 0.6)
            } else {
                CVector::new(f32::from(index) * vehicle_spacing - offset, 0.0, 0.6)
            };
            let mut vehicle_matrix = map_matrix.multiply(&local);
            let collision_radius = vehicle_col.bounding_sphere.radius - 0.25;
            if roadblock_spawn_is_blocked(
                vehicle_matrix.position,
                collision_radius,
                &script_entities,
                &file_loader,
                player_position,
                player_in_vehicle,
            ) {
                continue;
            }

            vehicle_matrix.position.z += collision_radius - 0.6;
            let request = ScriptCreateCarRequest {
                model_id: vehicle_model as i32,
                position: vehicle_matrix.position,
                time_ms: timer.time_in_milliseconds(),
                clear_space_for_mission_entity: false,
            };
            let Some(handle) = script_entities.insert_script_car(
                &mut pools,
                request,
                file_loader.model_info.get_model_info(vehicle_model),
                Some(&handling_manager),
                &mut general,
                &file_loader,
                taxi_lights.all_enabled,
            ) else {
                continue;
            };

            let bbox_min = CVector::new(
                vehicle_col.bounding_box.min.x,
                vehicle_col.bounding_box.min.y,
                vehicle_col.bounding_box.min.z,
            );
            let bbox_max = CVector::new(
                vehicle_col.bounding_box.max.x,
                vehicle_col.bounding_box.max.y,
                vehicle_col.bounding_box.max.z,
            );
            let accepted = match script_entities.vehicle_get_mut(handle) {
                Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) => {
                    car.vehicle.core.physical.entity.set_matrix(vehicle_matrix);
                    car.place_on_road_properly(bbox_min, bbox_max, |probe, low_z| {
                        file_loader
                            .world
                            .process_vertical_line_buildings_only_with_entity(
                                [probe.x, probe.y, probe.z],
                                low_z,
                                &file_loader.model_info,
                            )
                            .map(|(point, entity)| (point.point.z, entity))
                    });
                    car.vehicle.set_status(eEntityStatus::STATUS_ABANDONED);
                    car.vehicle.core.physical.entity.set_is_static(false);
                    car.vehicle.core.vehicle_created_by = RANDOM_VEHICLE;
                    car.vehicle.core.physical.zone_level =
                        crate::collision::ColModel::LEVEL_GENERIC as i8;
                    car.vehicle.core.physical.entity.clump_alpha = 0;
                    car.vehicle.core.flags.is_locked = false;
                    car.vehicle.core.flags.has_been_owned_by_player = false;
                    car.vehicle.core.flags.extended_range = true;
                    car.vehicle.entry.door_lock = eCarLock::CARLOCK_UNLOCKED;
                    join_car_with_road_system(
                        &file_loader.path_find,
                        &mut car.vehicle,
                        &mut general,
                    );
                    car.vehicle.core.auto_pilot.car_mission = eCarMission::MISSION_NONE;
                    car.vehicle.core.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                    car.vehicle.core.auto_pilot.current_lane = 0;
                    car.vehicle.core.auto_pilot.next_lane = 0;
                    car.vehicle.core.auto_pilot.max_traffic_speed = 0.0;
                    car.vehicle.core.auto_pilot.cruise_speed = 0;
                    if crate::vehicles::Vehicle::vehicle_uses_siren(vehicle_model)
                        && general.get_random_number() & 1 != 0
                    {
                        car.vehicle.audio.siren_or_alarm = true;
                    }
                    if car.vehicle.core.physical.entity.get_up().z > 0.94 {
                        car.vehicle.core.flags.create_road_block_peds = true;
                        car.vehicle.core.roadblock_type = roadblock_type;
                        car.vehicle.core.roadblock_node = roadblock_node as i16;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if !accepted {
                script_entities.remove_car(&mut pools, handle);
            }
        }
    }
}

pub(crate) fn process_roadblock_cop_spawns(
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut general: ResMut<CGeneral>,
    file_loader: Res<CFileLoader>,
    players: CWorldPlayers,
    streaming: Res<CStreaming>,
    timer: Res<CTimer>,
) {
    let Some(player) = players.focused() else {
        return;
    };
    let player_position = player.position();
    let player_slot = CPoolHandle(player.pool_handle).index();
    let Some(police_radius) = file_loader
        .model_info
        .col_model(MI_POLICE)
        .map(|col| col.bounding_sphere.radius)
        .filter(|radius| *radius > 0.0)
    else {
        return;
    };

    let mut pending = Vec::new();
    for (&handle, vehicle) in script_entities.vehicles.iter() {
        let base = vehicle.base();
        if !base.flags.create_road_block_peds {
            continue;
        }
        let position = base.physical.entity.get_position();
        let dx = position.x - player_position.x;
        let dy = position.y - player_position.y;
        if dx * dx + dy * dy >= DISTANCE_TO_SPAWN_ROADBLOCK_PEDS.powi(2) {
            continue;
        }
        let model = base.physical.entity.model_index.max(0) as usize;
        let Some(radius) = file_loader
            .model_info
            .col_model(model)
            .map(|col| col.bounding_sphere.radius)
        else {
            continue;
        };
        pending.push((
            handle,
            base.physical.entity.matrix,
            model,
            radius,
            base.roadblock_type,
            base.roadblock_node,
        ));
    }

    for (
        vehicle_handle,
        vehicle_matrix,
        vehicle_model,
        vehicle_radius,
        roadblock_type,
        roadblock_node,
    ) in pending
    {
        if let Some(mut vehicle) = script_entities.vehicle_get_mut(vehicle_handle) {
            vehicle.base_mut().flags.create_road_block_peds = false;
        }

        let specialised_model = match vehicle_model {
            MI_FBICAR => MI_FBI,
            MI_ENFORCER => MI_SWAT,
            MI_BARRACKS => MI_ARMY,
            _ => MI_COP,
        };
        let cop_model =
            if specialised_model != MI_COP && !streaming.has_model_loaded(specialised_model) {
                MI_COP
            } else {
                specialised_model
            };
        let scale = vehicle_radius / police_radius;

        for offset in roadblock_cop_offsets(roadblock_type) {
            let spawn = vehicle_matrix.transform_point(CVector::new(
                offset.x * scale,
                offset.y * scale,
                0.0,
            ));
            let placed = CPedPlacement::find_z_coor_for_ped(
                spawn,
                &file_loader.world,
                &file_loader.model_info,
            )
            .position;
            let Some(handle) = script_entities.insert_script_char(
                &mut pools,
                ScriptCreateCharRequest {
                    ped_type: ePedType::PEDTYPE_COP as i32,
                    model_id: cop_model as i32,
                    position: placed,
                    vehicle_handle: None,
                    passenger_seat: None,
                },
                &mut general,
                None,
                &file_loader,
            ) else {
                continue;
            };

            if let Some(entity) = script_entities.ped_get_mut(handle) {
                let ped = &mut *entity.ped;
                ped.char_created_by = RANDOM_CHAR;
                ped.physical.zone_level = crate::collision::ColModel::LEVEL_GENERIC as i8;
                ped.physical.entity.clump_alpha = 0;
                ped.physical
                    .entity
                    .set_orientation(0.0, 0.0, -std::f32::consts::FRAC_PI_2);
                ped.flags.responds_to_threats = true;
                ped.flags.allow_medics_to_revive_me = true;
                ped.flags.kinda_stay_in_same_place = true;
                ped.flags.not_allowed_to_duck = false;
                ped.flags.crouch_when_shooting = roadblock_type != 2;
                ped.flags.cull_extra_far_away = true;
                ped.flags.is_pointing_gun_at = true;
                ped.point_gun_at = Some(player_slot);
                ped.seek_target = Some(player_slot);
                if cop_model == MI_COP {
                    ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
                }
                set_script_ped_idle(ped, &mut general, timer.time_in_milliseconds());
            }
            if let Some(entity) = script_entities.ped_entity(handle) {
                script_entities
                    .commands
                    .entity(entity)
                    .insert(RoadblockCop {
                        roadblock_node,
                        vehicle_handle,
                    });
            }
        }
    }
}

pub(crate) fn roadblock_map_object(
    roadblocks: &CRoadBlocks,
    file_loader: &CFileLoader,
    roadblock_node: i16,
) -> Option<CPathMapObject> {
    let index = usize::try_from(roadblock_node).ok()?;
    let object_index = usize::try_from(*roadblocks.road_block_objects.get(index)?).ok()?;
    file_loader
        .path_find
        .map_objects
        .get(object_index)
        .and_then(|object| *object)
}

pub(crate) fn map_object_matrix(map_object: CPathMapObject) -> CMatrix {
    let axis = [
        -map_object.rotation_axis[0],
        -map_object.rotation_axis[1],
        -map_object.rotation_axis[2],
    ];
    CMatrix {
        right: rotate_by_quaternion(CVector::new(1.0, 0.0, 0.0), axis, map_object.rotation_w),
        forward: rotate_by_quaternion(CVector::new(0.0, 1.0, 0.0), axis, map_object.rotation_w),
        up: rotate_by_quaternion(CVector::new(0.0, 0.0, 1.0), axis, map_object.rotation_w),
        position: map_object.position,
        has_rw_matrix: false,
    }
}

fn roadblock_spawn_is_blocked(
    position: CVector,
    radius: f32,
    script_entities: &ScriptEntityWorld<'_, '_>,
    file_loader: &CFileLoader,
    player_position: CVector,
    player_in_vehicle: bool,
) -> bool {
    let intersects = |candidate: CVector, candidate_radius: f32| {
        let delta = CVector::new(
            candidate.x - position.x,
            candidate.y - position.y,
            candidate.z - position.z,
        );
        delta.magnitude_sqr() < (radius + candidate_radius).powi(2)
    };
    if script_entities.vehicles.values().any(|vehicle| {
        let base = vehicle.base();
        file_loader
            .model_info
            .col_model(base.physical.entity.model_index.max(0) as usize)
            .is_some_and(|col| {
                intersects(
                    base.physical.entity.get_position(),
                    col.bounding_sphere.radius,
                )
            })
    }) {
        return true;
    }
    if script_entities.peds.iter().any(|(&handle, entity)| {
        if script_entities.relation_for_char(handle).is_some() {
            return false;
        }
        file_loader
            .model_info
            .col_model(entity.ped.physical.entity.model_index.max(0) as usize)
            .is_some_and(|col| {
                intersects(
                    entity.ped.physical.entity.get_position(),
                    col.bounding_sphere.radius,
                )
            })
    }) {
        return true;
    }
    !player_in_vehicle && intersects(player_position, 0.75)
}

fn roadblock_cop_offsets(roadblock_type: u8) -> [CVector; 2] {
    const OFFSETS: [[f32; 2]; 6] = [
        [-1.5, 1.8],
        [-1.5, -1.8],
        [1.5, 1.8],
        [1.5, -1.8],
        [-1.5, 0.0],
        [1.5, 0.0],
    ];
    let first = usize::from(roadblock_type.min(2)) * 2;
    [
        CVector::new(OFFSETS[first][0], OFFSETS[first][1], 0.0),
        CVector::new(OFFSETS[first + 1][0], OFFSETS[first + 1][1], 0.0),
    ]
}

fn rotate_by_quaternion(value: CVector, axis: [f32; 3], w: f32) -> CVector {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2] + w * w).sqrt();
    if len == 0.0 {
        return value;
    }
    let q = CVector::new(axis[0] / len, axis[1] / len, axis[2] / len);
    let qw = w / len;
    let uv = q.cross(value);
    let uuv = q.cross(uv);
    CVector::new(
        value.x + (uv.x * qw + uuv.x) * 2.0,
        value.y + (uv.y * qw + uuv.y) * 2.0,
        value.z + (uv.z * qw + uuv.z) * 2.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_arms_candidates_only_after_player_leaves_once() {
        let roadblocks = CRoadBlocks::init_from_path_objects([false, true, false, true]);
        assert_eq!(roadblocks.road_block_objects, vec![1, 3]);
        assert_eq!(roadblocks.in_or_out, vec![true, true]);
    }

    #[test]
    fn roadblock_cop_offsets_match_all_three_re3_layouts() {
        assert_eq!(
            roadblock_cop_offsets(0),
            [CVector::new(-1.5, 1.8, 0.0), CVector::new(-1.5, -1.8, 0.0)]
        );
        assert_eq!(
            roadblock_cop_offsets(1),
            [CVector::new(1.5, 1.8, 0.0), CVector::new(1.5, -1.8, 0.0)]
        );
        assert_eq!(
            roadblock_cop_offsets(2),
            [CVector::new(-1.5, 0.0, 0.0), CVector::new(1.5, 0.0, 0.0)]
        );
    }
}
