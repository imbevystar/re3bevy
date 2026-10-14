//! re3 reference: references/re-GTA/src/renderer/WaterLevel.cpp
//! re3 reference: references/re-GTA/src/renderer/WaterLevel.h
//!
//! `CWaterLevel::Initialise` binary `waterpro.dat` state.

use std::io;

use bevy::prelude::Resource;
use rustgta3_assets::install::FileMgr::CFileMgr;
use rustgta3_core::core::Coordinates::CVector;

pub const NO_WATER: i8 = -128;
pub const MAX_WATER_LEVELS: usize = 48;
pub const MAX_SMALL_SECTORS: usize = 128;
pub const MAX_LARGE_SECTORS: usize = 64;
pub const MAX_HUGE_SECTORS: usize = 32;
pub const MAX_EXTRAHUGE_SECTORS: usize = 16;
pub const SMALL_SECTOR_SIZE: f32 = 32.0;
pub const LARGE_SECTOR_SIZE: f32 = 64.0;
pub const HUGE_SECTOR_SIZE: f32 = 128.0;
pub const EXTRAHUGE_SECTOR_SIZE: f32 = 256.0;
pub const WATER_START_X: f32 = -2048.0;
pub const WATER_START_Y: f32 = -2048.0;
pub const WATER_END_X: f32 = 2048.0;
pub const WATER_END_Y: f32 = 2048.0;
pub const WATER_Z_OFFSET: f32 = 1.5;

const WATERPRO_DAT: &str = "DATA\\waterpro.dat";
const WATERPRO_LEN: usize = 4
    + MAX_WATER_LEVELS * 4
    + MAX_WATER_LEVELS * 4 * 4
    + MAX_LARGE_SECTORS * MAX_LARGE_SECTORS
    + MAX_SMALL_SECTORS * MAX_SMALL_SECTORS;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CWaterLevel {
    pub no_of_water_levels: i32,
    pub water_zs: [f32; MAX_WATER_LEVELS],
    pub water_rects: [CRect; MAX_WATER_LEVELS],
    pub water_block_list: Vec<i8>,
    pub water_fine_block_list: Vec<i8>,
    pub waves_calculated_this_frame: bool,
    pub geom_used: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterLevelSample {
    pub level: f32,
    pub block: i8,
}

pub use crate::math::Rect::CRect;

#[cfg(test)]
mod canonical_rect_tests {
    use super::*;

    #[test]
    fn native_waterpro_rect_field_order_and_empty_bounds_survive_type_unification() {
        let mut water = CWaterLevel::default();
        assert_eq!(water.water_rects[0], crate::math::Rect::CRect::default());
        water.no_of_water_levels = 1;
        water.water_rects[0] = CRect::new(1.0, 2.0, 3.0, 4.0);
        let bytes = water.to_waterpro_bytes();
        let rect_offset = std::mem::size_of::<i32>() + MAX_WATER_LEVELS * std::mem::size_of::<f32>();
        // waterpro.dat stores left, bottom, right, top, not constructor order.
        let expected = [1.0_f32, 4.0, 3.0, 2.0].into_iter()
            .flat_map(f32::to_le_bytes).collect::<Vec<_>>();
        assert_eq!(&bytes[rect_offset..rect_offset + expected.len()], expected.as_slice());
        assert_eq!(bytes.len(), WATERPRO_LEN);
        let restored = CWaterLevel::from_waterpro_bytes(&bytes).unwrap();
        assert_eq!(restored.water_rects[0], CRect::new(1.0, 2.0, 3.0, 4.0));
    }
}

impl CWaterLevel {
    pub fn initialise(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let bytes = file_mgr.load_file(WATERPRO_DAT, WATERPRO_LEN + 1, "rb")?;
        Self::from_waterpro_bytes(bytes.strip_suffix(&[0]).unwrap_or(&bytes))
    }

    /// Non-MASTER re3 fallback used when `DATA\\waterpro.dat` is absent.
    /// `line_hits_world` is the exact `CWorld::ProcessVerticalLine` /
    /// `ProcessLineOfSight` building+dummy collision query used while removing
    /// fine tiles hidden by land; no synthetic terrain is introduced here.
    pub fn initialise_from_water_dat<F>(
        file_mgr: &mut CFileMgr,
        line_hits_world: F,
    ) -> io::Result<Self>
    where
        F: FnMut(CVector, CVector) -> bool,
    {
        file_mgr.set_dir("");
        let bytes = file_mgr.load_file("DATA\\WATER.DAT", 64 * 1024, "r")?;
        Self::from_water_dat_bytes(&bytes, line_hits_world)
    }

    pub fn from_water_dat_bytes<F>(bytes: &[u8], mut line_hits_world: F) -> io::Result<Self>
    where
        F: FnMut(CVector, CVector) -> bool,
    {
        let mut water = Self::default();
        let text =
            std::str::from_utf8(bytes.strip_suffix(&[0]).unwrap_or(bytes)).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "DATA\\WATER.DAT is not text")
            })?;

        for source_line in text.lines() {
            let line = source_line.trim();
            if line.is_empty() || line.starts_with(';') || line.contains("* ;end of file") {
                continue;
            }
            // CFileLoader::LoadLine turns commas and control characters into
            // spaces before sscanf; WATER.DAT uses both comma and whitespace rows.
            let normalized = line.replace(',', " ");
            let mut fields = normalized.split_whitespace();
            let values = std::array::from_fn::<_, 5, _>(|_| {
                fields.next().and_then(|field| field.parse::<f32>().ok())
            });
            let [Some(z), Some(left), Some(bottom), Some(right), Some(top)] = values else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid DATA\\WATER.DAT row: {source_line}"),
                ));
            };
            if !water.add_water_level(left, bottom, right, top, z) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "DATA\\WATER.DAT has more than 48 water rectangles",
                ));
            }
        }

        water.rasterize_water_rectangles();
        water.remove_land_obscured_tiles(&mut line_hits_world);
        water.remove_isolated_water();
        water.rebuild_coarse_blocks();
        Ok(water)
    }

    /// re3 writes the generated fine/coarse tables back as `waterpro.dat` after
    /// taking the development `WATER.DAT` path. Failure to create the cache is
    /// deliberately non-fatal in the caller, matching the guarded C++ write.
    pub fn write_waterpro_cache(&self, file_mgr: &mut CFileMgr) -> io::Result<()> {
        let fd = file_mgr.open_file_for_writing(WATERPRO_DAT)?;
        let bytes = self.to_waterpro_bytes();
        let result = file_mgr.write(fd, &bytes).and_then(|written| {
            if written == bytes.len() {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "short write while creating DATA\\waterpro.dat",
                ))
            }
        });
        let close_result = file_mgr.close_file(fd);
        result.and(close_result)
    }

    pub fn to_waterpro_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(WATERPRO_LEN);
        bytes.extend_from_slice(&self.no_of_water_levels.to_le_bytes());
        for value in self.water_zs {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for rect in self.water_rects {
            bytes.extend_from_slice(&rect.left.to_le_bytes());
            bytes.extend_from_slice(&rect.bottom.to_le_bytes());
            bytes.extend_from_slice(&rect.right.to_le_bytes());
            bytes.extend_from_slice(&rect.top.to_le_bytes());
        }
        bytes.extend(self.water_block_list.iter().map(|value| *value as u8));
        bytes.extend(self.water_fine_block_list.iter().map(|value| *value as u8));
        bytes
    }

    pub fn from_waterpro_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < WATERPRO_LEN {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "DATA\\waterpro.dat is shorter than CWaterLevel binary layout",
            ));
        }

        let mut offset = 0;
        let no_of_water_levels = read_i32(bytes, &mut offset)?;
        let mut water_zs = [0.0; MAX_WATER_LEVELS];
        for z in &mut water_zs {
            *z = read_f32(bytes, &mut offset)?;
        }

        let mut water_rects = [CRect::default(); MAX_WATER_LEVELS];
        for rect in &mut water_rects {
            rect.left = read_f32(bytes, &mut offset)?;
            rect.bottom = read_f32(bytes, &mut offset)?;
            rect.right = read_f32(bytes, &mut offset)?;
            rect.top = read_f32(bytes, &mut offset)?;
        }

        let water_block_list_len = MAX_LARGE_SECTORS * MAX_LARGE_SECTORS;
        let water_block_list = read_i8_vec(bytes, &mut offset, water_block_list_len)?;
        let water_fine_block_list_len = MAX_SMALL_SECTORS * MAX_SMALL_SECTORS;
        let water_fine_block_list = read_i8_vec(bytes, &mut offset, water_fine_block_list_len)?;

        Ok(Self {
            no_of_water_levels,
            water_zs,
            water_rects,
            water_block_list,
            water_fine_block_list,
            waves_calculated_this_frame: false,
            geom_used: 0,
        })
    }

    pub fn add_water_level(
        &mut self,
        x_left: f32,
        y_bottom: f32,
        x_right: f32,
        y_top: f32,
        level: f32,
    ) -> bool {
        let Ok(index) = usize::try_from(self.no_of_water_levels) else {
            return false;
        };
        if index >= MAX_WATER_LEVELS {
            return false;
        }

        self.water_rects[index] = CRect {
            left: x_left,
            bottom: y_bottom,
            right: x_right,
            top: y_top,
        };
        self.water_zs[index] = level;
        self.no_of_water_levels += 1;
        true
    }

    pub fn water_level_according_to_rectangles(&self, x: f32, y: f32) -> Option<f32> {
        if self.no_of_water_levels <= 0 {
            return None;
        }

        for index in 0..self.active_water_level_count() {
            let rect = self.water_rects[index];
            if x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom {
                return Some(self.water_zs[index]);
            }
        }

        None
    }

    pub fn get_water_level(
        &self,
        x: f32,
        y: f32,
        z: f32,
        dont_check_z: bool,
        time_ms: u32,
        weather_wind: f32,
    ) -> Option<WaterLevelSample> {
        let (sector_x, sector_y) = water_to_huge_sector(x, y)?;
        let block = self.fine_block(sector_x, sector_y)?;
        if block == NO_WATER {
            return None;
        }

        let level_index = usize::try_from(block).ok()?;
        let mut level = *self.water_zs.get(level_index)?;
        let angle = (time_ms & 4095) as f32 * (std::f32::consts::TAU / 4096.0);
        let wave_phase = (((x as i32) & (MAX_HUGE_SECTORS as i32 - 1))
            + ((y as i32) & (MAX_HUGE_SECTORS as i32 - 1))) as f32
            * (std::f32::consts::TAU / MAX_HUGE_SECTORS as f32)
            + angle;
        let wind_factor = weather_wind * 0.7 + 0.3;
        level += wave_phase.sin() * wind_factor;

        if !dont_check_z && (level - z) > 3.0 {
            return None;
        }

        Some(WaterLevelSample { level, block })
    }

    pub fn get_water_level_vec(
        &self,
        coors: CVector,
        dont_check_z: bool,
        time_ms: u32,
        weather_wind: f32,
    ) -> Option<WaterLevelSample> {
        self.get_water_level(
            coors.x,
            coors.y,
            coors.z,
            dont_check_z,
            time_ms,
            weather_wind,
        )
    }

    pub fn get_water_level_no_waves(&self, x: f32, y: f32, _z: f32) -> Option<WaterLevelSample> {
        let (sector_x, sector_y) = water_to_huge_sector(x, y)?;
        let block = self.fine_block(sector_x, sector_y)?;
        if block == NO_WATER {
            return None;
        }

        let level_index = usize::try_from(block).ok()?;
        Some(WaterLevelSample {
            level: *self.water_zs.get(level_index)?,
            block,
        })
    }

    /// Rasterizes the text-file rectangles exactly as the non-MASTER
    /// `CWaterLevel::Initialise` path. re3's FIX_BUGS clamps malformed GTA III
    /// rectangles before indexing the 128x128 table.
    pub fn rasterize_water_rectangles(&mut self) {
        self.water_fine_block_list.fill(NO_WATER);
        for index in 0..self.active_water_level_count() {
            let rect = self.water_rects[index];
            let left = water_huge_x(rect.left).clamp(0, MAX_SMALL_SECTORS as i32 - 1);
            let right = (water_huge_x(rect.right) + 1).clamp(0, MAX_SMALL_SECTORS as i32 - 1);
            let top = water_huge_y(rect.top).clamp(0, MAX_SMALL_SECTORS as i32 - 1);
            let bottom = (water_huge_y(rect.bottom) + 1).clamp(0, MAX_SMALL_SECTORS as i32 - 1);
            for x in left..=right {
                for y in top..=bottom {
                    self.set_fine_block(x as usize, y as usize, index as i8);
                }
            }
        }
    }

    /// re3 samples each fine tile on a 9x9 grid and removes it only when every
    /// in-world water point is hidden by land. The callback returns whether a
    /// world-space segment hits a building or dummy collision model.
    pub fn remove_land_obscured_tiles<F>(&mut self, line_hits_world: &mut F)
    where
        F: FnMut(CVector, CVector) -> bool,
    {
        const WORLD_MIN: f32 = -2000.0;
        const WORLD_MAX: f32 = 2000.0;
        for x in 0..MAX_SMALL_SECTORS {
            let world_x = WATER_START_X + x as f32 * SMALL_SECTOR_SIZE;
            for y in 0..MAX_SMALL_SECTORS {
                let Some(block) = self.fine_block(x, y) else {
                    continue;
                };
                if block < 0 {
                    continue;
                }
                let world_y = WATER_START_Y + y as f32 * SMALL_SECTOR_SIZE;
                let water_z = self.water_zs[block as usize];
                let mut all_blocked = true;
                'samples: for i in 0..=8 {
                    for j in 0..=8 {
                        let point = CVector::new(
                            world_x + i as f32 * (SMALL_SECTOR_SIZE / 8.0),
                            world_y + j as f32 * (SMALL_SECTOR_SIZE / 8.0),
                            water_z,
                        );
                        let inside_world = point.x > WORLD_MIN
                            && point.x < WORLD_MAX
                            && point.y > WORLD_MIN
                            && point.y < WORLD_MAX;
                        if !inside_world
                            || self
                                .water_level_according_to_rectangles(point.x, point.y)
                                .is_some()
                                && !test_visibility_for_fine_water_block(point, line_hits_world)
                        {
                            all_blocked = false;
                            break 'samples;
                        }
                    }
                }
                if all_blocked {
                    self.set_fine_block(x, y, NO_WATER);
                }
            }
        }
    }

    pub fn remove_isolated_water(&mut self) -> usize {
        let mut connected = vec![false; MAX_SMALL_SECTORS * MAX_SMALL_SECTORS];
        connected[0] = true;
        loop {
            let mut keep_going = false;
            for x in 0..MAX_SMALL_SECTORS {
                for y in 0..MAX_SMALL_SECTORS {
                    let offset = x * MAX_SMALL_SECTORS + y;
                    if self.water_fine_block_list[offset] < 0 || connected[offset] {
                        continue;
                    }
                    let touches = (x > 0 && connected[(x - 1) * MAX_SMALL_SECTORS + y])
                        || (y > 0 && connected[x * MAX_SMALL_SECTORS + y - 1])
                        || (x + 1 < MAX_SMALL_SECTORS
                            && connected[(x + 1) * MAX_SMALL_SECTORS + y])
                        || (y + 1 < MAX_SMALL_SECTORS && connected[x * MAX_SMALL_SECTORS + y + 1]);
                    if touches {
                        connected[offset] = true;
                        keep_going = true;
                    }
                }
            }
            if !keep_going {
                break;
            }
        }

        let mut removed = 0;
        for (offset, block) in self.water_fine_block_list.iter_mut().enumerate() {
            if *block >= 0 && !connected[offset] && self.water_zs[*block as usize] == 0.0 {
                *block = NO_WATER;
                removed += 1;
            }
        }
        removed
    }

    pub fn rebuild_coarse_blocks(&mut self) {
        self.water_block_list.fill(NO_WATER);
        for x in 0..MAX_LARGE_SECTORS {
            for y in 0..MAX_LARGE_SECTORS {
                let candidates = [
                    self.fine_block(x * 2, y * 2),
                    self.fine_block(x * 2 + 1, y * 2),
                    self.fine_block(x * 2, y * 2 + 1),
                    self.fine_block(x * 2 + 1, y * 2 + 1),
                ];
                let block = candidates.into_iter().flatten().find(|block| *block >= 0);
                self.water_block_list[x * MAX_LARGE_SECTORS + y] = block.unwrap_or(NO_WATER);
            }
        }
    }

    pub fn calc_distance_to_water(&self, x: f32, y: f32) -> f32 {
        const SECTOR_MAX_RENDER_DIST: f32 = 75.0;

        let mut start_x = water_to_small_sector_x(x - SECTOR_MAX_RENDER_DIST) - 1;
        let mut end_x = water_to_small_sector_x(x + SECTOR_MAX_RENDER_DIST) + 1;
        let mut start_y = water_to_small_sector_y(y - SECTOR_MAX_RENDER_DIST) - 1;
        let mut end_y = water_to_small_sector_y(y + SECTOR_MAX_RENDER_DIST) + 1;

        start_x = start_x.clamp(0, MAX_SMALL_SECTORS as i32 - 1);
        end_x = end_x.clamp(0, MAX_SMALL_SECTORS as i32 - 1);
        start_y = start_y.clamp(0, MAX_SMALL_SECTORS as i32 - 1);
        end_y = end_y.clamp(0, MAX_SMALL_SECTORS as i32 - 1);

        let mut dist_sqr = 1.0e10_f32;
        for sector_x in start_x..=end_x {
            for sector_y in start_y..=end_y {
                let Some(block) = self.fine_block(sector_x as usize, sector_y as usize) else {
                    continue;
                };
                if block < 0 {
                    continue;
                }

                let sector_world_x = water_from_small_sector_x(sector_x);
                let sector_world_y = water_from_small_sector_y(sector_y);
                let dx = sector_world_x + SMALL_SECTOR_SIZE - x;
                let dy = sector_world_y + SMALL_SECTOR_SIZE - y;
                dist_sqr = dist_sqr.min(dx * dx + dy * dy);
            }
        }

        (dist_sqr.sqrt() - 23.0).clamp(0.0, SECTOR_MAX_RENDER_DIST)
    }

    fn active_water_level_count(&self) -> usize {
        self.no_of_water_levels.clamp(0, MAX_WATER_LEVELS as i32) as usize
    }

    fn fine_block(&self, sector_x: usize, sector_y: usize) -> Option<i8> {
        self.water_fine_block_list
            .get(
                sector_x
                    .checked_mul(MAX_SMALL_SECTORS)?
                    .checked_add(sector_y)?,
            )
            .copied()
    }

    fn set_fine_block(&mut self, sector_x: usize, sector_y: usize, block: i8) {
        if let Some(slot) = self
            .water_fine_block_list
            .get_mut(sector_x * MAX_SMALL_SECTORS + sector_y)
        {
            *slot = block;
        }
    }
}

fn test_visibility_for_fine_water_block<F>(point: CVector, line_hits_world: &mut F) -> bool
where
    F: FnMut(CVector, CVector) -> bool,
{
    const OFFSETS: [(f32, f32); 8] = [
        (50.0, 50.0),
        (-50.0, 50.0),
        (-50.0, -50.0),
        (50.0, -50.0),
        (50.0, 0.0),
        (-50.0, 0.0),
        (0.0, -50.0),
        (0.0, 50.0),
    ];

    let vertical_end = CVector::new(point.x, point.y, point.z + 100.0);
    if !line_hits_world(point, vertical_end) {
        let shifted = CVector::new(point.x + 0.4, point.y + 0.4, point.z);
        let shifted_end = CVector::new(shifted.x, shifted.y, shifted.z + 100.0);
        if !line_hits_world(shifted, shifted_end) {
            return false;
        }
    }

    for (dx, dy) in OFFSETS {
        let end = CVector::new(point.x + dx, point.y + dy, point.z + 100.0);
        if end.x > -2000.0
            && end.x < 2000.0
            && end.y > -2000.0
            && end.y < 2000.0
            && !line_hits_world(point, end)
        {
            let shifted_start = CVector::new(point.x + 0.4, point.y + 0.4, point.z);
            let shifted_end = CVector::new(end.x + 0.4, end.y + 0.4, end.z);
            if !line_hits_world(shifted_start, shifted_end) {
                return false;
            }
        }
    }
    true
}

impl Default for CWaterLevel {
    fn default() -> Self {
        Self {
            no_of_water_levels: 0,
            water_zs: [0.0; MAX_WATER_LEVELS],
            water_rects: [CRect::default(); MAX_WATER_LEVELS],
            water_block_list: vec![NO_WATER; MAX_LARGE_SECTORS * MAX_LARGE_SECTORS],
            water_fine_block_list: vec![NO_WATER; MAX_SMALL_SECTORS * MAX_SMALL_SECTORS],
            waves_calculated_this_frame: false,
            geom_used: 0,
        }
    }
}


fn read_i32(bytes: &[u8], offset: &mut usize) -> io::Result<i32> {
    let raw = read_exact::<4>(bytes, offset)?;
    Ok(i32::from_le_bytes(raw))
}

fn read_f32(bytes: &[u8], offset: &mut usize) -> io::Result<f32> {
    let raw = read_exact::<4>(bytes, offset)?;
    Ok(f32::from_le_bytes(raw))
}

fn read_i8_vec(bytes: &[u8], offset: &mut usize, len: usize) -> io::Result<Vec<i8>> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "waterpro offset overflow"))?;
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "waterpro array truncated"))?;
    *offset = end;
    Ok(slice.iter().map(|byte| *byte as i8).collect())
}

fn read_exact<const N: usize>(bytes: &[u8], offset: &mut usize) -> io::Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "waterpro offset overflow"))?;
    let raw = bytes
        .get(*offset..end)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "waterpro field truncated"))?;
    *offset = end;
    raw.try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid waterpro field"))
}

fn water_huge_x(x: f32) -> i32 {
    ((x - WATER_START_X) / MAX_HUGE_SECTORS as f32) as i32
}

fn water_huge_y(y: f32) -> i32 {
    ((y - WATER_START_Y) / MAX_HUGE_SECTORS as f32) as i32
}

fn water_to_small_sector_x(x: f32) -> i32 {
    ((x - WATER_START_X) / SMALL_SECTOR_SIZE) as i32
}

fn water_to_small_sector_y(y: f32) -> i32 {
    ((y - WATER_START_Y) / SMALL_SECTOR_SIZE) as i32
}

fn water_to_huge_sector(x: f32, y: f32) -> Option<(usize, usize)> {
    let sector_x = ((x - WATER_START_X) / SMALL_SECTOR_SIZE) as i32;
    let sector_y = ((y - WATER_START_Y) / SMALL_SECTOR_SIZE) as i32;
    if sector_x < 0
        || sector_y < 0
        || sector_x >= MAX_SMALL_SECTORS as i32
        || sector_y >= MAX_SMALL_SECTORS as i32
    {
        return None;
    }

    Some((sector_x as usize, sector_y as usize))
}

fn water_from_small_sector_x(x: i32) -> f32 {
    (x as f32 - MAX_SMALL_SECTORS as f32 / 2.0) * SMALL_SECTOR_SIZE
}

fn water_from_small_sector_y(y: i32) -> f32 {
    (y as f32 - MAX_SMALL_SECTORS as f32 / 2.0) * SMALL_SECTOR_SIZE
}
