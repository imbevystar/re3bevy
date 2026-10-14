//! re3 reference: references/re-GTA/src/rw/VisibilityPlugins.cpp
//! re3 reference: references/re-GTA/src/rw/VisibilityPlugins.h
//!
//! RenderWare visibility plugin state represented without raw RW pointers.

use bevy::prelude::Resource;
use rust_gta_3_core::core::Coordinates::CVector;

pub const NUMALPHALIST: usize = 20;
pub const NUMALPHAENTITYLIST: usize = 150;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CVisibilityPlugins {
    pub alpha_list: Vec<AlphaObjectInfo>,
    pub alpha_entity_list: Vec<AlphaObjectInfo>,
    pub alpha_building_list: Vec<AlphaObjectInfo>,
    pub camera_position: CVector,
    pub cull_comps_dist: f32,
    pub vehicle_lod0_dist: f32,
    pub vehicle_lod1_dist: f32,
    pub vehicle_fade_dist: f32,
    pub big_vehicle_lod0_dist: f32,
    pub big_vehicle_lod1_dist: f32,
    pub ped_lod0_dist: f32,
    pub ped_lod1_dist: f32,
    pub ped_fade_dist: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlphaObjectInfo {
    pub object_id: usize,
    pub sort: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AtomicExt {
    pub model_info_id: Option<usize>,
    pub flags: i32,
    pub atomic_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameExt {
    pub id: isize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClumpExt {
    pub model_info_id: Option<usize>,
    pub alpha: i32,
    pub visibility: ClumpVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClumpVisibility {
    #[default]
    Default,
    FrustumSphere,
    Mlo,
    Vehicle,
    BigVehicle,
}

impl CVisibilityPlugins {
    pub fn initialise(&mut self) {
        self.alpha_list.clear();
        self.alpha_entity_list.clear();
        self.alpha_building_list.clear();
        self.alpha_list.reserve(NUMALPHALIST);
        self.alpha_entity_list.reserve(NUMALPHAENTITYLIST);
        self.alpha_building_list.reserve(NUMALPHAENTITYLIST);
    }

    pub fn shutdown(&mut self) {
        self.alpha_list.clear();
        self.alpha_entity_list.clear();
        self.alpha_building_list.clear();
    }

    pub fn init_alpha_entity_list(&mut self) {
        self.alpha_entity_list.clear();
        self.alpha_building_list.clear();
    }

    pub fn insert_entity_into_sorted_list(&mut self, entity_id: usize, dist: f32) -> bool {
        insert_sorted(
            &mut self.alpha_entity_list,
            NUMALPHAENTITYLIST,
            AlphaObjectInfo {
                object_id: entity_id,
                sort: dist,
            },
        )
    }

    pub fn init_alpha_atomic_list(&mut self) {
        self.alpha_list.clear();
    }

    pub fn insert_atomic_into_sorted_list(&mut self, atomic_id: usize, dist: f32) -> bool {
        insert_sorted(
            &mut self.alpha_list,
            NUMALPHALIST,
            AlphaObjectInfo {
                object_id: atomic_id,
                sort: dist,
            },
        )
    }

    pub fn set_renderware_camera(
        &mut self,
        camera_position: CVector,
        lod_dist_multiplier: f32,
        generation_dist_multiplier: f32,
        top_down: bool,
    ) {
        self.camera_position = camera_position;
        self.cull_comps_dist = if top_down {
            1_000_000.0
        } else {
            (lod_dist_multiplier * 20.0).powi(2)
        };
        self.vehicle_lod0_dist = (70.0 * generation_dist_multiplier).powi(2);
        self.vehicle_lod1_dist = (90.0 * generation_dist_multiplier).powi(2);
        self.vehicle_fade_dist = (100.0 * generation_dist_multiplier).powi(2);
        self.big_vehicle_lod0_dist = (60.0 * generation_dist_multiplier).powi(2);
        self.big_vehicle_lod1_dist = (150.0 * generation_dist_multiplier).powi(2);
        self.ped_lod0_dist = (25.0 * lod_dist_multiplier).powi(2);
        self.ped_lod1_dist = (60.0 * lod_dist_multiplier).powi(2);
        self.ped_fade_dist = (70.0 * lod_dist_multiplier).powi(2);
    }
}

impl Default for CVisibilityPlugins {
    fn default() -> Self {
        let mut plugins = Self {
            alpha_list: Vec::new(),
            alpha_entity_list: Vec::new(),
            alpha_building_list: Vec::new(),
            camera_position: CVector::ZERO,
            cull_comps_dist: 0.0,
            vehicle_lod0_dist: 0.0,
            vehicle_lod1_dist: 0.0,
            vehicle_fade_dist: 0.0,
            big_vehicle_lod0_dist: 0.0,
            big_vehicle_lod1_dist: 0.0,
            ped_lod0_dist: 0.0,
            ped_lod1_dist: 0.0,
            ped_fade_dist: 0.0,
        };
        plugins.initialise();
        plugins
    }
}

pub fn set_alpha_cb(alpha: u8) -> u8 {
    alpha
}

fn insert_sorted(list: &mut Vec<AlphaObjectInfo>, capacity: usize, item: AlphaObjectInfo) -> bool {
    if list.len() >= capacity {
        return false;
    }
    let index = list.partition_point(|existing| existing.sort <= item.sort);
    list.insert(index, item);
    true
}
