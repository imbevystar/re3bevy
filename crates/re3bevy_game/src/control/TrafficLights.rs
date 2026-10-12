//! re3 reference: references/re-GTA/src/control/TrafficLights.cpp
//! re3 reference: references/re-GTA/src/control/TrafficLights.h
//!
//! `CTrafficLights::ScanForLightsOnMap` lifecycle state.

use bevy::prelude::*;

use crate::{
    control::PathFind::CPathFind,
    core::World::{CWorld, CWorldEntity, EEntityType},
};

const SOME_FLAG: u8 = 0x80;
pub const PED_LIGHTS_WALK: u8 = 0;
pub const PED_LIGHTS_WALK_BLINK: u8 = 1;
pub const PED_LIGHTS_DONT_WALK: u8 = 2;
pub const CAR_LIGHTS_GREEN: u8 = 0;
pub const CAR_LIGHTS_YELLOW: u8 = 1;
pub const CAR_LIGHTS_RED: u8 = 2;

#[derive(Debug, Clone, Resource)]
pub struct CTrafficLights {
    pub scan_requested: bool,
    pub scan_completed: bool,
    pub model_index: Option<usize>,
    pub lights_found: usize,
    pub path_links_updated: usize,
    pub ped_connections_updated: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CTrafficVehicleState {
    pub position: [f32; 3],
    pub next_path_node_info: usize,
    pub current_path_node_info: usize,
    pub previous_path_node_info: usize,
    pub next_route_node: i16,
    pub current_route_node: i16,
    pub prev_route_node: i16,
    pub next_direction: i8,
    pub current_direction: i8,
    pub previous_direction: i8,
    pub status_physics: bool,
}

impl CTrafficLights {
    pub fn scan_deferred_until_path_graph() -> Self {
        Self {
            scan_requested: true,
            scan_completed: false,
            model_index: None,
            lights_found: 0,
            path_links_updated: 0,
            ped_connections_updated: 0,
        }
    }

    pub fn scan_world_entities(
        world: &CWorld,
        path_find: &mut CPathFind,
        model_index: Option<usize>,
    ) -> Self {
        let traffic_lights: Vec<&CWorldEntity> = world
            .entities
            .iter()
            .filter(|entity| {
                entity.entity_type == EEntityType::Dummy
                    && model_index.is_some_and(|model_index| entity.model_index == model_index)
            })
            .collect();

        let mut path_links_updated = 0;
        let mut ped_connections_updated = 0;

        for light in &traffic_lights {
            path_links_updated += scan_car_path_links_for_light(light, path_find);
            ped_connections_updated += scan_ped_connections_for_light(light, path_find);
        }

        Self {
            scan_requested: true,
            scan_completed: true,
            model_index,
            lights_found: traffic_lights.len(),
            path_links_updated,
            ped_connections_updated,
        }
    }

    pub fn find_traffic_light_type(light: &CWorldEntity) -> u8 {
        let forward = entity_forward(light);
        let mut orientation = forward[1].atan2(forward[0]).to_degrees();
        if orientation < 0.0 {
            orientation += 360.0;
        }
        if (orientation > 60.0 && orientation < 150.0)
            || (orientation > 240.0 && orientation < 330.0)
        {
            1
        } else {
            2
        }
    }

    pub fn light_for_peds(time_ms: u32) -> u8 {
        let period = time_ms % 16384;
        if period < 12000 {
            PED_LIGHTS_DONT_WALK
        } else if period < 16384 - 1000 {
            PED_LIGHTS_WALK
        } else {
            PED_LIGHTS_WALK_BLINK
        }
    }

    pub fn light_for_cars1(time_ms: u32) -> u8 {
        let period = time_ms % 16384;
        if period < 5000 {
            CAR_LIGHTS_GREEN
        } else if period < 5000 + 1000 {
            CAR_LIGHTS_YELLOW
        } else {
            CAR_LIGHTS_RED
        }
    }

    pub fn light_for_cars2(time_ms: u32) -> u8 {
        let period = time_ms % 16384;
        if period < 6000 {
            CAR_LIGHTS_RED
        } else if period < 12000 - 1000 {
            CAR_LIGHTS_GREEN
        } else if period < 12000 {
            CAR_LIGHTS_YELLOW
        } else {
            CAR_LIGHTS_RED
        }
    }

    pub fn should_car_stop_for_light(
        path_find: &CPathFind,
        vehicle: &CTrafficVehicleState,
        always_stop: bool,
        time_ms: u32,
    ) -> bool {
        should_stop_for_link(
            path_find,
            vehicle,
            vehicle.next_path_node_info,
            vehicle.next_route_node,
            vehicle.next_direction,
            always_stop,
            time_ms,
            8.0,
        )
    }

    pub fn should_car_stop_for_bridge(
        path_find: &CPathFind,
        vehicle: &CTrafficVehicleState,
    ) -> bool {
        Self::should_car_stop_for_bridge_links(
            path_find,
            vehicle.next_path_node_info,
            vehicle.current_path_node_info,
        )
    }

    /// The bridge check only needs the two path-link identities. Keep it
    /// available to the ECS car-AI paths without rebuilding a traffic snapshot.
    pub fn should_car_stop_for_bridge_links(
        path_find: &CPathFind,
        next_path_node_info: usize,
        current_path_node_info: usize,
    ) -> bool {
        let next = path_find
            .car_path_links
            .get(next_path_node_info)
            .is_some_and(|link| link.bridge_lights);
        let current = path_find
            .car_path_links
            .get(current_path_node_info)
            .is_some_and(|link| link.bridge_lights);
        next && !current
    }
}

fn should_stop_for_link(
    path_find: &CPathFind,
    vehicle: &CTrafficVehicleState,
    node: usize,
    route_node: i16,
    direction: i8,
    always_stop: bool,
    time_ms: u32,
    stop_distance: f32,
) -> bool {
    let Some(link) = path_find.car_path_links.get(node) else {
        return false;
    };
    let traffic_type = link.traffic_light_type;
    if traffic_type == 0 {
        return false;
    }

    let route_matches = (traffic_type & SOME_FLAG != 0 || link.path_node_index == route_node)
        && (traffic_type & SOME_FLAG == 0 || link.path_node_index != route_node);
    if !route_matches {
        return false;
    }

    let base_type = traffic_type & !SOME_FLAG;
    let red_or_yellow = (base_type == 1
        && CTrafficLights::light_for_cars1(time_ms) != CAR_LIGHTS_GREEN)
        || (base_type == 2 && CTrafficLights::light_for_cars2(time_ms) != CAR_LIGHTS_GREEN);
    if !(always_stop || red_or_yellow) {
        return false;
    }

    let dist = dot2(
        [
            vehicle.position[0] - link.pos[0],
            vehicle.position[1] - link.pos[1],
        ],
        link.dir,
    );
    if direction == -1 {
        dist > 0.0 && dist < stop_distance
    } else {
        dist < 0.0 && dist > -stop_distance
    }
}

fn scan_car_path_links_for_light(light: &CWorldEntity, path_find: &mut CPathFind) -> usize {
    let right = entity_right(light);
    let forward = entity_forward(light);
    let mut updated = 0;

    for link_index in 0..path_find.num_car_path_links.max(0) as usize {
        let link = path_find.car_path_links[link_index];
        let dist = [
            link.pos[0] - light.position[0],
            link.pos[1] - light.position[1],
        ];
        let dot_y = dot2(dist, [forward[0], forward[1]]).abs();
        let dot_x = dot2(dist, [right[0], right[1]]);
        if !(dot_x < 0.0 && dot_x > -15.0 && dot_y < 3.0) {
            continue;
        }

        let Some(path_node) = path_find
            .path_nodes
            .get(link.path_node_index.max(0) as usize)
        else {
            continue;
        };
        if path_node.pos.z - light.position[2] >= 15.0 {
            continue;
        }

        let mut traffic_light_type = CTrafficLights::find_traffic_light_type(light);
        let mut neighbours = path_nodes_using_car_link(path_find, link_index);
        if neighbours.len() >= 2 {
            neighbours.sort_by_key(|node| path_find.path_nodes[*node].num_links);
            let selected = *neighbours.last().unwrap_or(&neighbours[0]);
            if link.path_node_index != selected as i16 {
                traffic_light_type |= SOME_FLAG;
            }
        }
        path_find.car_path_links[link_index].traffic_light_type = traffic_light_type;
        updated += 1;
    }

    updated
}

fn scan_ped_connections_for_light(light: &CWorldEntity, path_find: &mut CPathFind) -> usize {
    let mut updated = 0;
    let first_ped_node = path_find.num_car_path_nodes.max(0) as usize;
    let last_node = path_find.num_path_nodes.max(0) as usize;

    for node_index in first_ped_node..last_node {
        let node = path_find.path_nodes[node_index];
        let dist1 = (node.pos.x - light.position[0]).abs() + (node.pos.y - light.position[1]).abs();
        if dist1 >= 50.0 {
            continue;
        }

        for offset in 0..node.num_links as usize {
            let connection_index = node.first_link as usize + offset;
            if !path_find
                .connection_flags
                .get(connection_index)
                .is_some_and(|flags| flags.crosses_road)
            {
                continue;
            }

            let Some(connected_node) = path_find.connections.get(connection_index).copied() else {
                continue;
            };
            if connected_node < 0 {
                continue;
            }
            let connected = path_find.path_nodes[connected_node as usize];
            let dist2 = (connected.pos.x - light.position[0]).abs()
                + (connected.pos.y - light.position[1]).abs();
            if dist1 < 15.0 || dist2 < 15.0 {
                if let Some(flags) = path_find.connection_flags.get_mut(connection_index) {
                    flags.traffic_light = true;
                    updated += 1;
                }
            }
        }
    }

    updated
}

fn path_nodes_using_car_link(path_find: &CPathFind, car_link_index: usize) -> Vec<usize> {
    let mut nodes = Vec::with_capacity(2);
    for node_index in 0..path_find.num_path_nodes.max(0) as usize {
        let node = path_find.path_nodes[node_index];
        for offset in 0..node.num_links as usize {
            let connection_index = node.first_link as usize + offset;
            if path_find
                .car_path_connections
                .get(connection_index)
                .is_some_and(|link| *link == car_link_index as i16)
            {
                nodes.push(node_index);
            }
        }
    }
    nodes
}

fn entity_right(entity: &CWorldEntity) -> [f32; 3] {
    if let Some([right, _, _]) = entity.matrix_basis_override {
        return right;
    }
    rotate_axis_angle(
        [1.0, 0.0, 0.0],
        entity.rotation_axis,
        entity.rotation_degrees,
    )
}

fn entity_forward(entity: &CWorldEntity) -> [f32; 3] {
    if let Some([_, forward, _]) = entity.matrix_basis_override {
        return forward;
    }
    rotate_axis_angle(
        [0.0, 1.0, 0.0],
        entity.rotation_axis,
        entity.rotation_degrees,
    )
}

fn rotate_axis_angle(value: [f32; 3], axis: [f32; 3], degrees: f32) -> [f32; 3] {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if len <= f32::EPSILON || degrees.abs() <= f32::EPSILON {
        return value;
    }

    let q = [axis[0] / len, axis[1] / len, axis[2] / len];
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    let dot = q[0] * value[0] + q[1] * value[1] + q[2] * value[2];
    let crossed = cross(q, value);
    [
        value[0] * cos + crossed[0] * sin + q[0] * dot * (1.0 - cos),
        value[1] * cos + crossed[1] * sin + q[1] * dot * (1.0 - cos),
        value[2] * cos + crossed[2] * sin + q[2] * dot * (1.0 - cos),
    ]
}

fn dot2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::PathFind::CCarPathLink;

    #[test]
    fn bridge_holds_cars_entering_a_closed_link_only() {
        let mut paths = CPathFind::default();
        paths.car_path_links = vec![
            CCarPathLink {
                pos: [0.0; 2], dir: [0.0; 2], path_node_index: 0,
                num_left_lanes: 0, num_right_lanes: 0,
                traffic_light_type: 0, bridge_lights: false,
            },
            CCarPathLink {
                pos: [0.0; 2], dir: [0.0; 2], path_node_index: 1,
                num_left_lanes: 0, num_right_lanes: 0,
                traffic_light_type: 0, bridge_lights: true,
            },
        ];
        assert!(CTrafficLights::should_car_stop_for_bridge_links(&paths, 1, 0));
        assert!(!CTrafficLights::should_car_stop_for_bridge_links(&paths, 0, 1));
        assert!(!CTrafficLights::should_car_stop_for_bridge_links(&paths, 1, 1));
        paths.car_path_links[1].bridge_lights = false;
        assert!(!CTrafficLights::should_car_stop_for_bridge_links(&paths, 1, 0));
    }

    #[test]
    fn light_phase_boundaries_match_re3() {
        assert_eq!(CTrafficLights::light_for_cars1(4_999), CAR_LIGHTS_GREEN);
        assert_eq!(CTrafficLights::light_for_cars1(5_000), CAR_LIGHTS_YELLOW);
        assert_eq!(CTrafficLights::light_for_cars1(6_000), CAR_LIGHTS_RED);

        assert_eq!(CTrafficLights::light_for_cars2(5_999), CAR_LIGHTS_RED);
        assert_eq!(CTrafficLights::light_for_cars2(6_000), CAR_LIGHTS_GREEN);
        assert_eq!(CTrafficLights::light_for_cars2(11_000), CAR_LIGHTS_YELLOW);
        assert_eq!(CTrafficLights::light_for_cars2(12_000), CAR_LIGHTS_RED);

        assert_eq!(CTrafficLights::light_for_peds(11_999), PED_LIGHTS_DONT_WALK);
        assert_eq!(CTrafficLights::light_for_peds(12_000), PED_LIGHTS_WALK);
        assert_eq!(
            CTrafficLights::light_for_peds(15_384),
            PED_LIGHTS_WALK_BLINK
        );
    }

    #[test]
    fn traffic_light_basis_uses_world_axis_angle_sign() {
        let forward = rotate_axis_angle([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], -90.0);
        assert!((forward[0] - 1.0).abs() < 1.0e-5);
        assert!(forward[1].abs() < 1.0e-5);
        assert!(forward[2].abs() < 1.0e-5);
    }
}
