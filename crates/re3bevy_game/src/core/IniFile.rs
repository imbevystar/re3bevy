//! re3 reference: references/re-GTA/src/core/IniFile.cpp
//! re3 reference: references/re-GTA/src/core/IniFile.h
//!
//! GTA III's `gta3.ini` contains two positional density multipliers. re3
//! reads the first line as the ped multiplier and the second as the car
//! multiplier; it is not a generic key/value INI parser.

use bevy::prelude::Resource;
use re3bevy_assets::install::FileMgr::CFileMgr;

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct CIniFile {
    pub ped_number_multiplier: f32,
    pub car_number_multiplier: f32,
}

impl Default for CIniFile {
    fn default() -> Self {
        Self {
            ped_number_multiplier: 1.0,
            car_number_multiplier: 1.0,
        }
    }
}

impl CIniFile {
    /// re3 `CIniFile::LoadIniFile`.
    #[allow(non_snake_case)]
    pub fn LoadIniFile(file_mgr: &mut CFileMgr) -> Self {
        let mut ini = Self::default();
        file_mgr.set_dir("");
        let Ok(fd) = file_mgr.open_file("gta3.ini", "r") else {
            return ini;
        };

        let mut line = String::new();
        if file_mgr.read_line(fd, &mut line, 200).unwrap_or(false) {
            ini.ped_number_multiplier = parse_multiplier(&line, ini.ped_number_multiplier);
        }
        if file_mgr.read_line(fd, &mut line, 200).unwrap_or(false) {
            ini.car_number_multiplier = parse_multiplier(&line, ini.car_number_multiplier);
        }
        let _ = file_mgr.close_file(fd);
        ini
    }
}

fn parse_multiplier(line: &str, fallback: f32) -> f32 {
    line.trim()
        .parse::<f32>()
        .unwrap_or(fallback)
        .clamp(0.5, 3.0)
}

#[cfg(test)]
mod tests {
    use super::parse_multiplier;

    #[test]
    fn density_multiplier_matches_re3_clamp() {
        assert_eq!(parse_multiplier("0.1\r\n", 1.0), 0.5);
        assert_eq!(parse_multiplier("1.75\n", 1.0), 1.75);
        assert_eq!(parse_multiplier("9", 1.0), 3.0);
        assert_eq!(parse_multiplier("garbage", 1.0), 1.0);
    }
}
