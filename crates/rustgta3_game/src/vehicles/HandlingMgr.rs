//! re3 reference: references/re-GTA/src/vehicles/HandlingMgr.cpp
//! re3 reference: references/re-GTA/src/vehicles/HandlingMgr.h
//!
//! Vehicle handling data manager state.

use bevy::prelude::Resource;
use rustgta3_core::core::Coordinates::CVector;
use rustgta3_formats::formats::DataRead::{HandlingCfg, HandlingRecord};

use crate::{
    entities::Physical::GRAVITY,
    vehicles::{
        Automobile::CAutomobileHandlingSnapshot, Transmission::cTransmission,
        Vehicle::CVehicleWheelHandling,
    },
};

pub const NUMHANDLINGS: usize = 57;
/// Native HANDLING.CFG fields 1..31 after the vehicle name.
pub const HANDLING_DATA_FIELD_COUNT: usize = 31;
pub const HANDLING_1G_BOOST: u32 = 0x1;
pub const HANDLING_2G_BOOST: u32 = 0x2;
pub const HANDLING_REV_BONNET: u32 = 0x4;
pub const HANDLING_HANGING_BOOT: u32 = 0x8;
pub const HANDLING_NO_DOORS: u32 = 0x10;
pub const HANDLING_IS_VAN: u32 = 0x20;
pub const HANDLING_IS_BUS: u32 = 0x40;
pub const HANDLING_IS_LOW: u32 = 0x80;
pub const HANDLING_DBL_EXHAUST: u32 = 0x100;
pub const HANDLING_TAILGATE_BOOT: u32 = 0x200;
pub const HANDLING_NOSWING_BOOT: u32 = 0x400;
pub const HANDLING_NONPLAYER_STABILISER: u32 = 0x800;
pub const HANDLING_NEUTRALHANDLING: u32 = 0x1000;
pub const HANDLING_HAS_NO_ROOF: u32 = 0x2000;
pub const HANDLING_IS_BIG: u32 = 0x4000;
pub const HANDLING_HALOGEN_LIGHTS: u32 = 0x8000;
pub const VEHICLE_NAMES: [&str; NUMHANDLINGS] = [
    "LANDSTAL", "IDAHO", "STINGER", "LINERUN", "PEREN", "SENTINEL", "PATRIOT", "FIRETRUK", "TRASH",
    "STRETCH", "MANANA", "INFERNUS", "BLISTA", "PONY", "MULE", "CHEETAH", "AMBULAN", "FBICAR",
    "MOONBEAM", "ESPERANT", "TAXI", "KURUMA", "BOBCAT", "MRWHOOP", "BFINJECT", "POLICE",
    "ENFORCER", "SECURICA", "BANSHEE", "PREDATOR", "BUS", "RHINO", "BARRACKS", "TRAIN", "HELI",
    "DODO", "COACH", "CABBIE", "STALLION", "RUMPO", "RCBANDIT", "BELLYUP", "MRWONGS", "MAFIA",
    "YARDIE", "YAKUZA", "DIABLOS", "COLUMB", "HOODS", "AIRTRAIN", "DEADDODO", "SPEEDER", "REEFER",
    "PANLANT", "FLATBED", "YANKEE", "BORGNINE",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum tVehicleType {
    HANDLING_LANDSTAL = 0,
    HANDLING_IDAHO = 1,
    HANDLING_STINGER = 2,
    HANDLING_LINERUN = 3,
    HANDLING_PEREN = 4,
    HANDLING_SENTINEL = 5,
    HANDLING_PATRIOT = 6,
    HANDLING_FIRETRUK = 7,
    HANDLING_TRASH = 8,
    HANDLING_STRETCH = 9,
    HANDLING_MANANA = 10,
    HANDLING_INFERNUS = 11,
    HANDLING_BLISTA = 12,
    HANDLING_PONY = 13,
    HANDLING_MULE = 14,
    HANDLING_CHEETAH = 15,
    HANDLING_AMBULAN = 16,
    HANDLING_FBICAR = 17,
    HANDLING_MOONBEAM = 18,
    HANDLING_ESPERANT = 19,
    HANDLING_TAXI = 20,
    HANDLING_KURUMA = 21,
    HANDLING_BOBCAT = 22,
    HANDLING_MRWHOOP = 23,
    HANDLING_BFINJECT = 24,
    HANDLING_POLICE = 25,
    HANDLING_ENFORCER = 26,
    HANDLING_SECURICA = 27,
    HANDLING_BANSHEE = 28,
    HANDLING_PREDATOR = 29,
    HANDLING_BUS = 30,
    HANDLING_RHINO = 31,
    HANDLING_BARRACKS = 32,
    HANDLING_TRAIN = 33,
    HANDLING_HELI = 34,
    HANDLING_DODO = 35,
    HANDLING_COACH = 36,
    HANDLING_CABBIE = 37,
    HANDLING_STALLION = 38,
    HANDLING_RUMPO = 39,
    HANDLING_RCBANDIT = 40,
    HANDLING_BELLYUP = 41,
    HANDLING_MRWONGS = 42,
    HANDLING_MAFIA = 43,
    HANDLING_YARDIE = 44,
    HANDLING_YAKUZA = 45,
    HANDLING_DIABLOS = 46,
    HANDLING_COLUMB = 47,
    HANDLING_HOODS = 48,
    HANDLING_AIRTRAIN = 49,
    HANDLING_DEADDODO = 50,
    HANDLING_SPEEDER = 51,
    HANDLING_REEFER = 52,
    HANDLING_PANLANT = 53,
    HANDLING_FLATBED = 54,
    HANDLING_YANKEE = 55,
    HANDLING_BORGNINE = 56,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct tHandlingData {
    pub identifier: usize,
    pub mass: f32,
    pub inv_mass: f32,
    pub turn_mass: f32,
    pub dimension: CVector,
    pub centre_of_mass: CVector,
    pub percent_submerged: i8,
    pub buoyancy: f32,
    pub traction_multiplier: f32,
    pub transmission: cTransmission,
    pub brake_deceleration: f32,
    pub brake_bias: f32,
    pub abs: i8,
    pub steering_lock: f32,
    pub traction_loss: f32,
    pub traction_bias: f32,
    pub unused: f32,
    pub suspension_force_level: f32,
    pub suspension_damping_level: f32,
    pub suspension_upper_limit: f32,
    pub suspension_lower_limit: f32,
    pub suspension_bias: f32,
    pub collision_damage_multiplier: f32,
    pub flags: u32,
    pub seat_offset_distance: f32,
    pub monetary_value: i32,
    pub front_lights: i8,
    pub rear_lights: i8,
}

impl Default for tHandlingData {
    fn default() -> Self {
        Self {
            identifier: 0,
            mass: 0.0,
            inv_mass: 0.0,
            turn_mass: 0.0,
            dimension: CVector::ZERO,
            centre_of_mass: CVector::ZERO,
            percent_submerged: 0,
            buoyancy: 0.0,
            traction_multiplier: 0.0,
            transmission: cTransmission::default(),
            brake_deceleration: 0.0,
            brake_bias: 0.0,
            abs: 0,
            steering_lock: 0.0,
            traction_loss: 0.0,
            traction_bias: 0.0,
            unused: 0.0,
            suspension_force_level: 0.0,
            suspension_damping_level: 0.0,
            suspension_upper_limit: 0.0,
            suspension_lower_limit: 0.0,
            suspension_bias: 0.0,
            collision_damage_multiplier: 0.0,
            flags: 0,
            seat_offset_distance: 0.0,
            monetary_value: 0,
            front_lights: 0,
            rear_lights: 0,
        }
    }
}

/// re3's process-global `mod_HandlingManager`, owned as a Bevy resource.
///
/// Keeping the parsed manager resident is important: `HANDLING.CFG` is loaded
/// once during `CGame::InitialiseOnceAfterRW`, not reparsed for every spawned
/// car or every gameplay frame.
#[derive(Debug, Clone, PartialEq, Resource)]
pub struct cHandlingDataMgr {
    pub field_0: f32,
    pub wheel_friction: f32,
    pub field_8: f32,
    pub field_c: f32,
    pub field_10: f32,
    pub handling_data: [tHandlingData; NUMHANDLINGS],
    pub field_302c: u32,
}

impl Default for cHandlingDataMgr {
    fn default() -> Self {
        Self::new()
    }
}

impl cHandlingDataMgr {
    pub fn new() -> Self {
        Self {
            field_0: 0.0,
            wheel_friction: 0.9,
            field_8: 0.0,
            field_c: 0.0,
            field_10: 0.0,
            handling_data: [tHandlingData::default(); NUMHANDLINGS],
            field_302c: 0,
        }
    }

    pub fn get_handling_data(&self, handling: usize) -> Option<&tHandlingData> {
        self.handling_data.get(handling)
    }

    pub fn get_handling_data_mut(&mut self, handling: usize) -> Option<&mut tHandlingData> {
        self.handling_data.get_mut(handling)
    }

    pub fn initialise_from_cfg(&mut self, cfg: &HandlingCfg) -> Result<(), String> {
        *self = Self {
            field_0: 0.1,
            wheel_friction: 0.9,
            field_8: 1.0,
            field_c: 0.8,
            field_10: 0.98,
            handling_data: [tHandlingData::default(); NUMHANDLINGS],
            field_302c: 0,
        };

        for record in &cfg.records {
            self.apply_handling_record(record)?;
        }
        Ok(())
    }

    pub fn get_handling_id(name: &str) -> Option<usize> {
        VEHICLE_NAMES
            .iter()
            .position(|candidate| *candidate == name)
    }

    pub fn has_rear_wheel_drive(&self, handling: usize) -> bool {
        self.get_handling_data(handling)
            .map(|data| data.transmission.drive_type != 'F')
            .unwrap_or(false)
    }

    pub fn has_front_wheel_drive(&self, handling: usize) -> bool {
        self.get_handling_data(handling)
            .map(|data| data.transmission.drive_type != 'R')
            .unwrap_or(false)
    }

    pub fn automobile_snapshot(&self, handling: usize) -> Option<CAutomobileHandlingSnapshot> {
        self.get_handling_data(handling)
            .map(CAutomobileHandlingSnapshot::from)
    }

    pub fn wheel_handling(&self, handling: usize) -> Option<CVehicleWheelHandling> {
        self.get_handling_data(handling)
            .map(|data| CVehicleWheelHandling {
                mass: data.mass,
                traction_loss: data.traction_loss,
                wheel_friction: self.wheel_friction,
            })
    }

    fn apply_handling_record(&mut self, record: &HandlingRecord) -> Result<(), String> {
        if record.fields.len() < HANDLING_DATA_FIELD_COUNT {
            return Err(format!("HANDLING.CFG missing fields for {}: requires {HANDLING_DATA_FIELD_COUNT}, got {}", record.name, record.fields.len()));
        }
        let handling_id = Self::get_handling_id(&record.name)
            .ok_or_else(|| format!("unknown handling name {}", record.name))?;
        let mut handling = tHandlingData {
            identifier: handling_id,
            ..tHandlingData::default()
        };

        for (index, field) in record.fields.iter().enumerate() {
            match index {
                0 => handling.mass = parse_f32(field, &record.name, index)?,
                1 => handling.dimension.x = parse_f32(field, &record.name, index)?,
                2 => handling.dimension.y = parse_f32(field, &record.name, index)?,
                3 => handling.dimension.z = parse_f32(field, &record.name, index)?,
                4 => handling.centre_of_mass.x = parse_f32(field, &record.name, index)?,
                5 => handling.centre_of_mass.y = parse_f32(field, &record.name, index)?,
                6 => handling.centre_of_mass.z = parse_f32(field, &record.name, index)?,
                7 => handling.percent_submerged = parse_i8(field, &record.name, index)?,
                8 => handling.traction_multiplier = parse_f32(field, &record.name, index)?,
                9 => handling.traction_loss = parse_f32(field, &record.name, index)?,
                10 => handling.traction_bias = parse_f32(field, &record.name, index)?,
                11 => handling.transmission.number_of_gears = parse_i8(field, &record.name, index)?,
                12 => handling.transmission.max_velocity = parse_f32(field, &record.name, index)?,
                13 => {
                    handling.transmission.engine_acceleration =
                        parse_f32(field, &record.name, index)? * 0.4
                }
                14 => {
                    handling.transmission.drive_type = field.chars().next().unwrap_or('\0');
                }
                15 => {
                    handling.transmission.engine_type = field.chars().next().unwrap_or('\0');
                }
                16 => handling.brake_deceleration = parse_f32(field, &record.name, index)?,
                17 => handling.brake_bias = parse_f32(field, &record.name, index)?,
                18 => handling.abs = parse_i8(field, &record.name, index)?,
                19 => handling.steering_lock = parse_f32(field, &record.name, index)?,
                20 => handling.suspension_force_level = parse_f32(field, &record.name, index)?,
                21 => handling.suspension_damping_level = parse_f32(field, &record.name, index)?,
                22 => handling.seat_offset_distance = parse_f32(field, &record.name, index)?,
                23 => handling.collision_damage_multiplier = parse_f32(field, &record.name, index)?,
                24 => handling.monetary_value = parse_i32(field, &record.name, index)?,
                25 => handling.suspension_upper_limit = parse_f32(field, &record.name, index)?,
                26 => handling.suspension_lower_limit = parse_f32(field, &record.name, index)?,
                27 => handling.suspension_bias = parse_f32(field, &record.name, index)?,
                28 => {
                    handling.flags = u32::from_str_radix(field, 16).map_err(|err| {
                        format!(
                            "invalid hex handling field {} for {}: {err}",
                            index, record.name
                        )
                    })?;
                    handling.transmission.flags = handling.flags as u8;
                }
                29 => handling.front_lights = parse_i8(field, &record.name, index)?,
                30 => handling.rear_lights = parse_i8(field, &record.name, index)?,
                _ => {}
            }
        }

        convert_data_to_game_units(&mut handling);
        self.handling_data[handling_id] = handling;
        Ok(())
    }
}

impl From<&tHandlingData> for CAutomobileHandlingSnapshot {
    fn from(handling: &tHandlingData) -> Self {
        Self {
            mass: handling.mass,
            turn_mass: handling.turn_mass,
            centre_of_mass: handling.centre_of_mass,
            dimension: handling.dimension,
            buoyancy: handling.buoyancy,
            flags: handling.flags,
            handling_id: handling.identifier as i32,
            monetary_value: handling.monetary_value,
        }
    }
}

fn convert_data_to_game_units(handling: &mut tHandlingData) {
    handling.transmission.engine_acceleration *= 1.0 / (50.0 * 50.0);
    handling.transmission.max_velocity *= 1000.0 / (60.0 * 60.0 * 50.0);
    handling.brake_deceleration *= 1.0 / (50.0 * 50.0);
    handling.turn_mass = (handling.dimension.x * handling.dimension.x
        + handling.dimension.y * handling.dimension.y)
        * handling.mass
        / 12.0;
    if handling.turn_mass < 10.0 {
        handling.turn_mass *= 5.0;
    }
    handling.inv_mass = 1.0 / handling.mass;
    handling.buoyancy = 100.0 / f32::from(handling.percent_submerged) * GRAVITY * handling.mass;

    let mut a = 0.0;
    let mut b = 100.0;
    let mut velocity = handling.transmission.max_velocity;
    while a < b && velocity > 0.0 {
        velocity -= 0.01;
        a = handling.transmission.engine_acceleration / 6.0;
        let drag_accel =
            0.5 * velocity * velocity * handling.dimension.x * handling.dimension.z / handling.mass;
        b = -velocity * (1.0 / (drag_accel + 1.0) - 1.0);
    }

    if handling.identifier == tVehicleType::HANDLING_RCBANDIT as usize {
        handling.transmission.max_cruise_velocity = handling.transmission.max_velocity;
    } else {
        handling.transmission.max_cruise_velocity = velocity;
        handling.transmission.max_velocity = velocity * 1.2;
    }
    handling.transmission.max_reverse_velocity = -0.2;

    if handling.transmission.drive_type == '4' {
        handling.transmission.engine_acceleration /= 4.0;
    } else {
        handling.transmission.engine_acceleration /= 2.0;
    }
    handling.transmission.init_gear_ratios();
}

fn parse_f32(value: &str, name: &str, index: usize) -> Result<f32, String> {
    let parsed: f32 = value
        .parse()
        .map_err(|err| format!("invalid f32 handling field {index} for {name}: {err}"))?;
    if !parsed.is_finite() {
        return Err(format!("non-finite handling field {index} for {name}"));
    }
    Ok(parsed)
}

fn parse_i8(value: &str, name: &str, index: usize) -> Result<i8, String> {
    value
        .parse()
        .map_err(|err| format!("invalid i8 handling field {index} for {name}: {err}"))
}

fn parse_i32(value: &str, name: &str, index: usize) -> Result<i32, String> {
    value
        .parse()
        .map_err(|err| format!("invalid i32 handling field {index} for {name}: {err}"))
}

#[cfg(test)]
mod required_input_tests {
    use super::*;

    #[test]
    fn truncated_native_record_is_rejected_before_default_values_are_converted() {
        let cfg = HandlingCfg::parse("KURUMA 1500 1.9");
        let error = cHandlingDataMgr::new().initialise_from_cfg(&cfg).unwrap_err();
        assert!(error.contains("missing fields for KURUMA"));
    }

    #[test]
    fn non_finite_handling_numbers_cannot_enter_native_physics() {
        for value in ["NaN", "inf", "-inf"] {
            assert!(parse_f32(value, "KURUMA", 0).is_err());
        }
        assert_eq!(parse_f32("1500", "KURUMA", 0).unwrap(), 1500.0);
    }
}
