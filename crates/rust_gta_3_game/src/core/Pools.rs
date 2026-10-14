//! re3 reference: references/re-GTA/src/core/Pools.cpp
//! re3 reference: references/re-GTA/src/core/Pools.h
//! re3 reference: references/re-GTA/src/core/templates.h
//!
//! Pool handle and allocation state ported from re3 `CPool`.

use bevy::prelude::Resource;
use rust_gta_3_core::core::Coordinates::CVector;

use crate::entities::Entity::CEntityFlags;
use crate::save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter};

pub const POOLFLAG_ID: u8 = 0x7f;
pub const POOLFLAG_ISFREE: u8 = 0x80;

pub const NUMPTRNODES: usize = 30000;
pub const NUMENTRYINFOS: usize = 5400;
pub const NUMPEDS: usize = 140;
pub const NUMVEHICLES: usize = 110;
pub const NUMBUILDINGS: usize = 5500;
pub const NUMTREADABLES: usize = 1214;
pub const NUMOBJECTS: usize = 450;
pub const NUMDUMMIES: usize = 2802;
pub const NUMAUDIOSCRIPTOBJECTS: usize = 256;
pub const VEHICLE_TYPE_CAR: u32 = 0;
pub const VEHICLE_TYPE_BOAT: u32 = 1;
pub const COMPATIBLE_AUTOMOBILE_SAVE_SIZE: usize = 1448;
pub const COMPATIBLE_BOAT_SAVE_SIZE: usize = 1156;
pub const COMPATIBLE_VEHICLE_BASE_SAVE_SIZE: usize = 648;
pub const COMPATIBLE_AUTOMOBILE_TRAILING_SAVE_SIZE: usize = 800;
pub const COMPATIBLE_BOAT_TRAILING_SAVE_SIZE: usize = 508;
pub const COMPATIBLE_DAMAGE_MANAGER_SAVE_SIZE: usize = 0x1C;
pub const COMPATIBLE_PLAYER_PED_SAVE_SIZE: usize = 1520;
pub const COMPATIBLE_PED_BASE_SAVE_SIZE: usize = 1364;
pub const COMPATIBLE_WEAPON_SAVE_SIZE: usize = 24;
pub const COMPATIBLE_PED_WEAPON_COUNT: usize = 13;
pub const MAX_MODEL_NAME: usize = 24;
pub const COMPATIBLE_OBJECT_SAVE_RECORD_SIZE: usize = 84;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CPoolHandle(pub i32);

impl CPoolHandle {
    pub fn index(self) -> usize {
        (self.0 >> 8) as usize
    }

    pub fn flag(self) -> u8 {
        (self.0 & 0xff) as u8
    }
}

#[derive(Debug, Clone)]
pub struct CPool<T> {
    entries: Vec<Option<T>>,
    flags: Vec<u8>,
    alloc_ptr: usize,
}

impl<T> CPool<T> {
    pub fn new(size: usize) -> Self {
        Self {
            entries: (0..size).map(|_| None).collect(),
            flags: vec![POOLFLAG_ISFREE; size],
            alloc_ptr: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        self.entries.len()
    }

    pub fn get_id(&self, index: usize) -> u8 {
        self.flags[index] & POOLFLAG_ID
    }

    pub fn get_is_free(&self, index: usize) -> bool {
        self.flags[index] & POOLFLAG_ISFREE != 0
    }

    pub fn set_id(&mut self, index: usize, id: u8) {
        self.flags[index] = (self.flags[index] & POOLFLAG_ISFREE) | (id & POOLFLAG_ID);
    }

    pub fn set_is_free(&mut self, index: usize, is_free: bool) {
        if is_free {
            self.flags[index] |= POOLFLAG_ISFREE;
        } else {
            self.flags[index] &= !POOLFLAG_ISFREE;
        }
    }

    pub fn new_entry(&mut self, value: T) -> Option<CPoolHandle> {
        let size = self.get_size();
        if size == 0 {
            return None;
        }

        let mut wrapped = false;
        loop {
            self.alloc_ptr += 1;
            if self.alloc_ptr == size {
                if wrapped {
                    return None;
                }
                wrapped = true;
                self.alloc_ptr = 0;
            }

            if self.get_is_free(self.alloc_ptr) {
                break;
            }
        }

        let index = self.alloc_ptr;
        self.set_is_free(index, false);
        self.set_id(index, self.get_id(index).wrapping_add(1));
        self.entries[index] = Some(value);
        Some(self.get_index_from_slot(index))
    }

    pub fn new_at(&mut self, handle: CPoolHandle, value: T) -> Option<usize> {
        let index = handle.index();
        if index >= self.get_size() {
            return None;
        }

        self.entries[index] = Some(value);
        self.set_not_free_at(handle);
        Some(index)
    }

    pub fn set_not_free_at(&mut self, handle: CPoolHandle) {
        let index = handle.index();
        self.set_is_free(index, false);
        self.set_id(index, handle.flag() & POOLFLAG_ID);

        self.alloc_ptr = 0;
        while self.alloc_ptr < self.get_size() {
            if self.get_is_free(self.alloc_ptr) {
                return;
            }
            self.alloc_ptr += 1;
        }
    }

    pub fn delete_by_slot(&mut self, index: usize) -> Option<T> {
        if index >= self.get_size() || self.get_is_free(index) {
            return None;
        }

        self.set_is_free(index, true);
        if index < self.alloc_ptr {
            self.alloc_ptr = index;
        }
        self.entries[index].take()
    }

    pub fn get_slot(&self, index: usize) -> Option<&T> {
        if index >= self.get_size() || self.get_is_free(index) {
            return None;
        }
        self.entries[index].as_ref()
    }

    pub fn get_slot_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.get_size() || self.get_is_free(index) {
            return None;
        }
        self.entries[index].as_mut()
    }

    pub fn get_at(&self, handle: CPoolHandle) -> Option<&T> {
        let index = handle.index();
        if index >= self.get_size() {
            return None;
        }

        if self.flags[index] == handle.flag() {
            self.entries[index].as_ref()
        } else {
            None
        }
    }

    pub fn get_at_mut(&mut self, handle: CPoolHandle) -> Option<&mut T> {
        let index = handle.index();
        if index >= self.get_size() || self.flags[index] != handle.flag() {
            return None;
        }

        self.entries[index].as_mut()
    }

    pub fn get_index_from_slot(&self, index: usize) -> CPoolHandle {
        CPoolHandle(i32::from(self.flags[index]) + ((index as i32) << 8))
    }

    pub fn get_no_of_used_spaces(&self) -> usize {
        self.flags
            .iter()
            .filter(|flag| **flag & POOLFLAG_ISFREE == 0)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CPoolEntryKind {
    PtrNode,
    EntryInfoNode,
    Ped,
    Vehicle,
    Building,
    Treadable,
    Object,
    Dummy,
    AudioScriptObject,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CPoolEntry {
    pub kind: CPoolEntryKind,
    pub compatible_save: Option<CompatiblePoolSave>,
}

impl CPoolEntry {
    pub fn new(kind: CPoolEntryKind) -> Self {
        Self {
            kind,
            compatible_save: None,
        }
    }

    pub fn with_compatible_save(kind: CPoolEntryKind, compatible_save: CompatiblePoolSave) -> Self {
        Self {
            kind,
            compatible_save: Some(compatible_save),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompatiblePoolSave {
    Ped(CompatiblePedSave),
    Vehicle(CompatibleVehicleSave),
    Object(CompatibleObjectSave),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatiblePedSave {
    pub ped_type: u32,
    pub model_index: i16,
    pub handle: CPoolHandle,
    pub payload: Vec<u8>,
    pub base: CompatiblePedBaseSave,
    pub player: CompatiblePlayerPedSave,
    pub maximum_wanted_level: i32,
    pub n_maximum_wanted_level: i32,
    pub model_name: [u8; MAX_MODEL_NAME],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatiblePedBaseSave {
    pub position: CVector,
    pub char_created_by: u8,
    pub health: f32,
    pub armour: f32,
    pub weapons: [CompatibleWeaponSave; COMPATIBLE_PED_WEAPON_COUNT],
    pub max_weapon_type_allowed: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatibleWeaponSave {
    pub weapon_type: i32,
    pub weapon_state: i32,
    pub ammo_in_clip: i32,
    pub ammo_total: i32,
    pub timer: u32,
    pub add_rot_offset: bool,
}

impl Default for CompatibleWeaponSave {
    fn default() -> Self {
        Self {
            weapon_type: 0,
            weapon_state: 0,
            ammo_in_clip: 0,
            ammo_total: 0,
            timer: 0,
            add_rot_offset: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompatiblePlayerPedSave {
    pub max_stamina: f32,
    pub targettable_objects: [i32; 4],
}

/// Native compatible payload shared by live saves and pool record codecs.
/// re3 `CPed::Save`, `CPlayerPed::Save`, and `CWeapon::Save`.
pub fn encode_compatible_player_ped(
    base: &CompatiblePedBaseSave,
    player: &CompatiblePlayerPedSave,
) -> Vec<u8> {
    let mut writer = SaveBufWriter::new();
    writer.zero(52);
    writer.write(base.position.x);
    writer.write(base.position.y);
    writer.write(base.position.z);
    writer.zero(288);
    writer.write(base.char_created_by);
    writer.zero(351);
    writer.write(base.health);
    writer.write(base.armour);
    writer.zero(148);
    for weapon in &base.weapons {
        writer.write(weapon.weapon_type);
        writer.write(weapon.weapon_state);
        writer.write(weapon.ammo_in_clip);
        writer.write(weapon.ammo_total);
        writer.write(weapon.timer);
        writer.write(u8::from(weapon.add_rot_offset));
        writer.zero(3);
    }
    writer.zero(5);
    writer.write(base.max_weapon_type_allowed);
    writer.zero(162);
    writer.zero(16);
    writer.write(player.max_stamina);
    writer.zero(28);
    for object in player.targettable_objects {
        writer.write(object);
    }
    writer.zero(116);
    debug_assert_eq!(writer.offset(), COMPATIBLE_PLAYER_PED_SAVE_SIZE);
    writer.into_inner()
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleVehicleSave {
    pub vehicle_type: u32,
    pub model_index: i16,
    pub handle: CPoolHandle,
    pub payload: Vec<u8>,
    pub base: CompatibleVehicleBaseSave,
    pub specific: CompatibleVehicleSpecificSave,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompatibleVehicleSpecificSave {
    Automobile {
        damage: CompatibleDamageManagerSave,
        trailing_padding_len: usize,
    },
    Boat {
        trailing_padding_len: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompatibleDamageManagerSave {
    pub wheel_damage_effect: f32,
    pub engine_status: u8,
    pub wheel_status: [u8; 4],
    pub door_status: [u8; 6],
    pub light_status: u32,
    pub panel_status: u32,
    pub field_18: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleVehicleBaseSave {
    pub matrix: CompatibleVehicleMatrix,
    pub entity_flags: CompatibleEntityFlags,
    pub autopilot: CompatibleAutoPilotSave,
    pub current_colour1: u8,
    pub current_colour2: u8,
    pub alarm_state: i16,
    pub num_max_passengers: u8,
    pub field_1d0: [f32; 4],
    pub steer_angle: f32,
    pub gas_pedal: f32,
    pub brake_pedal: f32,
    pub vehicle_created_by: u8,
    pub is_law_enforcer: bool,
    pub is_locked: bool,
    pub engine_on: bool,
    pub is_handbrake_on: bool,
    pub lights_on: bool,
    pub freebies: bool,
    pub health: f32,
    pub current_gear: u8,
    pub change_gear_time: f32,
    pub time_of_death: u32,
    pub bomb_timer: i16,
    pub door_lock: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompatibleVehicleMatrix {
    pub right: CVector,
    pub forward: CVector,
    pub up: CVector,
    pub position: CVector,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompatibleAutoPilotSave {
    pub current_route_node: i32,
    pub next_route_node: i32,
    pub prev_route_node: i32,
    pub time_entered_curve: i32,
    pub time_to_spend_on_current_curve: i32,
    pub current_path_node_info: u32,
    pub next_path_node_info: u32,
    pub previous_path_node_info: u32,
    pub anti_reverse_timer: u32,
    pub time_to_start_mission: u32,
    pub previous_direction: i8,
    pub current_direction: i8,
    pub next_direction: i8,
    pub current_lane: i8,
    pub next_lane: i8,
    pub driving_style: u8,
    pub car_mission: u8,
    pub temp_action: u8,
    pub time_temp_action: u32,
    pub max_traffic_speed: f32,
    pub cruise_speed: u8,
    pub slowed_down_because_of_cars: bool,
    pub slowed_down_because_of_peds: bool,
    pub stay_in_current_level: bool,
    pub stay_in_fast_lane: bool,
    pub ignore_pathfinding: bool,
    pub destination_coors: CVector,
    pub path_find_nodes_count: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibleObjectSave {
    pub model_index: i16,
    pub handle: CPoolHandle,
    pub payload: Vec<u8>,
    pub matrix: CompatibleCompressedMatrix,
    pub uproot_limit: f32,
    pub object_matrix: CompatibleCompressedMatrix,
    pub object_created_by: i8,
    pub is_pickup: bool,
    pub pickup_obj_with_message: bool,
    pub out_of_stock: bool,
    pub glass_cracked: bool,
    pub glass_broken: bool,
    pub has_been_damaged: bool,
    pub use_vehicle_colours: bool,
    pub collision_damage_multiplier: f32,
    pub collision_damage_effect: i8,
    pub special_collision_response_cases: i8,
    pub end_of_life_time: u32,
    pub entity_flags: CompatibleEntityFlags,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompatibleCompressedMatrix {
    pub position: CVector,
    pub right: [i8; 3],
    pub up: [i8; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatibleEntityFlags {
    pub word1: u32,
    pub word2: u32,
    pub entity_type_bits: u8,
    pub status_bits: u8,
    pub flags: CEntityFlags,
}

pub type CCPtrNodePool = CPool<CPoolEntry>;
pub type CEntryInfoNodePool = CPool<CPoolEntry>;
pub type CPedPool = CPool<CPoolEntry>;
pub type CVehiclePool = CPool<CPoolEntry>;
pub type CBuildingPool = CPool<CPoolEntry>;
pub type CTreadablePool = CPool<CPoolEntry>;
pub type CObjectPool = CPool<CPoolEntry>;
pub type CDummyPool = CPool<CPoolEntry>;
pub type CAudioScriptObjectPool = CPool<CPoolEntry>;

#[derive(Debug, Clone, Resource)]
pub struct CPools {
    pub ptr_node_pool: CCPtrNodePool,
    pub entry_info_node_pool: CEntryInfoNodePool,
    pub ped_pool: CPedPool,
    pub vehicle_pool: CVehiclePool,
    pub building_pool: CBuildingPool,
    pub treadable_pool: CTreadablePool,
    pub object_pool: CObjectPool,
    pub dummy_pool: CDummyPool,
    pub audio_script_object_pool: CAudioScriptObjectPool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoolRuntimeRestorePlan {
    pub peds: Vec<PoolPedRestore>,
    pub vehicles: Vec<PoolVehicleRestore>,
    pub objects: Vec<PoolObjectRestore>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoolPedRestore {
    pub slot: usize,
    pub handle: CPoolHandle,
    pub record: CompatiblePedSave,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoolVehicleRestore {
    pub slot: usize,
    pub handle: CPoolHandle,
    pub record: CompatibleVehicleSave,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoolObjectRestore {
    pub slot: usize,
    pub handle: CPoolHandle,
    pub record: CompatibleObjectSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CObjectPoolSlotAction {
    AlreadyFree,
    RemovedTemporaryObject,
    RemovedProjectile,
    RelocatedTo(CPoolHandle),
    NoFreeSlot,
}

impl CPools {
    pub fn initialise() -> Self {
        Self {
            ptr_node_pool: CPool::new(NUMPTRNODES),
            entry_info_node_pool: CPool::new(NUMENTRYINFOS),
            ped_pool: CPool::new(NUMPEDS),
            vehicle_pool: CPool::new(NUMVEHICLES),
            building_pool: CPool::new(NUMBUILDINGS),
            treadable_pool: CPool::new(NUMTREADABLES),
            object_pool: CPool::new(NUMOBJECTS),
            dummy_pool: CPool::new(NUMDUMMIES),
            audio_script_object_pool: CPool::new(NUMAUDIOSCRIPTOBJECTS),
        }
    }

    pub fn get_ped_ref(&self, slot: usize) -> Option<CPoolHandle> {
        self.ped_pool
            .get_slot(slot)
            .map(|_| self.ped_pool.get_index_from_slot(slot))
    }

    pub fn get_ped(&self, handle: CPoolHandle) -> Option<&CPoolEntry> {
        self.ped_pool.get_at(handle)
    }

    pub fn get_vehicle_ref(&self, slot: usize) -> Option<CPoolHandle> {
        self.vehicle_pool
            .get_slot(slot)
            .map(|_| self.vehicle_pool.get_index_from_slot(slot))
    }

    pub fn get_vehicle(&self, handle: CPoolHandle) -> Option<&CPoolEntry> {
        self.vehicle_pool.get_at(handle)
    }

    pub fn get_object_ref(&self, slot: usize) -> Option<CPoolHandle> {
        self.object_pool
            .get_slot(slot)
            .map(|_| self.object_pool.get_index_from_slot(slot))
    }

    pub fn get_object(&self, handle: CPoolHandle) -> Option<&CPoolEntry> {
        self.object_pool.get_at(handle)
    }

    pub fn compatible_ped_records(&self) -> impl Iterator<Item = (usize, &CompatiblePedSave)> {
        self.ped_pool
            .entries
            .iter()
            .enumerate()
            .filter_map(
                |(slot, entry)| match entry.as_ref()?.compatible_save.as_ref()? {
                    CompatiblePoolSave::Ped(record) => Some((slot, record)),
                    _ => None,
                },
            )
    }

    pub fn compatible_vehicle_records(
        &self,
    ) -> impl Iterator<Item = (usize, &CompatibleVehicleSave)> {
        self.vehicle_pool
            .entries
            .iter()
            .enumerate()
            .filter_map(
                |(slot, entry)| match entry.as_ref()?.compatible_save.as_ref()? {
                    CompatiblePoolSave::Vehicle(record) => Some((slot, record)),
                    _ => None,
                },
            )
    }

    pub fn compatible_object_records(
        &self,
    ) -> impl Iterator<Item = (usize, &CompatibleObjectSave)> {
        self.object_pool
            .entries
            .iter()
            .enumerate()
            .filter_map(
                |(slot, entry)| match entry.as_ref()?.compatible_save.as_ref()? {
                    CompatiblePoolSave::Object(record) => Some((slot, record)),
                    _ => None,
                },
            )
    }

    pub fn build_runtime_restore_plan(&self) -> PoolRuntimeRestorePlan {
        PoolRuntimeRestorePlan {
            peds: self
                .ped_pool
                .entries
                .iter()
                .enumerate()
                .filter_map(
                    |(slot, entry)| match entry.as_ref()?.compatible_save.as_ref()? {
                        CompatiblePoolSave::Ped(record) => Some(PoolPedRestore {
                            slot,
                            handle: record.handle,
                            record: record.clone(),
                        }),
                        _ => None,
                    },
                )
                .collect(),
            vehicles: self
                .vehicle_pool
                .entries
                .iter()
                .enumerate()
                .filter_map(
                    |(slot, entry)| match entry.as_ref()?.compatible_save.as_ref()? {
                        CompatiblePoolSave::Vehicle(record) => Some(PoolVehicleRestore {
                            slot,
                            handle: record.handle,
                            record: record.clone(),
                        }),
                        _ => None,
                    },
                )
                .collect(),
            objects: self
                .object_pool
                .entries
                .iter()
                .enumerate()
                .filter_map(
                    |(slot, entry)| match entry.as_ref()?.compatible_save.as_ref()? {
                        CompatiblePoolSave::Object(record) => Some(PoolObjectRestore {
                            slot,
                            handle: record.handle,
                            record: record.clone(),
                        }),
                        _ => None,
                    },
                )
                .collect(),
        }
    }

    pub fn check_pools_empty(&self) -> bool {
        self.ped_pool.get_no_of_used_spaces() == 0 && self.vehicle_pool.get_no_of_used_spaces() == 0
    }

    pub fn make_sure_slot_in_object_pool_is_empty(
        &mut self,
        slot: usize,
        is_temp_object: bool,
        is_projectile: bool,
    ) -> CObjectPoolSlotAction {
        if slot >= self.object_pool.get_size() || self.object_pool.get_is_free(slot) {
            return CObjectPoolSlotAction::AlreadyFree;
        }

        if is_temp_object {
            self.object_pool.delete_by_slot(slot);
            return CObjectPoolSlotAction::RemovedTemporaryObject;
        }

        if is_projectile {
            self.object_pool.delete_by_slot(slot);
            return CObjectPoolSlotAction::RemovedProjectile;
        }

        let Some(object) = self.object_pool.get_slot(slot).cloned() else {
            return CObjectPoolSlotAction::AlreadyFree;
        };

        match self.object_pool.new_entry(object) {
            Some(handle) => {
                self.object_pool.delete_by_slot(slot);
                CObjectPoolSlotAction::RelocatedTo(handle)
            }
            None => CObjectPoolSlotAction::NoFreeSlot,
        }
    }

    pub fn save_ped_pool_compatible(&self) -> Vec<u8> {
        let records: Vec<&CompatiblePedSave> = self
            .ped_pool
            .entries
            .iter()
            .filter_map(|entry| match entry.as_ref()?.compatible_save.as_ref()? {
                CompatiblePoolSave::Ped(record) => Some(record),
                _ => None,
            })
            .collect();
        let mut writer = SaveBufWriter::new();
        writer.write(records.len() as i32);
        for record in records {
            writer.write(record.ped_type);
            writer.write(record.model_index);
            writer.write(record.handle.0);
            writer.write_bytes(&record.payload);
            writer.write(record.maximum_wanted_level);
            writer.write(record.n_maximum_wanted_level);
            writer.write_bytes(&record.model_name);
        }
        writer.into_inner()
    }

    pub fn save_vehicle_pool_compatible(&self) -> Vec<u8> {
        let records: Vec<&CompatibleVehicleSave> = self
            .vehicle_pool
            .entries
            .iter()
            .filter_map(|entry| match entry.as_ref()?.compatible_save.as_ref()? {
                CompatiblePoolSave::Vehicle(record) => Some(record),
                _ => None,
            })
            .collect();
        let cars = records
            .iter()
            .filter(|record| record.vehicle_type == VEHICLE_TYPE_CAR)
            .count();
        let boats = records
            .iter()
            .filter(|record| record.vehicle_type == VEHICLE_TYPE_BOAT)
            .count();
        let mut writer = SaveBufWriter::new();
        writer.write(cars as i32);
        writer.write(boats as i32);
        for record in records {
            writer.write(record.vehicle_type);
            writer.write(record.model_index);
            writer.write(record.handle.0);
            writer.write_bytes(&record.payload);
        }
        writer.into_inner()
    }

    pub fn save_object_pool_compatible(&self) -> Vec<u8> {
        let records: Vec<&CompatibleObjectSave> = self
            .object_pool
            .entries
            .iter()
            .filter_map(|entry| match entry.as_ref()?.compatible_save.as_ref()? {
                CompatiblePoolSave::Object(record) => Some(record),
                _ => None,
            })
            .collect();
        let mut writer = SaveBufWriter::new();
        writer.write(records.len() as i32);
        for record in records {
            writer.write(record.model_index);
            writer.write(record.handle.0);
            writer.write_bytes(&record.payload);
        }
        writer.into_inner()
    }

    pub fn load_ped_pool_compatible(&mut self, bytes: &[u8]) -> Result<(), SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let count = reader.read::<i32>()?;
        if count < 0 || count as usize > NUMPEDS {
            return Err(pool_count_error(reader.offset(), count, bytes.len()));
        }
        let mut ped_pool = CPool::new(NUMPEDS);
        for _ in 0..count {
            let ped_type = reader.read::<u32>()?;
            let model_index = reader.read::<i16>()?;
            let handle = CPoolHandle(reader.read::<i32>()?);
            validate_saved_pool_handle(&ped_pool, handle, reader.offset(), bytes.len())?;
            let payload = reader.read_bytes(COMPATIBLE_PLAYER_PED_SAVE_SIZE)?.to_vec();
            let (base, player) = parse_compatible_player_ped_save(&payload)?;
            let maximum_wanted_level = reader.read::<i32>()?;
            let n_maximum_wanted_level = reader.read::<i32>()?;
            let mut model_name = [0_u8; MAX_MODEL_NAME];
            model_name.copy_from_slice(reader.read_bytes(MAX_MODEL_NAME)?);
            ped_pool.new_at(
                handle,
                CPoolEntry::with_compatible_save(
                    CPoolEntryKind::Ped,
                    CompatiblePoolSave::Ped(CompatiblePedSave {
                        ped_type,
                        model_index,
                        handle,
                        payload,
                        base,
                        player,
                        maximum_wanted_level,
                        n_maximum_wanted_level,
                        model_name,
                    }),
                ),
            );
        }
        validate_pool_reader_finished(&reader, bytes)?;
        self.ped_pool = ped_pool;
        Ok(())
    }

    pub fn load_vehicle_pool_compatible(&mut self, bytes: &[u8]) -> Result<(), SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let cars = reader.read::<i32>()?;
        let boats = reader.read::<i32>()?;
        if cars < 0 || boats < 0 || cars.saturating_add(boats) as usize > NUMVEHICLES {
            return Err(pool_count_error(
                reader.offset(),
                cars.saturating_add(boats),
                bytes.len(),
            ));
        }
        let mut vehicle_pool = CPool::new(NUMVEHICLES);
        for _ in 0..cars.saturating_add(boats) {
            let vehicle_type = reader.read::<u32>()?;
            let model_index = reader.read::<i16>()?;
            let handle = CPoolHandle(reader.read::<i32>()?);
            validate_saved_pool_handle(&vehicle_pool, handle, reader.offset(), bytes.len())?;
            let payload_size = compatible_vehicle_payload_size(vehicle_type)?;
            let payload = reader.read_bytes(payload_size)?.to_vec();
            let base = parse_compatible_vehicle_base_save(&payload)?;
            let specific = parse_compatible_vehicle_specific_save(vehicle_type, &payload)?;
            vehicle_pool.new_at(
                handle,
                CPoolEntry::with_compatible_save(
                    CPoolEntryKind::Vehicle,
                    CompatiblePoolSave::Vehicle(CompatibleVehicleSave {
                        vehicle_type,
                        model_index,
                        handle,
                        payload,
                        base,
                        specific,
                    }),
                ),
            );
        }
        validate_pool_reader_finished(&reader, bytes)?;
        self.vehicle_pool = vehicle_pool;
        Ok(())
    }

    pub fn load_object_pool_compatible(&mut self, bytes: &[u8]) -> Result<(), SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let count = reader.read::<i32>()?;
        if count < 0 || count as usize > NUMOBJECTS {
            return Err(pool_count_error(reader.offset(), count, bytes.len()));
        }
        let mut object_pool = CPool::new(NUMOBJECTS);
        for _ in 0..count {
            let model_index = reader.read::<i16>()?;
            let handle = CPoolHandle(reader.read::<i32>()?);
            validate_saved_pool_handle(&object_pool, handle, reader.offset(), bytes.len())?;
            let payload = reader
                .read_bytes(COMPATIBLE_OBJECT_SAVE_RECORD_SIZE - 6)?
                .to_vec();
            let record = parse_compatible_object_save(model_index, handle, payload)?;
            object_pool.new_at(
                handle,
                CPoolEntry::with_compatible_save(
                    CPoolEntryKind::Object,
                    CompatiblePoolSave::Object(record),
                ),
            );
        }
        validate_pool_reader_finished(&reader, bytes)?;
        self.object_pool = object_pool;
        Ok(())
    }

    pub fn save_ped_pool_empty_compatible(&self) -> Vec<u8> {
        self.save_ped_pool_compatible()
    }

    pub fn save_vehicle_pool_empty_compatible(&self) -> Vec<u8> {
        self.save_vehicle_pool_compatible()
    }

    pub fn save_object_pool_empty_compatible(&self) -> Vec<u8> {
        self.save_object_pool_compatible()
    }

    pub fn load_ped_pool_empty_compatible(&mut self, bytes: &[u8]) -> Result<(), SaveBufError> {
        self.load_ped_pool_compatible(bytes)
    }

    pub fn load_vehicle_pool_empty_compatible(&mut self, bytes: &[u8]) -> Result<(), SaveBufError> {
        self.load_vehicle_pool_compatible(bytes)
    }

    pub fn load_object_pool_empty_compatible(&mut self, bytes: &[u8]) -> Result<(), SaveBufError> {
        self.load_object_pool_compatible(bytes)
    }
}

fn compatible_vehicle_payload_size(vehicle_type: u32) -> Result<usize, SaveBufError> {
    match vehicle_type {
        VEHICLE_TYPE_CAR => Ok(COMPATIBLE_AUTOMOBILE_SAVE_SIZE),
        VEHICLE_TYPE_BOAT => Ok(COMPATIBLE_BOAT_SAVE_SIZE),
        _ => Err(SaveBufError::ReadPastEnd {
            offset: 0,
            len: vehicle_type as usize,
            available: 2,
        }),
    }
}

fn parse_compatible_player_ped_save(
    payload: &[u8],
) -> Result<(CompatiblePedBaseSave, CompatiblePlayerPedSave), SaveBufError> {
    let mut reader = SaveBufReader::new(payload);
    let base = read_compatible_ped_base(&mut reader)?;
    reader.skip(16)?;
    let player = CompatiblePlayerPedSave {
        max_stamina: reader.read::<f32>()?,
        targettable_objects: {
            reader.skip(28)?;
            [
                reader.read::<i32>()?,
                reader.read::<i32>()?,
                reader.read::<i32>()?,
                reader.read::<i32>()?,
            ]
        },
    };
    reader.skip(116)?;
    validate_pool_reader_finished(&reader, payload)?;
    Ok((base, player))
}

fn read_compatible_ped_base(
    reader: &mut SaveBufReader<'_>,
) -> Result<CompatiblePedBaseSave, SaveBufError> {
    reader.skip(52)?;
    let position = read_vector3(reader)?;
    reader.skip(288)?;
    let char_created_by = reader.read::<u8>()?;
    reader.skip(351)?;
    let health = reader.read::<f32>()?;
    let armour = reader.read::<f32>()?;
    reader.skip(148)?;
    let mut weapons = [CompatibleWeaponSave::default(); COMPATIBLE_PED_WEAPON_COUNT];
    for weapon in &mut weapons {
        *weapon = read_compatible_weapon(reader)?;
    }
    reader.skip(5)?;
    let max_weapon_type_allowed = reader.read::<i32>()?;
    reader.skip(162)?;

    Ok(CompatiblePedBaseSave {
        position,
        char_created_by,
        health,
        armour,
        weapons,
        max_weapon_type_allowed,
    })
}

fn read_compatible_weapon(
    reader: &mut SaveBufReader<'_>,
) -> Result<CompatibleWeaponSave, SaveBufError> {
    let weapon = CompatibleWeaponSave {
        weapon_type: reader.read::<i32>()?,
        weapon_state: reader.read::<i32>()?,
        ammo_in_clip: reader.read::<i32>()?,
        ammo_total: reader.read::<i32>()?,
        timer: reader.read::<u32>()?,
        add_rot_offset: reader.read::<u8>()? != 0,
    };
    reader.skip(3)?;
    Ok(weapon)
}

fn parse_compatible_vehicle_base_save(
    payload: &[u8],
) -> Result<CompatibleVehicleBaseSave, SaveBufError> {
    let mut reader = SaveBufReader::new(payload);
    reader.skip(4)?;
    let right = read_vector3(&mut reader)?;
    reader.skip(4)?;
    let forward = read_vector3(&mut reader)?;
    reader.skip(4)?;
    let up = read_vector3(&mut reader)?;
    reader.skip(4)?;
    let position = read_vector3(&mut reader)?;
    reader.skip(16)?;
    let entity_flags = decode_compatible_entity_flags(reader.read::<u32>()?, reader.read::<u32>()?);
    reader.skip(212)?;
    let autopilot = read_compatible_autopilot(&mut reader)?;
    let current_colour1 = reader.read::<u8>()?;
    let current_colour2 = reader.read::<u8>()?;
    reader.skip(2)?;
    let alarm_state = reader.read::<i16>()?;
    reader.skip(43)?;
    let num_max_passengers = reader.read::<u8>()?;
    reader.skip(2)?;
    let field_1d0 = [
        reader.read::<f32>()?,
        reader.read::<f32>()?,
        reader.read::<f32>()?,
        reader.read::<f32>()?,
    ];
    reader.skip(8)?;
    let steer_angle = reader.read::<f32>()?;
    let gas_pedal = reader.read::<f32>()?;
    let brake_pedal = reader.read::<f32>()?;
    let vehicle_created_by = reader.read::<u8>()?;
    let vehicle_flags = reader.read::<u8>()?;
    reader.skip(10)?;
    let health = reader.read::<f32>()?;
    let current_gear = reader.read::<u8>()?;
    reader.skip(3)?;
    let change_gear_time = reader.read::<f32>()?;
    reader.skip(4)?;
    let time_of_death = reader.read::<u32>()?;
    reader.skip(2)?;
    let bomb_timer = reader.read::<i16>()?;
    reader.skip(12)?;
    let door_lock = reader.read::<i32>()?;
    reader.skip(96)?;
    if reader.offset() != COMPATIBLE_VEHICLE_BASE_SAVE_SIZE {
        return Err(SaveBufError::ReadPastEnd {
            offset: reader.offset(),
            len: COMPATIBLE_VEHICLE_BASE_SAVE_SIZE,
            available: payload.len(),
        });
    }

    Ok(CompatibleVehicleBaseSave {
        matrix: CompatibleVehicleMatrix {
            right,
            forward,
            up,
            position,
        },
        entity_flags,
        autopilot,
        current_colour1,
        current_colour2,
        alarm_state,
        num_max_passengers,
        field_1d0,
        steer_angle,
        gas_pedal,
        brake_pedal,
        vehicle_created_by,
        is_law_enforcer: bit(vehicle_flags as u32, 0),
        is_locked: bit(vehicle_flags as u32, 3),
        engine_on: bit(vehicle_flags as u32, 4),
        is_handbrake_on: bit(vehicle_flags as u32, 5),
        lights_on: bit(vehicle_flags as u32, 6),
        freebies: bit(vehicle_flags as u32, 7),
        health,
        current_gear,
        change_gear_time,
        time_of_death,
        bomb_timer,
        door_lock,
    })
}

fn parse_compatible_vehicle_specific_save(
    vehicle_type: u32,
    payload: &[u8],
) -> Result<CompatibleVehicleSpecificSave, SaveBufError> {
    match vehicle_type {
        VEHICLE_TYPE_CAR => {
            if payload.len() != COMPATIBLE_AUTOMOBILE_SAVE_SIZE {
                return Err(SaveBufError::ReadPastEnd {
                    offset: COMPATIBLE_VEHICLE_BASE_SAVE_SIZE,
                    len: COMPATIBLE_AUTOMOBILE_SAVE_SIZE,
                    available: payload.len(),
                });
            }
            let mut reader = SaveBufReader::new(&payload[COMPATIBLE_VEHICLE_BASE_SAVE_SIZE..]);
            let damage = read_compatible_damage_manager(&mut reader)?;
            reader.skip(
                COMPATIBLE_AUTOMOBILE_TRAILING_SAVE_SIZE - COMPATIBLE_DAMAGE_MANAGER_SAVE_SIZE,
            )?;
            validate_pool_reader_finished(&reader, &payload[COMPATIBLE_VEHICLE_BASE_SAVE_SIZE..])?;
            Ok(CompatibleVehicleSpecificSave::Automobile {
                damage,
                trailing_padding_len: COMPATIBLE_AUTOMOBILE_TRAILING_SAVE_SIZE
                    - COMPATIBLE_DAMAGE_MANAGER_SAVE_SIZE,
            })
        }
        VEHICLE_TYPE_BOAT => {
            if payload.len() != COMPATIBLE_BOAT_SAVE_SIZE {
                return Err(SaveBufError::ReadPastEnd {
                    offset: COMPATIBLE_VEHICLE_BASE_SAVE_SIZE,
                    len: COMPATIBLE_BOAT_SAVE_SIZE,
                    available: payload.len(),
                });
            }
            let mut reader = SaveBufReader::new(&payload[COMPATIBLE_VEHICLE_BASE_SAVE_SIZE..]);
            reader.skip(COMPATIBLE_BOAT_TRAILING_SAVE_SIZE)?;
            validate_pool_reader_finished(&reader, &payload[COMPATIBLE_VEHICLE_BASE_SAVE_SIZE..])?;
            Ok(CompatibleVehicleSpecificSave::Boat {
                trailing_padding_len: COMPATIBLE_BOAT_TRAILING_SAVE_SIZE,
            })
        }
        _ => Err(SaveBufError::ReadPastEnd {
            offset: 0,
            len: vehicle_type as usize,
            available: 2,
        }),
    }
}

fn read_compatible_damage_manager(
    reader: &mut SaveBufReader<'_>,
) -> Result<CompatibleDamageManagerSave, SaveBufError> {
    let wheel_damage_effect = reader.read::<f32>()?;
    let engine_status = reader.read::<u8>()?;
    let wheel_status = [
        reader.read::<u8>()?,
        reader.read::<u8>()?,
        reader.read::<u8>()?,
        reader.read::<u8>()?,
    ];
    let door_status = [
        reader.read::<u8>()?,
        reader.read::<u8>()?,
        reader.read::<u8>()?,
        reader.read::<u8>()?,
        reader.read::<u8>()?,
        reader.read::<u8>()?,
    ];
    reader.skip(1)?;
    let light_status = reader.read::<u32>()?;
    let panel_status = reader.read::<u32>()?;
    let field_18 = reader.read::<u8>()?;
    reader.skip(3)?;

    Ok(CompatibleDamageManagerSave {
        wheel_damage_effect,
        engine_status,
        wheel_status,
        door_status,
        light_status,
        panel_status,
        field_18,
    })
}

fn read_compatible_autopilot(
    reader: &mut SaveBufReader<'_>,
) -> Result<CompatibleAutoPilotSave, SaveBufError> {
    let current_route_node = reader.read::<i32>()?;
    let next_route_node = reader.read::<i32>()?;
    let prev_route_node = reader.read::<i32>()?;
    let time_entered_curve = reader.read::<i32>()?;
    let time_to_spend_on_current_curve = reader.read::<i32>()?;
    let current_path_node_info = reader.read::<u32>()?;
    let next_path_node_info = reader.read::<u32>()?;
    let previous_path_node_info = reader.read::<u32>()?;
    let anti_reverse_timer = reader.read::<u32>()?;
    let time_to_start_mission = reader.read::<u32>()?;
    let previous_direction = reader.read::<i8>()?;
    let current_direction = reader.read::<i8>()?;
    let next_direction = reader.read::<i8>()?;
    let current_lane = reader.read::<i8>()?;
    let next_lane = reader.read::<i8>()?;
    let driving_style = reader.read::<u8>()?;
    let car_mission = reader.read::<u8>()?;
    let temp_action = reader.read::<u8>()?;
    let time_temp_action = reader.read::<u32>()?;
    let max_traffic_speed = reader.read::<f32>()?;
    let cruise_speed = reader.read::<u8>()?;
    let flags = reader.read::<u8>()?;
    reader.skip(2)?;
    let destination_coors = read_vector3(reader)?;
    reader.skip(32)?;
    let path_find_nodes_count = reader.read::<i16>()?;
    reader.skip(6)?;

    Ok(CompatibleAutoPilotSave {
        current_route_node,
        next_route_node,
        prev_route_node,
        time_entered_curve,
        time_to_spend_on_current_curve,
        current_path_node_info,
        next_path_node_info,
        previous_path_node_info,
        anti_reverse_timer,
        time_to_start_mission,
        previous_direction,
        current_direction,
        next_direction,
        current_lane,
        next_lane,
        driving_style,
        car_mission,
        temp_action,
        time_temp_action,
        max_traffic_speed,
        cruise_speed,
        slowed_down_because_of_cars: bit(flags as u32, 0),
        slowed_down_because_of_peds: bit(flags as u32, 1),
        stay_in_current_level: bit(flags as u32, 2),
        stay_in_fast_lane: bit(flags as u32, 3),
        ignore_pathfinding: bit(flags as u32, 4),
        destination_coors,
        path_find_nodes_count,
    })
}

fn read_vector3(reader: &mut SaveBufReader<'_>) -> Result<CVector, SaveBufError> {
    Ok(CVector::new(
        reader.read::<f32>()?,
        reader.read::<f32>()?,
        reader.read::<f32>()?,
    ))
}

fn validate_saved_pool_handle<T>(pool: &CPool<T>, handle: CPoolHandle, offset: usize, available: usize) -> Result<(), SaveBufError> {
    if handle.0 < 0 || handle.flag() & POOLFLAG_ISFREE != 0
        || handle.index() >= pool.get_size()
        || !pool.get_is_free(handle.index())
    {
        return Err(pool_count_error(offset, handle.0, available));
    }
    Ok(())
}

fn pool_count_error(offset: usize, count: i32, available: usize) -> SaveBufError {
    SaveBufError::ReadPastEnd {
        offset,
        len: count.unsigned_abs() as usize,
        available,
    }
}

fn validate_pool_reader_finished(
    reader: &SaveBufReader<'_>,
    bytes: &[u8],
) -> Result<(), SaveBufError> {
    if reader.remaining() == 0 {
        Ok(())
    } else {
        Err(SaveBufError::ReadPastEnd {
            offset: reader.offset(),
            len: reader.remaining(),
            available: bytes.len(),
        })
    }
}

fn parse_compatible_object_save(
    model_index: i16,
    handle: CPoolHandle,
    payload: Vec<u8>,
) -> Result<CompatibleObjectSave, SaveBufError> {
    let mut reader = SaveBufReader::new(&payload);
    let matrix = read_compatible_compressed_matrix(&mut reader)?;
    let uproot_limit = reader.read::<f32>()?;
    let object_matrix = read_compatible_compressed_matrix(&mut reader)?;
    let object_created_by = reader.read::<i8>()?;
    let is_pickup = reader.read::<u8>()? != 0;
    let pickup_obj_with_message = reader.read::<u8>()? != 0;
    let out_of_stock = reader.read::<u8>()? != 0;
    let glass_cracked = reader.read::<u8>()? != 0;
    let glass_broken = reader.read::<u8>()? != 0;
    let has_been_damaged = reader.read::<u8>()? != 0;
    let use_vehicle_colours = reader.read::<u8>()? != 0;
    let collision_damage_multiplier = reader.read::<f32>()?;
    let collision_damage_effect = reader.read::<i8>()?;
    let special_collision_response_cases = reader.read::<i8>()?;
    let end_of_life_time = reader.read::<u32>()?;
    let entity_flags = decode_compatible_entity_flags(reader.read::<u32>()?, reader.read::<u32>()?);
    validate_pool_reader_finished(&reader, &payload)?;

    Ok(CompatibleObjectSave {
        model_index,
        handle,
        payload,
        matrix,
        uproot_limit,
        object_matrix,
        object_created_by,
        is_pickup,
        pickup_obj_with_message,
        out_of_stock,
        glass_cracked,
        glass_broken,
        has_been_damaged,
        use_vehicle_colours,
        collision_damage_multiplier,
        collision_damage_effect,
        special_collision_response_cases,
        end_of_life_time,
        entity_flags,
    })
}

fn decode_compatible_entity_flags(word1: u32, word2: u32) -> CompatibleEntityFlags {
    CompatibleEntityFlags {
        word1,
        word2,
        entity_type_bits: (word1 & 0b111) as u8,
        status_bits: ((word1 >> 3) & 0b1_1111) as u8,
        flags: CEntityFlags {
            uses_collision: bit(word1, 8),
            collision_processed: bit(word1, 9),
            is_static: bit(word1, 10),
            has_contacted: bit(word1, 11),
            ped_physics: bit(word1, 12),
            is_stuck: bit(word1, 13),
            is_in_safe_position: bit(word1, 14),
            use_collision_records: bit(word1, 15),
            was_postponed: bit(word1, 16),
            explosion_proof: bit(word1, 17),
            is_visible: bit(word1, 18),
            has_collided: bit(word1, 19),
            render_scorched: bit(word1, 20),
            has_blip: bit(word1, 21),
            is_big_building: bit(word1, 22),
            render_damaged: bit(word1, 23),
            bullet_proof: bit(word1, 24),
            fire_proof: bit(word1, 25),
            collision_proof: bit(word1, 26),
            melee_proof: bit(word1, 27),
            only_damaged_by_player: bit(word1, 28),
            streaming_dont_delete: bit(word1, 29),
            zone_culled: bit(word1, 30),
            zone_culled2: bit(word1, 31),
            remove_from_world: bit(word2, 0),
            has_hit_wall: bit(word2, 1),
            im_being_rendered: bit(word2, 2),
            touching_water: bit(word2, 3),
            is_subway: bit(word2, 4),
            draw_last: bit(word2, 5),
            no_bright_head_lights: bit(word2, 6),
            do_not_render: bit(word2, 7),
            distance_fade: bit(word2, 8),
            flag_e2: bit(word2, 9),
        },
    }
}

fn bit(word: u32, index: u32) -> bool {
    word & (1_u32 << index) != 0
}

fn read_compatible_compressed_matrix(
    reader: &mut SaveBufReader<'_>,
) -> Result<CompatibleCompressedMatrix, SaveBufError> {
    let matrix = CompatibleCompressedMatrix {
        position: CVector::new(
            reader.read::<f32>()?,
            reader.read::<f32>()?,
            reader.read::<f32>()?,
        ),
        right: [
            reader.read::<i8>()?,
            reader.read::<i8>()?,
            reader.read::<i8>()?,
        ],
        up: [
            reader.read::<i8>()?,
            reader.read::<i8>()?,
            reader.read::<i8>()?,
        ],
    };
    reader.skip(6)?;
    Ok(matrix)
}
