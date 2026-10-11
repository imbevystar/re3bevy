//! re3 reference: references/re-GTA/src/control/PathFind.cpp
//! re3 reference: references/re-GTA/src/control/PathFind.h
//!
//! Path object metadata used before full path graph preparation.

use std::{cmp::Reverse, collections::BinaryHeap};

use bevy::prelude::*;
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};
use re3bevy_formats::formats::FileLoader::{GtaIdePlan, GtaIdeRecord, GtaPathNode};

pub const NODE_TYPE_EXTERN: i8 = 1;
pub const NODE_TYPE_INTERN: i8 = 2;
pub const USE_IN_ROADBLOCK: u8 = 1;
pub const OBJECT_EAST_WEST: u8 = 2;
pub const PATH_CAR: u8 = 0;
pub const PATH_PED: u8 = 1;
pub const NUM_PATHNODES: usize = 4930;
pub const NUM_CARPATHLINKS: usize = 2076;
pub const NUM_MAPOBJECTS: usize = 1250;
pub const NUM_PATHCONNECTIONS: usize = 10260;
pub const PATHNODESIZE: usize = 4500;
pub const PATH_INFO_SLOTS_PER_OBJECT: usize = 12;
pub const NUMDETACHED_CARS: usize = 100;
pub const NUMDETACHED_PEDS: usize = 50;
pub const MAX_DIST: i16 = i16::MAX;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPathMapObject {
    pub model_index: usize,
    pub position: CVector,
    pub rotation_axis: [f32; 3],
    pub rotation_w: f32,
    pub node_indices: [[i16; PATH_INFO_SLOTS_PER_OBJECT]; 2],
}

impl CPathMapObject {
    pub fn new(
        model_index: usize,
        position: CVector,
        rotation_axis: [f32; 3],
        rotation_w: f32,
    ) -> Self {
        Self {
            model_index,
            position,
            rotation_axis,
            rotation_w,
            node_indices: [[-1; PATH_INFO_SLOTS_PER_OBJECT]; 2],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CPathInfoForObject {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub node_type: i8,
    pub next: i8,
    pub num_left_lanes: i8,
    pub num_right_lanes: i8,
    pub crossing: bool,
}

impl Default for CPathInfoForObject {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            z: 0,
            node_type: 0,
            next: 0,
            num_left_lanes: 0,
            num_right_lanes: 0,
            crossing: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPathNode {
    pub pos: CVector,
    pub distance: i16,
    pub object_index: i16,
    pub first_link: i16,
    pub num_links: u8,
    pub unk_bits: u8,
    pub dead_end: bool,
    pub disabled: bool,
    pub between_levels: bool,
    pub group: i8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CCarPathLink {
    pub pos: [f32; 2],
    pub dir: [f32; 2],
    pub path_node_index: i16,
    pub num_left_lanes: i8,
    pub num_right_lanes: i8,
    pub traffic_light_type: u8,
    pub bridge_lights: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CConnectionFlags {
    pub crosses_road: bool,
    pub traffic_light: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CPathSearchResult {
    pub nodes: Vec<usize>,
    pub num_nodes: i16,
    pub distance: f32,
}

impl CPathSearchResult {
    fn no_path() -> Self {
        Self {
            nodes: Vec::new(),
            num_nodes: 0,
            distance: 100000.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CCarCreationCoors {
    pub position: CVector,
    pub node1: i32,
    pub node2: i32,
    pub position_between_nodes: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPedCreationCoors {
    pub position: CVector,
    pub node1: i32,
    pub node2: i32,
    pub position_between_nodes: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CTempNode {
    pos: CVector,
    dir_x: f32,
    dir_y: f32,
    link1: i16,
    link2: i16,
    num_left_lanes: i8,
    num_right_lanes: i8,
    link_state: i8,
}

impl Default for CTempNode {
    fn default() -> Self {
        Self {
            pos: CVector::ZERO,
            dir_x: 0.0,
            dir_y: 0.0,
            link1: -1,
            link2: -1,
            num_left_lanes: 0,
            num_right_lanes: 0,
            link_state: 0,
        }
    }
}

impl Default for CPathNode {
    fn default() -> Self {
        Self {
            pos: CVector::ZERO,
            distance: MAX_DIST,
            object_index: 0,
            first_link: 0,
            num_links: 0,
            unk_bits: 0,
            dead_end: false,
            disabled: false,
            between_levels: false,
            group: 0,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CPathFind {
    pub path_nodes: Vec<CPathNode>,
    pub car_path_links: Vec<CCarPathLink>,
    pub map_objects: Vec<Option<CPathMapObject>>,
    pub object_flags: Vec<u8>,
    pub connections: Vec<i16>,
    pub distances: Vec<i16>,
    pub connection_flags: Vec<CConnectionFlags>,
    pub car_path_connections: Vec<i16>,
    pub num_path_nodes: i32,
    pub num_car_path_nodes: i32,
    pub num_ped_path_nodes: i32,
    pub num_map_objects: i16,
    pub num_connections: i16,
    pub num_car_path_links: i32,
    pub unk: i32,
    pub num_groups: [u8; 2],
    pub info_for_tile_cars: Vec<CPathInfoForObject>,
    pub info_for_tile_peds: Vec<CPathInfoForObject>,
    pub detached_nodes_cars_capacity: usize,
    pub detached_nodes_peds_capacity: usize,
    pub allocated_path_groups: i16,
    pub prepared: bool,
}

impl Default for CPathFind {
    fn default() -> Self {
        Self::init()
    }
}

impl CPathFind {
    pub fn init() -> Self {
        Self {
            path_nodes: vec![CPathNode::default(); NUM_PATHNODES],
            car_path_links: Vec::with_capacity(NUM_CARPATHLINKS),
            map_objects: vec![None; NUM_MAPOBJECTS],
            object_flags: vec![0; NUM_MAPOBJECTS],
            connections: Vec::with_capacity(NUM_PATHCONNECTIONS),
            distances: Vec::with_capacity(NUM_PATHCONNECTIONS),
            connection_flags: Vec::with_capacity(NUM_PATHCONNECTIONS),
            car_path_connections: Vec::with_capacity(NUM_PATHCONNECTIONS),
            num_path_nodes: 0,
            num_car_path_nodes: 0,
            num_ped_path_nodes: 0,
            num_map_objects: 0,
            num_connections: 0,
            num_car_path_links: 0,
            unk: 0,
            num_groups: [0; 2],
            info_for_tile_cars: Vec::new(),
            info_for_tile_peds: Vec::new(),
            detached_nodes_cars_capacity: 0,
            detached_nodes_peds_capacity: 0,
            allocated_path_groups: 0,
            prepared: false,
        }
    }

    pub fn allocate_path_find_info_mem(&mut self, num_path_groups: i16) {
        let count = PATH_INFO_SLOTS_PER_OBJECT * num_path_groups.max(0) as usize;
        self.info_for_tile_cars = vec![CPathInfoForObject::default(); count];
        self.info_for_tile_peds = vec![CPathInfoForObject::default(); count];
        self.detached_nodes_cars_capacity = NUMDETACHED_CARS;
        self.detached_nodes_peds_capacity = NUMDETACHED_PEDS;
        self.allocated_path_groups = num_path_groups;
        self.prepared = false;
    }

    pub fn register_map_object(
        &mut self,
        model_index: usize,
        position: CVector,
        rotation_axis: [f32; 3],
        rotation_w: f32,
    ) -> Option<usize> {
        let index = self.num_map_objects as usize;
        if index >= NUM_MAPOBJECTS {
            return None;
        }
        self.map_objects[index] = Some(CPathMapObject::new(
            model_index,
            position,
            rotation_axis,
            rotation_w,
        ));
        self.num_map_objects += 1;
        Some(index)
    }

    pub fn store_node_info_ped(&mut self, id: usize, node: usize, path_node: &GtaPathNode) {
        let index = path_info_index(id, node);
        if let Some(slot) = self.info_for_tile_peds.get_mut(index) {
            *slot = CPathInfoForObject::from_ped(path_node);
        }
    }

    pub fn store_node_info_car(&mut self, id: usize, node: usize, path_node: &GtaPathNode) {
        let index = path_info_index(id, node);
        if let Some(slot) = self.info_for_tile_cars.get_mut(index) {
            *slot = CPathInfoForObject::from_car(path_node);
        }
    }

    pub fn apply_ide_plan(&mut self, plan: &GtaIdePlan) {
        let mut current_header = None;
        let mut path_node_index = 0usize;

        for record in &plan.records {
            match record {
                GtaIdeRecord::PathHeader(header) => {
                    current_header = Some((header.id, header.path_type.to_ascii_lowercase()));
                    path_node_index = 0;
                }
                GtaIdeRecord::PathNode(node) => {
                    if let Some((id, path_type)) = &current_header {
                        if path_type == "ped" {
                            self.store_node_info_ped(*id, path_node_index, node);
                        } else {
                            self.store_node_info_car(*id, path_node_index, node);
                        }
                        path_node_index += 1;
                        if path_node_index == PATH_INFO_SLOTS_PER_OBJECT {
                            current_header = None;
                            path_node_index = 0;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn is_path_object(&self, id: usize) -> bool {
        if id >= PATHNODESIZE {
            return false;
        }
        let index = id * PATH_INFO_SLOTS_PER_OBJECT;
        self.info_for_tile_cars
            .get(index)
            .is_some_and(|slot| slot.node_type != 0)
            || self
                .info_for_tile_peds
                .get(index)
                .is_some_and(|slot| slot.node_type != 0)
    }

    pub fn derive_roadblock_flags_from_path_info(&mut self) {
        for map_object_index in 0..self.num_map_objects as usize {
            let Some(map_object) = self.map_objects[map_object_index] else {
                continue;
            };
            let mut num_extern = 0;
            let mut num_intern = 0;
            let mut num_lanes = 0;
            let mut max_x = 0i32;
            let mut max_y = 0i32;

            for node in 0..PATH_INFO_SLOTS_PER_OBJECT {
                let Some(info) = self
                    .info_for_tile_cars
                    .get(path_info_index(map_object.model_index, node))
                else {
                    continue;
                };
                if info.node_type == NODE_TYPE_EXTERN {
                    num_extern += 1;
                    num_lanes =
                        num_lanes.max(info.num_left_lanes as i32 + info.num_right_lanes as i32);
                    max_x = max_x.max(i32::from(info.x).abs());
                    max_y = max_y.max(i32::from(info.y).abs());
                } else if info.node_type == NODE_TYPE_INTERN {
                    num_intern += 1;
                }
            }

            if num_intern == 1 && num_extern == 2 {
                let use_for_roadblock = num_lanes >= 4 || (map_object_index & 7) == 4;
                if use_for_roadblock {
                    self.object_flags[map_object_index] |= USE_IN_ROADBLOCK;
                    if max_x > max_y {
                        self.object_flags[map_object_index] |= OBJECT_EAST_WEST;
                    } else {
                        self.object_flags[map_object_index] &= !OBJECT_EAST_WEST;
                    }
                }
            }
        }
    }

    pub fn prepare_path_data(&mut self) {
        if self.info_for_tile_cars.is_empty() || self.info_for_tile_peds.is_empty() {
            bevy::log::warn!(
                target: "re3bevy::pathfind",
                "CPathFind::PreparePathData skipped because car or ped path tile data is empty"
            );
            return;
        }

        self.num_path_nodes = 0;
        self.num_connections = 0;
        self.num_car_path_links = 0;
        self.connections.clear();
        self.distances.clear();
        self.connection_flags.clear();
        self.car_path_connections.clear();
        self.car_path_links.clear();

        for node in &mut self.path_nodes {
            node.unk_bits = 0;
            node.dead_end = false;
            node.disabled = false;
            node.between_levels = false;
            node.num_links = 0;
            node.first_link = 0;
            node.distance = MAX_DIST;
        }

        self.prepare_path_data_for_type(PATH_CAR);
        self.num_car_path_nodes = self.num_path_nodes;
        self.prepare_path_data_for_type(PATH_PED);
        self.num_ped_path_nodes = self.num_path_nodes - self.num_car_path_nodes;
        self.derive_roadblock_flags_from_path_info();
        self.count_flood_fill_groups(PATH_CAR);
        self.count_flood_fill_groups(PATH_PED);
        self.prepared = true;
    }

    /// Frame-yielding form of `PreparePathData` used by the real loading
    /// screen. re3 explicitly redraws its loading screen between the expensive
    /// setup stages; keeping those stages in one Bevy `OnEnter` frame caused a
    /// visible >1s application freeze.
    pub fn prepare_path_data_step(&mut self, phase: &mut PathDataPreparePhase) -> bool {
        if self.info_for_tile_cars.is_empty() || self.info_for_tile_peds.is_empty() {
            bevy::log::warn!(
                target: "re3bevy::pathfind",
                "CPathFind::PreparePathData skipped because car or ped path tile data is empty"
            );
            self.prepared = true;
            *phase = PathDataPreparePhase::Done;
            return true;
        }

        match *phase {
            PathDataPreparePhase::Reset => {
                self.num_path_nodes = 0;
                self.num_connections = 0;
                self.num_car_path_links = 0;
                self.connections.clear();
                self.distances.clear();
                self.connection_flags.clear();
                self.car_path_connections.clear();
                self.car_path_links.clear();
                for node in &mut self.path_nodes {
                    node.unk_bits = 0;
                    node.dead_end = false;
                    node.disabled = false;
                    node.between_levels = false;
                    node.num_links = 0;
                    node.first_link = 0;
                    node.distance = MAX_DIST;
                }
                *phase = PathDataPreparePhase::Cars;
            }
            PathDataPreparePhase::Cars => {
                self.prepare_path_data_for_type(PATH_CAR);
                self.num_car_path_nodes = self.num_path_nodes;
                *phase = PathDataPreparePhase::Peds;
            }
            PathDataPreparePhase::Peds => {
                self.prepare_path_data_for_type(PATH_PED);
                self.num_ped_path_nodes = self.num_path_nodes - self.num_car_path_nodes;
                *phase = PathDataPreparePhase::Roadblocks;
            }
            PathDataPreparePhase::Roadblocks => {
                self.derive_roadblock_flags_from_path_info();
                *phase = PathDataPreparePhase::CarFloodFill;
            }
            PathDataPreparePhase::CarFloodFill => {
                self.count_flood_fill_groups(PATH_CAR);
                *phase = PathDataPreparePhase::PedFloodFill;
            }
            PathDataPreparePhase::PedFloodFill => {
                self.count_flood_fill_groups(PATH_PED);
                self.prepared = true;
                *phase = PathDataPreparePhase::Done;
            }
            PathDataPreparePhase::Done => return true,
        }
        *phase == PathDataPreparePhase::Done
    }

    fn prepare_path_data_for_type(&mut self, path_type: u8) {
        let old_num_path_nodes = self.num_path_nodes as usize;
        let old_num_links = self.connections.len();

        for map_object in self
            .map_objects
            .iter_mut()
            .take(self.num_map_objects.max(0) as usize)
            .flatten()
        {
            map_object.node_indices[path_type as usize] = [-1; PATH_INFO_SLOTS_PER_OBJECT];
        }

        for map_object_index in 0..self.num_map_objects.max(0) as usize {
            let Some(map_object) = self.map_objects[map_object_index] else {
                continue;
            };
            let start = map_object.model_index * PATH_INFO_SLOTS_PER_OBJECT;
            for node_index in 0..PATH_INFO_SLOTS_PER_OBJECT {
                let Some(info) = self.path_info(path_type, start + node_index) else {
                    continue;
                };
                if info.node_type == NODE_TYPE_INTERN {
                    let path_node_index = self.num_path_nodes as usize;
                    if path_node_index >= self.path_nodes.len() {
                        return;
                    }
                    self.path_nodes[path_node_index].pos = self.calc_node_coors(map_object, info);
                    self.path_nodes[path_node_index].object_index = map_object_index as i16;
                    self.path_nodes[path_node_index].unk_bits = 1;
                    if let Some(map_object) = self.map_objects[map_object_index].as_mut() {
                        map_object.node_indices[path_type as usize][node_index] =
                            path_node_index as i16;
                    }
                    self.num_path_nodes += 1;
                }
            }
        }

        let mut temp_nodes: Vec<CTempNode> = Vec::new();
        for map_object_index in 0..self.num_map_objects.max(0) as usize {
            let Some(map_object) = self.map_objects[map_object_index] else {
                continue;
            };
            let start = map_object.model_index * PATH_INFO_SLOTS_PER_OBJECT;
            for node_index in 0..PATH_INFO_SLOTS_PER_OBJECT {
                let Some(info) = self.path_info(path_type, start + node_index) else {
                    continue;
                };
                if info.node_type != NODE_TYPE_EXTERN {
                    continue;
                }

                let coors = self.calc_node_coors(map_object, info);
                let nearest_id = temp_nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, temp)| temp.link_state == 1)
                    .filter_map(|(index, temp)| {
                        let dx = (temp.pos.x - coors.x).abs();
                        let dy = (temp.pos.y - coors.y).abs();
                        (dx < 1.0 && dy < 1.0).then_some((index, dx.max(dy)))
                    })
                    .min_by(|(_, a), (_, b)| a.total_cmp(b))
                    .map(|(index, _)| index);

                let next = self.resolve_next_path_node_index(path_type, start, node_index, info);
                let linked_internal = self.map_objects[map_object_index]
                    .as_ref()
                    .map(|object| object.node_indices[path_type as usize][next])
                    .unwrap_or(-1);

                if let Some(nearest_id) = nearest_id {
                    let temp = &mut temp_nodes[nearest_id];
                    temp.link2 = linked_internal;
                    temp.link_state = 2;
                    temp.pos = midpoint(temp.pos, coors);
                    if let (Some(link1), Some(link2)) = (
                        self.path_nodes.get(temp.link1.max(0) as usize),
                        self.path_nodes.get(temp.link2.max(0) as usize),
                    ) {
                        let dx = link1.pos.x - link2.pos.x;
                        let dy = link1.pos.y - link2.pos.y;
                        let mag = (dx * dx + dy * dy).sqrt();
                        if mag != 0.0 {
                            temp.dir_x = dx / mag;
                            temp.dir_y = dy / mag;
                        }
                    }
                    if path_type == PATH_CAR
                        && temp.num_left_lanes != 0
                        && temp.num_right_lanes != 0
                        && (info.num_left_lanes == 0 || info.num_right_lanes == 0)
                    {
                        temp.num_left_lanes = info.num_right_lanes;
                        temp.num_right_lanes = info.num_left_lanes;
                    }
                } else {
                    let mut temp = CTempNode {
                        pos: coors,
                        link1: linked_internal,
                        link_state: 1,
                        ..Default::default()
                    };
                    if path_type == PATH_CAR {
                        temp.num_left_lanes = info.num_left_lanes;
                        temp.num_right_lanes = info.num_right_lanes;
                    }
                    temp_nodes.push(temp);
                }
            }
        }

        for path_node_index in old_num_path_nodes..self.num_path_nodes as usize {
            self.path_nodes[path_node_index].num_links = 0;
            self.path_nodes[path_node_index].first_link = self.connections.len() as i16;

            for temp in temp_nodes.iter().filter(|temp| temp.link_state == 2) {
                let connected = if temp.link1 == path_node_index as i16 {
                    temp.link2
                } else if temp.link2 == path_node_index as i16 {
                    temp.link1
                } else {
                    continue;
                };
                if connected < 0 {
                    continue;
                }
                self.add_connection(
                    path_node_index,
                    connected as usize,
                    path_type,
                    Some(*temp),
                    false,
                );
            }

            let object_index = self.path_nodes[path_node_index].object_index;
            let iseg = (old_num_path_nodes..path_node_index)
                .filter(|index| self.path_nodes[*index].object_index == object_index)
                .count();
            let Some(map_object) = self.map_objects.get(object_index as usize).and_then(|m| *m)
            else {
                continue;
            };
            let istart = map_object.model_index * PATH_INFO_SLOTS_PER_OBJECT;

            let start = old_num_path_nodes.max(path_node_index.saturating_sub(12));
            let end = (self.num_path_nodes as usize).min(path_node_index + 12);
            for other_index in start..end {
                if other_index == path_node_index
                    || self.path_nodes[other_index].object_index != object_index
                {
                    continue;
                }
                let jseg = other_index as isize - path_node_index as isize + iseg as isize;
                if !(0..PATH_INFO_SLOTS_PER_OBJECT as isize).contains(&jseg) {
                    continue;
                }
                let Some(info_i) = self.path_info(path_type, istart + iseg) else {
                    continue;
                };
                let Some(info_j) = self.path_info(path_type, istart + jseg as usize) else {
                    continue;
                };
                let linked = info_i.next == jseg as i8 || info_j.next == iseg as i8;
                if !linked {
                    continue;
                }
                let crosses = path_type == PATH_PED
                    && ((info_i.next == jseg as i8 && info_i.crossing)
                        || (info_j.next == iseg as i8 && info_j.crossing));
                self.add_connection(path_node_index, other_index, path_type, None, crosses);
            }
        }

        if path_type == PATH_CAR {
            self.propagate_and_default_lane_counts();
            self.mark_car_dead_ends();
        } else {
            self.remove_isolated_ped_nodes(old_num_path_nodes, old_num_links);
        }
    }

    fn add_connection(
        &mut self,
        from: usize,
        to: usize,
        path_type: u8,
        temp: Option<CTempNode>,
        crosses_road: bool,
    ) {
        if self.connections.len() >= NUM_PATHCONNECTIONS {
            return;
        }

        self.connections.push(to as i16);
        let dist = distance3(self.path_nodes[from].pos, self.path_nodes[to].pos);
        self.distances.push(dist as i16);
        self.connection_flags.push(CConnectionFlags {
            crosses_road,
            traffic_light: false,
        });

        if path_type == PATH_CAR {
            let car_link = if let Some(temp) = temp {
                CCarPathLink {
                    pos: [temp.pos.x, temp.pos.y],
                    dir: [temp.dir_x, temp.dir_y],
                    path_node_index: from as i16,
                    num_left_lanes: temp.num_left_lanes,
                    num_right_lanes: temp.num_right_lanes,
                    traffic_light_type: 0,
                    bridge_lights: false,
                }
            } else {
                let a = self.path_nodes[from].pos;
                let b = self.path_nodes[to].pos;
                let pos = [(a.x + b.x) * 0.5, (a.y + b.y) * 0.5];
                let mut dx = b.x - a.x;
                let mut dy = b.y - a.y;
                let mag = (dx * dx + dy * dy).sqrt();
                if mag != 0.0 {
                    dx /= mag;
                    dy /= mag;
                }
                if from < to {
                    dx = -dx;
                    dy = -dy;
                }
                CCarPathLink {
                    pos,
                    dir: [dx, dy],
                    path_node_index: from as i16,
                    num_left_lanes: -1,
                    num_right_lanes: -1,
                    traffic_light_type: 0,
                    bridge_lights: false,
                }
            };
            let car_link_index = self.find_or_add_car_path_link(car_link);
            self.car_path_connections.push(car_link_index as i16);
        } else {
            self.car_path_connections.push(-1);
        }

        self.path_nodes[from].num_links = self.path_nodes[from].num_links.saturating_add(1);
        self.num_connections = self.connections.len() as i16;
    }

    fn path_info(&self, path_type: u8, index: usize) -> Option<CPathInfoForObject> {
        if path_type == PATH_CAR {
            self.info_for_tile_cars.get(index).copied()
        } else {
            self.info_for_tile_peds.get(index).copied()
        }
    }

    fn calc_node_coors(&self, map_object: CPathMapObject, info: CPathInfoForObject) -> CVector {
        let local = CVector::new(
            info.x as f32 / 16.0,
            info.y as f32 / 16.0,
            info.z as f32 / 16.0,
        );
        // `CFileLoader::LoadObjectInstance` converts the IPL quaternion to
        // `-RADTODEG(2 * acos(w))` before building the entity matrix
        // (`core/FileLoader.cpp:1560-1566`). `CPathFind::CalcNodeCoors`
        // multiplies path-local coordinates by that same matrix.  Applying
        // the quaternion with its original axis rotates in the opposite
        // direction and puts traffic rails out of alignment with buildings.
        let rotated = rotate_by_quaternion(
            local,
            [
                -map_object.rotation_axis[0],
                -map_object.rotation_axis[1],
                -map_object.rotation_axis[2],
            ],
            map_object.rotation_w,
        );
        CVector::new(
            rotated.x + map_object.position.x,
            rotated.y + map_object.position.y,
            rotated.z + map_object.position.z,
        )
    }

    fn resolve_next_path_node_index(
        &self,
        path_type: u8,
        start: usize,
        node_index: usize,
        info: CPathInfoForObject,
    ) -> usize {
        if info.next >= 0 {
            return info.next as usize;
        }

        for next in 0..PATH_INFO_SLOTS_PER_OBJECT {
            let Some(candidate) = self.path_info(path_type, start + next) else {
                continue;
            };
            if candidate.next == node_index as i8 {
                return next;
            }
        }
        0
    }

    fn find_or_add_car_path_link(&mut self, link: CCarPathLink) -> usize {
        if let Some(index) = self
            .car_path_links
            .iter()
            .position(|existing| existing.dir == link.dir && existing.pos == link.pos)
        {
            return index;
        }
        if self.car_path_links.len() >= NUM_CARPATHLINKS {
            return self.car_path_links.len().saturating_sub(1);
        }
        self.car_path_links.push(link);
        self.num_car_path_links = self.car_path_links.len() as i32;
        self.car_path_links.len() - 1
    }

    fn propagate_and_default_lane_counts(&mut self) {
        for _ in 0..10 {
            let mut done = true;
            for node_index in 0..self.num_path_nodes as usize {
                if self.path_nodes[node_index].num_links != 2 {
                    continue;
                }
                let first_link = self.path_nodes[node_index].first_link as usize;
                let Some(l1) = self
                    .car_path_connections
                    .get(first_link)
                    .and_then(|index| (*index >= 0).then_some(*index as usize))
                else {
                    continue;
                };
                let Some(l2) = self
                    .car_path_connections
                    .get(first_link + 1)
                    .and_then(|index| (*index >= 0).then_some(*index as usize))
                else {
                    continue;
                };

                let l1_unknown = self.car_path_links[l1].num_left_lanes == -1;
                let l2_unknown = self.car_path_links[l2].num_left_lanes == -1;
                if l1_unknown && !l2_unknown {
                    done = false;
                    let source = self.car_path_links[l2];
                    if source.path_node_index == node_index as i16 {
                        self.car_path_links[l1].num_left_lanes = source.num_right_lanes;
                        self.car_path_links[l1].num_right_lanes = source.num_left_lanes;
                    } else {
                        self.car_path_links[l1].num_left_lanes = source.num_left_lanes;
                        self.car_path_links[l1].num_right_lanes = source.num_right_lanes;
                    }
                    self.car_path_links[l1].path_node_index = node_index as i16;
                } else if !l1_unknown && l2_unknown {
                    done = false;
                    let source = self.car_path_links[l1];
                    if source.path_node_index == node_index as i16 {
                        self.car_path_links[l2].num_left_lanes = source.num_right_lanes;
                        self.car_path_links[l2].num_right_lanes = source.num_left_lanes;
                    } else {
                        self.car_path_links[l2].num_left_lanes = source.num_left_lanes;
                        self.car_path_links[l2].num_right_lanes = source.num_right_lanes;
                    }
                    self.car_path_links[l2].path_node_index = node_index as i16;
                } else if l1_unknown && l2_unknown {
                    done = false;
                }
            }
            if done {
                break;
            }
        }

        for link in &mut self.car_path_links {
            if link.num_left_lanes < 0 {
                link.num_left_lanes = 1;
            }
            if link.num_right_lanes < 0 {
                link.num_right_lanes = 1;
            }
        }
    }

    fn mark_car_dead_ends(&mut self) {
        loop {
            let mut changed = false;
            for node_index in 0..self.num_path_nodes as usize {
                self.path_nodes[node_index].disabled = false;
                self.path_nodes[node_index].between_levels = false;
                if self.path_nodes[node_index].dead_end {
                    continue;
                }

                let live_neighbours = (0..self.path_nodes[node_index].num_links as usize)
                    .filter_map(|offset| {
                        let connection = self.path_nodes[node_index].first_link as usize + offset;
                        self.connections.get(connection).copied()
                    })
                    .filter(|connected| {
                        *connected >= 0 && !self.path_nodes[*connected as usize].dead_end
                    })
                    .count();
                if live_neighbours < 2 {
                    self.path_nodes[node_index].dead_end = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn remove_isolated_ped_nodes(&mut self, old_num_path_nodes: usize, old_num_links: usize) {
        let mut node_index = old_num_path_nodes;
        while node_index < self.num_path_nodes as usize {
            if self.path_nodes[node_index].num_links != 0 {
                node_index += 1;
                continue;
            }

            for index in node_index..self.num_path_nodes as usize - 1 {
                self.path_nodes[index] = self.path_nodes[index + 1];
            }
            for connection in self.connections.iter_mut().skip(old_num_links) {
                if *connection >= node_index as i16 {
                    *connection -= 1;
                }
            }
            for map_object in self
                .map_objects
                .iter_mut()
                .take(self.num_map_objects.max(0) as usize)
                .flatten()
            {
                let node_indices = &mut map_object.node_indices[PATH_PED as usize];
                let mut k = 0;
                while k < 12 {
                    if node_indices[k] == node_index as i16 {
                        for l in k..12 - 1 {
                            node_indices[l] = node_indices[l + 1];
                        }
                        node_indices[11] = -1;
                        k += 1;
                    } else {
                        if node_indices[k] > node_index as i16 {
                            node_indices[k] -= 1;
                        }
                        k += 1;
                    }
                }
            }
            self.num_path_nodes -= 1;
        }
    }

    fn count_flood_fill_groups(&mut self, path_type: u8) {
        let (first, last) = if path_type == PATH_CAR {
            (0usize, self.num_car_path_nodes.max(0) as usize)
        } else {
            (
                self.num_car_path_nodes.max(0) as usize,
                self.num_path_nodes.max(0) as usize,
            )
        };

        let mut group = 0u16;
        for node in &mut self.path_nodes[first..last] {
            node.group = 0;
        }

        for start in first..last {
            if self.path_nodes[start].group != 0 {
                continue;
            }
            group = group.wrapping_add(1);
            let mut stack = vec![start];
            self.path_nodes[start].group = encoded_path_group(group);
            while let Some(node_index) = stack.pop() {
                let first_link = self.path_nodes[node_index].first_link as usize;
                let num_links = self.path_nodes[node_index].num_links as usize;
                for offset in 0..num_links {
                    let Some(connected) = self.connections.get(first_link + offset).copied() else {
                        continue;
                    };
                    if connected < first as i16 || connected >= last as i16 {
                        continue;
                    }
                    let connected = connected as usize;
                    if self.path_nodes[connected].group == 0 {
                        self.path_nodes[connected].group = encoded_path_group(group);
                        stack.push(connected);
                    }
                }
            }
        }
        self.num_groups[path_type as usize] = group.min(u8::MAX as u16) as u8;
    }

    pub fn calc_road_density(&self, x: f32, y: f32) -> f32 {
        let mut density = 0.0;
        for node_index in 0..self.num_car_path_nodes.max(0) as usize {
            let node = self.path_nodes[node_index];
            if (node.pos.x - x).abs() < 80.0 && (node.pos.y - y).abs() < 80.0 && node.num_links > 0
            {
                for offset in 0..node.num_links as usize {
                    let connection = node.first_link as usize + offset;
                    let Some(next_node) = self.connections.get(connection).copied() else {
                        continue;
                    };
                    if next_node < 0 {
                        continue;
                    }
                    let dist = distance2(node.pos, self.path_nodes[next_node as usize].pos);
                    let Some(car_link_index) = self.car_path_connections.get(connection) else {
                        continue;
                    };
                    if *car_link_index < 0 {
                        continue;
                    }
                    let link = self.car_path_links[*car_link_index as usize];
                    density += link.num_left_lanes as f32 * dist;
                    density += link.num_right_lanes as f32 * dist;
                }
            }
        }
        density / 2500.0
    }

    pub fn connected_node(&self, connection_index: usize) -> Option<usize> {
        self.connections
            .get(connection_index)
            .and_then(|node| (*node >= 0).then_some(*node as usize))
    }

    pub fn connection_crosses_road(&self, connection_index: usize) -> bool {
        self.connection_flags
            .get(connection_index)
            .is_some_and(|flags| flags.crosses_road)
    }

    pub fn connection_has_traffic_light(&self, connection_index: usize) -> bool {
        self.connection_flags
            .get(connection_index)
            .is_some_and(|flags| flags.traffic_light)
    }

    pub fn connection_set_traffic_light(&mut self, connection_index: usize) {
        if let Some(flags) = self.connection_flags.get_mut(connection_index) {
            flags.traffic_light = true;
        }
    }

    pub fn test_for_ped_traffic_light(&self, node1: usize, node2: usize) -> bool {
        self.find_connection_between(node1, node2)
            .is_some_and(|connection| self.connection_has_traffic_light(connection))
    }

    pub fn test_crosses_road(&self, node1: usize, node2: usize) -> bool {
        self.find_connection_between(node1, node2)
            .is_some_and(|connection| self.connection_crosses_road(connection))
    }

    pub fn set_links_bridge_lights(&mut self, x1: f32, x2: f32, y1: f32, y2: f32, enable: bool) {
        for link in &mut self.car_path_links {
            if x1 < link.pos[0] && link.pos[0] < x2 && y1 < link.pos[1] && link.pos[1] < y2 {
                link.bridge_lights = enable;
            }
        }
    }

    pub fn switch_off_node_and_neighbours(&mut self, node_id: usize, disable: bool) {
        if node_id >= self.num_path_nodes.max(0) as usize {
            return;
        }
        self.path_nodes[node_id].disabled = disable;
        if self.path_nodes[node_id].num_links >= 3 {
            return;
        }

        let links = self.node_connections(node_id);
        for next in links {
            if self.path_nodes[next].disabled != disable && self.path_nodes[next].num_links < 3 {
                self.switch_off_node_and_neighbours(next, disable);
            }
        }
    }

    pub fn switch_roads_off_in_area(
        &mut self,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        z1: f32,
        z2: f32,
        disable: bool,
    ) {
        for node_index in 0..self.num_car_path_nodes.max(0) as usize {
            let node = self.path_nodes[node_index];
            if point_in_closed_box(node.pos, x1, x2, y1, y2, z1, z2) && node.disabled != disable {
                self.switch_off_node_and_neighbours(node_index, disable);
            }
        }
    }

    pub fn switch_ped_roads_off_in_area(
        &mut self,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        z1: f32,
        z2: f32,
        disable: bool,
    ) {
        for node_index in
            self.num_car_path_nodes.max(0) as usize..self.num_path_nodes.max(0) as usize
        {
            let node = self.path_nodes[node_index];
            if point_in_closed_box(node.pos, x1, x2, y1, y2, z1, z2) && node.disabled != disable {
                self.switch_off_node_and_neighbours(node_index, disable);
            }
        }
    }

    pub fn switch_roads_in_angled_area(
        &mut self,
        p1: CVector,
        p2: CVector,
        length: f32,
        path_type: u8,
        enable: bool,
    ) {
        let (first_node, last_node) = if path_type != 0 {
            (0usize, self.num_car_path_nodes.max(0) as usize)
        } else {
            (
                self.num_car_path_nodes.max(0) as usize,
                self.num_path_nodes.max(0) as usize,
            )
        };
        let z1 = p1.z.min(p2.z);
        let z2 = p1.z.max(p2.z);
        let v12 = normalise2([p2.x - p1.x, p2.y - p1.y]);
        let len12 = distance2(p1, p2);
        let v23 = normalise2([v12[1], -v12[0]]);
        let disable = !enable;

        for node_index in first_node..last_node {
            let pos = self.path_nodes[node_index].pos;
            if pos.z < z1 || pos.z > z2 {
                continue;
            }
            let d = [pos.x - p1.x, pos.y - p1.y];
            let dot12 = dot2(d, v12);
            if dot12 < 0.0 || dot12 > len12 {
                continue;
            }
            let dot23 = dot2(d, v23);
            if dot23 < 0.0 || dot23 > length {
                continue;
            }
            if self.path_nodes[node_index].disabled != disable {
                self.switch_off_node_and_neighbours(node_index, disable);
            }
        }
    }

    pub fn mark_roads_between_levels_node_and_neighbours(&mut self, node_id: usize) {
        if node_id >= self.num_path_nodes.max(0) as usize {
            return;
        }
        self.path_nodes[node_id].between_levels = true;
        if self.path_nodes[node_id].num_links >= 3 {
            return;
        }

        let links = self.node_connections(node_id);
        for next in links {
            if !self.path_nodes[next].between_levels && self.path_nodes[next].num_links < 3 {
                self.mark_roads_between_levels_node_and_neighbours(next);
            }
        }
    }

    pub fn mark_roads_between_levels_in_area(
        &mut self,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        z1: f32,
        z2: f32,
    ) {
        for node_index in 0..self.num_path_nodes.max(0) as usize {
            let node = self.path_nodes[node_index];
            if point_in_open_box(node.pos, x1, x2, y1, y2, z1, z2) {
                self.mark_roads_between_levels_node_and_neighbours(node_index);
            }
        }
    }

    pub fn ped_mark_roads_between_levels_in_area(
        &mut self,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        z1: f32,
        z2: f32,
    ) {
        for node_index in
            self.num_car_path_nodes.max(0) as usize..self.num_path_nodes.max(0) as usize
        {
            let node = self.path_nodes[node_index];
            if point_in_open_box(node.pos, x1, x2, y1, y2, z1, z2) {
                self.mark_roads_between_levels_node_and_neighbours(node_index);
            }
        }
    }

    pub fn find_node_closest_to_coors(
        &self,
        coors: CVector,
        path_type: u8,
        dist_limit: f32,
        ignore_disabled: bool,
        ignore_between_levels: bool,
    ) -> i32 {
        let (first_node, last_node) = self.path_node_range(path_type);
        let mut closest_dist = 10000.0;
        let mut closest_node = 0i32;

        for node_index in first_node..last_node {
            let node = self.path_nodes[node_index];
            if ignore_disabled && node.disabled {
                continue;
            }
            if ignore_between_levels && node.between_levels {
                continue;
            }
            if node.unk_bits != 1 && node.unk_bits != 2 {
                continue;
            }
            let dist = (node.pos.x - coors.x).abs()
                + (node.pos.y - coors.y).abs()
                + 3.0 * (node.pos.z - coors.z).abs();
            if dist < closest_dist {
                closest_dist = dist;
                closest_node = node_index as i32;
            }
        }

        if closest_dist < dist_limit {
            closest_node
        } else {
            -1
        }
    }

    pub fn find_node_closest_to_coors_favour_direction(
        &self,
        coors: CVector,
        path_type: u8,
        dir_x: f32,
        dir_y: f32,
    ) -> i32 {
        let (first_node, last_node) = self.path_node_range(path_type);
        let dir = normalise2([dir_x, dir_y]);
        let mut closest_dist = 10000.0;
        let mut closest_node = 0i32;

        for node_index in first_node..last_node {
            let node = self.path_nodes[node_index];
            if node.unk_bits != 1 && node.unk_bits != 2 {
                continue;
            }
            let delta = [node.pos.x - coors.x, node.pos.y - coors.y];
            let mut dist = delta[0].abs() + delta[1].abs() + 3.0 * (node.pos.z - coors.z).abs();
            if dist < closest_dist {
                let delta_dir = normalise2(delta);
                dist -= (dot2(delta_dir, dir) - 1.0) * 20.0;
                if dist < closest_dist {
                    closest_dist = dist;
                    closest_node = node_index as i32;
                }
            }
        }

        closest_node
    }

    pub fn find_node_orientation_for_car_placement(&self, node_id: usize) -> f32 {
        let Some(node) = self.path_nodes.get(node_id).copied() else {
            return 0.0;
        };
        if node.num_links == 0 {
            return 0.0;
        }
        let Some(next) = self.connected_node(node.first_link as usize) else {
            return 0.0;
        };
        heading_degrees(self.path_nodes[next].pos, node.pos)
    }

    pub fn find_node_orientation_for_car_placement_facing_destination(
        &self,
        node_id: usize,
        x: f32,
        y: f32,
        towards: bool,
    ) -> f32 {
        let Some(node) = self.path_nodes.get(node_id).copied() else {
            return 0.0;
        };
        if node.num_links == 0 {
            return 0.0;
        }

        let target_dir = normalise2([x - node.pos.x, y - node.pos.y]);
        let mut best_node = self
            .connected_node(node.first_link as usize)
            .unwrap_or(node_id);
        let mut best_dot = if towards { -2.0 } else { 2.0 };

        for offset in 0..node.num_links as usize {
            let Some(next) = self.connected_node(node.first_link as usize + offset) else {
                continue;
            };
            let dir = normalise2([
                self.path_nodes[next].pos.x - node.pos.x,
                self.path_nodes[next].pos.y - node.pos.y,
            ]);
            let angle = dot2(dir, target_dir);
            if (towards && angle > best_dot) || (!towards && angle < best_dot) {
                best_dot = angle;
                best_node = next;
            }
        }

        heading_degrees(self.path_nodes[best_node].pos, node.pos)
    }

    pub fn new_generate_car_creation_coors(
        &self,
        x: f32,
        y: f32,
        dir_x: f32,
        dir_y: f32,
        spawn_dist: f32,
        angle_limit: f32,
        forward: bool,
        ignore_disabled: bool,
        general: &mut CGeneral,
    ) -> Option<CCarCreationCoors> {
        if self.num_car_path_nodes == 0 {
            return None;
        }

        let dir = [dir_x, dir_y];
        for _ in 0..500 {
            let node1 = ((general.get_random_number() >> 3) as usize)
                % self.num_car_path_nodes.max(1) as usize;
            if self.path_nodes[node1].disabled && !ignore_disabled {
                continue;
            }
            let dist1 = distance2(self.path_nodes[node1].pos, CVector::new(x, y, 0.0));
            if dist1 >= spawn_dist + 60.0 {
                continue;
            }

            let d1 = dist1 - spawn_dist;
            for offset in 0..self.path_nodes[node1].num_links as usize {
                let Some(node2) =
                    self.connected_node(self.path_nodes[node1].first_link as usize + offset)
                else {
                    continue;
                };
                if self.path_nodes[node2].disabled && !ignore_disabled {
                    continue;
                }
                let dist2 = distance2(self.path_nodes[node2].pos, CVector::new(x, y, 0.0));
                let d2 = dist2 - spawn_dist;
                if d1 * d2 >= 0.0 {
                    continue;
                }

                let f2 = d1.abs() / (d1.abs() + d2.abs());
                let f1 = 1.0 - f2;
                let pos = lerp_vector(
                    self.path_nodes[node1].pos,
                    self.path_nodes[node2].pos,
                    f1,
                    f2,
                );
                let dist2d = normalise2([pos.x - x, pos.y - y]);
                let dot = dot2(dist2d, dir);
                if (forward && dot > angle_limit) || (!forward && dot <= angle_limit) {
                    return Some(CCarCreationCoors {
                        position: pos,
                        node1: node1 as i32,
                        node2: node2 as i32,
                        position_between_nodes: f2,
                    });
                }
            }
        }

        None
    }

    pub fn generate_ped_creation_coors<F, G>(
        &self,
        x: f32,
        y: f32,
        mut min_dist: f32,
        mut max_dist: f32,
        min_dist_off_screen: f32,
        max_dist_off_screen: f32,
        general: &mut CGeneral,
        mut is_sphere_visible: F,
        mut find_ground_z: G,
    ) -> Option<CPedCreationCoors>
    where
        F: FnMut(CVector, f32) -> bool,
        G: FnMut(f32, f32, f32) -> Option<f32>,
    {
        if self.num_ped_path_nodes == 0 {
            return None;
        }

        for _ in 0..400 {
            let node1 = self.num_car_path_nodes.max(0) as usize
                + (general.get_random_number() as usize % self.num_ped_path_nodes.max(1) as usize);
            if distance2_sqr(self.path_nodes[node1].pos, CVector::new(x, y, 0.0))
                >= squared(max_dist + 30.0)
            {
                continue;
            }
            if self.path_nodes[node1].num_links == 0 {
                continue;
            }

            let link = self.path_nodes[node1].first_link as usize
                + (general.get_random_number() as usize
                    % self.path_nodes[node1].num_links as usize);
            if self.connection_crosses_road(link) {
                continue;
            }
            let Some(node2) = self.connected_node(link) else {
                continue;
            };
            if self.path_nodes[node1].disabled || self.path_nodes[node2].disabled {
                continue;
            }

            let f2 = (general.get_random_number() & 0xFF) as f32 / 256.0;
            let f1 = 1.0 - f2;
            let mut pos = lerp_vector(
                self.path_nodes[node1].pos,
                self.path_nodes[node2].pos,
                f1,
                f2,
            );
            if distance2(pos, CVector::new(x, y, 0.0)) >= max_dist + 20.0 {
                continue;
            }

            pos.x += ((general.get_random_number() & 0xFF) as i32 - 128) as f32 * 0.01;
            pos.y += ((general.get_random_number() & 0xFF) as i32 - 128) as f32 * 0.01;
            let dist = distance2(pos, CVector::new(x, y, 0.0));

            if !is_sphere_visible(pos, 2.0) {
                min_dist = min_dist_off_screen;
                max_dist = max_dist_off_screen;
            }
            if !(min_dist < dist && dist < max_dist) {
                continue;
            }

            let ground_z = find_ground_z(pos.x, pos.y, pos.z + 2.0)?;
            if (ground_z - pos.z).abs() > 3.0 {
                return None;
            }
            pos.z = ground_z;
            return Some(CPedCreationCoors {
                position: pos,
                node1: node1 as i32,
                node2: node2 as i32,
                position_between_nodes: f2,
            });
        }

        None
    }

    pub fn find_road_object_closest_to_coors(
        &self,
        coors: CVector,
        path_type: u8,
    ) -> Option<usize> {
        let mut closest_map_object = None;
        let mut closest_dist = 10000.0;

        for map_object_index in 0..self.num_map_objects.max(0) as usize {
            let Some(map_object) = self
                .map_objects
                .get(map_object_index)
                .and_then(|value| *value)
            else {
                continue;
            };
            if map_object.node_indices[path_type as usize][0] < 0 {
                continue;
            }

            let f_dist = (map_object.position.x - coors.x).abs()
                + (map_object.position.y - coors.y).abs()
                + (map_object.position.z - coors.z).abs();
            if f_dist >= 200.0 && f_dist >= closest_dist {
                continue;
            }

            for node1 in map_object.node_indices[path_type as usize] {
                if node1 < 0 {
                    break;
                }
                let node1 = node1 as usize;
                let Some(path_node) = self.path_nodes.get(node1).copied() else {
                    continue;
                };
                for offset in 0..path_node.num_links as usize {
                    let Some(node2) = self.connected_node(path_node.first_link as usize + offset)
                    else {
                        continue;
                    };
                    let line_dist = dist_to_line(
                        self.path_nodes[node1].pos,
                        self.path_nodes[node2].pos,
                        coors,
                    );
                    if line_dist < closest_dist {
                        closest_dist = line_dist;
                        let d1 = distance3(coors, self.path_nodes[node1].pos);
                        let d2 = distance3(coors, self.path_nodes[node2].pos);
                        let object_index = if d1 * d1 < d2 * d2 {
                            self.path_nodes[node1].object_index
                        } else {
                            self.path_nodes[node2].object_index
                        };
                        if object_index >= 0 {
                            closest_map_object = Some(object_index as usize);
                        }
                    }
                }
            }
        }

        closest_map_object
    }

    pub fn save_path_flags(&self) -> Vec<u8> {
        let count = self.num_path_nodes.max(0) as usize;
        let n = count / 8 + 1;
        let mut buffer = vec![0u8; 2 * n];

        for node_index in 0..count {
            if self.path_nodes[node_index].disabled {
                buffer[node_index / 8] |= 1 << (node_index % 8);
            }
            if self.path_nodes[node_index].between_levels {
                buffer[node_index / 8 + n] |= 1 << (node_index % 8);
            }
        }

        buffer
    }

    pub fn load_path_flags(&mut self, buffer: &[u8]) {
        let count = self.num_path_nodes.max(0) as usize;
        let n = count / 8 + 1;
        if buffer.len() < 2 * n {
            return;
        }

        for node_index in 0..count {
            self.path_nodes[node_index].disabled =
                buffer[node_index / 8] & (1 << (node_index % 8)) != 0;
            self.path_nodes[node_index].between_levels =
                buffer[node_index / 8 + n] & (1 << (node_index % 8)) != 0;
        }
    }

    pub fn do_path_search(
        &mut self,
        path_type: u8,
        start: CVector,
        start_node_id: Option<usize>,
        target: CVector,
        max_num_nodes: usize,
        dist_limit: f32,
        forced_target_node: Option<usize>,
    ) -> CPathSearchResult {
        self.do_path_search_with_start_object(
            path_type,
            start,
            start_node_id,
            None,
            target,
            max_num_nodes,
            dist_limit,
            forced_target_node,
        )
    }

    /// re3 `CPathFind::DoPathSearch` with the caller's current
    /// `CVehicle::m_treadable[path_type]` made explicit for ECS callers.
    pub fn do_path_search_with_start_object(
        &mut self,
        path_type: u8,
        start: CVector,
        start_node_id: Option<usize>,
        start_object_id: Option<usize>,
        target: CVector,
        max_num_nodes: usize,
        dist_limit: f32,
        forced_target_node: Option<usize>,
    ) -> CPathSearchResult {
        let target_node_id = if let Some(target_node_id) = forced_target_node {
            if target_node_id < self.num_path_nodes.max(0) as usize {
                target_node_id as i32
            } else {
                -1
            }
        } else {
            self.find_node_closest_to_coors(target, path_type, dist_limit, false, false)
        };
        if target_node_id < 0 {
            return CPathSearchResult::no_path();
        }
        let target_node_id = target_node_id as usize;

        let start_object_index = if let Some(start_node_id) = start_node_id {
            let Some(node) = self.path_nodes.get(start_node_id) else {
                return CPathSearchResult::no_path();
            };
            if node.object_index < 0 {
                return CPathSearchResult::no_path();
            }
            node.object_index as usize
        } else if let Some(start_object_id) = start_object_id {
            if self
                .map_objects
                .get(start_object_id)
                .and_then(|value| value.as_ref())
                .is_none()
            {
                return CPathSearchResult::no_path();
            }
            start_object_id
        } else {
            let Some(object_index) = self.find_road_object_closest_to_coors(start, path_type)
            else {
                return CPathSearchResult::no_path();
            };
            object_index
        };

        let Some(start_object) = self
            .map_objects
            .get(start_object_index)
            .and_then(|value| *value)
        else {
            return CPathSearchResult::no_path();
        };

        let target_group = self.path_nodes[target_node_id].group;
        let mut start_candidates = Vec::new();
        if let Some(start_node_id) = start_node_id {
            if self
                .path_nodes
                .get(start_node_id)
                .is_some_and(|node| node.group == target_group)
            {
                start_candidates.push(start_node_id);
            }
        } else {
            for node_index in start_object.node_indices[path_type as usize] {
                if node_index < 0 {
                    break;
                }
                let node_index = node_index as usize;
                if self.path_nodes[node_index].group == target_group {
                    start_candidates.push(node_index);
                }
            }
        }
        if start_candidates.is_empty() {
            return CPathSearchResult::no_path();
        }

        if start_node_id.is_none() {
            let first_node = start_object.node_indices[path_type as usize][0];
            if first_node < 0 || self.path_nodes[first_node as usize].group != target_group {
                return CPathSearchResult::no_path();
            }
        }

        let distances = self.dijkstra_from_target(target_node_id, &start_candidates);
        let mut current_node = if let Some(start_node_id) = start_node_id {
            start_node_id
        } else {
            let mut min_dist = i32::from(MAX_DIST);
            let mut selected = start_candidates[0];
            for node_index in start_candidates {
                let dist = distances[node_index]
                    + distance3(self.path_nodes[node_index].pos, start) as i32;
                if dist < min_dist {
                    min_dist = dist;
                    selected = node_index;
                }
            }
            selected
        };

        let mut result = CPathSearchResult {
            nodes: Vec::new(),
            num_nodes: 0,
            distance: distances[current_node] as f32,
        };
        if start_node_id.is_none() {
            result.distance += distance3(self.path_nodes[current_node].pos, start) as i32 as f32;
            if max_num_nodes != 0 {
                result.nodes.push(current_node);
                result.num_nodes = 1;
            }
        }

        while result.nodes.len() < max_num_nodes && current_node != target_node_id {
            let Some(next) = self.next_node_towards_target(current_node, &distances) else {
                break;
            };
            current_node = next;
            result.nodes.push(current_node);
            result.num_nodes = result.nodes.len() as i16;
        }

        result
    }

    pub fn test_coors_closeness(&mut self, target: CVector, path_type: u8, start: CVector) -> bool {
        let result = if path_type == PATH_CAR {
            self.do_path_search(path_type, start, None, target, 32, 999999.88, None)
        } else {
            self.do_path_search(path_type, start, None, target, 0, 50.0, None)
        };
        if path_type == PATH_CAR {
            result.distance < 160.0
        } else {
            result.distance < 100.0
        }
    }

    pub fn find_next_node_wandering<F>(
        &self,
        path_type: u8,
        coors: CVector,
        last_node: Option<usize>,
        cur_dir: u8,
        mut line_of_sight_clear: F,
    ) -> Option<(usize, u8)>
    where
        F: FnMut(CVector, CVector) -> bool,
    {
        let mut node_index = last_node.unwrap_or(usize::MAX);
        if node_index >= self.num_path_nodes.max(0) as usize
            || distance3(coors, self.path_nodes[node_index].pos).powi(2) > 7.0
        {
            let Some(object_index) = self.find_road_object_closest_to_coors(coors, path_type)
            else {
                return None;
            };
            let Some(map_object) = self.map_objects.get(object_index).and_then(|value| *value)
            else {
                return None;
            };
            let mut node_dist = f32::MAX;
            for candidate in map_object.node_indices[path_type as usize] {
                if candidate < 0 {
                    break;
                }
                let candidate = candidate as usize;
                let dist = distance3(coors, self.path_nodes[candidate].pos).powi(2);
                if dist < node_dist {
                    node_dist = dist;
                    node_index = candidate;
                }
            }
        }

        if node_index >= self.num_path_nodes.max(0) as usize {
            return None;
        }

        let v_cur_dir = [
            (cur_dir as f32 * std::f32::consts::PI / 4.0).sin(),
            (cur_dir as f32 * std::f32::consts::PI / 4.0).cos(),
        ];
        let node = self.path_nodes[node_index];
        let mut next_node = None;
        let mut next_dir = 0u8;
        let mut best_dot = -999999.0;

        for offset in 0..node.num_links as usize {
            let Some(next) = self.connected_node(node.first_link as usize + offset) else {
                continue;
            };
            if !node.disabled && self.path_nodes[next].disabled {
                continue;
            }
            let ped_coors = CVector::new(coors.x, coors.y, coors.z + 1.0);
            let node_coors = CVector::new(
                self.path_nodes[next].pos.x,
                self.path_nodes[next].pos.y,
                self.path_nodes[next].pos.z + 1.0,
            );
            if !line_of_sight_clear(ped_coors, node_coors) {
                continue;
            }

            let node_dir = normalise2([
                self.path_nodes[next].pos.x - node.pos.x,
                self.path_nodes[next].pos.y - node.pos.y,
            ]);
            let dot = dot2(node_dir, v_cur_dir);
            if dot >= best_dot {
                next_node = Some(next);
                best_dot = dot;
                next_dir = direction_from_node_dir(node_dir);
            }
        }

        // re3 `FindNextNodeWandering` falls back to the current node and
        // writes direction 0 when no connected node survives the filters.
        // Direction 0 is also the valid north heading, so callers must use
        // the returned node rather than treating the direction as failure.
        Some((next_node.unwrap_or(node_index), next_dir))
    }

    fn find_connection_between(&self, node1: usize, node2: usize) -> Option<usize> {
        let node = self.path_nodes.get(node1)?;
        for offset in 0..node.num_links as usize {
            let connection = node.first_link as usize + offset;
            if self.connected_node(connection) == Some(node2) {
                return Some(connection);
            }
        }
        None
    }

    fn node_connections(&self, node_id: usize) -> Vec<usize> {
        let Some(node) = self.path_nodes.get(node_id) else {
            return Vec::new();
        };
        (0..node.num_links as usize)
            .filter_map(|offset| self.connected_node(node.first_link as usize + offset))
            .collect()
    }

    fn path_node_range(&self, path_type: u8) -> (usize, usize) {
        if path_type == PATH_CAR {
            (0usize, self.num_car_path_nodes.max(0) as usize)
        } else {
            (
                self.num_car_path_nodes.max(0) as usize,
                self.num_path_nodes.max(0) as usize,
            )
        }
    }

    fn dijkstra_from_target(&self, target_node_id: usize, start_candidates: &[usize]) -> Vec<i32> {
        let node_count = self.num_path_nodes.max(0) as usize;
        let mut distances = vec![i32::from(MAX_DIST); node_count];
        let mut heap = BinaryHeap::new();

        distances[target_node_id] = 0;
        heap.push(Reverse((0i32, target_node_id)));

        let mut remaining = start_candidates.to_vec();

        while let Some(Reverse((distance, node_index))) = heap.pop() {
            if distance != distances[node_index] {
                continue;
            }

            if let Some(pos) = remaining.iter().position(|&x| x == node_index) {
                remaining.swap_remove(pos);
                if remaining.is_empty() {
                    break;
                }
            }

            let node = self.path_nodes[node_index];
            for offset in 0..node.num_links as usize {
                let connection_index = node.first_link as usize + offset;
                let Some(next) = self.connected_node(connection_index) else {
                    continue;
                };
                if next >= node_count {
                    continue;
                }
                let next_distance = distance
                    + self
                        .distances
                        .get(connection_index)
                        .copied()
                        .map(i32::from)
                        .unwrap_or(i32::from(MAX_DIST));
                if next_distance < distances[next] {
                    distances[next] = next_distance;
                    heap.push(Reverse((next_distance, next)));
                }
            }
        }

        distances
    }

    fn next_node_towards_target(&self, current_node: usize, distances: &[i32]) -> Option<usize> {
        let node = self.path_nodes[current_node];
        for offset in 0..node.num_links as usize {
            let connection_index = node.first_link as usize + offset;
            let next = self.connected_node(connection_index)?;
            let dist = self
                .distances
                .get(connection_index)
                .copied()
                .map(i32::from)?;
            if distances[current_node] - dist == distances[next] {
                return Some(next);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathDataPreparePhase {
    #[default]
    Reset,
    Cars,
    Peds,
    Roadblocks,
    CarFloodFill,
    PedFloodFill,
    Done,
}

impl CPathInfoForObject {
    fn from_ped(node: &GtaPathNode) -> Self {
        Self {
            x: path_coord(node.position[0]),
            y: path_coord(node.position[1]),
            z: path_coord(node.position[2]),
            node_type: node.node_type as i8,
            next: node.next as i8,
            num_left_lanes: 0,
            num_right_lanes: 0,
            crossing: node.cross != 0,
        }
    }

    fn from_car(node: &GtaPathNode) -> Self {
        Self {
            x: path_coord(node.position[0]),
            y: path_coord(node.position[1]),
            z: path_coord(node.position[2]),
            node_type: node.node_type as i8,
            next: node.next as i8,
            num_left_lanes: node.num_left.unwrap_or(0) as i8,
            num_right_lanes: node.num_right.unwrap_or(0) as i8,
            crossing: node.cross != 0,
        }
    }
}

fn path_info_index(id: usize, node: usize) -> usize {
    id * PATH_INFO_SLOTS_PER_OBJECT + node
}

fn encoded_path_group(group: u16) -> i8 {
    let value = group as u8 as i8;
    if value == 0 { i8::MIN } else { value }
}

fn path_coord(value: f32) -> i16 {
    // re3 parses PATH coordinates as floats, passes them directly to the
    // int16 StoreNodeInfo* parameters, then CalcNodeCoors divides by 16.
    // The IDE data is already in GTA's fixed-point 1/16 unit scale.
    value as i16
}

fn midpoint(a: CVector, b: CVector) -> CVector {
    CVector::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5, (a.z + b.z) * 0.5)
}

fn lerp_vector(a: CVector, b: CVector, f1: f32, f2: f32) -> CVector {
    CVector::new(
        a.x * f1 + b.x * f2,
        a.y * f1 + b.y * f2,
        a.z * f1 + b.z * f2,
    )
}

fn distance3(a: CVector, b: CVector) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn distance2(a: CVector, b: CVector) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn distance2_sqr(a: CVector, b: CVector) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn squared(value: f32) -> f32 {
    value * value
}

fn point_in_closed_box(
    point: CVector,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    z1: f32,
    z2: f32,
) -> bool {
    x1 <= point.x
        && point.x <= x2
        && y1 <= point.y
        && point.y <= y2
        && z1 <= point.z
        && point.z <= z2
}

fn point_in_open_box(point: CVector, x1: f32, x2: f32, y1: f32, y2: f32, z1: f32, z2: f32) -> bool {
    x1 < point.x && point.x < x2 && y1 < point.y && point.y < y2 && z1 < point.z && point.z < z2
}

fn normalise2(value: [f32; 2]) -> [f32; 2] {
    let mag = (value[0] * value[0] + value[1] * value[1]).sqrt();
    if mag == 0.0 {
        [0.0, 0.0]
    } else {
        [value[0] / mag, value[1] / mag]
    }
}

fn dot2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn heading_degrees(target: CVector, origin: CVector) -> f32 {
    let x = target.x - origin.x;
    let y = target.y - origin.y;
    (-x).atan2(y).to_degrees()
}

fn direction_from_node_dir(node_dir: [f32; 2]) -> u8 {
    let x = node_dir[0];
    let y = node_dir[1];
    if x < 0.0 {
        if 2.0 * y.abs() < -x {
            6
        } else if -2.0 * x < y {
            0
        } else if 2.0 * x > y {
            4
        } else if y > 0.0 {
            7
        } else {
            5
        }
    } else if 2.0 * y.abs() < x {
        2
    } else if 2.0 * x < y {
        0
    } else if -2.0 * x > y {
        4
    } else if y > 0.0 {
        1
    } else {
        3
    }
}

fn dist_to_line(a: CVector, b: CVector, point: CVector) -> f32 {
    let ab = CVector::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ap = CVector::new(point.x - a.x, point.y - a.y, point.z - a.z);
    let ab_len_sqr = ab.x * ab.x + ab.y * ab.y + ab.z * ab.z;
    if ab_len_sqr == 0.0 {
        return distance3(a, point);
    }

    let t = ((ap.x * ab.x + ap.y * ab.y + ap.z * ab.z) / ab_len_sqr).clamp(0.0, 1.0);
    let closest = CVector::new(a.x + ab.x * t, a.y + ab.y * t, a.z + ab.z * t);
    distance3(closest, point)
}

fn rotate_by_quaternion(value: CVector, axis: [f32; 3], w: f32) -> CVector {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2] + w * w).sqrt();
    if len == 0.0 {
        return value;
    }

    let qx = axis[0] / len;
    let qy = axis[1] / len;
    let qz = axis[2] / len;
    let qw = w / len;

    let uv = cross([qx, qy, qz], [value.x, value.y, value.z]);
    let uuv = cross([qx, qy, qz], uv);
    CVector::new(
        value.x + (uv[0] * qw + uuv[0]) * 2.0,
        value.y + (uv[1] * qw + uuv[1]) * 2.0,
        value.z + (uv[2] * qw + uuv[2]) * 2.0,
    )
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

    #[test]
    fn wandering_keeps_north_as_valid_direction_zero() {
        let mut paths = CPathFind::init();
        paths.num_path_nodes = 2;
        paths.num_ped_path_nodes = 2;
        paths.path_nodes[0].pos = CVector::ZERO;
        paths.path_nodes[0].first_link = 0;
        paths.path_nodes[0].num_links = 1;
        paths.path_nodes[1].pos = CVector::new(0.0, 10.0, 0.0);
        paths.connections = vec![1];

        let next = paths.find_next_node_wandering(PATH_PED, CVector::ZERO, Some(0), 0, |_, _| true);

        assert_eq!(next, Some((1, 0)));
    }

    #[test]
    fn wandering_without_a_usable_link_returns_current_node_like_re3() {
        let mut paths = CPathFind::init();
        paths.num_path_nodes = 1;
        paths.num_ped_path_nodes = 1;
        paths.path_nodes[0].pos = CVector::ZERO;

        let next = paths.find_next_node_wandering(PATH_PED, CVector::ZERO, Some(0), 3, |_, _| true);

        assert_eq!(next, Some((0, 0)));
    }

    #[test]
    fn path_search_prefers_vehicle_treadable_over_nearer_parallel_road() {
        let mut paths = CPathFind::init();
        paths.num_map_objects = 2;
        paths.num_path_nodes = 4;
        paths.num_car_path_nodes = 4;

        let mut lower = CPathMapObject::new(0, CVector::ZERO, [0.0; 3], 1.0);
        lower.node_indices[PATH_CAR as usize][0] = 0;
        paths.map_objects[0] = Some(lower);

        let mut upper = CPathMapObject::new(1, CVector::new(0.0, 1.0, 0.0), [0.0; 3], 1.0);
        upper.node_indices[PATH_CAR as usize][0] = 2;
        paths.map_objects[1] = Some(upper);

        paths.path_nodes[0].pos = CVector::new(0.0, 0.0, 0.0);
        paths.path_nodes[0].object_index = 0;
        paths.path_nodes[0].first_link = 0;
        paths.path_nodes[0].num_links = 1;
        paths.path_nodes[0].group = 1;

        paths.path_nodes[2].pos = CVector::new(0.0, 1.0, 0.0);
        paths.path_nodes[2].object_index = 1;
        paths.path_nodes[2].first_link = 1;
        paths.path_nodes[2].num_links = 1;
        paths.path_nodes[2].group = 1;

        paths.path_nodes[3].pos = CVector::new(10.0, 0.0, 0.0);
        paths.path_nodes[3].object_index = 1;
        paths.path_nodes[3].first_link = 2;
        paths.path_nodes[3].num_links = 2;
        paths.path_nodes[3].group = 1;

        paths.connections = vec![3, 3, 0, 2];
        paths.distances = vec![10, 10, 10, 10];
        paths.connection_flags = vec![CConnectionFlags::default(); 4];
        paths.car_path_connections = vec![-1; 4];
        paths.num_connections = 4;

        let start = CVector::new(0.0, 0.0, 0.0);
        let target = paths.path_nodes[3].pos;

        let guessed = paths.do_path_search(PATH_CAR, start, None, target, 2, 999_999.9, Some(3));
        assert_eq!(guessed.nodes, vec![0, 3]);

        let from_treadable = paths.do_path_search_with_start_object(
            PATH_CAR,
            start,
            None,
            Some(1),
            target,
            2,
            999_999.9,
            Some(3),
        );
        assert_eq!(from_treadable.nodes, vec![2, 3]);
    }
}
