//! re3 reference: references/re-GTA/src/collision/Collision.cpp
//! re3 reference: references/re-GTA/src/collision/Collision.h
//!
//! `CCollision::Init` state.

use crate::entities::EntityReference::EntityReference;
use bevy::prelude::Resource;
use re3bevy_core::core::Coordinates::CVector;
use re3bevy_core::core::GameState::{CGameState, ELevelName};

use crate::animation::CutsceneMgr::CCutsceneMgr;
use crate::collision::ColModel::{
    CColBox, CColLine, CColModel, CColPoint, CColSphere, CColTriangle, CColTrianglePlane,
    CColVector, CompressedVector, dist_to_line, dist_to_line_with_closest, process_line_box,
    process_line_sphere, process_line_triangle, process_sphere_box, process_sphere_sphere,
    process_sphere_triangle, test_line_box, test_line_sphere, test_sphere_box, test_sphere_sphere,
    test_sphere_triangle, test_vertical_line_box,
};
use crate::control::Replay::CReplay;
use crate::core::{Game::CGameBeforeRw, Streaming::CStreaming};
use crate::math::Matrix::CMatrix;
use crate::objects::Object::CollisionSpecialResponseCase;
use crate::entities::Physical::CPhysical;

pub const NUMCOLCACHELINKS: usize = 200;
pub const MAX_COLLISION_POINTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CStoredCollPoly {
    pub vertices: [CColVector; 3],
    pub valid: bool,
}

impl Default for CStoredCollPoly {
    fn default() -> Self {
        Self {
            vertices: [CColVector::new(0.0, 0.0, 0.0); 3],
            valid: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionUpdateContext {
    pub player_position: CColVector,
    pub zone_level: ELevelName,
    pub current_level: ELevelName,
    pub time_in_milliseconds: u32,
    pub cutscene_processing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionUpdateResult {
    pub current_level: ELevelName,
    pub force_level_change: bool,
    pub load_collision: bool,
    pub check_big_buildings: bool,
}

#[derive(Debug, Clone, Resource)]
pub struct CCollision {
    pub collision_in_memory: ELevelName,
    pub waiting_player_frames: u64,
    pub col_model_cache_capacity: usize,
    pub last_load_error: Option<String>,
}

impl CCollision {
    pub fn init() -> Self {
        Self {
            collision_in_memory: ELevelName::Generic,
            waiting_player_frames: 0,
            col_model_cache_capacity: NUMCOLCACHELINKS,
            last_load_error: None,
        }
    }

    /// Level-selection half of re3 `CCollision::Update`. The caller consumes
    /// the load flag through the streaming/model-info owners in schedule order.
    pub fn update(&self, context: CollisionUpdateContext) -> CollisionUpdateResult {
        if context.time_in_milliseconds < 2_000 || context.cutscene_processing {
            return CollisionUpdateResult {
                current_level: context.current_level,
                force_level_change: false,
                load_collision: false,
                check_big_buildings: false,
            };
        }

        let mut level = context.zone_level;
        let mut force_level_change = false;
        if level == ELevelName::Generic {
            if context.current_level == ELevelName::Industrial && context.player_position.x < 400.0
            {
                level = ELevelName::Commercial;
                force_level_change = true;
            } else if context.current_level == ELevelName::Suburban
                && context.player_position.x > -450.0
                && context.player_position.y < -1_400.0
            {
                level = ELevelName::Commercial;
                force_level_change = true;
            } else if context.player_position.x > 800.0 {
                level = ELevelName::Industrial;
                force_level_change = true;
            } else if context.player_position.x < -800.0 {
                level = ELevelName::Suburban;
                force_level_change = true;
            }
        }

        let current_level = if level != ELevelName::Generic && level != context.current_level {
            level
        } else {
            context.current_level
        };
        CollisionUpdateResult {
            current_level,
            force_level_change,
            load_collision: self.collision_in_memory != current_level,
            check_big_buildings: true,
        }
    }

    pub fn test_sphere_sphere(first: &CColSphere, second: &CColSphere) -> bool {
        test_sphere_sphere(first, second)
    }

    pub fn test_sphere_box(sphere: &CColSphere, box_: &CColBox) -> bool {
        test_sphere_box(sphere, box_)
    }

    pub fn test_line_box(line: &CColLine, box_: &CColBox) -> bool {
        test_line_box(line, box_)
    }

    pub fn test_vertical_line_box(line: &CColLine, box_: &CColBox) -> bool {
        test_vertical_line_box(line, box_)
    }

    pub fn test_line_sphere(line: &CColLine, sphere: &CColSphere) -> bool {
        test_line_sphere(line, sphere)
    }

    pub fn test_sphere_triangle(
        sphere: &CColSphere,
        vertices: &[CompressedVector],
        triangle: &CColTriangle,
        plane: &CColTrianglePlane,
    ) -> bool {
        test_sphere_triangle(sphere, vertices, triangle, plane)
    }

    pub fn process_sphere_sphere(
        first: &CColSphere,
        second: &CColSphere,
        point: &mut CColPoint,
        mindistsq: &mut f32,
    ) -> bool {
        process_sphere_sphere(first, second, point, mindistsq)
    }

    pub fn process_sphere_box(
        sphere: &CColSphere,
        box_: &CColBox,
        point: &mut CColPoint,
        mindistsq: &mut f32,
    ) -> bool {
        process_sphere_box(sphere, box_, point, mindistsq)
    }

    pub fn process_line_box(
        line: &CColLine,
        box_: &CColBox,
        point: &mut CColPoint,
        mindist: &mut f32,
    ) -> bool {
        process_line_box(line, box_, point, mindist)
    }

    pub fn process_line_sphere(
        line: &CColLine,
        sphere: &CColSphere,
        point: &mut CColPoint,
        mindist: &mut f32,
    ) -> bool {
        process_line_sphere(line, sphere, point, mindist)
    }

    pub fn process_line_triangle(
        line: &CColLine,
        vertices: &[CompressedVector],
        triangle: &CColTriangle,
        plane: &CColTrianglePlane,
        point: &mut CColPoint,
        mindist: &mut f32,
    ) -> bool {
        process_line_triangle(line, vertices, triangle, plane, point, mindist)
    }

    pub fn process_sphere_triangle(
        sphere: &CColSphere,
        vertices: &[CompressedVector],
        triangle: &CColTriangle,
        plane: &CColTrianglePlane,
        point: &mut CColPoint,
        mindistsq: &mut f32,
    ) -> bool {
        process_sphere_triangle(sphere, vertices, triangle, plane, point, mindistsq)
    }

    /// Sphere-contact half of re3 `CCollision::ProcessColModels`.
    ///
    /// The original routine treats model A's spheres as the moving contact
    /// probes and tests them in model B space against B's spheres, boxes and
    /// triangles. One closest `CColPoint` is emitted per colliding A sphere,
    /// then transformed back to world space. Vehicle body piece ids therefore
    /// survive all the way to `CPhysical::SetDamagedPieceRecord`.
    pub fn process_col_models_spheres(
        matrix_a: &CMatrix,
        model_a: &CColModel,
        matrix_b: &CMatrix,
        model_b: &CColModel,
    ) -> Vec<CColPoint> {
        if !col_bound_spheres_overlap(matrix_a, model_a, matrix_b, model_b) {
            return Vec::new();
        }

        let matrix_ab = matrix_b.invert().multiply(matrix_a);
        let bound_center = matrix_ab.transform_point(col_to_game(model_a.bounding_sphere.center));
        let bound_sphere = CColSphere::new(
            model_a.bounding_sphere.radius,
            game_to_col(bound_center),
            model_a.bounding_sphere.surface,
            model_a.bounding_sphere.piece,
        );
        if !test_sphere_box(&bound_sphere, &model_b.bounding_box) {
            return Vec::new();
        }

        let mut contacts = Vec::with_capacity(model_a.spheres.len().min(MAX_COLLISION_POINTS));
        for sphere_a in &model_a.spheres {
            if contacts.len() == MAX_COLLISION_POINTS {
                break;
            }
            let center_b = matrix_ab.transform_point(col_to_game(sphere_a.center));
            let transformed_a = CColSphere::new(
                sphere_a.radius,
                game_to_col(center_b),
                sphere_a.surface,
                sphere_a.piece,
            );
            if !test_sphere_box(&transformed_a, &model_b.bounding_box) {
                continue;
            }

            let mut point = CColPoint::new();
            let mut distance_sq = 1.0e24_f32;
            let mut collided = false;
            for sphere_b in &model_b.spheres {
                collided |=
                    process_sphere_sphere(&transformed_a, sphere_b, &mut point, &mut distance_sq);
            }
            for box_b in &model_b.boxes {
                collided |= process_sphere_box(&transformed_a, box_b, &mut point, &mut distance_sq);
            }
            for (triangle_index, triangle_b) in model_b.triangles.iter().enumerate() {
                if !model_b.triangle_maybe_overlaps_sphere(
                    triangle_index,
                    transformed_a.center,
                    transformed_a.radius,
                ) {
                    continue;
                }
                let Some(plane) = model_b.triangle_plane(triangle_index) else {
                    continue;
                };
                collided |= process_sphere_triangle(
                    &transformed_a,
                    &model_b.vertices,
                    triangle_b,
                    &plane,
                    &mut point,
                    &mut distance_sq,
                );
            }
            if !collided {
                continue;
            }

            let world_point = matrix_b.transform_point(col_to_game(point.point));
            let world_normal = matrix_b.multiply3x3(col_to_game(point.normal));
            point.point = game_to_col(world_point);
            point.normal = game_to_col(world_normal);
            contacts.push(point);
        }
        contacts
    }

    pub fn test_col_models_spheres(
        matrix_a: &CMatrix,
        model_a: &CColModel,
        matrix_b: &CMatrix,
        model_b: &CColModel,
    ) -> bool {
        if !col_bound_spheres_overlap(matrix_a, model_a, matrix_b, model_b) {
            return false;
        }

        let matrix_ab = matrix_b.invert().multiply(matrix_a);
        let bound_center = matrix_ab.transform_point(col_to_game(model_a.bounding_sphere.center));
        let bound_sphere = CColSphere::new(
            model_a.bounding_sphere.radius,
            game_to_col(bound_center),
            model_a.bounding_sphere.surface,
            model_a.bounding_sphere.piece,
        );
        if !test_sphere_box(&bound_sphere, &model_b.bounding_box) {
            return false;
        }

        for sphere_a in &model_a.spheres {
            let center_b = matrix_ab.transform_point(col_to_game(sphere_a.center));
            let transformed_a = CColSphere::new(
                sphere_a.radius,
                game_to_col(center_b),
                sphere_a.surface,
                sphere_a.piece,
            );
            if !test_sphere_box(&transformed_a, &model_b.bounding_box) {
                continue;
            }

            for sphere_b in &model_b.spheres {
                if test_sphere_sphere(&transformed_a, sphere_b) {
                    return true;
                }
            }
            for box_b in &model_b.boxes {
                if test_sphere_box(&transformed_a, box_b) {
                    return true;
                }
            }
            for (triangle_index, triangle_b) in model_b.triangles.iter().enumerate() {
                if !model_b.triangle_maybe_overlaps_sphere(
                    triangle_index,
                    transformed_a.center,
                    transformed_a.radius,
                ) {
                    continue;
                }
                let Some(plane) = model_b.triangle_plane(triangle_index) else {
                    continue;
                };
                if test_sphere_triangle(&transformed_a, &model_b.vertices, triangle_b, &plane) {
                    return true;
                }
            }
        }
        false
    }

    pub fn dist_to_line(start: CColVector, end: CColVector, point: CColVector) -> f32 {
        dist_to_line(start, end, point)
    }

    pub fn dist_to_line_with_closest(
        start: CColVector,
        end: CColVector,
        point: CColVector,
    ) -> (f32, CColVector) {
        dist_to_line_with_closest(start, end, point)
    }
}

fn col_to_game(value: CColVector) -> CVector {
    CVector::new(value.x, value.y, value.z)
}

fn game_to_col(value: CVector) -> CColVector {
    CColVector::new(value.x, value.y, value.z)
}

impl Default for CCollision {
    fn default() -> Self {
        Self::init()
    }
}

// === FAZ6: moved from core/Game.rs (verbatim, parity-preserving) ===
// Script-entity collisions, player dynamic/static resolvers and the contact
// probes they share. Player ground/jump helpers live in peds/PlayerPed.rs.
// Schedule stays in core/Game.rs.

#[allow(unused_imports)]
#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::control::AutoPilot::{eCarMission, eCarTempAction};
#[allow(unused_imports)]
use crate::control::Darkel::CDarkel;
#[allow(unused_imports)]
use crate::core::Camera::CCamera;
#[allow(unused_imports)]
use crate::core::EventList::CEventList;
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Game::{
    GameplayAuditTimer, ScriptEntityWorld, ScriptGameplayBridge, ScriptObjectStore,
    ScriptPedEntity, ScriptVehicleEntity,
};
#[allow(unused_imports)]
use crate::core::Pad::CPads;
#[allow(unused_imports)]
use crate::core::PlayerInfo::CPlayerInfo;
#[allow(unused_imports)]
use crate::core::Pools::{CPoolHandle, CPools};
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::entities::Physical::{
    CPhysicalCollisionOptions, CPhysicalStaticObjectCollisionOptions, GTA_PHYSICS_HZ,
};
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::{MI_PLAYER, MI_TRAIN};
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::objects::Object::emit_object_damage_effects;
#[allow(unused_imports)]
use crate::objects::Object::{CObject, CObjectDamageContext, CObjectGlobalState};
#[allow(unused_imports)]
use crate::peds::Ped::{
    CPed, PedCarCollisionInput, PedCarCollisionResult, ped_seat_world_position, player_seat_index,
};
#[allow(unused_imports)]
use crate::peds::PedType::CPedStatsRuntime;
#[allow(unused_imports)]
use crate::peds::PlayerPed::CWorldPlayers;
#[allow(unused_imports)]
use crate::peds::PlayerPed::{
    PlayerPhysicalSurfaceMotion, apply_player_ped_air_resistance,
    apply_player_standing_update_position, begin_player_collision_frame,
    finish_player_collision_frame, player_can_start_jump_from_surface,
    player_check_if_in_air_has_ground, player_ground_support_extension,
    player_static_ground_support, player_subdivided_dynamic_landing_support,
    should_start_player_run_stop, update_player_current_phys_surface,
    with_player_native_collision_speed,
};
#[allow(unused_imports)]
use crate::peds::Population::CPopulation;
#[allow(unused_imports)]
use crate::renderer::Particle::CParticle;
#[allow(unused_imports)]
use crate::vehicles::Automobile::CAutomobile;
#[allow(unused_imports)]
use crate::vehicles::Automobile::automobile_static_collision_step;
#[allow(unused_imports)]
use crate::vehicles::Automobile::automobile_static_contacts_for_matrix;
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::CVehicle;
#[allow(unused_imports)]
use crate::weapons::Explosion::CExplosionPool;
#[allow(unused_imports)]
use bevy::ecs::system::SystemParam;
#[allow(unused_imports)]
use bevy::prelude::*;
#[allow(unused_imports)]
use re3bevy_assets::assets::WorldAssets::CSurfaceTableAsset;
#[allow(unused_imports)]
use re3bevy_core::core::{General::CGeneral, Timer::CTimer};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub(crate) struct CollisionScratch {
    handles: Vec<i32>,
    broad_phase: Vec<(i32, f32, f32, f32)>,
    object_broad_phase: Vec<(i32, f32, f32, f32)>,
    moving_object_broad_phase: Vec<(i32, f32, f32, f32)>,
    player_object_broad_phase: Vec<(i32, f32, f32, f32)>,
    ped_broad_phase: Vec<(i32, f32, f32, f32)>,
    dynamic_collision_rollbacks: HashSet<i32>,
    vehicle_handle_by_slot: HashMap<usize, i32>,
    pair_sources: Vec<Entity>,
    pair_candidates: Vec<Entity>,
    pair_refresh: Vec<Entity>,
    pair_visited: HashSet<(Entity, Entity)>,
    pair_queued: HashSet<Entity>,
}

#[derive(SystemParam)]
pub(crate) struct ScriptCollisionResources<'w> {
    camera: Option<Res<'w, CCamera>>,
    particle_systems: Option<Res<'w, crate::renderer::ParticleMgr::cParticleSystemMgr>>,
    collision: Res<'w, CCollision>,
    file_loader: Res<'w, CFileLoader>,
    handling_manager: Res<'w, cHandlingDataMgr>,
    ped_stats: Res<'w, CPedStatsRuntime>,

    surface_table: Res<'w, CSurfaceTableAsset>,
    timer: Res<'w, CTimer>,
    darkel: Option<Res<'w, CDarkel>>,
}

/// Vehicle/vehicle branch of re3 `CPhysical::ProcessShiftSectorList`.
///
/// The generic deepest-contact 0.2/0.4 term is deliberately absent here:
/// native code replaces it with the centre-direction half-depth term when
/// both entities are vehicles. The caller invokes this once for A and then
/// recomputes the reversed B/A manifold, matching the sequential world pass.
pub(crate) fn apply_vehicle_pair_process_shift<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    other: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    contacts: &[CColPoint],
    second_shift: bool,
) -> bool
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    if contacts.is_empty() {
        return false;
    }
    let manifold_scale = if second_shift { 1.5 } else { 1.2 } / contacts.len() as f32;
    let mut shift = CVector::ZERO;
    for point in contacts {
        let amount = point.depth * manifold_scale;
        shift.x += point.normal.x * amount;
        shift.y += point.normal.y * amount;
        shift.z += point.normal.z * amount;
    }

    if let Some(deepest) = contacts
        .iter()
        .max_by(|left, right| left.depth.total_cmp(&right.depth))
    {
        let position = vehicle.physical.entity.get_position();
        let other_position = other.physical.entity.get_position();
        let mut direction = CVector::new(
            position.x - other_position.x,
            position.y - other_position.y,
            position.z - other_position.z,
        );
        let magnitude = direction.magnitude();
        if magnitude > 0.0 {
            direction.x /= magnitude;
            direction.y /= magnitude;
            direction.z /= magnitude;
            let forward_z = vehicle.physical.entity.get_forward().z;
            let right_z = vehicle.physical.entity.get_right().z;
            if direction.z < 0.0 && direction.z < forward_z && direction.z < right_z {
                direction.z = 0.0_f32.min(forward_z.min(right_z));
            }
            let amount = deepest.depth * 0.5;
            shift.x += direction.x * amount;
            shift.y += direction.y * amount;
            shift.z += direction.z * amount;
        }
    }
    vehicle.physical.apply_process_shift(shift)
}

/// CPhysical::ProcessShift accepts a shifted pose when the follow-up
/// collision pass finds contacts but none of them produces an impulse. Sphere
/// manifolds can therefore remain microscopically overlapped while two cars
/// are separating or sliding past each other. Requiring an empty manifold
/// over-corrects those resting contacts and makes the pair oscillate.
fn vehicle_pair_contacts_require_response<VehCore, VehVitality, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    vehicle: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    other: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    contacts: &[CColPoint],
    time_step: f32,
) -> bool
where
    VehCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    if contacts.is_empty() {
        return false;
    }

    // Probe on clones because native ProcessShift runs ProcessCollisionSectorList
    // after the translation. We only need its boolean result here; the actual
    // pair response has already been applied by the owning collision pass.
    let mut vehicle_probe = vehicle.physical.clone();
    let mut other_probe = other.physical.clone();
    contacts.iter().any(|point| {
        vehicle_probe
            .apply_collision_dynamic(
                &mut other_probe,
                point,
                CPhysicalCollisionOptions {
                    no_more_collision_torque: true,
                    time_step,
                    ..Default::default()
                },
            )
            .applied
    })
}

#[cfg(test)]
mod vehicle_shift_response_tests {
    use super::*;

    #[test]
    fn resting_overlap_does_not_request_another_vehicle_shift() {
        let mut general = CGeneral::with_seed(1);
        let mut vehicle = CVehicle::new(0, 0, &mut general);
        let mut other = CVehicle::new(0, 0, &mut general);
        vehicle.physical.mass = 1_000.0;
        vehicle.physical.turn_mass = 2_500.0;
        other.physical.mass = 1_000.0;
        other.physical.turn_mass = 2_500.0;
        let mut contact = CColPoint::new();
        contact.point = CColVector::new(0.0, 0.0, 0.0);
        contact.normal = CColVector::new(1.0, 0.0, 0.0);
        contact.depth = 0.01;

        assert!(!vehicle_pair_contacts_require_response(
            &vehicle,
            &other,
            &[contact],
            1.0,
        ));
    }

    #[test]
    fn closing_overlap_still_requests_vehicle_collision_response() {
        let mut general = CGeneral::with_seed(2);
        let mut vehicle = CVehicle::new(0, 0, &mut general);
        let mut other = CVehicle::new(0, 0, &mut general);
        vehicle.physical.mass = 1_000.0;
        vehicle.physical.turn_mass = 2_500.0;
        other.physical.mass = 1_000.0;
        other.physical.turn_mass = 2_500.0;
        vehicle.physical.move_speed = CVector::new(-0.1, 0.0, 0.0);
        let mut contact = CColPoint::new();
        contact.point = CColVector::new(0.0, 0.0, 0.0);
        contact.normal = CColVector::new(1.0, 0.0, 0.0);
        contact.depth = 0.01;

        assert!(vehicle_pair_contacts_require_response(
            &vehicle,
            &other,
            &[contact],
            1.0,
        ));
    }
}

/// `CPed::ProcessEntityCollision` records geometry before `ApplyCollision`.
/// Resting or separating contacts still belong to both actors' contact history.
fn record_ped_manifold_contact(
    ped: &mut CPhysical,
    other: &mut CPhysical,
    contacts: &[CColPoint],
    ped_reference: EntityReference,
    other_reference: EntityReference,
    time_ms: u32,
) -> bool {
    if contacts.is_empty() {
        return false;
    }
    ped.add_collision_record(other_reference, time_ms);
    other.add_collision_record(ped_reference, time_ms);
    if other.entity.flags.is_static {
        ped.entity.flags.has_hit_wall = true;
    }
    true
}

#[cfg(test)]
mod ped_contact_record_tests {
    use super::*;
    use crate::peds::{PedLocomotion::PedLocomotion, PedType::ePedType, PedVitality::PedVitality};

    const PED_HANDLE: i32 = (3 << 8) | 1;
    const OTHER_HANDLE: i32 = (2 << 8) | 1;
    const TIME_MS: u32 = 750;

    fn resting_contact() -> CColPoint {
        let mut point = CColPoint::new();
        point.normal = CColVector::new(1.0, 0.0, 0.0);
        point.depth = 0.01;
        point
    }

    fn surfaces() -> CSurfaceTableAsset {
        CSurfaceTableAsset {
            filename: "unit-test-no-response".into(),
            data: re3bevy_formats::formats::DataRead::SurfaceDat::parse(
                "Rubber 1\nHard 1 1\nRoad 1 1 1\nLoose 1 1 1 1\nWet 1 1 1 1 1"
            ).expect("complete unit-adhesion test table"),
        }
    }

    fn assert_resting_records(ped: &CPed, other: &CPhysical, other_reference: EntityReference) {
        assert!(ped.physical.get_has_collided_with(other_reference));
        assert!(other.get_has_collided_with(EntityReference::Ped(PED_HANDLE)));
        assert_eq!(ped.physical.last_time_collided, TIME_MS);
        assert_eq!(other.last_time_collided, TIME_MS);
        assert_eq!(ped.physical.move_speed, CVector::ZERO);
        assert_eq!(other.move_speed, CVector::ZERO);
        assert_eq!(ped.physical.damage.impulse, 0.0);
    }

    #[test]
    fn resting_vehicle_contact_records_without_impulse_or_damage() {
        for is_static in [false, true] {
            let mut general = CGeneral::with_seed(11);
            let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
            let mut vehicle = CVehicle::new(0, 0, &mut general);
            vehicle.physical.entity.flags.use_collision_records = true;
            vehicle.physical.entity.flags.is_static = is_static;
            vehicle.physical.mass = 1_000.0;
            vehicle.physical.turn_mass = 2_500.0;
            let mut vitality = PedVitality::default();
            let health_before = vitality.health;
            let result = vehicle_ped_collision_step(
                &mut vehicle, None, &mut ped, &PedLocomotion::default(), &mut vitality,
                CVector::ZERO, false, true, &[resting_contact()],
                OTHER_HANDLE, PED_HANDLE,
                CVector::ZERO, CVector::ZERO, false, false, false, 1.0,
                1.0, TIME_MS, 0, &mut general,
            );
            assert_eq!(result, (false, None));
            assert_resting_records(&ped, &vehicle.physical, EntityReference::Vehicle(OTHER_HANDLE));
            assert_eq!(ped.physical.entity.flags.has_hit_wall, is_static);
            assert_eq!(vitality.health, health_before);
            assert_eq!(ped.physical.entity.get_position(), CVector::ZERO);
        }
    }

    #[test]
    fn resting_ped_contact_records_without_response() {
        let mut general = CGeneral::with_seed(12);
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut other = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        assert!(!player_ped_collision_step(
            &mut ped, &mut other, false, &[resting_contact()], PED_HANDLE,
            OTHER_HANDLE, &surfaces(), 1.0, TIME_MS,
        ));
        assert_resting_records(&ped, &other.physical, EntityReference::Ped(OTHER_HANDLE));
        assert!(!ped.physical.entity.flags.has_hit_wall);
    }

    #[test]
    fn resting_object_contact_records_and_preserves_static_wall_classification() {
        for is_static in [false, true] {
            let mut general = CGeneral::with_seed(13);
            let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
            let mut object = CObject::new(&mut general);
            object.physical.entity.flags.use_collision_records = true;
            object.physical.entity.flags.is_static = is_static;
            object.uproot_limit = if is_static { 10.0 } else { 0.0 };
            assert_eq!(player_object_collision_step(
                &mut ped, &mut object, Entity::PLACEHOLDER, false,
                &[resting_contact()], PED_HANDLE, OTHER_HANDLE, &surfaces(), 1.0, TIME_MS,
            ), None);
            assert_resting_records(&ped, &object.physical, EntityReference::Object(OTHER_HANDLE));
            assert_eq!(ped.physical.entity.flags.has_hit_wall, is_static);
        }
    }

    #[test]
    fn empty_manifold_does_not_create_contact_or_wall_flags() {
        let mut general = CGeneral::with_seed(14);
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut other = CPhysical::new(&mut general);
        other.entity.flags.is_static = true;
        assert!(!record_ped_manifold_contact(
            &mut ped.physical, &mut other, &[], EntityReference::Ped(PED_HANDLE), EntityReference::Object(OTHER_HANDLE), TIME_MS,
        ));
        assert!(ped.physical.collision_records.is_empty());
        assert!(!ped.physical.entity.flags.has_collided);
        assert!(!ped.physical.entity.flags.has_hit_wall);
        assert!(!other.entity.flags.has_collided);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn vehicle_ped_collision_step<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    collision_matrix: Option<CMatrix>,
    ped: &mut CPed, locomotion: &crate::peds::PedLocomotion::PedLocomotion, vitality: &mut crate::peds::PedVitality::PedVitality,
    ped_collision_position: CVector,
    player_pose_was_rolled_back: bool,
    ped_is_collision_owner: bool,
    contacts: &[CColPoint],
    vehicle_handle: i32,
    ped_handle: i32,
    vehicle_col_min: CVector,
    vehicle_col_max: CVector,
    current_surface_is_this_vehicle: bool,
    current_surface_is_vehicle: bool,
    current_surface_is_boat: bool,
    defend_weakness: f32,

    time_step: f32,
    time_ms: u32,
    frame_counter: u32,
    general: &mut CGeneral,
) -> (bool, Option<PedCarCollisionResult>)
where VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
      VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    if !record_ped_manifold_contact(
        &mut ped.physical,
        &mut vehicle.physical,
        contacts,
        EntityReference::Ped(ped_handle),
        EntityReference::Vehicle(vehicle_handle),
        time_ms,
    ) {
        return (false, None);
    }
    let vehicle_is_player = vehicle.physical.entity.status == eEntityStatus::STATUS_PLAYER;
    let vehicle_is_train = vehicle.physical.entity.model_index == MI_TRAIN as i16;
    let vehicle_position = vehicle.physical.entity.get_position();
    let vehicle_right = vehicle.physical.entity.get_right();
    let vehicle_forward = vehicle.physical.entity.get_forward();
    let vehicle_up = vehicle.physical.entity.get_up();
    let vehicle_move_speed = vehicle.physical.move_speed;
    let vehicle_mass = vehicle.physical.mass;
    let ped_has_hit_wall = ped.physical.entity.flags.has_hit_wall;
    let vehicle_to_ped = {
        let delta = CVector::new(
            ped_collision_position.x - vehicle_position.x,
            ped_collision_position.y - vehicle_position.y,
            0.0,
        );
        let length = (delta.x * delta.x + delta.y * delta.y).sqrt();
        if length > 1.0e-6 {
            CVector::new(delta.x / length, delta.y / length, 0.0)
        } else {
            CVector::ZERO
        }
    };
    let player_vehicle_speed_dot =
        vehicle_to_ped.x * vehicle_move_speed.x + vehicle_to_ped.y * vehicle_move_speed.y;
    let player_vehicle_process_hit = ped.is_player()
        && vehicle_move_speed.magnitude_sqr() > 0.0025
        && player_vehicle_speed_dot > 0.1;
    let player_ped_against_abandoned_or_wrecked_vehicle = ped.is_player()
        && matches!(
            vehicle.physical.entity.status,
            eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED
        );
    let mut strongest_ped_impulse = 0.0_f32;
    let mut strongest_normal = CVector::ZERO;
    for point in contacts {
        let response = if ped_is_collision_owner {
            ped.physical.apply_collision_dynamic(
                &mut vehicle.physical,
                point,
                CPhysicalCollisionOptions {
                    time_step,
                    player_ped_against_abandoned_or_wrecked_vehicle,
                    b_is_current_physical_surface_for_a: current_surface_is_this_vehicle,
                    ..Default::default()
                },
            )
        } else {
            vehicle.physical.apply_collision_dynamic(
                &mut ped.physical,
                point,
                CPhysicalCollisionOptions {
                    time_step,
                    player_ped_against_abandoned_or_wrecked_vehicle,
                    a_is_current_physical_surface_for_b: current_surface_is_this_vehicle,
                    ..Default::default()
                },
            )
        };
        if !response.applied {
            continue;
        }
        let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
        let ped_impulse = if ped_is_collision_owner {
            response.impulse_a
        } else {
            response.impulse_b
        };
        if ped_impulse > strongest_ped_impulse {
            strongest_ped_impulse = ped_impulse;
            strongest_normal = normal;
        }
    }
    if strongest_ped_impulse <= 0.0 {
        return (false, None);
    }

    // This response corresponds to the early-return collision path in
    // `CPhysical::ProcessCollision`; the integrated displacement is rejected
    // even though ProcessShift may subsequently separate the pair.
    ped.physical.distance_travelled = 0.0;
    if contacts.iter().any(|point| {
        point.surface_a == crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8
    }) {
        ped.flags.hit_steep_slope = true;
    }

    if !ped_is_collision_owner && let Some(matrix) = collision_matrix {
        vehicle.physical.entity.set_matrix(matrix);
    }
    // A subdivided player collision is probed at the colliding pose, but
    // `CPhysical::ProcessCollision` restores its saved matrix immediately on
    // a responding collision. CWorld reaches ProcessShift only after the
    // collision retries fail. If another physical moved into the player's
    // already-live pose there was no earlier rollback, so that case still
    // needs immediate separation in this ECS collision stage.
    if !player_pose_was_rolled_back {
        let contact_count = contacts.len() as f32;
        let mut ped_shift = CVector::ZERO;
        let normal_scale = if ped_is_collision_owner { 1.0 } else { -1.0 };
        for point in contacts {
            let amount = point.depth * 1.2 / contact_count;
            ped_shift.x += point.normal.x * amount * normal_scale;
            ped_shift.y += point.normal.y * amount * normal_scale;
            ped_shift.z += point.normal.z * amount * normal_scale;
        }
        if let Some(deepest) = contacts
            .iter()
            .max_by(|left, right| left.depth.total_cmp(&right.depth))
        {
            ped_shift.x += deepest.normal.x * deepest.depth * 0.2 * normal_scale;
            ped_shift.y += deepest.normal.y * deepest.depth * 0.2 * normal_scale;
            ped_shift.z += deepest.normal.z * deepest.depth * 0.2 * normal_scale;
        }
        ped.physical.apply_process_shift(ped_shift);
        ped.physical.entity.flags.is_in_safe_position = true;
    }
    ped.physical.record_collision_damage(
        0,
        strongest_ped_impulse,
        Some(EntityReference::Vehicle(vehicle_handle)),
        strongest_normal,
    );

    // ProcessCollisionSectorList only calls KillPedWithCar for the player
    // after the ped has hit a wall and the vehicle still carries speed.
    // In GTA III, m_vecMoveSpeed is per 50Hz tick; 0.05 corresponds to 2.5 m/s.
    // Vehicle CPhysical velocities use those same GTA step units.
    let min_car_kill_speed = 0.05;
    let vehicle_moving =
        vehicle_move_speed.magnitude_sqr() > min_car_kill_speed * min_car_kill_speed;
    if (!ped.is_player() && vehicle_moving)
        || player_vehicle_process_hit
        || (ped_has_hit_wall && vehicle_moving)
    {
        let result = ped.kill_ped_with_car_detailed(locomotion, vitality, 
            PedCarCollisionInput {
                vehicle_slot: CPoolHandle(vehicle_handle).index(),
                impulse: strongest_ped_impulse,
                vehicle_is_player,
                vehicle_is_train,
                current_surface_is_vehicle,
                current_surface_is_boat,
                vehicle_position,
                vehicle_move_speed,
                vehicle_right,
                vehicle_forward,
                vehicle_up,
                vehicle_mass,
                vehicle_col_min,
                vehicle_col_max,
                damage_normal: strongest_normal,
                ped_has_hit_wall,
                player_vehicle_process_hit,
                player_vehicle_speed_dot,
                defend_weakness,
                now_ms: time_ms,
                frame_counter,

            },
            general,
        );
        if result.vehicle_move_force.magnitude_sqr() > 0.0 {
            vehicle.physical.apply_move_force(result.vehicle_move_force);
        }
        if result.damage > 0.0 {
            // KillPedWithCar says SOUND_PED_DEFEND for both the fatal/high
            // impulse and ordinary fall branches.  The queued ped-talk path
            // owns phrase selection/voice throttling in ECS.
            crate::control::Script::queue_ped_script_say(
                ped,
                crate::audio::soundlist::SOUND_PED_DEFEND as i16,
            );
        }
        return (true, Some(result));
    }
    (true, None)
}

/// Ped/ped dynamic branch of `CPhysical::ProcessCollisionSectorList` for a
/// local player. Native `ApplyCollision` deliberately applies the ped-physics
/// impulse only to A in a ped/ped pair; the other ped gets its own response
/// when its physical is processed. Keep that asymmetry instead of inventing a
/// controller-style reciprocal shove.
#[allow(clippy::too_many_arguments)]
pub(crate) fn player_ped_collision_step(
    player: &mut CPed,
    other: &mut CPed,
    player_pose_was_rolled_back: bool,
    contacts: &[CColPoint],
    player_handle: i32,
    other_handle: i32,
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
    time_ms: u32,
) -> bool {
    if !record_ped_manifold_contact(
        &mut player.physical,
        &mut other.physical,
        contacts,
        EntityReference::Ped(player_handle),
        EntityReference::Ped(other_handle),
        time_ms,
    ) {
        return false;
    }

    let player_had_contacted = player.physical.entity.flags.has_contacted;
    let other_had_contacted = other.physical.entity.flags.has_contacted;
    let apply_pair_friction = !(player_had_contacted && other_had_contacted);
    let saved_player_friction = if apply_pair_friction && player_had_contacted {
        let saved = (player.physical.move_friction, player.physical.turn_friction);
        player.physical.move_friction = CVector::ZERO;
        player.physical.turn_friction = CVector::ZERO;
        player.physical.entity.flags.has_contacted = false;
        Some(saved)
    } else {
        None
    };
    let saved_other_friction = if apply_pair_friction && other_had_contacted {
        let saved = (other.physical.move_friction, other.physical.turn_friction);
        other.physical.move_friction = CVector::ZERO;
        other.physical.turn_friction = CVector::ZERO;
        other.physical.entity.flags.has_contacted = false;
        Some(saved)
    } else {
        None
    };

    let mut strongest_player: Option<(f32, u8, CVector)> = None;
    let mut responses = 0_u32;
    for point in contacts {
        let response = player.physical.apply_collision_dynamic(
            &mut other.physical,
            point,
            CPhysicalCollisionOptions {
                time_step,
                ..Default::default()
            },
        );
        if !response.applied {
            continue;
        }
        responses += 1;
        let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
        if strongest_player.is_none_or(|(impulse, _, _)| response.impulse_a > impulse) {
            strongest_player = Some((response.impulse_a, point.piece_a, normal));
        }
        if apply_pair_friction {
            let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                &surface_table.data,
                point.surface_a,
                point.surface_b,
            ) / contacts.len() as f32;
            if player
                .physical
                .apply_collision_friction_between(&mut other.physical, adhesion, point, time_step)
                .applied
            {
                player.physical.entity.flags.has_contacted = true;
                other.physical.entity.flags.has_contacted = true;
            }
        }
    }

    if let Some((move_friction, turn_friction)) = saved_player_friction
        && !player.physical.entity.flags.has_contacted
    {
        player.physical.entity.flags.has_contacted = true;
        player.physical.move_friction = move_friction;
        player.physical.turn_friction = turn_friction;
    }
    if let Some((move_friction, turn_friction)) = saved_other_friction
        && !other.physical.entity.flags.has_contacted
    {
        other.physical.entity.flags.has_contacted = true;
        other.physical.move_friction = move_friction;
        other.physical.turn_friction = turn_friction;
    }
    if responses == 0 {
        return false;
    }

    player.physical.distance_travelled = 0.0;
    if let Some((impulse, piece, normal)) = strongest_player {
        player
            .physical
            .record_collision_damage(piece as u16, impulse, Some(EntityReference::Ped(other_handle)), normal);
    }

    if !player_pose_was_rolled_back {
        let manifold_scale = 1.2 / contacts.len() as f32;
        let mut shift = CVector::ZERO;
        for point in contacts {
            shift.x += point.normal.x * point.depth * manifold_scale;
            shift.y += point.normal.y * point.depth * manifold_scale;
            shift.z += point.normal.z * point.depth * manifold_scale;
        }
        if let Some(deepest) = contacts
            .iter()
            .max_by(|left, right| left.depth.total_cmp(&right.depth))
        {
            shift.x += deepest.normal.x * deepest.depth * 0.2;
            shift.y += deepest.normal.y * deepest.depth * 0.2;
            shift.z += deepest.normal.z * deepest.depth * 0.2;
        }
        player.physical.apply_process_shift(shift);
        player.physical.entity.flags.is_in_safe_position = true;
    }
    true
}

/// Ped/object half of `CPhysical::ProcessCollisionSectorList`.
///
/// Positive `m_fUprootLimit`, stuck and infinite-mass objects deliberately use
/// the immovable `ApplyCollisionAlt` branch in re3. A zero-limit finite object,
/// however, can leave the static list during `ApplyCollision` and immediately
/// continue through the ordinary two-body response. Keeping that distinction
/// here is what lets a running player actually transfer momentum to authored
/// movable props without turning every CObject into either scenery or a wall.
pub(crate) fn player_object_collision_step(
    ped: &mut CPed,
    object: &mut CObject,
    object_entity: Entity,
    player_pose_was_rolled_back: bool,
    contacts: &[CColPoint],
    ped_handle: i32,
    object_handle: i32,
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
    time_ms: u32,
) -> Option<f32> {
    if !record_ped_manifold_contact(
        &mut ped.physical,
        &mut object.physical,
        contacts,
        EntityReference::Ped(ped_handle),
        EntityReference::Object(object_handle),
        time_ms,
    ) {
        return None;
    }

    let object_is_immovable = object.physical.flags.infinite_mass
        || object.physical.entity.flags.is_stuck
        || object.uproot_limit > 0.0;
    let current_surface = ped.current_phys_surface == Some(object_entity);
    let had_contacted = ped.physical.entity.flags.has_contacted;
    let object_had_contacted = object.physical.entity.flags.has_contacted;
    let apply_pair_friction = !object_is_immovable && !(had_contacted && object_had_contacted);
    let saved_ped_friction = if apply_pair_friction && had_contacted {
        let saved = (ped.physical.move_friction, ped.physical.turn_friction);
        ped.physical.move_friction = CVector::ZERO;
        ped.physical.turn_friction = CVector::ZERO;
        ped.physical.entity.flags.has_contacted = false;
        Some(saved)
    } else {
        None
    };
    let saved_object_friction = if apply_pair_friction && object_had_contacted {
        let saved = (object.physical.move_friction, object.physical.turn_friction);
        object.physical.move_friction = CVector::ZERO;
        object.physical.turn_friction = CVector::ZERO;
        object.physical.entity.flags.has_contacted = false;
        Some(saved)
    } else {
        None
    };
    let mut strongest_ped: Option<(f32, u8, CVector)> = None;
    let mut strongest_object: Option<(f32, u8, CVector)> = None;
    let mut responses = 0_u32;

    for point in contacts {
        let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);

        if object_is_immovable {
            let response = ped.physical.apply_collision_alt(
                point,
                object.physical.flags.infinite_mass,
                time_step,
            );
            if !response.applied {
                continue;
            }
            responses += 1;
            if strongest_ped.is_none_or(|(impulse, _, _)| response.impulse > impulse) {
                strongest_ped = Some((response.impulse, point.piece_a, normal));
            }

            // Native's immovable branch uses the single-body friction path
            // when A had not already established a contact this frame.
            if !had_contacted {
                let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                    &surface_table.data,
                    point.surface_a,
                    point.surface_b,
                ) / contacts.len() as f32;
                if ped
                    .physical
                    .apply_collision_friction_single(adhesion, point, time_step)
                    .applied
                {
                    ped.physical.entity.flags.has_contacted = true;
                }
            }
            continue;
        }

        if object.physical.entity.flags.is_static {
            let response = ped.physical.apply_collision_with_static_object(
                point,
                CPhysicalStaticObjectCollisionOptions {
                    object_uproot_limit: object.uproot_limit,
                    object_collision_damage_effect: object.collision_damage_effect,
                    object_uses_collision_after_damage: object.physical.entity.flags.uses_collision,
                    object_infinite_mass: object.physical.flags.infinite_mass,
                    object_is_glass: false,
                    no_more_collision_torque: false,
                },
            );
            if !response.applied {
                continue;
            }
            if !response.should_uproot_object {
                responses += 1;
                if strongest_ped.is_none_or(|(impulse, _, _)| response.impulse > impulse) {
                    strongest_ped = Some((response.impulse, point.piece_a, normal));
                }
                if apply_pair_friction {
                    let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                        &surface_table.data,
                        point.surface_a,
                        point.surface_b,
                    ) / contacts.len() as f32;
                    if ped
                        .physical
                        .apply_collision_friction_between(
                            &mut object.physical,
                            adhesion,
                            point,
                            time_step,
                        )
                        .applied
                    {
                        ped.physical.entity.flags.has_contacted = true;
                        object.physical.entity.flags.has_contacted = true;
                    }
                }
                continue;
            }

            // `ApplyCollision` changes a finite zero-limit CObject to dynamic
            // and then falls through to the same two-body response in this
            // very contact. Do not defer the reciprocal force by one frame.
            object.physical.entity.set_is_static(false);
            object.physical.flags.infinite_mass = false;
        }

        let response = ped.physical.apply_collision_dynamic(
            &mut object.physical,
            point,
            CPhysicalCollisionOptions {
                b_is_current_physical_surface_for_a: current_surface,
                time_step,
                ..Default::default()
            },
        );
        if !response.applied {
            continue;
        }
        responses += 1;
        if strongest_ped.is_none_or(|(impulse, _, _)| response.impulse_a > impulse) {
            strongest_ped = Some((response.impulse_a, point.piece_a, normal));
        }
        if strongest_object.is_none_or(|(impulse, _, _)| response.impulse_b > impulse) {
            strongest_object = Some((response.impulse_b, point.piece_b, normal));
        }

        if apply_pair_friction {
            let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                &surface_table.data,
                point.surface_a,
                point.surface_b,
            ) / contacts.len() as f32;
            if ped
                .physical
                .apply_collision_friction_between(&mut object.physical, adhesion, point, time_step)
                .applied
            {
                ped.physical.entity.flags.has_contacted = true;
                object.physical.entity.flags.has_contacted = true;
            }
        }
    }

    if let Some((move_friction, turn_friction)) = saved_ped_friction
        && !ped.physical.entity.flags.has_contacted
    {
        ped.physical.entity.flags.has_contacted = true;
        ped.physical.move_friction = move_friction;
        ped.physical.turn_friction = turn_friction;
    }
    if let Some((move_friction, turn_friction)) = saved_object_friction
        && !object.physical.entity.flags.has_contacted
    {
        object.physical.entity.flags.has_contacted = true;
        object.physical.move_friction = move_friction;
        object.physical.turn_friction = turn_friction;
    }

    if responses == 0 {
        return None;
    }

    // Native ProcessCollision restores the pre-integration matrix and returns
    // with distance zero on a responding collision. ProcessShift is only the
    // later penetration fix and must not masquerade as travelled distance.
    ped.physical.distance_travelled = 0.0;
    if contacts.iter().any(|point| {
        point.surface_b == crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8
    }) {
        ped.flags.hit_steep_slope = true;
    }

    if let Some((impulse, piece, normal)) = strongest_ped {
        ped.physical
            .record_collision_damage(piece as u16, impulse, Some(EntityReference::Object(object_handle)), normal);
    }
    if let Some((impulse, piece, normal)) = strongest_object {
        object
            .physical
            .record_collision_damage(piece as u16, impulse, Some(EntityReference::Ped(ped_handle)), normal);
    }

    if !player_pose_was_rolled_back {
        let manifold_scale = 1.2 / contacts.len() as f32;
        let mut shift = CVector::ZERO;
        for point in contacts {
            let amount = point.depth * manifold_scale;
            shift.x += point.normal.x * amount;
            shift.y += point.normal.y * amount;
            shift.z += point.normal.z * amount;
        }
        if let Some(deepest) = contacts
            .iter()
            .max_by(|left, right| left.depth.total_cmp(&right.depth))
        {
            shift.x += deepest.normal.x * deepest.depth * 0.2;
            shift.y += deepest.normal.y * deepest.depth * 0.2;
            shift.z += deepest.normal.z * deepest.depth * 0.2;
        }
        ped.physical.apply_process_shift(shift);
        ped.physical.entity.flags.is_in_safe_position = true;
    }
    object.object_matrix = object.physical.entity.matrix;

    Some(strongest_object.map_or(0.0, |(impulse, _, _)| impulse))
}

/// Live `CWorld::Process` collision pass for script-owned automobiles. The
/// control systems integrate first; this pass then mirrors re3's sector-list
/// dynamic branch (`ProcessColModels` -> `ApplyCollision` -> damage records).
pub(crate) fn process_script_entity_collisions(
    mut script_entities: ScriptEntityWorld,
    mut bridge: ResMut<ScriptGameplayBridge>,
    mut world_players: CWorldPlayers,
    resources: ScriptCollisionResources,
    mut events: ResMut<CEventList>,
    mut general: ResMut<CGeneral>,
    mut object_globals: ResMut<CObjectGlobalState>,
    object_collision_states: Query<&crate::objects::Object::ObjectCollisionState>,
    mut particles: Option<ResMut<CParticle>>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut scratch: Local<CollisionScratch>,
    mut vehicle_spatial: ResMut<super::VehicleSpatial::VehicleCollisionSpatial>,
    mut pair_diagnostics: ResMut<super::VehiclePair::VehiclePairDiagnostics>,
) {
    crate::profile_scope!("process_script_entity_collisions");
    let _audit_timer = GameplayAuditTimer::start("process_script_entity_collisions");
    let ScriptCollisionResources {
        camera,
        particle_systems,
        collision,
        file_loader,
        handling_manager,
        ped_stats,

        surface_table,
        timer,
        darkel,
    } = resources;
    let time_step = timer.time_step().max(0.0);
    let time_ms = timer.time_in_milliseconds();
    let frame_counter = timer.frame_counter();
    let mut particle_emitter = particles
        .as_deref_mut()
        .zip(particle_systems.as_deref())
        .zip(camera.as_deref())
        .map(|((particles, systems), camera)| particles.emitter(systems, camera.position));
    let audio_camera_position = camera
        .as_ref()
        .map_or(file_loader.renderer.camera_position, |camera| {
            camera.position
        });
    // Retain pooled allocation across frames: `Local<CollisionScratch>` keeps
    // capacity for broad-phase sweeps, avoiding per-frame heap allocation
    // spikes that froze the game when traffic+ped count grew after the
    // `RANDOM_VEHICLE`/`RANDOM_CHAR` population fix.
    scratch.handles.clear();
    scratch.broad_phase.clear();
    scratch.object_broad_phase.clear();
    scratch.moving_object_broad_phase.clear();
    scratch.player_object_broad_phase.clear();
    scratch.ped_broad_phase.clear();
    scratch.dynamic_collision_rollbacks.clear();
    scratch.vehicle_handle_by_slot.clear();
    scratch.handles.extend(
        script_entities
            .vehicles
            .iter()
            .filter_map(|(&handle, vehicle)| {
                let base = vehicle.base();
                if !base.physical.entity.flags.uses_collision {
                    return None;
                }
                let zone_level = base.physical.zone_level;
                if zone_level > crate::collision::ColModel::LEVEL_GENERIC as i8
                    && zone_level != collision.collision_in_memory as i8
                {
                    return None;
                }
                Some(handle)
            }),
    );
    scratch
        .handles
        .sort_unstable_by_key(|handle| CPoolHandle(*handle).index());
    let CollisionScratch {
        handles,
        vehicle_handle_by_slot,
        ..
    } = &mut *scratch;
    for &handle in handles.iter() {
        vehicle_handle_by_slot.insert(CPoolHandle(handle).index(), handle);
    }
    // CPhysical::ProcessCollision publishes this before CheckCollision, even
    // when a moving car reaches no positive contact in the narrow phase.
    for &handle in handles.iter() {
        let Some(mut vehicle) = script_entities.vehicle_get_mut(handle) else { continue; };
        let mut base = vehicle.base_mut();
        if base.physical.entity.status != eEntityStatus::STATUS_SIMPLE
            && (base.physical.move_speed != CVector::ZERO
                || base.physical.turn_speed != CVector::ZERO
                || base.physical.entity.status == eEntityStatus::STATUS_PLAYER) {
            base.flags.vehicle_col_processed = true;
        }
    }
    for index in 0..scratch.handles.len() {
        let handle = scratch.handles[index];
        let Some(base) = script_entities
            .vehicle_get(handle)
            .map(|vehicle| vehicle.base())
        else {
            continue;
        };
        let position = base.physical.entity.get_position();
        let Some(radius) = file_loader
            .model_info
            .col_model(base.physical.entity.model_index.max(0) as usize)
            .map(|model| model.bounding_sphere.radius)
        else {
            vehicle_spatial.missing_bounds += 1;
            warn_once!("vehicle_collision_waiting_original_bounds");
            continue;
        };
        scratch
            .broad_phase
            .push((handle, position.x, position.y, radius));
    }
    scratch
        .broad_phase
        .sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
    let max_radius = scratch
        .broad_phase
        .iter()
        .map(|entry| entry.3)
        .fold(0.0_f32, f32::max);

    // Buildings and dummies are infinite-mass entities in the static branch.
    for &handle in scratch.handles.iter() {
        let multiplier = script_entities.vehicles.get(&script_entities.index, &handle)
            .and_then(|vehicle| vehicle.base().handling_id)
            .and_then(|id| usize::try_from(id).ok())
            .and_then(|id| handling_manager.get_handling_data(id))
            .map_or(1.0, |handling| handling.collision_damage_multiplier);
        let Some(previous_matrix) = script_entities.vehicles.collision_pose(&script_entities.index, handle) else { continue; };
        let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
            script_entities.vehicle_get_mut(handle)
        else {
            continue;
        };
        automobile_static_collision_step(
            &mut car,
            previous_matrix,
            Some(handle),
            &file_loader,
            audio_camera_position,
            &surface_table,
            time_step,
            Some(time_ms),
            multiplier,
            &mut general,
            dmaudio.as_deref_mut(),
        );
    }

    for (entity, _, mut object) in script_entities.objects.entities.iter_mut() {
        let collision_state = object_collision_states.get(entity)
            .expect("live CObject must retain its required ObjectCollisionState");
        object_static_collision_step(
            &mut object,
            collision_state,
            &file_loader,
            &surface_table,
            time_step,
            time_ms,
        );
    }

    // Mission/game objects are real CObject instances, not static-map
    // triangles.  Re3 routes vehicle contacts through ApplyCollision's
    // static-object branch so uproot thresholds and ObjectDamage are handled
    // before the next sector pass.
    scratch
        .object_broad_phase
        .extend(script_entities.objects.values().filter_map(|entity| {
            let object = &entity.object;
            if !object.physical.entity.flags.uses_collision
                || object.physical.entity.flags.is_static == false
            {
                return None;
            }
            let position = object.physical.entity.get_position();
            let radius = file_loader
                .model_info
                .col_model(object.physical.entity.model_index.max(0) as usize)?
                .bounding_sphere
                .radius;
            Some((entity.handle, position.x, position.y, radius))
        }));
    scratch
        .object_broad_phase
        .sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
    let object_max_radius = scratch
        .object_broad_phase
        .iter()
        .map(|entry| entry.3)
        .fold(0.0_f32, f32::max);
    for &(vehicle_handle, vehicle_x, vehicle_y, vehicle_radius) in scratch.broad_phase.iter() {
        let first_object = scratch
            .object_broad_phase
            .partition_point(|entry| entry.1 < vehicle_x - vehicle_radius - object_max_radius);
        for &(object_handle, object_x, object_y, object_radius) in
            scratch.object_broad_phase[first_object..].iter()
        {
            if object_x - vehicle_x > vehicle_radius + object_max_radius {
                break;
            }
            if (object_x - vehicle_x).abs() > vehicle_radius + object_radius {
                continue;
            }
            if (object_y - vehicle_y).abs() > vehicle_radius + object_radius {
                continue;
            }
            let Some(vehicle) = script_entities.vehicle_get(vehicle_handle) else {
                continue;
            };
            let Some(object) = script_entities.objects.get(&script_entities.index, &object_handle) else {
                continue;
            };
            let vehicle_matrix = vehicle.base().physical.entity.matrix;
            let vehicle_model_id = vehicle.base().physical.entity.model_index.max(0) as usize;
            let object_matrix = object.object.physical.entity.matrix;
            let object_model_id = object.object.physical.entity.model_index.max(0) as usize;
            let (Some(vehicle_model), Some(object_model)) = (
                file_loader.model_info.col_model(vehicle_model_id),
                file_loader.model_info.col_model(object_model_id),
            ) else {
                continue;
            };
            let contacts = CCollision::process_col_models_spheres(
                &vehicle_matrix,
                vehicle_model,
                &object_matrix,
                object_model,
            );
            if contacts.is_empty() {
                continue;
            }

            let Some(vehicle_entity_id) = script_entities.index.vehicle(vehicle_handle) else {
                continue;
            };
            let Some(mut vehicle_entity) = script_entities.vehicles.get_entity_mut(vehicle_entity_id)
            else {
                continue;
            };
            let Some(object_entity) = script_entities.objects.get_mut(&script_entities.index, &object_handle) else {
                continue;
            };
            let mut vehicle = vehicle_entity.base_mut();
            let object = &mut *object_entity.object;
            let object_options = CPhysicalStaticObjectCollisionOptions {
                object_uproot_limit: object.uproot_limit,
                object_collision_damage_effect: object.collision_damage_effect,
                object_uses_collision_after_damage: object.physical.entity.flags.uses_collision,
                object_infinite_mass: object.physical.flags.infinite_mass,
                object_is_glass: false,
                no_more_collision_torque: false,
            };
            let mut strongest: Option<(f32, u8, CVector)> = None;
            let mut strongest_object: Option<(f32, u8, CVector)> = None;
            let mut response_count = 0_u32;
            for point in &contacts {
                let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
                if object.physical.entity.flags.is_static {
                    let response = vehicle
                        .physical
                        .apply_collision_with_static_object(point, object_options);
                    if !response.applied {
                        continue;
                    }
                    response_count += 1;
                    if strongest.is_none_or(|(impulse, _, _)| response.impulse > impulse) {
                        strongest = Some((response.impulse, point.piece_a, normal));
                    }
                    if !response.should_uproot_object {
                        continue;
                    }
                    // re3 `CPhysical::ApplyCollision` uproots the prop and then
                    // falls through to the ordinary two-body exchange in this
                    // very contact (Physical.cpp:617-651). Deferring the
                    // impulse by a frame would leave the prop standing: the
                    // colliding-entity lock installed below suppresses any
                    // later response against this vehicle while the pair's
                    // bounds still overlap.
                    object.physical.entity.set_is_static(false);
                    object.physical.flags.infinite_mass = false;
                    object.colliding_entity = Some(CPoolHandle(vehicle_handle).index());
                    object.flags.has_been_damaged = response.should_mark_object_damaged;
                }
                let response = vehicle.physical.apply_collision_dynamic(
                    &mut object.physical,
                    point,
                    CPhysicalCollisionOptions {
                        time_step,
                        ..Default::default()
                    },
                );
                if !response.applied {
                    continue;
                }
                if strongest.is_none_or(|(impulse, _, _)| response.impulse_a > impulse) {
                    strongest = Some((response.impulse_a, point.piece_a, normal));
                }
                if strongest_object.is_none_or(|(impulse, _, _)| response.impulse_b > impulse) {
                    strongest_object = Some((response.impulse_b, point.piece_b, normal));
                }
            }
            if response_count == 0 {
                continue;
            }
            let object_slot = CPoolHandle(object_handle).index();
            let vehicle_slot = CPoolHandle(vehicle_handle).index();
            let simple_vehicle = vehicle.physical.entity.status == eEntityStatus::STATUS_SIMPLE;
            vehicle.physical.entity.flags.has_hit_wall = true;
            vehicle.physical.add_collision_record(EntityReference::Object(object_handle), time_ms);
            // The narrow phase was sampled from `vehicle_matrix`, the pose at
            // the start of this collision pass.  Restore that integrated pose
            // after applying the impulse; restoring the pre-control matrix
            // would rewind the car before ProcessControl and make suspension
            // corrections visibly oscillate on every frame.
            vehicle.physical.entity.set_matrix(vehicle_matrix);
            if simple_vehicle {
                // `ProcessCollision_SimpleCar` promotes a traffic car to the
                // ordinary physics branch after a vehicle/object hit; the
                // rail position itself remains the integrated pose sampled
                // above.
                vehicle.set_status(eEntityStatus::STATUS_PHYSICS);
                vehicle.switch_to_real_physics(time_ms);
            }
            if let Some((impulse, piece_id, normal)) = strongest {
                vehicle.physical.record_collision_damage(
                    piece_id as u16,
                    impulse,
                    Some(EntityReference::Object(object_handle)),
                    normal,
                );
                if object.collision_damage_effect != 0 && impulse > 20.0 {
                    let damage = object.object_damage(
                        impulse,
                        &mut object_globals,
                        CObjectDamageContext {
                            body_cast_model_index: file_loader
                                .model_info
                                .get_model_info_by_name("bodycast")
                                .map(|(model_id, _)| model_id as i16),
                        },
                        &mut general,
                    );
                    emit_object_damage_effects(
                        &damage,
                        object.physical.entity.get_position(),
                        particle_emitter.as_mut(),
                        dmaudio.as_deref_mut(),
                    );
                    if damage.smashed {
                        object.physical.entity.flags.uses_collision = false;
                    }
                }
            }
            // re3's two-body branch records the damaged-piece impulse on both
            // sides (Physical.cpp:1651-1652); the object's record feeds the
            // next frame's `CObject::ProcessControl` ObjectDamage pass.
            if let Some((impulse, piece_id, normal)) = strongest_object {
                object.physical.record_collision_damage(
                    piece_id as u16,
                    impulse,
                    Some(EntityReference::Vehicle(vehicle_handle)),
                    normal,
                );
            }
        }
    }

    // Once an object is uprooted it leaves the static branch, but remains in
    // CWorld's moving list. Re3 then uses the ordinary two-body collision path
    // and applies ObjectDamage from the largest B impulse.
    scratch
        .moving_object_broad_phase
        .extend(script_entities.objects.values().filter_map(|entity| {
            let object = &entity.object;
            if !object.physical.entity.flags.uses_collision
                || object.physical.entity.flags.is_static
            {
                return None;
            }
            let position = object.physical.entity.get_position();
            let radius = file_loader
                .model_info
                .col_model(object.physical.entity.model_index.max(0) as usize)?
                .bounding_sphere
                .radius;
            Some((entity.handle, position.x, position.y, radius))
        }));
    scratch
        .moving_object_broad_phase
        .sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
    let moving_object_max_radius = scratch
        .moving_object_broad_phase
        .iter()
        .map(|entry| entry.3)
        .fold(0.0_f32, f32::max);

    // ProcessCollisionSectorList clears m_pCollidingEntity only after the
    // pair's bounding spheres stop touching. Until then a damaged, temporary
    // or uprooted object is deliberately skipped so it cannot receive the
    // same vehicle impulse every frame.
    {
        let vehicles = &script_entities.vehicles;
        for entity in script_entities.objects.values_mut() {
            let object = &mut *entity.object;
            let Some(vehicle_slot) = object.colliding_entity else {
                continue;
            };
            let vehicle = vehicles
                .iter()
                .find(|(handle, _)| CPoolHandle(**handle).index() == vehicle_slot)
                .map(|(_, vehicle)| vehicle.base());
            let still_touching = vehicle.is_some_and(|vehicle| {
                let object_position = object.physical.entity.get_position();
                let vehicle_position = vehicle.physical.entity.get_position();
                let Some(object_model) = file_loader
                    .model_info
                    .col_model(object.physical.entity.model_index.max(0) as usize)
                else {
                    return false;
                };
                let Some(vehicle_model) = file_loader
                    .model_info
                    .col_model(vehicle.physical.entity.model_index.max(0) as usize)
                else {
                    return false;
                };
                let radius =
                    object_model.bounding_sphere.radius + vehicle_model.bounding_sphere.radius;
                let dx = object_position.x - vehicle_position.x;
                let dy = object_position.y - vehicle_position.y;
                let dz = object_position.z - vehicle_position.z;
                dx * dx + dy * dy + dz * dz <= radius * radius
            });
            let _ = object.update_vehicle_collision_lock(vehicle_slot, still_touching);
        }
    }

    for &(vehicle_handle, vehicle_x, vehicle_y, vehicle_radius) in scratch.broad_phase.iter() {
        let first_object = scratch.moving_object_broad_phase.partition_point(|entry| {
            entry.1 < vehicle_x - vehicle_radius - moving_object_max_radius
        });
        for &(object_handle, object_x, object_y, object_radius) in
            scratch.moving_object_broad_phase[first_object..].iter()
        {
            if object_x - vehicle_x > vehicle_radius + moving_object_max_radius {
                break;
            }
            if (object_x - vehicle_x).abs() > vehicle_radius + object_radius {
                continue;
            }
            if (object_y - vehicle_y).abs() > vehicle_radius + object_radius {
                continue;
            }
            let vehicle_slot = CPoolHandle(vehicle_handle).index();
            if script_entities.objects.get(&script_entities.index, &object_handle)
                .is_some_and(|entity| entity.object.colliding_entity == Some(vehicle_slot))
            {
                continue;
            }
            let Some(vehicle) = script_entities.vehicle_get(vehicle_handle) else {
                continue;
            };
            let Some(object) = script_entities.objects.get(&script_entities.index, &object_handle) else {
                continue;
            };
            let vehicle_matrix = vehicle.base().physical.entity.matrix;
            let vehicle_model_id = vehicle.base().physical.entity.model_index.max(0) as usize;
            let object_matrix = object.object.physical.entity.matrix;
            let object_model_id = object.object.physical.entity.model_index.max(0) as usize;
            let (Some(vehicle_model), Some(object_model)) = (
                file_loader.model_info.col_model(vehicle_model_id),
                file_loader.model_info.col_model(object_model_id),
            ) else {
                continue;
            };
            let contacts = CCollision::process_col_models_spheres(
                &vehicle_matrix,
                vehicle_model,
                &object_matrix,
                object_model,
            );
            if contacts.is_empty() {
                continue;
            }

            let Some(vehicle_entity_id) = script_entities.index.vehicle(vehicle_handle) else {
                continue;
            };
            let Some(mut vehicle_entity) = script_entities.vehicles.get_entity_mut(vehicle_entity_id)
            else {
                continue;
            };
            let Some(object_entity) = script_entities.objects.get_mut(&script_entities.index, &object_handle) else {
                continue;
            };
            let mut vehicle = vehicle_entity.base_mut();
            let object = &mut *object_entity.object;
            let mut strongest_vehicle: Option<(f32, u8, CVector)> = None;
            let mut strongest_object: Option<(f32, u8, CVector)> = None;
            for point in &contacts {
                let response = vehicle.physical.apply_collision_dynamic(
                    &mut object.physical,
                    point,
                    CPhysicalCollisionOptions {
                        time_step,
                        ..Default::default()
                    },
                );
                if !response.applied {
                    continue;
                }
                let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
                if strongest_vehicle.is_none_or(|(impulse, _, _)| response.impulse_a > impulse) {
                    strongest_vehicle = Some((response.impulse_a, point.piece_a, normal));
                }
                if strongest_object.is_none_or(|(impulse, _, _)| response.impulse_b > impulse) {
                    strongest_object = Some((response.impulse_b, point.piece_b, normal));
                }
                let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                    &surface_table.data,
                    point.surface_a,
                    point.surface_b,
                ) / contacts.len() as f32;
                if vehicle
                    .physical
                    .apply_collision_friction_between(
                        &mut object.physical,
                        adhesion,
                        point,
                        time_step,
                    )
                    .applied
                {
                    vehicle.physical.entity.flags.has_contacted = true;
                    object.physical.entity.flags.has_contacted = true;
                }
            }
            let Some((vehicle_impulse, vehicle_piece, vehicle_normal)) = strongest_vehicle else {
                continue;
            };
            let object_slot = CPoolHandle(object_handle).index();
            vehicle.physical.add_collision_record(EntityReference::Object(object_handle), time_ms);
            object.physical.add_collision_record(EntityReference::Vehicle(vehicle_handle), time_ms);
            object.colliding_entity = Some(vehicle_slot);
            vehicle.physical.entity.set_matrix(vehicle_matrix);
            object.physical.entity.set_matrix(object_matrix);
            object.object_matrix = object.physical.entity.matrix;
            vehicle.physical.record_collision_damage(
                vehicle_piece as u16,
                vehicle_impulse,
                Some(EntityReference::Object(object_handle)),
                vehicle_normal,
            );
            // re3's two-body branch records the damaged-piece impulse on both
            // sides (Physical.cpp:1651-1652); the object's record feeds the
            // next frame's `CObject::ProcessControl` ObjectDamage pass.
            if let Some((object_impulse, object_piece, object_normal)) = strongest_object {
                object.physical.record_collision_damage(
                    object_piece as u16,
                    object_impulse,
                    Some(EntityReference::Vehicle(vehicle_handle)),
                    object_normal,
                );
                  if object.collision_damage_effect != 0 && object_impulse > 20.0 {
                      let damage = object.object_damage(
                          object_impulse,
                          &mut object_globals,
                          CObjectDamageContext {
                              body_cast_model_index: file_loader
                                  .model_info
                                  .get_model_info_by_name("bodycast")
                                  .map(|(model_id, _)| model_id as i16),
                          },
                          &mut general,
                      );
                        emit_object_damage_effects(
                          &damage,
                          object.physical.entity.get_position(),
                          particle_emitter.as_mut(),
                          dmaudio.as_deref_mut(),
                    );
                  }
              }
        }
    }

    // Dynamic CObject/CObject contacts use the same two-body branch as re3's
    // `CPhysical::ProcessCollisionSectorList`.  Once both objects have been
    // uprooted they are no longer part of the static-object response above,
    // but still need reciprocal impulse, friction and damage records.
    for first_index in 0..scratch.moving_object_broad_phase.len() {
        let (handle_a, x_a, y_a, radius_a) = scratch.moving_object_broad_phase[first_index];
        for &(handle_b, x_b, y_b, radius_b) in &scratch.moving_object_broad_phase[first_index + 1..]
        {
            if x_b - x_a > radius_a + moving_object_max_radius {
                break;
            }
            let combined_radius = radius_a + radius_b;
            if (y_b - y_a).abs() > combined_radius {
                continue;
            }
            let Some(object_a) = script_entities.objects.get(&script_entities.index, &handle_a) else {
                continue;
            };
            let Some(object_b) = script_entities.objects.get(&script_entities.index, &handle_b) else {
                continue;
            };
            let matrix_a = object_a.object.physical.entity.matrix;
            let model_a_id = object_a.object.physical.entity.model_index.max(0) as usize;
            let matrix_b = object_b.object.physical.entity.matrix;
            let model_b_id = object_b.object.physical.entity.model_index.max(0) as usize;
            let (Some(model_a), Some(model_b)) = (
                file_loader.model_info.col_model(model_a_id),
                file_loader.model_info.col_model(model_b_id),
            ) else {
                continue;
            };
            let contacts =
                CCollision::process_col_models_spheres(&matrix_a, model_a, &matrix_b, model_b);
            if contacts.is_empty() {
                continue;
            }

            let (Some(entity_a_id), Some(entity_b_id)) = (
                script_entities.index.object(handle_a),
                script_entities.index.object(handle_b),
            ) else {
                continue;
            };
            let Ok([mut entity_a, mut entity_b]) = script_entities
                .objects
                .get_many_mut([entity_a_id, entity_b_id])
            else {
                continue;
            };
            let object_a = &mut entity_a.object;
            let object_b = &mut entity_b.object;
            let mut strongest_a: Option<(f32, u8, CVector)> = None;
            let mut strongest_b: Option<(f32, u8, CVector)> = None;
            for point in &contacts {
                let response = object_a.physical.apply_collision_dynamic(
                    &mut object_b.physical,
                    point,
                    CPhysicalCollisionOptions {
                        time_step,
                        ..Default::default()
                    },
                );
                if !response.applied {
                    continue;
                }
                let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
                if strongest_a.is_none_or(|(impulse, _, _)| response.impulse_a > impulse) {
                    strongest_a = Some((response.impulse_a, point.piece_a, normal));
                }
                if strongest_b.is_none_or(|(impulse, _, _)| response.impulse_b > impulse) {
                    strongest_b = Some((response.impulse_b, point.piece_b, normal));
                }
                let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                    &surface_table.data,
                    point.surface_a,
                    point.surface_b,
                ) / contacts.len() as f32;
                if object_a
                    .physical
                    .apply_collision_friction_between(
                        &mut object_b.physical,
                        adhesion,
                        point,
                        time_step,
                    )
                    .applied
                {
                    object_a.physical.entity.flags.has_contacted = true;
                    object_b.physical.entity.flags.has_contacted = true;
                }
            }
            let Some((impulse_a, piece_a, normal_a)) = strongest_a else {
                continue;
            };
            let Some((impulse_b, piece_b, normal_b)) = strongest_b else {
                continue;
            };
            let slot_a = CPoolHandle(handle_a).index();
            let slot_b = CPoolHandle(handle_b).index();
            object_a.physical.entity.set_matrix(matrix_a);
            object_b.physical.entity.set_matrix(matrix_b);
            object_a.object_matrix = object_a.physical.entity.matrix;
            object_b.object_matrix = object_b.physical.entity.matrix;
            object_a.physical.add_collision_record(EntityReference::Object(handle_b), time_ms);
            object_b.physical.add_collision_record(EntityReference::Object(handle_a), time_ms);
            object_a.physical.record_collision_damage(
                piece_a as u16,
                impulse_a,
                Some(EntityReference::Object(handle_b)),
                normal_a,
            );
            object_b.physical.record_collision_damage(
                piece_b as u16,
                impulse_b,
                Some(EntityReference::Object(handle_a)),
                normal_b,
            );
        }
    }

    // Player collision retries happen after object/object and vehicle/object
    // response, so cache one current object broad phase here. Native gets the
    // same locality from CWorld sector lists; rebuilding a Vec by scanning all
    // objects on each of six player retries caused large frame spikes.
    scratch
        .player_object_broad_phase
        .extend(script_entities.objects.values().filter_map(|entity| {
            let object = &entity.object;
            if !object.physical.entity.flags.uses_collision {
                return None;
            }
            let position = object.physical.entity.get_position();
            let radius = file_loader
                .model_info
                .col_model(object.physical.entity.model_index.max(0) as usize)?
                .bounding_sphere
                .radius;
            Some((entity.handle, position.x, position.y, radius))
        }));
    scratch
        .player_object_broad_phase
        .sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
    let player_object_max_radius = scratch
        .player_object_broad_phase
        .iter()
        .map(|entry| entry.3)
        .fold(0.0_f32, f32::max);

    // Ped/vehicle contacts use the dynamic ApplyCollision branch.  Keep
    // seated peds out of this list: CPed::SetPedPositionInCar owns those
    // transforms and the driver is synchronised from the vehicle below.
    scratch
        .ped_broad_phase
        .extend(script_entities.peds.iter().filter_map(|(&handle, ped)| {
            if script_entities.relation_for_char(handle).is_some()
                || !ped.ped.physical.entity.flags.uses_collision
            {
                return None;
            }
            let position = ped.ped.physical.entity.get_position();
            let radius = file_loader
                .model_info
                .col_model(ped.ped.physical.entity.model_index.max(0) as usize)
                .map_or(0.8, |model| model.bounding_sphere.radius);
            Some((handle, position.x, position.y, radius))
        }));
    scratch
        .ped_broad_phase
        .sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
    let ped_max_radius = scratch
        .ped_broad_phase
        .iter()
        .map(|entry| entry.3)
        .fold(0.8_f32, f32::max);
    for &(vehicle_handle, vehicle_x, vehicle_y, vehicle_radius) in scratch.broad_phase.iter() {
        let first_ped = scratch
            .ped_broad_phase
            .partition_point(|entry| entry.1 < vehicle_x - vehicle_radius - ped_max_radius);
        for &(ped_handle, ped_x, ped_y, ped_radius) in scratch.ped_broad_phase[first_ped..].iter() {
            if ped_x - vehicle_x > vehicle_radius + ped_max_radius {
                break;
            }
            if (ped_x - vehicle_x).abs() > vehicle_radius + ped_radius {
                continue;
            }
            if (ped_y - vehicle_y).abs() > vehicle_radius + ped_radius {
                continue;
            }
            let Some((vehicle_matrix, vehicle_model_id, ped_matrix, ped_model_id)) =
                script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                    .zip(script_entities.ped_get(ped_handle))
                    .map(|(vehicle, ped)| {
                        (
                            vehicle.base().physical.entity.matrix,
                            vehicle.base().physical.entity.model_index.max(0) as usize,
                            ped.ped.physical.entity.matrix,
                            ped.ped.physical.entity.model_index.max(0) as usize,
                        )
                    })
            else {
                continue;
            };
            let (Some(vehicle_model), Some(ped_model)) = (
                file_loader.model_info.col_model(vehicle_model_id),
                file_loader.model_info.col_model(ped_model_id),
            ) else {
                continue;
            };
            let contacts = CCollision::process_col_models_spheres(
                &vehicle_matrix,
                vehicle_model,
                &ped_matrix,
                ped_model,
            );
            if contacts.is_empty() {
                continue;
            }

            let vehicle_col_min = CVector::new(
                vehicle_model.bounding_box.min.x,
                vehicle_model.bounding_box.min.y,
                vehicle_model.bounding_box.min.z,
            );
            let vehicle_col_max = CVector::new(
                vehicle_model.bounding_box.max.x,
                vehicle_model.bounding_box.max.y,
                vehicle_model.bounding_box.max.z,
            );
            let current_surface_vehicle_handle = script_entities.peds.get(&script_entities.index, &ped_handle)
                .and_then(|entity| entity.ped.current_phys_surface)
                .and_then(|entity| script_entities.vehicle_handle_for_entity(entity));
            let (current_surface_is_vehicle, current_surface_is_boat) =
                current_surface_vehicle_handle
                    .and_then(|handle| script_entities.vehicle_get(handle))
                    .map_or((false, false), |vehicle| {
                        (true, matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }))
                    });

            let vehicle_entity_id = script_entities.vehicle_entity(vehicle_handle);
            let script_driver = vehicle_entity_id.and_then(|vehicle_entity| {
                script_entities
                    .script_occupant_handles(vehicle_entity)
                    .find_map(|(handle, seat)| {
                        matches!(seat, crate::peds::VehicleRelations::VehicleSeat::Driver)
                            .then_some(handle)
                    })
            });
            let player_driver = vehicle_entity_id.and_then(|vehicle_entity| {
                world_players.iter_slots().find_map(|(index, player)| {
                    let player = player?;
                    let relation = world_players.relation(index as i32)?;
                    (relation.vehicle == vehicle_entity
                        && matches!(
                            relation.seat,
                            crate::peds::VehicleRelations::VehicleSeat::Driver
                        ))
                    .then_some((index as i32, player.pool_handle))
                })
            });
            let victim_type = script_entities.peds.get(&script_entities.index, &ped_handle)
                .map(|entity| entity.ped.ped_type);

            let reaction = {
                let ScriptEntityWorld { vehicles, peds, index, .. } = &mut script_entities;
                let Some(mut vehicle_entity) = vehicles.get_mut(index, &vehicle_handle) else {
                    continue;
                };
                let Some(ped_entity) = peds.get_mut(index, &ped_handle) else {
                    continue;
                };
                let collision_matrix = Some(vehicle_matrix);
                let mut vehicle = vehicle_entity.base_mut();
                let ped = &mut *ped_entity.ped;
                let vitality = &mut *ped_entity.vitality;
                let defend_weakness = ped_stats.GetPedStats(ped.ped_stats).m_defend_weakness;
                vehicle_ped_collision_step(
                    &mut vehicle,
                    collision_matrix,
                    ped, ped_entity.locomotion, vitality,
                    ped_matrix.position,
                    false,
                    false,
                    &contacts,
                    vehicle_handle,
                    ped_handle,
                    vehicle_col_min,
                    vehicle_col_max,
                    current_surface_vehicle_handle == Some(vehicle_handle),
                    current_surface_is_vehicle,
                    current_surface_is_boat,
                    defend_weakness,

                    time_step,
                    time_ms,
                    frame_counter,
                    &mut general,
                )
                .1
            };

            // PedFight.cpp registers hit-and-run only in KillPedWithCar's
            // high-impulse non-player branch (the 1000 damage path). Keep the
            // event's criminal tied to the actual driver relation, including
            // the player who lives outside ScriptEntityWorld::peds.
            if reaction.is_some_and(|result| result.damage >= 1_000.0)
                && let Some(criminal_ref) =
                    player_driver.map(|(_, handle)| handle).or(script_driver)
            {
                let event_type = if victim_type == Some(crate::peds::PedType::ePedType::PEDTYPE_COP)
                {
                    crate::core::EventList::EEventType::HitAndRunCop
                } else {
                    crate::core::EventList::EEventType::HitAndRun
                };
                let already_registered = events.events.iter().any(|event| {
                    event.event_type == event_type
                        && event.entity_type == crate::core::EventList::EEventEntity::Ped
                        && event.entity_ref == ped_handle
                });
                let _ = events.register_entity_event(
                    event_type,
                    crate::core::EventList::EEventEntity::Ped,
                    ped_handle,
                    Some(criminal_ref),
                    ped_matrix.position,
                    1_000,
                    time_ms,
                );

                // CEventList::RegisterEvent immediately reports a newly
                // registered crime when the criminal is FindPlayerPed().
                // Preserve its 14 m WorkOutPolicePresence split between
                // immediate and queued wanted crimes.
                if !already_registered && let Some((player_index, _)) = player_driver {
                    let player_position = world_players
                        .player(player_index)
                        .map(|player| player.position())
                        .unwrap_or(vehicle_matrix.position);
                    let radius_sqr = 14.0_f32 * 14.0_f32;
                    let mut police_presence = 0_i32;
                    for ped in script_entities.peds.values() {
                        let model = ped.model_id.max(0) as usize;
                        if !matches!(
                            model,
                            crate::modelinfo::ModelIndices::MI_COP
                                | crate::modelinfo::ModelIndices::MI_SWAT
                                | crate::modelinfo::ModelIndices::MI_FBI
                                | crate::modelinfo::ModelIndices::MI_ARMY
                        ) {
                            continue;
                        }
                        let pos = ped.ped.physical.entity.get_position();
                        let delta = CVector::new(
                            pos.x - vehicle_matrix.position.x,
                            pos.y - vehicle_matrix.position.y,
                            pos.z - vehicle_matrix.position.z,
                        );
                        if delta.magnitude_sqr() < radius_sqr {
                            police_presence += 1;
                        }
                    }
                    for (&other_handle, other) in script_entities.vehicles.iter() {
                        if other_handle == vehicle_handle {
                            continue;
                        }
                        let base = other.base();
                        let model = base.physical.entity.model_index.max(0) as usize;
                        if !base.flags.is_law_enforcer
                            || !matches!(
                                model,
                                crate::modelinfo::ModelIndices::MI_CHOPPER
                                    | crate::modelinfo::ModelIndices::MI_PREDATOR
                                    | crate::modelinfo::ModelIndices::MI_POLICE
                                    | crate::modelinfo::ModelIndices::MI_ENFORCER
                            )
                            || matches!(
                                base.physical.entity.status,
                                eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED
                            )
                        {
                            continue;
                        }
                        let pos = base.physical.entity.get_position();
                        let delta = CVector::new(
                            pos.x - vehicle_matrix.position.x,
                            pos.y - vehicle_matrix.position.y,
                            pos.z - vehicle_matrix.position.z,
                        );
                        if delta.magnitude_sqr() < radius_sqr {
                            police_presence += 1;
                        }
                    }

                    let police_doesnt_care = (10..=25).contains(&ped_model_id);
                    let crime = CEventList::crime_for_event(event_type);
                    let (_, wanted_level_drop) =
                        file_loader.world.cull_zones.find_attributes_for_coors([
                            player_position.x,
                            player_position.y,
                            player_position.z,
                        ]);
                    let frenzy_ongoing = darkel
                        .as_deref()
                        .is_some_and(|darkel| darkel.frenzy_on_going());
                    if let Some(mut player) = world_players.player_mut(player_index) {
                        if police_presence != 0 {
                            player.wanted.register_crime_immediately(
                                crime,
                                player_position,
                                ped_handle,
                                police_doesnt_care,
                                frenzy_ongoing,
                                wanted_level_drop,
                                time_ms,
                            );
                            player.wanted.set_wanted_level_no_drop(1);
                        } else {
                            player.wanted.register_crime(
                                crime,
                                player_position,
                                ped_handle,
                                police_doesnt_care,
                                time_ms,
                            );
                        }
                    }
                }
            }
        }
    }

    // Players live outside ScriptEntityWorld::peds but share CWorld's ped
    // sectors. Run the same authored narrow phase for every active local slot.
    let player_probes = world_players
        .iter_slots()
        .filter_map(|(local_player, player)| {
            let player = player?;
            if !player.active_in_world
                || world_players.relation(local_player as i32).is_some()
                || !player.ped.physical.entity.flags.uses_collision
                || player.vitality.health <= 0.0
            {
                return None;
            }
            let raw_model = player.ped.physical.entity.model_index;
            let model_id = if raw_model <= 0 {
                MI_PLAYER
            } else {
                raw_model as usize
            };
            let model = file_loader.model_info.col_model(model_id)?;
            let mut collision_matrix = player.ped.physical.entity.matrix;
            let collision_probe = player.dynamic_collision_probe;
            if let Some(probe) = collision_probe {
                collision_matrix.position = probe;
            }
            Some((
                local_player,
                player.pool_handle,
                collision_matrix,
                model_id,
                model.bounding_sphere.radius,
                player.ped.current_phys_surface,
                player.ped.ped_stats,
                collision_probe.is_some(),
            ))
        })
        .collect::<Vec<_>>();
    'players: for (
        local_player,
        player_handle,
        mut player_matrix,
        player_model_id,
        player_radius,
        player_surface,
        player_stat_type,
        player_pose_was_rolled_back,
    ) in player_probes
    {
        let current_surface_vehicle_handle =
            player_surface.and_then(|entity| script_entities.vehicle_handle_for_entity(entity));
        let (current_surface_is_vehicle, current_surface_is_boat) = current_surface_vehicle_handle
            .and_then(|handle| script_entities.vehicle_get(handle))
            .map_or((false, false), |vehicle| {
                (true, matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }))
            });
        let Some(player_model) = file_loader.model_info.col_model(player_model_id) else {
            continue;
        };
        let mut collision_retry = 0_usize;
        // During ProcessCollision retries player/object response can change an
        // object's static bit and velocity, but the controller sweep only
        // depends on its matrix, collision enable, stuck/infinite state and
        // uproot limit. Those are stable for this retry sequence, so snapshot
        // once instead of cloning the entire object store up to five times.
        let retry_object_colliders = script_object_collision_snapshots(&script_entities.objects);

        'collision_attempts: loop {
            let player_x = player_matrix.position.x;
            let player_y = player_matrix.position.y;
            let mut responded = false;

            let first_vehicle = scratch
                .broad_phase
                .partition_point(|entry| entry.1 < player_x - player_radius - max_radius);
            for &(vehicle_handle, vehicle_x, vehicle_y, vehicle_radius) in
                scratch.broad_phase[first_vehicle..].iter()
            {
                if vehicle_x - player_x > player_radius + max_radius {
                    break;
                }
                let combined_radius = vehicle_radius + player_radius;
                if (vehicle_x - player_x).abs() > combined_radius
                    || (vehicle_y - player_y).abs() > combined_radius
                {
                    continue;
                }
                let Some((vehicle_matrix, vehicle_model_id)) = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                    .map(|vehicle| {
                        (
                            vehicle.base().physical.entity.matrix,
                            vehicle.base().physical.entity.model_index.max(0) as usize,
                        )
                    })
                else {
                    continue;
                };
                let Some(vehicle_model) = file_loader.model_info.col_model(vehicle_model_id) else {
                    continue;
                };
                let contacts = CCollision::process_col_models_spheres(
                    &player_matrix,
                    player_model,
                    &vehicle_matrix,
                    vehicle_model,
                );
                if contacts.is_empty() {
                    continue;
                }
                let vehicle_col_min = CVector::new(
                    vehicle_model.bounding_box.min.x,
                    vehicle_model.bounding_box.min.y,
                    vehicle_model.bounding_box.min.z,
                );
                let vehicle_col_max = CVector::new(
                    vehicle_model.bounding_box.max.x,
                    vehicle_model.bounding_box.max.y,
                    vehicle_model.bounding_box.max.z,
                );
                let collided = {
                    let Some(mut vehicle_entity) = script_entities.vehicle_get_mut(vehicle_handle)
                    else {
                        continue;
                    };
                    let mut vehicle = vehicle_entity.base_mut();
                    let Some(player) = world_players.player_mut(local_player as i32) else {
                        break 'collision_attempts;
                    };
                    with_player_native_collision_speed(&mut *player.ped, |ped| {
                        vehicle_ped_collision_step(
                            &mut vehicle,
                            None,
                            ped,
                            player.locomotion,
                            player.vitality,
                            player_matrix.position,
                            player_pose_was_rolled_back,
                            true,
                            &contacts,
                            vehicle_handle,
                            player_handle,
                            vehicle_col_min,
                            vehicle_col_max,
                            current_surface_vehicle_handle == Some(vehicle_handle),
                            current_surface_is_vehicle,
                            current_surface_is_boat,
                            ped_stats.GetPedStats(player_stat_type).m_defend_weakness,

                            time_step,
                            time_ms,
                            frame_counter,
                            &mut general,
                        )
                        .0
                    })
                };
                if collided {
                    responded = true;
                    break;
                }
            }

            if !responded {
                let first_ped = scratch
                    .ped_broad_phase
                    .partition_point(|entry| entry.1 < player_x - player_radius - ped_max_radius);
                for &(ped_handle, ped_x, ped_y, ped_radius) in
                    scratch.ped_broad_phase[first_ped..].iter()
                {
                    if ped_x - player_x > player_radius + ped_max_radius {
                        break;
                    }
                    let combined_radius = ped_radius + player_radius;
                    if (ped_x - player_x).abs() > combined_radius
                        || (ped_y - player_y).abs() > combined_radius
                    {
                        continue;
                    }
                    let Some((ped_matrix, ped_model_id)) =
                        script_entities.ped_get(ped_handle).map(|ped| {
                            (
                                ped.ped.physical.entity.matrix,
                                ped.ped.physical.entity.model_index.max(0) as usize,
                            )
                        })
                    else {
                        continue;
                    };
                    let Some(ped_model) = file_loader.model_info.col_model(ped_model_id) else {
                        continue;
                    };
                    let contacts = CCollision::process_col_models_spheres(
                        &player_matrix,
                        player_model,
                        &ped_matrix,
                        ped_model,
                    );
                    if contacts.is_empty() {
                        continue;
                    }

                    let collided = {
                        let Some(other) = script_entities.ped_get_mut(ped_handle) else {
                            continue;
                        };
                        let Some(player) = world_players.player_mut(local_player as i32) else {
                            break 'collision_attempts;
                        };
                        with_player_native_collision_speed(&mut *player.ped, |player_ped| {
                            player_ped_collision_step(
                                player_ped,
                                &mut *other.ped,
                                player_pose_was_rolled_back,
                                &contacts,
                                player_handle,
                                ped_handle,
                                &surface_table,
                                time_step,
                                time_ms,
                            )
                        })
                    };
                    if collided {
                        responded = true;
                        break;
                    }
                }
            }

            if !responded {
                // `CWorld` keeps objects in the same physical-sector pass as
                // vehicles. The derived broad phase above reflects the current
                // post-object-collision state for this frame and the object is
                // still revalidated before each narrow phase.
                let current_player_position = player_matrix.position;
                let first_object = scratch.player_object_broad_phase.partition_point(|entry| {
                    entry.1
                        < current_player_position.x
                            - player_model.bounding_sphere.radius
                            - player_object_max_radius
                });
                for &(object_handle, object_x, object_y, object_radius) in
                    scratch.player_object_broad_phase[first_object..].iter()
                {
                    if object_x - current_player_position.x
                        > player_model.bounding_sphere.radius + player_object_max_radius
                    {
                        break;
                    }
                    let combined_radius = player_model.bounding_sphere.radius + object_radius;
                    if (object_x - current_player_position.x).abs() > combined_radius
                        || (object_y - current_player_position.y).abs() > combined_radius
                    {
                        continue;
                    }
                    let Some((object_matrix, object_model_id)) = script_entities.objects.get(&script_entities.index, &object_handle)
                        .and_then(|entity| {
                            let object = &entity.object;
                            object.physical.entity.flags.uses_collision.then_some((
                                object.physical.entity.matrix,
                                object.physical.entity.model_index.max(0) as usize,
                            ))
                        })
                    else {
                        continue;
                    };
                    let Some(object_model) = file_loader.model_info.col_model(object_model_id)
                    else {
                        continue;
                    };
                    let contacts = CCollision::process_col_models_spheres(
                        &player_matrix,
                        player_model,
                        &object_matrix,
                        object_model,
                    );
                    if contacts.is_empty() {
                        continue;
                    }

                    let object_slot = CPoolHandle(object_handle).index();
                    let Some(object_bevy_entity) = script_entities.object_entity(object_handle)
                    else {
                        continue;
                    };
                    let collided = {
                        let Some(object_entity) = script_entities.objects.get_mut(&script_entities.index, &object_handle)
                        else {
                            continue;
                        };
                        let Some(player) = world_players.player_mut(local_player as i32) else {
                            break 'collision_attempts;
                        };
                        with_player_native_collision_speed(&mut *player.ped, |ped| {
                            player_object_collision_step(
                                ped,
                                &mut *object_entity.object,
                                object_bevy_entity,
                                player_pose_was_rolled_back,
                                &contacts,
                                player_handle,
                                object_handle,
                                &surface_table,
                                time_step,
                                time_ms,
                            )
                        })
                        .is_some()
                    };
                    if collided {
                        responded = true;
                        break;
                    }
                }
            }

            if !responded {
                if let Some(mut player) = world_players.player_mut(local_player as i32) {
                    player.dynamic_collision_probe = None;
                    if player_pose_was_rolled_back
                        && let Some(target) = player.dynamic_collision_target.take()
                    {
                        let saved = player.position();
                        player.set_position(target);
                        player.ped.physical.distance_travelled = CVector::new(
                            target.x - saved.x,
                            target.y - saved.y,
                            target.z - saved.z,
                        )
                        .magnitude();
                        player.ped.physical.entity.flags.is_in_safe_position = true;
                        player.ped.physical.entity.flags.is_stuck = false;
                    } else {
                        player.dynamic_collision_target = None;
                    }
                    if local_player == 0 {
                        bridge.player_position = player.position();
                        bridge.player_heading_degrees = player.heading_degrees();
                    }
                }
                continue 'players;
            }

            // If another physical moved into an already-live player pose,
            // vehicle_ped/player_object collision handling owns the immediate
            // separation. The retry sequence below is specifically the native
            // saved-matrix path used by a player's own subdivided movement.
            if !player_pose_was_rolled_back {
                if let Some(mut player) = world_players.player_mut(local_player as i32) {
                    player.dynamic_collision_probe = None;
                    player.dynamic_collision_target = None;
                    if local_player == 0 {
                        bridge.player_position = player.position();
                        bridge.player_heading_degrees = player.heading_degrees();
                    }
                }
                continue 'players;
            }

            if collision_retry == 5 {
                break 'collision_attempts;
            }
            collision_retry += 1;

            let (saved_position, retry_target) = {
                let Some(player) = world_players.player_mut(local_player as i32) else {
                    break 'collision_attempts;
                };
                let saved = player.position();
                let dt_seconds = time_step / GTA_PHYSICS_HZ;
                let target = CVector::new(
                    saved.x + player.ped.physical.move_speed.x * dt_seconds,
                    saved.y + player.ped.physical.move_speed.y * dt_seconds,
                    saved.z + player.ped.physical.move_speed.z * dt_seconds,
                );
                (saved, target)
            };
            let retry_motion = {
                let Some(player) = world_players.player(local_player as i32) else {
                    break 'collision_attempts;
                };
                resolve_player_dynamic_motion(
                    saved_position,
                    retry_target,
                    &player.ped,
                    &script_entities,
                    &retry_object_colliders,
                    &file_loader,
                    None,
                )
            };
            if let Some(probe) = retry_motion.collision_probe {
                let Some(mut player) = world_players.player_mut(local_player as i32) else {
                    break 'collision_attempts;
                };
                if let Some(support) = retry_motion.landing_support {
                    player.set_position(retry_motion.position);
                    player.ped.physical.move_speed.z = 0.0;
                    player.ped.flags.is_standing = true;
                    player.ped.flags.is_in_the_air = false;
                    update_player_current_phys_surface(&mut player.ped, None, Some(support), true);
                }
                player.dynamic_collision_probe = Some(probe);
                player.dynamic_collision_target = Some(CVector::new(
                    retry_target.x,
                    retry_target.y,
                    if retry_motion.landing_support.is_some() {
                        retry_motion.position.z
                    } else {
                        retry_target.z
                    },
                ));
                player_matrix = player.ped.physical.entity.matrix;
                player_matrix.position = probe;
                continue 'collision_attempts;
            }

            if let Some(mut player) = world_players.player_mut(local_player as i32) {
                player.set_position(retry_motion.position);
                if let Some(support) = retry_motion.landing_support {
                    player.ped.physical.move_speed.z = 0.0;
                    player.ped.flags.is_standing = true;
                    player.ped.flags.is_in_the_air = false;
                    update_player_current_phys_surface(&mut player.ped, None, Some(support), true);
                }
                player.dynamic_collision_probe = None;
                player.dynamic_collision_target = None;
                player.ped.physical.distance_travelled = CVector::new(
                    retry_target.x - saved_position.x,
                    retry_target.y - saved_position.y,
                    retry_target.z - saved_position.z,
                )
                .magnitude();
                player.ped.physical.entity.flags.is_in_safe_position = true;
                player.ped.physical.entity.flags.is_stuck = false;
                if local_player == 0 {
                    bridge.player_position = retry_target;
                    bridge.player_heading_degrees = player.heading_degrees();
                }
            }
            continue 'players;
        }

        // All six normal ProcessCollision attempts still responded. Mirror the
        // two CWorld ProcessShift passes before declaring the player stuck.
        // ProcessShift only moves the player here, so one fresh post-collision
        // object snapshot is valid for both native shift passes and rechecks.
        let shift_object_colliders = script_object_collision_snapshots(&script_entities.objects);
        for second_shift in [false, true] {
            let (saved_position, candidate, was_standing, base_matrix) = {
                let Some(player) = world_players.player(local_player as i32) else {
                    continue 'players;
                };
                let saved = player.position();
                let dt_seconds = time_step / GTA_PHYSICS_HZ;
                let integrated = CVector::new(
                    saved.x + player.ped.physical.move_speed.x * dt_seconds,
                    saved.y + player.ped.physical.move_speed.y * dt_seconds,
                    saved.z + player.ped.physical.move_speed.z * dt_seconds,
                );
                let candidate = player_physical_process_shift_target(
                    integrated,
                    &player.ped,
                    &script_entities,
                    &shift_object_colliders,
                    &file_loader,
                    second_shift,
                );
                (
                    saved,
                    candidate,
                    player.ped.flags.was_standing,
                    player.ped.physical.entity.matrix,
                )
            };

            let static_clear = player_ped_static_contacts_for_position(
                candidate,
                base_matrix,
                player_model_id,
                was_standing,
                &file_loader,
            )
            .is_empty();
            let dynamic_clear = world_players
                .player(local_player as i32)
                .is_some_and(|player| {
                    player_dynamic_motion_contact(
                        candidate,
                        &player.ped,
                        &script_entities,
                        &shift_object_colliders,
                        &file_loader,
                        None,
                    ) == PlayerDynamicMotionContact::None
                });
            if static_clear && dynamic_clear {
                if let Some(mut player) = world_players.player_mut(local_player as i32) {
                    player.set_position(candidate);
                    player.dynamic_collision_probe = None;
                    player.dynamic_collision_target = None;
                    player.ped.physical.distance_travelled = CVector::new(
                        candidate.x - saved_position.x,
                        candidate.y - saved_position.y,
                        candidate.z - saved_position.z,
                    )
                    .magnitude();
                    player.ped.physical.entity.flags.is_in_safe_position = true;
                    player.ped.physical.entity.flags.is_stuck = false;
                    if local_player == 0 {
                        bridge.player_position = candidate;
                        bridge.player_heading_degrees = player.heading_degrees();
                    }
                }
                continue 'players;
            }
        }

        if let Some(mut player) = world_players.player_mut(local_player as i32) {
            let saved_position = player.position();
            player.dynamic_collision_probe = None;
            player.dynamic_collision_target = None;
            player.ped.physical.distance_travelled = 0.0;
            player.ped.physical.entity.flags.is_in_safe_position = false;
            player.ped.physical.entity.flags.is_stuck = true;
            if player.ped.physical.entity.status == eEntityStatus::STATUS_PLAYER {
                player.ped.physical.move_speed.x *= 0.3;
                player.ped.physical.move_speed.y *= 0.3;
                player.ped.physical.move_speed.z *= 0.3;
                let dt_seconds = time_step / GTA_PHYSICS_HZ;
                player.set_position(CVector::new(
                    saved_position.x + player.ped.physical.move_speed.x * dt_seconds,
                    saved_position.y + player.ped.physical.move_speed.y * dt_seconds,
                    saved_position.z + player.ped.physical.move_speed.z * dt_seconds,
                ));
            }
            if local_player == 0 {
                bridge.player_position = player.position();
                bridge.player_heading_degrees = player.heading_degrees();
            }
        }
    }

    // CPhysical sector membership stores Entity identities. Bounds and poses
    // come from the live owners at each contact, including earlier responses.
    scratch.pair_sources.clear();
    scratch.pair_sources.extend(vehicle_spatial.grid.entities());
    // TODO(parity): CWorld::Process moving-list/sector insertion ordering.
    scratch.pair_sources.sort_by_key(|&entity| script_entities.index.vehicle_handle_for_entity(entity)
        .map(|handle| CPoolHandle(handle).index()));
    scratch.pair_visited.clear();
    let CollisionScratch {
        pair_sources,
        pair_candidates,
        pair_refresh,
        pair_visited,
        pair_queued,
        dynamic_collision_rollbacks,
        ..
    } = &mut *scratch;
    for &entity_a in pair_sources.iter() {
        let Some(handle_a) = script_entities.index.vehicle_handle_for_entity(entity_a) else { continue; };
        let Some(source) = script_entities.vehicle_get(handle_a) else { continue; };
        let source = source.base();
        let Some(model) = file_loader.model_info.col_model(source.physical.entity.model_index.max(0) as usize) else {
            warn_once!("vehicle_pair_waiting_original_col");
            continue;
        };
        let previous = script_entities.vehicles.collision_pose(&script_entities.index, handle_a);
        let bounds = super::VehiclePair::candidate_bounds(&source.physical, model, previous,
            dynamic_collision_rollbacks.contains(&handle_a), time_step);
        if !vehicle_spatial.grid.candidates_rect(bounds, pair_candidates) {
            warn_once!("vehicle_pair_invalid_original_bounds");
            continue;
        }
        pair_queued.clear();
        pair_queued.extend(pair_candidates.iter().copied());
        let mut cursor = 0;
        while cursor < pair_candidates.len() {
            let entity_b = pair_candidates[cursor];
            cursor += 1;
            let pair = if entity_a.to_bits() <= entity_b.to_bits() { (entity_a, entity_b) } else { (entity_b, entity_a) };
            if entity_a == entity_b || pair_visited.contains(&pair) {
                continue;
            }
            let Some(handle_b) = script_entities.index.vehicle_handle_for_entity(entity_b) else { continue; };
            vehicle_spatial.candidate_visits += 1;
            let Some((model_a_id, model_b_id)) = script_entities.vehicles.get(&script_entities.index, &handle_a)
                .zip(script_entities.vehicle_get(handle_b))
                .and_then(|(vehicle_a, vehicle_b)| {
                    let base_a = vehicle_a.base();
                    let base_b = vehicle_b.base();
                    if !base_a.physical.entity.flags.uses_collision
                        || !base_b.physical.entity.flags.uses_collision
                    {
                        return None;
                    }
                    Some((
                        base_a.physical.entity.model_index.max(0) as usize,
                        base_b.physical.entity.model_index.max(0) as usize,
                    ))
                })
            else {
                continue;
            };
            let (Some(model_a), Some(model_b)) = (
                file_loader.model_info.col_model(model_a_id),
                file_loader.model_info.col_model(model_b_id),
            ) else {
                warn_once!("vehicle_pair_waiting_original_col");
                continue;
            };
            // Each source probes its own native partial poses. A miss must not
            // suppress the counterpart's later source turn.
            let previous_a = script_entities.vehicles.collision_pose(&script_entities.index, handle_a);
            let previous_b = script_entities.vehicles.collision_pose(&script_entities.index, handle_b);
            let probe = script_entities.vehicle_get(handle_a).zip(script_entities.vehicle_get(handle_b))
                .map(|(a, b)| super::VehiclePair::first_contact(&a.base().physical, model_a,
                    previous_a, dynamic_collision_rollbacks.contains(&handle_a),
                    &b.base().physical, model_b, time_step));
            let Some(probe) = probe else { continue; };
            if probe.contacts_seen && probe.contact.is_none() {
                let [Some(mut a), Some(mut b)] = script_entities.vehicles.get_disjoint_mut(
                    &script_entities.index, [&handle_a, &handle_b]) else { continue; };
                a.base_mut().physical.add_collision_record(EntityReference::Vehicle(handle_b), time_ms);
                b.base_mut().physical.add_collision_record(EntityReference::Vehicle(handle_a), time_ms);
            }
            let Some(contact) = probe.contact else { continue; };
            if contact.time_step < time_step { pair_diagnostics.partial_contacts += 1; }
            if contact.endpoint_missed {
                pair_diagnostics.contacts_missed_by_endpoint += 1;
                pair_diagnostics.last_endpoint_miss_pair = Some((handle_a, handle_b));
            }
            pair_visited.insert(pair);
            let collision_time_step = contact.time_step;
            let contacts = contact.contacts;

            let [Some(mut vehicle_a), Some(mut vehicle_b)] = script_entities.vehicles.get_disjoint_mut(&script_entities.index, [&handle_a, &handle_b])
            else {
                continue;
            };
            let mut base_a = vehicle_a.base_mut();
            let mut base_b = vehicle_b.base_mut();
            base_a.physical.entity.set_matrix(contact.matrix);
            // ProcessEntityCollision publishes contacts before any response;
            // resting contacts must remain visible to SCM and physical state.
            base_a.physical.add_collision_record(EntityReference::Vehicle(handle_b), time_ms);
            base_b.physical.add_collision_record(EntityReference::Vehicle(handle_a), time_ms);
            let mut strongest_a: Option<(f32, u8, CVector)> = None;
            let mut strongest_b: Option<(f32, u8, CVector)> = None;
            // Match ProcessCollisionSectorList's contact-friction state
            // machine. If exactly one body already established contact this
            // frame, native temporarily removes that body's accumulated
            // friction, lets this pair try to establish a shared contact, and
            // restores the old friction only when the pair added none.
            let a_had_contacted = base_a.physical.entity.flags.has_contacted;
            let b_had_contacted = base_b.physical.entity.flags.has_contacted;
            let apply_pair_friction = !(a_had_contacted && b_had_contacted);
            let saved_a_friction = if apply_pair_friction && a_had_contacted {
                let saved = (base_a.physical.move_friction, base_a.physical.turn_friction);
                base_a.physical.move_friction = CVector::ZERO;
                base_a.physical.turn_friction = CVector::ZERO;
                base_a.physical.entity.flags.has_contacted = false;
                Some(saved)
            } else {
                None
            };
            let saved_b_friction = if apply_pair_friction && b_had_contacted {
                let saved = (base_b.physical.move_friction, base_b.physical.turn_friction);
                base_b.physical.move_friction = CVector::ZERO;
                base_b.physical.turn_friction = CVector::ZERO;
                base_b.physical.entity.flags.has_contacted = false;
                Some(saved)
            } else {
                None
            };
            for point in &contacts {
                let response = base_a.physical.apply_collision_dynamic(
                    &mut base_b.physical,
                    point,
                    CPhysicalCollisionOptions {
                        time_step: collision_time_step,
                        ..Default::default()
                    },
                );
                if !response.applied {
                    continue;
                }
                let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
                if strongest_a.is_none_or(|(impulse, _, _)| response.impulse_a > impulse) {
                    strongest_a = Some((response.impulse_a, point.piece_a, normal));
                }
                if strongest_b.is_none_or(|(impulse, _, _)| response.impulse_b > impulse) {
                    strongest_b = Some((response.impulse_b, point.piece_b, normal));
                }

                if let Some(dmaudio) = dmaudio.as_deref_mut() {
                    let move_delta = CVector::new(
                        base_b.physical.move_speed.x - base_a.physical.move_speed.x,
                        base_b.physical.move_speed.y - base_a.physical.move_speed.y,
                        base_b.physical.move_speed.z - base_a.physical.move_speed.z,
                    );
                    let turn_delta = CVector::new(
                        base_b.physical.turn_speed.x - base_a.physical.turn_speed.x,
                        base_b.physical.turn_speed.y - base_a.physical.turn_speed.y,
                        base_b.physical.turn_speed.z - base_a.physical.turn_speed.z,
                    );
                    dmaudio.ReportCollision(
                        Some(handle_a),
                        Some(handle_b),
                        point.surface_a,
                        point.surface_b,
                        response.impulse_a,
                        move_delta.magnitude_sqr().max(turn_delta.magnitude_sqr()),
                        base_a.physical.entity.get_position(),
                        base_b.physical.entity.get_position(),
                        false,
                        false,
                        camera
                            .as_ref()
                            .map_or(file_loader.renderer.camera_position, |c| c.position),
                    );
                }

                if apply_pair_friction {
                    let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                        &surface_table.data,
                        point.surface_a,
                        point.surface_b,
                    ) / contacts.len() as f32;
                    if base_a
                        .physical
                        .apply_collision_friction_between(
                            &mut base_b.physical,
                            adhesion,
                            point,
                            collision_time_step,
                        )
                        .applied
                    {
                        base_a.physical.entity.flags.has_contacted = true;
                        base_b.physical.entity.flags.has_contacted = true;
                    }
                }
            }
            if let Some((move_friction, turn_friction)) = saved_a_friction
                && !base_a.physical.entity.flags.has_contacted
            {
                base_a.physical.entity.flags.has_contacted = true;
                base_a.physical.move_friction = move_friction;
                base_a.physical.turn_friction = turn_friction;
            }
            if let Some((move_friction, turn_friction)) = saved_b_friction
                && !base_b.physical.entity.flags.has_contacted
            {
                base_b.physical.entity.flags.has_contacted = true;
                base_b.physical.move_friction = move_friction;
                base_b.physical.turn_friction = turn_friction;
            }
            // ProcessCollision rejects integration back to savedMatrix.
            // ProcessShift then retries from that matrix with response speed.
            let rolled_back_a = base_a.physical.entity.status != eEntityStatus::STATUS_SIMPLE
                && strongest_a.is_some()
                && dynamic_collision_rollbacks.insert(handle_a);
            if rolled_back_a && let Some(matrix) = previous_a {
                base_a.physical.entity.set_matrix(matrix);
                base_a.physical.apply_move_speed(time_step);
                base_a.physical.apply_turn_speed(time_step);
                base_a.physical.entity.matrix.reorthogonalise();
            }
            let rolled_back_b = base_b.physical.entity.status != eEntityStatus::STATUS_SIMPLE
                && strongest_b.is_some()
                && dynamic_collision_rollbacks.insert(handle_b);
            if rolled_back_b && let Some(matrix) = previous_b {
                base_b.physical.entity.set_matrix(matrix);
                base_b.physical.apply_move_speed(time_step);
                base_b.physical.apply_turn_speed(time_step);
                base_b.physical.entity.matrix.reorthogonalise();
            }

            // Re3 runs ProcessShift per physical, not as one doubled reciprocal
            // displacement. If the first (1.2) shift still collides, it is
            // rejected and the second pass (1.5) starts from that saved pose.
            // `ProcessShiftSectorList` skips a vehicle's shift when that
            // vehicle already hit a static wall this frame (`bHasHitWall`),
            // otherwise the static response and the vehicle manifold would
            // fight each other and visibly vibrate at parked-car contacts.
            let shift_a_allowed = !base_a.physical.entity.flags.has_hit_wall;
            let shift_b_allowed = !base_b.physical.entity.flags.has_hit_wall;
            // Each physical starts ProcessShift with a fresh scan code. B's
            // own pass reads the manifold after A's sequential displacement.
            let before_shift_a = base_a.physical.entity.matrix;
            if shift_a_allowed {
                let contacts_a = CCollision::process_col_models_spheres(
                    &before_shift_a,
                    model_a,
                    &base_b.physical.entity.matrix,
                    model_b,
                );
                apply_vehicle_pair_process_shift(&mut base_a, &base_b, &contacts_a, false);
                let still_a = CCollision::process_col_models_spheres(
                    &base_a.physical.entity.matrix,
                    model_a,
                    &base_b.physical.entity.matrix,
                    model_b,
                );
                if vehicle_pair_contacts_require_response(&base_a, &base_b, &still_a, time_step) {
                    base_a.physical.entity.set_matrix(before_shift_a);
                    let restored_contacts = CCollision::process_col_models_spheres(
                        &base_a.physical.entity.matrix, model_a, &base_b.physical.entity.matrix, model_b);
                    apply_vehicle_pair_process_shift(&mut base_a, &base_b, &restored_contacts, true);
                }
            }

            let before_shift_b = base_b.physical.entity.matrix;
            if shift_b_allowed {
                let contacts_b = CCollision::process_col_models_spheres(
                    &before_shift_b,
                    model_b,
                    &base_a.physical.entity.matrix,
                    model_a,
                );
                apply_vehicle_pair_process_shift(&mut base_b, &base_a, &contacts_b, false);
                let still_b = CCollision::process_col_models_spheres(
                    &base_b.physical.entity.matrix,
                    model_b,
                    &base_a.physical.entity.matrix,
                    model_a,
                );
                if vehicle_pair_contacts_require_response(&base_b, &base_a, &still_b, time_step) {
                    base_b.physical.entity.set_matrix(before_shift_b);
                    let restored_contacts = CCollision::process_col_models_spheres(
                        &base_b.physical.entity.matrix, model_b, &base_a.physical.entity.matrix, model_a);
                    apply_vehicle_pair_process_shift(&mut base_b, &base_a, &restored_contacts, true);
                }
            }
            base_a.physical.entity.flags.is_stuck = false;
            base_b.physical.entity.flags.is_stuck = false;
            base_a.physical.entity.flags.is_in_safe_position = true;
            base_b.physical.entity.flags.is_in_safe_position = true;

            base_a.physical.static_frames = 0;
            base_b.physical.static_frames = 0;

            if base_a.physical.entity.status == eEntityStatus::STATUS_SIMPLE {
                base_a.set_status(eEntityStatus::STATUS_PHYSICS);
                base_a.switch_to_real_physics(time_ms);
            }
            if base_b.physical.entity.status == eEntityStatus::STATUS_SIMPLE {
                base_b.set_status(eEntityStatus::STATUS_PHYSICS);
                base_b.switch_to_real_physics(time_ms);
            }

            if let Some((impulse, piece_id, normal)) = strongest_a {
                base_a.physical.record_collision_damage(
                    piece_id as u16,
                    impulse,
                    Some(EntityReference::Vehicle(handle_b)),
                    normal,
                );
            }
            if let Some((impulse, piece_id, normal)) = strongest_b {
                base_b.physical.record_collision_damage(
                    piece_id as u16,
                    impulse,
                    Some(EntityReference::Vehicle(handle_a)),
                    normal,
                );
            }
            vehicle_spatial.refresh(entity_a, &base_a.core, &file_loader);
            vehicle_spatial.refresh(entity_b, &base_b.core, &file_loader);
            if vehicle_spatial.grid.candidates_rect(base_a.physical.get_bound_rect(model_a), pair_refresh) {
                for &entity in pair_refresh.iter() {
                    if pair_queued.insert(entity) { pair_candidates.push(entity); }
                }
            }
        }
    }

    // CPed::ProcessControl keeps the seated player matrix attached to the
    // post-collision vehicle matrix. Do this after every response so bridge,
    // streaming and the camera never observe the rejected pre-collision step.
    for local_player in 0..world_players.slot_count() {
        let player_vehicle = world_players
            .relation(local_player as i32)
            .and_then(|relation| {
                let handle = script_entities.vehicle_handle_for_entity(relation.vehicle)?;
                let vehicle = script_entities.vehicle_get(handle)?;
                let base = vehicle.base();
                let seat = player_seat_index(relation.seat);
                let is_boat = matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. });
                Some((
                    ped_seat_world_position(&base, seat, is_boat, &file_loader),
                    base.physical.entity.get_heading_degrees(),
                ))
            });
        if let (Some(mut player), Some((position, heading))) = (
            world_players.player_mut(local_player as i32),
            player_vehicle,
        ) {
            player.set_position(position);
            player.set_heading_degrees(heading);
            if local_player == 0 {
                bridge.player_position = position;
                bridge.player_heading_degrees = heading;
            }
        }
    }
}

// FAZ3b: moved to control/CarRails.rs verbatim (7 items): struct TrafficVehicleScan, struct TrafficPedScan, fn modify_speed_on_rails, fn rails_update_car_on_rails, fn slow_car_on_rails_down_for_traffic_and_lights, fn find_maximum_speed_for_this_car_in_traffic, fn drag_car_to_point


/// Dynamic half of re3 `CPed::ProcessEntityCollision`'s vertical support
/// query. The native line starts at the ped root, never above it, so a roof or
/// ledge above the character cannot become ground merely because the body
/// sphere brushed its side. A candidate must also be within one ped step of
/// the feet; standing on cars/objects remains supported.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScriptObjectCollisionSnapshot {
    pub entity: Entity,
    pub matrix: CMatrix,
    pub model_index: usize,
    pub uses_collision: bool,
    pub is_static: bool,
    pub is_stuck: bool,
    pub infinite_mass: bool,
    pub has_been_damaged: bool,
    pub uproot_limit: f32,
}

pub(crate) fn script_object_collision_snapshots(
    objects: &ScriptObjectStore,
) -> Vec<ScriptObjectCollisionSnapshot> {
    objects
        .entities
        .iter()
        .map(|(bevy_entity, _, object)| ScriptObjectCollisionSnapshot {
            entity: bevy_entity,
            matrix: object.physical.entity.matrix,
            model_index: object.physical.entity.model_index.max(0) as usize,
            uses_collision: object.physical.entity.flags.uses_collision,
            is_static: object.physical.entity.flags.is_static,
            is_stuck: object.physical.entity.flags.is_stuck,
            infinite_mass: object.physical.flags.infinite_mass,
            has_been_damaged: object.flags.has_been_damaged,
            uproot_limit: object.uproot_limit,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerDynamicMotionContact {
    None,
    /// Authored ped/vehicle COL manifold. Keep the overlapping sample so the
    /// later authoritative ECS collision pass can apply the native two-body
    /// impulse and ProcessShift-style separation.
    VehicleContact,
    PedContact,
    ImmovableObject,
    PushableObject,
}

#[derive(Default)]
pub(crate) struct PlayerDynamicMotionCandidates {
    pub(crate) vehicles: Vec<i32>,
    pub(crate) peds: Vec<i32>,
    pub(crate) objects: Vec<usize>,
}

pub(crate) fn player_object_blocks_motion(object: &ScriptObjectCollisionSnapshot) -> bool {
    // `CPhysical::ProcessCollisionSectorList` deliberately routes a ped
    // against a positive-uproot-limit object through `ApplyCollisionAlt`, the
    // same immovable branch used for stuck/infinite-mass entities. Objects
    // without those constraints must be allowed to reach the two-body
    // response so the ped can transfer momentum instead of treating every
    // CObject as a wall in the controller sweep.
    object.infinite_mass || object.is_stuck || object.uproot_limit > 0.0
}

pub(crate) fn col_bound_spheres_overlap(
    matrix_a: &CMatrix,
    col_a: &crate::collision::ColModel::CColModel,
    matrix_b: &CMatrix,
    col_b: &crate::collision::ColModel::CColModel,
) -> bool {
    let center_a = matrix_a.transform_point(CVector::new(
        col_a.bounding_sphere.center.x,
        col_a.bounding_sphere.center.y,
        col_a.bounding_sphere.center.z,
    ));
    let center_b = matrix_b.transform_point(CVector::new(
        col_b.bounding_sphere.center.x,
        col_b.bounding_sphere.center.y,
        col_b.bounding_sphere.center.z,
    ));
    let radius = col_a.bounding_sphere.radius + col_b.bounding_sphere.radius;
    let delta = CVector::new(
        center_a.x - center_b.x,
        center_a.y - center_b.y,
        center_a.z - center_b.z,
    );
    delta.magnitude_sqr() <= radius * radius
}

pub(crate) fn swept_col_bound_spheres_maybe_overlap(
    start: CVector,
    end: CVector,
    player_matrix: &CMatrix,
    player_col: &crate::collision::ColModel::CColModel,
    other_matrix: &CMatrix,
    other_col: &crate::collision::ColModel::CColModel,
) -> bool {
    let player_offset = player_matrix.multiply3x3(CVector::new(
        player_col.bounding_sphere.center.x,
        player_col.bounding_sphere.center.y,
        player_col.bounding_sphere.center.z,
    ));
    let start_center = CVector::new(
        start.x + player_offset.x,
        start.y + player_offset.y,
        start.z + player_offset.z,
    );
    let end_center = CVector::new(
        end.x + player_offset.x,
        end.y + player_offset.y,
        end.z + player_offset.z,
    );
    let other_center = other_matrix.transform_point(CVector::new(
        other_col.bounding_sphere.center.x,
        other_col.bounding_sphere.center.y,
        other_col.bounding_sphere.center.z,
    ));
    let radius = player_col.bounding_sphere.radius + other_col.bounding_sphere.radius;

    other_center.x >= start_center.x.min(end_center.x) - radius
        && other_center.x <= start_center.x.max(end_center.x) + radius
        && other_center.y >= start_center.y.min(end_center.y) - radius
        && other_center.y <= start_center.y.max(end_center.y) + radius
        && other_center.z >= start_center.z.min(end_center.z) - radius
        && other_center.z <= start_center.z.max(end_center.z) + radius
}

pub(crate) fn player_dynamic_motion_candidates(
    start: CVector,
    end: CVector,
    ped: &CPed,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
    excluded_vehicle: Option<i32>,
) -> PlayerDynamicMotionCandidates {
    let Some(player_col) = player_collision_col_model(ped, file_loader) else {
        return PlayerDynamicMotionCandidates::default();
    };
    let player_matrix = ped.physical.entity.matrix;
    let mut candidates = PlayerDynamicMotionCandidates::default();

    for (&handle, vehicle) in entities.vehicles.iter() {
        if Some(handle) == excluded_vehicle {
            continue;
        }
        let base = vehicle.base();
        if !base.physical.entity.flags.uses_collision {
            continue;
        }
        let Some(col) = file_loader
            .model_info
            .col_model(base.physical.entity.model_index.max(0) as usize)
        else {
            continue;
        };
        if swept_col_bound_spheres_maybe_overlap(
            start,
            end,
            &player_matrix,
            player_col,
            &base.physical.entity.matrix,
            col,
        ) {
            candidates.vehicles.push(handle);
        }
    }

    for (&handle, other) in entities.peds.iter() {
        if entities.relation_for_char(handle).is_some()
            || !other.ped.physical.entity.flags.uses_collision
        {
            continue;
        }
        let Some(col) = file_loader
            .model_info
            .col_model(other.ped.physical.entity.model_index.max(0) as usize)
        else {
            continue;
        };
        if swept_col_bound_spheres_maybe_overlap(
            start,
            end,
            &player_matrix,
            player_col,
            &other.ped.physical.entity.matrix,
            col,
        ) {
            candidates.peds.push(handle);
        }
    }

    for (index, object) in object_colliders.iter().enumerate() {
        if !object.uses_collision {
            continue;
        }
        let Some(col) = file_loader.model_info.col_model(object.model_index) else {
            continue;
        };
        if swept_col_bound_spheres_maybe_overlap(
            start,
            end,
            &player_matrix,
            player_col,
            &object.matrix,
            col,
        ) {
            candidates.objects.push(index);
        }
    }

    candidates
}

pub(crate) fn player_object_position_contact(
    position: CVector,
    ped: &CPed,
    object: &ScriptObjectCollisionSnapshot,
    file_loader: &CFileLoader,
) -> bool {
    if !object.uses_collision {
        return false;
    }
    let Some(object_col) = file_loader.model_info.col_model(object.model_index) else {
        return false;
    };
    let Some(player_col) = player_collision_col_model(ped, file_loader) else {
        return false;
    };
    let mut player_matrix = ped.physical.entity.matrix;
    player_matrix.position = position;
    if !col_bound_spheres_overlap(&player_matrix, player_col, &object.matrix, object_col) {
        return false;
    }
    CCollision::test_col_models_spheres(&player_matrix, player_col, &object.matrix, object_col)
}

pub(crate) fn player_dynamic_motion_contact_from_candidates(
    position: CVector,
    ped: &CPed,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
    candidates: &PlayerDynamicMotionCandidates,
) -> PlayerDynamicMotionContact {
    let Some(player_col) = player_collision_col_model(ped, file_loader) else {
        return PlayerDynamicMotionContact::None;
    };
    let mut test_matrix = ped.physical.entity.matrix;
    test_matrix.position = position;
    for &handle in &candidates.vehicles {
        let Some(vehicle) = entities.vehicles.get(&entities.index, &handle) else {
            continue;
        };
        let base = vehicle.base();
        if !base.physical.entity.flags.uses_collision {
            continue;
        }
        let Some(col) = file_loader
            .model_info
            .col_model(base.physical.entity.model_index.max(0) as usize)
        else {
            continue;
        };
        if !col_bound_spheres_overlap(&test_matrix, player_col, &base.physical.entity.matrix, col) {
            continue;
        }
        let local = base.physical.entity.matrix.multiply_inverse_point(position);
        let ped_feet_local_z = local.z - crate::peds::PedPlacement::PED_FEET_OFFSET;
        if ped_feet_local_z > col.bounding_box.max.z + 0.1 {
            continue;
        }
        if CCollision::test_col_models_spheres(
            &test_matrix,
            player_col,
            &base.physical.entity.matrix,
            col,
        ) {
            return PlayerDynamicMotionContact::VehicleContact;
        }
    }

    for &handle in &candidates.peds {
        let Some(other) = entities.peds.get(&entities.index, &handle) else {
            continue;
        };
        if entities.relation_for_char(handle).is_some()
            || !other.ped.physical.entity.flags.uses_collision
        {
            continue;
        }
        let Some(col) = file_loader
            .model_info
            .col_model(other.ped.physical.entity.model_index.max(0) as usize)
        else {
            continue;
        };
        if !col_bound_spheres_overlap(
            &test_matrix,
            player_col,
            &other.ped.physical.entity.matrix,
            col,
        ) {
            continue;
        }
        if CCollision::test_col_models_spheres(
            &test_matrix,
            player_col,
            &other.ped.physical.entity.matrix,
            col,
        ) {
            return PlayerDynamicMotionContact::PedContact;
        }
    }

    let mut pushable_contact = false;
    for &index in &candidates.objects {
        let Some(object) = object_colliders.get(index) else {
            continue;
        };
        if !player_object_position_contact(position, ped, object, file_loader) {
            continue;
        }
        if player_object_blocks_motion(object) {
            return PlayerDynamicMotionContact::ImmovableObject;
        }
        pushable_contact = true;
    }
    if pushable_contact {
        PlayerDynamicMotionContact::PushableObject
    } else {
        PlayerDynamicMotionContact::None
    }
}

pub(crate) fn player_dynamic_motion_contact(
    position: CVector,
    ped: &CPed,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
    excluded_vehicle: Option<i32>,
) -> PlayerDynamicMotionContact {
    let candidates = player_dynamic_motion_candidates(
        position,
        position,
        ped,
        entities,
        object_colliders,
        file_loader,
        excluded_vehicle,
    );
    player_dynamic_motion_contact_from_candidates(
        position,
        ped,
        entities,
        object_colliders,
        file_loader,
        &candidates,
    )
}

pub(crate) fn accumulate_player_dynamic_process_shift(
    shift: &mut CVector,
    contacts: &[CColPoint],
    second_shift: bool,
    boat: bool,
) {
    if contacts.is_empty() {
        return;
    }
    let manifold_scale = if second_shift { 1.5 } else { 1.2 } / contacts.len() as f32;
    for point in contacts {
        shift.x += point.normal.x * point.depth * manifold_scale;
        shift.y += point.normal.y * point.depth * manifold_scale;
        shift.z += point.normal.z * point.depth * manifold_scale;
    }
    let Some(deepest) = contacts
        .iter()
        .max_by(|left, right| left.depth.total_cmp(&right.depth))
    else {
        return;
    };
    if boat {
        // `ProcessShiftSectorList` replaces the generic 0.2/0.4 deepest term
        // for ped -> boat with a horizontal escape vector amplified by the
        // contact's vertical component.
        let horizontal = deepest.normal.x.hypot(deepest.normal.y);
        if horizontal > 1.0e-6 {
            let f = deepest.normal.z.abs().min(0.9);
            let amount = deepest.depth / (1.0 - f);
            shift.x += deepest.normal.x / horizontal * amount;
            shift.y += deepest.normal.y / horizontal * amount;
        }
    } else {
        let deepest_scale = if second_shift { 0.4 } else { 0.2 };
        shift.x += deepest.normal.x * deepest.depth * deepest_scale;
        shift.y += deepest.normal.y * deepest.depth * deepest_scale;
        shift.z += deepest.normal.z * deepest.depth * deepest_scale;
    }
}

/// Rebuild the ProcessShift manifold at the position produced by the current
/// collision-adjusted velocity. Native does not reuse the last collision
/// manifold: it scans buildings, objects, vehicles and peds again and sums
/// every eligible separation before the collision recheck.
pub(crate) fn player_physical_process_shift_target(
    integrated: CVector,
    ped: &CPed,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
    second_shift: bool,
) -> CVector {
    let raw_model = ped.physical.entity.model_index;
    let model_index = if raw_model <= 0 {
        MI_PLAYER
    } else {
        raw_model as usize
    };
    let mut shift = CVector::ZERO;

    if let Some(static_shifted) = player_ped_static_process_shift(
        integrated,
        ped.physical.entity.matrix,
        model_index,
        ped.flags.was_standing,
        file_loader,
        second_shift,
    ) {
        shift.x += static_shifted.x - integrated.x;
        shift.y += static_shifted.y - integrated.y;
        shift.z += static_shifted.z - integrated.z;
    }

    let Some(player_col) = player_collision_col_model(ped, file_loader) else {
        return CVector::new(
            integrated.x + shift.x,
            integrated.y + shift.y,
            integrated.z + shift.z,
        );
    };
    let mut player_matrix = ped.physical.entity.matrix;
    player_matrix.position = integrated;

    // Vehicles and peds are not `canshift` entities in native. Once a wall
    // contact exists, ProcessShift skips them and only immovable/static world
    // geometry may push the player out.
    if !ped.physical.entity.flags.has_hit_wall {
        for vehicle in entities.vehicles.values() {
            let base = vehicle.base();
            if !base.physical.entity.flags.uses_collision {
                continue;
            }
            let Some(col) = file_loader
                .model_info
                .col_model(base.physical.entity.model_index.max(0) as usize)
            else {
                continue;
            };
            if !col_bound_spheres_overlap(
                &player_matrix,
                player_col,
                &base.physical.entity.matrix,
                col,
            ) {
                continue;
            }
            let contacts = CCollision::process_col_models_spheres(
                &player_matrix,
                player_col,
                &base.physical.entity.matrix,
                col,
            );
            accumulate_player_dynamic_process_shift(
                &mut shift,
                &contacts,
                second_shift,
                matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }),
            );
        }
        for (&handle, other) in entities.peds.iter() {
            if entities.relation_for_char(handle).is_some()
                || !other.ped.physical.entity.flags.uses_collision
                {
                continue;
            }
            let Some(col) = file_loader
                .model_info
                .col_model(other.ped.physical.entity.model_index.max(0) as usize)
            else {
                continue;
            };
            if !col_bound_spheres_overlap(
                &player_matrix,
                player_col,
                &other.ped.physical.entity.matrix,
                col,
            ) {
                continue;
            }
            let contacts = CCollision::process_col_models_spheres(
                &player_matrix,
                player_col,
                &other.ped.physical.entity.matrix,
                col,
            );
            accumulate_player_dynamic_process_shift(&mut shift, &contacts, second_shift, false);
        }
    }

    for object in object_colliders {
        if !object.uses_collision {
            continue;
        }
        let can_shift = object.infinite_mass || (object.is_static && !object.has_been_damaged);
        if ped.physical.entity.flags.has_hit_wall && !can_shift {
            continue;
        }
        let Some(col) = file_loader.model_info.col_model(object.model_index) else {
            continue;
        };
        if !col_bound_spheres_overlap(&player_matrix, player_col, &object.matrix, col) {
            continue;
        }
        let contacts =
            CCollision::process_col_models_spheres(&player_matrix, player_col, &object.matrix, col);
        accumulate_player_dynamic_process_shift(&mut shift, &contacts, second_shift, false);
    }

    CVector::new(
        integrated.x + shift.x,
        integrated.y + shift.y,
        integrated.z + shift.z,
    )
}

fn player_collision_col_model<'a>(
    ped: &CPed,
    file_loader: &'a CFileLoader,
) -> Option<&'a CColModel> {
    let raw_model = ped.physical.entity.model_index;
    let Ok(model_index) = usize::try_from(raw_model) else {
        bevy::log::warn_once!(raw_model, "ped_physical_waiting_original_col_model");
        return None;
    };
    // CPed::ProcessEntityCollision always uses GetColModel(GetModelIndex()).
    // GroundHit is a weapon hit model, including while this actor is dying.
    let col = file_loader.model_info.col_model(model_index);
    if col.is_none() {
        bevy::log::warn_once!(model_index, "ped_physical_waiting_original_col_model");
    }
    col
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlayerDynamicGroundSupport {
    pub z: f32,
    pub entity: Entity,
    pub offset_from_surface: CVector,
    pub surface: u8,
    pub normal: CVector,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlayerStaticGroundSupport {
    pub z: f32,
    pub surface: u8,
    pub normal: CVector,
}

/// Sweep a player movement segment through dynamic bodies before accepting
/// the final pose.  re3's `CPhysical::ProcessCollision` subdivides player
/// motion at 0.2 world units (and always uses at least two samples for a
/// player), so testing only the destination lets a fast walk step tunnel
/// through a thin vehicle COL.  The caller still performs the authored static
/// COL slide after this conservative dynamic sweep.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlayerDynamicMotionResult {
    pub position: CVector,
    pub collision_probe: Option<CVector>,
    /// `CPed::ProcessEntityCollision` found below-torso dynamic support during
    /// an intermediate player subdivision and zeroed the native Z speed.
    pub landing_support: Option<PlayerDynamicGroundSupport>,
}

pub(crate) fn resolve_player_dynamic_motion(
    start: CVector,
    end: CVector,
    ped: &CPed,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
    excluded_vehicle: Option<i32>,
) -> PlayerDynamicMotionResult {
    let delta = CVector::new(end.x - start.x, end.y - start.y, end.z - start.z);
    let distance = delta.magnitude();
    let steps = if distance <= f32::EPSILON {
        1
    } else {
        ((distance / 0.2).ceil() as usize).max(2)
    };
    // CWorld gives ProcessCollision a sector-local candidate list before its
    // subdivision loop. Build the Bevy equivalent once for the whole swept
    // segment; individual 0.2-unit probes then narrow-phase only this local
    // set instead of walking every vehicle, ped and object again.
    let candidates = player_dynamic_motion_candidates(
        start,
        end,
        ped,
        entities,
        object_colliders,
        file_loader,
        excluded_vehicle,
    );
    let mut saved_z = start.z;
    let mut landing_support = None;
    for step in 1..=steps {
        let amount = step as f32 / steps as f32;
        let mut candidate = CVector::new(
            start.x + delta.x * amount,
            start.y + delta.y * amount,
            if landing_support.is_some() {
                saved_z
            } else {
                start.z + delta.z * amount
            },
        );
        if let Some(support) = player_subdivided_dynamic_landing_support(
            candidate,
            ped,
            entities,
            object_colliders,
            file_loader,
            &candidates,
        ) {
            // `CPed::ProcessEntityCollision` sets the ped root to
            // FEET_OFFSET+intersectionZ and m_vecMoveSpeed.z=0. Then
            // `CPhysical::ProcessCollision` copies only that Z into
            // savedMatrix before restoring the subdivided probe matrix.
            saved_z = support.z + crate::peds::PedPlacement::PED_FEET_OFFSET;
            candidate.z = saved_z;
            landing_support = Some(support);
        }
        match player_dynamic_motion_contact_from_candidates(
            candidate,
            ped,
            entities,
            object_colliders,
            file_loader,
            &candidates,
        ) {
            // Native subdivision samples are collision probes only. Every
            // successful intermediate sample is restored to `savedMatrix`,
            // and a responding collision also returns with that same matrix.
            // Preserve the colliding pose separately so the later ECS stage
            // can apply the reciprocal impulse without making the live player
            // occupy an overlapping vehicle/object pose.
            PlayerDynamicMotionContact::VehicleContact => {
                return PlayerDynamicMotionResult {
                    position: CVector::new(start.x, start.y, saved_z),
                    collision_probe: Some(candidate),
                    landing_support,
                };
            }
            PlayerDynamicMotionContact::PedContact
            | PlayerDynamicMotionContact::ImmovableObject
            | PlayerDynamicMotionContact::PushableObject => {
                return PlayerDynamicMotionResult {
                    position: CVector::new(start.x, start.y, saved_z),
                    collision_probe: Some(candidate),
                    landing_support,
                };
            }
            PlayerDynamicMotionContact::None => {}
        }
    }
    PlayerDynamicMotionResult {
        position: CVector::new(
            end.x,
            end.y,
            if landing_support.is_some() {
                saved_z
            } else {
                end.z
            },
        ),
        collision_probe: None,
        landing_support,
    }
}

pub(crate) fn resolve_player_static_collision(
    mut position: CVector,
    file_loader: &CFileLoader,
) -> CVector {
    // CTempColModels::ms_colModelPed1 supplies lateral body contacts. In re3,
    // CPed::ProcessEntityCollision owns standing Z through its vertical line
    // and stored collision polygon, so body penetration must not also shift Z.
    for _ in 0..4 {
        let mut deepest = None;
        for height in [-0.25_f32, 0.15, 0.55] {
            let contact = file_loader.world.sphere_contact_static_world(
                [position.x, position.y, position.z + height],
                0.35,
                &file_loader.model_info,
            );
            if let Some(contact) = contact
                && deepest.is_none_or(|old: crate::core::World::StaticSphereContact| {
                    contact.penetration > old.penetration
                })
            {
                deepest = Some(contact);
            }
        }
        let Some(contact) = deepest else { break };
        let horizontal = contact.normal[0].hypot(contact.normal[1]);
        if horizontal <= 1.0e-5 {
            break;
        }
        position.x += contact.normal[0] / horizontal * contact.penetration;
        position.y += contact.normal[1] / horizontal * contact.penetration;
    }
    position
}

/// Sweep a non-player ped from its current pose to the requested pose using
/// re3's `CPhysical::ProcessCollision` ped subdivision distance. Resolving only
/// the destination can put a ped completely past a thin wall before the body
/// spheres are tested; once that happens the contact normal points out of the
/// far side and the penetration solver can legitimately keep the ped there.
pub(crate) fn resolve_ped_static_motion(
    start: CVector,
    end: CVector,
    file_loader: &CFileLoader,
) -> CVector {
    let delta = CVector::new(end.x - start.x, end.y - start.y, end.z - start.z);
    let distance = delta.magnitude();
    if !distance.is_finite() || distance <= f32::EPSILON {
        return resolve_player_static_collision(end, file_loader);
    }

    // `CPhysical::ProcessCollision` uses NUMSTEPS(0.3f) for ordinary peds.
    // A 0.3-unit maximum sample spacing is smaller than the authored 0.35 ped
    // sphere radius, so the first wall contact is observed on the approach
    // side rather than after the center has crossed the surface.
    let steps = ((distance / 0.3).ceil() as usize).max(1);
    let mut accepted = start;
    for step in 1..=steps {
        let amount = step as f32 / steps as f32;
        let candidate = if step == steps {
            end
        } else {
            CVector::new(
                start.x + delta.x * amount,
                start.y + delta.y * amount,
                start.z + delta.z * amount,
            )
        };
        let resolved = resolve_player_static_collision(candidate, file_loader);
        let correction_x = resolved.x - candidate.x;
        let correction_y = resolved.y - candidate.y;
        accepted = resolved;
        if correction_x * correction_x + correction_y * correction_y > 1.0e-10 {
            break;
        }
    }
    accepted
}

#[cfg(test)]
mod ped_static_motion_tests {
    use super::*;
    use crate::core::World::{CWorldEntity, EEntityType};

    #[test]
    fn sweep_blocks_a_thin_wall_that_endpoint_penetration_misses() {
        const WALL_MODEL: usize = 4000;

        let mut file_loader = CFileLoader::default();
        let wall_min = CColVector::new(-0.05, -2.0, -2.0);
        let wall_max = CColVector::new(0.05, 2.0, 2.0);
        let mut wall_col = CColModel::new(
            CColSphere::new(3.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(wall_min, wall_max, 0, 0),
            Vec::new(),
        );
        wall_col.boxes.push(CColBox::new(wall_min, wall_max, 0, 0));
        file_loader
            .model_info
            .add_simple_model(WALL_MODEL)
            .base
            .set_col_model(wall_col, true);

        let wall = CWorldEntity {
            entity_type: EEntityType::Building,
            model_index: WALL_MODEL,
            name: "thin_wall".to_owned(),
            position: [0.0, 0.0, 0.0],
            scale: [1.0; 3],
            rotation_axis: [0.0, 0.0, 1.0],
            rotation_w: 1.0,
            rotation_degrees: 0.0,
            matrix_basis_override: None,
            render_position_offset: [0.0; 3],
            level: 1,
            is_static: true,
            is_visible: true,
            is_big_building: false,
            is_subway: false,
            streaming_dont_delete: false,
            uses_collision: true,
            zone_culled: false,
            zone_culled2: false,
            building_pool_slot: None,
            treadable_pool_slot: None,
            map_object_slot: None,
        };
        let wall_index = file_loader.world.entities.len();
        file_loader.world.entities.push(wall);
        file_loader.world.add(wall_index, &file_loader.model_info);

        let start = CVector::new(-0.5, 0.0, 0.0);
        let end = CVector::new(0.5, 0.0, 0.0);

        // Destination-only penetration sees a clear pose after the ped has
        // completely crossed this 10 cm wall.
        assert_eq!(resolve_player_static_collision(end, &file_loader), end);

        let resolved = resolve_ped_static_motion(start, end, &file_loader);
        assert!(
            resolved.x < -0.05,
            "swept ped must remain on the approach side of the wall, got x={}",
            resolved.x
        );
    }
    #[test]
    fn standing_feet_line_survives_a_clear_body_sphere_pass() {
        const WALL_MODEL: usize = 4000;

        let mut file_loader = CFileLoader::default();
        let physical_col = file_loader.temp_col_models.col_model_ped1.clone();
        file_loader.model_info.add_ped_model(MI_PLAYER).clump.base
            .set_col_model(physical_col, false);
        let wall_min = CColVector::new(-10.0, -10.0, -1.0);
        let wall_max = CColVector::new(10.0, 10.0, 0.0);
        let mut wall_col = CColModel::new(
            CColSphere::new(15.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(wall_min, wall_max, 0, 0),
            Vec::new(),
        );
        wall_col.boxes.push(CColBox::new(wall_min, wall_max, 0, 0));
        file_loader
            .model_info
            .add_simple_model(WALL_MODEL)
            .base
            .set_col_model(wall_col, true);

        let wall = CWorldEntity {
            entity_type: EEntityType::Building,
            model_index: WALL_MODEL,
            name: "test_ground".to_owned(),
            position: [0.0, 0.0, 0.0],
            scale: [1.0; 3],
            rotation_axis: [0.0, 0.0, 1.0],
            rotation_w: 1.0,
            rotation_degrees: 0.0,
            matrix_basis_override: None,
            render_position_offset: [0.0; 3],
            level: 1,
            is_static: true,
            is_visible: true,
            is_big_building: false,
            is_subway: false,
            streaming_dont_delete: false,
            uses_collision: true,
            zone_culled: false,
            zone_culled2: false,
            building_pool_slot: None,
            treadable_pool_slot: None,
            map_object_slot: None,
        };
        let wall_index = file_loader.world.entities.len();
        file_loader.world.entities.push(wall);
        file_loader.world.add(wall_index, &file_loader.model_info);

        let mut random = re3bevy_core::core::General::CGeneral::with_seed(1);
        let mut ped = CPed::new(crate::peds::PedType::ePedType::PEDTYPE_CIVMALE, &mut random);
        ped.physical.entity.model_index = MI_PLAYER as i16;
        ped.ped_state = crate::peds::Ped::PedState::PED_DIE;
        ped.flags.was_standing = true;
        ped.flags.is_standing = false;
        let root = CVector::new(0.0, 0.0, crate::peds::PedPlacement::PED_FEET_OFFSET);
        ped.physical.entity.set_position(root);
        let result = first_player_ped_static_collision(
            root, root, ped.physical.entity.matrix, 0, &ped, &file_loader, 1.0,
        );
        match result {
            PlayerPedStaticSubdivisionResult::Clear { landing_support, .. }
            | PlayerPedStaticSubdivisionResult::Collision { landing_support, .. } => {
                assert!(landing_support.is_some(), "native feet support must reach the response even when the ped was already standing");
            }
        }
        // A dying ped must descend through the weapon hit model's low spheres
        // until the physical feet line reaches the authored world surface.
        // Selecting GroundHit here would produce a body contact at z=1.20,
        // cancel gravity and strand DIE above its standing/death gate.
        for state in [crate::peds::Ped::PedState::PED_FALL,
            crate::peds::Ped::PedState::PED_DIVE_AWAY,
            crate::peds::Ped::PedState::PED_DIE,
            crate::peds::Ped::PedState::PED_DEAD] {
            ped.ped_state = state;
            ped.flags.was_standing = false;
            let above = CVector::new(0.0, 0.0, 1.20);
            assert!(player_ped_static_contacts_for_position(above,
                ped.physical.entity.matrix, MI_PLAYER, false, &file_loader).is_empty());
            assert!(std::ptr::eq(player_collision_col_model(&ped, &file_loader).unwrap(),
                file_loader.model_info.col_model(MI_PLAYER).unwrap()));
            let landed = first_player_ped_static_collision(above,
                CVector::new(0.0, 0.0, 1.02), ped.physical.entity.matrix,
                MI_PLAYER, &ped, &file_loader, 1.0);
            match landed {
                PlayerPedStaticSubdivisionResult::Clear { position, landing_support } => {
                    assert!(landing_support.is_some());
                    assert!((position.z - crate::peds::PedPlacement::PED_FEET_OFFSET).abs() < 1.0e-5);
                }
                PlayerPedStaticSubdivisionResult::Collision { .. } => panic!("weapon-only body spheres blocked native feet support in {state:?}"),
            }
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerPedStaticCollisionResult {
    pub position: CVector,
    pub movement_rejected: bool,
    pub hit_steep_slope: bool,
}

pub(crate) fn player_ped_static_contact_normal(
    contact: &crate::core::World::StaticModelContact,
    was_standing: bool,
) -> Option<CVector> {
    let mut normal = CVector::new(contact.normal[0], contact.normal[1], contact.normal[2]);
    if was_standing {
        // `CPed::ProcessEntityCollision`: for a ped that was standing, first
        // normalise XY as a 2D vector, then normalise the resulting 3D vector.
        // Pure vertical contacts stay valid because re3 does not require a
        // non-zero horizontal component before the final 3D normalisation.
        let horizontal = normal.x.hypot(normal.y);
        if horizontal > 0.0 {
            normal.x /= horizontal;
            normal.y /= horizontal;
        }
    }
    let length = normal.magnitude();
    if length <= f32::EPSILON {
        return None;
    }
    Some(CVector::new(
        normal.x / length,
        normal.y / length,
        normal.z / length,
    ))
}

pub(crate) fn player_ped_static_contacts_for_position(
    position: CVector,
    matrix: CMatrix,
    model_index: usize,
    was_standing: bool,
    file_loader: &CFileLoader,
) -> Vec<crate::core::World::StaticModelContact> {
    let basis = [
        [matrix.right.x, matrix.right.y, matrix.right.z],
        [matrix.forward.x, matrix.forward.y, matrix.forward.z],
        [matrix.up.x, matrix.up.y, matrix.up.z],
    ];
    let Some(moving_col) = file_loader.model_info.col_model(model_index) else {
        bevy::log::warn_once!(model_index, "ped_physical_waiting_original_col_model");
        return Vec::new();
    };
    file_loader
        .world
        .col_model_contacts_static_world(
            [position.x, position.y, position.z],
            basis,
            moving_col,
            &file_loader.model_info,
        )
        .into_iter()
        .filter(|contact| player_ped_static_contact_normal(contact, was_standing).is_some())
        .collect()
}

pub(crate) enum PlayerPedStaticSubdivisionResult {
    Clear {
        position: CVector,
        landing_support: Option<PlayerStaticGroundSupport>,
    },
    Collision {
        position: CVector,
        contacts: Vec<crate::core::World::StaticModelContact>,
        time_step: f32,
        landing_support: Option<PlayerStaticGroundSupport>,
    },
}

pub(crate) fn first_player_ped_static_collision(
    saved_position: CVector,
    integrated_position: CVector,
    matrix: CMatrix,
    model_index: usize,
    ped: &CPed,
    file_loader: &CFileLoader,
    time_step: f32,
) -> PlayerPedStaticSubdivisionResult {
    let was_standing = ped.flags.was_standing;
    let delta = CVector::new(
        integrated_position.x - saved_position.x,
        integrated_position.y - saved_position.y,
        integrated_position.z - saved_position.z,
    );
    // Native player peds always use at least two collision probes, even for a
    // movement shorter than 0.2 world units. Keep re3's ~0.2-unit spacing for
    // the complete movement; the old 16-step cap let sufficiently large frame
    // displacement jump across thin authored COL without ever sampling it.
    let distance = delta.magnitude();
    let steps = if distance.is_finite() {
        ((distance / 0.2).ceil() as usize).clamp(2, i8::MAX as usize)
    } else {
        2
    };
    let mut saved_z = saved_position.z;
    let mut landing_support = None;
    let mut standing_support = None;
    for step in 1..=steps {
        let amount = step as f32 / steps as f32;
        let mut position = if step == steps {
            integrated_position
        } else {
            CVector::new(
                saved_position.x + delta.x * amount,
                saved_position.y + delta.y * amount,
                if landing_support.is_some() {
                    saved_z
                } else {
                    saved_position.z + delta.z * amount
                },
            )
        };
        if landing_support.is_some() {
            position.z = saved_z;
        }
        let sample_time_step = time_step * amount;
        if let Some(support) = player_static_ground_support(
            position.x,
            position.y,
            position.z,
            ped,
            sample_time_step / GTA_PHYSICS_HZ,
            file_loader,
        ) {
            standing_support = Some(support);
            // `CPed::ProcessEntityCollision` moves the probe to the feet-line
            // intersection before processing body spheres. ProcessCollision
            // only persists that Z across the saved-matrix restore when the
            // ped was not standing at the start of the pass.
            position.z = support.z + crate::peds::PedPlacement::PED_FEET_OFFSET;
            if !was_standing {
                saved_z = position.z;
                landing_support = Some(support);
            }
        }
        let contacts = player_ped_static_contacts_for_position(
            position,
            matrix,
            model_index,
            was_standing,
            file_loader,
        );
        if !contacts.is_empty() {
            return PlayerPedStaticSubdivisionResult::Collision {
                position,
                contacts,
                time_step: sample_time_step,
                landing_support: landing_support.or(standing_support),
            };
        }
    }
    PlayerPedStaticSubdivisionResult::Clear {
        position: CVector::new(
            integrated_position.x,
            integrated_position.y,
            if landing_support.is_some() {
                saved_z
            } else {
                standing_support.map_or(integrated_position.z, |support| {
                    support.z + crate::peds::PedPlacement::PED_FEET_OFFSET
                })
            },
        ),
        landing_support: landing_support.or(standing_support),
    }
}

pub(crate) fn apply_player_ped_static_contacts(
    ped: &mut CPed,
    contacts: &[crate::core::World::StaticModelContact],
    was_standing: bool,
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
    time_ms: u32,
) -> (
    u32,
    bool,
    Option<(f32, crate::core::World::StaticModelContact, CVector)>,
) {
    let contact_count = contacts.len().max(1) as f32;
    let apply_body_friction = !ped.physical.entity.flags.has_contacted;
    let mut response_count = 0_u32;
    let mut hit_steep_slope = false;
    let mut strongest = None;

    with_player_native_collision_speed(ped, |ped| {
        for contact in contacts {
            let Some(normal) = player_ped_static_contact_normal(contact, was_standing) else {
                continue;
            };
            if contact.surface_b
                == crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8
            {
                hit_steep_slope = true;
            }

            let mut point = CColPoint::new();
            point.point = CColVector::new(contact.point[0], contact.point[1], contact.point[2]);
            point.normal = CColVector::new(normal.x, normal.y, normal.z);
            point.set_surfaces(
                contact.surface_a,
                contact.piece_a,
                contact.surface_b,
                contact.piece_b,
            );
            point.depth = contact.depth;

            let response = ped
                .physical
                .apply_collision_alt(&point, true, time_step.max(0.0));
            if response.applied {
                response_count += 1;
                if strongest
                    .as_ref()
                    .is_none_or(|(impulse, _, _)| response.impulse > *impulse)
                {
                    strongest = Some((response.impulse, *contact, normal));
                }
                if apply_body_friction {
                    let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                        &surface_table.data,
                        point.surface_a,
                        point.surface_b,
                    ) / contact_count;
                    if ped
                        .physical
                        .apply_collision_friction_single(adhesion, &point, time_step.max(0.0))
                        .applied
                    {
                        ped.physical.entity.flags.has_contacted = true;
                    }
                }
            }
            ped.physical
                .add_collision_record(EntityReference::World(contact.entity_index), time_ms);
        }
    });
    (response_count, hit_steep_slope, strongest)
}

pub(crate) fn player_ped_static_process_shift(
    position: CVector,
    matrix: CMatrix,
    model_index: usize,
    was_standing: bool,
    file_loader: &CFileLoader,
    second_shift: bool,
) -> Option<CVector> {
    let contacts = player_ped_static_contacts_for_position(
        position,
        matrix,
        model_index,
        was_standing,
        file_loader,
    );
    let deepest_scale = if second_shift { 0.4 } else { 0.2 };
    let mut shift = CVector::ZERO;

    // ProcessShiftSectorList calls ProcessEntityCollision separately for each
    // candidate entity. Its 1.2/1.5 manifold divisor and 0.2/0.4 deepest-point
    // term are therefore applied once per building/object, then accumulated in
    // the final shift. `col_model_contacts_static_world` preserves contacts for
    // each visited entity contiguously, so reproduce that grouping here instead
    // of treating an entire corner/sector as one weak manifold.
    let mut cursor = 0usize;
    while cursor < contacts.len() {
        let entity_index = contacts[cursor].entity_index;
        let mut end = cursor + 1;
        while end < contacts.len() && contacts[end].entity_index == entity_index {
            end += 1;
        }
        let group = &contacts[cursor..end];
        let manifold_scale = if second_shift { 1.5 } else { 1.2 } / group.len() as f32;
        let deepest = group
            .iter()
            .max_by(|left, right| left.depth.total_cmp(&right.depth))
            .expect("non-empty per-entity collision group");

        for contact in group {
            let Some(normal) = player_ped_static_contact_normal(contact, was_standing) else {
                continue;
            };
            shift.x += normal.x * contact.depth * manifold_scale;
            shift.y += normal.y * contact.depth * manifold_scale;
            shift.z += normal.z * contact.depth * manifold_scale;
        }
        if let Some(normal) = player_ped_static_contact_normal(deepest, was_standing) {
            shift.x += normal.x * deepest.depth * deepest_scale;
            shift.y += normal.y * deepest.depth * deepest_scale;
            shift.z += normal.z * deepest.depth * deepest_scale;
        }
        cursor = end;
    }
    if shift.magnitude_sqr() <= 1.0e-10 {
        return None;
    }
    Some(CVector::new(
        position.x + shift.x,
        position.y + shift.y,
        position.z + shift.z,
    ))
}

/// Player/static portion of re3's `CWorld::Process` physical pipeline: probe
/// the authored ped COL, retain collision-adjusted velocity while rejecting a
/// colliding matrix, retry from the saved pose, then fall back to the two
/// ProcessShift passes only after the normal collision attempts remain unsafe.
pub fn resolve_player_ped_static_collision_result(
    saved_position: CVector,
    attempted_position: CVector,
    ped: &mut CPed,
    file_loader: &CFileLoader,
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
    time_ms: u32,
) -> PlayerPedStaticCollisionResult {
    let raw_model = ped.physical.entity.model_index;
    let model_index = if raw_model <= 0 {
        MI_PLAYER
    } else {
        raw_model as usize
    };
    let matrix = ped.physical.entity.matrix;
    let was_standing = ped.flags.was_standing;
    let dt_seconds = time_step.max(0.0) / GTA_PHYSICS_HZ;
    let mut collision_saved_position = saved_position;
    let mut position = attempted_position;
    let mut integrated_position = attempted_position;
    let mut responded = false;
    let mut safe = false;
    let mut hit_steep_slope = false;
    let mut strongest_response: Option<(f32, crate::core::World::StaticModelContact, CVector)> =
        None;

    // CWorld performs one ProcessCollision call, four retries, then one final
    // stuck attempt before entering ProcessShift.
    for attempt in 0..=5 {
        let (collision_position, contacts, collision_time_step, landing_support) =
            match first_player_ped_static_collision(
                collision_saved_position,
                integrated_position,
                matrix,
                model_index,
                ped,
                file_loader,
                time_step.max(0.0),
            ) {
                PlayerPedStaticSubdivisionResult::Clear {
                    position: clear_position,
                    landing_support,
                } => {
                    if let Some(support) = landing_support {
                        if !was_standing {
                            collision_saved_position.z = clear_position.z;
                        }
                        ped.physical.move_speed.z = 0.0;
                        ped.flags.is_standing = true;
                        ped.flags.is_in_the_air = false;
                        update_player_current_phys_surface(ped, Some(support), None, true);
                    }
                    position = clear_position;
                    safe = true;
                    break;
                }
                PlayerPedStaticSubdivisionResult::Collision {
                    position,
                    contacts,
                    time_step,
                    landing_support,
                } => (position, contacts, time_step, landing_support),
            };

        if let Some(support) = landing_support {
            if !was_standing {
                collision_saved_position.z = support.z + crate::peds::PedPlacement::PED_FEET_OFFSET;
            }
            ped.physical.move_speed.z = 0.0;
            ped.flags.is_standing = true;
            ped.flags.is_in_the_air = false;
            update_player_current_phys_surface(ped, Some(support), None, true);
        }

        position = collision_position;
        ped.physical.entity.matrix.position = collision_position;
        ped.physical.entity.flags.has_hit_wall = true;
        let (response_count, pass_hit_steep, pass_strongest) = apply_player_ped_static_contacts(
            ped,
            &contacts,
            was_standing,
            surface_table,
            collision_time_step,
            time_ms,
        );
        let mut restored_matrix = matrix;
        restored_matrix.position = collision_saved_position;
        ped.physical.entity.set_matrix(restored_matrix);
        hit_steep_slope |= pass_hit_steep;
        if pass_strongest.is_some_and(|candidate| {
            strongest_response.is_none_or(|current| candidate.0 > current.0)
        }) {
            strongest_response = pass_strongest;
        }
        if response_count == 0 {
            // A geometric manifold while already moving out of the surface is
            // not a responding collision and therefore does not reject the
            // integrated matrix in ProcessCollisionSectorList.
            position = CVector::new(
                integrated_position.x,
                integrated_position.y,
                if landing_support.is_some() {
                    collision_saved_position.z
                } else {
                    integrated_position.z
                },
            );
            safe = true;
            break;
        }
        responded = true;
        if attempt == 5 {
            break;
        }
        integrated_position = CVector::new(
            collision_saved_position.x + ped.physical.move_speed.x * dt_seconds,
            collision_saved_position.y + ped.physical.move_speed.y * dt_seconds,
            collision_saved_position.z + ped.physical.move_speed.z * dt_seconds,
        );
    }

    if responded && !safe {
        for second_shift in [false, true] {
            let integrated = CVector::new(
                collision_saved_position.x + ped.physical.move_speed.x * dt_seconds,
                collision_saved_position.y + ped.physical.move_speed.y * dt_seconds,
                collision_saved_position.z + ped.physical.move_speed.z * dt_seconds,
            );
            let contacts = player_ped_static_contacts_for_position(
                integrated,
                matrix,
                model_index,
                was_standing,
                file_loader,
            );
            if contacts.is_empty() {
                position = integrated;
                safe = true;
                break;
            }
            let Some(shifted) = player_ped_static_process_shift(
                integrated,
                matrix,
                model_index,
                was_standing,
                file_loader,
                second_shift,
            ) else {
                position = collision_saved_position;
                continue;
            };

            // ProcessShift rechecks ordinary collision response after applying
            // separation. A responding recheck restores the matrix captured at
            // ProcessShift entry; a non-responding manifold is accepted.
            let shifted_contacts = player_ped_static_contacts_for_position(
                shifted,
                matrix,
                model_index,
                was_standing,
                file_loader,
            );
            if shifted_contacts.is_empty() {
                position = shifted;
                safe = true;
                break;
            }
            ped.physical.entity.flags.has_hit_wall = true;
            ped.physical.entity.matrix.position = shifted;
            let (response_count, pass_hit_steep, pass_strongest) = apply_player_ped_static_contacts(
                ped,
                &shifted_contacts,
                was_standing,
                surface_table,
                time_step.max(0.0),
                time_ms,
            );
            ped.physical.entity.set_matrix(matrix);
            hit_steep_slope |= pass_hit_steep;
            if pass_strongest.is_some_and(|candidate| {
                strongest_response.is_none_or(|current| candidate.0 > current.0)
            }) {
                strongest_response = pass_strongest;
            }
            if response_count == 0 {
                position = shifted;
                safe = true;
                break;
            }
            position = collision_saved_position;
        }
    }

    let movement_rejected = !safe;
    if movement_rejected {
        position = collision_saved_position;
        ped.physical.entity.flags.is_stuck = true;
        ped.physical.entity.flags.is_in_safe_position = false;
        if ped.physical.entity.status == eEntityStatus::STATUS_PLAYER {
            // CWorld's final stuck fallback for player physicals.
            ped.physical.move_speed.x *= 0.3;
            ped.physical.move_speed.y *= 0.3;
            ped.physical.move_speed.z *= 0.3;
            position = CVector::new(
                collision_saved_position.x + ped.physical.move_speed.x * dt_seconds,
                collision_saved_position.y + ped.physical.move_speed.y * dt_seconds,
                collision_saved_position.z + ped.physical.move_speed.z * dt_seconds,
            );
        }
    } else {
        ped.physical.entity.flags.is_stuck = false;
        ped.physical.entity.flags.is_in_safe_position = true;
    }
    ped.physical.entity.set_matrix(matrix);

    if let Some((impulse, contact, normal)) = strongest_response {
        ped.physical.record_collision_damage(
            contact.piece_a as u16,
            impulse,
            Some(EntityReference::World(contact.entity_index)),
            normal,
        );
    }
    PlayerPedStaticCollisionResult {
        position,
        movement_rejected,
        hit_steep_slope,
    }
}

// === FAZ9d: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn object_static_collision_step(
    object: &mut CObject,
    collision: &crate::objects::Object::ObjectCollisionState,
    file_loader: &CFileLoader,
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
    time_ms: u32,
) {
    if object.physical.entity.flags.is_static || !object.physical.entity.flags.uses_collision {
        return;
    }
    let saved_matrix = collision.previous_matrix;
    let model_id = object.physical.entity.model_index.max(0) as usize;
    let mut integrated_matrix = object.physical.entity.matrix;
    let mut strongest: Option<(f32, crate::core::World::StaticModelContact)> = None;

    // `CWorld::Process` gives an unsafe physical one initial collision pass,
    // four retries, then one final stuck pass before falling back to
    // `ProcessShift`. Keep those retries local to this ECS world stage so an
    // object cannot spend whole render frames repeatedly rewinding to an old
    // airborne matrix.
    for attempt in 0..=5 {
        let Some((collision_matrix, contacts, collision_time_step)) = first_object_static_collision(
            object,
            saved_matrix,
            integrated_matrix,
            model_id,
            file_loader,
            time_step.max(0.0),
        ) else {
            object.physical.entity.set_matrix(integrated_matrix);
            object.physical.entity.flags.is_in_safe_position = true;
            object.physical.entity.flags.is_stuck = false;
            let final_position = object.physical.entity.get_position();
            object.physical.distance_travelled = CVector::new(
                final_position.x - saved_matrix.position.x,
                final_position.y - saved_matrix.position.y,
                final_position.z - saved_matrix.position.z,
            )
            .magnitude();
            object.object_matrix = object.physical.entity.matrix;
            return;
        };

        object.physical.entity.set_matrix(collision_matrix);
        object.physical.entity.flags.has_hit_wall = true;
        let (responses, pass_strongest) =
            apply_object_static_contacts(object, &contacts, surface_table, collision_time_step);
        if responses != 0 {
            object.physical.distance_travelled = 0.0;
        }
        if pass_strongest
            .is_some_and(|candidate| strongest.is_none_or(|current| candidate.0 > current.0))
        {
            strongest = pass_strongest;
        }

        // Native ProcessCollision restores the matrix saved at the beginning
        // of the pass whenever the static response rejects the integrated
        // pose. The changed velocity/torque is retained for the retry.
        object.physical.entity.set_matrix(saved_matrix);
        if responses == 0 {
            object.physical.entity.set_matrix(integrated_matrix);
            object.physical.entity.flags.is_in_safe_position = true;
            object.physical.entity.flags.is_stuck = false;
            let final_position = object.physical.entity.get_position();
            object.physical.distance_travelled = CVector::new(
                final_position.x - saved_matrix.position.x,
                final_position.y - saved_matrix.position.y,
                final_position.z - saved_matrix.position.z,
            )
            .magnitude();
            object.object_matrix = object.physical.entity.matrix;
            return;
        }
        if attempt == 5 {
            break;
        }
        integrated_matrix = integrate_physical_matrix(
            saved_matrix,
            object.physical.move_speed,
            object.physical.turn_speed,
            time_step.max(0.0),
        );
    }

    if let Some((impulse, contact)) = strongest {
        object
            .physical
            .add_collision_record(EntityReference::World(contact.entity_index), time_ms);
        object.physical.record_collision_damage(
            contact.piece_a as u16,
            impulse,
            Some(EntityReference::World(contact.entity_index)),
            CVector::new(contact.normal[0], contact.normal[1], contact.normal[2]),
        );
        object.current_surface = Some(contact.entity_index);
    }

    // re3 then runs two ProcessShift passes (1.2/0.2 followed by 1.5/0.4).
    // Each pass starts from the saved pose, applies this frame's movement,
    // shifts out of the manifold, and accepts only a collision-free result.
    let mut safe = false;
    for second_shift in [false, true] {
        object.physical.entity.set_matrix(integrate_physical_matrix(
            saved_matrix,
            object.physical.move_speed,
            object.physical.turn_speed,
            time_step.max(0.0),
        ));
        if object_static_contacts_for_matrix(file_loader, model_id, &object.physical.entity.matrix)
            .is_empty()
            || object_static_process_shift(object, file_loader, model_id, second_shift)
        {
            safe = true;
            break;
        }
        object.physical.entity.set_matrix(saved_matrix);
    }
    if !safe {
        object.physical.entity.set_matrix(saved_matrix);
        object.physical.distance_travelled = 0.0;
    } else {
        let final_position = object.physical.entity.get_position();
        object.physical.distance_travelled = CVector::new(
            final_position.x - saved_matrix.position.x,
            final_position.y - saved_matrix.position.y,
            final_position.z - saved_matrix.position.z,
        )
        .magnitude();
    }
    object.physical.entity.flags.is_in_safe_position = safe;
    object.physical.entity.flags.is_stuck = !safe;
    object.object_matrix = object.physical.entity.matrix;
}

pub(crate) fn object_static_contacts_for_matrix(
    file_loader: &CFileLoader,
    model_id: usize,
    matrix: &CMatrix,
) -> Vec<crate::core::World::StaticModelContact> {
    automobile_static_contacts_for_matrix(file_loader, model_id, matrix)
}

pub(crate) fn integrate_physical_matrix(
    matrix: CMatrix, move_speed: CVector, turn_speed: CVector, time_step: f32,
) -> CMatrix {
    let mut matrix = partial_physical_matrix(matrix, move_speed, turn_speed, time_step);
    matrix.reorthogonalise();
    matrix
}

pub(crate) fn partial_physical_matrix(
    mut matrix: CMatrix,
    move_speed: CVector,
    turn_speed: CVector,
    time_step: f32,
) -> CMatrix {
    matrix.position.x += move_speed.x * time_step;
    matrix.position.y += move_speed.y * time_step;
    matrix.position.z += move_speed.z * time_step;
    let turn = CVector::new(
        turn_speed.x * time_step,
        turn_speed.y * time_step,
        turn_speed.z * time_step,
    );
    matrix.right = CVector::new(
        matrix.right.x + turn.cross(matrix.right).x,
        matrix.right.y + turn.cross(matrix.right).y,
        matrix.right.z + turn.cross(matrix.right).z,
    );
    matrix.forward = CVector::new(
        matrix.forward.x + turn.cross(matrix.forward).x,
        matrix.forward.y + turn.cross(matrix.forward).y,
        matrix.forward.z + turn.cross(matrix.forward).z,
    );
    matrix.up = CVector::new(
        matrix.up.x + turn.cross(matrix.up).x,
        matrix.up.y + turn.cross(matrix.up).y,
        matrix.up.z + turn.cross(matrix.up).z,
    );
    matrix
}

pub(crate) fn first_object_static_collision(
    object: &CObject,
    saved_matrix: CMatrix,
    integrated_matrix: CMatrix,
    model_id: usize,
    file_loader: &CFileLoader,
    time_step: f32,
) -> Option<(CMatrix, Vec<crate::core::World::StaticModelContact>, f32)> {
    let steps = object_collision_substeps(object, file_loader, time_step);
    for step in 1..=steps {
        let amount = step as f32 / steps as f32;
        let matrix = if step == steps {
            integrated_matrix
        } else {
            integrate_physical_matrix(
                saved_matrix,
                object.physical.move_speed,
                object.physical.turn_speed,
                time_step * amount,
            )
        };
        let contacts = object_static_contacts_for_matrix(file_loader, model_id, &matrix);
        if !contacts.is_empty() {
            return Some((matrix, contacts, time_step * amount));
        }
    }
    None
}

pub(crate) fn object_collision_substeps(
    object: &CObject,
    file_loader: &CFileLoader,
    time_step: f32,
) -> usize {
    let mut dist_sq = object.physical.move_speed.magnitude_sqr() * time_step * time_step;
    let response_case = object.special_collision_response_cases;
    let (threshold_sq, step_size) =
        if response_case == CollisionSpecialResponseCase::COLLRESPONSE_LAMPOST as u8 {
            let radius = file_loader
                .model_info
                .col_model(object.physical.entity.model_index.max(0) as usize)
                .map(|col| col.bounding_sphere.radius)
                .unwrap_or(0.0);
            let up = object
                .physical
                .entity
                .transform_vector(CVector::new(0.0, 0.0, radius));
            let down = object
                .physical
                .entity
                .transform_vector(CVector::new(0.0, 0.0, -radius));
            dist_sq = object
                .physical
                .get_speed_at_point(up)
                .magnitude_sqr()
                .max(object.physical.get_speed_at_point(down).magnitude_sqr())
                * time_step
                * time_step;
            (0.3_f32 * 0.3, 0.3_f32)
        } else if response_case == CollisionSpecialResponseCase::COLLRESPONSE_UNKNOWN5 as u8 {
            (0.009_f32, 0.09_f32)
        } else if response_case == CollisionSpecialResponseCase::COLLRESPONSE_SMALLBOX as u8
            || response_case == CollisionSpecialResponseCase::COLLRESPONSE_FENCEPART as u8
        {
            (0.15_f32 * 0.15, 0.15_f32)
        } else {
            (0.3_f32 * 0.3, 0.3_f32)
        };
    if dist_sq < threshold_sq {
        1
    } else {
        // Same hang guard as the automobile subdivision: re3 keeps this count
        // in an `int8 n` (`Physical.cpp:1888`), and 16 steps already cover
        // frame travel no legitimate object can reach.
        (dist_sq.sqrt() / step_size).ceil().clamp(1.0, 16.0) as usize
    }
}

pub(crate) fn apply_object_static_contacts(
    object: &mut CObject,
    contacts: &[crate::core::World::StaticModelContact],
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
) -> (u32, Option<(f32, crate::core::World::StaticModelContact)>) {
    let base_move_speed = object.physical.move_speed;
    let base_turn_speed = object.physical.turn_speed;
    let mut move_response = CVector::ZERO;
    let mut turn_response = CVector::ZERO;
    let mut response_count = 0_u32;
    let mut strongest = None;
    let apply_friction = !object.physical.entity.flags.has_contacted;

    for contact in contacts {
        let mut point = CColPoint::new();
        point.point = CColVector::new(contact.point[0], contact.point[1], contact.point[2]);
        point.normal = CColVector::new(contact.normal[0], contact.normal[1], contact.normal[2]);
        point.set_surfaces(
            contact.surface_a,
            contact.piece_a,
            contact.surface_b,
            contact.piece_b,
        );
        point.depth = contact.depth;
        let response = object
            .physical
            .apply_collision_alt(&point, true, time_step.max(0.0));
        if !response.applied {
            continue;
        }
        response_count += 1;
        move_response.x += response.move_speed.x - base_move_speed.x;
        move_response.y += response.move_speed.y - base_move_speed.y;
        move_response.z += response.move_speed.z - base_move_speed.z;
        turn_response.x += response.turn_speed.x - base_turn_speed.x;
        turn_response.y += response.turn_speed.y - base_turn_speed.y;
        turn_response.z += response.turn_speed.z - base_turn_speed.z;
        if strongest.is_none_or(|(impulse, _)| response.impulse > impulse) {
            strongest = Some((response.impulse, *contact));
        }
        if apply_friction {
            let adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                &surface_table.data,
                point.surface_a,
                point.surface_b,
            ) / contacts.len().max(1) as f32;
            if object
                .physical
                .apply_collision_friction_single(adhesion, &point, time_step.max(0.0))
                .applied
            {
                object.physical.entity.flags.has_contacted = true;
            }
        }
    }
    if response_count != 0 {
        let divisor = response_count as f32;
        object.physical.move_speed = CVector::new(
            base_move_speed.x + move_response.x / divisor,
            base_move_speed.y + move_response.y / divisor,
            base_move_speed.z + move_response.z / divisor,
        );
        object.physical.turn_speed = CVector::new(
            base_turn_speed.x + turn_response.x / divisor,
            base_turn_speed.y + turn_response.y / divisor,
            base_turn_speed.z + turn_response.z / divisor,
        );
    }
    (response_count, strongest)
}

pub(crate) fn object_static_process_shift(
    object: &mut CObject,
    file_loader: &CFileLoader,
    model_id: usize,
    second_shift: bool,
) -> bool {
    let contacts =
        object_static_contacts_for_matrix(file_loader, model_id, &object.physical.entity.matrix);
    let Some(deepest) = contacts
        .iter()
        .max_by(|left, right| left.depth.total_cmp(&right.depth))
    else {
        return true;
    };
    let count = contacts.len() as f32;
    let manifold_scale = if second_shift { 1.5 } else { 1.2 } / count;
    let deepest_scale = if second_shift { 0.4 } else { 0.2 };
    let mut shift = CVector::ZERO;
    for contact in &contacts {
        shift.x += contact.normal[0] * contact.depth * manifold_scale;
        shift.y += contact.normal[1] * contact.depth * manifold_scale;
        shift.z += contact.normal[2] * contact.depth * manifold_scale;
    }
    shift.x += deepest.normal[0] * deepest.depth * deepest_scale;
    shift.y += deepest.normal[1] * deepest.depth * deepest_scale;
    shift.z += deepest.normal[2] * deepest.depth * deepest_scale;
    if !object.physical.apply_process_shift(shift) {
        return false;
    }
    object_static_contacts_for_matrix(file_loader, model_id, &object.physical.entity.matrix)
        .is_empty()
}

// === FAZ10c-collision: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn update_collision_runtime(
    mut collision: ResMut<CCollision>,
    mut game_state: ResMut<CGameState>,
    mut timer: ResMut<CTimer>,
    cutscene: Res<CCutsceneMgr>,
    players: crate::peds::PlayerRead::CWorldPlayersRead,
    mut file_loader: ResMut<CFileLoader>,
    mut streaming: ResMut<CStreaming>,
    mut before_rw: ResMut<CGameBeforeRw>,
    mut population: ResMut<CPopulation>,
    mut replay: ResMut<CReplay>,
    mut dmaudio: ResMut<cDMAudio>,
    mut pads: ResMut<CPads>,
    mut mission_debug: Option<ResMut<crate::control::ScriptDebug::GtaMissionDebugSwitcher>>,
) {
    // re3's FindPlayerCoors always resolves a live player. Until SCM has made
    // that player, running the island heuristic would invent a position.
    let Some(player) = players.focused() else {
        collision.waiting_player_frames += 1;
        return;
    };
    let position = player.position();
    let zone_level = match file_loader.world.zones.get_level_from_position([position.x, position.y, position.z]) as i32 {
        1 => ELevelName::Industrial,
        2 => ELevelName::Commercial,
        3 => ELevelName::Suburban,
        _ => ELevelName::Generic,
    };
    // A debug entry bypasses the story's preceding island travel. Its SCM
    // teleport requests the same native island selection/load below, even if
    // the newly loaded cutscene would normally defer CCollision::Update.
    let entry_collision_pending = mission_debug.as_deref()
        .is_some_and(|debug| debug.needs_entry_collision());
    if entry_collision_pending && let Some(debug) = mission_debug.as_deref_mut() {
        debug.entry_collision_wait_frames += 1;
    }
    let result = collision.update(CollisionUpdateContext {
        player_position: CColVector::new(
            position.x,
            position.y,
            position.z,
        ),
        zone_level,
        current_level: game_state.curr_level,
        time_in_milliseconds: timer.time_in_milliseconds(),
        cutscene_processing: cutscene.is_cutscene_processing() && !entry_collision_pending,
    });
    game_state.curr_level = result.current_level;

    if result.load_collision {
        let old_level = collision.collision_in_memory;
        let new_level = result.current_level;
        dmaudio.SampleManager.SetEffectsFadeVolume(0);
        pads.stop_pads_shaking();
        population.initial_zone_change = Some((old_level, new_level));

        // Collision.cpp LoadCollisionWhenINeedIt first drops the old island
        // big-building instances, then changes collision residency. Ordinary
        // non-current buildings are removed after the new collision DAT is in.
        for island_level in 1..=3 {
            file_loader.remove_big_buildings(&mut streaming, island_level);
        }
        file_loader
            .model_info
            .remove_col_models_from_other_levels(new_level as i32);
        let dat_file = game_state.dat_file.clone();
        if let Err(error) = file_loader.load_collision_from_dat_file(
            &mut before_rw.file_mgr,
            &dat_file,
            new_level as i32,
        ) {
            collision.last_load_error = Some(error.to_string());
            dmaudio.SampleManager.SetEffectsFadeVolume(127);
            return;
        }

        collision.collision_in_memory = new_level;
        collision.last_load_error = None;
        replay.empty_replay_buffer();
        file_loader.remove_unused_big_buildings(&mut streaming, new_level as i32);
        file_loader.remove_unused_buildings(&mut streaming, new_level as i32);
        file_loader.request_big_buildings(&mut streaming, new_level as i32);

        // re3 blocks in LoadAllRequestedModels here while the timer is
        // suspended. Doing that disk/TXD/DFF work inside one Bevy Update frame
        // stalls rendering. Keep the same residency barrier, but let the
        // Streaming set drain the already-authored request list over rendered
        // frames while gameplay code is paused.
        file_loader.begin_load_all_models_streaming(&streaming);
        if file_loader.script_streaming_barrier_in_progress() && !timer.get_is_code_paused() {
            timer.set_code_pause(true);
            file_loader.set_script_streaming_owns_code_pause(true);
        }
    }

    if result.check_big_buildings {
        file_loader.have_all_big_buildings_loaded(&mut streaming, game_state.curr_level as i32);
    }
    if entry_collision_pending
        && result.check_big_buildings
        && !file_loader.script_streaming_barrier_in_progress()
        && collision.collision_in_memory == result.current_level
        && collision.last_load_error.is_none()
        && let Some(debug) = mission_debug.as_deref_mut()
    {
        debug.finish_entry_collision();
    }
    dmaudio.SampleManager.SetEffectsFadeVolume(127);
}
