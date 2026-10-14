//! re3 reference: references/re-GTA/src/control/CarCtrl.cpp
//! re3 reference: references/re-GTA/src/control/CarCtrl.h
//! re3 reference: references/re-GTA/src/control/Curves.cpp
//! re3 reference: references/re-GTA/src/vehicles/Automobile.cpp (case
//! STATUS_SIMPLE of CAutomobile::ProcessControl)
//!
//! On-rails movement support for STATUS_SIMPLE traffic cars.  Node picks,
//! curve evaluation and traffic slow-down math follow re3 exactly, including
//! the FIX_BUGS lane branch (re3 `core/config.h:267` defines `FIX_BUGS`;
//! `FIX_PATHFIND_BUG` is not defined).  Entity-list ownership stays with the
//! Bevy system in `core/Game.rs`; this module only owns the pure behavior.

use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};

use crate::control::AutoPilot::{CAutoPilot, LANE_WIDTH, NUM_PATH_NODES_IN_AUTOPILOT, eCarMission};
use crate::control::Curves::CCurves;
use crate::control::PathFind::{CCarCreationCoors, CCarPathLink, CPathFind};

/// re3 `CarCtrl.h:6`.
pub const GAME_SPEED_TO_METERS_PER_SECOND: f32 = 50.0;
/// re3 `CarCtrl.cpp:36`.
pub const DISTANCE_TO_SCAN_FOR_DANGER: f32 = 11.0;
/// re3 `CarCtrl.cpp:37`.
pub const SAFE_DISTANCE_TO_PED: f32 = 3.0;
/// re3 `CarCtrl.cpp:47-50`.
pub const PATH_DIRECTION_NONE: u8 = 0;
pub const PATH_DIRECTION_STRAIGHT: u8 = 1;
pub const PATH_DIRECTION_RIGHT: u8 = 2;
pub const PATH_DIRECTION_LEFT: u8 = 4;
/// re3 `CarCtrl.cpp:52`.
pub const ATTEMPTS_TO_FIND_NEXT_NODE: usize = 15;

/// re3 `CCarCtrl::MapCouldMoveInThisArea` (`control/CarCtrl.cpp:2820-2825`).
/// The bridge corridor is animated by `CBridge::Update`; abandoned/wrecked
/// vehicles in that rectangle must stay awake so they follow the moving road
/// instead of entering the ordinary static rest state.
pub fn map_could_move_in_this_area(x: f32, y: f32) -> bool {
    x > -342.0 && x < -219.0 && y > -677.0 && y < -580.0
}

/// re3 `CCarPathLink::OneWayLaneOffset` (`control/PathFind.h:114-120`).
pub fn one_way_lane_offset(link: &CCarPathLink) -> f32 {
    if link.num_left_lanes == 0 {
        0.5 - 0.5 * link.num_right_lanes as f32
    } else if link.num_right_lanes == 0 {
        0.5 - 0.5 * link.num_left_lanes as f32
    } else {
        0.5
    }
}

/// re3 `CCarCtrl::FindPathDirection` (`CarCtrl.cpp:1677-1702`).
pub fn find_path_direction(
    paths: &CPathFind,
    prev_node: usize,
    cur_node: usize,
    next_node: usize,
) -> u8 {
    let (Some(prev), Some(cur), Some(next)) = (
        paths.path_nodes.get(prev_node),
        paths.path_nodes.get(cur_node),
        paths.path_nodes.get(next_node),
    ) else {
        return PATH_DIRECTION_NONE;
    };
    let prev_to_cur_x = cur.pos.x - prev.pos.x;
    let prev_to_cur_y = cur.pos.y - prev.pos.y;
    let dist_prev_to_cur = (prev_to_cur_x * prev_to_cur_x + prev_to_cur_y * prev_to_cur_y).sqrt();
    if dist_prev_to_cur == 0.0 {
        return PATH_DIRECTION_NONE;
    }
    let sin_a = prev_to_cur_x / dist_prev_to_cur;
    let cos_a = prev_to_cur_y / dist_prev_to_cur;
    let cur_to_next_x = next.pos.x - cur.pos.x;
    let cur_to_next_y = next.pos.y - cur.pos.y;
    let dist_cur_to_next = (cur_to_next_x * cur_to_next_x + cur_to_next_y * cur_to_next_y).sqrt();
    if dist_cur_to_next == 0.0 {
        return PATH_DIRECTION_NONE;
    }
    let sin_b = cur_to_next_y / dist_cur_to_next;
    let cos_b = cur_to_next_x / dist_cur_to_next;
    let sin_direction = sin_a * sin_b - cos_a * cos_b;
    if sin_direction > 0.77 {
        return PATH_DIRECTION_LEFT;
    }
    if sin_direction < -0.77 {
        return PATH_DIRECTION_RIGHT;
    }
    PATH_DIRECTION_STRAIGHT
}

/// Outcome flags for node picks that re3 applies inline through the entity
/// pointer (`SetStatus(STATUS_PHYSICS)` on a forced U-turn,
/// `CarCtrl.cpp:1602-1606`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PickNodeResult {
    pub switched_to_physics: bool,
    /// re3 `PickNextNodeToFollowPath` returns true when the path search must
    /// restart next frame (`CarCtrl.cpp:1863-1864`).
    pub restart_path_search: bool,
}

/// re3 advances the route history in every pick function
/// (`CarCtrl.cpp:1607-1612`, `1784-1787`, `1868-1877`).
fn shift_route_history(auto_pilot: &mut CAutoPilot) {
    auto_pilot.time_entered_curve = auto_pilot
        .time_entered_curve
        .wrapping_add(auto_pilot.time_to_spend_on_current_curve);
    auto_pilot.previous_path_node_info = auto_pilot.current_path_node_info;
    auto_pilot.current_path_node_info = auto_pilot.next_path_node_info;
    auto_pilot.previous_direction = auto_pilot.current_direction;
    auto_pilot.current_direction = auto_pilot.next_direction;
    auto_pilot.current_lane = auto_pilot.next_lane;
}

fn finish_pick_with_link(
    paths: &CPathFind,
    auto_pilot: &mut CAutoPilot,
    cur_node: usize,
    next_link_index: i16,
    cur_link: CCarPathLink,
    general: &mut CGeneral,
    lane_switch_distance_sqr: f32,
) -> PickNodeResult {
    let Some(next_link) = paths
        .car_path_links
        .get(next_link_index.max(0) as usize)
        .copied()
    else {
        return PickNodeResult::default();
    };
    shift_route_history(auto_pilot);
    auto_pilot.next_path_node_info = next_link_index.max(0) as u32;
    let next_node_index = auto_pilot.next_route_node.max(0) as usize;
    let lanes_on_next_node;
    if cur_node >= next_node_index {
        auto_pilot.next_direction = 1;
        lanes_on_next_node = next_link.num_left_lanes;
    } else {
        auto_pilot.next_direction = -1;
        lanes_on_next_node = next_link.num_right_lanes;
    }
    let current_forward_x = auto_pilot.current_direction as f32 * cur_link.dir[0];
    let next_forward_x = auto_pilot.next_direction as f32 * next_link.dir[0];
    // FIX_BUGS branch (`CarCtrl.cpp:1624-1627`).
    let current_forward_y = auto_pilot.current_direction as f32 * cur_link.dir[1];
    let next_forward_y = auto_pilot.next_direction as f32 * next_link.dir[1];
    if lanes_on_next_node >= 0 {
        if general.get_random_number() & 0x600 == 0 {
            let node_distance = paths
                .path_nodes
                .get(next_node_index)
                .zip(paths.path_nodes.get(cur_node))
                .map(|(next, cur)| {
                    let dx = next.pos.x - cur.pos.x;
                    let dy = next.pos.y - cur.pos.y;
                    dx * dx + dy * dy
                })
                .unwrap_or(0.0);
            if node_distance >= lane_switch_distance_sqr && general.get_random_true_false() {
                auto_pilot.next_lane += 1;
            } else if node_distance >= lane_switch_distance_sqr {
                auto_pilot.next_lane -= 1;
            }
        }
        auto_pilot.next_lane = (lanes_on_next_node - 1).min(auto_pilot.next_lane).max(0);
    } else {
        auto_pilot.next_lane = auto_pilot.current_lane;
    }
    if auto_pilot.stay_in_fast_lane {
        auto_pilot.next_lane = 0;
    }
    // FIX_BUGS branch for the lane offset terms (`CarCtrl.cpp:1646-1659`).
    let position_on_current_link = CVector::new(
        cur_link.pos[0]
            + (auto_pilot.current_lane as f32 + one_way_lane_offset(&cur_link))
                * LANE_WIDTH
                * current_forward_y,
        cur_link.pos[1]
            - (auto_pilot.current_lane as f32 + one_way_lane_offset(&cur_link))
                * LANE_WIDTH
                * current_forward_x,
        0.0,
    );
    let position_on_next_link = CVector::new(
        next_link.pos[0]
            + (auto_pilot.next_lane as f32 + one_way_lane_offset(&next_link))
                * LANE_WIDTH
                * next_forward_y,
        next_link.pos[1]
            - (auto_pilot.next_lane as f32 + one_way_lane_offset(&next_link))
                * LANE_WIDTH
                * next_forward_x,
        0.0,
    );
    auto_pilot.time_to_spend_on_current_curve = (CCurves::CalcSpeedScaleFactor(
        position_on_current_link,
        position_on_next_link,
        current_forward_x,
        current_forward_y,
        next_forward_x,
        next_forward_y,
    ) * (1000.0 / auto_pilot.max_traffic_speed))
        as i32;
    auto_pilot.time_to_spend_on_current_curve = auto_pilot.time_to_spend_on_current_curve.max(10);
    PickNodeResult::default()
}

/// re3 `CCarCtrl::PickNextNodeRandomly` (`CarCtrl.cpp:1510-1675`).
pub fn pick_next_node_randomly(
    paths: &CPathFind,
    auto_pilot: &mut CAutoPilot,
    general: &mut CGeneral,
) -> PickNodeResult {
    let prev_node = auto_pilot.current_route_node.max(0) as usize;
    let cur_node = auto_pilot.next_route_node.max(0) as usize;
    let total_links = paths
        .path_nodes
        .get(cur_node)
        .map(|node| node.num_links as usize)
        .unwrap_or(0);
    let Some(cur_link) = paths
        .car_path_links
        .get(auto_pilot.next_path_node_info as usize)
        .copied()
    else {
        return PickNodeResult::default();
    };
    // FIX_BUGS branch (`CarCtrl.cpp:1516-1518`).
    let lanes_on_current_path = if cur_link.path_node_index as usize == cur_node {
        cur_link.num_left_lanes
    } else {
        cur_link.num_right_lanes
    };
    let mut allowed_directions = PATH_DIRECTION_NONE;
    if auto_pilot.next_lane == 0 {
        // We are always allowed to turn left from the leftmost lane.
        allowed_directions |= PATH_DIRECTION_LEFT;
    }
    if auto_pilot.next_lane == lanes_on_current_path - 1 {
        // We are always allowed to turn right from the rightmost lane.
        allowed_directions |= PATH_DIRECTION_RIGHT;
    }
    if lanes_on_current_path < 3 || allowed_directions == PATH_DIRECTION_NONE {
        // We are always allowed to go straight on a one/two-laned road or
        // from one of the middle lanes of the road.
        allowed_directions |= PATH_DIRECTION_STRAIGHT;
    }
    auto_pilot.prev_route_node = auto_pilot.current_route_node;
    auto_pilot.current_route_node = auto_pilot.next_route_node;

    let first_link = paths
        .path_nodes
        .get(cur_node)
        .map(|node| node.first_link.max(0) as usize)
        .unwrap_or(0);
    let (prev_dead_end, prev_disabled, prev_between_levels) = paths
        .path_nodes
        .get(prev_node)
        .map(|node| (node.dead_end, node.disabled, node.between_levels))
        .unwrap_or((false, false, false));
    let stay_in_current_level = auto_pilot.stay_in_current_level;
    let mut going_against_one_way_road = false;
    let mut next_link: i32 = 0;
    let mut direction = PATH_DIRECTION_NONE;
    let mut attempt = 0usize;
    let mut link_index_for_next: Option<i16> = None;
    while attempt < ATTEMPTS_TO_FIND_NEXT_NODE {
        if attempt != 0 && auto_pilot.next_route_node != prev_node as i32 {
            let node_ok = paths
                .path_nodes
                .get(auto_pilot.next_route_node.max(0) as usize)
                .map(|node| {
                    (!node.dead_end || prev_dead_end)
                        && (!node.disabled || prev_disabled)
                        && (!node.between_levels || prev_between_levels || !stay_in_current_level)
                })
                .unwrap_or(false);
            if node_ok && direction & allowed_directions != 0 && !going_against_one_way_road {
                break;
            }
        }
        next_link = if total_links == 0 {
            0
        } else {
            (general.get_random_number() as usize % total_links) as i32
        };
        auto_pilot.next_route_node = paths
            .connected_node(first_link + next_link as usize)
            .map(|node| node as i32)
            .unwrap_or(0);
        direction = find_path_direction(
            paths,
            prev_node,
            cur_node,
            auto_pilot.next_route_node.max(0) as usize,
        );
        let link_index = paths
            .car_path_connections
            .get(first_link + next_link as usize)
            .copied()
            .unwrap_or_default();
        going_against_one_way_road = paths
            .car_path_links
            .get(link_index.max(0) as usize)
            .map(|link| {
                if link.path_node_index as usize == cur_node {
                    link.num_right_lanes == 0
                } else {
                    link.num_left_lanes == 0
                }
            })
            .unwrap_or(false);
        link_index_for_next = Some(link_index);
        attempt += 1;
    }
    if attempt >= ATTEMPTS_TO_FIND_NEXT_NODE {
        // Failed 15 times: remove the dead-end and current-lane limitations
        // (`CarCtrl.cpp:1564-1581`).
        attempt = 0;
        while attempt < ATTEMPTS_TO_FIND_NEXT_NODE {
            if attempt != 0 && auto_pilot.next_route_node != prev_node as i32 {
                let node_ok = paths
                    .path_nodes
                    .get(auto_pilot.next_route_node.max(0) as usize)
                    .map(|node| {
                        (!node.disabled || prev_disabled)
                            && (!node.between_levels
                                || prev_between_levels
                                || !stay_in_current_level)
                    })
                    .unwrap_or(false);
                if node_ok && !going_against_one_way_road {
                    break;
                }
            }
            next_link = if total_links == 0 {
                0
            } else {
                (general.get_random_number() as usize % total_links) as i32
            };
            auto_pilot.next_route_node = paths
                .connected_node(first_link + next_link as usize)
                .map(|node| node as i32)
                .unwrap_or(0);
            let link_index = paths
                .car_path_connections
                .get(first_link + next_link as usize)
                .copied()
                .unwrap_or_default();
            going_against_one_way_road = paths
                .car_path_links
                .get(link_index.max(0) as usize)
                .map(|link| {
                    if link.path_node_index as usize == cur_node {
                        link.num_right_lanes == 0
                    } else {
                        link.num_left_lanes == 0
                    }
                })
                .unwrap_or(false);
            link_index_for_next = Some(link_index);
            attempt += 1;
        }
    }
    if attempt >= ATTEMPTS_TO_FIND_NEXT_NODE {
        // Remove the no U-turn limitation and the randomness
        // (`CarCtrl.cpp:1582-1599`).
        let mut found = false;
        for link in 0..total_links as i32 {
            auto_pilot.next_route_node = paths
                .connected_node(first_link + link as usize)
                .map(|node| node as i32)
                .unwrap_or(0);
            let link_index = paths
                .car_path_connections
                .get(first_link + link as usize)
                .copied()
                .unwrap_or_default();
            going_against_one_way_road = paths
                .car_path_links
                .get(link_index.max(0) as usize)
                .map(|link| {
                    if link.path_node_index as usize == cur_node {
                        link.num_right_lanes == 0
                    } else {
                        link.num_left_lanes == 0
                    }
                })
                .unwrap_or(false);
            if !going_against_one_way_road {
                let node_ok = paths
                    .path_nodes
                    .get(auto_pilot.next_route_node.max(0) as usize)
                    .map(|node| {
                        (!node.disabled || prev_disabled)
                            && (!node.between_levels
                                || prev_between_levels
                                || !stay_in_current_level)
                    })
                    .unwrap_or(false);
                if node_ok {
                    // "Nice way to exit loop but this will fail because this
                    // is used for indexing!" (CarCtrl.cpp:1592).
                    next_link = 1000;
                    link_index_for_next = Some(link_index);
                    found = true;
                }
            }
        }
        if !found && next_link < 999 {
            // Everything else failed: turn the vehicle around.
            auto_pilot.next_route_node = prev_node as i32;
        }
    }

    if prev_node == auto_pilot.next_route_node.max(0) as usize {
        // We can no longer shift the vehicle without physics on a U-turn.
        return PickNodeResult {
            switched_to_physics: true,
            restart_path_search: false,
        };
    }

    let Some(next_link_index) = link_index_for_next else {
        return PickNodeResult::default();
    };
    finish_pick_with_link(
        paths,
        auto_pilot,
        cur_node,
        next_link_index,
        cur_link,
        general,
        14.0 * 14.0,
    )
}

/// re3 `CCarCtrl::PickNextNodeToChaseCar` (`CarCtrl.cpp:1705-1853`,
/// non-FIX_PATHFIND_BUG two-coordinate overload).
pub fn pick_next_node_to_chase_car(
    paths: &mut CPathFind,
    auto_pilot: &mut CAutoPilot,
    target_x: f32,
    target_y: f32,
    vehicle_position: CVector,
    general: &mut CGeneral,
) {
    let prev_node = auto_pilot.current_route_node.max(0) as usize;
    let cur_node = auto_pilot.next_route_node.max(0) as usize;
    let Some(cur_path_node) = paths.path_nodes.get(cur_node).copied() else {
        return;
    };
    let search = paths.do_path_search(
        0,
        cur_path_node.pos,
        Some(cur_node),
        CVector::new(target_x, target_y, 0.0),
        1,
        999_999.9,
        None,
    );
    let first_node = search.nodes.first().copied();
    let found_target = search.num_nodes == 1 && first_node.is_some_and(|node| node != cur_node);
    let mut next_link: i32 = 0;
    let new_next_node: i32;
    if !found_target {
        let current_angle =
            CGeneral::get_atan_of_xy(target_x - vehicle_position.x, target_y - vehicle_position.y);
        let mut lowest_angle_change = 10.0_f32;
        let mut candidate_node = 0_i32;
        let first_link = cur_path_node.first_link.max(0) as usize;
        let num_links = cur_path_node.num_links as usize;
        for offset in 0..num_links {
            let Some(con_node) = paths.connected_node(first_link + offset) else {
                continue;
            };
            if con_node == prev_node && offset > 1 {
                continue;
            }
            let Some(test_node) = paths.path_nodes.get(con_node) else {
                continue;
            };
            let mut angle = CGeneral::get_atan_of_xy(
                test_node.pos.x - cur_path_node.pos.x,
                test_node.pos.y - cur_path_node.pos.y,
            );
            angle = limit_radian_angle(angle - current_angle);
            angle = angle.abs();
            if angle < lowest_angle_change {
                lowest_angle_change = angle;
                candidate_node = con_node as i32;
                next_link = offset as i32;
            }
        }
        new_next_node = candidate_node;
    } else {
        new_next_node = first_node.unwrap_or(0) as i32;
        let first_link = cur_path_node.first_link.max(0) as usize;
        let mut offset = first_link;
        while paths.connected_node(offset) != Some(new_next_node as usize) {
            offset += 1;
            next_link += 1;
        }
    }
    let Some(cur_link) = paths
        .car_path_links
        .get(auto_pilot.next_path_node_info as usize)
        .copied()
    else {
        return;
    };
    auto_pilot.prev_route_node = auto_pilot.current_route_node;
    auto_pilot.current_route_node = auto_pilot.next_route_node;
    auto_pilot.next_route_node = new_next_node;
    let next_link_index = paths
        .car_path_connections
        .get(cur_path_node.first_link.max(0) as usize + next_link.max(0) as usize)
        .copied()
        .unwrap_or_default();
    finish_pick_with_link(
        paths,
        auto_pilot,
        cur_node,
        next_link_index,
        cur_link,
        general,
        7.0 * 7.0,
    );
}

/// re3 `CCarCtrl::PickNextNodeToFollowPath` (`CarCtrl.cpp:1854-1976`).
pub fn pick_next_node_to_follow_path(
    paths: &mut CPathFind,
    auto_pilot: &mut CAutoPilot,
    position: CVector,
    general: &mut CGeneral,
) -> PickNodeResult {
    let cur_node = auto_pilot.next_route_node.max(0) as usize;
    let Some(cur_path_node) = paths.path_nodes.get(cur_node).copied() else {
        return PickNodeResult::default();
    };
    if auto_pilot.path_find_nodes_count == 0 {
        let search = paths.do_path_search(
            0,
            position,
            Some(cur_node),
            auto_pilot.destination_coors,
            NUM_PATH_NODES_IN_AUTOPILOT,
            999_999.9,
            None,
        );
        if search.nodes.is_empty() {
            return PickNodeResult {
                restart_path_search: true,
                ..Default::default()
            };
        }
        auto_pilot.path_find_nodes = [None; NUM_PATH_NODES_IN_AUTOPILOT];
        for (slot, node) in auto_pilot
            .path_find_nodes
            .iter_mut()
            .zip(search.nodes.iter().copied())
        {
            *slot = Some(node);
        }
        auto_pilot.path_find_nodes_count = search.nodes.len() as i16;
    }
    let Some(cur_link) = paths
        .car_path_links
        .get(auto_pilot.next_path_node_info as usize)
        .copied()
    else {
        return PickNodeResult::default();
    };
    auto_pilot.prev_route_node = auto_pilot.current_route_node;
    auto_pilot.current_route_node = auto_pilot.next_route_node;
    auto_pilot.next_route_node = auto_pilot
        .path_find_nodes
        .first()
        .copied()
        .flatten()
        .map(|node| node as i32)
        .unwrap_or(0);
    auto_pilot.RemoveOnePathNode();
    let mut next_link = 0_i32;
    let first_link = cur_path_node.first_link.max(0) as usize;
    let mut offset = first_link;
    while paths.connected_node(offset) != Some(auto_pilot.next_route_node.max(0) as usize) {
        offset += 1;
        next_link += 1;
    }
    let next_link_index = paths
        .car_path_connections
        .get(first_link + next_link.max(0) as usize)
        .copied()
        .unwrap_or_default();
    finish_pick_with_link(
        paths,
        auto_pilot,
        cur_node,
        next_link_index,
        cur_link,
        general,
        7.0 * 7.0,
    )
}

/// re3 `CCarCtrl::PickNextNodeAccordingStrategy` (`CarCtrl.cpp:1478-1508`).
/// The RAMCAR target position is the live target car position
/// (`m_pTargetCar->GetPosition()`), supplied by the caller from the ECS pool.
pub fn pick_next_node_according_to_strategy(
    paths: &mut CPathFind,
    auto_pilot: &mut CAutoPilot,
    vehicle_position: CVector,
    player_position: CVector,
    target_car_position: Option<CVector>,
    general: &mut CGeneral,
) -> PickNodeResult {
    match auto_pilot.car_mission {
        eCarMission::MISSION_RAMPLAYER_FARAWAY | eCarMission::MISSION_BLOCKPLAYER_FARAWAY => {
            pick_next_node_to_chase_car(
                paths,
                auto_pilot,
                player_position.x,
                player_position.y,
                vehicle_position,
                general,
            );
            PickNodeResult::default()
        }
        eCarMission::MISSION_RAMCAR_FARAWAY | eCarMission::MISSION_BLOCKCAR_FARAWAY => {
            let (target_x, target_y) = target_car_position
                .map(|position| (position.x, position.y))
                .unwrap_or((vehicle_position.x, vehicle_position.y));
            pick_next_node_to_chase_car(
                paths,
                auto_pilot,
                target_x,
                target_y,
                vehicle_position,
                general,
            );
            PickNodeResult::default()
        }
        eCarMission::MISSION_GOTOCOORDS | eCarMission::MISSION_GOTOCOORDS_ACCURATE => {
            pick_next_node_to_follow_path(paths, auto_pilot, vehicle_position, general)
        }
        _ => pick_next_node_randomly(paths, auto_pilot, general),
    }
}

/// re3 `CCarCtrl::LimitRadianAngle` (`CarCtrl.h:115-122`).
pub fn limit_radian_angle(mut angle: f32) -> f32 {
    while angle < -std::f32::consts::PI {
        angle += std::f32::consts::TAU;
    }
    while angle > std::f32::consts::PI {
        angle -= std::f32::consts::TAU;
    }
    angle
}

/// re3 `CCarCtrl::TestCollisionBetween2MovingRects` (`CarCtrl.cpp:1116-1237`).
///
/// Naming follows the C++ exactly: `pForwardA`/`pForwardB` are independent
/// direction arguments (the callers pass the slowed vehicle's and the other
/// vehicle's forwards in swapped pairings), while lengths/widths always come
/// from vehicle A and vehicle B.  Corner offsets are copied verbatim,
/// including the case-1 `widthA * forward_b.x` term.
#[allow(clippy::too_many_arguments)]
pub fn test_collision_between_2_moving_rects(
    position_a: CVector,
    bbox_min_a: CVector,
    bbox_max_a: CVector,
    position_b: CVector,
    bbox_min_b: CVector,
    bbox_max_b: CVector,
    forward_a: CVector,
    forward_b: CVector,
    projection_x: f32,
    projection_y: f32,
) -> f32 {
    let vec_b_to_a_x = position_a.x - position_b.x;
    let vec_b_to_a_y = position_a.y - position_b.y;
    let len_b = bbox_max_b.y;
    let width_b = bbox_max_b.x;
    let back_len_b = -bbox_min_b.y;
    let len_a = bbox_max_a.y;
    let width_a = bbox_max_a.x;
    let back_len_a = -bbox_min_a.y;
    let mut proximity = 1.0_f32;
    let full_width_b = 2.0 * width_b;
    let full_len_b = len_b + back_len_b;
    for corner in 0..4usize {
        let (tested_offset_x, tested_offset_y) = match corner {
            // Front right corner
            0 => (
                vec_b_to_a_x + width_a * forward_b.y + len_a * forward_b.x,
                vec_b_to_a_y + len_a * forward_b.y - width_a * forward_b.x,
            ),
            // Front left corner
            1 => (
                vec_b_to_a_x + -width_a * forward_b.x + len_a * forward_b.x,
                vec_b_to_a_y + len_a * forward_b.y + width_a * forward_b.x,
            ),
            // Rear right corner
            2 => (
                vec_b_to_a_x + width_a * forward_b.y - back_len_a * forward_b.x,
                vec_b_to_a_y - back_len_a * forward_b.y - width_a * forward_b.x,
            ),
            // Rear left corner
            _ => (
                vec_b_to_a_x - width_a * forward_b.y - back_len_a * forward_b.x,
                vec_b_to_a_y - back_len_a * forward_b.y + width_a * forward_b.x,
            ),
        };
        // Testing width collision
        let mut base_width_proximity = 0.0_f32;
        let mut full_width_proximity = 1.0_f32;
        let width_distance = tested_offset_x * forward_a.y - tested_offset_y * forward_a.x;
        let width_projection = projection_x * forward_a.y - projection_y * forward_a.x;
        if width_distance > width_b {
            if width_projection < 0.0 {
                let proximity_width = -(width_distance - width_b) / width_projection;
                if proximity_width < 1.0 {
                    base_width_proximity = proximity_width;
                    full_width_proximity =
                        (proximity_width - full_width_b / width_projection).min(1.0);
                } else {
                    base_width_proximity = 1.0;
                }
            } else {
                base_width_proximity = 1.0;
                full_width_proximity = 1.0;
            }
        } else if width_distance < -width_b {
            if width_projection > 0.0 {
                let proximity_width = -(width_distance + width_b) / width_projection;
                if proximity_width < 1.0 {
                    base_width_proximity = proximity_width;
                    full_width_proximity =
                        (proximity_width + full_width_b / width_projection).min(1.0);
                } else {
                    base_width_proximity = 1.0;
                }
            } else {
                base_width_proximity = 1.0;
                full_width_proximity = 1.0;
            }
        } else if width_projection > 0.0 {
            full_width_proximity = (width_b - width_distance) / width_projection;
        } else if width_projection < 0.0 {
            full_width_proximity = -(width_b + width_distance) / width_projection;
        }
        // Testing length collision
        let mut base_length_proximity = 0.0_f32;
        let mut full_length_proximity = 1.0_f32;
        let len_distance = tested_offset_x * forward_a.x + tested_offset_y * forward_a.y;
        let len_projection = projection_x * forward_a.x + projection_y * forward_a.y;
        if len_distance > len_b {
            if len_projection < 0.0 {
                let proximity_length = -(len_distance - len_b) / len_projection;
                if proximity_length < 1.0 {
                    base_length_proximity = proximity_length;
                    full_length_proximity =
                        (proximity_length - full_len_b / len_projection).min(1.0);
                } else {
                    base_length_proximity = 1.0;
                }
            } else {
                base_length_proximity = 1.0;
                full_length_proximity = 1.0;
            }
        } else if len_distance < -back_len_b {
            if len_projection > 0.0 {
                let proximity_length = -(len_distance + back_len_b) / len_projection;
                if proximity_length < 1.0 {
                    base_length_proximity = proximity_length;
                    full_length_proximity =
                        (proximity_length + full_len_b / len_projection).min(1.0);
                } else {
                    base_length_proximity = 1.0;
                }
            } else {
                base_length_proximity = 1.0;
                full_length_proximity = 1.0;
            }
        } else if len_projection > 0.0 {
            full_length_proximity = (len_b - len_distance) / len_projection;
        } else if len_projection < 0.0 {
            full_length_proximity = -(back_len_b + len_distance) / len_projection;
        }
        let base_proximity = base_width_proximity.max(base_length_proximity);
        if base_proximity < full_width_proximity && base_proximity < full_length_proximity {
            proximity = proximity.min(base_proximity);
        }
    }
    proximity
}

/// Speed clamp of re3 `CCarCtrl::SlowCarDownForOtherCar` (`CarCtrl.cpp:1075-1114`).
/// Returns `Some(switch_to_real_physics)` for the 15-second standing-car
/// annoyance branch (`CarCtrl.cpp:1098-1113`); the caller applies entity
/// status and the driving-style change.
#[allow(clippy::too_many_arguments)]
pub fn slow_car_down_for_other_car(
    other_position: CVector,
    other_forward: CVector,
    other_move_speed: CVector,
    other_bbox_min: CVector,
    other_bbox_max: CVector,
    position: CVector,
    forward: CVector,
    bbox_min: CVector,
    bbox_max: CVector,
    auto_pilot: &mut CAutoPilot,
    speed: &mut f32,
    cur_speed: f32,
) -> bool {
    let mut forward_a = CVector::new(forward.x, forward.y, 0.0);
    let forward_a_len2 = forward_a.x * forward_a.x + forward_a.y * forward_a.y;
    if forward_a_len2 > f32::EPSILON {
        let len = forward_a_len2.sqrt();
        forward_a.x /= len;
        forward_a.y /= len;
    }
    let to_other_x = other_position.x - position.x;
    let to_other_y = other_position.y - position.y;
    if to_other_x * forward_a.x + to_other_y * forward_a.y < 0.0 {
        return false;
    }
    let mut forward_b = CVector::new(other_forward.x, other_forward.y, 0.0);
    let forward_b_len2 = forward_b.x * forward_b.x + forward_b.y * forward_b.y;
    if forward_b_len2 > f32::EPSILON {
        let len = forward_b_len2.sqrt();
        forward_b.x /= len;
        forward_b.y /= len;
    }
    let speed_other_x = 60.0 * other_move_speed.x;
    let speed_other_y = 60.0 * other_move_speed.y;
    let projection_x = speed_other_x - forward_a.x * cur_speed;
    let projection_y = speed_other_y - forward_a.y * cur_speed;
    let proximity_a = test_collision_between_2_moving_rects(
        other_position,
        other_bbox_min,
        other_bbox_max,
        position,
        bbox_min,
        bbox_max,
        forward_a,
        forward_b,
        projection_x,
        projection_y,
    );
    let proximity_b = test_collision_between_2_moving_rects(
        position,
        bbox_min,
        bbox_max,
        other_position,
        other_bbox_min,
        other_bbox_max,
        forward_b,
        forward_a,
        -projection_x,
        -projection_y,
    );
    let mut min_proximity = proximity_a.min(proximity_b);
    if (0.0..1.0).contains(&min_proximity) {
        min_proximity = ((min_proximity - 0.2) * 1.25).max(0.0);
        auto_pilot.slowed_down_because_of_cars = true;
        *speed = (*speed).min(min_proximity * cur_speed);
    }
    let _ = SAFE_DISTANCE_TO_PED;
    min_proximity >= 0.0 && min_proximity < 0.5
}

/// Speed-clamp tail of re3 `CCarCtrl::SlowCarDownForPedsSectorList`
/// (`CarCtrl.cpp:944-1051`).  The matching ped evasive/flee state transition
/// is applied by `core::Game::process_ambient_ped_evasion` after the rails
/// snapshot; this helper keeps only the vehicle-side speed mutation so the
/// CCarCtrl math remains borrow-free and reusable.
#[allow(clippy::too_many_arguments)]
pub fn slow_car_down_for_ped(
    ped_position: CVector,
    position: CVector,
    forward: CVector,
    right: CVector,
    move_speed: CVector,
    _bbox_min: CVector,
    bbox_max: CVector,
    is_bike: bool,
    auto_pilot: &mut CAutoPilot,
    speed: &mut f32,
    cur_speed: f32,
    now_ms: u32,
) {
    let front_offset = bbox_max.y;
    let vec_to_ped_x = ped_position.x - position.x;
    let vec_to_ped_y = ped_position.y - position.y;
    let dot_direction = forward.x * vec_to_ped_x + forward.y * vec_to_ped_y;
    let dot_velocity = forward.x * move_speed.x + forward.y * move_speed.y;
    if dot_direction <= front_offset {
        // If already run him over, don't care (`CarCtrl.cpp:965`).
        return;
    }
    let distance_until_hit = dot_direction - front_offset;
    let movement_towards_ped_per_second = GAME_SPEED_TO_METERS_PER_SECOND * dot_velocity;
    if 4.0 * movement_towards_ped_per_second <= distance_until_hit {
        // If the car isn't projected to hit the ped in 4 seconds, don't care.
        return;
    }
    let sideways_distance = (right.x * vec_to_ped_x + right.y * vec_to_ped_y).abs();
    let mut side_length = bbox_max.x;
    if is_bike {
        side_length *= 1.6;
    }
    if side_length + 0.5 < sideways_distance {
        return;
    }
    let _ = SAFE_DISTANCE_TO_PED;
    if distance_until_hit < 10.0
        && matches!(
            auto_pilot.driving_style,
            eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS
                | eCarDrivingStyle::DRIVINGSTYLE_SLOW_DOWN_FOR_CARS
        )
    {
        // DRIVINGSTYLE_STOP_FOR_CARS / SLOW_DOWN_FOR_CARS branch
        // (`CarCtrl.cpp:1041-1049`).
        *speed = (*speed).min((distance_until_hit - 1.0).abs() * 0.1 * cur_speed);
        auto_pilot.slowed_down_because_of_peds = true;
        if distance_until_hit < 2.0 {
            auto_pilot.temp_action = crate::control::AutoPilot::eCarTempAction::TEMPACT_WAIT;
            auto_pilot.time_temp_action = now_ms + 3000;
        }
    }
}

/// re3 `CCarCtrl::GetPositionAlongCurrentCurve` (`CarCtrl.h:109-113`).
pub fn get_position_along_current_curve(auto_pilot: &CAutoPilot, now_ms: u32) -> f32 {
    let time_in_curve = now_ms.wrapping_sub(auto_pilot.time_entered_curve as u32);
    time_in_curve as f32 / auto_pilot.time_to_spend_on_current_curve.max(1) as f32
}

/// Initialise the route state produced by `CCarCtrl::GenerateOneRandomCar`.
///
/// The spawn routine already selected a valid `(node1,node2,fraction)` pair.
/// Copying that state into the vehicle before the first SIMPLE tick preserves
/// the native spawn interpolation and avoids the old nearest-node teleport.
pub fn initialise_ambient_traffic_car<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    paths: &CPathFind,
    vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    creation: &mut CCarCreationCoors,
    car_length: f32,
    now_ms: u32,
    general: &mut CGeneral,
) -> bool
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let core = &mut *vehicle.core;
    let Ok(current_node) = usize::try_from(creation.node1) else {
        return false;
    };
    let Ok(next_node) = usize::try_from(creation.node2) else {
        return false;
    };
    let Some(current_path_node) = paths.path_nodes.get(current_node).copied() else {
        return false;
    };
    let Some(next_path_node) = paths.path_nodes.get(next_node).copied() else {
        return false;
    };
    let first_link = current_path_node.first_link.max(0) as usize;
    let link_count = current_path_node.num_links as usize;
    let Some(next_offset) = (0..link_count)
        .find(|offset| paths.connected_node(first_link + *offset) == Some(next_node))
    else {
        return false;
    };
    let next_connection = paths
        .car_path_connections
        .get(first_link + next_offset)
        .copied()
        .unwrap_or_default();
    let Some(next_link) = paths
        .car_path_links
        .get(next_connection.max(0) as usize)
        .copied()
    else {
        return false;
    };
    let lanes_on_current_road = if next_link.path_node_index as usize == next_node {
        next_link.num_left_lanes
    } else {
        next_link.num_right_lanes
    };
    if lanes_on_current_road <= 0 {
        return false;
    }

    // GenerateOneRandomCar keeps the full vehicle body between the two route
    // nodes before it turns the spawn fraction into a curve time.
    let between_nodes_x = current_path_node.pos.x - next_path_node.pos.x;
    let between_nodes_y = current_path_node.pos.y - next_path_node.pos.y;
    let distance_between_nodes =
        (between_nodes_x * between_nodes_x + between_nodes_y * between_nodes_y).sqrt();
    if distance_between_nodes * 0.5 < car_length {
        creation.position_between_nodes = 0.5;
    } else if distance_between_nodes > f32::EPSILON {
        let min_fraction = car_length / distance_between_nodes;
        creation.position_between_nodes = creation
            .position_between_nodes
            .clamp(min_fraction, 1.0 - min_fraction);
    }

    // Native consumes the lane RNG before rejecting a one-link start node.
    let initial_lane = (general.get_random_number() % lanes_on_current_road.max(1) as u16) as i8;
    if link_count == 1 {
        return false;
    }

    let mut selected_link = None;
    for _ in 0..link_count.saturating_mul(4).max(8) {
        let candidate = general.get_random_number() as usize % link_count;
        let connection = paths
            .car_path_connections
            .get(first_link + candidate)
            .copied()
            .unwrap_or_default();
        if connection != next_connection {
            selected_link = Some((connection, candidate));
            break;
        }
    }
    let Some((current_connection, current_link_offset)) = selected_link else {
        return false;
    };
    let Some(current_link) = paths
        .car_path_links
        .get(current_connection.max(0) as usize)
        .copied()
    else {
        return false;
    };

    let auto_pilot = &mut core.auto_pilot;
    auto_pilot.prev_route_node = 0;
    auto_pilot.current_route_node = creation.node1;
    auto_pilot.next_route_node = creation.node2;
    auto_pilot.current_path_node_info = current_connection.max(0) as u32;
    auto_pilot.next_path_node_info = next_connection.max(0) as u32;
    auto_pilot.previous_path_node_info = auto_pilot.current_path_node_info;
    auto_pilot.current_direction = if paths
        .connected_node(first_link + current_link_offset)
        .unwrap_or(current_node)
        >= current_node
    {
        1
    } else {
        -1
    };
    auto_pilot.next_direction = if current_node >= next_node { 1 } else { -1 };
    auto_pilot.current_lane = initial_lane;
    auto_pilot.next_lane = auto_pilot.current_lane;
    auto_pilot.path_find_nodes = [None; NUM_PATH_NODES_IN_AUTOPILOT];
    auto_pilot.path_find_nodes_count = 0;

    // GenerateOneRandomCar initialises its world orientation from the node
    // segment, not from the model's previous transform.
    let mut forward = CVector::new(
        next_path_node.pos.x - current_path_node.pos.x,
        next_path_node.pos.y - current_path_node.pos.y,
        0.0,
    );
    let length = (forward.x * forward.x + forward.y * forward.y).sqrt();
    if length <= f32::EPSILON {
        forward = CVector::new(1.0, 0.0, 0.0);
    } else {
        forward.x /= length;
        forward.y /= length;
    }
    let mut matrix = core.physical.entity.matrix;
    matrix.forward = forward;
    matrix.right = CVector::new(forward.y, -forward.x, 0.0);
    matrix.up = CVector::new(0.0, 0.0, 1.0);
    matrix.position = creation.position;
    core.physical.entity.set_matrix(matrix);

    // FIX_BUGS `GenerateOneRandomCar` treats the selected road fraction as
    // two half-curves. Once the spawn lies in the second half, advance the
    // route first; otherwise the first SIMPLE tick sees a curve age greater
    // than one full curve and visibly snaps the car to the next road link.
    if creation.position_between_nodes >= 0.5 {
        let _ = pick_next_node_randomly(paths, auto_pilot, general);
        auto_pilot.time_entered_curve = now_ms.wrapping_sub(
            ((creation.position_between_nodes - 0.5)
                * auto_pilot.time_to_spend_on_current_curve.max(1) as f32) as u32,
        ) as i32;
        return true;
    }

    let current_forward = CVector::new(
        auto_pilot.current_direction as f32 * current_link.dir[0],
        auto_pilot.current_direction as f32 * current_link.dir[1],
        0.0,
    );
    let next_forward = CVector::new(
        auto_pilot.next_direction as f32 * next_link.dir[0],
        auto_pilot.next_direction as f32 * next_link.dir[1],
        0.0,
    );
    let current_lane = auto_pilot.current_lane as f32 + one_way_lane_offset(&current_link);
    let next_lane = auto_pilot.next_lane as f32 + one_way_lane_offset(&next_link);
    let current_pos = CVector::new(
        current_link.pos[0] + current_lane * LANE_WIDTH * current_forward.y,
        current_link.pos[1] - current_lane * LANE_WIDTH * current_forward.x,
        0.0,
    );
    let next_pos = CVector::new(
        next_link.pos[0] + next_lane * LANE_WIDTH * next_forward.y,
        next_link.pos[1] - next_lane * LANE_WIDTH * next_forward.x,
        0.0,
    );
    let max_speed = auto_pilot.max_traffic_speed.max(0.01);
    auto_pilot.time_to_spend_on_current_curve = (CCurves::CalcSpeedScaleFactor(
        current_pos,
        next_pos,
        current_forward.x,
        current_forward.y,
        next_forward.x,
        next_forward.y,
    ) * (1000.0 / max_speed))
        .max(10.0) as i32;
    auto_pilot.time_entered_curve = now_ms.wrapping_sub(
        ((0.5 + creation.position_between_nodes) * auto_pilot.time_to_spend_on_current_curve as f32)
            as u32,
    ) as i32;
    true
}

/// Finish re3 `GenerateOneRandomCar`'s pre-insertion placement.
///
/// The creation coordinate is only the path-search seed. The original computes
/// the current curve point, chooses the static surface nearest the authored road
/// Z (rejecting another bridge/flyover layer farther than 7 m), and gives the
/// car its initial rail velocity before it enters the world.
pub(crate) fn place_ambient_traffic_car_at_creation_curve<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    file_loader: &CFileLoader,
    creation: CCarCreationCoors,
    now_ms: u32,
) -> bool
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let height_above_road = car.height_above_road();
    let core = &mut *car.vehicle.core;
    let Ok(current_node_index) = usize::try_from(creation.node1) else {
        return false;
    };
    let Ok(next_node_index) = usize::try_from(creation.node2) else {
        return false;
    };
    let Some(current_node) = file_loader
        .path_find
        .path_nodes
        .get(current_node_index)
        .copied()
    else {
        return false;
    };
    let Some(next_node) = file_loader
        .path_find
        .path_nodes
        .get(next_node_index)
        .copied()
    else {
        return false;
    };
    let auto_pilot = &core.auto_pilot;
    let Some(current_link) = file_loader
        .path_find
        .car_path_links
        .get(auto_pilot.current_path_node_info as usize)
        .copied()
    else {
        return false;
    };
    let Some(next_link) = file_loader
        .path_find
        .car_path_links
        .get(auto_pilot.next_path_node_info as usize)
        .copied()
    else {
        return false;
    };

    let current_forward = CVector::new(
        auto_pilot.current_direction as f32 * current_link.dir[0],
        auto_pilot.current_direction as f32 * current_link.dir[1],
        0.0,
    );
    let next_forward = CVector::new(
        auto_pilot.next_direction as f32 * next_link.dir[0],
        auto_pilot.next_direction as f32 * next_link.dir[1],
        0.0,
    );
    let current_lane = auto_pilot.current_lane as f32 + one_way_lane_offset(&current_link);
    let next_lane = auto_pilot.next_lane as f32 + one_way_lane_offset(&next_link);
    let current_pos = CVector::new(
        current_link.pos[0] + current_lane * LANE_WIDTH * current_forward.y,
        current_link.pos[1] - current_lane * LANE_WIDTH * current_forward.x,
        0.0,
    );
    let next_pos = CVector::new(
        next_link.pos[0] + next_lane * LANE_WIDTH * next_forward.y,
        next_link.pos[1] - next_lane * LANE_WIDTH * next_forward.x,
        0.0,
    );
    let (position_on_curve, direction_on_curve) = CCurves::CalcCurvePoint(
        current_pos,
        next_pos,
        current_forward,
        next_forward,
        get_position_along_current_curve(auto_pilot, now_ms),
        auto_pilot.time_to_spend_on_current_curve,
    );

    let between_nodes = CVector::new(
        current_node.pos.x - next_node.pos.x,
        current_node.pos.y - next_node.pos.y,
        0.0,
    );
    let between_length =
        (between_nodes.x * between_nodes.x + between_nodes.y * between_nodes.y).sqrt();
    let mut final_position = position_on_curve;
    if between_length > f32::EPSILON {
        final_position.x += between_nodes.x * 2.0 / between_length;
        final_position.y += between_nodes.y * 2.0 / between_length;
    }
    let fraction = creation.position_between_nodes.clamp(0.0, 1.0);
    let expected_road_z = fraction * next_node.pos.z + (1.0 - fraction) * current_node.pos.z;

    let downward = file_loader
        .world
        .process_vertical_line_static(
            [final_position.x, final_position.y, expected_road_z],
            -1000.0,
            &file_loader.model_info,
        )
        .map(|hit| hit.point.z);
    let upward = file_loader
        .world
        .process_vertical_line_static(
            [final_position.x, final_position.y, expected_road_z],
            1000.0,
            &file_loader.model_info,
        )
        .map(|hit| hit.point.z);
    let ground_z = match (downward, upward) {
        (Some(a), Some(b)) => {
            if (a - expected_road_z).abs() <= (b - expected_road_z).abs() {
                a
            } else {
                b
            }
        }
        (Some(z), None) | (None, Some(z)) => z,
        (None, None) => return false,
    };
    if (ground_z - expected_road_z).abs() > 7.0 {
        return false;
    }

    final_position.z = ground_z + height_above_road;
    let mut matrix = core.physical.entity.matrix;
    matrix.position = final_position;
    core.physical.entity.set_matrix(matrix);
    core.physical.move_speed = CVector::new(
        direction_on_curve.x / crate::control::CarAI::GAME_SPEED_TO_CARAI_SPEED,
        direction_on_curve.y / crate::control::CarAI::GAME_SPEED_TO_CARAI_SPEED,
        direction_on_curve.z / crate::control::CarAI::GAME_SPEED_TO_CARAI_SPEED,
    );
    true
}

/// re3 `CCarCtrl::UpdateCarOnRails` (`control/CarCtrl.cpp:806-853`).
///
/// Returns the authored curve position and velocity for the current frame;
/// the ECS owner applies the world-ground projection and writes the physical
/// matrix. Keeping that final write outside this module avoids a second world
/// representation while preserving the exact path/lane/curve state updates.
pub fn update_car_on_rails<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    paths: &mut CPathFind,
    vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    now_ms: u32,
    general: &mut CGeneral,
) -> Option<(CVector, CVector)>
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let core = &mut *vehicle.core;
    let auto_pilot = &mut core.auto_pilot;
    if auto_pilot.temp_action == crate::control::AutoPilot::eCarTempAction::TEMPACT_WAIT {
        core.physical.move_speed = CVector::ZERO;
        auto_pilot.max_traffic_speed = 0.0;
        if now_ms > auto_pilot.time_temp_action {
            auto_pilot.temp_action = crate::control::AutoPilot::eCarTempAction::TEMPACT_NONE;
            auto_pilot.anti_reverse_timer = now_ms;
            auto_pilot.time_to_start_mission = now_ms;
        }
        return None;
    }
    if now_ms.wrapping_sub(auto_pilot.time_entered_curve as u32)
        >= auto_pilot.time_to_spend_on_current_curve.max(1) as u32
    {
        let result = pick_next_node_according_to_strategy(
            paths,
            auto_pilot,
            core.physical.entity.get_position(),
            CVector::ZERO,
            None,
            general,
        );
        if result.switched_to_physics {
            return None;
        }
    }
    let current_link = paths
        .car_path_links
        .get(auto_pilot.current_path_node_info as usize)
        .copied()?;
    let next_link = paths
        .car_path_links
        .get(auto_pilot.next_path_node_info as usize)
        .copied()?;
    let current_direction = auto_pilot.current_direction as f32;
    let next_direction = auto_pilot.next_direction as f32;
    let current_forward = CVector::new(
        current_direction * current_link.dir[0],
        current_direction * current_link.dir[1],
        0.0,
    );
    let next_forward = CVector::new(
        next_direction * next_link.dir[0],
        next_direction * next_link.dir[1],
        0.0,
    );
    let current_lane = auto_pilot.current_lane as f32 + one_way_lane_offset(&current_link);
    let next_lane = auto_pilot.next_lane as f32 + one_way_lane_offset(&next_link);
    let current_pos = CVector::new(
        current_link.pos[0] + current_lane * LANE_WIDTH * current_forward.y,
        current_link.pos[1] - current_lane * LANE_WIDTH * current_forward.x,
        0.0,
    );
    let next_pos = CVector::new(
        next_link.pos[0] + next_lane * LANE_WIDTH * next_forward.y,
        next_link.pos[1] - next_lane * LANE_WIDTH * next_forward.x,
        0.0,
    );
    let between = get_position_along_current_curve(auto_pilot, now_ms).clamp(0.0, 1.0);
    Some(CCurves::CalcCurvePoint(
        current_pos,
        next_pos,
        current_forward,
        next_forward,
        between,
        auto_pilot.time_to_spend_on_current_curve,
    ))
}

// === FAZ3b: moved from core/Game.rs (verbatim, parity-preserving) ===
// On-rails traffic math: scans, curve following, traffic/light slow-down,
// drag-to-point. Consumed by control/CarCtrl.rs simple control.

#[allow(unused_imports)]
use crate::control::AutoPilot::{eCarDrivingStyle, eCarTempAction};
#[allow(unused_imports)]
use crate::control::TrafficLights::CTrafficLights;
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::World::{CWorld, NUMSECTORS_X, NUMSECTORS_Y};
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::vehicles::Automobile::CAutomobile;
#[allow(unused_imports)]
use bevy::prelude::*;
#[allow(unused_imports)]
use rustgta3_core::core::Timer::CTimer;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

/// Snapshot used by the rails traffic scan. re3 walks CWorld sector lists
/// (`ENTITYLIST_VEHICLES[/_OVERLAP]` etc.) inside
/// `FindMaximumSpeedForThisCarInTraffic`; the ECS equivalent captures the same
/// observable fields once per frame and scans them with an explicit distance
/// filter (documented broad-phase adaptation — same result set, no sector
/// bookkeeping).
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrafficVehicleScan {
    pub(crate) slot: usize,
    pub(crate) position: CVector,
    pub(crate) forward: CVector,
    pub(crate) move_speed: CVector,
    pub(crate) bbox_min: CVector,
    pub(crate) bbox_max: CVector,
    pub(crate) is_player_vehicle: bool,
    pub(crate) time_to_start_mission: u32,
}

/// Ped snapshot for the vehicle-speed part of
/// `SlowCarDownForPedsSectorList`; the ped-side evasive/flee reactions of that
/// C++ function belong to the ped behavior layer and remain an open surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrafficPedScan {
    pub(crate) position: CVector,
}

/// Frame-local broadphase for GTA traffic queries. re3 never scans the whole
/// vehicle/ped pool for `FindMaximumSpeedForThisCarInTraffic`; it visits only
/// the CWorld sectors touched by the 22 m danger square. Keep the ECS snapshot
/// arrays, but index them by the same 40 m world sectors so dense traffic stays
/// proportional to nearby entities instead of becoming O(cars * entities).
#[derive(Default)]
pub(crate) struct TrafficSpatialIndex {
    vehicle_buckets: Vec<Vec<usize>>,
    ped_buckets: Vec<Vec<usize>>,
    touched_vehicle_buckets: Vec<usize>,
    touched_ped_buckets: Vec<usize>,
}

impl TrafficSpatialIndex {
    pub(crate) fn begin_frame(&mut self) {
        let sector_count = NUMSECTORS_X * NUMSECTORS_Y;
        if self.vehicle_buckets.len() != sector_count {
            self.vehicle_buckets = (0..sector_count).map(|_| Vec::new()).collect();
            self.touched_vehicle_buckets.clear();
        } else {
            for index in self.touched_vehicle_buckets.drain(..) {
                self.vehicle_buckets[index].clear();
            }
        }
        if self.ped_buckets.len() != sector_count {
            self.ped_buckets = (0..sector_count).map(|_| Vec::new()).collect();
            self.touched_ped_buckets.clear();
        } else {
            for index in self.touched_ped_buckets.drain(..) {
                self.ped_buckets[index].clear();
            }
        }
    }

    pub(crate) fn insert_vehicle(&mut self, scan_index: usize, position: CVector) {
        let index = Self::sector_index(position);
        let bucket = &mut self.vehicle_buckets[index];
        if bucket.is_empty() {
            self.touched_vehicle_buckets.push(index);
        }
        bucket.push(scan_index);
    }

    pub(crate) fn insert_ped(&mut self, scan_index: usize, position: CVector) {
        let index = Self::sector_index(position);
        let bucket = &mut self.ped_buckets[index];
        if bucket.is_empty() {
            self.touched_ped_buckets.push(index);
        }
        bucket.push(scan_index);
    }

    pub(crate) fn vehicle_bucket(&self, x: usize, y: usize) -> &[usize] {
        self.vehicle_buckets
            .get(y * NUMSECTORS_X + x)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn ped_bucket(&self, x: usize, y: usize) -> &[usize] {
        self.ped_buckets
            .get(y * NUMSECTORS_X + x)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn sector_index(position: CVector) -> usize {
        CWorld::get_sector_index_y(position.y) * NUMSECTORS_X
            + CWorld::get_sector_index_x(position.x)
    }
}

/// re3 `CAutoPilot::ModifySpeed` with the live link snapshots.
pub(crate) fn modify_speed_on_rails<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    path_find: &CPathFind,
    speed: f32,
    time_ms: u32,
)
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let snapshot = |index: usize| {
        path_find
            .car_path_links
            .get(index)
            .map(|link| crate::control::AutoPilot::CarPathLinkSnapshot {
                x: link.pos[0],
                y: link.pos[1],
                dir_x: link.dir[0],
                dir_y: link.dir[1],
            })
            .unwrap_or(crate::control::AutoPilot::CarPathLinkSnapshot {
                x: 0.0,
                y: 0.0,
                dir_x: 0.0,
                dir_y: 0.0,
            })
    };
    let current_link = snapshot(car.vehicle.core.auto_pilot.current_path_node_info as usize);
    let next_link = snapshot(car.vehicle.core.auto_pilot.next_path_node_info as usize);
    car.vehicle.core.auto_pilot
        .ModifySpeed(speed, time_ms, current_link, next_link);
}

/// re3 `CCarCtrl::UpdateCarOnRails` (`control/CarCtrl.cpp:806-854`) with the
/// `SlowCarOnRailsDownForTrafficAndLights` speed gate
/// (`CarCtrl.cpp:922-942`), `DragCarToPoint` ground snapping
/// (`CarCtrl.cpp:1984-2044`) and the `FindMaximumSpeedForThisCarInTraffic`
/// scan (`CarCtrl.cpp:856-890`).
///
/// TODO(parity): `CCollision::IsStoredPolyStillValidVerticalLine` and
/// `CCarCtrl::DragCarToPoint` cached-triangle acquisition/invalidation.
/// The current rails path queries original world COL each frame; the canonical
/// collision polygon has vertices, but this path does not yet populate/reuse it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rails_update_car_on_rails<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    file_loader: &mut CFileLoader,
    self_slot: usize,
    vehicle_scans: &[TrafficVehicleScan],
    ped_scans: &[TrafficPedScan],
    traffic_index: &TrafficSpatialIndex,
    player_position: CVector,
    player_vehicle_slot: Option<usize>,
    bbox_min: CVector,
    bbox_max: CVector,
    time_ms: u32,
    time_step: f32,
    general: &mut CGeneral,
)
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    crate::profile_scope!("rails_update_car_on_rails");
    // TEMPACT_WAIT branch (CarCtrl.cpp:808-817).
    if car.vehicle.core.auto_pilot.temp_action == eCarTempAction::TEMPACT_WAIT {
        car.vehicle.core.physical.move_speed = CVector::ZERO;
        modify_speed_on_rails(car, &file_loader.path_find, 0.0, time_ms);
        // CarCtrl stores the wake-up deadline separately from the enum state;
        // comparing against the enum discriminant released WAIT on the next
        // frame and made traffic oscillate at blocked crossings.
        if time_ms > car.vehicle.core.auto_pilot.time_temp_action {
            car.vehicle.core.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
            car.vehicle.core.auto_pilot.anti_reverse_timer = time_ms;
            car.vehicle.core.auto_pilot.time_to_start_mission = time_ms;
        }
        return;
    }

    slow_car_on_rails_down_for_traffic_and_lights(
        car,
        file_loader,
        self_slot,
        vehicle_scans,
        ped_scans,
        traffic_index,
        player_vehicle_slot,
        bbox_min,
        bbox_max,
        time_ms,
        time_step,
        general,
    );
    if car.vehicle.core.physical.entity.status != eEntityStatus::STATUS_SIMPLE {
        return;
    }

    let curve_finished = car
        .vehicle.core.auto_pilot
        .time_entered_curve
        .wrapping_add(car.vehicle.core.auto_pilot.time_to_spend_on_current_curve)
        <= time_ms as i32;
    if curve_finished {
        let position = car.vehicle.core.physical.entity.get_position();
        let target_car_position = car.vehicle.core.auto_pilot.target_car.and_then(|slot| {
            vehicle_scans
                .iter()
                .find(|scan| scan.slot == slot)
                .map(|scan| scan.position)
        });
        let result = crate::control::CarRails::pick_next_node_according_to_strategy(
            &mut file_loader.path_find,
            &mut car.vehicle.core.auto_pilot,
            position,
            player_position,
            target_car_position,
            general,
        );
        if result.switched_to_physics {
            // CarCtrl.cpp:1602-1606.
            car.vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
            car.vehicle.switch_to_real_physics(time_ms);
        }
    }
    // CarCtrl.cpp:821-822: a pick can hand the car to real physics.
    if car.vehicle.core.physical.entity.status != eEntityStatus::STATUS_SIMPLE {
        return;
    }

    let current_link_index = car.vehicle.core.auto_pilot.current_path_node_info as usize;
    let next_link_index = car.vehicle.core.auto_pilot.next_path_node_info as usize;
    let (Some(current_link), Some(next_link)) = (
        file_loader
            .path_find
            .car_path_links
            .get(current_link_index)
            .copied(),
        file_loader
            .path_find
            .car_path_links
            .get(next_link_index)
            .copied(),
    ) else {
        return;
    };
    let current_forward_x = car.vehicle.core.auto_pilot.current_direction as f32 * current_link.dir[0];
    let current_forward_y = car.vehicle.core.auto_pilot.current_direction as f32 * current_link.dir[1];
    let next_forward_x = car.vehicle.core.auto_pilot.next_direction as f32 * next_link.dir[0];
    let next_forward_y = car.vehicle.core.auto_pilot.next_direction as f32 * next_link.dir[1];
    let one_way_current = crate::control::CarRails::one_way_lane_offset(&current_link);
    let one_way_next = crate::control::CarRails::one_way_lane_offset(&next_link);
    let lane_width = crate::control::AutoPilot::LANE_WIDTH;
    let position_on_current_link_including_lane = CVector::new(
        current_link.pos[0]
            + (car.vehicle.core.auto_pilot.current_lane as f32 + one_way_current)
                * lane_width
                * current_forward_y,
        current_link.pos[1]
            - (car.vehicle.core.auto_pilot.current_lane as f32 + one_way_current)
                * lane_width
                * current_forward_x,
        0.0,
    );
    let position_on_next_link_including_lane = CVector::new(
        next_link.pos[0]
            + (car.vehicle.core.auto_pilot.next_lane as f32 + one_way_next)
                * lane_width
                * next_forward_y,
        next_link.pos[1]
            - (car.vehicle.core.auto_pilot.next_lane as f32 + one_way_next)
                * lane_width
                * next_forward_x,
        0.0,
    );
    let position_along_curve = crate::control::CarRails::get_position_along_current_curve(
        &car.vehicle.core.auto_pilot,
        time_ms,
    );
    let (mut position_including_curve, direction_including_curve) = CCurves::CalcCurvePoint(
        position_on_current_link_including_lane,
        position_on_next_link_including_lane,
        CVector::new(current_forward_x, current_forward_y, 0.0),
        CVector::new(next_forward_x, next_forward_y, 0.0),
        position_along_curve,
        car.vehicle.core.auto_pilot.time_to_spend_on_current_curve,
    );
    position_including_curve.z = 15.0;
    drag_car_to_point(car, file_loader, position_including_curve);
    // CarCtrl.cpp:853.
    car.vehicle.core.physical.move_speed = CVector::new(
        direction_including_curve.x / crate::control::CarAI::GAME_SPEED_TO_CARAI_SPEED,
        direction_including_curve.y / crate::control::CarAI::GAME_SPEED_TO_CARAI_SPEED,
        direction_including_curve.z / crate::control::CarAI::GAME_SPEED_TO_CARAI_SPEED,
    );
}

/// re3 `CCarCtrl::SlowCarOnRailsDownForTrafficAndLights`
/// (`control/CarCtrl.cpp:922-942`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn slow_car_on_rails_down_for_traffic_and_lights<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    file_loader: &mut CFileLoader,
    self_slot: usize,
    vehicle_scans: &[TrafficVehicleScan],
    ped_scans: &[TrafficPedScan],
    traffic_index: &TrafficSpatialIndex,
    player_vehicle_slot: Option<usize>,
    bbox_min: CVector,
    bbox_max: CVector,
    time_ms: u32,
    time_step: f32,
    general: &mut CGeneral,
)
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    crate::profile_scope!("slow_car_on_rails_down_for_traffic_and_lights");
    let position = car.vehicle.core.physical.entity.get_position();
    let vehicle_state = crate::control::TrafficLights::CTrafficVehicleState {
        position: [position.x, position.y, position.z],
        next_path_node_info: car.vehicle.core.auto_pilot.next_path_node_info as usize,
        current_path_node_info: car.vehicle.core.auto_pilot.current_path_node_info as usize,
        previous_path_node_info: car.vehicle.core.auto_pilot.previous_path_node_info as usize,
        next_route_node: car.vehicle.core.auto_pilot.next_route_node as i16,
        current_route_node: car.vehicle.core.auto_pilot.current_route_node as i16,
        prev_route_node: car.vehicle.core.auto_pilot.prev_route_node as i16,
        next_direction: car.vehicle.core.auto_pilot.next_direction,
        current_direction: car.vehicle.core.auto_pilot.current_direction,
        previous_direction: car.vehicle.core.auto_pilot.previous_direction,
        status_physics: false,
    };
    let stop_for_light =
        CTrafficLights::should_car_stop_for_light(
            &file_loader.path_find,
            &vehicle_state,
            false,
            time_ms,
        ) || CTrafficLights::should_car_stop_for_bridge(&file_loader.path_find, &vehicle_state);
    let max_speed = if stop_for_light {
        crate::control::CarAI::CCarAI::CarHasReasonToStop(&mut car.vehicle.core.auto_pilot, time_ms);
        0.0
    } else {
        find_maximum_speed_for_this_car_in_traffic(
            car,
            file_loader,
            self_slot,
            vehicle_scans,
            ped_scans,
            traffic_index,
            player_vehicle_slot,
            bbox_min,
            bbox_max,
            time_ms,
            general,
        )
    };
    let cur_speed = car.vehicle.core.auto_pilot.max_traffic_speed;
    if max_speed >= cur_speed {
        if max_speed > cur_speed {
            modify_speed_on_rails(
                car,
                &file_loader.path_find,
                max_speed.min(cur_speed + 0.05 * time_step),
                time_ms,
            );
        }
    } else if cur_speed != 0.0 {
        if cur_speed < 0.1 {
            modify_speed_on_rails(car, &file_loader.path_find, 0.0, time_ms);
        } else {
            modify_speed_on_rails(
                car,
                &file_loader.path_find,
                max_speed.max(cur_speed - 0.5 * time_step),
                time_ms,
            );
        }
    }
}

/// re3 `CCarCtrl::FindMaximumSpeedForThisCarInTraffic`
/// (`control/CarCtrl.cpp:856-890`) with `SlowCarDownForOtherCar`
/// (`CarCtrl.cpp:1075-1114`) and the vehicle-speed tail of
/// `SlowCarDownForPedsSectorList` (`CarCtrl.cpp:1040-1050`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn find_maximum_speed_for_this_car_in_traffic<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    _file_loader: &mut CFileLoader,
    self_slot: usize,
    vehicle_scans: &[TrafficVehicleScan],
    ped_scans: &[TrafficPedScan],
    traffic_index: &TrafficSpatialIndex,
    _player_vehicle_slot: Option<usize>,
    bbox_min: CVector,
    bbox_max: CVector,
    time_ms: u32,
    general: &mut CGeneral,
) -> f32
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    if car.vehicle.core.auto_pilot.driving_style == eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS
        || car.vehicle.core.auto_pilot.driving_style == eCarDrivingStyle::DRIVINGSTYLE_PLOUGH_THROUGH
    {
        return f32::from(car.vehicle.core.auto_pilot.cruise_speed);
    }
    let position = car.vehicle.core.physical.entity.get_position();
    let left = position.x - crate::control::CarRails::DISTANCE_TO_SCAN_FOR_DANGER;
    let right = position.x + crate::control::CarRails::DISTANCE_TO_SCAN_FOR_DANGER;
    let top = position.y - crate::control::CarRails::DISTANCE_TO_SCAN_FOR_DANGER;
    let bottom = position.y + crate::control::CarRails::DISTANCE_TO_SCAN_FOR_DANGER;
    let cur_speed = car.vehicle.core.auto_pilot.max_traffic_speed;
    let mut max_speed = f32::from(car.vehicle.core.auto_pilot.cruise_speed);
    let forward = car.vehicle.core.physical.entity.get_forward();
    let right_axis = car.vehicle.core.physical.entity.get_right();
    let move_speed = car.vehicle.core.physical.move_speed;

    let xstart = CWorld::get_sector_index_x(left);
    let xend = CWorld::get_sector_index_x(right);
    let ystart = CWorld::get_sector_index_y(top);
    let yend = CWorld::get_sector_index_y(bottom);
    for y in ystart..=yend.min(NUMSECTORS_Y - 1) {
        for x in xstart..=xend.min(NUMSECTORS_X - 1) {
            for &scan_index in traffic_index.vehicle_bucket(x, y) {
                let Some(other) = vehicle_scans.get(scan_index) else {
                    continue;
                };
                // SlowCarDownForCarsSectorList bounds (`CarCtrl.cpp:1060-1071`).
                if other.slot == self_slot {
                    continue;
                }
                if other.position.x < left || other.position.x > right {
                    continue;
                }
                if other.position.y < top || other.position.y > bottom {
                    continue;
                }
                if (other.position.z - position.z).abs() >= 5.0 {
                    continue;
                }
                let annoyance_candidate = crate::control::CarRails::slow_car_down_for_other_car(
                    other.position,
                    other.forward,
                    other.move_speed,
                    other.bbox_min,
                    other.bbox_max,
                    position,
                    forward,
                    bbox_min,
                    bbox_max,
                    &mut car.vehicle.core.auto_pilot,
                    &mut max_speed,
                    cur_speed,
                );
                // Standing-car annoyance branch (`CarCtrl.cpp:1098-1113`). The C++
                // pointer comparison `pVehicle < pOtherVehicle` becomes a pool-index
                // ordering comparison in ECS (deterministic equivalent).
                if annoyance_candidate
                    && !other.is_player_vehicle
                    && time_ms - other.time_to_start_mission > 15000
                    && time_ms - car.vehicle.core.auto_pilot.time_to_start_mission > 15000
                    && forward.x * other.forward.x + forward.y * other.forward.y < -0.5
                    && self_slot < other.slot
                {
                    max_speed = (cur_speed / 5.0).max(max_speed);
                    if car.vehicle.core.physical.entity.status == eEntityStatus::STATUS_SIMPLE {
                        car.vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
                        car.vehicle.switch_to_real_physics(time_ms);
                    }
                    car.vehicle.core.auto_pilot.driving_style =
                        eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
                    car.vehicle.core.auto_pilot.time_temp_action = time_ms + 1000;
                    return max_speed;
                }
            }
        }
    }
    car.vehicle.core.flags.warned_peds = true;
    for y in ystart..=yend.min(NUMSECTORS_Y - 1) {
        for x in xstart..=xend.min(NUMSECTORS_X - 1) {
            for &scan_index in traffic_index.ped_bucket(x, y) {
                let Some(ped) = ped_scans.get(scan_index) else {
                    continue;
                };
                if ped.position.x < left || ped.position.x > right {
                    continue;
                }
                if ped.position.y < top || ped.position.y > bottom {
                    continue;
                }
                if (ped.position.z - position.z).abs() >= 4.0 {
                    continue;
                }
                crate::control::CarRails::slow_car_down_for_ped(
                    ped.position,
                    position,
                    forward,
                    right_axis,
                    move_speed,
                    bbox_min,
                    bbox_max,
                    false,
                    &mut car.vehicle.core.auto_pilot,
                    &mut max_speed,
                    cur_speed,
                    time_ms,
                );
            }
        }
    }
    let _ = general;
    if car.vehicle.core.auto_pilot.driving_style == eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS {
        return max_speed;
    }
    (max_speed + f32::from(car.vehicle.core.auto_pilot.cruise_speed)) / 2.0
}

/// re3 `CCarCtrl::DragCarToPoint` (`control/CarCtrl.cpp:1984-2044`).
pub(crate) fn drag_car_to_point<AutoCore, AutoVitality, AutoState, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    file_loader: &mut CFileLoader,
    target_point: CVector,
)
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let position = car.vehicle.core.physical.entity.get_position();
    let forward = car.vehicle.core.physical.entity.get_forward();
    // posBehind = position - 3*forward/2 (CarCtrl.cpp:1986).
    let pos_behind = CVector::new(
        position.x - 3.0 * forward.x / 2.0,
        position.y - 3.0 * forward.y / 2.0,
        0.0,
    );
    let direction = CVector::new(
        pos_behind.x - target_point.x,
        pos_behind.y - target_point.y,
        0.0,
    );
    let magnitude = (direction.x * direction.x + direction.y * direction.y).sqrt();
    let mid_pos = if magnitude > f32::EPSILON {
        CVector::new(
            target_point.x + direction.x * 3.0 / magnitude,
            target_point.y + direction.y * 3.0 / magnitude,
            0.0,
        )
    } else {
        target_point
    };

    let ground_at =
        |x: f32, y: f32, z: f32, file_loader: &CFileLoader| -> Option<(f32, usize)> {
        // CarCtrl.cpp:1997-1999 first query, 2004-2006 fallback.
        let hit = file_loader
            .world
            .process_vertical_line_buildings_only_with_entity(
                [x, y, z + 1.5],
                z - 2.0,
                &file_loader.model_info,
            )
            .or_else(|| {
                file_loader
                    .world
                    .process_vertical_line_buildings_only_with_entity(
                        [x, y, z + 3.0],
                        z - 3.0,
                        &file_loader.model_info,
                    )
            })?;
        Some((hit.0.point.z, hit.1))
    };
    let ahead_ground = ground_at(target_point.x, target_point.y, position.z, file_loader);
    let actual_ahead_z = ahead_ground
        .map(|(height, _)| height)
        .unwrap_or(car.vehicle.core.map_object_height_ahead);
    if let Some((_, entity_index)) = ahead_ground {
        // re3 `DragCarToPoint` refreshes `m_pCurGroundEntity` whenever either
        // vertical line resolves a road object. Cull-zone visibility and the
        // subsequent physical handoff both consume that identity.
        car.vehicle.core.current_ground_entity = Some(entity_index);
    }
    car.vehicle.core.map_object_height_ahead = actual_ahead_z;
    let behind_ground = ground_at(mid_pos.x, mid_pos.y, position.z, file_loader);
    let actual_behind_z = behind_ground
        .map(|(height, _)| height)
        .unwrap_or(car.vehicle.core.map_object_height_behind);
    if let Some((_, entity_index)) = behind_ground {
        car.vehicle.core.current_ground_entity = Some(entity_index);
    }
    car.vehicle.core.map_object_height_behind = actual_behind_z;

    // CarCtrl.cpp:2036-2043.
    let angle_z = ((actual_ahead_z - actual_behind_z) / 3.0).atan();
    let cos_z = angle_z.cos();
    let sin_z = angle_z.sin();
    let right = CVector::new(
        (target_point.y - mid_pos.y) / 3.0,
        -(target_point.x - mid_pos.x) / 3.0,
        0.0,
    );
    let forward_new = CVector::new(-cos_z * right.y, cos_z * right.x, sin_z);
    let up = CVector::new(
        right.y * forward_new.z - right.z * forward_new.y,
        right.z * forward_new.x - right.x * forward_new.z,
        right.x * forward_new.y - right.y * forward_new.x,
    );
    let mut matrix = car.vehicle.core.physical.entity.matrix;
    matrix.right = right;
    matrix.forward = forward_new;
    matrix.up = up;
    matrix.position = CVector::new(
        (mid_pos.x + target_point.x) / 2.0,
        (mid_pos.y + target_point.y) / 2.0,
        (actual_behind_z + actual_ahead_z) / 2.0,
    );
    matrix.position.z += car.height_above_road();
    car.vehicle.core.physical.entity.set_matrix(matrix);
}

// === FAZ8c-carrails: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn join_car_with_road_system_goto_coors<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    paths: &mut CPathFind,
    vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    target: CVector,
    general: &mut CGeneral,
) -> bool
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let core = &mut *vehicle.core;
    let start = core.physical.entity.get_position();
    let start_map_object = core.physical.treadable[crate::control::PathFind::PATH_CAR as usize];
    let auto_pilot = &mut core.auto_pilot;
    auto_pilot.destination_coors = target;
    let mut nodes = paths
        .do_path_search_with_start_object(
            crate::control::PathFind::PATH_CAR,
            start,
            None,
            start_map_object,
            target,
            crate::control::AutoPilot::NUM_PATH_NODES_IN_AUTOPILOT,
            999_999.9,
            None,
        )
        .nodes;

    // CPathFind::RemoveBadStartNode.
    if nodes.len() >= 2 {
        let first = paths.path_nodes[nodes[0]].pos;
        let second = paths.path_nodes[nodes[1]].pos;
        let first_delta = CVector::new(first.x - start.x, first.y - start.y, 0.0);
        let second_delta = CVector::new(second.x - start.x, second.y - start.y, 0.0);
        if first_delta.x * second_delta.x + first_delta.y * second_delta.y < 0.0 {
            nodes.remove(0);
        }
    }

    auto_pilot.path_find_nodes = [None; crate::control::AutoPilot::NUM_PATH_NODES_IN_AUTOPILOT];
    for (slot, node) in auto_pilot
        .path_find_nodes
        .iter_mut()
        .zip(nodes.iter().copied())
    {
        *slot = Some(node);
    }
    auto_pilot.path_find_nodes_count = nodes.len() as i16;

    if nodes.len() < 2 {
        auto_pilot.prev_route_node = 0;
        auto_pilot.current_route_node = 0;
        auto_pilot.next_route_node = 0;
        return true;
    }

    auto_pilot.prev_route_node = 0;
    auto_pilot.current_route_node = nodes[0] as i32;
    auto_pilot.next_route_node = nodes[1] as i32;
    let remaining = &nodes[2..];
    auto_pilot.path_find_nodes = [None; crate::control::AutoPilot::NUM_PATH_NODES_IN_AUTOPILOT];
    for (slot, node) in auto_pilot
        .path_find_nodes
        .iter_mut()
        .zip(remaining.iter().copied())
    {
        *slot = Some(node);
    }
    auto_pilot.path_find_nodes_count = remaining.len() as i16;

    find_links_to_go_with_these_nodes(paths, auto_pilot, general);
    auto_pilot.current_lane = 0;
    auto_pilot.next_lane = 0;
    false
}

pub(crate) fn join_car_with_road_system<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    paths: &CPathFind,
    vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    general: &mut CGeneral,
)
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let core = &mut *vehicle.core;
    let auto_pilot = &mut core.auto_pilot;
    auto_pilot.prev_route_node = 0;
    auto_pilot.current_route_node = 0;
    auto_pilot.next_route_node = 0;
    auto_pilot.current_path_node_info = 0;
    auto_pilot.previous_path_node_info = 0;
    auto_pilot.next_path_node_info = 0;

    let position = core.physical.entity.get_position();
    let mut forward = core.physical.entity.get_forward();
    let node_id = paths.find_node_closest_to_coors_favour_direction(
        position,
        crate::control::PathFind::PATH_CAR,
        forward.x,
        forward.y,
    );
    let Ok(mut node_index) = usize::try_from(node_id) else {
        return;
    };
    let Some(node) = paths.path_nodes.get(node_index).copied() else {
        return;
    };
    let mut previous_index = None;
    let mut minimum_distance = 999_999.9_f32;
    for offset in 0..node.num_links as usize {
        let Some(candidate_index) = paths.connected_node(node.first_link.max(0) as usize + offset)
        else {
            continue;
        };
        let Some(candidate) = paths.path_nodes.get(candidate_index) else {
            continue;
        };
        let dx = candidate.pos.x - node.pos.x;
        let dy = candidate.pos.y - node.pos.y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance < minimum_distance {
            minimum_distance = distance;
            previous_index = Some(candidate_index);
        }
    }
    let Some(mut previous_index) = previous_index else {
        return;
    };
    if forward.x == 0.0 && forward.y == 0.0 {
        forward.x = 1.0;
    }
    let previous = paths.path_nodes[previous_index].pos;
    let dx = node.pos.x - previous.x;
    let dy = node.pos.y - previous.y;
    if dx * forward.x + dy * forward.y < 0.0 {
        std::mem::swap(&mut previous_index, &mut node_index);
    }

    auto_pilot.prev_route_node = 0;
    auto_pilot.current_route_node = previous_index as i32;
    auto_pilot.next_route_node = node_index as i32;
    auto_pilot.path_find_nodes = [None; crate::control::AutoPilot::NUM_PATH_NODES_IN_AUTOPILOT];
    auto_pilot.path_find_nodes_count = 0;
    find_links_to_go_with_these_nodes(paths, auto_pilot, general);
    auto_pilot.current_lane = 0;
    auto_pilot.next_lane = 0;
}

pub(crate) fn find_links_to_go_with_these_nodes(
    paths: &CPathFind,
    auto_pilot: &mut CAutoPilot,
    general: &mut CGeneral,
) {
    let current = auto_pilot.current_route_node.max(0) as usize;
    let next = auto_pilot.next_route_node.max(0) as usize;
    let Some(current_node) = paths.path_nodes.get(current).copied() else {
        return;
    };
    let first_link = current_node.first_link.max(0) as usize;
    let link_count = current_node.num_links as usize;
    let Some(next_offset) =
        (0..link_count).find(|offset| paths.connected_node(first_link + offset) == Some(next))
    else {
        return;
    };
    let next_connection = paths
        .car_path_connections
        .get(first_link + next_offset)
        .copied()
        .unwrap_or_default();
    auto_pilot.next_path_node_info = next_connection.max(0) as u32;
    auto_pilot.next_direction = if current >= next { 1 } else { -1 };

    let current_offset = if link_count <= 1 {
        0
    } else {
        loop {
            let candidate = general.get_random_number() as usize % link_count;
            let connection = paths
                .car_path_connections
                .get(first_link + candidate)
                .copied()
                .unwrap_or_default();
            if connection != next_connection {
                break candidate;
            }
        }
    };
    let current_connection = paths
        .car_path_connections
        .get(first_link + current_offset)
        .copied()
        .unwrap_or_default();
    auto_pilot.current_path_node_info = current_connection.max(0) as u32;
    auto_pilot.current_direction = if paths
        .connected_node(first_link + current_offset)
        .unwrap_or(current)
        >= current
    {
        1
    } else {
        -1
    };
}
