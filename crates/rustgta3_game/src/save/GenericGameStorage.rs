//! re3 reference: references/re-GTA/src/save/GenericGameStorage.cpp
//! re3 reference: references/re-GTA/src/save/GenericGameStorage.h
//!
//! PC save-game block layout, checksum and slot metadata state.

use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

use bevy::prelude::Resource;
use rustgta3_core::core::{Coordinates::CVector, GameState::ELevelName};

use super::{
    Date::CDate,
    SaveBuf::{SaveBufReader, SaveBufWriter, SavePrimitive, align4bytes, checksum_add_aligned},
};

pub const SLOT_COUNT: usize = 8;
pub const BLOCK_COUNT: usize = 20;
pub const SIZE_OF_SIMPLEVARS: usize = 0xBC;
pub const SIZE_OF_ONE_GAME_IN_BYTES: usize = 201_729;
pub const WORK_BUFF_SIZE: usize = 55_000;
pub const TOP_LINE_EMPTY_FILE: &str = "THIS FILE IS NOT VALID YET";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveSlotState {
    SlotOk,
    SlotEmpty,
    SlotCorrupted,
}

impl SaveSlotState {
    pub fn as_re3_pc_slot_value(self) -> i32 {
        match self {
            SaveSlotState::SlotOk => 0,
            SaveSlotState::SlotEmpty => 1,
            SaveSlotState::SlotCorrupted => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveBlockKind {
    SimpleVarsAndScripts,
    PedPool,
    Garages,
    VehiclePool,
    ObjectPool,
    Paths,
    Cranes,
    Pickups,
    PhoneInfo,
    Restart,
    RadarBlips,
    Zones,
    GangData,
    CarGenerators,
    Particles,
    AudioScriptObjects,
    PlayerInfo,
    Stats,
    Streaming,
    PedType,
}

impl SaveBlockKind {
    pub const RE3_ORDER: [SaveBlockKind; BLOCK_COUNT] = [
        SaveBlockKind::SimpleVarsAndScripts,
        SaveBlockKind::PedPool,
        SaveBlockKind::Garages,
        SaveBlockKind::VehiclePool,
        SaveBlockKind::ObjectPool,
        SaveBlockKind::Paths,
        SaveBlockKind::Cranes,
        SaveBlockKind::Pickups,
        SaveBlockKind::PhoneInfo,
        SaveBlockKind::Restart,
        SaveBlockKind::RadarBlips,
        SaveBlockKind::Zones,
        SaveBlockKind::GangData,
        SaveBlockKind::CarGenerators,
        SaveBlockKind::Particles,
        SaveBlockKind::AudioScriptObjects,
        SaveBlockKind::PlayerInfo,
        SaveBlockKind::Stats,
        SaveBlockKind::Streaming,
        SaveBlockKind::PedType,
    ];
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaveSystemTime {
    pub year: u16,
    pub month: u16,
    pub day_of_week: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub milliseconds: u16,
}

impl Default for SaveSystemTime {
    fn default() -> Self {
        Self {
            year: 0,
            month: 0,
            day_of_week: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            milliseconds: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaveSimpleVars {
    pub save_name: String,
    pub save_time: SaveSystemTime,
    pub save_size: i32,
    pub curr_level: ELevelName,
    pub camera_position: CVector,
    pub milliseconds_per_game_minute: i32,
    pub last_clock_tick: i32,
    pub game_clock_hours: u8,
    pub game_clock_minutes: u8,
    pub pad_mode: i16,
    pub timer_ms: u32,
    pub time_scale: f32,
    pub time_step: f32,
    pub time_step_non_clipped: f32,
    pub frame_counter: u32,
    pub timestep_time_step: f32,
    pub frames_per_update: f32,
    pub timestep_time_scale: f32,
    pub old_weather_type: i16,
    pub new_weather_type: i16,
    pub forced_weather_type: i16,
    pub weather_interpolation: f32,
    pub compile_date: CDate,
    pub weather_type_in_list: i16,
    pub car_zoom_indicator: f32,
    pub ped_zoom_indicator: f32,
}

impl Default for SaveSimpleVars {
    fn default() -> Self {
        Self {
            save_name: String::new(),
            save_time: SaveSystemTime::default(),
            save_size: SIZE_OF_ONE_GAME_IN_BYTES as i32,
            curr_level: ELevelName::Industrial,
            camera_position: CVector::default(),
            milliseconds_per_game_minute: 0,
            last_clock_tick: 0,
            game_clock_hours: 0,
            game_clock_minutes: 0,
            pad_mode: 0,
            timer_ms: 0,
            time_scale: 1.0,
            time_step: 0.0,
            time_step_non_clipped: 0.0,
            frame_counter: 0,
            timestep_time_step: 0.0,
            frames_per_update: 0.0,
            timestep_time_scale: 1.0,
            old_weather_type: 0,
            new_weather_type: 0,
            forced_weather_type: -1,
            weather_interpolation: 0.0,
            compile_date: CDate::default(),
            weather_type_in_list: 0,
            car_zoom_indicator: 0.0,
            ped_zoom_indicator: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveBlock {
    pub kind: SaveBlockKind,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFileBlocks {
    pub blocks: Vec<SaveBlock>,
    pub checksum: i32,
    pub bytes_processed: usize,
    pub level_to_load: ELevelName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveStorageError {
    Io(String),
    MissingChecksum,
    ChecksumMismatch { expected: i32, found: i32 },
    BlockTooLarge { size: usize },
    InvalidBlockOrder { index: usize },
    MissingBlock { kind: SaveBlockKind },
    Buffer(String),
    SaveImageTooLarge { size: usize },
    WrongBlockCount { expected: usize, found: usize },
}

impl fmt::Display for SaveStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveStorageError::Io(error) => write!(f, "{error}"),
            SaveStorageError::MissingChecksum => write!(f, "save file ended before checksum"),
            SaveStorageError::ChecksumMismatch { expected, found } => {
                write!(
                    f,
                    "save checksum mismatch expected {expected} found {found}"
                )
            }
            SaveStorageError::BlockTooLarge { size } => write!(f, "save block too large: {size}"),
            SaveStorageError::InvalidBlockOrder { index } => {
                write!(f, "save block order index {index} is invalid")
            }
            SaveStorageError::MissingBlock { kind } => {
                write!(f, "missing save block: {kind:?}")
            }
            SaveStorageError::Buffer(error) => write!(f, "{error}"),
            SaveStorageError::SaveImageTooLarge { size } => {
                write!(f, "save image too large: {size}")
            }
            SaveStorageError::WrongBlockCount { expected, found } => {
                write!(f, "save block count expected {expected} found {found}")
            }
        }
    }
}

impl std::error::Error for SaveStorageError {}

impl From<io::Error> for SaveStorageError {
    fn from(value: io::Error) -> Self {
        SaveStorageError::Io(value.to_string())
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CGenericGameStorage {
    pub DefaultPCSaveFileName: String,
    pub ValidSaveName: String,
    pub LoadFileName: String,
    pub SlotFileName: [String; SLOT_COUNT],
    pub SlotSaveDate: [String; SLOT_COUNT],
    pub CheckSum: i32,
    pub m_LevelToLoad: ELevelName,
    pub Slots: [SaveSlotState; SLOT_COUNT + 1],
    pub b_FoundRecentSavedGameWantToLoad: bool,
    pub JustLoadedDontFadeInYet: bool,
    pub StillToFadeOut: bool,
    pub TimeStartedCountingForFade: u32,
    pub TimeToStayFadedBeforeFadeOut: u32,
    pub SaveFileNameJustSaved: String,
    pub CompileDateAndTime: CDate,
}

impl Default for CGenericGameStorage {
    fn default() -> Self {
        Self {
            DefaultPCSaveFileName: String::new(),
            ValidSaveName: String::new(),
            LoadFileName: String::new(),
            SlotFileName: std::array::from_fn(|_| String::new()),
            SlotSaveDate: std::array::from_fn(|_| String::new()),
            CheckSum: 0,
            m_LevelToLoad: ELevelName::Generic,
            Slots: [SaveSlotState::SlotEmpty; SLOT_COUNT + 1],
            b_FoundRecentSavedGameWantToLoad: false,
            JustLoadedDontFadeInYet: false,
            StillToFadeOut: false,
            TimeStartedCountingForFade: 0,
            TimeToStayFadedBeforeFadeOut: 1750,
            SaveFileNameJustSaved: String::new(),
            CompileDateAndTime: CDate::default(),
        }
    }
}

impl CGenericGameStorage {
    pub fn MakeValidSaveName(&mut self, slot: usize) -> String {
        let name = format!("{}{}.b", self.DefaultPCSaveFileName, slot + 1);
        self.ValidSaveName = name.clone();
        name
    }

    pub fn GetSavedGameDateAndTime(&self, slot: usize) -> Option<&str> {
        self.SlotSaveDate.get(slot).map(String::as_str)
    }

    pub fn GetNameOfSavedGame(&self, slot: usize) -> Option<&str> {
        self.SlotFileName.get(slot).map(String::as_str)
    }

    pub fn DoGameSpecificStuffAfterSucessLoad(&mut self) {
        self.StillToFadeOut = true;
        self.JustLoadedDontFadeInYet = true;
    }

    pub fn CheckDataNotCorrupt(
        &mut self,
        slot: usize,
        base_save_name: &str,
    ) -> Result<bool, SaveStorageError> {
        let path = format!("{}{}.b", base_save_name, slot + 1);
        self.LoadFileName = path.clone();
        match read_save_file_blocks(&path) {
            Ok(blocks) => {
                self.CheckSum = blocks.checksum;
                self.m_LevelToLoad = blocks.level_to_load;
                Ok(true)
            }
            Err(SaveStorageError::ChecksumMismatch { .. }) => Ok(false),
            Err(SaveStorageError::MissingChecksum) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn RestoreForStartLoad(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<SaveSimpleVars, SaveStorageError> {
        let blocks = read_save_file_blocks(path.as_ref())?;
        let simple = read_simple_vars_block(
            blocks
                .blocks
                .first()
                .ok_or(SaveStorageError::InvalidBlockOrder { index: 0 })?
                .data
                .as_slice(),
        )?;
        self.m_LevelToLoad = simple.curr_level;
        Ok(simple)
    }
}

pub fn align4bytes_i32(size: i32) -> usize {
    align4bytes(size.max(0) as usize)
}

pub fn make_space_for_size_in_buffer_pointer(writer: &mut SaveBufWriter) -> usize {
    let presize = writer.offset();
    writer.write(0_u32);
    presize
}

pub fn copy_size_and_prepare_pointer(
    writer: &mut SaveBufWriter,
    presize: usize,
    size: u32,
) -> Result<(), SaveStorageError> {
    let end = presize + 4;
    let bytes = writer
        .as_slice()
        .get(..)
        .ok_or_else(|| SaveStorageError::Buffer("missing save writer buffer".to_owned()))?;
    if end > bytes.len() {
        return Err(SaveStorageError::Buffer(
            "save size pointer outside buffer".to_owned(),
        ));
    }

    let mut data = writer.clone().into_inner();
    data[presize..end].copy_from_slice(&size.to_le_bytes());
    *writer = SaveBufWriter::from_vec(data);
    let aligned_end = end + align4bytes(size as usize);
    if writer.offset() < aligned_end {
        writer.skip(aligned_end - writer.offset());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericLoadBlocks {
    pub simple_vars: SaveSimpleVars,
    pub scripts: Vec<u8>,
    pub subsystem_blocks: Vec<SaveBlock>,
}

pub fn build_simple_vars_and_scripts_block(simple: &SaveSimpleVars, scripts: &[u8]) -> Vec<u8> {
    let mut out = write_simple_vars_prefix(simple);
    out.extend_from_slice(&(scripts.len() as u32).to_le_bytes());
    out.extend_from_slice(scripts);
    out.resize(SIZE_OF_SIMPLEVARS + 4 + align4bytes(scripts.len()), 0);
    out
}

pub fn build_generic_save_blocks(
    simple: &SaveSimpleVars,
    scripts: &[u8],
    subsystem_blocks: impl IntoIterator<Item = SaveBlock>,
) -> Result<Vec<SaveBlock>, SaveStorageError> {
    let mut blocks = vec![SaveBlock {
        kind: SaveBlockKind::SimpleVarsAndScripts,
        data: build_simple_vars_and_scripts_block(simple, scripts),
    }];
    let mut available: Vec<SaveBlock> = subsystem_blocks.into_iter().collect();

    for kind in SaveBlockKind::RE3_ORDER.iter().copied().skip(1) {
        let Some(index) = available.iter().position(|block| block.kind == kind) else {
            return Err(SaveStorageError::MissingBlock { kind });
        };
        blocks.push(available.remove(index));
    }

    Ok(blocks)
}

pub fn split_generic_load_blocks(
    save: &SaveFileBlocks,
) -> Result<GenericLoadBlocks, SaveStorageError> {
    if save.blocks.len() < BLOCK_COUNT {
        return Err(SaveStorageError::WrongBlockCount {
            expected: BLOCK_COUNT,
            found: save.blocks.len(),
        });
    }

    for (index, kind) in SaveBlockKind::RE3_ORDER.iter().copied().enumerate() {
        if save.blocks[index].kind != kind {
            return Err(SaveStorageError::InvalidBlockOrder { index });
        }
    }

    let simple_block = &save.blocks[0].data;
    let simple_vars = read_simple_vars_block(simple_block)?;
    let scripts_size_offset = SIZE_OF_SIMPLEVARS;
    let Some(size_bytes) = simple_block.get(scripts_size_offset..scripts_size_offset + 4) else {
        return Err(SaveStorageError::Buffer(
            "simple vars block ended before nested scripts size".to_owned(),
        ));
    };
    let scripts_size =
        u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]) as usize;
    let scripts_start = scripts_size_offset + 4;
    let scripts_end = scripts_start + scripts_size;
    let Some(scripts) = simple_block.get(scripts_start..scripts_end) else {
        return Err(SaveStorageError::Buffer(
            "simple vars block ended before nested scripts payload".to_owned(),
        ));
    };

    Ok(GenericLoadBlocks {
        simple_vars,
        scripts: scripts.to_vec(),
        subsystem_blocks: save.blocks[1..BLOCK_COUNT].to_vec(),
    })
}

pub fn encode_save_blocks(blocks: &[SaveBlock]) -> (Vec<u8>, i32) {
    let mut out = Vec::new();
    let mut checksum = 0;
    for block in blocks {
        let size = block.data.len() as u32;
        out.extend_from_slice(&size.to_le_bytes());
        let aligned = align4bytes(block.data.len());
        out.extend_from_slice(&block.data);
        out.resize(out.len() + aligned.saturating_sub(block.data.len()), 0);
        let start = out.len() - aligned;
        checksum_add_aligned(&mut checksum, size, &out[start..]);
    }
    out.extend_from_slice(&checksum.to_le_bytes());
    (out, checksum)
}

pub fn encode_pc_save_image(blocks: &[SaveBlock]) -> Result<(Vec<u8>, i32), SaveStorageError> {
    if blocks.len() != BLOCK_COUNT {
        return Err(SaveStorageError::WrongBlockCount {
            expected: BLOCK_COUNT,
            found: blocks.len(),
        });
    }

    let mut out = Vec::new();
    let mut checksum = 0;
    let mut bytes_processed = 0;
    for (index, block) in blocks.iter().enumerate() {
        let expected = SaveBlockKind::RE3_ORDER[index];
        if block.kind != expected {
            return Err(SaveStorageError::InvalidBlockOrder { index });
        }
        append_save_block(&mut out, &mut checksum, &block.data)?;
        bytes_processed += align4bytes(block.data.len());
    }

    let target_payload_bytes = SIZE_OF_ONE_GAME_IN_BYTES - 4;
    if bytes_processed > target_payload_bytes {
        return Err(SaveStorageError::SaveImageTooLarge {
            size: bytes_processed,
        });
    }

    for _ in 0..4 {
        let mut padding_size = align4bytes(target_payload_bytes - bytes_processed);
        if padding_size > WORK_BUFF_SIZE {
            padding_size = WORK_BUFF_SIZE;
        }
        if padding_size > 4 {
            let padding = vec![0_u8; padding_size];
            append_save_block(&mut out, &mut checksum, &padding)?;
            bytes_processed += padding_size;
        }
    }
    out.extend_from_slice(&checksum.to_le_bytes());
    Ok((out, checksum))
}

fn append_save_block(
    out: &mut Vec<u8>,
    checksum: &mut i32,
    data: &[u8],
) -> Result<(), SaveStorageError> {
    if data.len() > SIZE_OF_ONE_GAME_IN_BYTES {
        return Err(SaveStorageError::BlockTooLarge { size: data.len() });
    }

    let size = data.len() as u32;
    out.extend_from_slice(&size.to_le_bytes());
    let aligned = align4bytes(data.len());
    let start = out.len();
    out.extend_from_slice(data);
    out.resize(start + aligned, 0);
    checksum_add_aligned(checksum, size, &out[start..start + aligned]);
    Ok(())
}

pub fn read_save_file_blocks(path: impl AsRef<Path>) -> Result<SaveFileBlocks, SaveStorageError> {
    let bytes = fs::read(path)?;
    parse_save_file_blocks(&bytes)
}

pub fn parse_save_file_blocks(bytes: &[u8]) -> Result<SaveFileBlocks, SaveStorageError> {
    let mut offset = 0;
    let mut blocknum = 0;
    let mut checksum = 0;
    let mut bytes_processed = 0;
    let mut blocks = Vec::new();
    let mut level_to_load = ELevelName::Generic;

    while SIZE_OF_ONE_GAME_IN_BYTES - 4 > bytes_processed && blocknum < 40 {
        let Some(size_bytes) = bytes.get(offset..offset + 4) else {
            return Err(SaveStorageError::MissingChecksum);
        };
        let size = u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]])
            as usize;
        offset += 4;

        let aligned = align4bytes(size);
        if aligned > SIZE_OF_ONE_GAME_IN_BYTES {
            return Err(SaveStorageError::BlockTooLarge { size });
        }
        let Some(payload) = bytes.get(offset..offset + aligned) else {
            return Err(SaveStorageError::MissingChecksum);
        };
        offset += aligned;

        checksum += size_bytes.iter().map(|byte| *byte as i32).sum::<i32>();
        checksum += payload.iter().map(|byte| *byte as i32).sum::<i32>();
        bytes_processed += aligned;

        if blocknum == 0 {
            level_to_load = level_from_simple_vars(payload);
        }

        let kind = SaveBlockKind::RE3_ORDER
            .get(blocknum)
            .copied()
            .unwrap_or(SaveBlockKind::PedType);
        blocks.push(SaveBlock {
            kind,
            data: payload[..size].to_vec(),
        });
        blocknum += 1;
    }

    let Some(checksum_bytes) = bytes.get(offset..offset + 4) else {
        return Err(SaveStorageError::MissingChecksum);
    };
    let found = i32::from_le_bytes([
        checksum_bytes[0],
        checksum_bytes[1],
        checksum_bytes[2],
        checksum_bytes[3],
    ]);
    if checksum != found {
        return Err(SaveStorageError::ChecksumMismatch {
            expected: checksum,
            found,
        });
    }

    Ok(SaveFileBlocks {
        blocks,
        checksum,
        bytes_processed,
        level_to_load,
    })
}

pub fn read_simple_vars_block(bytes: &[u8]) -> Result<SaveSimpleVars, SaveStorageError> {
    let mut reader = SaveBufReader::new(bytes);
    let mut save_name_units = Vec::new();
    for _ in 0..24 {
        let value = reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?;
        if value == 0 {
            break;
        }
        save_name_units.push(value);
    }
    if save_name_units.len() < 24 {
        reader
            .skip((24 - save_name_units.len() - 1) * 2)
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?;
    }
    let save_name = String::from_utf16(&save_name_units).unwrap_or_default();
    let save_time = SaveSystemTime {
        year: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
        month: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
        day_of_week: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
        day: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
        hour: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
        minute: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
        second: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
        milliseconds: reader
            .read::<u16>()
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?,
    };

    let save_size = read_aligned::<i32>(&mut reader)?;
    let curr_level = level_from_i32(read_aligned::<i32>(&mut reader)?);
    let camera_position = CVector {
        x: read_aligned::<f32>(&mut reader)?,
        y: read_aligned::<f32>(&mut reader)?,
        z: read_aligned::<f32>(&mut reader)?,
    };

    let mut out = SaveSimpleVars {
        save_name,
        save_time,
        save_size,
        curr_level,
        camera_position,
        milliseconds_per_game_minute: read_aligned::<i32>(&mut reader)?,
        last_clock_tick: read_aligned::<i32>(&mut reader)?,
        game_clock_hours: read_aligned::<u8>(&mut reader)?,
        game_clock_minutes: read_aligned::<u8>(&mut reader)?,
        pad_mode: read_aligned::<i16>(&mut reader)?,
        timer_ms: read_aligned::<u32>(&mut reader)?,
        time_scale: read_aligned::<f32>(&mut reader)?,
        time_step: read_aligned::<f32>(&mut reader)?,
        time_step_non_clipped: read_aligned::<f32>(&mut reader)?,
        frame_counter: read_aligned::<u32>(&mut reader)?,
        timestep_time_step: read_aligned::<f32>(&mut reader)?,
        frames_per_update: read_aligned::<f32>(&mut reader)?,
        timestep_time_scale: read_aligned::<f32>(&mut reader)?,
        old_weather_type: read_aligned::<i16>(&mut reader)?,
        new_weather_type: read_aligned::<i16>(&mut reader)?,
        forced_weather_type: read_aligned::<i16>(&mut reader)?,
        weather_interpolation: read_aligned::<f32>(&mut reader)?,
        ..SaveSimpleVars::default()
    };
    out.compile_date.m_nSecond = read_aligned::<i32>(&mut reader)?;
    out.compile_date.m_nMinute = read_aligned::<i32>(&mut reader)?;
    out.compile_date.m_nHour = read_aligned::<i32>(&mut reader)?;
    out.compile_date.m_nDay = read_aligned::<i32>(&mut reader)?;
    out.compile_date.m_nMonth = read_aligned::<i32>(&mut reader)?;
    out.compile_date.m_nYear = read_aligned::<i32>(&mut reader)?;
    out.weather_type_in_list = read_aligned::<i16>(&mut reader)?;
    out.car_zoom_indicator = read_aligned::<f32>(&mut reader)?;
    out.ped_zoom_indicator = read_aligned::<f32>(&mut reader)?;
    Ok(out)
}

pub fn write_simple_vars_prefix(simple: &SaveSimpleVars) -> Vec<u8> {
    let mut writer = SaveBufWriter::new();
    let mut name_units = [0_u16; 24];
    for (index, unit) in simple.save_name.encode_utf16().take(23).enumerate() {
        name_units[index] = unit;
    }
    for unit in name_units {
        writer.write(unit);
    }
    writer.write(simple.save_time.year);
    writer.write(simple.save_time.month);
    writer.write(simple.save_time.day_of_week);
    writer.write(simple.save_time.day);
    writer.write(simple.save_time.hour);
    writer.write(simple.save_time.minute);
    writer.write(simple.save_time.second);
    writer.write(simple.save_time.milliseconds);
    write_aligned(&mut writer, simple.save_size);
    write_aligned(&mut writer, level_to_i32(simple.curr_level));
    write_aligned(&mut writer, simple.camera_position.x);
    write_aligned(&mut writer, simple.camera_position.y);
    write_aligned(&mut writer, simple.camera_position.z);
    write_aligned(&mut writer, simple.milliseconds_per_game_minute);
    write_aligned(&mut writer, simple.last_clock_tick);
    write_aligned(&mut writer, simple.game_clock_hours);
    write_aligned(&mut writer, simple.game_clock_minutes);
    write_aligned(&mut writer, simple.pad_mode);
    write_aligned(&mut writer, simple.timer_ms);
    write_aligned(&mut writer, simple.time_scale);
    write_aligned(&mut writer, simple.time_step);
    write_aligned(&mut writer, simple.time_step_non_clipped);
    write_aligned(&mut writer, simple.frame_counter);
    write_aligned(&mut writer, simple.timestep_time_step);
    write_aligned(&mut writer, simple.frames_per_update);
    write_aligned(&mut writer, simple.timestep_time_scale);
    write_aligned(&mut writer, simple.old_weather_type);
    write_aligned(&mut writer, simple.new_weather_type);
    write_aligned(&mut writer, simple.forced_weather_type);
    write_aligned(&mut writer, simple.weather_interpolation);
    write_aligned(&mut writer, simple.compile_date.m_nSecond);
    write_aligned(&mut writer, simple.compile_date.m_nMinute);
    write_aligned(&mut writer, simple.compile_date.m_nHour);
    write_aligned(&mut writer, simple.compile_date.m_nDay);
    write_aligned(&mut writer, simple.compile_date.m_nMonth);
    write_aligned(&mut writer, simple.compile_date.m_nYear);
    write_aligned(&mut writer, simple.weather_type_in_list);
    write_aligned(&mut writer, simple.car_zoom_indicator);
    write_aligned(&mut writer, simple.ped_zoom_indicator);
    writer.into_inner()
}

fn read_aligned<T: SavePrimitive>(reader: &mut SaveBufReader<'_>) -> Result<T, SaveStorageError> {
    let value = reader
        .read::<T>()
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))?;
    let padding = align4bytes(T::SIZE) - T::SIZE;
    if padding > 0 {
        reader
            .skip(padding)
            .map_err(|error| SaveStorageError::Buffer(error.to_string()))?;
    }
    Ok(value)
}

fn write_aligned<T: SavePrimitive>(writer: &mut SaveBufWriter, value: T) {
    writer.write(value);
    let padding = align4bytes(T::SIZE) - T::SIZE;
    if padding > 0 {
        writer.zero(padding);
    }
}

pub fn default_pc_save_file_name(user_files_folder: impl AsRef<Path>) -> String {
    user_files_folder
        .as_ref()
        .join("GTA3sf")
        .to_string_lossy()
        .into_owned()
}

pub fn save_slot_path(base: &str, slot: usize) -> PathBuf {
    PathBuf::from(format!("{}{}.b", base, slot + 1))
}

fn level_from_simple_vars(payload: &[u8]) -> ELevelName {
    payload
        .get(0x44..0x48)
        .map(|bytes| i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .map(level_from_i32)
        .unwrap_or(ELevelName::Generic)
}

fn level_from_i32(level: i32) -> ELevelName {
    match level {
        1 => ELevelName::Industrial,
        2 => ELevelName::Commercial,
        3 => ELevelName::Suburban,
        _ => ELevelName::Generic,
    }
}

fn level_to_i32(level: ELevelName) -> i32 {
    match level {
        ELevelName::Ignore => -1,
        ELevelName::Generic => 0,
        ELevelName::Industrial => 1,
        ELevelName::Commercial => 2,
        ELevelName::Suburban => 3,
    }
}
