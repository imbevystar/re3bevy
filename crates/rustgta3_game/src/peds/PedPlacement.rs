//! re3 reference: references/re-GTA/src/peds/PedPlacement.cpp
//! re3 reference: references/re-GTA/src/peds/PedPlacement.h
//!
//! Ped ground placement and world-overlap query gates.

use rustgta3_core::core::Coordinates::CVector;

use crate::{core::World::CWorld, modelinfo::ModelInfo::CModelInfo};

pub const PED_FEET_OFFSET: f32 = 1.04;
pub const PED_PLACEMENT_LINE_DEPTH: f32 = 100.0;
pub const PED_CAR_CLEAR_RADIUS: f32 = 0.25;
pub const PED_POSITION_CLEAR_RADIUS: f32 = 0.75;
pub const PED_POSITION_MAX_COLLIDERS: i16 = 2;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CPedPlacementResult {
    pub position: CVector,
    pub found: bool,
}

pub struct CPedPlacement;

impl CPedPlacement {
    /// re3 `FindZCoorForPed`: two offset vertical rays are required so an edge
    /// hit cannot leave a ped below the higher adjacent surface.
    pub fn find_z_coor_for_ped(
        mut position: CVector,
        world: &CWorld,
        model_info: &CModelInfo,
    ) -> CPedPlacementResult {
        let start_z = position.z - PED_PLACEMENT_LINE_DEPTH;
        let first = world
            .process_vertical_line_static(
                [position.x, position.y, position.z + 1.0],
                start_z,
                model_info,
            )
            .map(|point| point.point.z)
            .unwrap_or(-100.0);
        let second = world
            .process_vertical_line_static(
                [position.x + 0.1, position.y + 0.1, position.z + 1.0],
                start_z,
                model_info,
            )
            .map(|point| point.point.z)
            .unwrap_or(-100.0);
        let ground = first.max(second);
        let found = ground > -99.0;
        if found {
            position.z = PED_FEET_OFFSET + ground;
        }
        CPedPlacementResult { position, found }
    }

    /// re3 `IsPositionClearOfCars`. The closure is the ECS equivalent of
    /// `CWorld::TestSphereAgainstWorld` with buildings+vehicles enabled and
    /// all other entity families disabled. It returns the first colliding
    /// entity's stable pool handle.
    pub fn is_position_clear_of_cars<F>(position: CVector, mut sphere_probe: F) -> Option<i32>
    where
        F: FnMut(CVector, f32) -> Option<i32>,
    {
        sphere_probe(position, PED_CAR_CLEAR_RADIUS)
    }

    /// re3 `IsPositionClearForPed`. The closure must perform the 2D-only
    /// `FindObjectsKindaColliding` query for vehicles and peds, capped at two
    /// results exactly like the C++ call.
    pub fn is_position_clear_for_ped<F>(position: CVector, mut colliding_count: F) -> bool
    where
        F: FnMut(CVector, f32, bool, i16) -> i16,
    {
        colliding_count(
            position,
            PED_POSITION_CLEAR_RADIUS,
            true,
            PED_POSITION_MAX_COLLIDERS,
        ) == 0
    }
}
