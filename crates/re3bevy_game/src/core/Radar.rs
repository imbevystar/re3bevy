//! re3 reference: references/re-GTA/src/core/Radar.cpp
//! re3 reference: references/re-GTA/src/core/Radar.h
//!
//! Radar texture streaming, blip table and coordinate transforms.

use crate::rw::TxdStore::CTxdStore;
use crate::save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter};
use re3bevy_core::core::Coordinates::{CVector, CVector2D};

pub const RADAR_MIN_X: f32 = -2000.0;
pub const RADAR_MIN_Y: f32 = -2000.0;
pub const RADAR_NUM_TILES: usize = 8;
pub const RADAR_TILE_SIZE: f32 = 500.0;
pub const RADAR_TXD_COUNT: usize = RADAR_NUM_TILES * RADAR_NUM_TILES;
pub const NUMRADARBLIPS: usize = 32;
pub const RADAR_MIN_RANGE: f32 = 120.0;
pub const RADAR_MAX_RANGE: f32 = 350.0;
pub const RADAR_LEFT: f32 = 40.0;
pub const RADAR_BOTTOM: f32 = 47.0;
pub const RADAR_WIDTH: f32 = 94.0;
pub const RADAR_HEIGHT: f32 = 76.0;
pub const DEFAULT_SCREEN_HEIGHT: f32 = 448.0;
pub const RADAR_TRACE_SAVE_SIZE: u32 = 0x30;
pub const RADAR_BLIPS_SAVE_SIZE: u32 =
    SAVE_HEADER_SIZE as u32 + RADAR_TRACE_SAVE_SIZE * NUMRADARBLIPS as u32;

pub const RADAR_STREAM_FLAGS: u8 = crate::core::Streaming::STREAMFLAGS_DONT_REMOVE
    | crate::core::Streaming::STREAMFLAGS_DEPENDENCY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum eBlipType {
    BLIP_NONE = 0,
    BLIP_CAR = 1,
    BLIP_CHAR = 2,
    BLIP_OBJECT = 3,
    BLIP_COORD = 4,
    BLIP_CONTACT_POINT = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum eBlipDisplay {
    BLIP_DISPLAY_NEITHER = 0,
    BLIP_DISPLAY_MARKER_ONLY = 1,
    BLIP_DISPLAY_BLIP_ONLY = 2,
    BLIP_DISPLAY_BOTH = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum eRadarSprite {
    RADAR_SPRITE_NONE = 0,
    RADAR_SPRITE_ASUKA = 1,
    RADAR_SPRITE_BOMB = 2,
    RADAR_SPRITE_CAT = 3,
    RADAR_SPRITE_CENTRE = 4,
    RADAR_SPRITE_COPCAR = 5,
    RADAR_SPRITE_DON = 6,
    RADAR_SPRITE_EIGHT = 7,
    RADAR_SPRITE_EL = 8,
    RADAR_SPRITE_ICE = 9,
    RADAR_SPRITE_JOEY = 10,
    RADAR_SPRITE_KENJI = 11,
    RADAR_SPRITE_LIZ = 12,
    RADAR_SPRITE_LUIGI = 13,
    RADAR_SPRITE_NORTH = 14,
    RADAR_SPRITE_RAY = 15,
    RADAR_SPRITE_SAL = 16,
    RADAR_SPRITE_SAVE = 17,
    RADAR_SPRITE_SPRAY = 18,
    RADAR_SPRITE_TONY = 19,
    RADAR_SPRITE_WEAPON = 20,
}

impl eRadarSprite {
    pub fn from_re3(value: i32) -> Self {
        match value {
            1 => Self::RADAR_SPRITE_ASUKA,
            2 => Self::RADAR_SPRITE_BOMB,
            3 => Self::RADAR_SPRITE_CAT,
            4 => Self::RADAR_SPRITE_CENTRE,
            5 => Self::RADAR_SPRITE_COPCAR,
            6 => Self::RADAR_SPRITE_DON,
            7 => Self::RADAR_SPRITE_EIGHT,
            8 => Self::RADAR_SPRITE_EL,
            9 => Self::RADAR_SPRITE_ICE,
            10 => Self::RADAR_SPRITE_JOEY,
            11 => Self::RADAR_SPRITE_KENJI,
            12 => Self::RADAR_SPRITE_LIZ,
            13 => Self::RADAR_SPRITE_LUIGI,
            14 => Self::RADAR_SPRITE_NORTH,
            15 => Self::RADAR_SPRITE_RAY,
            16 => Self::RADAR_SPRITE_SAL,
            17 => Self::RADAR_SPRITE_SAVE,
            18 => Self::RADAR_SPRITE_SPRAY,
            19 => Self::RADAR_SPRITE_TONY,
            20 => Self::RADAR_SPRITE_WEAPON,
            _ => Self::RADAR_SPRITE_NONE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum eRadarTraceColour {
    RADAR_TRACE_RED = 0,
    RADAR_TRACE_GREEN = 1,
    RADAR_TRACE_LIGHT_BLUE = 2,
    RADAR_TRACE_GRAY = 3,
    RADAR_TRACE_YELLOW = 4,
    RADAR_TRACE_MAGENTA = 5,
    RADAR_TRACE_CYAN = 6,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct sRadarTrace {
    pub color: u32,
    pub blip_type: eBlipType,
    pub entity_handle: i32,
    pub vec2d_pos: CVector2D,
    pub vec_pos: CVector,
    pub blip_index: u16,
    pub dim: bool,
    pub in_use: bool,
    pub radius: f32,
    pub scale: i16,
    pub blip_display: eBlipDisplay,
    pub radar_sprite: eRadarSprite,
}

impl Default for sRadarTrace {
    fn default() -> Self {
        Self {
            color: 0,
            blip_type: eBlipType::BLIP_NONE,
            entity_handle: 0,
            vec2d_pos: CVector2D::new(0.0, 0.0),
            vec_pos: CVector::ZERO,
            blip_index: 1,
            dim: false,
            in_use: false,
            radius: 0.0,
            scale: 1,
            blip_display: eBlipDisplay::BLIP_DISPLAY_NEITHER,
            radar_sprite: eRadarSprite::RADAR_SPRITE_NONE,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CRadar {
    pub radar_range: f32,
    pub radar_trace: [sRadarTrace; NUMRADARBLIPS],
    pub radar_txd_ids: [Option<usize>; RADAR_TXD_COUNT],
    pub streamed_sections: [bool; RADAR_TXD_COUNT],
    pub cached_cos: f32,
    pub cached_sin: f32,
    pub radar_origin: CVector2D,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadarStreamingPlan {
    pub request_txd_ids: Vec<usize>,
    pub remove_txd_ids: Vec<usize>,
}

impl Default for CRadar {
    fn default() -> Self {
        Self {
            radar_range: 350.0,
            radar_trace: [sRadarTrace::default(); NUMRADARBLIPS],
            radar_txd_ids: [None; RADAR_TXD_COUNT],
            streamed_sections: [false; RADAR_TXD_COUNT],
            cached_cos: 1.0,
            cached_sin: 0.0,
            radar_origin: CVector2D::new(0.0, 0.0),
        }
    }
}

impl CRadar {
    pub fn initialise(&mut self, txd_store: &CTxdStore) {
        self.radar_range = 350.0;
        for trace in &mut self.radar_trace {
            *trace = sRadarTrace::default();
        }
        self.streamed_sections = [false; RADAR_TXD_COUNT];
        for index in 0..RADAR_TXD_COUNT {
            self.radar_txd_ids[index] = txd_store.find_txd_slot(&radar_tex_name(index));
        }
    }

    pub fn calculate_blip_alpha(dist: f32) -> u8 {
        if dist <= 1.0 {
            255
        } else if dist <= 5.0 {
            (128.0 * ((dist - 1.0) / 4.0) + ((1.0 - (dist - 1.0) / 4.0) * 255.0)) as u8
        } else {
            128
        }
    }

    pub fn change_blip_brightness(&mut self, i: i32, bright: i32) {
        if let Some(index) = self.get_actual_blip_array_index(i) {
            self.radar_trace[index].dim = bright != 1;
        }
    }

    pub fn change_blip_colour(&mut self, i: i32, color: i32) {
        if let Some(index) = self.get_actual_blip_array_index(i) {
            self.radar_trace[index].color = color as u32;
        }
    }

    pub fn change_blip_display(&mut self, i: i32, display: eBlipDisplay) {
        if let Some(index) = self.get_actual_blip_array_index(i) {
            self.radar_trace[index].blip_display = display;
        }
    }

    pub fn change_blip_scale(&mut self, i: i32, scale: i32) {
        if let Some(index) = self.get_actual_blip_array_index(i) {
            self.radar_trace[index].scale = scale as i16;
        }
    }

    pub fn clear_blip(&mut self, i: i32) {
        if let Some(index) = self.get_actual_blip_array_index(i) {
            let blip_index = self.radar_trace[index].blip_index;
            self.radar_trace[index] = sRadarTrace {
                blip_index,
                ..sRadarTrace::default()
            };
        }
    }

    pub fn clear_blip_for_entity(&mut self, blip_type: eBlipType, id: i32) {
        for trace in &mut self.radar_trace {
            if trace.blip_type == blip_type && trace.entity_handle == id {
                let blip_index = trace.blip_index;
                *trace = sRadarTrace {
                    blip_index,
                    ..sRadarTrace::default()
                };
            }
        }
    }

    pub fn display_this_blip(&self, counter: usize) -> bool {
        self.radar_trace.get(counter).is_some_and(|trace| {
            trace.in_use && trace.radar_sprite != eRadarSprite::RADAR_SPRITE_NONE
        })
    }

    pub fn get_actual_blip_array_index(&self, i: i32) -> Option<usize> {
        if i == -1 {
            return None;
        }
        let low = (i as u32 & 0xFFFF) as usize;
        if low >= NUMRADARBLIPS {
            return None;
        }
        let high = ((i as u32 & 0xFFFF0000) >> 16) as u16;
        (high == self.radar_trace[low].blip_index).then_some(low)
    }

    pub fn get_new_unique_blip_index(&mut self, i: usize) -> i32 {
        if self.radar_trace[i].blip_index >= u16::MAX - 1 {
            self.radar_trace[i].blip_index = 1;
        } else {
            self.radar_trace[i].blip_index += 1;
        }
        i as i32 | ((self.radar_trace[i].blip_index as i32) << 16)
    }

    pub fn get_radar_trace_colour(color: u32, bright: bool) -> u32 {
        match color {
            0 if bright => 0x712B49FF,
            0 => 0x7F0000FF,
            1 if bright => 0x5FA06AFF,
            1 => 0x007F00FF,
            2 if bright => 0x80A7F3FF,
            2 => 0x00007FFF,
            3 if bright => 0xE1E1E1FF,
            3 => 0x7F7F7FFF,
            4 if bright => 0xFFFF00FF,
            4 => 0x7F7F00FF,
            5 if bright => 0xFF00FFFF,
            5 => 0x7F007FFF,
            6 if bright => 0x00FFFFFF,
            6 => 0x007F7FFF,
            other => other,
        }
    }

    pub fn limit_radar_point(&self, point: &mut CVector2D) -> f32 {
        let dist = (point.x * point.x + point.y * point.y).sqrt();
        if dist > 1.0 {
            point.x /= dist;
            point.y /= dist;
        }
        dist
    }

    pub fn set_blip_sprite(&mut self, i: i32, icon: eRadarSprite) {
        if let Some(index) = self.get_actual_blip_array_index(i) {
            self.radar_trace[index].radar_sprite = icon;
        }
    }

    pub fn set_coord_blip(
        &mut self,
        blip_type: eBlipType,
        pos: CVector,
        color: i32,
        display: eBlipDisplay,
    ) -> i32 {
        let Some(next_blip) = self.radar_trace.iter().position(|trace| !trace.in_use) else {
            return -1;
        };
        self.radar_trace[next_blip] = sRadarTrace {
            blip_type,
            color: color as u32,
            dim: true,
            in_use: true,
            radius: 1.0,
            vec2d_pos: CVector2D::new(pos.x, pos.y),
            vec_pos: pos,
            entity_handle: 0,
            scale: 1,
            blip_display: display,
            radar_sprite: eRadarSprite::RADAR_SPRITE_NONE,
            blip_index: self.radar_trace[next_blip].blip_index,
        };
        self.get_new_unique_blip_index(next_blip)
    }

    pub fn set_entity_blip(
        &mut self,
        blip_type: eBlipType,
        handle: i32,
        color: i32,
        display: eBlipDisplay,
    ) -> i32 {
        let Some(next_blip) = self.radar_trace.iter().position(|trace| !trace.in_use) else {
            return -1;
        };
        self.radar_trace[next_blip] = sRadarTrace {
            blip_type,
            color: color as u32,
            dim: true,
            in_use: true,
            radius: 1.0,
            entity_handle: handle,
            scale: 1,
            blip_display: display,
            radar_sprite: eRadarSprite::RADAR_SPRITE_NONE,
            blip_index: self.radar_trace[next_blip].blip_index,
            ..sRadarTrace::default()
        };
        self.get_new_unique_blip_index(next_blip)
    }

    pub fn set_radar_marker_state(&mut self, i: i32, flag: bool) {
        if let Some(index) = self.get_actual_blip_array_index(i) {
            self.radar_trace[index].blip_display = if flag {
                eBlipDisplay::BLIP_DISPLAY_BOTH
            } else {
                eBlipDisplay::BLIP_DISPLAY_BLIP_ONLY
            };
        }
    }

    pub fn transform_real_world_to_tex_coord_space(
        out: &mut CVector2D,
        input: CVector2D,
        x: i32,
        y: i32,
    ) {
        out.x = input.x - (x as f32 * RADAR_TILE_SIZE + RADAR_MIN_X);
        out.y = -(input.y - ((RADAR_NUM_TILES as i32 - y) as f32 * RADAR_TILE_SIZE + RADAR_MIN_Y));
        out.x /= RADAR_TILE_SIZE;
        out.y /= RADAR_TILE_SIZE;
    }

    pub fn transform_radar_point_to_real_world_space(&self, out: &mut CVector2D, input: CVector2D) {
        let s = -self.cached_sin;
        let c = self.cached_cos;
        out.x = s * input.y + c * input.x;
        out.y = c * input.y - s * input.x;
        out.x = out.x * self.radar_range + self.radar_origin.x;
        out.y = out.y * self.radar_range + self.radar_origin.y;
    }

    pub fn transform_radar_point_to_screen_space(&self, out: &mut CVector2D, input: CVector2D) {
        out.x = (input.x + 1.0) * 0.5 * RADAR_WIDTH + RADAR_LEFT;
        out.y = (1.0 - input.y) * 0.5 * RADAR_HEIGHT
            + (DEFAULT_SCREEN_HEIGHT - (RADAR_BOTTOM + RADAR_HEIGHT));
    }

    pub fn transform_real_world_point_to_radar_space(&self, out: &mut CVector2D, input: CVector2D) {
        let x = (input.x - self.radar_origin.x) / self.radar_range;
        let y = (input.y - self.radar_origin.y) / self.radar_range;
        out.x = self.cached_sin * y + self.cached_cos * x;
        out.y = self.cached_cos * y - self.cached_sin * x;
    }

    pub fn calculate_cached_sin_cos(&mut self, heading_radians: f32) {
        self.cached_sin = heading_radians.sin();
        self.cached_cos = heading_radians.cos();
    }

    pub fn stream_radar_sections_for_position(&mut self, position: [f32; 3]) -> RadarStreamingPlan {
        let x = ((2000.0 + position[0]) / RADAR_TILE_SIZE).floor() as i32;
        let y = (7.0 - (2000.0 + position[1]) / RADAR_TILE_SIZE).ceil() as i32;
        self.stream_radar_sections(x, y)
    }

    /// Local co-op extension: union the same re3 3×3 radar windows for every
    /// active viewport, without letting the second call unload the first.
    pub fn stream_radar_sections_for_positions(
        &mut self,
        positions: &[[f32; 3]],
    ) -> RadarStreamingPlan {
        let mut desired = [false; RADAR_TXD_COUNT];
        for position in positions {
            let x = ((2000.0 + position[0]) / RADAR_TILE_SIZE).floor() as i32;
            let y = (7.0 - (2000.0 + position[1]) / RADAR_TILE_SIZE).ceil() as i32;
            for tile_y in (y - 1)..=(y + 1) {
                for tile_x in (x - 1)..=(x + 1) {
                    let clipped = radar_tile_index(
                        clip_radar_tile_coord(tile_x),
                        clip_radar_tile_coord(tile_y),
                    );
                    desired[clipped] = true;
                }
            }
        }
        let mut plan = RadarStreamingPlan {
            request_txd_ids: Vec::new(),
            remove_txd_ids: Vec::new(),
        };
        for index in 0..RADAR_TXD_COUNT {
            if desired[index] {
                if let Some(txd_id) = self.radar_txd_ids[index] {
                    plan.request_txd_ids.push(txd_id);
                }
            } else if self.streamed_sections[index]
                && let Some(txd_id) = self.radar_txd_ids[index]
            {
                plan.remove_txd_ids.push(txd_id);
            }
        }
        self.streamed_sections = desired;
        plan
    }

    pub fn stream_radar_sections(&mut self, x: i32, y: i32) -> RadarStreamingPlan {
        let mut plan = RadarStreamingPlan {
            request_txd_ids: Vec::new(),
            remove_txd_ids: Vec::new(),
        };

        for i in 0..RADAR_NUM_TILES as i32 {
            for j in 0..RADAR_NUM_TILES as i32 {
                let index = (i as usize) + RADAR_NUM_TILES * (j as usize);
                if i >= x - 1 && i <= x + 1 && j >= y - 1 && j <= y + 1 {
                    let clipped =
                        radar_tile_index(clip_radar_tile_coord(i), clip_radar_tile_coord(j));
                    if let Some(txd_id) = self.radar_txd_ids[clipped] {
                        plan.request_txd_ids.push(txd_id);
                    }
                    self.streamed_sections[index] = true;
                } else {
                    if let Some(txd_id) = self.radar_txd_ids[index] {
                        plan.remove_txd_ids.push(txd_id);
                    }
                    self.streamed_sections[index] = false;
                }
            }
        }

        plan
    }

    pub fn remove_radar_sections(&mut self) -> RadarStreamingPlan {
        let mut plan = RadarStreamingPlan {
            request_txd_ids: Vec::new(),
            remove_txd_ids: Vec::new(),
        };
        for index in 0..RADAR_TXD_COUNT {
            if let Some(txd_id) = self.radar_txd_ids[index] {
                plan.remove_txd_ids.push(txd_id);
            }
            self.streamed_sections[index] = false;
        }
        plan
    }

    pub fn save_all_radar_blips(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write_header(*b"RDR\0", RADAR_BLIPS_SAVE_SIZE - SAVE_HEADER_SIZE as u32);
        for trace in self.radar_trace {
            write_radar_trace(&mut writer, trace);
        }
        writer.into_inner()
    }

    pub fn load_all_radar_blips(&mut self, bytes: &[u8]) -> Result<(), SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        reader.check_header(*b"RDR\0", RADAR_BLIPS_SAVE_SIZE - SAVE_HEADER_SIZE as u32)?;
        for trace in &mut self.radar_trace {
            *trace = read_radar_trace(&mut reader)?;
        }
        Ok(())
    }
}

fn write_radar_trace(writer: &mut SaveBufWriter, trace: sRadarTrace) {
    writer.write(trace.color);
    writer.write(trace.blip_type as u32);
    writer.write(trace.entity_handle);
    writer.write(trace.vec2d_pos.x);
    writer.write(trace.vec2d_pos.y);
    writer.write(trace.vec_pos.x);
    writer.write(trace.vec_pos.y);
    writer.write(trace.vec_pos.z);
    writer.write(trace.blip_index);
    writer.write(trace.dim as u8);
    writer.write(trace.in_use as u8);
    writer.write(trace.radius);
    writer.write(trace.scale);
    writer.write(trace.blip_display as u16);
    writer.write(trace.radar_sprite as u16);
    writer.zero(2);
}

fn read_radar_trace(reader: &mut SaveBufReader<'_>) -> Result<sRadarTrace, SaveBufError> {
    let color = reader.read::<u32>()?;
    let blip_type = blip_type_from_u32(reader.read::<u32>()?);
    let entity_handle = reader.read::<i32>()?;
    let vec2d_pos = CVector2D::new(reader.read::<f32>()?, reader.read::<f32>()?);
    let vec_pos = CVector {
        x: reader.read::<f32>()?,
        y: reader.read::<f32>()?,
        z: reader.read::<f32>()?,
    };
    let blip_index = reader.read::<u16>()?;
    let dim = reader.read::<u8>()? != 0;
    let in_use = reader.read::<u8>()? != 0;
    let radius = reader.read::<f32>()?;
    let scale = reader.read::<i16>()?;
    let blip_display = blip_display_from_u16(reader.read::<u16>()?);
    let radar_sprite = radar_sprite_from_u16(reader.read::<u16>()?);
    reader.skip(2)?;
    Ok(sRadarTrace {
        color,
        blip_type,
        entity_handle,
        vec2d_pos,
        vec_pos,
        blip_index,
        dim,
        in_use,
        radius,
        scale,
        blip_display,
        radar_sprite,
    })
}

fn blip_type_from_u32(value: u32) -> eBlipType {
    match value {
        1 => eBlipType::BLIP_CAR,
        2 => eBlipType::BLIP_CHAR,
        3 => eBlipType::BLIP_OBJECT,
        4 => eBlipType::BLIP_COORD,
        5 => eBlipType::BLIP_CONTACT_POINT,
        _ => eBlipType::BLIP_NONE,
    }
}

fn blip_display_from_u16(value: u16) -> eBlipDisplay {
    match value {
        1 => eBlipDisplay::BLIP_DISPLAY_MARKER_ONLY,
        2 => eBlipDisplay::BLIP_DISPLAY_BLIP_ONLY,
        3 => eBlipDisplay::BLIP_DISPLAY_BOTH,
        _ => eBlipDisplay::BLIP_DISPLAY_NEITHER,
    }
}

fn radar_sprite_from_u16(value: u16) -> eRadarSprite {
    match value {
        1 => eRadarSprite::RADAR_SPRITE_ASUKA,
        2 => eRadarSprite::RADAR_SPRITE_BOMB,
        3 => eRadarSprite::RADAR_SPRITE_CAT,
        4 => eRadarSprite::RADAR_SPRITE_CENTRE,
        5 => eRadarSprite::RADAR_SPRITE_COPCAR,
        6 => eRadarSprite::RADAR_SPRITE_DON,
        7 => eRadarSprite::RADAR_SPRITE_EIGHT,
        8 => eRadarSprite::RADAR_SPRITE_EL,
        9 => eRadarSprite::RADAR_SPRITE_ICE,
        10 => eRadarSprite::RADAR_SPRITE_JOEY,
        11 => eRadarSprite::RADAR_SPRITE_KENJI,
        12 => eRadarSprite::RADAR_SPRITE_LIZ,
        13 => eRadarSprite::RADAR_SPRITE_LUIGI,
        14 => eRadarSprite::RADAR_SPRITE_NORTH,
        15 => eRadarSprite::RADAR_SPRITE_RAY,
        16 => eRadarSprite::RADAR_SPRITE_SAL,
        17 => eRadarSprite::RADAR_SPRITE_SAVE,
        18 => eRadarSprite::RADAR_SPRITE_SPRAY,
        19 => eRadarSprite::RADAR_SPRITE_TONY,
        20 => eRadarSprite::RADAR_SPRITE_WEAPON,
        _ => eRadarSprite::RADAR_SPRITE_NONE,
    }
}

fn clip_radar_tile_coord(value: i32) -> i32 {
    value.clamp(0, RADAR_NUM_TILES as i32 - 1)
}

fn radar_tile_index(x: i32, y: i32) -> usize {
    x as usize + RADAR_NUM_TILES * y as usize
}

fn radar_tex_name(index: usize) -> String {
    format!("radar{index:02}")
}
