//! re3 reference: references/re-GTA/src/core/Zones.cpp
//! re3 reference: references/re-GTA/src/core/Zones.h
//!
//! `CTheZones` state and compatible save block layout.

use bevy::prelude::*;
use rustgta3_core::core::GameState::ELevelName;

use crate::{control::Script::ScriptGameplayBridge, core::FileLoader::CFileLoader};

use crate::save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter};

pub const NUMAUDIOZONES: usize = 36;
pub const NUMZONES: usize = 50;
pub const NUMMAPZONES: usize = 25;
pub const NUM_GANGS: usize = 9;
pub const ZONE_NAME_LEN: usize = 8;
pub const CZONE_SAVE_SIZE: usize = 0x38;
pub const CZONEINFO_SAVE_SIZE: usize = 0x3A;
pub const ZONEARRAY_SAVE_SIZE: usize = NUMZONES * CZONE_SAVE_SIZE;
pub const MAPZONEARRAY_SAVE_SIZE: usize = NUMMAPZONES * CZONE_SAVE_SIZE;
pub const ZONES_SAVE_SIZE: usize = SAVE_HEADER_SIZE
    + 4
    + 4
    + 2
    + 2
    + ZONEARRAY_SAVE_SIZE
    + 2 * NUMZONES * CZONEINFO_SAVE_SIZE
    + 2
    + 2
    + MAPZONEARRAY_SAVE_SIZE
    + NUMAUDIOZONES * 2
    + 2
    + 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EZoneType {
    Default = 0,
    Navig = 1,
    Info = 2,
    Mapzone = 3,
}

impl EZoneType {
    fn from_re3(value: i32) -> Self {
        match value {
            1 => Self::Navig,
            2 => Self::Info,
            3 => Self::Mapzone,
            _ => Self::Default,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CZone {
    pub name: [u8; ZONE_NAME_LEN],
    pub minx: f32,
    pub miny: f32,
    pub minz: f32,
    pub maxx: f32,
    pub maxy: f32,
    pub maxz: f32,
    pub zone_type: EZoneType,
    pub level: ELevelName,
    pub zoneinfo_day: i16,
    pub zoneinfo_night: i16,
    pub child: i32,
    pub parent: i32,
    pub next: i32,
}

impl CZone {
    pub const fn empty() -> Self {
        Self {
            name: [0; ZONE_NAME_LEN],
            minx: 0.0,
            miny: 0.0,
            minz: 0.0,
            maxx: 0.0,
            maxy: 0.0,
            maxz: 0.0,
            zone_type: EZoneType::Default,
            level: ELevelName::Generic,
            zoneinfo_day: 0,
            zoneinfo_night: 0,
            child: -1,
            parent: -1,
            next: -1,
        }
    }

    pub const fn mapzone_empty() -> Self {
        Self {
            zone_type: EZoneType::Mapzone,
            ..Self::empty()
        }
    }

    fn city_zone() -> Self {
        let mut zone = Self::empty();
        zone.name = *b"CITYZON\0";
        zone.minx = -4000.0;
        zone.miny = -4000.0;
        zone.minz = -500.0;
        zone.maxx = 4000.0;
        zone.maxy = 4000.0;
        zone.maxz = 500.0;
        zone.level = ELevelName::Generic;
        zone
    }

    fn the_map_zone() -> Self {
        let mut zone = Self::mapzone_empty();
        zone.name = *b"THEMAP\0\0";
        zone.minx = -4000.0;
        zone.miny = -4000.0;
        zone.minz = -500.0;
        zone.maxx = 4000.0;
        zone.maxy = 4000.0;
        zone.maxz = 500.0;
        zone.level = ELevelName::Generic;
        zone
    }

    fn write_save(self, writer: &mut SaveBufWriter) {
        for byte in self.name {
            writer.write(byte);
        }
        writer.write(self.minx);
        writer.write(self.miny);
        writer.write(self.minz);
        writer.write(self.maxx);
        writer.write(self.maxy);
        writer.write(self.maxz);
        writer.write(self.zone_type as i32);
        writer.write(level_to_re3(self.level));
        writer.write(self.zoneinfo_day);
        writer.write(self.zoneinfo_night);
        writer.write(self.child);
        writer.write(self.parent);
        writer.write(self.next);
    }

    fn read_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let mut name = [0_u8; ZONE_NAME_LEN];
        for byte in &mut name {
            *byte = reader.read::<u8>()?;
        }
        Ok(Self {
            name,
            minx: reader.read::<f32>()?,
            miny: reader.read::<f32>()?,
            minz: reader.read::<f32>()?,
            maxx: reader.read::<f32>()?,
            maxy: reader.read::<f32>()?,
            maxz: reader.read::<f32>()?,
            zone_type: EZoneType::from_re3(reader.read::<i32>()?),
            level: level_from_re3(reader.read::<i32>()?),
            zoneinfo_day: reader.read::<i16>()?,
            zoneinfo_night: reader.read::<i16>()?,
            child: reader.read::<i32>()?,
            parent: reader.read::<i32>()?,
            next: reader.read::<i32>()?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CZoneInfo {
    pub car_density: i16,
    pub car_threshold: [i16; 6],
    pub cop_threshold: i16,
    pub gang_threshold: [i16; NUM_GANGS],
    pub ped_density: u16,
    pub cop_density: u16,
    pub gang_density: [u16; NUM_GANGS],
    pub ped_group: u16,
}

impl CZoneInfo {
    pub fn init() -> Self {
        let step = 1000 / 6;
        let mut car_threshold = [0_i16; 6];
        car_threshold[0] = step;
        car_threshold[1] = car_threshold[0] + step;
        car_threshold[2] = car_threshold[1] + step;
        car_threshold[3] = car_threshold[2] + step;
        car_threshold[4] = car_threshold[3];
        car_threshold[5] = car_threshold[4];
        let cop_threshold = car_threshold[5] + step;
        Self {
            car_density: 10,
            car_threshold,
            cop_threshold,
            gang_threshold: [cop_threshold; NUM_GANGS],
            ped_density: 0,
            cop_density: 0,
            gang_density: [0; NUM_GANGS],
            ped_group: 0,
        }
    }

    fn write_save(self, writer: &mut SaveBufWriter) {
        writer.write(self.car_density);
        for value in self.car_threshold {
            writer.write(value);
        }
        writer.write(self.cop_threshold);
        for value in self.gang_threshold {
            writer.write(value);
        }
        writer.write(self.ped_density);
        writer.write(self.cop_density);
        for value in self.gang_density {
            writer.write(value);
        }
        writer.write(self.ped_group);
    }

    fn read_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let car_density = reader.read::<i16>()?;
        let mut car_threshold = [0_i16; 6];
        for value in &mut car_threshold {
            *value = reader.read::<i16>()?;
        }
        let cop_threshold = reader.read::<i16>()?;
        let mut gang_threshold = [0_i16; NUM_GANGS];
        for value in &mut gang_threshold {
            *value = reader.read::<i16>()?;
        }
        let ped_density = reader.read::<u16>()?;
        let cop_density = reader.read::<u16>()?;
        let mut gang_density = [0_u16; NUM_GANGS];
        for value in &mut gang_density {
            *value = reader.read::<u16>()?;
        }
        Ok(Self {
            car_density,
            car_threshold,
            cop_threshold,
            gang_threshold,
            ped_density,
            cop_density,
            gang_density,
            ped_group: reader.read::<u16>()?,
        })
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CTheZones {
    pub players_zone: i32,
    pub find_index: i16,
    pub number_of_audio_zones: u16,
    pub audio_zone_array: [i16; NUMAUDIOZONES],
    pub total_number_of_map_zones: u16,
    pub total_number_of_zones: u16,
    pub zone_array: [CZone; NUMZONES],
    pub map_zone_array: [CZone; NUMMAPZONES],
    pub total_number_of_zone_infos: u16,
    pub zone_info_array: [CZoneInfo; 2 * NUMZONES],
    pub curr_level: ELevelName,
}

impl CTheZones {
    pub fn init() -> Self {
        let mut zone_array = [CZone::empty(); NUMZONES];
        zone_array[0] = CZone::city_zone();
        let mut map_zone_array = [CZone::mapzone_empty(); NUMMAPZONES];
        map_zone_array[0] = CZone::the_map_zone();
        Self {
            players_zone: 0,
            find_index: 0,
            number_of_audio_zones: 0,
            audio_zone_array: [-1; NUMAUDIOZONES],
            total_number_of_map_zones: 1,
            total_number_of_zones: 1,
            zone_array,
            map_zone_array,
            total_number_of_zone_infos: 1,
            zone_info_array: [CZoneInfo::init(); 2 * NUMZONES],
            curr_level: ELevelName::Generic,
        }
    }

    pub fn save_all_zones(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write_header(*b"ZNS\0", (ZONES_SAVE_SIZE - SAVE_HEADER_SIZE) as u32);
        writer.write(self.players_zone);
        writer.write(level_to_re3(self.curr_level));
        writer.write(self.find_index);
        writer.write(0_i16);
        for zone in self.zone_array {
            zone.write_save(&mut writer);
        }
        for zone_info in self.zone_info_array {
            zone_info.write_save(&mut writer);
        }
        writer.write(self.total_number_of_zones);
        writer.write(self.total_number_of_zone_infos);
        for zone in self.map_zone_array {
            zone.write_save(&mut writer);
        }
        for zone_index in self.audio_zone_array {
            writer.write(zone_index);
        }
        writer.write(self.total_number_of_map_zones);
        writer.write(self.number_of_audio_zones);
        debug_assert_eq!(writer.offset(), ZONES_SAVE_SIZE);
        writer.into_inner()
    }

    pub fn load_all_zones(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        reader.check_header(*b"ZNS\0", (ZONES_SAVE_SIZE - SAVE_HEADER_SIZE) as u32)?;
        let players_zone = reader.read::<i32>()?;
        let curr_level = level_from_re3(reader.read::<i32>()?);
        let find_index = reader.read::<i16>()?;
        reader.skip(2)?;
        let mut zone_array = [CZone::empty(); NUMZONES];
        for zone in &mut zone_array {
            *zone = CZone::read_save(&mut reader)?;
        }
        let mut zone_info_array = [CZoneInfo::init(); 2 * NUMZONES];
        for zone_info in &mut zone_info_array {
            *zone_info = CZoneInfo::read_save(&mut reader)?;
        }
        let total_number_of_zones = reader.read::<u16>()?;
        let total_number_of_zone_infos = reader.read::<u16>()?;
        let mut map_zone_array = [CZone::mapzone_empty(); NUMMAPZONES];
        for zone in &mut map_zone_array {
            *zone = CZone::read_save(&mut reader)?;
        }
        let mut audio_zone_array = [-1_i16; NUMAUDIOZONES];
        for zone_index in &mut audio_zone_array {
            *zone_index = reader.read::<i16>()?;
        }
        let total_number_of_map_zones = reader.read::<u16>()?;
        let number_of_audio_zones = reader.read::<u16>()?;
        Ok(Self {
            players_zone,
            find_index,
            number_of_audio_zones,
            audio_zone_array,
            total_number_of_map_zones,
            total_number_of_zones,
            zone_array,
            map_zone_array,
            total_number_of_zone_infos,
            zone_info_array,
            curr_level,
        })
    }
}

fn level_to_re3(level: ELevelName) -> i32 {
    level as i32
}

fn level_from_re3(value: i32) -> ELevelName {
    match value {
        -1 => ELevelName::Ignore,
        1 => ELevelName::Industrial,
        2 => ELevelName::Commercial,
        3 => ELevelName::Suburban,
        _ => ELevelName::Generic,
    }
}

// === FAZ10c-zones: moved from core/Game.rs (verbatim, parity-preserving) ===

/// re3 `CTheZones::Update` runs after streaming and before clock/weather/SCM.
/// `CCollision::Update` consumes this exact level later in the same frame.
pub(crate) fn update_zones(
    mut file_loader: ResMut<CFileLoader>,
    bridge: Res<ScriptGameplayBridge>,
) {
    if !bridge.player_spawned {
        return;
    }
    let position = bridge.player_position;
    file_loader
        .world
        .zones
        .update([position.x, position.y, position.z]);
}
