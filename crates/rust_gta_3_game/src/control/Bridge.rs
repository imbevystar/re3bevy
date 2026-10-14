//! re3 reference: references/re-GTA/src/control/Bridge.cpp
//! re3 reference: references/re-GTA/src/control/Bridge.h
//!
//! Callahan bridge cycle. Building discovery remains a world/pool adapter;
//! once the three model slots are supplied this resource owns the exact re3
//! state/timing and emits height/path-light changes.

use bevy::prelude::*;
use rust_gta_3_core::core::Timer::CTimer;

use crate::{
    core::{FileLoader::CFileLoader, Stats::CStats, World::CWorld},
    modelinfo::ModelInfo::CModelInfo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BridgeState {
    #[default]
    Locked = 0,
    LiftPartIsUp = 1,
    LiftPartMovingDown = 2,
    LiftPartIsDown = 3,
    LiftPartAboutToMoveUp = 4,
    LiftPartMovingUp = 5,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BridgeUpdate {
    pub state: BridgeState,
    pub heights_changed: bool,
    pub lift_part_z: f32,
    pub lift_road_z: Option<f32>,
    pub weight_z: f32,
    /// `Some(true)` closes bridge links; `Some(false)` opens them.
    pub close_path_links: Option<bool>,
}

#[derive(Debug, Clone, Resource)]
pub struct CBridge {
    pub lift_road: Option<usize>,
    pub lift_part: Option<usize>,
    pub weight: Option<usize>,
    pub state: BridgeState,
    pub old_state: BridgeState,
    pub default_z_lift_part: f32,
    pub default_z_lift_road: f32,
    pub default_z_lift_weight: f32,
    pub old_lift: f32,
    pub time_of_bridge_becoming_operational: u32,
}

impl CBridge {
    pub fn init_without_entity_lookup() -> Self {
        Self {
            lift_road: None,
            lift_part: None,
            weight: None,
            state: BridgeState::Locked,
            old_state: BridgeState::Locked,
            default_z_lift_part: 0.0,
            default_z_lift_road: 0.0,
            default_z_lift_weight: 0.0,
            old_lift: -1.0,
            time_of_bridge_becoming_operational: 0,
        }
    }

    /// `FindBridgeEntities`: resolve the three IDE model names, then scan the
    /// building pool from the last slot to the first exactly like re3.
    pub fn find_bridge_entities(&mut self, world: &CWorld, model_info: &CModelInfo) -> bool {
        let lift_model = model_info
            .get_model_info_by_name("bridge_liftsec")
            .map(|entry| entry.0);
        let road_model = model_info
            .get_model_info_by_name("subbridge_lift")
            .map(|entry| entry.0);
        let weight_model = model_info
            .get_model_info_by_name("bridge_liftweight")
            .map(|entry| entry.0);
        let mut lift_part = None;
        let mut lift_road = None;
        let mut weight = None;
        for entity_index in world.building_pool.iter().rev().copied() {
            let Some(entity) = world.entities.get(entity_index) else {
                continue;
            };
            if Some(entity.model_index) == lift_model {
                lift_part = Some((entity_index, entity.position[2]));
            } else if Some(entity.model_index) == road_model {
                lift_road = Some((entity_index, entity.position[2]));
            } else if Some(entity.model_index) == weight_model {
                weight = Some((entity_index, entity.position[2]));
            }
        }
        self.bind_entities(lift_part, lift_road, weight)
    }

    /// Result of `FindBridgeEntities`, supplied by the building-pool adapter.
    /// The initial path-link call is `SetLinksBridgeLights(..., true)`.
    pub fn bind_entities(
        &mut self,
        lift_part: Option<(usize, f32)>,
        lift_road: Option<(usize, f32)>,
        weight: Option<(usize, f32)>,
    ) -> bool {
        self.lift_part = lift_part.map(|entry| entry.0);
        self.lift_road = lift_road.map(|entry| entry.0);
        self.weight = weight.map(|entry| entry.0);
        self.old_lift = -1.0;
        if let (Some((_, part_z)), Some((_, weight_z))) = (lift_part, weight) {
            self.default_z_lift_part = part_z;
            self.default_z_lift_weight = weight_z;
            if let Some((_, road_z)) = lift_road {
                self.default_z_lift_road = road_z;
            }
            true
        } else {
            false
        }
    }

    pub fn update(
        &mut self,
        commercial_passed: bool,
        current_time_ms: u32,
    ) -> Option<BridgeUpdate> {
        if self.lift_part.is_none() || self.weight.is_none() {
            return None;
        }
        self.old_state = self.state;
        let lift_height;
        if commercial_passed {
            if self.time_of_bridge_becoming_operational == 0 {
                self.time_of_bridge_becoming_operational = current_time_ms;
            }
            // The original truncates to uint16, intentionally repeating after
            // 65535 ms rather than using a monotonic one-shot phase.
            let elapsed =
                current_time_ms.wrapping_sub(self.time_of_bridge_becoming_operational) as u16;
            if elapsed < 10_000 {
                self.state = BridgeState::LiftPartMovingDown;
                lift_height = 25.0 - elapsed as f32 / 10_000.0 * 25.0;
            } else if elapsed < 40_000 {
                self.state = BridgeState::LiftPartIsDown;
                lift_height = 0.0;
            } else if elapsed < 50_000 {
                self.state = BridgeState::LiftPartAboutToMoveUp;
                lift_height = 0.0;
            } else if elapsed < 60_000 {
                self.state = BridgeState::LiftPartMovingUp;
                lift_height = (elapsed as f32 - 50_000.0) / 10_000.0 * 25.0;
            } else {
                self.state = BridgeState::LiftPartIsUp;
                lift_height = 25.0;
            }
        } else {
            lift_height = 25.0;
            self.time_of_bridge_becoming_operational = 0;
            self.state = BridgeState::Locked;
        }

        let close_path_links = if self.state == BridgeState::LiftPartAboutToMoveUp
            && self.old_state == BridgeState::LiftPartIsDown
        {
            Some(true)
        } else if self.state == BridgeState::LiftPartIsDown
            && self.old_state == BridgeState::LiftPartMovingDown
        {
            Some(false)
        } else {
            None
        };
        let heights_changed = lift_height != self.old_lift;
        if heights_changed {
            self.old_lift = lift_height;
        }
        (heights_changed || close_path_links.is_some()).then_some(BridgeUpdate {
            state: self.state,
            heights_changed,
            lift_part_z: self.default_z_lift_part + lift_height,
            lift_road_z: self
                .lift_road
                .map(|_| self.default_z_lift_road + lift_height),
            weight_z: self.default_z_lift_weight - lift_height,
            close_path_links,
        })
    }

    pub fn should_lights_be_flashing(&self) -> bool {
        self.state != BridgeState::LiftPartIsDown
    }

    pub fn this_is_a_bridge_object_moving_up(&self, is_road_segment_or_lift: bool) -> bool {
        is_road_segment_or_lift
            && matches!(
                self.state,
                BridgeState::LiftPartAboutToMoveUp | BridgeState::LiftPartMovingUp
            )
    }
}

impl Default for CBridge {
    fn default() -> Self {
        Self::init_without_entity_lookup()
    }
}

// === FAZ8c-bridge: moved from core/Game.rs (verbatim, parity-preserving) ===

/// re3 `CBridge::Update`, after `CGameLogic::Update` and before corona/shadow
/// processing. Building pool indices remain stable because map buildings own
/// their world slots for the loaded level.
pub(crate) fn update_bridge(
    mut bridge: Option<ResMut<CBridge>>,
    stats: Option<Res<CStats>>,
    timer: Res<CTimer>,
    mut file_loader: ResMut<CFileLoader>,
) {
    let (Some(bridge), Some(stats)) = (bridge.as_mut(), stats.as_ref()) else {
        return;
    };
    // Save-load/level replacement can rebuild CWorld after Bridge::Init.
    if bridge.lift_part.is_none() || bridge.weight.is_none() {
        if bridge.find_bridge_entities(&file_loader.world, &file_loader.model_info) {
            file_loader
                .path_find
                .set_links_bridge_lights(-330.0, -230.0, -700.0, -588.0, true);
        }
    }
    let Some(update) = bridge.update(stats.commercial_passed != 0, timer.time_in_milliseconds())
    else {
        return;
    };
    if update.heights_changed {
        if let Some(index) = bridge.lift_part {
            if let Some(entity) = file_loader.world.entities.get_mut(index) {
                entity.position[2] = update.lift_part_z;
            }
        }
        if let (Some(index), Some(z)) = (bridge.lift_road, update.lift_road_z) {
            if let Some(entity) = file_loader.world.entities.get_mut(index) {
                entity.position[2] = z;
            }
        }
        if let Some(index) = bridge.weight {
            if let Some(entity) = file_loader.world.entities.get_mut(index) {
                entity.position[2] = update.weight_z;
            }
        }
    }
    if let Some(close) = update.close_path_links {
        file_loader
            .path_find
            .set_links_bridge_lights(-330.0, -230.0, -700.0, -588.0, close);
    }
}
