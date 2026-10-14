//! re3 reference: references/re-GTA/src/peds/Gangs.cpp
//! re3 reference: references/re-GTA/src/peds/Gangs.h
//!
//! `CGangs::Initialise` state.

use bevy::prelude::Resource;

use crate::modelinfo::ModelIndices::{
    MI_BELLYUP, MI_BUS, MI_COLUMB, MI_DIABLOS, MI_HOODS, MI_MAFIA, MI_YAKUZA, MI_YARDIE,
};
use crate::save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter};

pub const NUM_GANGS: usize = 9;
pub const WEAPONTYPE_UNARMED: i32 = 0;
pub const GANG_INFO_SAVE_SIZE: u32 = 0x10;
pub const GANGS_SAVE_SIZE: u32 = SAVE_HEADER_SIZE as u32 + GANG_INFO_SAVE_SIZE * NUM_GANGS as u32;

#[derive(Debug, Clone, Resource)]
pub struct CGangs {
    pub gangs: [CGangInfo; NUM_GANGS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CGangInfo {
    pub vehicle_model: i32,
    pub ped_model_override: i8,
    pub weapon1: i32,
    pub weapon2: i32,
}

impl CGangs {
    pub fn initialise() -> Self {
        let mut gangs = [CGangInfo::default(); NUM_GANGS];
        gangs[0].vehicle_model = MI_MAFIA as i32;
        gangs[1].vehicle_model = MI_BELLYUP as i32;
        gangs[2].vehicle_model = MI_DIABLOS as i32;
        gangs[3].vehicle_model = MI_YAKUZA as i32;
        gangs[4].vehicle_model = MI_YARDIE as i32;
        gangs[5].vehicle_model = MI_COLUMB as i32;
        gangs[6].vehicle_model = MI_HOODS as i32;
        gangs[7].vehicle_model = -1;
        gangs[8].vehicle_model = -1;
        Self { gangs }
    }

    /// re3 `CGangs::SetGangVehicleModel`
    pub fn set_gang_vehicle_model(&mut self, gang: i32, model: i32) {
        if let Some(info) = self.gangs.get_mut(gang as usize) {
            info.vehicle_model = model;
        }
    }

    /// re3 `CGangs::SetGangWeapons`
    pub fn set_gang_weapons(&mut self, gang: i32, weapon1: i32, weapon2: i32) {
        if let Some(info) = self.gangs.get_mut(gang as usize) {
            info.weapon1 = weapon1;
            info.weapon2 = weapon2;
        }
    }

    pub fn save_all_gang_data(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write_header(*b"GNG\0", GANGS_SAVE_SIZE - SAVE_HEADER_SIZE as u32);
        for gang in self.gangs {
            write_gang_info(&mut writer, gang);
        }
        writer.into_inner()
    }

    pub fn load_all_gang_data(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut out = Self::initialise();
        let mut reader = SaveBufReader::new(bytes);
        reader.check_header(*b"GNG\0", GANGS_SAVE_SIZE - SAVE_HEADER_SIZE as u32)?;
        for gang in &mut out.gangs {
            *gang = read_gang_info(&mut reader)?;
        }
        Ok(out)
    }
}

impl Default for CGangs {
    fn default() -> Self {
        Self::initialise()
    }
}

impl Default for CGangInfo {
    fn default() -> Self {
        Self {
            vehicle_model: MI_BUS as i32,
            ped_model_override: -1,
            weapon1: WEAPONTYPE_UNARMED,
            weapon2: WEAPONTYPE_UNARMED,
        }
    }
}

fn write_gang_info(writer: &mut SaveBufWriter, gang: CGangInfo) {
    writer.write(gang.vehicle_model);
    writer.write(gang.ped_model_override);
    writer.zero(3);
    writer.write(gang.weapon1);
    writer.write(gang.weapon2);
}

fn read_gang_info(reader: &mut SaveBufReader<'_>) -> Result<CGangInfo, SaveBufError> {
    let vehicle_model = reader.read::<i32>()?;
    let ped_model_override = reader.read::<i8>()?;
    reader.skip(3)?;
    let weapon1 = reader.read::<i32>()?;
    let weapon2 = reader.read::<i32>()?;
    Ok(CGangInfo {
        vehicle_model,
        ped_model_override,
        weapon1,
        weapon2,
    })
}
