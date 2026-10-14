//! re3 reference: references/re-GTA/src/core/World.cpp
//! re3 reference: references/re-GTA/src/core/World.h
//!
//! Static world records loaded by `CFileLoader::LoadScene`.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    io,
};

use bevy::prelude::{Component, Res, Resource};
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};
use rust_gta_3_formats::formats::FileLoader::{
    GtaCullZone, GtaIplInstance, GtaIplPlan, GtaIplRecord, GtaPickup, GtaZone,
};

use crate::{
    collision::{
        ColModel::{
            process_sphere_box, process_sphere_sphere, process_sphere_triangle, test_line_box,
            CColBox, CColLine, CColModel, CColPoint, CColSphere, CColVector,
        },
        Collision::MAX_COLLISION_POINTS,
    },
    math::Matrix::CMatrix,
    modelinfo::ModelInfo::CModelInfo,
};

pub const WORLD_MIN_X: f32 = -2000.0;
pub const WORLD_MIN_Y: f32 = -2000.0;
pub const WORLD_MAX_X: f32 = 2000.0;
pub const WORLD_MAX_Y: f32 = 2000.0;
pub const NUMSECTORS_X: usize = 100;
pub const NUMSECTORS_Y: usize = 100;
pub const SECTOR_SIZE_X: f32 = (WORLD_MAX_X - WORLD_MIN_X) / NUMSECTORS_X as f32;
pub const SECTOR_SIZE_Y: f32 = (WORLD_MAX_Y - WORLD_MIN_Y) / NUMSECTORS_Y as f32;

/// `CWorld::Process` dispatches normal moving actors only in its non-cutscene
/// branch. Chase, cutscene objects and CGame-owned managers retain their own
/// stages; derived entity membership must keep draining removals in both branches.
pub(crate) fn world_actor_processing_enabled(
    loader: Option<Res<crate::core::FileLoader::CFileLoader>>,
) -> bool {
    let Some(loader) = loader else {
        bevy::log::warn_once!("world_actor_processing_waiting_file_loader");
        return false;
    };
    !loader.world.process_cutscene_only
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticSphereContact {
    pub normal: [f32; 3],
    pub penetration: f32,
}

/// Narrow-phase contact produced by a vehicle COL primitive against a static
/// world COL primitive.  This is the same information that re3's
/// `CPhysical::ProcessCollisionSectorList` forwards to `ApplyCollisionAlt` and
/// `CAutomobile::VehicleDamage` (including the authored vehicle piece id).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticModelContact {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub piece_a: u8,
    pub piece_b: u8,
    pub surface_a: u8,
    pub surface_b: u8,
    pub depth: f32,
    pub entity_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticLineContact {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub surface: u8,
    pub entity_index: usize,
}

/// One `CPopulation::ManagePopulation` dummy↔object conversion performed by
/// [`CWorld::update_object_dummy_collision_lifecycle`]. The ECS half of the
/// conversion (spawning/removing the live `CObject`) is driven by
/// `process_map_object_collision_lifecycle` in the population pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectDummyLifecycleChange {
    /// re3 `CPopulation::ConvertToRealObject`: the world entity handed its
    /// render and collision ownership to a live ECS CObject.
    Promoted {
        entity_index: usize,
        model_index: usize,
    },
    /// re3 `CPopulation::ConvertToDummyObject`: the ECS CObject is released
    /// and the world entity is the visible collisionless dummy again.
    Demoted { entity_index: usize },
}

/// One native ReplaceWithNewModel operation. This is a frame command, not a
/// second world/model cache. Consume destruction before replacement rendering.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WorldModelReplacement {
    pub entity_index: usize,
    pub old_model_index: usize,
    pub new_model_index: usize,
    /// SCM decides against its current level when it issues the command.
    /// Save restoration borrows the current native level at consumption.
    pub request_big_building: Option<bool>,
}

#[derive(Debug, Clone, Resource)]
pub struct CWorld {
    pub zones: CTheZones,
    pub cull_zones: CCullZones,
    pub sectors: Vec<CWorldSector>,
    pub entities: Vec<CWorldEntity>,
    pub building_pool: Vec<usize>,
    pub treadable_pool: Vec<usize>,
    pub map_objects: Vec<usize>,
    /// IPL instances whose model has OBJECT.DAT data. re3 initially stores
    /// these as `CDummyObject`s and CPopulation promotes a 1/32 slice to real
    /// colliding `CObject`s when the player is within 80 metres.
    pub object_dummy_pool: Vec<usize>,
    /// Static IPL records already transferred to live ECS CObjects by an SCM
    /// command. Their stable world indices remain only as hidden provenance.
    promoted_object_dummies: HashSet<usize>,
    /// World-entity index → live ECS CObject handle for OBJECT.DAT dummies
    /// promoted by `CPopulation::ManagePopulation`. While promoted, the world
    /// entity stays hidden and collisionless and the ECS CObject owns both
    /// rendering and the physical response.
    pub ecs_lifecycle_objects: HashMap<usize, i32>,
    pub big_buildings_by_level: [Vec<usize>; 4],
    pub pickups: Vec<GtaPickup>,
    /// re3 `CWorld::bProcessCutsceneOnly`.
    pub process_cutscene_only: bool,
    /// Incremented whenever an IPL entity model is swapped (e.g. SCM ReplaceWithNewModel).
    pub model_swap_epoch: u64,
    pending_model_replacements: Vec<WorldModelReplacement>,
}

#[derive(Debug, Clone, Default)]
pub struct CWorldSector {
    pub buildings: Vec<usize>,
    pub buildings_overlap: Vec<usize>,
    pub dummies: Vec<usize>,
    pub dummies_overlap: Vec<usize>,
    pub objects: Vec<usize>,
    pub objects_overlap: Vec<usize>,
    pub vehicles: Vec<usize>,
    pub vehicles_overlap: Vec<usize>,
    pub peds: Vec<usize>,
    pub peds_overlap: Vec<usize>,
}

#[derive(Debug, Clone, Component)]
pub struct CWorldEntity {
    pub entity_type: EEntityType,
    pub model_index: usize,
    pub name: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation_axis: [f32; 3],
    pub rotation_w: f32,
    pub rotation_degrees: f32,
    /// Runtime matrix basis override in GTA coordinates, stored as
    /// `[right, forward, up]`. re3's rotating garage doors directly rewrite
    /// these three vectors and can produce a reflected basis that cannot be
    /// represented by an axis-angle rotation alone.
    pub matrix_basis_override: Option<[[f32; 3]; 3]>,
    /// Render-only displacement used by re3 operations that update the RW
    /// frame with a temporary matrix and then restore the logical position
    /// (notably `CGarage::UpdateCrusherShake`).
    pub render_position_offset: [f32; 3],
    pub level: i32,
    pub is_static: bool,
    pub is_visible: bool,
    pub is_big_building: bool,
    pub is_subway: bool,
    pub streaming_dont_delete: bool,
    pub uses_collision: bool,
    pub zone_culled: bool,
    pub zone_culled2: bool,
    pub building_pool_slot: Option<usize>,
    pub treadable_pool_slot: Option<usize>,
    pub map_object_slot: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EEntityType {
    Nothing = 0,
    Building = 1,
    Vehicle = 2,
    Ped = 3,
    Object = 4,
    Dummy = 5,
}

impl CWorldEntity {
    pub fn get_bound_rect(&self, model_info: &CModelInfo) -> Option<[f32; 4]> {
        let col_model = model_info.col_model(self.model_index)?;
        let min = col_model.bounding_box.min;
        let max = col_model.bounding_box.max;

        let p1 = transform_entity_point(self, [min.x, min.y, min.z]);
        let p2 = transform_entity_point(self, [max.x, max.y, max.z]);
        let p3 = transform_entity_point(self, [max.x, min.y, min.z]);
        let p4 = transform_entity_point(self, [min.x, max.y, max.z]);

        let min_x = p1[0].min(p2[0]).min(p3[0]).min(p4[0]);
        let max_x = p1[0].max(p2[0]).max(p3[0]).max(p4[0]);
        let min_y = p1[1].min(p2[1]).min(p3[1]).min(p4[1]);
        let max_y = p1[1].max(p2[1]).max(p3[1]).max(p4[1]);

        Some([min_x, max_x, min_y, max_y])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZonePedInfo {
    pub car_density: i16,
    pub car_threshold: [i16; 6],
    pub cop_threshold: i16,
    pub gang_threshold: [i16; 9],
    pub ped_density: u16,
    pub cop_density: u16,
    pub gang_density: [u16; 9],
    pub ped_group: u16,
}

impl Default for ZonePedInfo {
    fn default() -> Self {
        let step = 1000 / 6;
        let car_threshold = [step, step * 2, step * 3, step * 4, step * 4, step * 4];
        let cop_threshold = car_threshold[5] + step;
        Self {
            car_density: 10,
            car_threshold,
            cop_threshold,
            gang_threshold: [cop_threshold; 9],
            ped_density: 0,
            cop_density: 0,
            gang_density: [0; 9],
            ped_group: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CZone {
    pub name: String,
    pub zone_type: i32,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub level: i32,
    pub ped_info_day: ZonePedInfo,
    pub ped_info_night: ZonePedInfo,
}

pub const ZONE_DEFAULT: i32 = 0;
pub const ZONE_NAVIG: i32 = 1;

#[derive(Debug, Clone, Resource)]
pub struct CTheZones {
    pub zones: Vec<CZone>,
    pub map_zones: Vec<CZone>,
    pub curr_level: i32,
}

pub const NUMCULLZONES: usize = 512;
pub const NUMATTRIBZONES: usize = 288;
pub const NUMZONEINDICES: usize = 55000;
pub const NUMBUILDINGS: usize = 5500;
pub const NUMTREADABLES: usize = 1214;
pub const NUMUNCOMPRESSED_CULL_INDICES: u16 = 6000;
pub const INVALID_CULL_INDEX: u16 = 0xFFFF;
pub const STREAM_DISTANCE: f32 = 30.0;
pub const FADE_DISTANCE: f32 = 20.0;

#[derive(Debug, Clone, Resource)]
pub struct CCullZones {
    pub cull_zones: Vec<GtaCullZone>,
    pub attribute_zones: Vec<GtaCullZone>,
    pub resolved_zones: Vec<CResolvedCullZone>,
    pub resolved_attribute_zones: Vec<CResolvedAttributeZone>,
    pub indices: Vec<u16>,
    pub building_to_big_building: Vec<i16>,
    pub treadable_to_big_building: Vec<i16>,
    pub visibility_candidate_plans: Vec<CVisibilityCandidatePlan>,
    pub old_cull_zone: i32,
    pub entity_indices_used: i32,
    pub loaded_visibility_cache: bool,
    /// C++ `CCullZones::CurrentFlags_Camera/Player`; refreshed on the same
    /// 8-frame cadence as `ZoneCull.cpp::Update` and consumed by weather,
    /// police and camera adapters.
    pub current_flags_camera: i32,
    pub current_flags_player: i32,
    pub current_wanted_level_drop_player: i32,
    /// C++ `CCullZones::bCurrentSubwayIsInvisible`.
    pub current_subway_is_invisible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CResolvedCullZone {
    pub position: [f32; 3],
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub index_start: i32,
    pub group_index_count: [i16; 3],
    pub num_buildings: i16,
    pub num_treadables_plus_10m: i16,
    pub num_treadables: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CResolvedAttributeZone {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub attributes: i16,
    pub wanted_level: i16,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CVisibilityCandidatePlan {
    pub zone_index: usize,
    pub building_pool_indices: Vec<usize>,
    pub treadable_plus_10m_pool_indices: Vec<usize>,
    pub treadable_pool_indices: Vec<usize>,
    pub test_points: Vec<[f32; 3]>,
    pub bounding_probes: Vec<CVisibilityBoundingProbe>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CVisibilityBoundingProbe {
    pub pool_kind: CVisibilityPoolKind,
    pub pool_index: usize,
    pub extra_dist: f32,
    pub vec_min: [f32; 3],
    pub dir_x: [f32; 3],
    pub dir_y: [f32; 3],
    pub dir_z: [f32; 3],
    pub intersects_zone: bool,
    pub ray_sample_counts: [usize; 3],
    pub center_line_clear: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CVisibilityPoolKind {
    Building,
    TreadablePlus10m,
    Treadable,
}

impl Default for CWorld {
    fn default() -> Self {
        Self::initialise()
    }
}

impl CWorld {
    pub fn initialise() -> Self {
        Self {
            zones: CTheZones::init(),
            cull_zones: CCullZones::init(),
            sectors: vec![CWorldSector::default(); NUMSECTORS_X * NUMSECTORS_Y],
            entities: Vec::new(),
            building_pool: Vec::new(),
            treadable_pool: Vec::new(),
            map_objects: Vec::new(),
            object_dummy_pool: Vec::new(),
            promoted_object_dummies: HashSet::new(),
            ecs_lifecycle_objects: HashMap::new(),
            big_buildings_by_level: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            pickups: Vec::new(),
            process_cutscene_only: false,
            model_swap_epoch: 0,
            pending_model_replacements: Vec::new(),
        }
    }

    pub fn apply_ipl_plan(
        &mut self,
        plan: &GtaIplPlan,
        map_zones_only: bool,
        model_info: &CModelInfo,
        path_object_model_ids: &BTreeSet<usize>,
    ) {
        for record in &plan.records {
            match record {
                GtaIplRecord::Zone(zone) => {
                    if map_zones_only {
                        self.zones.create_map_zone(zone);
                    } else {
                        self.zones.create_zone(zone);
                    }
                }
                GtaIplRecord::Instance(instance) if !map_zones_only => {
                    self.load_object_instance(instance, model_info, path_object_model_ids);
                }
                GtaIplRecord::Cull(cull_zone) if !map_zones_only => {
                    self.cull_zones.add_cull_zone(cull_zone.clone());
                }
                GtaIplRecord::Pickup(pickup) if !map_zones_only => {
                    self.pickups.push(pickup.clone());
                }
                _ => {}
            }
        }
    }

    pub fn load_object_instance(
        &mut self,
        instance: &GtaIplInstance,
        model_info: &CModelInfo,
        path_object_model_ids: &BTreeSet<usize>,
    ) -> Option<usize> {
        let simple = model_info.simple_model_info(instance.id)?;
        let is_building = simple.base.object_id == -1;
        let is_path_object = is_building && path_object_model_ids.contains(&instance.id);
        let building_pool_slot =
            (is_building && !is_path_object).then_some(self.building_pool.len());
        let treadable_pool_slot = is_path_object.then_some(self.treadable_pool.len());
        let map_object_slot = is_path_object.then_some(self.map_objects.len());
        let mut entity = CWorldEntity {
            entity_type: if is_building {
                EEntityType::Building
            } else {
                EEntityType::Dummy
            },
            model_index: instance.id,
            name: instance.name.clone(),
            position: instance.position,
            scale: instance.scale,
            rotation_axis: instance.rotation_axis,
            rotation_w: instance.angle,
            rotation_degrees: -((2.0 * instance.angle.acos()) * 180.0 / std::f32::consts::PI),
            matrix_basis_override: None,
            render_position_offset: [0.0; 3],
            level: self.zones.get_level_from_position(instance.position),
            is_static: true,
            // re3 FileLoader::LoadObjectInstance: buildings with a largest LOD
            // distance below 2.0 start invisible; glass dummies start
            // invisible (IsGlass). Rust previously forced both visible.
            is_visible: if is_building {
                simple.GetLargestLodDistance() >= 2.0
            } else {
                !is_glass_dummy(&instance.name)
            },
            is_big_building: false,
            is_subway: false,
            streaming_dont_delete: false,
            // re3 creates OBJECT.DAT-backed IPL instances as CDummyObject.
            // Dummies do not participate in physical collision until
            // CPopulation::ConvertToRealObject replaces them with CObject.
            uses_collision: is_building,
            zone_culled: false,
            zone_culled2: false,
            building_pool_slot,
            treadable_pool_slot,
            map_object_slot,
        };

        if entity.entity_type == EEntityType::Building {
            if simple.is_big_building {
                setup_big_building(&mut entity, simple.lod_distances[0]);
            }
            if simple.flags.is_subway {
                entity.is_subway = true;
            }
        }

        let index = self.entities.len();
        self.entities.push(entity);
        if is_path_object {
            self.treadable_pool.push(index);
            self.map_objects.push(index);
        } else if is_building {
            self.building_pool.push(index);
        } else {
            self.object_dummy_pool.push(index);
        }
        self.add(index, model_info);
        Some(index)
    }

    /// re3 `CPopulation::ManagePopulation` map-object conversion, represented
    /// without reallocating the stable Rust world entity.
    ///
    /// In C++ OBJECT.DAT-backed IPL instances start as collisionless
    /// `CDummyObject`s. One 1/32 pool slice per frame is converted to a real
    /// `CObject` inside 80 m (`ConvertToRealObject`) and converted back to a
    /// dummy outside it (`ConvertToDummyObject`). Rust keeps the stable world
    /// entity: while promoted it is hidden and collisionless and the live ECS
    /// CObject spawned by `process_map_object_collision_lifecycle` owns the
    /// render slot, uproot thresholds, `ObjectDamage` and debris. The returned
    /// changes drive that ECS side of the conversion.
    pub fn update_object_dummy_collision_lifecycle(
        &mut self,
        player_position: CVector,
        frame_counter: u32,
        model_info: &CModelInfo,
    ) -> Vec<ObjectDummyLifecycleChange> {
        let mut changes = Vec::new();
        let count = self.object_dummy_pool.len();
        if count == 0 {
            return changes;
        }
        let slice = (frame_counter & 31) as usize;
        let start = count * slice / 32;
        let end = count * (slice + 1) / 32;
        for pool_index in start..end {
            let Some(&entity_index) = self.object_dummy_pool.get(pool_index) else {
                continue;
            };
            if self.promoted_object_dummies.contains(&entity_index) {
                continue;
            }
            let Some(entity) = self.entities.get(entity_index).cloned() else {
                continue;
            };
            let has_object_dat = model_info
                .simple_model_info(entity.model_index)
                .is_some_and(|simple| simple.base.object_id != -1);
            let dx = entity.position[0] - player_position.x;
            let dy = entity.position[1] - player_position.y;
            let dz = entity.position[2] - player_position.z;
            let should_be_object = has_object_dat && dx * dx + dy * dy + dz * dz < 80.0 * 80.0;
            let is_object = entity.entity_type == EEntityType::Object;

            if should_be_object == is_object {
                // The world entity never owns collision for OBJECT.DAT
                // instances: dummies are collisionless and promoted instances
                // delegate to the live ECS CObject.
                if let Some(entity) = self.entities.get_mut(entity_index) {
                    entity.uses_collision = false;
                }
                continue;
            }

            self.remove_from_sector_lists(entity_index, &entity, model_info);
            if let Some(entity_slot) = self.entities.get_mut(entity_index) {
                if should_be_object {
                    entity_slot.entity_type = EEntityType::Object;
                    entity_slot.uses_collision = false;
                    entity_slot.is_visible = false;
                    changes.push(ObjectDummyLifecycleChange::Promoted {
                        entity_index,
                        model_index: entity.model_index,
                    });
                } else {
                    entity_slot.entity_type = EEntityType::Dummy;
                    entity_slot.uses_collision = false;
                    // re3 ConvertToDummyObject restores the authored placement
                    // matrix; only glass panes stay invisible.
                    entity_slot.is_visible = !is_glass_dummy(&entity_slot.name);
                    changes.push(ObjectDummyLifecycleChange::Demoted { entity_index });
                }
            }
            self.add(entity_index, model_info);
        }
        changes
    }

    /// Reverts a promotion the population pass could not complete (re3
    /// `TestSafeForRealObject` rejection or object-pool exhaustion). The
    /// world entity becomes a visible collisionless dummy again and the next
    /// 1/32 slice retries the conversion.
    pub fn abort_object_dummy_promotion(&mut self, entity_index: usize, model_info: &CModelInfo) {
        let Some(entity) = self.entities.get(entity_index).cloned() else {
            return;
        };
        if entity.entity_type != EEntityType::Object {
            return;
        }
        self.remove_from_sector_lists(entity_index, &entity, model_info);
        if let Some(entity_slot) = self.entities.get_mut(entity_index) {
            entity_slot.entity_type = EEntityType::Dummy;
            entity_slot.uses_collision = false;
            entity_slot.is_visible = !is_glass_dummy(&entity_slot.name);
        }
        self.add(entity_index, model_info);
    }

    fn remove_from_sector_lists(
        &mut self,
        entity_index: usize,
        entity: &CWorldEntity,
        model_info: &CModelInfo,
    ) {
        if entity.is_big_building {
            let level = entity.level.clamp(0, 3) as usize;
            self.big_buildings_by_level[level].retain(|&index| index != entity_index);
            return;
        }

        let remove_from = |sector: &mut CWorldSector, overlap: bool| {
            let list = match (entity.entity_type, overlap) {
                (EEntityType::Building, false) => &mut sector.buildings,
                (EEntityType::Building, true) => &mut sector.buildings_overlap,
                (EEntityType::Dummy, false) => &mut sector.dummies,
                (EEntityType::Dummy, true) => &mut sector.dummies_overlap,
                (EEntityType::Object, false) => &mut sector.objects,
                (EEntityType::Object, true) => &mut sector.objects_overlap,
                (EEntityType::Vehicle, false) => &mut sector.vehicles,
                (EEntityType::Vehicle, true) => &mut sector.vehicles_overlap,
                (EEntityType::Ped, false) => &mut sector.peds,
                (EEntityType::Ped, true) => &mut sector.peds_overlap,
                (EEntityType::Nothing, _) => return,
            };
            list.retain(|&index| index != entity_index);
        };

        let Some(bounds) = entity.get_bound_rect(model_info) else {
            let x = Self::get_sector_index_x(entity.position[0]);
            let y = Self::get_sector_index_y(entity.position[1]);
            remove_from(&mut self.sectors[y * NUMSECTORS_X + x], false);
            return;
        };

        let xstart = Self::get_sector_index_x(bounds[0]);
        let xend = Self::get_sector_index_x(bounds[1]);
        let xmid = Self::get_sector_index_x((bounds[0] + bounds[1]) / 2.0);
        let ystart = Self::get_sector_index_y(bounds[2]);
        let yend = Self::get_sector_index_y(bounds[3]);
        let ymid = Self::get_sector_index_y((bounds[2] + bounds[3]) / 2.0);

        for y in ystart..=yend {
            for x in xstart..=xend {
                remove_from(
                    &mut self.sectors[y * NUMSECTORS_X + x],
                    x != xmid || y != ymid,
                );
            }
        }
    }

    pub fn add(&mut self, entity_index: usize, model_info: &CModelInfo) {
        let entity = &self.entities[entity_index];
        if entity.is_big_building {
            let level = entity.level.clamp(0, 3) as usize;
            // re3 CWorld stores these in CPtrList and InsertItem inserts at
            // the head, so iteration is reverse insertion order.
            self.big_buildings_by_level[level].insert(0, entity_index);
            return;
        }

        let Some(bounds) = entity.get_bound_rect(model_info) else {
            let x = Self::get_sector_index_x(entity.position[0]);
            let y = Self::get_sector_index_y(entity.position[1]);
            let sector = &mut self.sectors[y * NUMSECTORS_X + x];
            match entity.entity_type {
                EEntityType::Building => sector.buildings.insert(0, entity_index),
                EEntityType::Dummy => sector.dummies.insert(0, entity_index),
                EEntityType::Object => sector.objects.insert(0, entity_index),
                EEntityType::Vehicle => sector.vehicles.insert(0, entity_index),
                EEntityType::Ped => sector.peds.insert(0, entity_index),
                EEntityType::Nothing => {}
            }
            return;
        };

        let xstart = Self::get_sector_index_x(bounds[0]);
        let xend = Self::get_sector_index_x(bounds[1]);
        let xmid = Self::get_sector_index_x((bounds[0] + bounds[1]) / 2.0);
        let ystart = Self::get_sector_index_y(bounds[2]);
        let yend = Self::get_sector_index_y(bounds[3]);
        let ymid = Self::get_sector_index_y((bounds[2] + bounds[3]) / 2.0);

        for y in ystart..=yend {
            for x in xstart..=xend {
                let sector = &mut self.sectors[y * NUMSECTORS_X + x];
                if x == xmid && y == ymid {
                    match entity.entity_type {
                        EEntityType::Building => sector.buildings.insert(0, entity_index),
                        EEntityType::Dummy => sector.dummies.insert(0, entity_index),
                        EEntityType::Object => sector.objects.insert(0, entity_index),
                        EEntityType::Vehicle => sector.vehicles.insert(0, entity_index),
                        EEntityType::Ped => sector.peds.insert(0, entity_index),
                        EEntityType::Nothing => {}
                    }
                } else {
                    match entity.entity_type {
                        EEntityType::Building => sector.buildings_overlap.insert(0, entity_index),
                        EEntityType::Dummy => sector.dummies_overlap.insert(0, entity_index),
                        EEntityType::Object => sector.objects_overlap.insert(0, entity_index),
                        EEntityType::Vehicle => sector.vehicles_overlap.insert(0, entity_index),
                        EEntityType::Ped => sector.peds_overlap.insert(0, entity_index),
                        EEntityType::Nothing => {}
                    }
                }
            }
        }
    }

    pub fn get_sector_index_x(x: f32) -> usize {
        (((x - WORLD_MIN_X) / SECTOR_SIZE_X).floor() as isize).clamp(0, NUMSECTORS_X as isize - 1)
            as usize
    }

    pub fn get_sector_index_y(y: f32) -> usize {
        (((y - WORLD_MIN_Y) / SECTOR_SIZE_Y).floor() as isize).clamp(0, NUMSECTORS_Y as isize - 1)
            as usize
    }

    /// re3 `CWorld::FindObjectsIntersectingCube` boolean fast path.
    ///
    /// The native query walks only sectors touched by the cube and tests each
    /// candidate's entity origin plus bound radius. Script predicates only ask
    /// whether at least one entity exists (their native max result count is 2),
    /// so duplicate overlap-list entries do not need a scan-code allocation.
    pub fn any_entity_intersects_cube(
        &self,
        min: CVector,
        max: CVector,
        model_info: &CModelInfo,
        entity_types: [bool; 5],
    ) -> bool {
        let xstart = Self::get_sector_index_x(min.x);
        let xend = Self::get_sector_index_x(max.x);
        let ystart = Self::get_sector_index_y(min.y);
        let yend = Self::get_sector_index_y(max.y);

        let intersects = |entity: &CWorldEntity| {
            let radius = model_info
                .col_model(entity.model_index)
                .map_or(0.0, |col| col.bounding_sphere.radius);
            entity.position[0] + radius >= min.x
                && entity.position[0] - radius <= max.x
                && entity.position[1] + radius >= min.y
                && entity.position[1] - radius <= max.y
                && entity.position[2] + radius >= min.z
                && entity.position[2] - radius <= max.z
        };

        for y in ystart..=yend.min(NUMSECTORS_Y - 1) {
            for x in xstart..=xend.min(NUMSECTORS_X - 1) {
                let sector = &self.sectors[y * NUMSECTORS_X + x];
                let lists: [(&[usize], bool); 10] = [
                    (&sector.buildings, entity_types[0]),
                    (&sector.buildings_overlap, entity_types[0]),
                    (&sector.vehicles, entity_types[1]),
                    (&sector.vehicles_overlap, entity_types[1]),
                    (&sector.peds, entity_types[2]),
                    (&sector.peds_overlap, entity_types[2]),
                    (&sector.objects, entity_types[3]),
                    (&sector.objects_overlap, entity_types[3]),
                    (&sector.dummies, entity_types[4]),
                    (&sector.dummies_overlap, entity_types[4]),
                ];
                for (list, enabled) in lists {
                    if !enabled {
                        continue;
                    }
                    for &entity_index in list {
                        if self.entities.get(entity_index).is_some_and(&intersects) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn force_cull_zone_coors(&mut self, coors: [f32; 3]) -> CullZoneTransition {
        let transition = self.cull_zones.force_cull_zone_coors(coors);
        if let Some(zone_index) = transition.left_zone {
            self.do_stuff_leaving_zone(zone_index);
        }
        if let Some(zone_index) = transition.entered_zone {
            self.do_stuff_entering_zone(zone_index);
        }
        transition
    }

    /// re3 `CCullZones::MarkSubwayAsInvisible` static-world half. The train
    /// pool is not represented by `CWorldEntity`; buildings and treadables
    /// retain their IPL `bIsSubway` bit here and are toggled together.
    pub fn mark_subway_as_visible(&mut self, visible: bool) {
        for entity in &mut self.entities {
            if entity.is_subway {
                entity.is_visible = visible;
            }
        }
    }

    pub fn build_list_for_big_buildings(&mut self, model_info: &CModelInfo) {
        self.cull_zones.reset_big_building_pointers();

        for building_pool_index in (0..self.building_pool.len()).rev() {
            let Some(big_building_index) = self.building_pool.get(building_pool_index).copied()
            else {
                continue;
            };
            let big_building = &self.entities[big_building_index];
            if !big_building.is_big_building {
                continue;
            }
            let Some(related_model_id) =
                model_info.simple_related_model_id(big_building.model_index)
            else {
                continue;
            };

            for other_pool_index in (0..self.building_pool.len()).rev() {
                let Some(other_index) = self.building_pool.get(other_pool_index).copied() else {
                    continue;
                };
                let other = &self.entities[other_index];
                if other.is_big_building || other.model_index != related_model_id {
                    continue;
                }
                if distance(other.position, big_building.position) < 5.0 {
                    if let Some(pointer) = self
                        .cull_zones
                        .building_to_big_building
                        .get_mut(other_pool_index)
                    {
                        *pointer = building_pool_index as i16;
                    }
                }
            }

            for treadable_pool_index in (0..self.treadable_pool.len()).rev() {
                let Some(treadable_index) = self.treadable_pool.get(treadable_pool_index).copied()
                else {
                    continue;
                };
                let treadable = &self.entities[treadable_index];
                if treadable.is_big_building || treadable.model_index != related_model_id {
                    continue;
                }
                if distance(treadable.position, big_building.position) < 5.0 {
                    if let Some(pointer) = self
                        .cull_zones
                        .treadable_to_big_building
                        .get_mut(treadable_pool_index)
                    {
                        *pointer = building_pool_index as i16;
                    }
                }
            }
        }
    }

    pub fn prepare_cull_zone_visibility_candidates_without_cache(
        &mut self,
        model_info: &CModelInfo,
        general: &mut CGeneral,
    ) {
        self.cull_zones.prepare_resolution_zones_from_ipl();
        self.build_list_for_big_buildings(model_info);

        let mut plans = Vec::with_capacity(self.cull_zones.resolved_zones.len());
        let mut temp_indices = Vec::new();
        for zone_index in 0..self.cull_zones.resolved_zones.len() {
            let plan = self.collect_visibility_test_candidates(zone_index, model_info);
            let (group_index_count, zone_indices) =
                self.do_visibility_test_cull_zone(zone_index, model_info);
            if let Some(zone) = self.cull_zones.resolved_zones.get_mut(zone_index) {
                zone.index_start = temp_indices.len() as i32;
                zone.group_index_count = group_index_count;
                zone.num_buildings = group_index_count[0];
                zone.num_treadables_plus_10m = group_index_count[1];
                zone.num_treadables = group_index_count[2];
            }
            temp_indices.extend(zone_indices);
            plans.push(plan);
        }
        self.cull_zones
            .compress_indices_array_from_temp(temp_indices, general);
        self.cull_zones.visibility_candidate_plans = plans;
    }

    fn do_visibility_test_cull_zone(
        &self,
        zone_index: usize,
        model_info: &CModelInfo,
    ) -> ([i16; 3], Vec<u16>) {
        let Some(zone) = self.cull_zones.resolved_zones.get(zone_index) else {
            return ([0; 3], Vec::new());
        };

        let test_points = self.find_cull_zone_test_points(zone, model_info);
        let mut group_index_count = [0i16; 3];
        let mut indices = Vec::new();
        let mut invisible_treadables_plus_10m = BTreeSet::new();

        for pool_index in (0..self.building_pool.len()).rev() {
            let Some(entity_index) = self.building_pool.get(pool_index).copied() else {
                continue;
            };
            let entity = &self.entities[entity_index];
            if entity.is_big_building {
                continue;
            }
            let check_level = self
                .cull_zones
                .building_to_big_building
                .get(pool_index)
                .is_some_and(|big_building| *big_building != -1);
            if !self.is_entity_close_enough_to_cull_zone(zone, entity, check_level, model_info) {
                continue;
            }
            if !self.test_entity_visibility_from_cull_zone(
                zone,
                &test_points,
                entity_index,
                0.0,
                self.big_building_for_building_pool_index(pool_index),
                model_info,
            ) {
                if let Ok(index) = u16::try_from(pool_index) {
                    indices.push(index);
                    group_index_count[0] += 1;
                }
            }
        }

        for pool_index in (0..self.treadable_pool.len()).rev() {
            let Some(entity_index) = self.treadable_pool.get(pool_index).copied() else {
                continue;
            };
            let entity = &self.entities[entity_index];
            let check_level = self
                .cull_zones
                .treadable_to_big_building
                .get(pool_index)
                .is_some_and(|big_building| *big_building != -1);
            if !self.is_entity_close_enough_to_cull_zone(zone, entity, check_level, model_info) {
                continue;
            }
            if !self.test_entity_visibility_from_cull_zone(
                zone,
                &test_points,
                entity_index,
                10.0,
                self.big_building_for_treadable_pool_index(pool_index),
                model_info,
            ) {
                if let Ok(index) = u16::try_from(pool_index) {
                    indices.push(index);
                    invisible_treadables_plus_10m.insert(pool_index);
                    group_index_count[1] += 1;
                }
            }
        }

        for pool_index in (0..self.treadable_pool.len()).rev() {
            let Some(entity_index) = self.treadable_pool.get(pool_index).copied() else {
                continue;
            };
            let entity = &self.entities[entity_index];
            if zone.calc_dist_to_cull_zone_squared(entity.position[0], entity.position[1])
                >= 200.0 * 200.0
            {
                continue;
            }
            if invisible_treadables_plus_10m.contains(&pool_index) {
                continue;
            }
            if !self.test_entity_visibility_from_cull_zone(
                zone,
                &test_points,
                entity_index,
                0.0,
                self.big_building_for_treadable_pool_index(pool_index),
                model_info,
            ) {
                if let Ok(index) = u16::try_from(pool_index) {
                    indices.push(index);
                    group_index_count[2] += 1;
                }
            }
        }

        (group_index_count, indices)
    }

    fn collect_visibility_test_candidates(
        &self,
        zone_index: usize,
        model_info: &CModelInfo,
    ) -> CVisibilityCandidatePlan {
        let Some(zone) = self.cull_zones.resolved_zones.get(zone_index) else {
            return CVisibilityCandidatePlan {
                zone_index,
                ..Default::default()
            };
        };
        let mut plan = CVisibilityCandidatePlan {
            zone_index,
            test_points: self.find_cull_zone_test_points(zone, model_info),
            ..Default::default()
        };

        for pool_index in (0..self.building_pool.len()).rev() {
            let Some(entity_index) = self.building_pool.get(pool_index).copied() else {
                continue;
            };
            let entity = &self.entities[entity_index];
            if entity.is_big_building {
                continue;
            }
            let check_level = self
                .cull_zones
                .building_to_big_building
                .get(pool_index)
                .is_some_and(|big_building| *big_building != -1);
            if self.is_entity_close_enough_to_cull_zone(zone, entity, check_level, model_info) {
                plan.building_pool_indices.push(pool_index);
                if let Some(probe) = self.visibility_bounding_probe_for_entity(
                    zone,
                    CVisibilityPoolKind::Building,
                    pool_index,
                    entity,
                    0.0,
                    self.big_building_for_building_pool_index(pool_index),
                    model_info,
                ) {
                    plan.bounding_probes.push(probe);
                }
            }
        }

        for pool_index in (0..self.treadable_pool.len()).rev() {
            let Some(entity_index) = self.treadable_pool.get(pool_index).copied() else {
                continue;
            };
            let entity = &self.entities[entity_index];
            let check_level = self
                .cull_zones
                .treadable_to_big_building
                .get(pool_index)
                .is_some_and(|big_building| *big_building != -1);
            if self.is_entity_close_enough_to_cull_zone(zone, entity, check_level, model_info) {
                plan.treadable_plus_10m_pool_indices.push(pool_index);
                if let Some(probe) = self.visibility_bounding_probe_for_entity(
                    zone,
                    CVisibilityPoolKind::TreadablePlus10m,
                    pool_index,
                    entity,
                    10.0,
                    self.big_building_for_treadable_pool_index(pool_index),
                    model_info,
                ) {
                    plan.bounding_probes.push(probe);
                }
            }
        }

        for pool_index in (0..self.treadable_pool.len()).rev() {
            let Some(entity_index) = self.treadable_pool.get(pool_index).copied() else {
                continue;
            };
            let entity = &self.entities[entity_index];
            if zone.calc_dist_to_cull_zone_squared(entity.position[0], entity.position[1])
                >= 200.0 * 200.0
            {
                continue;
            }
            if plan.treadable_plus_10m_pool_indices.contains(&pool_index) {
                continue;
            }
            plan.treadable_pool_indices.push(pool_index);
            if let Some(probe) = self.visibility_bounding_probe_for_entity(
                zone,
                CVisibilityPoolKind::Treadable,
                pool_index,
                entity,
                0.0,
                self.big_building_for_treadable_pool_index(pool_index),
                model_info,
            ) {
                plan.bounding_probes.push(probe);
            }
        }

        plan
    }

    fn visibility_bounding_probe_for_entity(
        &self,
        zone: &CResolvedCullZone,
        pool_kind: CVisibilityPoolKind,
        pool_index: usize,
        entity: &CWorldEntity,
        extra_dist: f32,
        lod_entity_index: Option<usize>,
        model_info: &CModelInfo,
    ) -> Option<CVisibilityBoundingProbe> {
        let col_model = model_info.col_model(entity.model_index)?;
        let mut bounds = col_model.bounding_box.clone();
        if let Some(lod_entity_index) = lod_entity_index {
            let lod_entity = self.entities.get(lod_entity_index)?;
            if let Some(lod_col_model) = model_info.col_model(lod_entity.model_index) {
                merge_col_bounds(&mut bounds, &lod_col_model.bounding_box);
            }
        }

        if bounds.max.z - bounds.min.z + extra_dist < 0.5 {
            bounds.max.z = bounds.min.z + 0.5;
        } else {
            bounds.max.z += extra_dist;
        }

        let vec_min = transform_entity_point(entity, [bounds.min.x, bounds.min.y, bounds.min.z]);
        let vec_max_x = transform_entity_point(entity, [bounds.max.x, bounds.min.y, bounds.min.z]);
        let vec_max_y = transform_entity_point(entity, [bounds.min.x, bounds.max.y, bounds.min.z]);
        let vec_max_z = transform_entity_point(entity, [bounds.min.x, bounds.min.y, bounds.max.z]);
        let dir_x = sub_vec3(vec_max_x, vec_min);
        let dir_y = sub_vec3(vec_max_y, vec_min);
        let dir_z = sub_vec3(vec_max_z, vec_min);
        let intersects_zone = bounding_probe_intersects_zone(zone, vec_min, dir_x, dir_y, dir_z);

        let dist_to_zone = zone
            .calc_dist_to_cull_zone(entity.position[0], entity.position[1])
            .max(7.0 * 15.0)
            / 15.0;
        let ray_sample_counts = [
            (((bounds.max.x - bounds.min.x) / dist_to_zone) + 2.0) as usize,
            (((bounds.max.y - bounds.min.y) / dist_to_zone) + 2.0) as usize,
            (((bounds.max.z - bounds.min.z) / dist_to_zone) + 2.0) as usize,
        ];
        let mid = add_vec3(
            transform_entity_point(
                entity,
                [
                    (bounds.max.x + bounds.min.x) / 2.0,
                    (bounds.max.y + bounds.min.y) / 2.0,
                    (bounds.max.z + bounds.min.z) / 2.0,
                ],
            ),
            [0.0, 0.0, 0.1],
        );
        let center_line_clear = Some(self.do_thorough_line_test_buildings(
            zone.position,
            mid,
            self.entity_index_for_visibility_probe(pool_kind, pool_index)?,
            model_info,
        ));

        Some(CVisibilityBoundingProbe {
            pool_kind,
            pool_index,
            extra_dist,
            vec_min,
            dir_x,
            dir_y,
            dir_z,
            intersects_zone,
            ray_sample_counts,
            center_line_clear,
        })
    }

    fn test_entity_visibility_from_cull_zone(
        &self,
        zone: &CResolvedCullZone,
        test_points: &[[f32; 3]],
        entity_index: usize,
        extra_dist: f32,
        lod_entity_index: Option<usize>,
        model_info: &CModelInfo,
    ) -> bool {
        let Some(entity) = self.entities.get(entity_index) else {
            return false;
        };
        let Some(mut bounds) = model_info
            .col_model(entity.model_index)
            .map(|col_model| col_model.bounding_box.clone())
        else {
            return false;
        };
        if let Some(lod_entity_index) = lod_entity_index {
            if let Some(lod_entity) = self.entities.get(lod_entity_index) {
                if let Some(lod_col_model) = model_info.col_model(lod_entity.model_index) {
                    merge_col_bounds(&mut bounds, &lod_col_model.bounding_box);
                }
            }
        }

        if bounds.max.z - bounds.min.z + extra_dist < 0.5 {
            bounds.max.z = bounds.min.z + 0.5;
        } else {
            bounds.max.z += extra_dist;
        }

        let vec_min = transform_entity_point(entity, [bounds.min.x, bounds.min.y, bounds.min.z]);
        let vec_max_x = transform_entity_point(entity, [bounds.max.x, bounds.min.y, bounds.min.z]);
        let vec_max_y = transform_entity_point(entity, [bounds.min.x, bounds.max.y, bounds.min.z]);
        let vec_max_z = transform_entity_point(entity, [bounds.min.x, bounds.min.y, bounds.max.z]);
        let dir_x = sub_vec3(vec_max_x, vec_min);
        let dir_y = sub_vec3(vec_max_y, vec_min);
        let dir_z = sub_vec3(vec_max_z, vec_min);

        if bounding_probe_intersects_zone(zone, vec_min, dir_x, dir_y, dir_z) {
            return true;
        }

        let dist_to_zone =
            (zone.calc_dist_to_cull_zone(entity.position[0], entity.position[1]) / 15.0).max(7.0);
        let num_x = (((bounds.max.x - bounds.min.x) / dist_to_zone) + 2.0) as usize;
        let num_y = (((bounds.max.y - bounds.min.y) / dist_to_zone) + 2.0) as usize;
        let num_z = (((bounds.max.z - bounds.min.z) / dist_to_zone) + 2.0) as usize;
        if num_x < 2 || num_y < 2 || num_z < 2 {
            return false;
        }

        let step_x = 1.0 / (num_x as f32 - 1.0);
        let step_y = 1.0 / (num_y as f32 - 1.0);
        let step_z = 1.0 / (num_z as f32 - 1.0);
        let mid = add_vec3(
            transform_entity_point(
                entity,
                [
                    (bounds.max.x + bounds.min.x) / 2.0,
                    (bounds.max.y + bounds.min.y) / 2.0,
                    (bounds.max.z + bounds.min.z) / 2.0,
                ],
            ),
            [0.0, 0.0, 0.1],
        );

        for test_point in test_points {
            if self.do_thorough_line_test_buildings(*test_point, mid, entity_index, model_info) {
                return true;
            }

            let ray = sub_vec3(entity.position, *test_point);
            let dot_x = dot_vec3(ray, dir_x);
            let dot_y = dot_vec3(ray, dir_y);
            let dot_z = dot_vec3(ray, dir_z);

            for x in 0..num_x {
                let p_min_z = add_vec3(vec_min, mul_vec3(dir_x, x as f32 * step_x));
                let p_max_z = add_vec3(p_min_z, dir_z);
                for y in 0..num_y {
                    let target = if dot_z > 0.0 {
                        add_vec3(p_min_z, mul_vec3(dir_y, y as f32 * step_y))
                    } else {
                        add_vec3(p_max_z, mul_vec3(dir_y, y as f32 * step_y))
                    };
                    if self.do_thorough_line_test_buildings(
                        *test_point,
                        target,
                        entity_index,
                        model_info,
                    ) {
                        return true;
                    }
                }
            }

            for x in 0..num_x {
                let p_min_y = add_vec3(vec_min, mul_vec3(dir_x, x as f32 * step_x));
                let p_max_y = add_vec3(p_min_y, dir_y);
                for z in 1..num_z - 1 {
                    let target = if dot_y > 0.0 {
                        add_vec3(p_min_y, mul_vec3(dir_z, z as f32 * step_z))
                    } else {
                        add_vec3(p_max_y, mul_vec3(dir_z, z as f32 * step_z))
                    };
                    if self.do_thorough_line_test_buildings(
                        *test_point,
                        target,
                        entity_index,
                        model_info,
                    ) {
                        return true;
                    }
                }
            }

            for y in 1..num_y - 1 {
                let p_min_x = add_vec3(vec_min, mul_vec3(dir_y, y as f32 * step_y));
                let p_max_x = add_vec3(p_min_x, dir_x);
                for z in 1..num_z - 1 {
                    let target = if dot_x > 0.0 {
                        add_vec3(p_min_x, mul_vec3(dir_z, z as f32 * step_z))
                    } else {
                        add_vec3(p_max_x, mul_vec3(dir_z, z as f32 * step_z))
                    };
                    if self.do_thorough_line_test_buildings(
                        *test_point,
                        target,
                        entity_index,
                        model_info,
                    ) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn entity_index_for_visibility_probe(
        &self,
        pool_kind: CVisibilityPoolKind,
        pool_index: usize,
    ) -> Option<usize> {
        match pool_kind {
            CVisibilityPoolKind::Building => self.building_pool.get(pool_index).copied(),
            CVisibilityPoolKind::TreadablePlus10m | CVisibilityPoolKind::Treadable => {
                self.treadable_pool.get(pool_index).copied()
            }
        }
    }

    fn big_building_for_building_pool_index(&self, pool_index: usize) -> Option<usize> {
        let big_pool_index = *self.cull_zones.building_to_big_building.get(pool_index)?;
        (big_pool_index != -1)
            .then_some(big_pool_index as usize)
            .and_then(|index| self.building_pool.get(index).copied())
    }

    fn big_building_for_treadable_pool_index(&self, pool_index: usize) -> Option<usize> {
        let big_pool_index = *self.cull_zones.treadable_to_big_building.get(pool_index)?;
        (big_pool_index != -1)
            .then_some(big_pool_index as usize)
            .and_then(|index| self.building_pool.get(index).copied())
    }

    fn is_entity_close_enough_to_cull_zone(
        &self,
        zone: &CResolvedCullZone,
        entity: &CWorldEntity,
        check_level: bool,
        model_info: &CModelInfo,
    ) -> bool {
        let Some(simple) = model_info.simple_model_info(entity.model_index) else {
            return false;
        };
        let dist_to_zone = zone.calc_dist_to_cull_zone(entity.position[0], entity.position[1]);
        let lod_dist = if simple.flags.no_fade {
            largest_lod_distance(simple.lod_distances) + STREAM_DISTANCE
        } else {
            largest_lod_distance(simple.lod_distances) + STREAM_DISTANCE + FADE_DISTANCE
        };

        if lod_dist > dist_to_zone {
            return true;
        }
        if !check_level {
            return false;
        }
        self.zones.get_level_from_position(entity.position)
            == self.zones.get_level_from_position(zone.min)
    }

    fn find_cull_zone_test_points(
        &self,
        zone: &CResolvedCullZone,
        model_info: &CModelInfo,
    ) -> Vec<[f32; 3]> {
        const FLAG_FREE: u8 = 1;
        const FLAG_PROCESSED: u8 = 2;

        let elements_x = zone_grid_elements(zone.max[0] - zone.min[0]);
        let elements_y = zone_grid_elements(zone.max[1] - zone.min[1]);
        let elements_z = zone_grid_elements(zone.max[2] - zone.min[2]);
        let step_x = (zone.max[0] - zone.min[0]) / (elements_x as f32 - 1.0);
        let step_y = (zone.max[1] - zone.min[1]) / (elements_y as f32 - 1.0);
        let step_z = (zone.max[2] - zone.min[2]) / (elements_z as f32 - 1.0);
        let mut mem = vec![0u8; elements_x * elements_y * elements_z];
        let index = |x: usize, y: usize, z: usize| (x * elements_y + y) * elements_z + z;
        let point = |x: usize, y: usize, z: usize| {
            [
                x as f32 * step_x + zone.min[0],
                y as f32 * step_y + zone.min[1],
                z as f32 * step_z + zone.min[2],
            ]
        };

        let start_x = grid_position(zone.position[0], zone.min[0], zone.max[0], elements_x);
        let start_y = grid_position(zone.position[1], zone.min[1], zone.max[1], elements_y);
        let start_z = grid_position(zone.position[2], zone.min[2], zone.max[2], elements_z);
        mem[index(start_x, start_y, start_z)] |= FLAG_FREE;

        let mut not_done_yet = true;
        while not_done_yet {
            not_done_yet = false;
            for x in 0..elements_x {
                for y in 0..elements_y {
                    for z in 0..elements_z {
                        let current = index(x, y, z);
                        if mem[current] & FLAG_FREE == 0 || mem[current] & FLAG_PROCESSED != 0 {
                            continue;
                        }
                        let p = point(x, y, z);
                        for (nx, ny, nz) in
                            grid_neighbours(x, y, z, elements_x, elements_y, elements_z)
                        {
                            let neighbour = index(nx, ny, nz);
                            if mem[neighbour] & (FLAG_FREE | FLAG_PROCESSED) != 0 {
                                continue;
                            }
                            if !self.test_line_buildings(p, point(nx, ny, nz), model_info) {
                                mem[neighbour] |= FLAG_FREE;
                            }
                        }
                        not_done_yet = true;
                        mem[current] |= FLAG_PROCESSED;
                    }
                }
            }
        }

        let mut grid_points = vec![(start_x, start_y, start_z)];
        add_plane_extrema(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            0,
            0,
        );
        add_plane_extrema(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            0,
            elements_x - 1,
        );
        add_plane_extrema(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            1,
            0,
        );
        add_plane_extrema(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            1,
            elements_y - 1,
        );
        add_plane_extrema(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            2,
            0,
        );
        add_plane_extrema(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            2,
            elements_z - 1,
        );
        add_plane_subdivide_points(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            0,
            0,
        );
        add_plane_subdivide_points(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            0,
            elements_x - 1,
        );
        add_plane_subdivide_points(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            1,
            0,
        );
        add_plane_subdivide_points(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            1,
            elements_y - 1,
        );
        add_plane_subdivide_points(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            2,
            0,
        );
        add_plane_subdivide_points(
            &mut grid_points,
            &mem,
            elements_x,
            elements_y,
            elements_z,
            2,
            elements_z - 1,
        );
        for point in EXTRA_FUDGE_POINTS {
            if zone.point_falls_within_zone(point, 0.0) {
                let x = grid_position(point[0], zone.min[0], zone.max[0], elements_x);
                let y = grid_position(point[1], zone.min[1], zone.max[1], elements_y);
                let z = grid_position(point[2], zone.min[2], zone.max[2], elements_z);
                if mem[index(x, y, z)] & FLAG_FREE != 0 {
                    grid_points.push((x, y, z));
                }
            }
        }
        dedup_grid_points(&mut grid_points);
        grid_points
            .into_iter()
            .take(100)
            .map(|(x, y, z)| point(x, y, z))
            .collect()
    }

    fn test_line_buildings(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
    ) -> bool {
        const OFFSETS: [[f32; 3]; 7] = [
            [0.0, 0.0, 0.0],
            [0.05, 0.0, 0.0],
            [-0.05, 0.0, 0.0],
            [0.0, 0.05, 0.0],
            [0.0, -0.05, 0.0],
            [0.0, 0.0, 0.05],
            [0.0, 0.0, -0.05],
        ];
        OFFSETS.iter().any(|offset| {
            self.process_line_of_sight_building_bounds(
                add_vec3(point1, *offset),
                add_vec3(point2, *offset),
                model_info,
                None,
            )
            .is_some()
        })
    }

    /// re3 `COMMAND_SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE` search order:
    /// normal sector building/object/dummy lists first, then generic big
    /// buildings, then the point's island big-building list.
    pub fn find_nearest_entity_of_model(
        &self,
        model_index: usize,
        position: [f32; 3],
        radius: f32,
    ) -> Option<usize> {
        self.find_nearest_script_model_entity(model_index, position, radius, true)
    }

    /// re3 `FindObjectsOfTypeInRange` candidate phase used by
    /// `COMMAND_GET_CLOSEST_OBJECT_OF_TYPE`: buildings, live world objects,
    /// then dummies; sector order and the caller's fixed candidate cap are
    /// observable because the native query stops after the cap.
    pub fn find_closest_object_command_candidates(
        &self,
        model_index: usize,
        position: CVector,
        radius: f32,
    ) -> Vec<usize> {
        const MAX_CANDIDATES: usize = 16;
        if radius <= 0.0 {
            return Vec::new();
        }

        let min_x = (((position.x - radius - WORLD_MIN_X) / SECTOR_SIZE_X) as i32).max(0) as usize;
        let max_x = (((position.x + radius - WORLD_MIN_X) / SECTOR_SIZE_X) as i32)
            .min(NUMSECTORS_X as i32 - 1);
        let min_y = (((position.y - radius - WORLD_MIN_Y) / SECTOR_SIZE_Y) as i32).max(0) as usize;
        let max_y = (((position.y + radius - WORLD_MIN_Y) / SECTOR_SIZE_Y) as i32)
            .min(NUMSECTORS_Y as i32 - 1);
        if max_x < 0 || max_y < 0 {
            return Vec::new();
        }
        let max_x = max_x as usize;
        let max_y = max_y as usize;
        let radius_sq = radius * radius;
        let mut scanned = HashSet::with_capacity(MAX_CANDIDATES);
        let mut candidates = Vec::with_capacity(MAX_CANDIDATES);

        'sectors: for y in min_y..=max_y {
            for x in min_x..=max_x {
                let sector = &self.sectors[y * NUMSECTORS_X + x];
                for list in [
                    &sector.buildings,
                    &sector.buildings_overlap,
                    &sector.objects,
                    &sector.objects_overlap,
                    &sector.dummies,
                    &sector.dummies_overlap,
                ] {
                    for &entity_index in list {
                        if !scanned.insert(entity_index) {
                            continue;
                        }

                        let Some(entity) = self.entities.get(entity_index) else {
                            continue;
                        };
                        if entity.model_index != model_index {
                            continue;
                        }
                        let dx = entity.position[0] - position.x;
                        let dy = entity.position[1] - position.y;
                        if dx * dx + dy * dy < radius_sq {
                            candidates.push(entity_index);
                            if candidates.len() == MAX_CANDIDATES {
                                break 'sectors;
                            }
                        }
                    }
                }
            }
        }
        candidates
    }

    /// Rebuild the native dummy matrix before `CObject(CDummyObject*)` takes
    /// ownership of its pose.
    pub fn matrix_for_static_entity(&self, entity_index: usize) -> Option<CMatrix> {
        let entity = self.entities.get(entity_index)?;
        let vector = |axis| {
            let transformed = transform_entity_vector(entity, axis);
            CVector::new(transformed[0], transformed[1], transformed[2])
        };
        Some(CMatrix {
            right: vector([1.0, 0.0, 0.0]),
            forward: vector([0.0, 1.0, 0.0]),
            up: vector([0.0, 0.0, 1.0]),
            position: CVector::new(entity.position[0], entity.position[1], entity.position[2]),
            has_rw_matrix: false,
        })
    }

    /// Remove a map dummy's static-world projection after a live CObject owns
    /// it. The ECS object supplies rendering and collision from this point on.
    pub fn retire_promoted_object_dummy(
        &mut self,
        entity_index: usize,
        model_info: &CModelInfo,
    ) -> bool {
        let Some(entity) = self.entities.get(entity_index).cloned() else {
            return false;
        };
        if !matches!(entity.entity_type, EEntityType::Dummy | EEntityType::Object) {
            return false;
        }
        self.remove_from_sector_lists(entity_index, &entity, model_info);
        let Some(entity) = self.entities.get_mut(entity_index) else {
            return false;
        };
        entity.entity_type = EEntityType::Nothing;
        entity.is_visible = false;
        entity.uses_collision = false;
        self.promoted_object_dummies.insert(entity_index);
        true
    }

    /// Building-only half of re3 `COMMAND_SWAP_NEAREST_BUILDING_MODEL`.
    /// `FindObjectsOfTypeInRange(..., bBuildings=true, ...)` never considers
    /// mission objects, peds or vehicles; using the generic query here could
    /// swap a coincident temporary object instead of the authored IPL bridge
    /// piece.
    pub fn find_nearest_building_of_model(
        &self,
        model_index: usize,
        position: [f32; 3],
        radius: f32,
    ) -> Option<usize> {
        self.find_nearest_script_model_entity(model_index, position, radius, false)
    }

    fn find_nearest_script_model_entity(
        &self,
        model_index: usize,
        position: [f32; 3],
        radius: f32,
        include_objects_and_dummies: bool,
    ) -> Option<usize> {
        const MAX_SCRIPT_MATCHES: usize = 16;

        let radius_sq = radius * radius;
        let ixmin = Self::get_sector_index_x(position[0] - radius);
        let ixmax = Self::get_sector_index_x(position[0] + radius);
        let iymin = Self::get_sector_index_y(position[1] - radius);
        let iymax = Self::get_sector_index_y(position[1] + radius);
        let mut scanned = vec![false; self.entities.len()];
        let mut matches = Vec::with_capacity(MAX_SCRIPT_MATCHES);

        for y in iymin..=iymax {
            for x in ixmin..=ixmax {
                let sector = &self.sectors[y * NUMSECTORS_X + x];
                self.collect_script_model_matches(
                    &sector.buildings,
                    model_index,
                    position,
                    radius_sq,
                    &mut scanned,
                    &mut matches,
                );
                self.collect_script_model_matches(
                    &sector.buildings_overlap,
                    model_index,
                    position,
                    radius_sq,
                    &mut scanned,
                    &mut matches,
                );
                if include_objects_and_dummies {
                    self.collect_script_model_matches(
                        &sector.objects,
                        model_index,
                        position,
                        radius_sq,
                        &mut scanned,
                        &mut matches,
                    );
                    self.collect_script_model_matches(
                        &sector.objects_overlap,
                        model_index,
                        position,
                        radius_sq,
                        &mut scanned,
                        &mut matches,
                    );
                    self.collect_script_model_matches(
                        &sector.dummies,
                        model_index,
                        position,
                        radius_sq,
                        &mut scanned,
                        &mut matches,
                    );
                    self.collect_script_model_matches(
                        &sector.dummies_overlap,
                        model_index,
                        position,
                        radius_sq,
                        &mut scanned,
                        &mut matches,
                    );
                }
            }
        }

        // Script4.cpp only falls back to big-building lists when the normal
        // sector query found nothing. Do not merge these categories and pick a
        // globally closer LOD: that can swap/hide the wrong authored entity.
        if matches.is_empty() {
            self.collect_script_model_matches(
                &self.big_buildings_by_level[0],
                model_index,
                position,
                radius_sq,
                &mut scanned,
                &mut matches,
            );
        }
        if matches.is_empty() {
            let level = self.zones.get_level_from_position(position).clamp(0, 3) as usize;
            self.collect_script_model_matches(
                &self.big_buildings_by_level[level],
                model_index,
                position,
                radius_sq,
                &mut scanned,
                &mut matches,
            );
        }

        let mut best = None;
        let mut min_dist_sq = (2.0 * radius) * (2.0 * radius);
        for entity_index in matches {
            let Some(entity) = self.entities.get(entity_index) else {
                continue;
            };
            let dx = entity.position[0] - position[0];
            let dy = entity.position[1] - position[1];
            let dz = entity.position[2] - position[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                best = Some(entity_index);
            }
        }
        best
    }

    fn collect_script_model_matches(
        &self,
        list: &[usize],
        model_index: usize,
        position: [f32; 3],
        radius_sq: f32,
        scanned: &mut [bool],
        matches: &mut Vec<usize>,
    ) {
        const MAX_SCRIPT_MATCHES: usize = 16;
        if matches.len() >= MAX_SCRIPT_MATCHES {
            return;
        }
        for &entity_index in list {
            if matches.len() >= MAX_SCRIPT_MATCHES {
                return;
            }
            let Some(was_scanned) = scanned.get_mut(entity_index) else {
                continue;
            };
            if *was_scanned {
                continue;
            }
            *was_scanned = true;
            let Some(entity) = self.entities.get(entity_index) else {
                continue;
            };
            if entity.model_index != model_index {
                continue;
            }
            let dx = entity.position[0] - position[0];
            let dy = entity.position[1] - position[1];
            if dx * dx + dy * dy < radius_sq {
                matches.push(entity_index);
            }
        }
    }

    /// re3 `CBuilding::ReplaceWithNewModel` — swap IPL entity model for SCM map updates.
    pub fn replace_entity_model(&mut self, entity_index: usize, new_model_index: usize) -> bool {
        self.replace_entity_model_with_request(entity_index, new_model_index, None)
    }

    pub(crate) fn replace_entity_model_with_request(
        &mut self,
        entity_index: usize,
        new_model_index: usize,
        request_big_building: Option<bool>,
    ) -> bool {
        let Some(entity) = self.entities.get_mut(entity_index) else {
            return false;
        };
        self.pending_model_replacements.push(WorldModelReplacement {
            entity_index,
            old_model_index: entity.model_index,
            new_model_index,
            request_big_building,
        });
        entity.model_index = new_model_index;
        self.model_swap_epoch = self.model_swap_epoch.wrapping_add(1);
        true
    }

    pub(crate) fn has_pending_model_replacements(&self) -> bool {
        !self.pending_model_replacements.is_empty()
    }

    pub(crate) fn take_model_replacements(&mut self) -> Vec<WorldModelReplacement> {
        std::mem::take(&mut self.pending_model_replacements)
    }

    /// re3 entity `bIsVisible` for SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE.
    pub fn set_entity_visible(&mut self, entity_index: usize, visible: bool) -> bool {
        let Some(entity) = self.entities.get_mut(entity_index) else {
            return false;
        };
        entity.is_visible = visible;
        true
    }

    /// re3 `CWorld::FindGroundZForCoord` — vertical line buildings/dummies.
    /// Returns hit Z or re3 default `20.0` when no collision is found.
    pub fn find_ground_z_for_coord(&self, x: f32, y: f32, model_info: &CModelInfo) -> f32 {
        match self.process_vertical_line_static([x, y, 1000.0], -1000.0, model_info) {
            Some(point) => point.point.z,
            None => 20.0,
        }
    }

    /// re3 `CWorld::FindGroundZFor3DCoord` — cast down from (x,y,z) to z=-1000.
    /// The 3D helper's native flag combination tests buildings only; unlike
    /// `FindGroundZForCoord`, treadable/dummy entities are not candidates.
    pub fn find_ground_z_for_3d_coord(
        &self,
        x: f32,
        y: f32,
        z: f32,
        model_info: &CModelInfo,
    ) -> Option<f32> {
        self.process_vertical_line_buildings_only_with_entity([x, y, z], -1000.0, model_info)
            .map(|(hit, _)| hit.point.z)
    }

    /// re3 `CWorld::FindRoofZFor3DCoord` static building/dummy branch.
    pub fn find_roof_z_for_3d_coord(
        &self,
        x: f32,
        y: f32,
        z: f32,
        model_info: &CModelInfo,
    ) -> Option<f32> {
        self.process_vertical_line_static([x, y, z], 1000.0, model_info)
            .map(|hit| hit.point.z)
    }

    /// Static building/dummy branch of re3 `CWorld::ProcessVerticalLine`.
    /// The endpoint may be below (ground) or above (roof) the start.
    pub fn process_vertical_line_static(
        &self,
        point1: [f32; 3],
        lower_z: f32,
        model_info: &CModelInfo,
    ) -> Option<crate::collision::ColModel::CColPoint> {
        self.process_vertical_line_static_with_entity(point1, lower_z, model_info)
            .map(|(point, _)| point)
    }

    /// Static building/dummy branch of re3 `CWorld::ProcessVerticalLine`,
    /// retaining the hit entity needed by callers such as
    /// `CPacManPickups::GeneratePMPickUps`.
    pub fn process_vertical_line_static_with_entity(
        &self,
        point1: [f32; 3],
        lower_z: f32,
        model_info: &CModelInfo,
    ) -> Option<(crate::collision::ColModel::CColPoint, usize)> {
        let point2 = [point1[0], point1[1], lower_z];
        let mut mindist = 1.0_f32;
        let mut hit_local: Option<[f32; 3]> = None;
        let mut hit_entity: Option<usize> = None;

        let sx = Self::get_sector_index_x(point1[0]);
        let sy = Self::get_sector_index_y(point1[1]);
        if sx < NUMSECTORS_X && sy < NUMSECTORS_Y {
            let sector = &self.sectors[sy * NUMSECTORS_X + sx];
            let mut visited_buf = [0usize; 64];
            let mut visited_len = 0;
            let mut visited_overflow: Vec<usize> = Vec::new();
            for list in [
                sector.buildings.as_slice(),
                sector.buildings_overlap.as_slice(),
                sector.dummies.as_slice(),
                sector.dummies_overlap.as_slice(),
            ] {
                self.scan_vertical_line_list(
                    list,
                    point1,
                    point2,
                    model_info,
                    &mut mindist,
                    &mut hit_local,
                    &mut hit_entity,
                    &mut visited_buf,
                    &mut visited_len,
                    &mut visited_overflow,
                );
            }
        }

        let (local, entity_index) = (hit_local?, hit_entity?);
        let entity = self.entities.get(entity_index)?;
        let world = transform_entity_point(entity, local);
        let mut point = crate::collision::ColModel::CColPoint::new();
        point.point = CColVector::new(world[0], world[1], world[2]);
        Some((point, entity_index))
    }

    /// re3 `CWorld::ProcessVerticalLine` with `bTestBuildings=true` and every
    /// other list disabled — the flag combination `DragCarToPoint` uses
    /// (`control/CarCtrl.cpp:1997-1999`, `2004-2006`, `2018-2020`,
    /// `2025-2027`).  Dummy (treadable) entities are deliberately excluded so
    /// rails cars snap to the road surface, not to ped-path map objects.
    pub fn process_vertical_line_buildings_only_with_entity(
        &self,
        point1: [f32; 3],
        lower_z: f32,
        model_info: &CModelInfo,
    ) -> Option<(crate::collision::ColModel::CColPoint, usize)> {
        let point2 = [point1[0], point1[1], lower_z];
        let mut mindist = 1.0_f32;
        let mut hit_local: Option<[f32; 3]> = None;
        let mut hit_entity: Option<usize> = None;

        let sx = Self::get_sector_index_x(point1[0]);
        let sy = Self::get_sector_index_y(point1[1]);
        if sx < NUMSECTORS_X && sy < NUMSECTORS_Y {
            let sector = &self.sectors[sy * NUMSECTORS_X + sx];
            let mut visited_buf = [0usize; 64];
            let mut visited_len = 0;
            let mut visited_overflow: Vec<usize> = Vec::new();
            for list in [
                sector.buildings.as_slice(),
                sector.buildings_overlap.as_slice(),
            ] {
                self.scan_vertical_line_list(
                    list,
                    point1,
                    point2,
                    model_info,
                    &mut mindist,
                    &mut hit_local,
                    &mut hit_entity,
                    &mut visited_buf,
                    &mut visited_len,
                    &mut visited_overflow,
                );
            }
        }

        let (local, entity_index) = (hit_local?, hit_entity?);
        let entity = self.entities.get(entity_index)?;
        let world = transform_entity_point(entity, local);
        let mut point = crate::collision::ColModel::CColPoint::new();
        point.point = CColVector::new(world[0], world[1], world[2]);
        Some((point, entity_index))
    }

    fn scan_vertical_line_list(
        &self,
        list: &[usize],
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        mindist: &mut f32,
        hit_local: &mut Option<[f32; 3]>,
        hit_entity: &mut Option<usize>,
        visited_buf: &mut [usize; 64],
        visited_len: &mut usize,
        visited_overflow: &mut Vec<usize>,
    ) {
        for &entity_index in list {
            let is_visited = visited_buf[..*visited_len].contains(&entity_index)
                || (!visited_overflow.is_empty() && visited_overflow.contains(&entity_index));
            if is_visited {
                continue;
            }
            if *visited_len < visited_buf.len() {
                visited_buf[*visited_len] = entity_index;
                *visited_len += 1;
            } else {
                visited_overflow.push(entity_index);
            }
            let Some(entity) = self.entities.get(entity_index) else {
                continue;
            };
            if !entity.uses_collision {
                continue;
            }
            let Some(col_model) = model_info.col_model(entity.model_index) else {
                continue;
            };
            let local_line = entity_local_line(entity, point1, point2);
            let mut col_point = crate::collision::ColModel::CColPoint::new();
            if col_model.process_line_of_sight_primitives(&local_line, &mut col_point, mindist) {
                *hit_local = Some([col_point.point.x, col_point.point.y, col_point.point.z]);
                *hit_entity = Some(entity_index);
                continue;
            }
            // re3 has no bounding-box fallback here: a collision model with
            // no spheres/boxes/triangles (e.g. stripped by
            // RemoveColModelsFromOtherLevels, whose shell keeps only the
            // bounding volumes) reports no hit. Inventing one from the
            // leftover bounding box created invisible walls and ground and
            // floated rubbish/debris over empty COL shells.
        }
    }

    /// CPed's three 0.35-radius collision spheres against nearby static COL
    /// primitives. Mirrors the narrow phase of CCollision::ProcessColModels.
    pub fn sphere_contact_static_world(
        &self,
        center: [f32; 3],
        radius: f32,
        model_info: &CModelInfo,
    ) -> Option<StaticSphereContact> {
        self.sphere_contact_static_world_with_dummies(center, radius, model_info, true)
    }

    /// Building-only form of `CWorld::TestSphereAgainstWorld`. Several ped
    /// probes pass `checkDummies=false`; sharing the primitive narrow phase but
    /// preserving that list selection avoids treating treadable/dummy map
    /// entities as invisible walls for those probes.
    pub fn sphere_contact_buildings_only(
        &self,
        center: [f32; 3],
        radius: f32,
        model_info: &CModelInfo,
    ) -> Option<StaticSphereContact> {
        self.sphere_contact_static_world_with_dummies(center, radius, model_info, false)
    }

    fn sphere_contact_static_world_with_dummies(
        &self,
        center: [f32; 3],
        radius: f32,
        model_info: &CModelInfo,
        check_dummies: bool,
    ) -> Option<StaticSphereContact> {
        let min_x = Self::get_sector_index_x(center[0] - radius);
        let max_x = Self::get_sector_index_x(center[0] + radius);
        let min_y = Self::get_sector_index_y(center[1] - radius);
        let max_y = Self::get_sector_index_y(center[1] + radius);
        let mut best: Option<StaticSphereContact> = None;
        for y in min_y..=max_y.min(NUMSECTORS_Y - 1) {
            for x in min_x..=max_x.min(NUMSECTORS_X - 1) {
                let sector = &self.sectors[y * NUMSECTORS_X + x];
                for (list, is_dummy) in [
                    (sector.buildings.as_slice(), false),
                    (sector.buildings_overlap.as_slice(), false),
                    (sector.objects.as_slice(), false),
                    (sector.objects_overlap.as_slice(), false),
                    (sector.dummies.as_slice(), true),
                    (sector.dummies_overlap.as_slice(), true),
                ] {
                    if is_dummy && !check_dummies {
                        continue;
                    }
                    for &entity_index in list {
                        let Some(entity) = self.entities.get(entity_index) else {
                            continue;
                        };
                        if !entity.uses_collision {
                            continue;
                        }
                        let Some(col) = model_info.col_model(entity.model_index) else {
                            continue;
                        };
                        let local = inverse_transform_entity_point(entity, center);
                        let Some(local_contact) = sphere_contact_col_model(local, radius, col)
                        else {
                            continue;
                        };
                        let normal = transform_entity_vector(entity, local_contact.normal);
                        let contact = StaticSphereContact {
                            normal: normalize_vec3(normal),
                            penetration: local_contact.penetration,
                        };
                        if best.is_none_or(|old| contact.penetration > old.penetration) {
                            best = Some(contact);
                        }
                    }
                }
            }
        }
        best
    }

    /// Sphere half of `CCollision::ProcessColModels` for one authored model
    /// against loaded building/object/dummy sectors. Vehicles and peds both
    /// use this path in re3; piece/surface ids from the moving model are retained.
    pub fn model_contacts_static_world(
        &self,
        position: [f32; 3],
        basis: [[f32; 3]; 3],
        model_index: usize,
        model_info: &CModelInfo,
    ) -> Vec<StaticModelContact> {
        let Some(moving_col) = model_info.col_model(model_index) else {
            return Vec::new();
        };
        self.col_model_contacts_static_world(position, basis, moving_col, model_info)
    }

    /// Same native ProcessColModels query as model_contacts_static_world,
    /// but accepts a temporary COL directly. re3 uses this for peds that are on
    /// the ground (ms_colModelPedGroundHit), which deliberately has no model ID.
    pub fn col_model_contacts_static_world(
        &self,
        position: [f32; 3],
        basis: [[f32; 3]; 3],
        moving_col: &CColModel,
        model_info: &CModelInfo,
    ) -> Vec<StaticModelContact> {
        let radius = moving_col.bounding_sphere.radius.max(1.0);
        let min_x = Self::get_sector_index_x(position[0] - radius);
        let max_x = Self::get_sector_index_x(position[0] + radius);
        let min_y = Self::get_sector_index_y(position[1] - radius);
        let max_y = Self::get_sector_index_y(position[1] + radius);
        let mut contacts = Vec::with_capacity(MAX_COLLISION_POINTS);
        let mut triangle_candidates = Vec::new();
        // Hot-path dedup: this query runs up to ~6 attempts x N substeps per
        // vehicle/ped per frame. A stack buffer handles typical sector queries
        // with zero dynamic heap allocations.
        let mut visited_buf = [0usize; 128];
        let mut visited_len = 0;
        let mut visited_overflow: Vec<usize> = Vec::new();

        let to_world = |local: CColVector| {
            [
                position[0] + basis[0][0] * local.x + basis[1][0] * local.y + basis[2][0] * local.z,
                position[1] + basis[0][1] * local.x + basis[1][1] * local.y + basis[2][1] * local.z,
                position[2] + basis[0][2] * local.x + basis[1][2] * local.y + basis[2][2] * local.z,
            ]
        };
        let to_local = |world: [f32; 3], entity: &CWorldEntity| {
            let local = inverse_transform_entity_point(entity, world);
            CColVector::new(local[0], local[1], local[2])
        };
        let normal_to_world = |normal: CColVector, entity: &CWorldEntity| {
            let transformed = transform_entity_vector(entity, [normal.x, normal.y, normal.z]);
            normalize_vec3(transformed)
        };

        let moving_bound_center = to_world(moving_col.bounding_sphere.center);
        let moving_bound_radius = moving_col.bounding_sphere.radius;

        for y in min_y..=max_y.min(NUMSECTORS_Y - 1) {
            for x in min_x..=max_x.min(NUMSECTORS_X - 1) {
                let sector = &self.sectors[y * NUMSECTORS_X + x];
                for list in [
                    sector.buildings.as_slice(),
                    sector.buildings_overlap.as_slice(),
                    sector.objects.as_slice(),
                    sector.objects_overlap.as_slice(),
                    sector.dummies.as_slice(),
                    sector.dummies_overlap.as_slice(),
                ] {
                    for &entity_index in list {
                        let is_visited = visited_buf[..visited_len].contains(&entity_index)
                            || (!visited_overflow.is_empty()
                                && visited_overflow.contains(&entity_index));
                        if is_visited {
                            continue;
                        }
                        if visited_len < visited_buf.len() {
                            visited_buf[visited_len] = entity_index;
                            visited_len += 1;
                        } else {
                            visited_overflow.push(entity_index);
                        }

                        let Some(entity) = self.entities.get(entity_index) else {
                            continue;
                        };
                        if !entity.uses_collision {
                            continue;
                        }
                        let Some(static_col) = model_info.col_model(entity.model_index) else {
                            continue;
                        };

                        // re3 CEntity::GetIsTouching / CPhysical::ProcessCollisionSectorList:
                        // Skip candidate if the moving and static bounding spheres do not overlap.
                        if moving_bound_radius > 0.0 && static_col.bounding_sphere.radius > 0.0 {
                            let static_bound_center = transform_entity_point(
                                entity,
                                [
                                    static_col.bounding_sphere.center.x,
                                    static_col.bounding_sphere.center.y,
                                    static_col.bounding_sphere.center.z,
                                ],
                            );
                            let max_touch_dist =
                                moving_bound_radius + static_col.bounding_sphere.radius;
                            let dx = static_bound_center[0] - moving_bound_center[0];
                            let dy = static_bound_center[1] - moving_bound_center[1];
                            let dz = static_bound_center[2] - moving_bound_center[2];
                            if dx * dx + dy * dy + dz * dz > max_touch_dist * max_touch_dist {
                                continue;
                            }
                        }

                        // re3 reuses a MAX_COLLISION_POINTS scratch array for
                        // each candidate entity in ProcessCollisionSectorList.
                        // The cap is therefore per entity, not for the whole
                        // sector query. A global cap can let one detailed
                        // floor/prop consume all 32 slots and make a later
                        // wall entity completely invisible to collision.
                        let mut entity_contact_count = 0usize;

                        let local_moving_sphere = |sphere: &CColSphere| {
                            let world_center = to_world(sphere.center);
                            to_local(world_center, entity)
                        };
                        for sphere in &moving_col.spheres {
                            if entity_contact_count == MAX_COLLISION_POINTS {
                                break;
                            }
                            let center = local_moving_sphere(sphere);

                            // Bounding box early-out: if moving sphere doesn't touch static bounding box, skip primitives.
                            let box_bounds = &static_col.bounding_box;
                            if box_bounds.min.x <= box_bounds.max.x
                                && (center.x + sphere.radius < box_bounds.min.x
                                    || center.x - sphere.radius > box_bounds.max.x
                                    || center.y + sphere.radius < box_bounds.min.y
                                    || center.y - sphere.radius > box_bounds.max.y
                                    || center.z + sphere.radius < box_bounds.min.z
                                    || center.z - sphere.radius > box_bounds.max.z)
                            {
                                continue;
                            }

                            let mut best_for_sphere: Option<StaticModelContact> = None;
                            let mut register = |point: CColPoint| {
                                if point.depth <= 0.0
                                    || best_for_sphere.is_some_and(|old| old.depth >= point.depth)
                                {
                                    return;
                                }
                                let world_point = transform_entity_point(
                                    entity,
                                    [point.point.x, point.point.y, point.point.z],
                                );
                                best_for_sphere = Some(StaticModelContact {
                                    point: world_point,
                                    normal: normal_to_world(point.normal, entity),
                                    piece_a: point.piece_a,
                                    piece_b: point.piece_b,
                                    surface_a: point.surface_a,
                                    surface_b: point.surface_b,
                                    depth: point.depth,
                                    entity_index,
                                });
                            };

                            for other in &static_col.spheres {
                                let mut point = CColPoint::new();
                                let mut distance_sq = f32::MAX;
                                if process_sphere_sphere(
                                    &CColSphere::new(
                                        sphere.radius,
                                        center,
                                        sphere.surface,
                                        sphere.piece,
                                    ),
                                    other,
                                    &mut point,
                                    &mut distance_sq,
                                ) {
                                    register(point);
                                }
                            }
                            for other in &static_col.boxes {
                                let mut point = CColPoint::new();
                                let mut distance_sq = f32::MAX;
                                if process_sphere_box(
                                    &CColSphere::new(
                                        sphere.radius,
                                        center,
                                        sphere.surface,
                                        sphere.piece,
                                    ),
                                    other,
                                    &mut point,
                                    &mut distance_sq,
                                ) {
                                    register(point);
                                }
                            }
                            static_col.triangle_spatial_index.sphere_candidates(
                                static_col.triangles.len(), center, sphere.radius, &mut triangle_candidates);
                            for &triangle_index in &triangle_candidates {
                                let triangle = &static_col.triangles[triangle_index];
                                if !static_col.triangle_maybe_overlaps_sphere(
                                    triangle_index,
                                    center,
                                    sphere.radius,
                                ) {
                                    continue;
                                }
                                let Some(plane) = static_col.triangle_plane(triangle_index) else {
                                    continue;
                                };
                                let mut point = CColPoint::new();
                                let mut distance_sq = f32::MAX;
                                if process_sphere_triangle(
                                    &CColSphere::new(
                                        sphere.radius,
                                        center,
                                        sphere.surface,
                                        sphere.piece,
                                    ),
                                    &static_col.vertices,
                                    triangle,
                                    &plane,
                                    &mut point,
                                    &mut distance_sq,
                                ) {
                                    register(point);
                                }
                            }
                            if let Some(contact) = best_for_sphere {
                                contacts.push(contact);
                                entity_contact_count += 1;
                            }
                        }
                    }
                }
            }
        }
        contacts
    }

    /// Vehicle-named compatibility wrapper. The generic implementation above
    /// is also used by the player ped so both consume the authored model COL.
    pub fn vehicle_contacts_static_world(
        &self,
        position: [f32; 3],
        basis: [[f32; 3]; 3],
        model_index: usize,
        model_info: &CModelInfo,
    ) -> Vec<StaticModelContact> {
        self.model_contacts_static_world(position, basis, model_index, model_info)
    }

    pub fn test_sphere_against_static_world(
        &self,
        center: [f32; 3],
        radius: f32,
        model_info: &CModelInfo,
    ) -> bool {
        self.sphere_contact_static_world(center, radius, model_info)
            .is_some()
    }

    /// Building+object+dummy LOS returning the actual closest collision point. This is
    /// the `CColPoint::point` surface needed by camera source clipping; unlike
    /// the legacy entity-only helper it never invents a midpoint for bounds-only
    /// models.
    pub fn process_line_of_sight_static_hit(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        ignore_entity_index: Option<usize>,
        check_dummies: bool,
    ) -> Option<([f32; 3], usize)> {
        self.process_line_of_sight_static_contact(
            point1,
            point2,
            model_info,
            ignore_entity_index,
            check_dummies,
        )
        .map(|contact| (contact.point, contact.entity_index))
    }

    pub fn process_line_of_sight_static_contact(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        ignore_entity_index: Option<usize>,
        check_dummies: bool,
    ) -> Option<StaticLineContact> {
        self.process_line_of_sight_static_contact_filtered(
            point1,
            point2,
            model_info,
            ignore_entity_index,
            true,
            check_dummies,
        )
    }

    /// CWeapon::HitsGround requests building COL only, excluding objects/dummies.
    pub fn process_line_of_sight_building_contact(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
    ) -> Option<StaticLineContact> {
        self.process_line_of_sight_static_contact_filtered(
            point1, point2, model_info, None, false, false,
        )
    }

    fn process_line_of_sight_static_contact_filtered(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        ignore_entity_index: Option<usize>,
        check_objects: bool,
        check_dummies: bool,
    ) -> Option<StaticLineContact> {
        let mut closest = None;
        let mut min_dist = 1.0_f32;
        let x0 = Self::get_sector_index_x(point1[0]).min(Self::get_sector_index_x(point2[0]));
        let x1 = Self::get_sector_index_x(point1[0]).max(Self::get_sector_index_x(point2[0]));
        let y0 = Self::get_sector_index_y(point1[1]).min(Self::get_sector_index_y(point2[1]));
        let y1 = Self::get_sector_index_y(point1[1]).max(Self::get_sector_index_y(point2[1]));
        let mut visited_buf = [0usize; 64];
        let mut visited_len = 0;
        let mut visited_overflow: Vec<usize> = Vec::new();

        for y in y0..=y1.min(NUMSECTORS_Y - 1) {
            for x in x0..=x1.min(NUMSECTORS_X - 1) {
                let sector = &self.sectors[y * NUMSECTORS_X + x];
                for (list, enabled) in [
                    (sector.buildings.as_slice(), true),
                    (sector.buildings_overlap.as_slice(), true),
                    (sector.objects.as_slice(), check_objects),
                    (sector.objects_overlap.as_slice(), check_objects),
                    (sector.dummies.as_slice(), check_dummies),
                    (sector.dummies_overlap.as_slice(), check_dummies),
                ] {
                    if !enabled {
                        continue;
                    }
                    for &entity_index in list {
                        if Some(entity_index) == ignore_entity_index {
                            continue;
                        }
                        let is_visited = visited_buf[..visited_len].contains(&entity_index)
                            || (!visited_overflow.is_empty()
                                && visited_overflow.contains(&entity_index));
                        if is_visited {
                            continue;
                        }
                        if visited_len < visited_buf.len() {
                            visited_buf[visited_len] = entity_index;
                            visited_len += 1;
                        } else {
                            visited_overflow.push(entity_index);
                        }
                        let Some(entity) = self.entities.get(entity_index) else {
                            continue;
                        };
                        if !entity.uses_collision {
                            continue;
                        }
                        let Some(col_model) = model_info.col_model(entity.model_index) else {
                            continue;
                        };
                        let local_line = entity_local_line(entity, point1, point2);
                        let mut col_point = crate::collision::ColModel::CColPoint::new();
                        if col_model.process_line_of_sight_primitives(
                            &local_line,
                            &mut col_point,
                            &mut min_dist,
                        ) {
                            let normal = transform_entity_vector(
                                entity,
                                [col_point.normal.x, col_point.normal.y, col_point.normal.z],
                            );
                            closest = Some(StaticLineContact {
                                point: transform_entity_point(
                                    entity,
                                    [col_point.point.x, col_point.point.y, col_point.point.z],
                                ),
                                normal: normalize_vec3(normal),
                                surface: col_point.surface_b,
                                entity_index,
                            });
                        }
                    }
                }
            }
        }
        closest
    }

    pub fn process_line_of_sight_building_bounds(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        ignore_entity_index: Option<usize>,
    ) -> Option<usize> {
        self.process_line_of_sight_building_lists(
            point1,
            point2,
            model_info,
            ignore_entity_index,
            true,
        )
    }

    /// re3 `CWorld::GetIsLineOfSightClear` with only `checkBuildings` enabled.
    /// `CPathFind::FindNextNodeWandering` uses exactly this filter: objects and
    /// dummies beside the pavement must not invalidate a pedestrian path link.
    pub fn process_line_of_sight_buildings_hit(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        ignore_entity_index: Option<usize>,
    ) -> Option<usize> {
        self.process_line_of_sight_building_lists(
            point1,
            point2,
            model_info,
            ignore_entity_index,
            false,
        )
    }

    fn process_line_of_sight_building_lists(
        &self,
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        ignore_entity_index: Option<usize>,
        check_dummies: bool,
    ) -> Option<usize> {
        let mut hit_entity = None;
        let mut mindist = 1.0;
        // re3 walks only the sectors crossed by the LOS. Scanning every world
        // sector here made the third-person camera and player collision consume
        // most of a frame in free roam.
        let x0 = Self::get_sector_index_x(point1[0]).min(Self::get_sector_index_x(point2[0]));
        let x1 = Self::get_sector_index_x(point1[0]).max(Self::get_sector_index_x(point2[0]));
        let y0 = Self::get_sector_index_y(point1[1]).min(Self::get_sector_index_y(point2[1]));
        let y1 = Self::get_sector_index_y(point1[1]).max(Self::get_sector_index_y(point2[1]));
        for y in y0.saturating_sub(1)..=(y1 + 1).min(NUMSECTORS_Y - 1) {
            for x in x0.saturating_sub(1)..=(x1 + 1).min(NUMSECTORS_X - 1) {
                let sector = &self.sectors[y * NUMSECTORS_X + x];
                self.process_line_of_sight_sector_list_bounds(
                    &sector.buildings,
                    point1,
                    point2,
                    model_info,
                    ignore_entity_index,
                    &mut hit_entity,
                    &mut mindist,
                );
                self.process_line_of_sight_sector_list_bounds(
                    &sector.buildings_overlap,
                    point1,
                    point2,
                    model_info,
                    ignore_entity_index,
                    &mut hit_entity,
                    &mut mindist,
                );
                if check_dummies {
                    self.process_line_of_sight_sector_list_bounds(
                        &sector.dummies,
                        point1,
                        point2,
                        model_info,
                        ignore_entity_index,
                        &mut hit_entity,
                        &mut mindist,
                    );
                    self.process_line_of_sight_sector_list_bounds(
                        &sector.dummies_overlap,
                        point1,
                        point2,
                        model_info,
                        ignore_entity_index,
                        &mut hit_entity,
                        &mut mindist,
                    );
                }
            }
        }
        hit_entity
    }

    pub fn do_thorough_line_test_buildings(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        test_entity_index: usize,
        model_info: &CModelInfo,
    ) -> bool {
        if self
            .process_line_of_sight_building_bounds(start, end, model_info, None)
            .is_some_and(|entity_index| entity_index != test_entity_index)
        {
            return false;
        }

        let line = sub_vec3(end, start);
        let side_seed = if start[0] != end[0] || start[1] != end[1] {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let up = normalize_vec3(cross_vec3(side_seed, line));
        let side = normalize_vec3(cross_vec3(up, line));
        let side = mul_vec3(side, 0.1);
        let up = mul_vec3(up, 0.1);

        for offset in [side, mul_vec3(side, -1.0), up, mul_vec3(up, -1.0)] {
            if self
                .process_line_of_sight_building_bounds(
                    add_vec3(start, offset),
                    add_vec3(end, offset),
                    model_info,
                    None,
                )
                .is_some_and(|entity_index| entity_index != test_entity_index)
            {
                return false;
            }
        }

        true
    }

    fn process_line_of_sight_sector_list_bounds(
        &self,
        list: &[usize],
        point1: [f32; 3],
        point2: [f32; 3],
        model_info: &CModelInfo,
        ignore_entity_index: Option<usize>,
        hit_entity: &mut Option<usize>,
        mindist: &mut f32,
    ) {
        for entity_index in list {
            if Some(*entity_index) == ignore_entity_index {
                continue;
            }
            let Some(entity) = self.entities.get(*entity_index) else {
                continue;
            };
            if !entity.uses_collision {
                continue;
            }
            let Some(col_model) = model_info.col_model(entity.model_index) else {
                continue;
            };
            let local_line = entity_local_line(entity, point1, point2);
            let mut col_point = crate::collision::ColModel::CColPoint::new();
            let has_primitives = !col_model.spheres.is_empty()
                || !col_model.boxes.is_empty()
                || !col_model.triangles.is_empty();
            if col_model.process_line_of_sight_primitives(&local_line, &mut col_point, mindist)
                || (!has_primitives
                    && hit_entity.is_none()
                    && test_line_box(&local_line, &col_model.bounding_box))
            {
                *hit_entity = Some(*entity_index);
            }
        }
    }

    fn do_stuff_leaving_zone(&mut self, zone_index: usize) {
        let Some(zone) = self.cull_zones.resolved_zones.get(zone_index).cloned() else {
            return;
        };
        let Some(index_start) = usize_from_i32(zone.index_start) else {
            return;
        };
        let num_buildings = usize_from_i16(zone.num_buildings);
        let num_treadables_plus_10m = usize_from_i16(zone.num_treadables_plus_10m);
        let num_treadables = usize_from_i16(zone.num_treadables);

        for offset in 0..num_buildings {
            if let Some(pool_index) = self.cull_index_at(index_start + offset) {
                self.do_stuff_leaving_zone_one_building(pool_index);
            }
        }
        for offset in num_buildings..num_buildings + num_treadables_plus_10m + num_treadables {
            if let Some(pool_index) = self.cull_index_at(index_start + offset) {
                self.do_stuff_leaving_zone_one_treadable_both(pool_index);
            }
        }
    }

    fn do_stuff_entering_zone(&mut self, zone_index: usize) {
        let Some(zone) = self.cull_zones.resolved_zones.get(zone_index).cloned() else {
            return;
        };
        let Some(index_start) = usize_from_i32(zone.index_start) else {
            return;
        };
        let num_buildings = usize_from_i16(zone.num_buildings);
        let num_treadables_plus_10m = usize_from_i16(zone.num_treadables_plus_10m);
        let num_treadables = usize_from_i16(zone.num_treadables);

        for offset in 0..num_buildings {
            if let Some(pool_index) = self.cull_index_at(index_start + offset) {
                self.do_stuff_entering_zone_one_building(pool_index);
            }
        }
        for offset in num_buildings..num_buildings + num_treadables_plus_10m {
            if let Some(pool_index) = self.cull_index_at(index_start + offset) {
                self.do_stuff_entering_zone_one_treadable_plus_10m(pool_index);
            }
        }
        for offset in num_buildings + num_treadables_plus_10m
            ..num_buildings + num_treadables_plus_10m + num_treadables
        {
            if let Some(pool_index) = self.cull_index_at(index_start + offset) {
                self.do_stuff_entering_zone_one_treadable(pool_index);
            }
        }
    }

    fn do_stuff_leaving_zone_one_building(&mut self, pool_index: u16) {
        self.apply_building_cull_index(pool_index, false);
    }

    fn do_stuff_leaving_zone_one_treadable_both(&mut self, pool_index: u16) {
        self.apply_treadable_cull_index(pool_index, false, Some(false));
    }

    fn do_stuff_entering_zone_one_building(&mut self, pool_index: u16) {
        self.apply_building_cull_index(pool_index, true);
    }

    fn do_stuff_entering_zone_one_treadable_plus_10m(&mut self, pool_index: u16) {
        self.apply_treadable_cull_index(pool_index, true, Some(true));
    }

    fn do_stuff_entering_zone_one_treadable(&mut self, pool_index: u16) {
        self.apply_treadable_cull_index(pool_index, true, None);
    }

    fn apply_building_cull_index(&mut self, pool_index: u16, zone_culled: bool) {
        if pool_index < NUMUNCOMPRESSED_CULL_INDICES {
            let pool_index = pool_index as usize;
            self.set_building_pool_zone_culled(pool_index, zone_culled);
            if let Some(big_building_pool_index) =
                self.cull_zones.building_to_big_building.get(pool_index)
            {
                if *big_building_pool_index != -1 {
                    self.set_building_pool_zone_culled(
                        *big_building_pool_index as usize,
                        zone_culled,
                    );
                }
            }
            return;
        }

        let compressed_start = (pool_index - NUMUNCOMPRESSED_CULL_INDICES) as usize;
        for offset in 0..3 {
            if let Some(next_pool_index) = self.cull_index_at(compressed_start + offset) {
                self.apply_building_cull_index(next_pool_index, zone_culled);
            }
        }
    }

    fn apply_treadable_cull_index(
        &mut self,
        pool_index: u16,
        zone_culled: bool,
        zone_culled2: Option<bool>,
    ) {
        if pool_index < NUMUNCOMPRESSED_CULL_INDICES {
            let pool_index = pool_index as usize;
            self.set_treadable_pool_zone_culled(pool_index, zone_culled, zone_culled2);
            if let Some(big_building_pool_index) =
                self.cull_zones.treadable_to_big_building.get(pool_index)
            {
                if *big_building_pool_index != -1 {
                    self.set_building_pool_zone_culled(
                        *big_building_pool_index as usize,
                        zone_culled,
                    );
                }
            }
            return;
        }

        let compressed_start = (pool_index - NUMUNCOMPRESSED_CULL_INDICES) as usize;
        for offset in 0..3 {
            if let Some(next_pool_index) = self.cull_index_at(compressed_start + offset) {
                self.apply_treadable_cull_index(next_pool_index, zone_culled, zone_culled2);
            }
        }
    }

    fn cull_index_at(&self, index: usize) -> Option<u16> {
        self.cull_zones.indices.get(index).copied()
    }

    fn set_building_pool_zone_culled(&mut self, pool_index: usize, zone_culled: bool) {
        if let Some(entity_index) = self.building_pool.get(pool_index).copied() {
            if let Some(entity) = self.entities.get_mut(entity_index) {
                entity.zone_culled = zone_culled;
            }
        }
    }

    fn set_treadable_pool_zone_culled(
        &mut self,
        pool_index: usize,
        zone_culled: bool,
        zone_culled2: Option<bool>,
    ) {
        if let Some(entity_index) = self.treadable_pool.get(pool_index).copied() {
            if let Some(entity) = self.entities.get_mut(entity_index) {
                entity.zone_culled = zone_culled;
                if let Some(zone_culled2) = zone_culled2 {
                    entity.zone_culled2 = zone_culled2;
                }
            }
        }
    }
}

impl CTheZones {
    pub fn init() -> Self {
        Self {
            zones: vec![CZone {
                name: "CITYZON".to_owned(),
                zone_type: 0,
                min: [-4000.0, -4000.0, -500.0],
                max: [4000.0, 4000.0, 500.0],
                level: 0,
                ped_info_day: ZonePedInfo::default(),
                ped_info_night: ZonePedInfo::default(),
            }],
            map_zones: vec![CZone {
                name: "THEMAP".to_owned(),
                zone_type: 3,
                min: [-4000.0, -4000.0, -500.0],
                max: [4000.0, 4000.0, 500.0],
                level: 0,
                ped_info_day: ZonePedInfo::default(),
                ped_info_night: ZonePedInfo::default(),
            }],
            curr_level: 0,
        }
    }

    pub fn create_zone(&mut self, zone: &GtaZone) {
        self.zones.push(CZone::from_gta_zone(zone));
    }

    pub fn create_map_zone(&mut self, zone: &GtaZone) {
        self.map_zones.push(CZone::from_gta_zone(zone));
    }

    /// re3 `CTheZones::Update`: refresh the current map/island level from the
    /// live player's coordinates before the SCM VM and collision update run.
    pub fn update(&mut self, player_position: [f32; 3]) {
        self.curr_level = self.get_level_from_position(player_position);
    }

    fn zone_info_mut(&mut self, label: &[u8], day: bool) -> Option<&mut ZonePedInfo> {
        let label_end = label
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(label.len());
        let label = std::str::from_utf8(&label[..label_end]).ok()?;
        let zone = self
            .zones
            .iter_mut()
            .find(|zone| zone.name.eq_ignore_ascii_case(label))?;
        Some(if day {
            &mut zone.ped_info_day
        } else {
            &mut zone.ped_info_night
        })
    }

    pub fn set_zone_ped_info(
        &mut self,
        label: &[u8],
        day: bool,
        ped_density: i32,
        gang_density: [i32; 9],
        cop_density: i32,
    ) -> bool {
        let Some(info) = self.zone_info_mut(label, day) else {
            return false;
        };
        if ped_density != -1 {
            info.ped_density = ped_density as u16;
        }
        if cop_density != -1 {
            info.cop_density = cop_density as u16;
        }
        for (current, value) in info.gang_density.iter_mut().zip(gang_density) {
            if value != -1 {
                *current = value as u16;
            }
        }
        true
    }

    pub fn set_zone_group(&mut self, label: &[u8], day: bool, ped_group: i32) -> bool {
        let Some(info) = self.zone_info_mut(label, day) else {
            return false;
        };
        info.ped_group = ped_group as u16;
        true
    }

    pub fn set_zone_car_info(
        &mut self,
        label: &[u8],
        day: bool,
        car_density: i32,
        gang_counts: [i32; 9],
        cop_count: i32,
        car_counts: [i32; 6],
    ) -> bool {
        let Some(info) = self.zone_info_mut(label, day) else {
            return false;
        };
        if car_density != -1 {
            info.car_density = car_density as i16;
        }
        let old_car_counts = successive_counts(&info.car_threshold, 0);
        let old_cop_count = info.cop_threshold - info.car_threshold[5];
        let old_gang_counts = successive_counts(&info.gang_threshold, info.cop_threshold);
        let mut total = 0_i16;
        for ((threshold, value), old) in info
            .car_threshold
            .iter_mut()
            .zip(car_counts)
            .zip(old_car_counts)
        {
            total += if value == -1 { old } else { value as i16 };
            *threshold = total;
        }
        info.cop_threshold = info.car_threshold[5]
            + if cop_count == -1 {
                old_cop_count
            } else {
                cop_count as i16
            };
        total = info.cop_threshold;
        for ((threshold, value), old) in info
            .gang_threshold
            .iter_mut()
            .zip(gang_counts)
            .zip(old_gang_counts)
        {
            total += if value == -1 { old } else { value as i16 };
            *threshold = total;
        }
        true
    }

    pub fn get_level_from_position(&self, position: [f32; 3]) -> i32 {
        self.map_zones
            .iter()
            .skip(1)
            .find(|zone| zone.contains(position))
            .unwrap_or(&self.map_zones[0])
            .level
    }

    /// Smallest (most specific) navigation zone containing `position`.
    /// re3 \`FindSmallestZonePositionType\` equivalent for the flat Bevy-side
    /// zone table. The original follows the zone tree to the deepest matching
    /// child; choosing the smallest containing volume preserves that result.
    pub fn find_zone_index_for_point_type(
        &self,
        position: [f32; 3],
        zone_type: i32,
    ) -> Option<usize> {
        let mut best = None;
        let mut best_volume = f32::INFINITY;
        for (index, zone) in self.zones.iter().enumerate() {
            if zone.zone_type != zone_type || !zone.contains(position) {
                continue;
            }
            let volume = (zone.max[0] - zone.min[0])
                * (zone.max[1] - zone.min[1])
                * (zone.max[2] - zone.min[2]).abs().max(1.0);
            if volume < best_volume {
                best_volume = volume;
                best = Some(index);
            }
        }
        best
    }

    /// Smallest (most specific) zone containing \`position\`.
    pub fn find_zone_for_point(&self, position: [f32; 3]) -> Option<&CZone> {
        let mut best: Option<&CZone> = None;
        let mut best_volume = f32::INFINITY;
        for zone in &self.zones {
            if !zone.contains(position) {
                continue;
            }
            let volume = (zone.max[0] - zone.min[0])
                * (zone.max[1] - zone.min[1])
                * (zone.max[2] - zone.min[2]).abs().max(1.0);
            if volume < best_volume {
                best_volume = volume;
                best = Some(zone);
            }
        }
        best.or_else(|| self.zones.first())
    }

    /// Zone name for HUD (`CHud::SetZoneName`).
    pub fn zone_name_for_point(&self, position: [f32; 3]) -> String {
        self.find_zone_for_point(position)
            .map(|z| z.name.clone())
            .unwrap_or_else(|| "CITYZON".to_owned())
    }
}

fn successive_counts<const N: usize>(thresholds: &[i16; N], base: i16) -> [i16; N] {
    std::array::from_fn(|index| {
        thresholds[index]
            - if index == 0 {
                base
            } else {
                thresholds[index - 1]
            }
    })
}

impl CZone {
    fn from_gta_zone(zone: &GtaZone) -> Self {
        let mut min = zone.min;
        let mut max = zone.max;
        for index in 0..3 {
            if min[index] > max[index] {
                std::mem::swap(&mut min[index], &mut max[index]);
            }
        }

        Self {
            name: uppercase_zone_name(&zone.name),
            zone_type: zone.zone_type,
            min,
            max,
            level: zone.level,
            ped_info_day: ZonePedInfo::default(),
            ped_info_night: ZonePedInfo::default(),
        }
    }

    fn contains(&self, position: [f32; 3]) -> bool {
        self.min[0] <= position[0]
            && position[0] <= self.max[0]
            && self.min[1] <= position[1]
            && position[1] <= self.max[1]
            && self.min[2] <= position[2]
            && position[2] <= self.max[2]
    }
}

impl CCullZones {
    pub fn init() -> Self {
        Self {
            cull_zones: Vec::new(),
            attribute_zones: Vec::new(),
            resolved_zones: Vec::new(),
            resolved_attribute_zones: Vec::new(),
            indices: Vec::new(),
            building_to_big_building: vec![-1; NUMBUILDINGS],
            treadable_to_big_building: vec![-1; NUMTREADABLES],
            visibility_candidate_plans: Vec::new(),
            old_cull_zone: -1,
            entity_indices_used: 0,
            loaded_visibility_cache: false,
            current_flags_camera: 0,
            current_flags_player: 0,
            current_wanted_level_drop_player: 0,
            current_subway_is_invisible: false,
        }
    }

    pub fn add_cull_zone(&mut self, zone: GtaCullZone) {
        const ATTRZONE_NOTCULLZONE: i32 = 0x20;

        if zone.flags & ATTRZONE_NOTCULLZONE == 0 {
            self.cull_zones
                .push(cull_zone_with_re3_position(zone.clone()));
        }
        if zone.flags & !ATTRZONE_NOTCULLZONE != 0 {
            self.attribute_zones.push(zone);
        }
    }

    pub fn reset_big_building_pointers(&mut self) {
        self.building_to_big_building.resize(NUMBUILDINGS, -1);
        self.building_to_big_building.fill(-1);
        self.treadable_to_big_building.resize(NUMTREADABLES, -1);
        self.treadable_to_big_building.fill(-1);
    }

    pub fn prepare_resolution_zones_from_ipl(&mut self) {
        self.resolved_zones = self
            .cull_zones
            .iter()
            .map(CResolvedCullZone::from_gta_cull_zone)
            .collect();
        self.resolved_attribute_zones = self
            .attribute_zones
            .iter()
            .map(CResolvedAttributeZone::from_gta_cull_zone)
            .collect();
        self.indices.clear();
        self.entity_indices_used = 0;
        self.loaded_visibility_cache = false;
        self.old_cull_zone = -1;
        self.current_flags_camera = 0;
        self.current_flags_player = 0;
        self.current_wanted_level_drop_player = 0;
        self.current_subway_is_invisible = false;
    }

    pub fn load_visibility_cache(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut offset = 0;
        let num_cull_zones = read_i32(bytes, &mut offset)? as usize;
        let mut resolved_zones = Vec::with_capacity(num_cull_zones.min(NUMCULLZONES));
        for index in 0..NUMCULLZONES {
            let zone = CResolvedCullZone::read(bytes, &mut offset)?;
            if index < num_cull_zones {
                resolved_zones.push(zone);
            }
        }

        let num_attribute_zones = read_i32(bytes, &mut offset)? as usize;
        let mut resolved_attribute_zones =
            Vec::with_capacity(num_attribute_zones.min(NUMATTRIBZONES));
        for index in 0..NUMATTRIBZONES {
            let zone = CResolvedAttributeZone::read(bytes, &mut offset)?;
            if index < num_attribute_zones {
                resolved_attribute_zones.push(zone);
            }
        }

        let mut indices = Vec::with_capacity(NUMZONEINDICES);
        for _ in 0..NUMZONEINDICES {
            indices.push(read_u16(bytes, &mut offset)?);
        }

        let mut building_to_big_building = Vec::with_capacity(NUMBUILDINGS);
        for _ in 0..NUMBUILDINGS {
            building_to_big_building.push(read_i16(bytes, &mut offset)?);
        }

        let mut treadable_to_big_building = Vec::with_capacity(NUMTREADABLES);
        for _ in 0..NUMTREADABLES {
            treadable_to_big_building.push(read_i16(bytes, &mut offset)?);
        }

        self.resolved_zones = resolved_zones;
        self.resolved_attribute_zones = resolved_attribute_zones;
        self.indices = indices;
        self.building_to_big_building = building_to_big_building;
        self.treadable_to_big_building = treadable_to_big_building;
        self.entity_indices_used = self.indices.len() as i32;
        self.loaded_visibility_cache = true;
        self.old_cull_zone = -1;
        self.current_flags_camera = 0;
        self.current_flags_player = 0;
        self.current_wanted_level_drop_player = 0;
        self.current_subway_is_invisible = false;
        Ok(())
    }

    pub fn force_cull_zone_coors(&mut self, coors: [f32; 3]) -> CullZoneTransition {
        let new_zone = self.find_cull_zone_for_coors(coors);
        if new_zone != self.old_cull_zone {
            let transition = CullZoneTransition {
                left_zone: (self.old_cull_zone >= 0).then_some(self.old_cull_zone as usize),
                entered_zone: (new_zone >= 0).then_some(new_zone as usize),
            };
            self.old_cull_zone = new_zone;
            transition
        } else {
            CullZoneTransition {
                left_zone: None,
                entered_zone: None,
            }
        }
    }

    pub fn find_cull_zone_for_coors(&self, coors: [f32; 3]) -> i32 {
        self.resolved_zones
            .iter()
            .position(|zone| zone.contains(coors))
            .map(|index| index as i32)
            .unwrap_or(-1)
    }

    /// re3 `CCullZones::FindAttributesForCoors`. Attribute volumes overlap;
    /// flags are ORed and the greatest wanted-level drop wins.
    pub fn find_attributes_for_coors(&self, coors: [f32; 3]) -> (i32, i32) {
        let mut attributes = 0_i32;
        let mut wanted_level = 0_i32;
        for zone in &self.resolved_attribute_zones {
            if coors[0] >= zone.min[0]
                && coors[0] <= zone.max[0]
                && coors[1] >= zone.min[1]
                && coors[1] <= zone.max[1]
                && coors[2] >= zone.min[2]
                && coors[2] <= zone.max[2]
            {
                attributes |= i32::from(zone.attributes);
                wanted_level = wanted_level.max(i32::from(zone.wanted_level));
            }
        }
        (attributes, wanted_level)
    }

    /// Runtime half of re3 `CCullZones::Update` (`ZoneCull.cpp:257-283`).
    /// Attribute flags are intentionally sampled on their authored frame
    /// slots: camera flags on `frame & 7 == 2`, player flags on `== 6`.
    pub fn update_attributes(
        &mut self,
        camera_coors: [f32; 3],
        player_coors: [f32; 3],
        frame_counter: u32,
    ) {
        if frame_counter & 7 == 2 {
            self.current_flags_camera = self.find_attributes_for_coors(camera_coors).0;
        }
        if frame_counter & 7 == 6 {
            let (flags, wanted_level_drop) = self.find_attributes_for_coors(player_coors);
            self.current_flags_player = flags;
            self.current_wanted_level_drop_player = wanted_level_drop;
        }
    }

    pub fn camera_no_rain(&self) -> bool {
        self.current_flags_camera & 8 != 0
    }

    pub fn player_no_rain(&self) -> bool {
        self.current_flags_player & 8 != 0
    }

    pub fn player_no_police(&self) -> bool {
        self.current_flags_player & 0x10 != 0
    }

    pub fn player_needs_collision(&self) -> bool {
        self.current_flags_player & 0x40 != 0
    }

    pub fn player_subway_visible(&self) -> bool {
        self.current_flags_camera & 0x80 != 0
    }

    pub fn find_zone_with_stairs_attribute(&self, coors: [f32; 3]) -> Option<usize> {
        const ATTRZONE_STAIRS: i16 = 2;
        self.resolved_attribute_zones.iter().position(|zone| {
            zone.attributes & ATTRZONE_STAIRS != 0
                && coors[0] >= zone.min[0]
                && coors[0] <= zone.max[0]
                && coors[1] >= zone.min[1]
                && coors[1] <= zone.max[1]
                && coors[2] >= zone.min[2]
                && coors[2] <= zone.max[2]
        })
    }

    pub fn compress_indices_array_from_temp(
        &mut self,
        mut temp_indices: Vec<u16>,
        general: &mut CGeneral,
    ) {
        let mut extra_indices = Vec::new();

        for num_occurrences in (2..=6).rev() {
            if self.resolved_zones.is_empty() {
                break;
            }

            let mut attempt = 0;
            while attempt < 10_000 {
                loop {
                    attempt += 1;
                    let zone = general.get_random_number() as usize % self.resolved_zones.len();
                    let group = general.get_random_number() as usize % 3;
                    let Some(set) =
                        self.pick_random_set_for_group(&temp_indices, zone, group, general)
                    else {
                        break;
                    };
                    if !self.do_we_have_more_than_x_occurrences_of_set(
                        &temp_indices,
                        num_occurrences,
                        set,
                    ) {
                        break;
                    }
                    let Ok(extra_len) = u16::try_from(extra_indices.len()) else {
                        break;
                    };
                    let Some(set_id) = NUMUNCOMPRESSED_CULL_INDICES.checked_add(extra_len) else {
                        break;
                    };

                    extra_indices.extend(set);
                    self.replace_set_for_all_groups(&mut temp_indices, set, set_id);
                    attempt = 1;
                }
            }
        }

        self.tidy_up_and_merge_lists(&temp_indices, &extra_indices);
    }

    fn pick_random_set_for_group(
        &self,
        temp_indices: &[u16],
        zone_index: usize,
        group: usize,
        general: &mut CGeneral,
    ) -> Option<[u16; 3]> {
        let (start, size) = self
            .resolved_zones
            .get(zone_index)?
            .group_start_and_size(group);
        if size <= 0 {
            return None;
        }

        let start = start.max(0) as usize;
        let size = size as usize;
        let num_indices = temp_indices
            .iter()
            .skip(start)
            .take(size)
            .filter(|index| **index != INVALID_CULL_INDEX)
            .count();
        if num_indices < 3 {
            return None;
        }

        let first = general.get_random_number() as usize % (num_indices - 2);
        let mut set = [0u16; 3];
        let mut num_set_indices = 0;
        let mut live_index = 0;
        for index in temp_indices.iter().skip(start).take(size) {
            if *index == INVALID_CULL_INDEX {
                continue;
            }
            if live_index < first {
                live_index += 1;
                continue;
            }
            set[num_set_indices] = *index;
            num_set_indices += 1;
            if num_set_indices == 3 {
                return Some(set);
            }
        }

        None
    }

    fn do_we_have_more_than_x_occurrences_of_set(
        &self,
        temp_indices: &[u16],
        count: i32,
        set: [u16; 3],
    ) -> bool {
        let mut cur_count = 0;
        for zone in &self.resolved_zones {
            for group in 0..3 {
                let (start, size) = zone.group_start_and_size(group);
                if size <= 0 {
                    continue;
                }
                let start = start.max(0) as usize;
                let mut n = 0;
                for index in temp_indices.iter().skip(start).take(size as usize) {
                    for set_index in set {
                        if *index == set_index {
                            n += 1;
                        }
                    }
                }
                if n == 3 {
                    cur_count += 1;
                    if cur_count >= count {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn replace_set_for_all_groups(&self, temp_indices: &mut [u16], set: [u16; 3], set_id: u16) {
        for zone in &self.resolved_zones {
            for group in 0..3 {
                let (start, size) = zone.group_start_and_size(group);
                if size <= 0 {
                    continue;
                }
                let start = start.max(0) as usize;
                let size = size as usize;

                let mut n = 0;
                for index in temp_indices.iter().skip(start).take(size) {
                    for set_index in set {
                        if *index == set_index {
                            n += 1;
                        }
                    }
                }
                if n != 3 {
                    continue;
                }

                let mut inserted_set = false;
                for index in temp_indices.iter_mut().skip(start).take(size) {
                    for set_index in set {
                        if *index == set_index {
                            if !inserted_set {
                                *index = set_id;
                            } else {
                                *index = INVALID_CULL_INDEX;
                            }
                            inserted_set = true;
                        }
                    }
                }
            }
        }
    }

    fn tidy_up_and_merge_lists(&mut self, temp_indices: &[u16], extra_indices: &[u16]) {
        let num_temp_indices = temp_indices
            .iter()
            .filter(|index| **index != INVALID_CULL_INDEX)
            .count();

        for zone_index in 0..self.resolved_zones.len() {
            let original_zone = self.resolved_zones[zone_index].clone();
            let start = temp_indices
                .iter()
                .take(original_zone.index_start.max(0) as usize)
                .filter(|index| **index != INVALID_CULL_INDEX)
                .count();
            let mut read_index = original_zone.index_start.max(0) as usize;
            let mut num_buildings = 0;
            let mut num_treadables_plus_10m = 0;
            let mut num_treadables = 0;

            for _ in 0..usize_from_i16(original_zone.group_index_count[0]) {
                if temp_indices.get(read_index).copied() != Some(INVALID_CULL_INDEX) {
                    num_buildings += 1;
                }
                read_index += 1;
            }
            for _ in 0..usize_from_i16(original_zone.group_index_count[1]) {
                if temp_indices.get(read_index).copied() != Some(INVALID_CULL_INDEX) {
                    num_treadables_plus_10m += 1;
                }
                read_index += 1;
            }
            for _ in 0..usize_from_i16(original_zone.group_index_count[2]) {
                if temp_indices.get(read_index).copied() != Some(INVALID_CULL_INDEX) {
                    num_treadables += 1;
                }
                read_index += 1;
            }

            let zone = &mut self.resolved_zones[zone_index];
            zone.index_start = start as i32;
            zone.num_buildings = num_buildings;
            zone.num_treadables_plus_10m = num_treadables_plus_10m;
            zone.num_treadables = num_treadables;
        }

        let mut indices = Vec::with_capacity(num_temp_indices + extra_indices.len());
        for index in temp_indices
            .iter()
            .chain(extra_indices.iter())
            .filter(|index| **index != INVALID_CULL_INDEX)
        {
            if *index < NUMUNCOMPRESSED_CULL_INDICES {
                indices.push(*index);
            } else if let Ok(num_temp_indices) = u16::try_from(num_temp_indices) {
                indices.push(index.saturating_add(num_temp_indices));
            }
        }

        self.indices = indices;
        self.entity_indices_used = self.indices.len() as i32;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CullZoneTransition {
    pub left_zone: Option<usize>,
    pub entered_zone: Option<usize>,
}

impl CResolvedCullZone {
    fn from_gta_cull_zone(zone: &GtaCullZone) -> Self {
        Self {
            position: zone.position,
            min: zone.min,
            max: zone.max,
            index_start: 0,
            group_index_count: [0; 3],
            num_buildings: 0,
            num_treadables_plus_10m: 0,
            num_treadables: 0,
        }
    }

    fn read(bytes: &[u8], offset: &mut usize) -> io::Result<Self> {
        let position = read_vec3(bytes, offset)?;
        let minx = read_f32(bytes, offset)?;
        let maxx = read_f32(bytes, offset)?;
        let miny = read_f32(bytes, offset)?;
        let maxy = read_f32(bytes, offset)?;
        let minz = read_f32(bytes, offset)?;
        let maxz = read_f32(bytes, offset)?;
        let index_start = read_i32(bytes, offset)?;
        let group_index_count = [
            read_i16(bytes, offset)?,
            read_i16(bytes, offset)?,
            read_i16(bytes, offset)?,
        ];
        Ok(Self {
            position,
            min: [minx, miny, minz],
            max: [maxx, maxy, maxz],
            index_start,
            group_index_count,
            num_buildings: read_i16(bytes, offset)?,
            num_treadables_plus_10m: read_i16(bytes, offset)?,
            num_treadables: read_i16(bytes, offset)?,
        })
    }

    fn contains(&self, coors: [f32; 3]) -> bool {
        coors[0] >= self.min[0]
            && coors[0] <= self.max[0]
            && coors[1] >= self.min[1]
            && coors[1] <= self.max[1]
            && coors[2] >= self.min[2]
            && coors[2] <= self.max[2]
    }

    fn calc_dist_to_cull_zone_squared(&self, x: f32, y: f32) -> f32 {
        let rx = if x < self.min[0] {
            squared(x - self.min[0])
        } else if x > self.max[0] {
            squared(x - self.max[0])
        } else {
            0.0
        };
        let ry = if y < self.min[1] {
            squared(y - self.min[1])
        } else if y > self.max[1] {
            squared(y - self.max[1])
        } else {
            0.0
        };
        rx + ry
    }

    fn calc_dist_to_cull_zone(&self, x: f32, y: f32) -> f32 {
        self.calc_dist_to_cull_zone_squared(x, y).sqrt()
    }

    pub fn point_falls_within_zone(&self, position: [f32; 3], radius: f32) -> bool {
        !(self.min[0] - radius > position[0]
            || self.max[0] + radius < position[0]
            || self.min[1] - radius > position[1]
            || self.max[1] + radius < position[1]
            || self.min[2] - radius > position[2]
            || self.max[2] + radius < position[2])
    }

    fn group_start_and_size(&self, group_id: usize) -> (i32, i32) {
        match group_id {
            0 => (self.index_start, self.group_index_count[0] as i32),
            1 => (
                self.index_start + self.group_index_count[0] as i32,
                self.group_index_count[1] as i32,
            ),
            _ => (
                self.index_start
                    + self.group_index_count[0] as i32
                    + self.group_index_count[1] as i32,
                self.group_index_count[2] as i32,
            ),
        }
    }
}

impl CResolvedAttributeZone {
    fn from_gta_cull_zone(zone: &GtaCullZone) -> Self {
        Self {
            min: zone.min,
            max: zone.max,
            attributes: zone.flags as i16,
            wanted_level: zone.wanted_level_drop as i16,
        }
    }

    fn read(bytes: &[u8], offset: &mut usize) -> io::Result<Self> {
        let minx = read_f32(bytes, offset)?;
        let maxx = read_f32(bytes, offset)?;
        let miny = read_f32(bytes, offset)?;
        let maxy = read_f32(bytes, offset)?;
        let minz = read_f32(bytes, offset)?;
        let maxz = read_f32(bytes, offset)?;
        Ok(Self {
            min: [minx, miny, minz],
            max: [maxx, maxy, maxz],
            attributes: read_i16(bytes, offset)?,
            wanted_level: read_i16(bytes, offset)?,
        })
    }
}

fn setup_big_building(entity: &mut CWorldEntity, lod_distance: f32) {
    entity.is_big_building = true;
    entity.streaming_dont_delete = true;
    entity.uses_collision = false;
    if lod_distance > 2000.0 {
        entity.level = 0;
    }
}

fn largest_lod_distance(distances: [f32; 3]) -> f32 {
    distances.into_iter().fold(0.0, f32::max)
}

/// re3 `IsGlass` (ModelIndices.h): the eight `glassfx*` dummy models start
/// invisible in `FileLoader::LoadObjectInstance`.
pub fn is_glass_dummy(name: &str) -> bool {
    matches!(
        name,
        "glassfx1"
            | "glassfx2"
            | "glassfx3"
            | "glassfx4"
            | "glassfx55"
            | "glassfxsub1"
            | "glassfxsub2"
            | "glassfx_composh"
    )
}

fn usize_from_i32(value: i32) -> Option<usize> {
    (value >= 0).then_some(value as usize)
}

fn usize_from_i16(value: i16) -> usize {
    value.max(0) as usize
}

fn uppercase_zone_name(name: &str) -> String {
    name.chars().take(7).flat_map(char::to_uppercase).collect()
}

fn cull_zone_with_re3_position(mut zone: GtaCullZone) -> GtaCullZone {
    let mut position = zone.position;
    if distance(position, [1032.14, -624.255, 24.93]) < 1.0 {
        position = [1061.7, -613.0, 19.0];
    }
    if distance(position, [1029.48, -495.757, 21.98]) < 1.0 {
        position = [1061.4, -506.0, 18.5];
    }

    zone.position = [
        position[0].clamp(zone.min[0], zone.max[0]),
        position[1].clamp(zone.min[1], zone.max[1]),
        position[2].clamp(zone.min[2], zone.max[2]),
    ];
    zone
}

fn merge_col_bounds(bounds: &mut CColBox, lod_bounds: &CColBox) {
    bounds.max.x = bounds.max.x.max(lod_bounds.max.x);
    bounds.max.y = bounds.max.y.max(lod_bounds.max.y);
    bounds.max.z = bounds.max.z.max(lod_bounds.max.z);
    bounds.min.x = bounds.min.x.min(lod_bounds.min.x);
    bounds.min.y = bounds.min.y.min(lod_bounds.min.y);
    bounds.min.z = bounds.min.z.min(lod_bounds.min.z);
}

fn bounding_probe_intersects_zone(
    zone: &CResolvedCullZone,
    vec_min: [f32; 3],
    dir_x: [f32; 3],
    dir_y: [f32; 3],
    dir_z: [f32; 3],
) -> bool {
    for x in 0..9 {
        let pos_x = add_vec3(vec_min, mul_vec3(dir_x, x as f32 / 8.0));
        for y in 0..9 {
            let pos_y = add_vec3(pos_x, mul_vec3(dir_y, y as f32 / 8.0));
            for z in 0..9 {
                let pos_z = add_vec3(pos_y, mul_vec3(dir_z, z as f32 / 8.0));
                if zone.point_falls_within_zone(pos_z, 2.0) {
                    return true;
                }
            }
        }
    }
    false
}

fn sphere_contact_col_model(
    center: [f32; 3],
    radius: f32,
    col: &CColModel,
) -> Option<StaticSphereContact> {
    let bounds = &col.bounding_box;
    if center[0] + radius < bounds.min.x
        || center[0] - radius > bounds.max.x
        || center[1] + radius < bounds.min.y
        || center[1] - radius > bounds.max.y
        || center[2] + radius < bounds.min.z
        || center[2] - radius > bounds.max.z
    {
        return None;
    }
    let mut best: Option<StaticSphereContact> = None;
    let mut register = |normal: [f32; 3], penetration: f32| {
        if penetration > 0.0 && best.is_none_or(|old| penetration > old.penetration) {
            best = Some(StaticSphereContact {
                normal: normalize_vec3(normal),
                penetration,
            });
        }
    };
    for sphere in &col.spheres {
        let offset = [
            center[0] - sphere.center.x,
            center[1] - sphere.center.y,
            center[2] - sphere.center.z,
        ];
        let distance = dot_vec3(offset, offset).sqrt();
        let sum = radius + sphere.radius;
        if distance < sum {
            register(
                if distance > 1.0e-6 {
                    offset
                } else {
                    [0.0, 0.0, 1.0]
                },
                sum - distance,
            );
        }
    }
    for box_ in &col.boxes {
        let inside = center[0] >= box_.min.x
            && center[0] <= box_.max.x
            && center[1] >= box_.min.y
            && center[1] <= box_.max.y
            && center[2] >= box_.min.z
            && center[2] <= box_.max.z;
        if inside {
            let faces = [
                (center[0] - box_.min.x, [-1.0, 0.0, 0.0]),
                (box_.max.x - center[0], [1.0, 0.0, 0.0]),
                (center[1] - box_.min.y, [0.0, -1.0, 0.0]),
                (box_.max.y - center[1], [0.0, 1.0, 0.0]),
                (center[2] - box_.min.z, [0.0, 0.0, -1.0]),
                (box_.max.z - center[2], [0.0, 0.0, 1.0]),
            ];
            if let Some((distance, normal)) = faces.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)) {
                register(normal, radius + distance);
            }
        } else {
            let closest = [
                center[0].clamp(box_.min.x, box_.max.x),
                center[1].clamp(box_.min.y, box_.max.y),
                center[2].clamp(box_.min.z, box_.max.z),
            ];
            let offset = sub_vec3(center, closest);
            let distance = dot_vec3(offset, offset).sqrt();
            if distance < radius {
                register(offset, radius - distance);
            }
        }
    }
    for triangle in &col.triangles {
        let Some(a) = col.vertices.get(triangle.a as usize).map(|v| v.get()) else {
            continue;
        };
        let Some(b) = col.vertices.get(triangle.b as usize).map(|v| v.get()) else {
            continue;
        };
        let Some(c) = col.vertices.get(triangle.c as usize).map(|v| v.get()) else {
            continue;
        };
        let a = [a.x, a.y, a.z];
        let b = [b.x, b.y, b.z];
        let c = [c.x, c.y, c.z];
        let closest = closest_point_on_triangle(center, a, b, c);
        let offset = sub_vec3(center, closest);
        let distance = dot_vec3(offset, offset).sqrt();
        if distance < radius {
            let triangle_normal = normalize_vec3(cross_vec3(sub_vec3(b, a), sub_vec3(c, a)));
            register(
                if distance > 1.0e-6 {
                    offset
                } else {
                    triangle_normal
                },
                radius - distance,
            );
        }
    }
    best
}

fn closest_point_on_triangle(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = sub_vec3(b, a);
    let ac = sub_vec3(c, a);
    let ap = sub_vec3(p, a);
    let d1 = dot_vec3(ab, ap);
    let d2 = dot_vec3(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub_vec3(p, b);
    let d3 = dot_vec3(ab, bp);
    let d4 = dot_vec3(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return add_vec3(a, mul_vec3(ab, d1 / (d1 - d3)));
    }
    let cp = sub_vec3(p, c);
    let d5 = dot_vec3(ab, cp);
    let d6 = dot_vec3(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return add_vec3(a, mul_vec3(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let edge = sub_vec3(c, b);
        return add_vec3(b, mul_vec3(edge, (d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denom = 1.0 / (va + vb + vc);
    add_vec3(
        a,
        add_vec3(mul_vec3(ab, vb * denom), mul_vec3(ac, vc * denom)),
    )
}

fn transform_entity_point(entity: &CWorldEntity, point: [f32; 3]) -> [f32; 3] {
    add_vec3(transform_entity_vector(entity, point), entity.position)
}

/// GTA-space placement matrix of a static world entity, matching the basis
/// `transform_entity_point` applies (rotation only; authored object scale is
/// baked into the model/COL in this port). Used when a promoted dummy's
/// authored pose is transferred to the live ECS CObject.
pub fn world_entity_gta_matrix(entity: &CWorldEntity) -> CMatrix {
    let (right, forward, up) = if let Some([right, forward, up]) = entity.matrix_basis_override {
        (right, forward, up)
    } else {
        (
            rotate_axis_angle(
                [1.0, 0.0, 0.0],
                entity.rotation_axis,
                entity.rotation_degrees,
            ),
            rotate_axis_angle(
                [0.0, 1.0, 0.0],
                entity.rotation_axis,
                entity.rotation_degrees,
            ),
            rotate_axis_angle(
                [0.0, 0.0, 1.0],
                entity.rotation_axis,
                entity.rotation_degrees,
            ),
        )
    };
    let mut matrix = CMatrix::unity();
    matrix.right = CVector::new(right[0], right[1], right[2]);
    matrix.forward = CVector::new(forward[0], forward[1], forward[2]);
    matrix.up = CVector::new(up[0], up[1], up[2]);
    matrix.position = CVector::new(entity.position[0], entity.position[1], entity.position[2]);
    matrix
}

fn transform_entity_vector(entity: &CWorldEntity, vector: [f32; 3]) -> [f32; 3] {
    if let Some([right, forward, up]) = entity.matrix_basis_override {
        return [
            right[0] * vector[0] + forward[0] * vector[1] + up[0] * vector[2],
            right[1] * vector[0] + forward[1] * vector[1] + up[1] * vector[2],
            right[2] * vector[0] + forward[2] * vector[1] + up[2] * vector[2],
        ];
    }
    rotate_axis_angle(vector, entity.rotation_axis, entity.rotation_degrees)
}

fn entity_local_line(entity: &CWorldEntity, point1: [f32; 3], point2: [f32; 3]) -> CColLine {
    CColLine::new(
        col_vec3(inverse_transform_entity_point(entity, point1)),
        col_vec3(inverse_transform_entity_point(entity, point2)),
    )
}

fn inverse_transform_entity_point(entity: &CWorldEntity, point: [f32; 3]) -> [f32; 3] {
    if let Some([right, forward, up]) = entity.matrix_basis_override {
        let relative = sub_vec3(point, entity.position);
        // Garage door bases are orthonormal (possibly reflected), therefore
        // the inverse is the transpose in both cases.
        return [
            dot_vec3(relative, right),
            dot_vec3(relative, forward),
            dot_vec3(relative, up),
        ];
    }
    rotate_axis_angle(
        sub_vec3(point, entity.position),
        entity.rotation_axis,
        -entity.rotation_degrees,
    )
}

fn col_vec3(value: [f32; 3]) -> CColVector {
    CColVector::new(value[0], value[1], value[2])
}

fn rotate_axis_angle(point: [f32; 3], axis: [f32; 3], degrees: f32) -> [f32; 3] {
    let axis_len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if axis_len == 0.0 {
        return point;
    }
    let unit = [axis[0] / axis_len, axis[1] / axis_len, axis[2] / axis_len];
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dot = dot_vec3(unit, point);
    add_vec3(
        add_vec3(mul_vec3(point, cos), mul_vec3(cross_vec3(unit, point), sin)),
        mul_vec3(unit, dot * (1.0 - cos)),
    )
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn squared(value: f32) -> f32 {
    value * value
}

fn add_vec3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub_vec3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul_vec3(value: [f32; 3], scale: f32) -> [f32; 3] {
    [value[0] * scale, value[1] * scale, value[2] * scale]
}

fn dot_vec3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross_vec3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize_vec3(value: [f32; 3]) -> [f32; 3] {
    let len = dot_vec3(value, value).sqrt();
    if len == 0.0 {
        return value;
    }
    mul_vec3(value, 1.0 / len)
}

fn zone_grid_elements(size: f32) -> usize {
    if size < 1.0 {
        2
    } else {
        ((size + 1.0) as usize).min(32).max(2)
    }
}

fn grid_position(value: f32, min: f32, max: f32, elements: usize) -> usize {
    if max == min {
        return 0;
    }
    ((elements as f32 * (value - min) / (max - min)) as isize).clamp(0, elements as isize - 1)
        as usize
}

fn grid_neighbours(
    x: usize,
    y: usize,
    z: usize,
    elements_x: usize,
    elements_y: usize,
    elements_z: usize,
) -> Vec<(usize, usize, usize)> {
    let mut neighbours = Vec::with_capacity(6);
    if x > 0 {
        neighbours.push((x - 1, y, z));
    }
    if x < elements_x - 1 {
        neighbours.push((x + 1, y, z));
    }
    if y > 0 {
        neighbours.push((x, y - 1, z));
    }
    if y < elements_y - 1 {
        neighbours.push((x, y + 1, z));
    }
    if z > 0 {
        neighbours.push((x, y, z - 1));
    }
    if z < elements_z - 1 {
        neighbours.push((x, y, z + 1));
    }
    neighbours
}

fn add_plane_extrema(
    points: &mut Vec<(usize, usize, usize)>,
    mem: &[u8],
    elements_x: usize,
    elements_y: usize,
    elements_z: usize,
    axis: usize,
    plane: usize,
) {
    const FLAG_FREE: u8 = 1;
    let index = |x: usize, y: usize, z: usize| (x * elements_y + y) * elements_z + z;
    let mut extrema: [Option<(usize, usize, usize, isize)>; 4] = [None, None, None, None];

    for x in 0..elements_x {
        for y in 0..elements_y {
            for z in 0..elements_z {
                if [x, y, z][axis] != plane || mem[index(x, y, z)] & FLAG_FREE == 0 {
                    continue;
                }
                let (a, b) = match axis {
                    0 => (y as isize, z as isize),
                    1 => (x as isize, z as isize),
                    _ => (x as isize, y as isize),
                };
                update_extrema(&mut extrema[0], (x, y, z), a + b, true);
                update_extrema(&mut extrema[1], (x, y, z), a + b, false);
                update_extrema(&mut extrema[2], (x, y, z), a - b, true);
                update_extrema(&mut extrema[3], (x, y, z), a - b, false);
            }
        }
    }

    for point in extrema.into_iter().flatten() {
        points.push((point.0, point.1, point.2));
    }
}

fn add_plane_subdivide_points(
    points: &mut Vec<(usize, usize, usize)>,
    mem: &[u8],
    elements_x: usize,
    elements_y: usize,
    elements_z: usize,
    axis: usize,
    plane: usize,
) {
    const FLAG_FREE: u8 = 1;
    let index = |x: usize, y: usize, z: usize| (x * elements_y + y) * elements_z + z;
    let mut min_a = usize::MAX;
    let mut min_b = usize::MAX;
    let mut max_a = 0usize;
    let mut max_b = 0usize;

    for x in 0..elements_x {
        for y in 0..elements_y {
            for z in 0..elements_z {
                if [x, y, z][axis] != plane || mem[index(x, y, z)] & FLAG_FREE == 0 {
                    continue;
                }
                let (a, b) = match axis {
                    0 => (y, z),
                    1 => (x, z),
                    _ => (x, y),
                };
                min_a = min_a.min(a);
                min_b = min_b.min(b);
                max_a = max_a.max(a);
                max_b = max_b.max(b);
            }
        }
    }

    if max_a == 0 || max_b == 0 || min_a == usize::MAX || min_b == usize::MAX {
        return;
    }

    for point_a in 0..2 {
        for point_b in 0..2 {
            let new_a =
                ((point_a as f32 + 0.5) * (max_a - min_a) as f32 * 0.5 + min_a as f32) as usize;
            let new_b =
                ((point_b as f32 + 0.5) * (max_b - min_b) as f32 * 0.5 + min_b as f32) as usize;
            let (x, y, z) = match axis {
                0 => (plane, new_a, new_b),
                1 => (new_a, plane, new_b),
                _ => (new_a, new_b, plane),
            };
            if x < elements_x
                && y < elements_y
                && z < elements_z
                && mem[index(x, y, z)] & FLAG_FREE != 0
            {
                points.push((x, y, z));
            }
        }
    }
}

fn update_extrema(
    slot: &mut Option<(usize, usize, usize, isize)>,
    point: (usize, usize, usize),
    value: isize,
    want_min: bool,
) {
    let replace = slot.as_ref().is_none_or(|current| {
        if want_min {
            value < current.3
        } else {
            value > current.3
        }
    });
    if replace {
        *slot = Some((point.0, point.1, point.2, value));
    }
}

const EXTRA_FUDGE_POINTS: [[f32; 3]; 60] = [
    [978.0, -394.0, 18.0],
    [1189.7, -414.6, 27.0],
    [978.8, -391.0, 19.0],
    [1199.0, -502.3, 28.0],
    [1037.0, -391.9, 18.4],
    [1140.0, -608.7, 16.0],
    [1051.0, -26.0, 11.0],
    [951.5, -345.1, 12.0],
    [958.2, -394.6, 16.0],
    [1036.5, -390.0, 15.2],
    [960.6, -390.5, 20.9],
    [1061.0, -640.6, 16.3],
    [1034.5, -388.96, 14.78],
    [1038.4, -13.98, 12.2],
    [1047.2, -16.7, 10.6],
    [1257.9, -333.3, 40.0],
    [885.6, -424.9, 17.0],
    [1127.5, -795.8, 17.7],
    [1133.0, -716.0, 19.0],
    [1125.0, -694.0, 18.5],
    [1125.0, -670.0, 16.3],
    [1051.6, 36.3, 17.9],
    [1054.6, -11.4, 15.0],
    [1058.9, -278.0, 15.0],
    [1059.4, -261.0, 10.9],
    [1051.5, -638.5, 16.5],
    [1058.2, -643.4, 15.5],
    [1058.2, -643.4, 18.0],
    [826.0, -260.0, 7.0],
    [826.0, -260.0, 11.0],
    [833.0, -603.6, 16.4],
    [833.0, -603.6, 20.0],
    [1002.0, -318.5, 10.5],
    [998.0, -318.0, 9.8],
    [1127.0, -183.0, 18.1],
    [1123.0, -331.5, 23.8],
    [1123.8, -429.0, 24.0],
    [1197.0, -30.0, 13.7],
    [1117.5, -230.0, 17.3],
    [1117.5, -230.0, 20.0],
    [1120.0, -281.6, 21.5],
    [1120.0, -281.6, 24.0],
    [1084.5, -1022.7, 17.0],
    [1071.5, 5.4, 4.6],
    [1177.2, -215.7, 27.6],
    [841.6, -460.0, 19.7],
    [874.8, -456.6, 16.6],
    [918.3, -451.8, 17.8],
    [844.0, -495.7, 16.7],
    [842.0, -493.4, 21.0],
    [1433.5, -774.4, 16.9],
    [1051.0, -205.0, 7.5],
    [885.5, -425.6, 15.6],
    [182.6, -470.4, 27.8],
    [132.5, -930.2, 29.0],
    [124.7, -904.0, 28.0],
    [-50.0, -686.0, 22.0],
    [-49.1, -694.5, 22.5],
    [1063.8, -404.45, 16.2],
    [1062.2, -405.5, 17.0],
];

fn dedup_grid_points(points: &mut Vec<(usize, usize, usize)>) {
    let mut unique = Vec::with_capacity(points.len());
    for point in points.drain(..) {
        if !unique.contains(&point) {
            unique.push(point);
        }
    }
    *points = unique;
}

fn read_vec3(bytes: &[u8], offset: &mut usize) -> io::Result<[f32; 3]> {
    Ok([
        read_f32(bytes, offset)?,
        read_f32(bytes, offset)?,
        read_f32(bytes, offset)?,
    ])
}

fn read_f32(bytes: &[u8], offset: &mut usize) -> io::Result<f32> {
    let data = read_array::<4>(bytes, offset)?;
    Ok(f32::from_le_bytes(data))
}

fn read_i32(bytes: &[u8], offset: &mut usize) -> io::Result<i32> {
    let data = read_array::<4>(bytes, offset)?;
    Ok(i32::from_le_bytes(data))
}

fn read_i16(bytes: &[u8], offset: &mut usize) -> io::Result<i16> {
    let data = read_array::<2>(bytes, offset)?;
    Ok(i16::from_le_bytes(data))
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> io::Result<u16> {
    let data = read_array::<2>(bytes, offset)?;
    Ok(u16::from_le_bytes(data))
}

fn read_array<const N: usize>(bytes: &[u8], offset: &mut usize) -> io::Result<[u8; N]> {
    let end = *offset + N;
    let Some(slice) = bytes.get(*offset..end) else {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "cullzone.dat ended before the expected fixed-size data was read",
        ));
    };
    *offset = end;
    Ok(slice.try_into().expect("slice length is fixed"))
}

#[cfg(test)]
mod sector_list_order_tests {
    use super::*;

    fn test_building(position: [f32; 3], big: bool) -> CWorldEntity {
        CWorldEntity {
            entity_type: EEntityType::Building,
            model_index: 0,
            name: String::new(),
            position,
            scale: [1.0; 3],
            rotation_axis: [0.0, 0.0, 1.0],
            rotation_w: 1.0,
            rotation_degrees: 0.0,
            matrix_basis_override: None,
            render_position_offset: [0.0; 3],
            level: 1,
            is_static: true,
            is_visible: true,
            is_big_building: big,
            is_subway: false,
            streaming_dont_delete: big,
            uses_collision: true,
            zone_culled: false,
            zone_culled2: false,
            building_pool_slot: None,
            treadable_pool_slot: None,
            map_object_slot: None,
        }
    }

    #[test]
    fn sector_lists_use_cpp_head_insertion_order() {
        let mut world = CWorld::default();
        let model_info = CModelInfo::initialise_empty();
        for _ in 0..2 {
            let index = world.entities.len();
            world.entities.push(test_building([0.0, 0.0, 0.0], false));
            world.add(index, &model_info);
        }

        let x = CWorld::get_sector_index_x(0.0);
        let y = CWorld::get_sector_index_y(0.0);
        assert_eq!(world.sectors[y * NUMSECTORS_X + x].buildings, vec![1, 0]);
    }

    #[test]
    fn big_building_lists_use_cpp_head_insertion_order() {
        let mut world = CWorld::default();
        let model_info = CModelInfo::initialise_empty();
        for _ in 0..2 {
            let index = world.entities.len();
            world.entities.push(test_building([0.0, 0.0, 0.0], true));
            world.add(index, &model_info);
        }

        assert_eq!(world.big_buildings_by_level[1], vec![1, 0]);
    }

    #[test]
    fn object_dat_ipl_dummy_gains_collision_inside_population_radius() {
        let mut world = CWorld::default();
        let mut model_info = CModelInfo::initialise_empty();
        let model_id = 10usize;
        model_info.add_simple_model(model_id);
        model_info.set_object_id(model_id, 0);

        let mut entity = test_building([10.0, 0.0, 0.0], false);
        entity.entity_type = EEntityType::Dummy;
        entity.model_index = model_id;
        entity.uses_collision = false;
        world.entities.push(entity);
        world.object_dummy_pool.push(0);

        // A one-entry native pool is visited in the final 1/32 slice.
        let mut promoted = Vec::new();
        for frame in 0..32 {
            promoted =
                world.update_object_dummy_collision_lifecycle(CVector::ZERO, frame, &model_info);
        }
        assert_eq!(
            promoted,
            vec![ObjectDummyLifecycleChange::Promoted {
                entity_index: 0,
                model_index: model_id
            }]
        );
        // The promoted world entity delegates render and collision to the ECS
        // CObject, so its own static projection is hidden and collisionless.
        assert!(!world.entities[0].uses_collision);
        assert!(!world.entities[0].is_visible);
        assert_eq!(world.entities[0].entity_type, EEntityType::Object);
        assert!(world
            .sectors
            .iter()
            .any(|sector| sector.objects.contains(&0)));
        assert!(!world
            .sectors
            .iter()
            .any(|sector| sector.dummies.contains(&0)));

        let mut demoted = Vec::new();
        for frame in 0..32 {
            demoted = world.update_object_dummy_collision_lifecycle(
                CVector::new(200.0, 0.0, 0.0),
                frame,
                &model_info,
            );
        }
        assert_eq!(
            demoted,
            vec![ObjectDummyLifecycleChange::Demoted { entity_index: 0 }]
        );
        assert!(!world.entities[0].uses_collision);
        assert!(world.entities[0].is_visible);
        assert_eq!(world.entities[0].entity_type, EEntityType::Dummy);
        assert!(world
            .sectors
            .iter()
            .any(|sector| sector.dummies.contains(&0)));
        assert!(!world
            .sectors
            .iter()
            .any(|sector| sector.objects.contains(&0)));
    }

    #[test]
    fn static_collision_queries_include_object_sector_lists_like_cpp() {
        let mut world = CWorld::default();
        let mut model_info = CModelInfo::initialise_empty();
        let object_model_id = 11usize;
        let min = CColVector::new(-0.5, -0.5, -0.5);
        let max = CColVector::new(0.5, 0.5, 0.5);
        let mut object_col = CColModel::new(
            CColSphere::new(1.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(min, max, 0, 0),
            Vec::new(),
        );
        object_col.boxes.push(CColBox::new(min, max, 0, 0));
        model_info
            .add_simple_model(object_model_id)
            .base
            .set_col_model(object_col, true);

        let mut object = test_building([0.0, 0.0, 0.0], false);
        object.entity_type = EEntityType::Object;
        object.model_index = object_model_id;
        let object_index = world.entities.len();
        world.entities.push(object);
        world.add(object_index, &model_info);

        assert!(
            world
                .sphere_contact_static_world([0.6, 0.0, 0.0], 0.2, &model_info)
                .is_some(),
            "CWorld::TestSphereAgainstWorld checks ENTITYLIST_OBJECTS"
        );
        assert!(
            world
                .process_line_of_sight_static_contact(
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, -1.0],
                    &model_info,
                    None,
                    false,
                )
                .is_some(),
            "CWorld::ProcessVerticalLine checks ENTITYLIST_OBJECTS even when dummies are disabled"
        );

        let moving = CColModel::new(
            CColSphere::new(0.2, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(
                CColVector::new(-0.2, -0.2, -0.2),
                CColVector::new(0.2, 0.2, 0.2),
                0,
                0,
            ),
            vec![CColSphere::new(0.2, CColVector::new(0.0, 0.0, 0.0), 0, 0)],
        );
        assert!(
            !world
                .col_model_contacts_static_world(
                    [0.6, 0.0, 0.0],
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    &moving,
                    &model_info,
                )
                .is_empty(),
            "CPhysical::ProcessCollisionSectorList checks ENTITYLIST_OBJECTS"
        );
    }

    #[test]
    fn static_collision_point_limit_is_per_entity_like_cpp() {
        let mut world = CWorld::default();
        let mut model_info = CModelInfo::initialise_empty();

        for (model_id, x) in [(21usize, 0.0f32), (22usize, 10.0f32)] {
            let min = CColVector::new(-0.5, -0.5, -0.5);
            let max = CColVector::new(0.5, 0.5, 0.5);
            let mut col = CColModel::new(
                CColSphere::new(1.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
                CColBox::new(min, max, 0, 0),
                Vec::new(),
            );
            col.boxes.push(CColBox::new(min, max, 0, 0));
            model_info
                .add_simple_model(model_id)
                .base
                .set_col_model(col, true);

            let mut entity = test_building([x, 0.0, 0.0], false);
            entity.model_index = model_id;
            let entity_index = world.entities.len();
            world.entities.push(entity);
            world.add(entity_index, &model_info);
        }

        let mut moving_spheres = Vec::with_capacity(MAX_COLLISION_POINTS + 1);
        for _ in 0..MAX_COLLISION_POINTS {
            moving_spheres.push(CColSphere::new(0.2, CColVector::new(0.0, 0.0, 0.0), 0, 0));
        }
        moving_spheres.push(CColSphere::new(0.2, CColVector::new(10.0, 0.0, 0.0), 0, 0));
        let moving = CColModel::new(
            CColSphere::new(11.0, CColVector::new(5.0, 0.0, 0.0), 0, 0),
            CColBox::new(
                CColVector::new(-0.2, -0.2, -0.2),
                CColVector::new(10.2, 0.2, 0.2),
                0,
                0,
            ),
            moving_spheres,
        );

        let contacts = world.col_model_contacts_static_world(
            [0.0, 0.0, 0.0],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            &moving,
            &model_info,
        );
        assert!(
            contacts.iter().any(|contact| contact.entity_index == 1),
            "ProcessCollisionSectorList resets its MAX_COLLISION_POINTS scratch array for each entity"
        );
    }

    #[test]
    fn script_building_search_prefers_normal_sector_match_before_big_building() {
        let mut world = CWorld::default();
        let model_info = CModelInfo::initialise_empty();

        let mut normal = test_building([10.0, 0.0, 0.0], false);
        normal.model_index = 77;
        let normal_index = world.entities.len();
        world.entities.push(normal);
        world.add(normal_index, &model_info);

        let mut big = test_building([1.0, 0.0, 0.0], true);
        big.model_index = 77;
        let big_index = world.entities.len();
        world.entities.push(big);
        world.add(big_index, &model_info);

        assert_eq!(
            world.find_nearest_building_of_model(77, [0.0, 0.0, 0.0], 20.0),
            Some(normal_index),
            "Script4.cpp falls back to big-building lists only when the normal sector query is empty"
        );
    }

    #[test]
    fn script_visibility_search_includes_objects_before_big_building_fallback() {
        let mut world = CWorld::default();
        let model_info = CModelInfo::initialise_empty();

        let mut object = test_building([8.0, 0.0, 0.0], false);
        object.entity_type = EEntityType::Object;
        object.model_index = 88;
        let object_index = world.entities.len();
        world.entities.push(object);
        world.add(object_index, &model_info);

        let mut big = test_building([1.0, 0.0, 0.0], true);
        big.model_index = 88;
        let big_index = world.entities.len();
        world.entities.push(big);
        world.add(big_index, &model_info);

        assert_eq!(
            world.find_nearest_entity_of_model(88, [0.0, 0.0, 0.0], 20.0),
            Some(object_index),
            "SET_VISIBILITY searches normal building/object/dummy sector lists before big-building fallback"
        );
    }
}

// === FAZ10e-world: moved from core/Game.rs (verbatim, parity-preserving) ===

/// Radius used when measuring “world at spawn” presence (metres, game units).
pub const WORLD_SPAWN_RADIUS: f32 = 250.0;

/// Count IPL/world entities within `radius` of `center` (XY plane).
pub fn count_world_entities_near(
    world: &crate::core::World::CWorld,
    center: rust_gta_3_core::core::Coordinates::CVector,
    radius: f32,
) -> usize {
    let r2 = radius * radius;
    world
        .entities
        .iter()
        .filter(|entity| {
            let dx = entity.position[0] - center.x;
            let dy = entity.position[1] - center.y;
            dx * dx + dy * dy <= r2
        })
        .count()
}
