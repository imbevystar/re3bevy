//! re3 reference: references/re-GTA/src/control/Restart.cpp
//! re3 reference: references/re-GTA/src/control/Restart.h
//!
//! `CRestart::Initialise` state.

use bevy::prelude::*;
use rustgta3_core::core::{Coordinates::CVector, GameState::ELevelName};

use crate::save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter};

pub const NUM_RESTART_POINTS: usize = 8;
pub const RESTART_SAVE_SIZE: u32 = 292;

#[derive(Debug, Clone, Resource)]
pub struct CRestart {
    pub override_hospital_level: ELevelName,
    pub override_police_station_level: ELevelName,
    pub fade_in_after_next_arrest: bool,
    pub fade_in_after_next_death: bool,
    pub override_restart: bool,
    pub override_position: CVector,
    pub override_heading: f32,
    pub hospital_restart_points: [CVector; NUM_RESTART_POINTS],
    pub hospital_restart_headings: [f32; NUM_RESTART_POINTS],
    pub number_of_hospital_restarts: u16,
    pub police_restart_points: [CVector; NUM_RESTART_POINTS],
    pub police_restart_headings: [f32; NUM_RESTART_POINTS],
    pub number_of_police_restarts: u16,
}

impl CRestart {
    pub fn initialise() -> Self {
        Self {
            override_hospital_level: ELevelName::Generic,
            override_police_station_level: ELevelName::Generic,
            fade_in_after_next_arrest: true,
            fade_in_after_next_death: true,
            override_restart: false,
            override_position: CVector::ZERO,
            override_heading: 0.0,
            hospital_restart_points: [CVector::ZERO; NUM_RESTART_POINTS],
            hospital_restart_headings: [0.0; NUM_RESTART_POINTS],
            number_of_hospital_restarts: 0,
            police_restart_points: [CVector::ZERO; NUM_RESTART_POINTS],
            police_restart_headings: [0.0; NUM_RESTART_POINTS],
            number_of_police_restarts: 0,
        }
    }

    pub fn add_hospital_restart_point(&mut self, pos: CVector, heading: f32) {
        let index = self.number_of_hospital_restarts as usize;
        if index < NUM_RESTART_POINTS {
            self.hospital_restart_points[index] = pos;
            self.hospital_restart_headings[index] = heading;
            self.number_of_hospital_restarts += 1;
        }
    }

    pub fn add_police_restart_point(&mut self, pos: CVector, heading: f32) {
        let index = self.number_of_police_restarts as usize;
        if index < NUM_RESTART_POINTS {
            self.police_restart_points[index] = pos;
            self.police_restart_headings[index] = heading;
            self.number_of_police_restarts += 1;
        }
    }

    pub fn override_next_restart(&mut self, pos: CVector, heading: f32) {
        self.override_restart = true;
        self.override_position = pos;
        self.override_heading = heading;
    }

    pub fn cancel_override_restart(&mut self) {
        self.override_restart = false;
    }

    pub fn find_closest_hospital_restart_point(&mut self, pos: CVector) -> (CVector, f32) {
        self.find_closest_hospital_restart_point_with_level(pos, |_| ELevelName::Generic, None)
    }

    pub fn find_closest_police_restart_point(&mut self, pos: CVector) -> (CVector, f32) {
        self.find_closest_police_restart_point_with_level(pos, |_| ELevelName::Generic, None)
    }

    pub fn find_closest_hospital_restart_point_with_level<F>(
        &mut self,
        pos: CVector,
        level_at: F,
        fallback_path_node: Option<CVector>,
    ) -> (CVector, f32)
    where
        F: Fn(CVector) -> ELevelName,
    {
        if let Some(overridden) = self.consume_override() {
            return overridden;
        }
        Self::find_closest_restart_point_on_level(
            pos,
            &self.hospital_restart_points,
            &self.hospital_restart_headings,
            self.number_of_hospital_restarts,
            self.override_hospital_level,
            level_at,
            fallback_path_node,
        )
    }

    pub fn find_closest_police_restart_point_with_level<F>(
        &mut self,
        pos: CVector,
        level_at: F,
        fallback_path_node: Option<CVector>,
    ) -> (CVector, f32)
    where
        F: Fn(CVector) -> ELevelName,
    {
        if let Some(overridden) = self.consume_override() {
            return overridden;
        }
        Self::find_closest_restart_point_on_level(
            pos,
            &self.police_restart_points,
            &self.police_restart_headings,
            self.number_of_police_restarts,
            self.override_police_station_level,
            level_at,
            fallback_path_node,
        )
    }

    fn consume_override(&mut self) -> Option<(CVector, f32)> {
        if !self.override_restart {
            return None;
        }
        let out = (self.override_position, self.override_heading);
        self.cancel_override_restart();
        Some(out)
    }

    pub fn save_all_restart_points(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write_header(*b"RST\0", RESTART_SAVE_SIZE - SAVE_HEADER_SIZE as u32);
        for index in 0..NUM_RESTART_POINTS {
            write_cvector(&mut writer, self.hospital_restart_points[index]);
            writer.write(self.hospital_restart_headings[index]);
        }
        for index in 0..NUM_RESTART_POINTS {
            write_cvector(&mut writer, self.police_restart_points[index]);
            writer.write(self.police_restart_headings[index]);
        }
        writer.write(self.number_of_hospital_restarts);
        writer.write(self.number_of_police_restarts);
        writer.write(self.override_restart as u8);
        writer.write(0_u8);
        writer.write(0_u16);
        write_cvector(&mut writer, self.override_position);
        writer.write(self.override_heading);
        writer.write(self.fade_in_after_next_death as u8);
        writer.write(self.fade_in_after_next_arrest as u8);
        writer.write(level_to_u8(self.override_hospital_level));
        writer.write(level_to_u8(self.override_police_station_level));
        writer.into_inner()
    }

    pub fn load_all_restart_points(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut out = Self::initialise();
        let mut reader = SaveBufReader::new(bytes);
        reader.check_header(*b"RST\0", RESTART_SAVE_SIZE - SAVE_HEADER_SIZE as u32)?;
        for index in 0..NUM_RESTART_POINTS {
            out.hospital_restart_points[index] = read_cvector(&mut reader)?;
            out.hospital_restart_headings[index] = reader.read::<f32>()?;
        }
        for index in 0..NUM_RESTART_POINTS {
            out.police_restart_points[index] = read_cvector(&mut reader)?;
            out.police_restart_headings[index] = reader.read::<f32>()?;
        }
        out.number_of_hospital_restarts = reader.read::<u16>()?;
        out.number_of_police_restarts = reader.read::<u16>()?;
        out.override_restart = reader.read::<u8>()? != 0;
        reader.skip(3)?;
        out.override_position = read_cvector(&mut reader)?;
        out.override_heading = reader.read::<f32>()?;
        out.fade_in_after_next_death = reader.read::<u8>()? != 0;
        out.fade_in_after_next_arrest = reader.read::<u8>()? != 0;
        out.override_hospital_level = level_from_u8(reader.read::<u8>()?);
        out.override_police_station_level = level_from_u8(reader.read::<u8>()?);
        Ok(out)
    }

    fn find_closest_restart_point_on_level<F>(
        pos: CVector,
        points: &[CVector; NUM_RESTART_POINTS],
        headings: &[f32; NUM_RESTART_POINTS],
        count: u16,
        override_level: ELevelName,
        level_at: F,
        fallback_path_node: Option<CVector>,
    ) -> (CVector, f32)
    where
        F: Fn(CVector) -> ELevelName,
    {
        let wanted_level = if override_level != ELevelName::Generic {
            override_level
        } else {
            level_at(pos)
        };
        let mut min_dist = 4000.0_f32 * 4000.0_f32;
        let mut closest = NUM_RESTART_POINTS;
        for index in 0..count as usize {
            if level_at(points[index]) != wanted_level {
                continue;
            }
            let dist = restart_distance_sqr(pos, points[index]);
            if min_dist >= dist {
                min_dist = dist;
                closest = index;
            }
        }
        if closest == NUM_RESTART_POINTS {
            for index in 0..count as usize {
                let dist = restart_distance_sqr(pos, points[index]);
                if min_dist >= dist {
                    min_dist = dist;
                    closest = index;
                }
            }
        }
        if closest == NUM_RESTART_POINTS {
            (fallback_path_node.unwrap_or(CVector::ZERO), 0.0)
        } else {
            (points[closest], headings[closest])
        }
    }
}

fn restart_distance_sqr(a: CVector, b: CVector) -> f32 {
    let x = a.x - b.x;
    let y = a.y - b.y;
    let z = a.z - b.z;
    x * x + y * y + z * z
}

fn write_cvector(writer: &mut SaveBufWriter, vector: CVector) {
    writer.write(vector.x);
    writer.write(vector.y);
    writer.write(vector.z);
}

fn read_cvector(reader: &mut SaveBufReader<'_>) -> Result<CVector, SaveBufError> {
    Ok(CVector {
        x: reader.read::<f32>()?,
        y: reader.read::<f32>()?,
        z: reader.read::<f32>()?,
    })
}

fn level_to_u8(level: ELevelName) -> u8 {
    match level {
        ELevelName::Ignore => 255,
        ELevelName::Generic => 0,
        ELevelName::Industrial => 1,
        ELevelName::Commercial => 2,
        ELevelName::Suburban => 3,
    }
}

fn level_from_u8(level: u8) -> ELevelName {
    match level {
        1 => ELevelName::Industrial,
        2 => ELevelName::Commercial,
        3 => ELevelName::Suburban,
        255 => ELevelName::Ignore,
        _ => ELevelName::Generic,
    }
}
