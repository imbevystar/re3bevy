//! re3 reference: references/re-GTA/src/peds/PedType.cpp
//! re3 reference: references/re-GTA/src/peds/PedType.h
//!
//! `CPedType`/`CPedStats` data tables and ped-type save payload.

use std::io;

use bevy::prelude::Resource;
use rust_gta_3_assets::install::FileMgr::CFileMgr;

use crate::save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ePedType {
    PEDTYPE_PLAYER1 = 0,
    PEDTYPE_PLAYER2,
    PEDTYPE_PLAYER3,
    PEDTYPE_PLAYER4,
    PEDTYPE_CIVMALE,
    PEDTYPE_CIVFEMALE,
    PEDTYPE_COP,
    PEDTYPE_GANG1,
    PEDTYPE_GANG2,
    PEDTYPE_GANG3,
    PEDTYPE_GANG4,
    PEDTYPE_GANG5,
    PEDTYPE_GANG6,
    PEDTYPE_GANG7,
    PEDTYPE_GANG8,
    PEDTYPE_GANG9,
    PEDTYPE_EMERGENCY,
    PEDTYPE_FIREMAN,
    PEDTYPE_CRIMINAL,
    PEDTYPE_UNUSED1,
    PEDTYPE_PROSTITUTE,
    PEDTYPE_SPECIAL,
    PEDTYPE_UNUSED2,
}

/// SCM ped-type argument -> re3 `ePedType` mapping used by script-created
/// characters. Keeping this next to `ePedType` avoids duplicating the numeric
/// contract in the game orchestrator.
pub(crate) fn script_ped_type(value: i32) -> ePedType {
    match value {
        0 => ePedType::PEDTYPE_PLAYER1,
        1 => ePedType::PEDTYPE_PLAYER2,
        2 => ePedType::PEDTYPE_PLAYER3,
        3 => ePedType::PEDTYPE_PLAYER4,
        5 => ePedType::PEDTYPE_CIVFEMALE,
        6 => ePedType::PEDTYPE_COP,
        7 => ePedType::PEDTYPE_GANG1,
        8 => ePedType::PEDTYPE_GANG2,
        9 => ePedType::PEDTYPE_GANG3,
        10 => ePedType::PEDTYPE_GANG4,
        11 => ePedType::PEDTYPE_GANG5,
        12 => ePedType::PEDTYPE_GANG6,
        13 => ePedType::PEDTYPE_GANG7,
        14 => ePedType::PEDTYPE_GANG8,
        15 => ePedType::PEDTYPE_GANG9,
        16 => ePedType::PEDTYPE_EMERGENCY,
        17 => ePedType::PEDTYPE_FIREMAN,
        18 => ePedType::PEDTYPE_CRIMINAL,
        20 => ePedType::PEDTYPE_PROSTITUTE,
        21 => ePedType::PEDTYPE_SPECIAL,
        _ => ePedType::PEDTYPE_CIVMALE,
    }
}

pub const NUM_PEDTYPES: usize = 23;
pub const PED_TYPE_SAVE_RECORD_SIZE: usize = 0x20;
pub const PED_TYPE_SAVE_SIZE: usize = SAVE_HEADER_SIZE + PED_TYPE_SAVE_RECORD_SIZE * NUM_PEDTYPES;

pub const PED_FLAG_PLAYER1: u32 = 1 << 0;
pub const PED_FLAG_PLAYER2: u32 = 1 << 1;
pub const PED_FLAG_PLAYER3: u32 = 1 << 2;
pub const PED_FLAG_PLAYER4: u32 = 1 << 3;
pub const PED_FLAG_CIVMALE: u32 = 1 << 4;
pub const PED_FLAG_CIVFEMALE: u32 = 1 << 5;
pub const PED_FLAG_COP: u32 = 1 << 6;
pub const PED_FLAG_GANG1: u32 = 1 << 7;
pub const PED_FLAG_GANG2: u32 = 1 << 8;
pub const PED_FLAG_GANG3: u32 = 1 << 9;
pub const PED_FLAG_GANG4: u32 = 1 << 10;
pub const PED_FLAG_GANG5: u32 = 1 << 11;
pub const PED_FLAG_GANG6: u32 = 1 << 12;
pub const PED_FLAG_GANG7: u32 = 1 << 13;
pub const PED_FLAG_GANG8: u32 = 1 << 14;
pub const PED_FLAG_GANG9: u32 = 1 << 15;
pub const PED_FLAG_EMERGENCY: u32 = 1 << 16;
pub const PED_FLAG_PROSTITUTE: u32 = 1 << 17;
pub const PED_FLAG_CRIMINAL: u32 = 1 << 18;
pub const PED_FLAG_SPECIAL: u32 = 1 << 19;
pub const PED_FLAG_GUN: u32 = 1 << 20;
pub const PED_FLAG_COP_CAR: u32 = 1 << 21;
pub const PED_FLAG_FAST_CAR: u32 = 1 << 22;
pub const PED_FLAG_EXPLOSION: u32 = 1 << 23;
pub const PED_FLAG_FIREMAN: u32 = 1 << 24;
pub const PED_FLAG_DEADPEDS: u32 = 1 << 25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPedType {
    pub m_flag: u32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
    pub unknown5: f32,
    pub m_threats: u32,
    pub m_avoid: u32,
}

impl Default for CPedType {
    fn default() -> Self {
        Self {
            m_flag: PED_FLAG_PLAYER1,
            unknown1: 0.0,
            unknown2: 0.0,
            // re3 leaves this member unwritten before LoadPedData. Rust cannot
            // represent an uninitialised float, and valid PED.DAT replaces it.
            unknown3: 0.0,
            unknown4: 0.0,
            unknown5: 0.0,
            m_threats: 0,
            m_avoid: 0,
        }
    }
}

impl CPedType {
    pub fn read_from_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        Ok(Self {
            m_flag: reader.read::<u32>()?,
            unknown1: reader.read::<f32>()?,
            unknown2: reader.read::<f32>()?,
            unknown3: reader.read::<f32>()?,
            unknown4: reader.read::<f32>()?,
            unknown5: reader.read::<f32>()?,
            m_threats: reader.read::<u32>()?,
            m_avoid: reader.read::<u32>()?,
        })
    }

    pub fn write_to_save(self, writer: &mut SaveBufWriter) {
        writer.write(self.m_flag);
        writer.write(self.unknown1);
        writer.write(self.unknown2);
        writer.write(self.unknown3);
        writer.write(self.unknown4);
        writer.write(self.unknown5);
        writer.write(self.m_threats);
        writer.write(self.m_avoid);
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CPedTypeRuntime {
    pub ms_apPedType: [CPedType; NUM_PEDTYPES],
}

impl CPedTypeRuntime {
    pub fn initialise() -> Self {
        Self {
            ms_apPedType: [CPedType::default(); NUM_PEDTYPES],
        }
    }

    /// `CPedType::Initialise` followed by `LoadPedData`.
    pub fn initialise_from_game_data(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let bytes = load_data_file(file_mgr, "PED.DAT")?;
        Self::from_ped_dat(&bytes)
    }

    pub fn from_ped_dat(bytes: &[u8]) -> io::Result<Self> {
        let mut runtime = Self::initialise();
        let text = text_data(bytes, "DATA\\PED.DAT")?;
        let mut current_type: Option<usize> = None;

        for source_line in text.lines() {
            let line = source_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let normalized = line.replace([',', '\t'], " ");
            let fields: Vec<_> = normalized.split_whitespace().collect();
            if fields[0] == "Threat" || fields[0] == "Avoid" {
                let index = current_type.ok_or_else(|| {
                    invalid_data(format!("{} appears before a PED.DAT type", fields[0]))
                })?;
                let flags = fields[1..]
                    .iter()
                    .fold(0, |flags, name| flags | find_ped_flag(name));
                if fields[0] == "Threat" {
                    runtime.ms_apPedType[index].m_threats = flags;
                } else {
                    runtime.ms_apPedType[index].m_avoid = flags;
                }
                continue;
            }
            if fields.len() < 6 {
                return Err(invalid_data(format!("invalid PED.DAT row: {source_line}")));
            }
            let ped_type = find_ped_type(fields[0])
                .ok_or_else(|| invalid_data(format!("unknown PED.DAT ped type: {}", fields[0])))?;
            let parse = |index: usize| {
                fields[index]
                    .parse::<f32>()
                    .map_err(|_| invalid_data(format!("invalid PED.DAT number: {source_line}")))
            };
            let index = ped_type as usize;
            let record = &mut runtime.ms_apPedType[index];
            record.m_flag = find_ped_flag(fields[0]);
            record.unknown1 = parse(1)? / 50.0;
            record.unknown2 = parse(2)? / 50.0;
            record.unknown3 = parse(3)? / 50.0;
            record.unknown4 = parse(4)?;
            record.unknown5 = parse(5)?;
            current_type = Some(index);
        }
        Ok(runtime)
    }

    pub fn FindPedType(name: &str) -> Option<ePedType> {
        find_ped_type(name)
    }

    pub fn FindPedFlag(name: &str) -> u32 {
        find_ped_flag(name)
    }

    pub fn GetFlag(&self, ped_type: usize) -> u32 {
        self.ms_apPedType
            .get(ped_type)
            .map_or(0, |entry| entry.m_flag)
    }

    pub fn GetAvoid(&self, ped_type: usize) -> u32 {
        self.ms_apPedType
            .get(ped_type)
            .map_or(0, |entry| entry.m_avoid)
    }

    pub fn GetThreats(&self, ped_type: usize) -> u32 {
        self.ms_apPedType
            .get(ped_type)
            .map_or(0, |entry| entry.m_threats)
    }

    pub fn SetThreats(&mut self, ped_type: usize, threat: u32) {
        if let Some(entry) = self.ms_apPedType.get_mut(ped_type) {
            entry.m_threats = threat;
        }
    }

    pub fn AddThreat(&mut self, ped_type: usize, threat: u32) {
        if let Some(entry) = self.ms_apPedType.get_mut(ped_type) {
            entry.m_threats |= threat;
        }
    }

    pub fn RemoveThreat(&mut self, ped_type: usize, threat: u32) {
        if let Some(entry) = self.ms_apPedType.get_mut(ped_type) {
            entry.m_threats &= !threat;
        }
    }

    pub fn IsThreat(&self, ped_type: usize, threat: u32) -> bool {
        self.GetThreats(ped_type) & threat != 0
    }

    pub fn save(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write_header(
            [b'P', b'T', b'P', 0],
            (PED_TYPE_SAVE_SIZE - SAVE_HEADER_SIZE) as u32,
        );
        for ped_type in self.ms_apPedType {
            ped_type.write_to_save(&mut writer);
        }
        writer.into_inner()
    }

    pub fn load(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        reader.check_header(
            [b'P', b'T', b'P', 0],
            (PED_TYPE_SAVE_SIZE - SAVE_HEADER_SIZE) as u32,
        )?;
        let mut ped_types = [CPedType::default(); NUM_PEDTYPES];
        for ped_type in &mut ped_types {
            *ped_type = CPedType::read_from_save(&mut reader)?;
        }
        Ok(Self {
            ms_apPedType: ped_types,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ePedStats {
    PEDSTAT_PLAYER = 0,
    PEDSTAT_COP,
    PEDSTAT_MEDIC,
    PEDSTAT_FIREMAN,
    PEDSTAT_GANG1,
    PEDSTAT_GANG2,
    PEDSTAT_GANG3,
    PEDSTAT_GANG4,
    PEDSTAT_GANG5,
    PEDSTAT_GANG6,
    PEDSTAT_GANG7,
    PEDSTAT_STREET_GUY,
    PEDSTAT_SUIT_GUY,
    PEDSTAT_SENSIBLE_GUY,
    PEDSTAT_GEEK_GUY,
    PEDSTAT_OLD_GUY,
    PEDSTAT_TOUGH_GUY,
    PEDSTAT_STREET_GIRL,
    PEDSTAT_SUIT_GIRL,
    PEDSTAT_SENSIBLE_GIRL,
    PEDSTAT_GEEK_GIRL,
    PEDSTAT_OLD_GIRL,
    PEDSTAT_TOUGH_GIRL,
    PEDSTAT_TRAMP_MALE,
    PEDSTAT_TRAMP_FEMALE,
    PEDSTAT_TOURIST,
    PEDSTAT_PROSTITUTE,
    PEDSTAT_CRIMINAL,
    PEDSTAT_BUSKER,
    PEDSTAT_TAXIDRIVER,
    PEDSTAT_PSYCHO,
    PEDSTAT_STEWARD,
    PEDSTAT_SPORTSFAN,
    PEDSTAT_SHOPPER,
    PEDSTAT_OLDSHOPPER,
}

pub const PED_STATS_TYPES: [ePedStats; 35] = [
    ePedStats::PEDSTAT_PLAYER,
    ePedStats::PEDSTAT_COP,
    ePedStats::PEDSTAT_MEDIC,
    ePedStats::PEDSTAT_FIREMAN,
    ePedStats::PEDSTAT_GANG1,
    ePedStats::PEDSTAT_GANG2,
    ePedStats::PEDSTAT_GANG3,
    ePedStats::PEDSTAT_GANG4,
    ePedStats::PEDSTAT_GANG5,
    ePedStats::PEDSTAT_GANG6,
    ePedStats::PEDSTAT_GANG7,
    ePedStats::PEDSTAT_STREET_GUY,
    ePedStats::PEDSTAT_SUIT_GUY,
    ePedStats::PEDSTAT_SENSIBLE_GUY,
    ePedStats::PEDSTAT_GEEK_GUY,
    ePedStats::PEDSTAT_OLD_GUY,
    ePedStats::PEDSTAT_TOUGH_GUY,
    ePedStats::PEDSTAT_STREET_GIRL,
    ePedStats::PEDSTAT_SUIT_GIRL,
    ePedStats::PEDSTAT_SENSIBLE_GIRL,
    ePedStats::PEDSTAT_GEEK_GIRL,
    ePedStats::PEDSTAT_OLD_GIRL,
    ePedStats::PEDSTAT_TOUGH_GIRL,
    ePedStats::PEDSTAT_TRAMP_MALE,
    ePedStats::PEDSTAT_TRAMP_FEMALE,
    ePedStats::PEDSTAT_TOURIST,
    ePedStats::PEDSTAT_PROSTITUTE,
    ePedStats::PEDSTAT_CRIMINAL,
    ePedStats::PEDSTAT_BUSKER,
    ePedStats::PEDSTAT_TAXIDRIVER,
    ePedStats::PEDSTAT_PSYCHO,
    ePedStats::PEDSTAT_STEWARD,
    ePedStats::PEDSTAT_SPORTSFAN,
    ePedStats::PEDSTAT_SHOPPER,
    ePedStats::PEDSTAT_OLDSHOPPER,
];
pub const NUM_PEDSTATS: usize = PED_STATS_TYPES.len();

pub const STAT_PUNCH_ONLY: i16 = 1;
pub const STAT_CAN_KNEE_HEAD: i16 = 2;
pub const STAT_CAN_KICK: i16 = 4;
pub const STAT_CAN_ROUNDHOUSE: i16 = 8;
pub const STAT_NO_DIVE: i16 = 0x10;
pub const STAT_ONE_HIT_KNOCKDOWN: i16 = 0x20;
pub const STAT_SHOPPING_BAGS: i16 = 0x40;
pub const STAT_GUN_PANIC: i16 = 0x80;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPedStats {
    pub m_type: ePedStats,
    pub m_name: [u8; 24],
    pub m_flee_distance: f32,
    pub m_heading_change_rate: f32,
    pub m_fear: i8,
    pub m_temper: i8,
    pub m_lawfulness: i8,
    pub m_sexiness: i8,
    pub m_attack_strength: f32,
    pub m_defend_weakness: f32,
    pub m_flags: i16,
}

impl CPedStats {
    fn default_for_type(m_type: ePedStats) -> Self {
        let mut m_name = [0; 24];
        m_name[8] = b'R'; // exact CPedStats::Initialise constructor write
        Self {
            m_type,
            m_name,
            m_flee_distance: 20.0,
            m_heading_change_rate: 15.0,
            m_fear: 50,
            m_temper: 50,
            m_lawfulness: 50,
            m_sexiness: 50,
            m_attack_strength: 1.0,
            m_defend_weakness: 1.0,
            m_flags: 0,
        }
    }

    pub fn name(&self) -> &str {
        let len = self.m_name.iter().position(|byte| *byte == 0).unwrap_or(24);
        std::str::from_utf8(&self.m_name[..len]).unwrap_or("")
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CPedStatsRuntime {
    pub ms_apPedStats: [CPedStats; NUM_PEDSTATS],
}

impl CPedStatsRuntime {
    /// `CPedStats::Initialise` followed by `LoadPedStats`.
    pub fn initialise(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let bytes = load_data_file(file_mgr, "PEDSTATS.DAT")?;
        Self::from_pedstats_dat(&bytes)
    }

    pub fn from_pedstats_dat(bytes: &[u8]) -> io::Result<Self> {
        let mut runtime = Self {
            ms_apPedStats: std::array::from_fn(|index| {
                CPedStats::default_for_type(PED_STATS_TYPES[index])
            }),
        };
        let text = text_data(bytes, "DATA\\PEDSTATS.DAT")?;
        let mut record = 0usize;
        for source_line in text.lines() {
            let line = source_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if record >= NUM_PEDSTATS {
                return Err(invalid_data("PEDSTATS.DAT has more than 35 records"));
            }
            let normalized = line.replace([',', '\t'], " ");
            let fields: Vec<_> = normalized.split_whitespace().collect();
            if fields.len() < 10 {
                return Err(invalid_data(format!(
                    "invalid PEDSTATS.DAT row: {source_line}"
                )));
            }
            let parse_f32 = |index: usize| {
                fields[index].parse::<f32>().map_err(|_| {
                    invalid_data(format!("invalid PEDSTATS.DAT number: {source_line}"))
                })
            };
            let parse_i8 = |index: usize| {
                fields[index].parse::<i8>().map_err(|_| {
                    invalid_data(format!("invalid PEDSTATS.DAT number: {source_line}"))
                })
            };
            let mut name = [0; 24];
            let source_name = fields[0].as_bytes();
            let copy_len = source_name.len().min(name.len());
            name[..copy_len].copy_from_slice(&source_name[..copy_len]);
            let stats = &mut runtime.ms_apPedStats[record];
            stats.m_name = name;
            stats.m_flee_distance = parse_f32(1)?;
            stats.m_heading_change_rate = parse_f32(2)?;
            stats.m_fear = parse_i8(3)?;
            stats.m_temper = parse_i8(4)?;
            stats.m_lawfulness = parse_i8(5)?;
            stats.m_sexiness = parse_i8(6)?;
            stats.m_attack_strength = parse_f32(7)?;
            stats.m_defend_weakness = parse_f32(8)?;
            stats.m_flags = fields[9]
                .parse::<i16>()
                .map_err(|_| invalid_data(format!("invalid PEDSTATS.DAT flags: {source_line}")))?;
            record += 1;
        }
        if record != NUM_PEDSTATS {
            return Err(invalid_data(format!(
                "PEDSTATS.DAT contains {record} records, expected {NUM_PEDSTATS}"
            )));
        }
        Ok(runtime)
    }

    pub fn GetPedStatType(&self, name: &str) -> Option<ePedStats> {
        self.ms_apPedStats
            .iter()
            .find(|stats| stats.name() == name)
            .map(|stats| stats.m_type)
    }

    pub fn GetPedStats(&self, stat_type: ePedStats) -> &CPedStats {
        &self.ms_apPedStats[stat_type as usize]
    }
}

fn load_data_file(file_mgr: &mut CFileMgr, name: &str) -> io::Result<Vec<u8>> {
    file_mgr.set_dir("DATA");
    let result = file_mgr.load_file(name, 16 * 1024, "r");
    file_mgr.set_dir("");
    result
}

fn text_data<'a>(bytes: &'a [u8], name: &str) -> io::Result<&'a str> {
    std::str::from_utf8(bytes.strip_suffix(&[0]).unwrap_or(bytes))
        .map_err(|_| invalid_data(format!("{name} is not text")))
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn find_ped_type(name: &str) -> Option<ePedType> {
    Some(match name {
        "PLAYER1" => ePedType::PEDTYPE_PLAYER1,
        "PLAYER2" => ePedType::PEDTYPE_PLAYER2,
        "PLAYER3" => ePedType::PEDTYPE_PLAYER3,
        "PLAYER4" => ePedType::PEDTYPE_PLAYER4,
        "CIVMALE" => ePedType::PEDTYPE_CIVMALE,
        "CIVFEMALE" => ePedType::PEDTYPE_CIVFEMALE,
        "COP" => ePedType::PEDTYPE_COP,
        "GANG1" => ePedType::PEDTYPE_GANG1,
        "GANG2" => ePedType::PEDTYPE_GANG2,
        "GANG3" => ePedType::PEDTYPE_GANG3,
        "GANG4" => ePedType::PEDTYPE_GANG4,
        "GANG5" => ePedType::PEDTYPE_GANG5,
        "GANG6" => ePedType::PEDTYPE_GANG6,
        "GANG7" => ePedType::PEDTYPE_GANG7,
        "GANG8" => ePedType::PEDTYPE_GANG8,
        "GANG9" => ePedType::PEDTYPE_GANG9,
        "EMERGENCY" => ePedType::PEDTYPE_EMERGENCY,
        "FIREMAN" => ePedType::PEDTYPE_FIREMAN,
        "CRIMINAL" => ePedType::PEDTYPE_CRIMINAL,
        "PROSTITUTE" => ePedType::PEDTYPE_PROSTITUTE,
        "SPECIAL" => ePedType::PEDTYPE_SPECIAL,
        _ => return None,
    })
}

fn find_ped_flag(name: &str) -> u32 {
    match name {
        "PLAYER1" => PED_FLAG_PLAYER1,
        "PLAYER2" => PED_FLAG_PLAYER2,
        "PLAYER3" => PED_FLAG_PLAYER3,
        "PLAYER4" => PED_FLAG_PLAYER4,
        "CIVMALE" => PED_FLAG_CIVMALE,
        "CIVFEMALE" => PED_FLAG_CIVFEMALE,
        "COP" => PED_FLAG_COP,
        "GANG1" => PED_FLAG_GANG1,
        "GANG2" => PED_FLAG_GANG2,
        "GANG3" => PED_FLAG_GANG3,
        "GANG4" => PED_FLAG_GANG4,
        "GANG5" => PED_FLAG_GANG5,
        "GANG6" => PED_FLAG_GANG6,
        "GANG7" => PED_FLAG_GANG7,
        "GANG8" => PED_FLAG_GANG8,
        "GANG9" => PED_FLAG_GANG9,
        "EMERGENCY" => PED_FLAG_EMERGENCY,
        "FIREMAN" => PED_FLAG_FIREMAN,
        "CRIMINAL" => PED_FLAG_CRIMINAL,
        "SPECIAL" => PED_FLAG_SPECIAL,
        "GUN" => PED_FLAG_GUN,
        "COP_CAR" => PED_FLAG_COP_CAR,
        "FAST_CAR" => PED_FLAG_FAST_CAR,
        "EXPLOSION" => PED_FLAG_EXPLOSION,
        "PROSTITUTE" => PED_FLAG_PROSTITUTE,
        "DEADPEDS" => PED_FLAG_DEADPEDS,
        _ => 0,
    }
}
