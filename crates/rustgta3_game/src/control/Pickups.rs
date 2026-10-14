//! re3 reference: references/re-GTA/src/control/Pickups.cpp
//! re3 reference: references/re-GTA/src/control/Pickups.h
//!
//! `CPickups::Init` and `CPacManPickups::Init` state.

use bevy::prelude::*;
use rustgta3_core::core::Coordinates::CVector;

use crate::save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter};

pub const NUMGENERALPICKUPS: usize = 320;
pub const NUMSCRIPTEDPICKUPS: usize = 16;
pub const NUMPICKUPS: usize = NUMGENERALPICKUPS + NUMSCRIPTEDPICKUPS;
pub const NUMCOLLECTEDPICKUPS: usize = 20;
pub const NUMPACMANPICKUPS: usize = 256;
pub const PICKUP_SAVE_RECORD_SIZE: usize = 0x1C;
pub const PICKUPS_SAVE_SIZE: usize =
    PICKUP_SAVE_RECORD_SIZE * NUMPICKUPS + 2 + 2 + 4 * NUMCOLLECTEDPICKUPS;

pub const PICKUP_NONE: u8 = 0;
pub const PICKUP_IN_SHOP: u8 = 1;
pub const PICKUP_ON_STREET: u8 = 2;
pub const PICKUP_ONCE: u8 = 3;
pub const PICKUP_ONCE_TIMEOUT: u8 = 4;
pub const PICKUP_COLLECTABLE1: u8 = 5;
pub const PICKUP_IN_SHOP_OUT_OF_STOCK: u8 = 6;
pub const PICKUP_MONEY: u8 = 7;
pub const PICKUP_MINE_INACTIVE: u8 = 8;
pub const PICKUP_MINE_ARMED: u8 = 9;
pub const PICKUP_NAUTICAL_MINE_INACTIVE: u8 = 10;
pub const PICKUP_NAUTICAL_MINE_ARMED: u8 = 11;
pub const PICKUP_FLOATINGPACKAGE: u8 = 12;
pub const PICKUP_FLOATINGPACKAGE_FLOATING: u8 = 13;
pub const PICKUP_ON_STREET_SLOW: u8 = 14;
pub const PACMAN_NONE: u8 = 0;
pub const PACMAN_SCRAMBLE: u8 = 1;
pub const PACMAN_RACE: u8 = 2;

/// Executable pickup policy: re3 AmmoForWeapon_OnStreet, shared by collection
/// and CPed::CreateDeadPedWeaponPickups. Models still resolve from installed IDE.
pub(crate) const STREET_AMMO: [u16; 20] = [
    0, 1, 9, 25, 5, 30, 60, 5, 1, 50, 1, 1, 0, 200, 0, 100, 0, 0, 0, 0,
];

pub(crate) fn reset_vehicle_pacman_load<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>)
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    vehicle.pac_man_pickups_carried = 0;
    vehicle.physical.mass /= vehicle.physical.force_multiplier;
    vehicle.physical.turn_mass /= vehicle.physical.force_multiplier;
    vehicle.physical.force_multiplier = 1.0;
}

pub(crate) fn add_vehicle_pacman_load<VehCore, VehVitality, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(vehicle: &mut crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>)
where
    VehCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    vehicle.pac_man_pickups_carried = vehicle.pac_man_pickups_carried.wrapping_add(1);
    vehicle.physical.move_speed.x *= 0.65;
    vehicle.physical.move_speed.y *= 0.65;
    vehicle.physical.move_speed.z *= 0.65;
    let mass_mult = (vehicle.physical.mass + 250.0) / vehicle.physical.mass;
    vehicle.physical.mass *= mass_mult;
    vehicle.physical.turn_mass *= mass_mult;
    vehicle.physical.force_multiplier *= mass_mult;
}

#[derive(Debug, Clone, Copy)]
pub struct CPickup {
    pub pickup_type: u8,
    pub removed: bool,
    pub quantity: u16,
    pub object_present: bool,
    pub object_pool_index: Option<i32>,
    pub timer: u32,
    pub model_index: i16,
    pub index: u16,
    pub pos: CVector,
}

impl CPickup {
    pub const fn init() -> Self {
        Self {
            pickup_type: PICKUP_NONE,
            removed: false,
            quantity: 0,
            object_present: false,
            object_pool_index: None,
            timer: 0,
            model_index: 0,
            index: 1,
            pos: CVector::ZERO,
        }
    }

    fn save_object_index(&self) -> i32 {
        self.object_pool_index
            .filter(|_| self.pickup_type != PICKUP_NONE && self.object_present)
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    pub fn write_compatible_save(self, writer: &mut SaveBufWriter) {
        writer.write(self.pickup_type);
        writer.write(bool_to_u8(self.removed));
        writer.write(self.quantity);
        writer.write(self.save_object_index());
        writer.write(self.timer);
        writer.write(self.model_index);
        writer.write(self.index);
        writer.write(self.pos.x);
        writer.write(self.pos.y);
        writer.write(self.pos.z);
    }

    pub fn read_compatible_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let pickup_type = reader.read::<u8>()?;
        let removed = reader.read::<u8>()? != 0;
        let quantity = reader.read::<u16>()?;
        let object_index = reader.read::<i32>()?;
        let object_pool_index = if pickup_type != PICKUP_NONE && object_index != 0 {
            Some(object_index - 1)
        } else {
            None
        };
        Ok(Self {
            pickup_type,
            removed,
            quantity,
            object_present: object_pool_index.is_some(),
            object_pool_index,
            timer: reader.read::<u32>()?,
            model_index: reader.read::<i16>()?,
            index: reader.read::<u16>()?,
            pos: CVector::new(
                reader.read::<f32>()?,
                reader.read::<f32>()?,
                reader.read::<f32>()?,
            ),
        })
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CPickups {
    pub pickups: [CPickup; NUMPICKUPS],
    pub pickups_collected: [i32; NUMCOLLECTEDPICKUPS],
    pub collected_pickup_index: i16,
    pub num_messages: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickupObjectRestore {
    pub pickup_slot: usize,
    pub object_pool_index: i32,
    pub model_index: i16,
    pub position: CVector,
}

impl CPickups {
    pub fn init() -> Self {
        Self {
            pickups: [CPickup::init(); NUMPICKUPS],
            pickups_collected: [0; NUMCOLLECTEDPICKUPS],
            collected_pickup_index: 0,
            num_messages: 0,
        }
    }

    /// re3 `CPickups::GenerateNewOne`; the returned value is the generation
    /// encoded SCM handle (`slot | index << 16`). Rendering its CObject is a
    /// separate runtime concern, so this state records its pending presence.
    pub fn generate_new_one(
        &mut self,
        position: CVector,
        model_index: i32,
        pickup_type: i32,
        quantity: i32,
        now_ms: u32,
    ) -> Option<i32> {
        let pickup_type = pickup_type as u8;
        let special_slot = matches!(
            pickup_type,
            PICKUP_FLOATINGPACKAGE | PICKUP_NAUTICAL_MINE_INACTIVE
        );
        let mut slot = if special_slot {
            (0..NUMPICKUPS)
                .rev()
                .find(|&slot| self.pickups[slot].pickup_type == PICKUP_NONE)
        } else {
            (0..NUMGENERALPICKUPS).find(|&slot| self.pickups[slot].pickup_type == PICKUP_NONE)
        };
        if slot.is_none() {
            slot = (0..NUMGENERALPICKUPS)
                .find(|&slot| self.pickups[slot].pickup_type == PICKUP_MONEY)
                .or_else(|| {
                    (0..NUMGENERALPICKUPS)
                        .find(|&slot| self.pickups[slot].pickup_type == PICKUP_ONCE_TIMEOUT)
                });
        }
        let slot = slot?;
        let pickup = &mut self.pickups[slot];
        pickup.pickup_type = match pickup_type {
            PICKUP_MINE_INACTIVE | PICKUP_MINE_ARMED => PICKUP_MINE_INACTIVE,
            PICKUP_NAUTICAL_MINE_INACTIVE | PICKUP_NAUTICAL_MINE_ARMED => {
                PICKUP_NAUTICAL_MINE_INACTIVE
            }
            other => other,
        };
        pickup.removed = false;
        pickup.quantity = quantity as u16;
        pickup.timer = match pickup_type {
            PICKUP_ONCE_TIMEOUT => now_ms.saturating_add(20_000),
            PICKUP_MONEY => now_ms.saturating_add(30_000),
            PICKUP_MINE_INACTIVE
            | PICKUP_MINE_ARMED
            | PICKUP_NAUTICAL_MINE_INACTIVE
            | PICKUP_NAUTICAL_MINE_ARMED => now_ms.saturating_add(1_500),
            _ => 0,
        };
        pickup.model_index = model_index as i16;
        pickup.pos = position;
        pickup.object_present = true;
        pickup.object_pool_index = None;
        pickup.index = if pickup.index >= 0xFFFE {
            1
        } else {
            pickup.index + 1
        };
        Some(slot as i32 | (i32::from(pickup.index) << 16))
    }

    /// re3 `CPickups::PassTime`: pickup timers are remaining durations here,
    /// not absolute clock values while game time is skipped by GameLogic.
    pub fn pass_time(&mut self, time_ms: u32) {
        for pickup in &mut self.pickups {
            if pickup.pickup_type != PICKUP_NONE {
                pickup.timer = pickup.timer.saturating_sub(time_ms);
            }
        }
    }

    /// re3 `GetActualPickupIndex` generation validation.
    pub fn actual_pickup_index(&self, handle: i32) -> Option<usize> {
        if handle == -1 {
            return None;
        }
        let slot = (handle as u32 & 0xFFFF) as usize;
        let generation = ((handle as u32 >> 16) & 0xFFFF) as u16;
        self.pickups
            .get(slot)
            .filter(|pickup| pickup.index == generation)
            .map(|_| slot)
    }

    /// re3 `RemovePickUp`.
    pub fn remove_pickup(&mut self, handle: i32) -> bool {
        let Some(slot) = self.actual_pickup_index(handle) else {
            return false;
        };
        let pickup = &mut self.pickups[slot];
        pickup.object_present = false;
        pickup.object_pool_index = None;
        pickup.pickup_type = PICKUP_NONE;
        pickup.removed = true;
        true
    }

    /// re3 `IsPickUpPickedUp`: consumes the matching collected id.
    pub fn is_pickup_picked_up(&mut self, handle: i32) -> bool {
        if let Some(collected) = self
            .pickups_collected
            .iter_mut()
            .find(|collected| **collected == handle)
        {
            *collected = 0;
            true
        } else {
            false
        }
    }

    pub fn save(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        for pickup in self.pickups {
            pickup.write_compatible_save(&mut writer);
        }
        writer.write(self.collected_pickup_index);
        writer.write(0_u16);
        for pickup in self.pickups_collected {
            writer.write(pickup);
        }
        debug_assert_eq!(writer.offset(), PICKUPS_SAVE_SIZE);
        writer.into_inner()
    }

    pub fn build_object_restore_plan(&self) -> Vec<PickupObjectRestore> {
        self.pickups
            .iter()
            .enumerate()
            .filter_map(|(pickup_slot, pickup)| {
                Some(PickupObjectRestore {
                    pickup_slot,
                    object_pool_index: pickup.object_pool_index?,
                    model_index: pickup.model_index,
                    position: pickup.pos,
                })
            })
            .collect()
    }

    pub fn load(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let mut pickups = [CPickup::init(); NUMPICKUPS];
        for pickup in &mut pickups {
            *pickup = CPickup::read_compatible_save(&mut reader)?;
        }

        let collected_pickup_index = reader.read::<i16>()?;
        reader.skip(2)?;
        let mut pickups_collected = [0_i32; NUMCOLLECTEDPICKUPS];
        for pickup in &mut pickups_collected {
            *pickup = reader.read::<i32>()?;
        }

        Ok(Self {
            pickups,
            pickups_collected,
            collected_pickup_index,
            num_messages: 0,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CPacManPickup {
    pub pos: CVector,
    /// Generation-checked `CPools::ms_pObjectPool` handle for `m_pObject`.
    pub object_handle: Option<i32>,
    pub pickup_type: u8,
}

impl CPacManPickup {
    pub const fn init() -> Self {
        Self {
            pos: CVector::ZERO,
            object_handle: None,
            pickup_type: PACMAN_NONE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PacManObjectSpawn {
    pub pickup_slot: usize,
    pub model_index: i32,
    pub position: CVector,
}

#[derive(Debug, Clone, Resource)]
pub struct CPacManPickups {
    pub pickups: [CPacManPickup; NUMPACMANPICKUPS],
    pub last_pickup_coors: CVector,
    pub pills_eaten_in_race: i32,
    pub active: bool,
    /// File globals adjacent to `CPacManPickups` in re3.
    pub collect_game_state: i32,
    pub things_to_collect: i16,
}

impl CPacManPickups {
    pub fn init() -> Self {
        Self {
            pickups: [CPacManPickup::init(); NUMPACMANPICKUPS],
            last_pickup_coors: CVector::ZERO,
            pills_eaten_in_race: 0,
            active: false,
            collect_game_state: 0,
            things_to_collect: 0,
        }
    }

    /// re3 `GeneratePMPickUpsForRace`. GTA III contains only race zero.
    pub fn start_pac_man_race(&mut self, race: i32) -> Option<Vec<PacManObjectSpawn>> {
        if race != 0 {
            return None;
        }
        let mut spawns = Vec::with_capacity(PACMAN_RACE_POINTS_1.len());
        for &position in PACMAN_RACE_POINTS_1 {
            let slot = self
                .pickups
                .iter()
                .position(|pickup| pickup.pickup_type == PACMAN_NONE)?;
            self.pickups[slot].pickup_type = PACMAN_RACE;
            self.pickups[slot].pos = position;
            self.pickups[slot].object_handle = None;
            spawns.push(PacManObjectSpawn {
                pickup_slot: slot,
                model_index: crate::modelinfo::ModelIndices::MI_DONKEYMAG as i32,
                position,
            });
        }
        self.active = true;
        self.pills_eaten_in_race = 0;
        Some(spawns)
    }

    /// re3 `GeneratePMPickUps`. `treadable_ground` must perform the same
    /// 1000→-1000 static vertical line and reject non-treadable entities.
    pub fn start_pac_man_scramble(
        &mut self,
        pos: CVector,
        scramble_mult: f32,
        count: i16,
        mut random_number: impl FnMut() -> u16,
        mut treadable_ground: impl FnMut(f32, f32) -> Option<f32>,
    ) -> Vec<PacManObjectSpawn> {
        let mut spawns = Vec::with_capacity(count.max(0) as usize);
        let mut remaining = count;
        let mut search_from = 0_usize;
        while remaining > 0 {
            let Some(relative_slot) = self.pickups[search_from..]
                .iter()
                .position(|pickup| pickup.pickup_type == PACMAN_NONE)
            else {
                break;
            };
            let slot = search_from + relative_slot;
            loop {
                let random = random_number();
                let x = pos.x + (i32::from(random & 0xff) - 128) as f32 * scramble_mult / 128.0;
                let y =
                    pos.y + (i32::from((random >> 8) & 0xff) - 128) as f32 * scramble_mult / 128.0;
                let Some(ground_z) = treadable_ground(x, y) else {
                    continue;
                };
                let position = CVector::new(x, y, ground_z + 0.7);
                self.pickups[slot].pickup_type = PACMAN_SCRAMBLE;
                self.pickups[slot].pos = position;
                self.pickups[slot].object_handle = None;
                spawns.push(PacManObjectSpawn {
                    pickup_slot: slot,
                    model_index: crate::modelinfo::ModelIndices::MI_BULLION as i32,
                    position,
                });
                break;
            }
            search_from = slot;
            remaining -= 1;
        }
        self.active = true;
        spawns
    }

    /// re3 `GenerateOnePMPickUp`: intentionally reuses slot zero and does not
    /// manufacture a CObject.
    pub fn generate_one_pm_pickup(&mut self, position: CVector) {
        self.active = true;
        self.pickups[0].pickup_type = PACMAN_RACE;
        self.pickups[0].pos = position;
    }

    pub fn start_pac_man_record(&mut self, player_position: CVector) {
        self.collect_game_state = 20;
        self.last_pickup_coors = player_position;
    }

    /// re3 `ClearPMPickUps`; returns live object handles for CWorld/CPools
    /// removal by the owning runtime.
    pub fn clear_pm_pickups(&mut self) -> Vec<i32> {
        self.active = false;
        let mut handles = Vec::new();
        for pickup in &mut self.pickups {
            if let Some(handle) = pickup.object_handle.take() {
                handles.push(handle);
            }
            pickup.pickup_type = PACMAN_NONE;
        }
        handles
    }

    pub fn frame_slice(frame_counter: u32) -> std::ops::Range<usize> {
        const FRAME_SPAN: usize = 4;
        let slice = frame_counter as usize % FRAME_SPAN;
        let width = NUMPACMANPICKUPS / FRAME_SPAN;
        slice * width..(slice + 1) * width
    }
}

// re3 `aRacePoints1` (Diablo "Big'n'Veiny"). The terminating zero vector is
// represented by the Rust slice length rather than stored as a pickup.
pub const PACMAN_RACE_POINTS_1: &[CVector] = &[
    CVector::new(913.62219, -155.13692, 4.9699469),
    CVector::new(913.92401, -124.12943, 4.9692569),
    CVector::new(913.27899, -93.524231, 7.4325991),
    CVector::new(912.60852, -63.15905, 7.4533591),
    CVector::new(934.22144, -42.049122, 7.4511471),
    CVector::new(958.88092, -23.863735, 7.4652338),
    CVector::new(978.50812, -0.78458798, 5.13515),
    CVector::new(1009.4175, -2.1041219, 2.4461579),
    CVector::new(1040.6313, -2.0793829, 2.293175),
    CVector::new(1070.7863, -2.084095, 2.2789791),
    CVector::new(1100.5773, -8.468729, 5.3248072),
    CVector::new(1119.9341, -31.738031, 7.1913071),
    CVector::new(1122.1664, -62.762737, 7.4703908),
    CVector::new(1122.814, -93.650566, 8.5577497),
    CVector::new(1125.8253, -124.26616, 9.9803305),
    CVector::new(1153.8727, -135.47169, 14.150617),
    CVector::new(1184.0831, -135.82845, 14.973998),
    CVector::new(1192.0432, -164.57816, 19.18627),
    CVector::new(1192.7761, -194.28871, 24.799675),
    CVector::new(1215.1527, -215.0714, 25.74975),
    CVector::new(1245.79, -215.39304, 28.70726),
    CVector::new(1276.2477, -216.39485, 33.71236),
    CVector::new(1306.5535, -216.71007, 39.711472),
    CVector::new(1335.0244, -224.59329, 46.474979),
    CVector::new(1355.4879, -246.27664, 49.934841),
    CVector::new(1362.6003, -276.47064, 49.96265),
    CVector::new(1363.027, -307.30847, 49.969173),
    CVector::new(1365.343, -338.08609, 49.967789),
    CVector::new(1367.5957, -368.01105, 50.092304),
    CVector::new(1368.2749, -398.38049, 50.061268),
    CVector::new(1366.9034, -429.98483, 50.057545),
    CVector::new(1356.8534, -459.09259, 50.035545),
    CVector::new(1335.5819, -481.13544, 47.217903),
    CVector::new(1306.7552, -491.07443, 40.202629),
    CVector::new(1275.5978, -491.33194, 33.969223),
    CVector::new(1244.702, -491.46451, 29.111021),
    CVector::new(1213.2222, -491.8754, 25.771168),
    CVector::new(1182.7729, -492.19995, 24.749964),
    CVector::new(1152.6874, -491.42221, 21.70038),
    CVector::new(1121.5352, -491.94604, 20.075182),
    CVector::new(1090.7056, -492.63751, 17.585758),
    CVector::new(1059.6008, -491.65762, 14.848632),
    CVector::new(1029.113, -489.66031, 14.918498),
    CVector::new(998.20679, -486.78107, 14.945688),
    CVector::new(968.00555, -484.91266, 15.001229),
    CVector::new(937.74939, -492.09015, 14.958629),
    CVector::new(927.17352, -520.97736, 14.972308),
    CVector::new(929.29749, -552.08643, 14.978855),
    CVector::new(950.69525, -574.47778, 14.972788),
    CVector::new(974.02826, -593.56024, 14.966445),
    CVector::new(989.04779, -620.12854, 14.951016),
    CVector::new(1014.1639, -637.3905, 14.966736),
    CVector::new(1017.5961, -667.3736, 14.956415),
    CVector::new(1041.9735, -685.94391, 15.003841),
    CVector::new(1043.3064, -716.11298, 14.974236),
    CVector::new(1043.5337, -746.63855, 14.96919),
    CVector::new(1044.142, -776.93823, 14.965424),
    CVector::new(1044.2657, -807.29395, 14.97171),
    CVector::new(1017.0797, -820.1076, 14.975431),
    CVector::new(986.23865, -820.37103, 14.972883),
    CVector::new(956.10065, -820.23291, 14.981133),
    CVector::new(925.86914, -820.19049, 14.976553),
    CVector::new(897.69702, -831.08734, 14.962709),
    CVector::new(868.06586, -835.99237, 14.970685),
    CVector::new(836.93054, -836.84387, 14.965049),
    CVector::new(811.63586, -853.7915, 15.067576),
    CVector::new(811.46344, -884.27368, 12.247812),
    CVector::new(811.60651, -914.70959, 9.2393751),
    CVector::new(811.10425, -945.16272, 5.817255),
    CVector::new(816.54584, -975.64587, 4.998558),
    CVector::new(828.2951, -1003.3685, 5.0471172),
    CVector::new(852.28839, -1021.5963, 4.9371028),
    CVector::new(882.50067, -1025.4459, 5.14077),
    CVector::new(912.84821, -1026.7874, 8.3415451),
    CVector::new(943.68274, -1026.6914, 11.341879),
    CVector::new(974.4129, -1027.3682, 14.410345),
    CVector::new(1004.1079, -1036.0778, 14.92961),
    CVector::new(1030.1144, -1051.1224, 14.850387),
    CVector::new(1058.7585, -1060.342, 14.821624),
    CVector::new(1087.7797, -1068.3263, 14.800561),
    CVector::new(1099.8807, -1095.656, 11.877907),
    CVector::new(1130.0005, -1101.994, 11.853914),
    CVector::new(1160.3809, -1101.6355, 11.854824),
    CVector::new(1191.8524, -1102.1577, 11.853843),
    CVector::new(1223.3307, -1102.7448, 11.852233),
    CVector::new(1253.564, -1098.1045, 11.853944),
    CVector::new(1262.0203, -1069.1785, 14.8147),
    CVector::new(1290.9998, -1059.1882, 14.816016),
    CVector::new(1316.246, -1041.0635, 14.81109),
    CVector::new(1331.7539, -1013.835, 14.81207),
    CVector::new(1334.0579, -983.55402, 14.827253),
    CVector::new(1323.2429, -954.23083, 14.954678),
    CVector::new(1302.7495, -932.21216, 14.962917),
    CVector::new(1317.418, -905.89325, 14.967506),
    CVector::new(1337.9503, -883.5025, 14.969675),
    CVector::new(1352.6929, -855.96954, 14.967854),
    CVector::new(1357.2388, -826.26971, 14.97295),
    CVector::new(1384.8668, -812.47693, 12.907736),
    CVector::new(1410.8983, -795.39056, 12.052228),
    CVector::new(1433.901, -775.55811, 11.96265),
    CVector::new(1443.8615, -746.92511, 11.976114),
    CVector::new(1457.7015, -720.00903, 11.971177),
    CVector::new(1481.5685, -701.30237, 11.977908),
    CVector::new(1511.4004, -696.83295, 11.972709),
    CVector::new(1542.1796, -695.61676, 11.970441),
    CVector::new(1570.3301, -684.6239, 11.969202),
];

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

#[cfg(test)]
mod pacman_tests {
    use super::*;
    use crate::{
        collision::ColModel::{CColBox, CColVector},
        vehicles::Vehicle::CVehicle,
    };
    use rustgta3_core::core::General::CGeneral;

    #[test]
    fn race_zero_uses_the_complete_cpp_point_table_and_object_models() {
        let mut pacman = CPacManPickups::init();
        pacman.pills_eaten_in_race = 9;
        let spawns = pacman.start_pac_man_race(0).unwrap();
        assert_eq!(spawns.len(), 106);
        assert_eq!(
            spawns[0].position,
            CVector::new(913.62219, -155.13692, 4.9699469)
        );
        assert_eq!(
            spawns.last().unwrap().position,
            CVector::new(1570.3301, -684.6239, 11.969202)
        );
        assert!(spawns.iter().all(|spawn| {
            spawn.model_index == crate::modelinfo::ModelIndices::MI_DONKEYMAG as i32
        }));
        assert_eq!(pacman.pills_eaten_in_race, 0);
        assert!(pacman.active);
        assert!(
            pacman.pickups[..106]
                .iter()
                .all(|pickup| pickup.pickup_type == PACMAN_RACE)
        );
    }

    #[test]
    fn scramble_uses_cpp_signed_byte_offsets_and_seventy_centimetre_lift() {
        let mut pacman = CPacManPickups::init();
        let mut random = [0x00ff_u16, 0xff00].into_iter();
        let spawns = pacman.start_pac_man_scramble(
            CVector::new(100.0, 200.0, 3.0),
            128.0,
            2,
            || random.next().unwrap(),
            |_x, _y| Some(12.0),
        );
        assert_eq!(spawns.len(), 2);
        assert_eq!(spawns[0].position, CVector::new(227.0, 72.0, 12.7));
        assert_eq!(spawns[1].position, CVector::new(-28.0, 327.0, 12.7));
        assert!(spawns.iter().all(|spawn| {
            spawn.model_index == crate::modelinfo::ModelIndices::MI_BULLION as i32
        }));
        assert_eq!(CPacManPickups::frame_slice(0), 0..64);
        assert_eq!(CPacManPickups::frame_slice(3), 192..256);
        assert_eq!(CPacManPickups::frame_slice(4), 0..64);
    }

    #[test]
    fn cleanup_returns_real_object_handles_and_resets_every_slot() {
        let mut pacman = CPacManPickups::init();
        pacman.active = true;
        pacman.pickups[3].pickup_type = PACMAN_RACE;
        pacman.pickups[3].object_handle = Some(0x301);
        pacman.pickups[200].pickup_type = PACMAN_SCRAMBLE;
        pacman.pickups[200].object_handle = Some(0xc801);
        let mut handles = pacman.clear_pm_pickups();
        handles.sort_unstable();
        assert_eq!(handles, vec![0x301, 0xc801]);
        assert!(!pacman.active);
        assert!(
            pacman.pickups.iter().all(|pickup| {
                pickup.pickup_type == PACMAN_NONE && pickup.object_handle.is_none()
            })
        );
    }

    #[test]
    fn vehicle_touch_uses_the_rotated_local_bounding_box_like_cpp() {
        let mut general = CGeneral::with_seed(1);
        let mut vehicle = CVehicle::new(0, 0, &mut general);
        vehicle
            .physical
            .entity
            .set_position(CVector::new(10.0, 20.0, 3.0));
        vehicle
            .physical
            .entity
            .set_heading(std::f32::consts::FRAC_PI_2);
        let bounds = CColBox::new(
            CColVector::new(-1.0, -2.0, -0.5),
            CColVector::new(1.0, 2.0, 0.5),
            0,
            0,
        );
        let touching = vehicle
            .physical
            .entity
            .transform_point(CVector::new(1.4, 0.0, 0.0));
        let outside = vehicle
            .physical
            .entity
            .transform_point(CVector::new(1.6, 0.0, 0.0));
        assert!(vehicle.is_sphere_touching_vehicle(touching, 0.5, &bounds));
        assert!(!vehicle.is_sphere_touching_vehicle(outside, 0.5, &bounds));
    }
}

// === FAZ7c: moved from core/Game.rs (verbatim, parity-preserving) ===
// Pac-Man pickup runtime: spawn/remove helpers and the world tick.
// Schedule stays in core/Game.rs.

#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::audio::soundlist::{SOUND_PICKUP_PACMAN_PACKAGE, SOUND_PICKUP_PACMAN_PILL};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Game::{
    ScriptEntityIndex, ScriptEntityWorld, ScriptObjectEntity, ScriptVehicleEntity,
    focused_player_vehicle_handle, remove_script_object, spawn_script_object,
};
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPools};
#[allow(unused_imports)]
use crate::objects::Object::{CObject, MISSION_OBJECT};
#[allow(unused_imports)]
use crate::peds::PlayerPed::CWorldPlayers;
#[allow(unused_imports)]
use rustgta3_core::core::{General::CGeneral, Timer::CTimer};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

pub(crate) fn remove_pacman_object(
    handle: i32,
    commands: &mut Commands,
    index: &mut ScriptEntityIndex,
    pools: Option<&mut CPools>,
) {
    let _ = remove_script_object(commands, index, pools, handle);
}

pub(crate) fn spawn_pacman_objects(
    spawns: &[PacManObjectSpawn],
    pacman: &mut CPacManPickups,
    commands: &mut Commands,
    index: &mut ScriptEntityIndex,
    pools: &mut CPools,
    general: &mut CGeneral,
    file_loader: &CFileLoader,
) {
    for spawn in spawns {
        let Some(handle) = pools
            .object_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Object))
            .map(|handle| handle.0)
        else {
            continue;
        };
        let mut object = CObject::new_with_model(spawn.model_index, true, general);
        object.initialize_world_state(file_loader);
        object.object_created_by = MISSION_OBJECT;
        object.physical.entity.set_position(spawn.position);
        object
            .physical
            .entity
            .set_orientation(0.0, 0.0, -std::f32::consts::FRAC_PI_2);
        object.object_matrix = object.physical.entity.matrix;
        object.physical.flags.affected_by_gravity = false;
        object.physical.entity.flags.explosion_proof = true;
        object.physical.entity.flags.uses_collision = false;
        object.flags.is_pickup = false;
        spawn_script_object(commands, index, ScriptObjectEntity { handle, object });
        if let Some(pickup) = pacman.pickups.get_mut(spawn.pickup_slot) {
            pickup.object_handle = Some(handle);
        }
    }
}

/// re3 `CPacManPickups::Update` plus `CPacManPickup::Update`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_pacman_runtime(
    mut pacman: Option<ResMut<CPacManPickups>>,
    mut script_entities: ScriptEntityWorld,
    mut commands: Commands,
    mut pools: Option<ResMut<CPools>>,
    mut world_players: CWorldPlayers,
    file_loader: Res<CFileLoader>,
    timer: Res<CTimer>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
) {
    let Some(pacman) = pacman.as_mut() else {
        return;
    };
    let player_position = world_players.focused().map(|player| player.position());
    let player_vehicle_handle = focused_player_vehicle_handle(&world_players, &script_entities);

    let mut cleanup_handles = Vec::new();
    if let (Some(handle), Some(player_position)) = (player_vehicle_handle, player_position) {
        let dx = player_position.x - 1072.0;
        let dy = player_position.y + 948.0;
        let dz = player_position.z - 14.5;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        match pacman.collect_game_state {
            1 => {
                if dist < 10.0
                    && let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    let mut vehicle = vehicle.base_mut();
                    pacman.things_to_collect -= i16::from(vehicle.pac_man_pickups_carried);
                    reset_vehicle_pacman_load(&mut vehicle);
                }
                if pacman.things_to_collect <= 0 {
                    pacman.collect_game_state = 2;
                    cleanup_handles = pacman.clear_pm_pickups();
                }
            }
            2 => {
                if dist > 11.0 {
                    pacman.collect_game_state = 0;
                }
            }
            20 => {
                let dx = player_position.x - pacman.last_pickup_coors.x;
                let dy = player_position.y - pacman.last_pickup_coors.y;
                let dz = player_position.z - pacman.last_pickup_coors.z;
                if dx * dx + dy * dy + dz * dz > 30.0 * 30.0 {
                    pacman.last_pickup_coors = player_position;
                }
            }
            _ => {}
        }
    }
    for handle in cleanup_handles {
        remove_pacman_object(
            handle,
            &mut commands,
            &mut script_entities.index,
            pools.as_deref_mut(),
        );
    }

    if pacman.active
        && let Some(vehicle_handle) = player_vehicle_handle
    {
        for slot in CPacManPickups::frame_slice(timer.frame_counter()) {
            let pickup = pacman.pickups[slot];
            if pickup.pickup_type == PACMAN_NONE {
                continue;
            }
            let touched = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                .and_then(|vehicle| {
                    let base = vehicle.base();
                    let vehicle_pos = base.physical.entity.get_position();
                    let dx = vehicle_pos.x - pickup.pos.x;
                    let dy = vehicle_pos.y - pickup.pos.y;
                    if dx * dx + dy * dy >= 100.0 {
                        return Some(false);
                    }
                    let col_model = file_loader
                        .model_info
                        .col_model(base.physical.entity.model_index.max(0) as usize)?;
                    Some(base.is_sphere_touching_vehicle(pickup.pos, 1.5, &col_model.bounding_box))
                })
                .unwrap_or(false);
            if !touched {
                continue;
            }

            match pickup.pickup_type {
                PACMAN_SCRAMBLE => {
                    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
                        add_vehicle_pacman_load(&mut vehicle.base_mut());
                    }
                    if let Some(mut player) = world_players.focused_mut() {
                        player.wanted.m_nChaos += 10;
                        player.wanted.UpdateWantedLevel();
                    }
                    if let Some(audio) = dmaudio.as_mut() {
                        audio.PlayFrontEndSound(SOUND_PICKUP_PACMAN_PACKAGE as u16, 0);
                    }
                }
                PACMAN_RACE => {
                    pacman.pills_eaten_in_race = pacman.pills_eaten_in_race.wrapping_add(1);
                    if let Some(audio) = dmaudio.as_mut() {
                        audio.PlayFrontEndSound(SOUND_PICKUP_PACMAN_PILL as u16, 0);
                    }
                }
                _ => {}
            }
            pacman.pickups[slot].pickup_type = PACMAN_NONE;
            if let Some(object_handle) = pacman.pickups[slot].object_handle.take() {
                remove_pacman_object(
                    object_handle,
                    &mut commands,
                    &mut script_entities.index,
                    pools.as_deref_mut(),
                );
            }
        }
    }

    if pacman.active {
        let angle = (timer.time_in_milliseconds() % 1024) as f32 * std::f32::consts::TAU / 1024.0;
        for pickup in &pacman.pickups {
            let Some(handle) = pickup.object_handle else {
                continue;
            };
            if let Some(bevy_entity) = script_entities.index.object(handle)
                && let Ok(entity) = script_entities.objects.get_entity_mut(bevy_entity)
            {
                entity.object.physical.entity.set_heading(angle);
                entity.object.object_matrix = entity.object.physical.entity.matrix;
            }
        }
    }
}
