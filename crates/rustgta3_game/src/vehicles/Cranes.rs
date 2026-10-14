//! re3 reference: references/re-GTA/src/vehicles/Cranes.cpp
//! re3 reference: references/re-GTA/src/vehicles/Cranes.h
//!
//! `CCranes::InitCranes` lifecycle state.

use bevy::prelude::*;
use rustgta3_core::core::Coordinates::CVector;

use crate::core::World::{CWorld, EEntityType};
use crate::save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter};

pub const NUM_CRANES: usize = 8;
pub const CRANE_SAVE_RECORD_SIZE: usize = 0x80;
pub const CRANES_SAVE_SIZE: usize = 2 * 4 + NUM_CRANES * CRANE_SAVE_RECORD_SIZE;
pub const DEFAULT_OFFSET: f32 = 20.0;
pub const TWO_PI: f32 = std::f32::consts::PI * 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CCraneStatus {
    None = 0,
    Activated = 1,
    Deactivated = 2,
}

impl CCraneStatus {
    fn from_re3(value: u8) -> Self {
        match value {
            1 => Self::Activated,
            2 => Self::Deactivated,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CCraneState {
    Idle = 0,
    GoingTowardsTarget = 1,
    LiftingTarget = 2,
    GoingTowardsTargetOnlyHeight = 3,
    RotatingTarget = 4,
    DroppingTarget = 5,
}

impl CCraneState {
    fn from_re3(value: u8) -> Self {
        match value {
            1 => Self::GoingTowardsTarget,
            2 => Self::LiftingTarget,
            3 => Self::GoingTowardsTargetOnlyHeight,
            4 => Self::RotatingTarget,
            5 => Self::DroppingTarget,
            _ => Self::Idle,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CCraneRuntime {
    pub crane_entity_index: usize,
    pub crane_entity_pool_index: Option<i32>,
    pub hook_object_pool_index: Option<i32>,
    pub audio_entity: i32,
    pub pickup_x1: f32,
    pub pickup_x2: f32,
    pub pickup_y1: f32,
    pub pickup_y2: f32,
    pub dropoff_target: CVector,
    pub dropoff_heading: f32,
    pub pickup_angle: f32,
    pub dropoff_angle: f32,
    pub pickup_distance: f32,
    pub dropoff_distance: f32,
    pub pickup_height: f32,
    pub dropoff_height: f32,
    pub crane_status: CCraneStatus,
    pub crane_state: CCraneState,
    pub hook_angle: f32,
    pub hook_offset: f32,
    pub hook_height: f32,
    pub hook_init_position: CVector,
    pub hook_current_position: [f32; 3],
    pub hook_velocity: [f32; 2],
    pub vehicle_picked_up_pool_index: Option<i32>,
    pub time_for_next_check: u32,
    pub vehicles_collected: u8,
    pub is_crusher: bool,
    pub is_military_crane: bool,
    pub was_military_crane: bool,
    pub is_top: bool,
    pub hook_object_required: bool,
    pub audio_entity_requested: bool,
}

impl CCraneRuntime {
    fn empty() -> Self {
        Self {
            crane_entity_index: 0,
            crane_entity_pool_index: None,
            hook_object_pool_index: None,
            audio_entity: 0,
            pickup_x1: 0.0,
            pickup_x2: 0.0,
            pickup_y1: 0.0,
            pickup_y2: 0.0,
            dropoff_target: CVector::ZERO,
            dropoff_heading: 0.0,
            pickup_angle: 0.0,
            dropoff_angle: 0.0,
            pickup_distance: 0.0,
            dropoff_distance: 0.0,
            pickup_height: 0.0,
            dropoff_height: 0.0,
            crane_status: CCraneStatus::None,
            crane_state: CCraneState::Idle,
            hook_angle: 0.0,
            hook_offset: 0.0,
            hook_height: 0.0,
            hook_init_position: CVector::ZERO,
            hook_current_position: [0.0; 3],
            hook_velocity: [0.0; 2],
            vehicle_picked_up_pool_index: None,
            time_for_next_check: 0,
            vehicles_collected: 0,
            is_crusher: false,
            is_military_crane: false,
            was_military_crane: false,
            is_top: false,
            hook_object_required: false,
            audio_entity_requested: false,
        }
    }

    fn write_save(&self, writer: &mut SaveBufWriter) {
        writer.write(index_plus_one(self.crane_entity_pool_index));
        writer.write(index_plus_one(self.hook_object_pool_index));
        writer.write(self.audio_entity);
        writer.write(self.pickup_x1);
        writer.write(self.pickup_x2);
        writer.write(self.pickup_y1);
        writer.write(self.pickup_y2);
        writer.write(self.dropoff_target.x);
        writer.write(self.dropoff_target.y);
        writer.write(self.dropoff_target.z);
        writer.write(self.dropoff_heading);
        writer.write(self.pickup_angle);
        writer.write(self.dropoff_angle);
        writer.write(self.pickup_distance);
        writer.write(self.dropoff_distance);
        writer.write(self.pickup_height);
        writer.write(self.dropoff_height);
        writer.write(self.hook_angle);
        writer.write(self.hook_offset);
        writer.write(self.hook_height);
        writer.write(self.hook_init_position.x);
        writer.write(self.hook_init_position.y);
        writer.write(self.hook_init_position.z);
        writer.write(self.hook_current_position[0]);
        writer.write(self.hook_current_position[1]);
        writer.write(self.hook_current_position[2]);
        writer.write(self.hook_velocity[0]);
        writer.write(self.hook_velocity[1]);
        writer.write(index_plus_one(self.vehicle_picked_up_pool_index));
        writer.write(self.time_for_next_check);
        writer.write(self.crane_status as u8);
        writer.write(self.crane_state as u8);
        writer.write(self.vehicles_collected);
        writer.write(bool_to_u8(self.is_crusher));
        writer.write(bool_to_u8(self.is_military_crane));
        writer.write(bool_to_u8(self.was_military_crane));
        writer.write(bool_to_u8(self.is_top));
        writer.zero(1);
    }

    fn read_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let crane_entity_pool_index = pool_index_from_saved(reader.read::<i32>()?);
        let hook_object_pool_index = pool_index_from_saved(reader.read::<i32>()?);
        let audio_entity = reader.read::<i32>()?;
        let pickup_x1 = reader.read::<f32>()?;
        let pickup_x2 = reader.read::<f32>()?;
        let pickup_y1 = reader.read::<f32>()?;
        let pickup_y2 = reader.read::<f32>()?;
        let dropoff_target = CVector::new(
            reader.read::<f32>()?,
            reader.read::<f32>()?,
            reader.read::<f32>()?,
        );
        let dropoff_heading = reader.read::<f32>()?;
        let pickup_angle = reader.read::<f32>()?;
        let dropoff_angle = reader.read::<f32>()?;
        let pickup_distance = reader.read::<f32>()?;
        let dropoff_distance = reader.read::<f32>()?;
        let pickup_height = reader.read::<f32>()?;
        let dropoff_height = reader.read::<f32>()?;
        let hook_angle = reader.read::<f32>()?;
        let hook_offset = reader.read::<f32>()?;
        let hook_height = reader.read::<f32>()?;
        let hook_init_position = CVector::new(
            reader.read::<f32>()?,
            reader.read::<f32>()?,
            reader.read::<f32>()?,
        );
        let hook_current_position = [
            reader.read::<f32>()?,
            reader.read::<f32>()?,
            reader.read::<f32>()?,
        ];
        let hook_velocity = [reader.read::<f32>()?, reader.read::<f32>()?];
        let vehicle_picked_up_pool_index = pool_index_from_saved(reader.read::<i32>()?);
        let time_for_next_check = reader.read::<u32>()?;
        let crane_status = CCraneStatus::from_re3(reader.read::<u8>()?);
        let crane_state = CCraneState::from_re3(reader.read::<u8>()?);
        let vehicles_collected = reader.read::<u8>()?;
        let is_crusher = reader.read::<u8>()? != 0;
        let is_military_crane = reader.read::<u8>()? != 0;
        let was_military_crane = reader.read::<u8>()? != 0;
        let is_top = reader.read::<u8>()? != 0;
        reader.skip(1)?;
        Ok(Self {
            crane_entity_index: crane_entity_pool_index.unwrap_or(0) as usize,
            crane_entity_pool_index,
            hook_object_pool_index,
            audio_entity,
            pickup_x1,
            pickup_x2,
            pickup_y1,
            pickup_y2,
            dropoff_target,
            dropoff_heading,
            pickup_angle,
            dropoff_angle,
            pickup_distance,
            dropoff_distance,
            pickup_height,
            dropoff_height,
            crane_status,
            crane_state,
            hook_angle,
            hook_offset,
            hook_height,
            hook_init_position,
            hook_current_position,
            hook_velocity,
            vehicle_picked_up_pool_index,
            time_for_next_check,
            vehicles_collected,
            is_crusher,
            is_military_crane,
            was_military_crane,
            is_top,
            hook_object_required: hook_object_pool_index.is_some(),
            audio_entity_requested: audio_entity != 0,
        })
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CCranes {
    pub cars_collected_military_crane: u32,
    pub num_cranes: i32,
    pub crane_slots_present: [bool; NUM_CRANES],
    pub world_scan_requested: bool,
    pub world_scan_completed: bool,
    pub crane_entity_indices: Vec<usize>,
    pub crane_runtimes: Vec<CCraneRuntime>,
    pub hooks_spawned: bool,
    pub audio_entities_created: bool,
}

impl CCranes {
    /// re3 `CCranes::HaveAllCarsBeenCollectedByMilitaryCrane`
    /// (vehicles/Cranes.cpp:507): `(CarsCollectedMilitaryCrane & 0x7F) == 0x7F`.
    pub fn have_all_cars_been_collected_by_military_crane(&self) -> bool {
        (self.cars_collected_military_crane & 0x7F) == 0x7F
    }

    /// re3 `CCranes::IsThisCarBeingTargettedByAnyCrane`.
    pub fn is_vehicle_targeted_by_any_crane(&self, vehicle_pool_index: i32) -> bool {
        self.crane_runtimes
            .iter()
            .any(|crane| crane.vehicle_picked_up_pool_index == Some(vehicle_pool_index))
    }

    pub fn init_deferred_until_world_entities() -> Self {
        Self {
            cars_collected_military_crane: 0,
            num_cranes: 0,
            crane_slots_present: [false; NUM_CRANES],
            world_scan_requested: true,
            world_scan_completed: false,
            crane_entity_indices: Vec::new(),
            crane_runtimes: Vec::new(),
            hooks_spawned: false,
            audio_entities_created: false,
        }
    }

    pub fn init_from_world_without_spawning_hooks(world: &CWorld) -> Self {
        let crane_entity_indices: Vec<usize> = world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| {
                entity.entity_type == EEntityType::Building
                    && (entity.name.eq_ignore_ascii_case("doc_crane_cab")
                        || entity.name.eq_ignore_ascii_case("cranetopb")
                        || entity.name.eq_ignore_ascii_case("cranetopa"))
            })
            .map(|(index, _)| index)
            .take(NUM_CRANES)
            .collect();

        let mut crane_slots_present = [false; NUM_CRANES];
        for slot in crane_slots_present
            .iter_mut()
            .take(crane_entity_indices.len())
        {
            *slot = true;
        }
        let crane_runtimes = crane_entity_indices
            .iter()
            .enumerate()
            .filter_map(|(slot, entity_index)| {
                let entity = world.entities.get(*entity_index)?;
                Some(add_this_one_crane_runtime(slot, *entity_index, entity))
            })
            .collect();

        Self {
            cars_collected_military_crane: 0,
            num_cranes: crane_entity_indices.len() as i32,
            crane_slots_present,
            world_scan_requested: true,
            world_scan_completed: true,
            crane_entity_indices,
            crane_runtimes,
            hooks_spawned: false,
            audio_entities_created: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn activate_crane(
        &mut self,
        world: &CWorld,
        pickup_min: CVector,
        pickup_max: CVector,
        dropoff: CVector,
        heading_radians: f32,
        is_crusher: bool,
        is_military: bool,
        crane_position: CVector,
    ) -> bool {
        const MAX_DISTANCE_TO_FIND_CRANE: f32 = 10.0;
        const MIN_VALID_POSITION: f32 = -10_000.0;
        const MILITARY_Z: f32 = 10.7862;
        const CRUSHER_Z: f32 = -0.950;

        let search =
            if crane_position.x <= MIN_VALID_POSITION || crane_position.y <= MIN_VALID_POSITION {
                dropoff
            } else {
                crane_position
            };
        let Some(index) = self
            .crane_runtimes
            .iter()
            .enumerate()
            .filter_map(|(index, crane)| {
                let position = world.entities.get(crane.crane_entity_index)?.position;
                let distance =
                    ((search.x - position[0]).powi(2) + (search.y - position[1]).powi(2)).sqrt();
                (distance < MAX_DISTANCE_TO_FIND_CRANE).then_some((index, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
        else {
            return false;
        };
        let crane = &mut self.crane_runtimes[index];
        let Some(position) = world
            .entities
            .get(crane.crane_entity_index)
            .map(|entity| entity.position)
        else {
            return false;
        };
        crane.pickup_x1 = pickup_min.x;
        crane.pickup_x2 = pickup_max.x;
        crane.pickup_y1 = pickup_min.y;
        crane.pickup_y2 = pickup_max.y;
        crane.dropoff_target = dropoff;
        crane.dropoff_heading = heading_radians;
        crane.crane_status = CCraneStatus::Activated;
        crane.crane_state = CCraneState::Idle;
        crane.vehicle_picked_up_pool_index = None;
        crane.vehicles_collected = 0;
        crane.is_crusher = is_crusher;
        crane.is_military_crane = is_military;
        crane.was_military_crane = is_military || crane.was_military_crane;
        crane.time_for_next_check = 0;

        let pickup_x = (pickup_min.x + pickup_max.x) * 0.5;
        let pickup_y = (pickup_min.y + pickup_max.y) * 0.5;
        let pickup_z = if is_crusher {
            CRUSHER_Z
        } else if is_military {
            MILITARY_Z
        } else {
            0.0
        };
        let parameters = |x: f32, y: f32, z: f32| {
            let dx = x - position[0];
            let dy = y - position[1];
            (
                dy.atan2(dx).rem_euclid(TWO_PI),
                (dx * dx + dy * dy).sqrt(),
                z,
            )
        };
        (
            crane.pickup_angle,
            crane.pickup_distance,
            crane.pickup_height,
        ) = parameters(pickup_x, pickup_y, pickup_z);
        (
            crane.dropoff_angle,
            crane.dropoff_distance,
            crane.dropoff_height,
        ) = parameters(dropoff.x, dropoff.y, dropoff.z);
        true
    }

    pub fn deactivate_crane(&mut self, world: &CWorld, position: CVector) -> bool {
        const MAX_DISTANCE_TO_FIND_CRANE: f32 = 10.0;
        let Some(index) = self
            .crane_runtimes
            .iter()
            .enumerate()
            .filter_map(|(index, crane)| {
                let crane_position = world.entities.get(crane.crane_entity_index)?.position;
                let distance = ((position.x - crane_position[0]).powi(2)
                    + (position.y - crane_position[1]).powi(2))
                .sqrt();
                (distance < MAX_DISTANCE_TO_FIND_CRANE).then_some((index, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
        else {
            return false;
        };
        let crane = &mut self.crane_runtimes[index];
        crane.crane_status = CCraneStatus::Deactivated;
        crane.crane_state = CCraneState::Idle;
        true
    }

    pub fn save(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write(self.num_cranes);
        writer.write(self.cars_collected_military_crane);
        for index in 0..NUM_CRANES {
            let runtime = self
                .crane_runtimes
                .get(index)
                .cloned()
                .unwrap_or_else(CCraneRuntime::empty);
            runtime.write_save(&mut writer);
        }
        debug_assert_eq!(writer.offset(), CRANES_SAVE_SIZE);
        writer.into_inner()
    }

    pub fn load(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let num_cranes = reader.read::<i32>()?;
        let cars_collected_military_crane = reader.read::<u32>()?;
        let mut crane_runtimes = Vec::with_capacity(NUM_CRANES);
        let mut crane_slots_present = [false; NUM_CRANES];
        let mut crane_entity_indices = Vec::new();
        for index in 0..NUM_CRANES {
            let runtime = CCraneRuntime::read_save(&mut reader)?;
            if (index as i32) < num_cranes {
                crane_slots_present[index] = true;
                crane_entity_indices.push(runtime.crane_entity_index);
                crane_runtimes.push(runtime);
            }
        }
        Ok(Self {
            cars_collected_military_crane,
            num_cranes,
            crane_slots_present,
            world_scan_requested: true,
            world_scan_completed: false,
            crane_entity_indices,
            crane_runtimes,
            hooks_spawned: false,
            audio_entities_created: false,
        })
    }
}

fn add_this_one_crane_runtime(
    slot: usize,
    crane_entity_index: usize,
    entity: &crate::core::World::CWorldEntity,
) -> CCraneRuntime {
    let mut hook_angle = slot as f32;
    while hook_angle > TWO_PI {
        hook_angle -= TWO_PI;
    }
    let is_top = !entity.name.eq_ignore_ascii_case("doc_crane_cab");
    let hook_object_required = is_top || entity.position[1] > 0.0;
    let hook_current_position = calc_hook_coordinates(entity.position, hook_angle, DEFAULT_OFFSET);

    CCraneRuntime {
        crane_entity_index,
        crane_entity_pool_index: Some(crane_entity_index as i32),
        hook_object_pool_index: None,
        audio_entity: 0,
        pickup_x1: 0.0,
        pickup_x2: 0.0,
        pickup_y1: 0.0,
        pickup_y2: 0.0,
        dropoff_target: CVector::ZERO,
        dropoff_heading: 0.0,
        pickup_angle: 0.0,
        dropoff_angle: 0.0,
        pickup_distance: 0.0,
        dropoff_distance: 0.0,
        pickup_height: 0.0,
        dropoff_height: 0.0,
        crane_status: CCraneStatus::None,
        crane_state: CCraneState::Idle,
        hook_angle,
        hook_offset: DEFAULT_OFFSET,
        hook_height: DEFAULT_OFFSET,
        hook_init_position: CVector::new(
            hook_current_position[0],
            hook_current_position[1],
            hook_current_position[2],
        ),
        hook_current_position,
        hook_velocity: [0.0; 2],
        vehicle_picked_up_pool_index: None,
        time_for_next_check: 0,
        vehicles_collected: 0,
        is_crusher: false,
        is_military_crane: false,
        was_military_crane: false,
        is_top,
        hook_object_required,
        audio_entity_requested: true,
    }
}

fn calc_hook_coordinates(position: [f32; 3], hook_angle: f32, hook_offset: f32) -> [f32; 3] {
    [
        hook_angle.cos() * hook_offset + position[0],
        hook_angle.sin() * hook_offset + position[1],
        DEFAULT_OFFSET,
    ]
}

fn index_plus_one(index: Option<i32>) -> i32 {
    index.map(|index| index + 1).unwrap_or(0)
}

fn pool_index_from_saved(saved: i32) -> Option<i32> {
    (saved != 0).then_some(saved - 1)
}

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}
