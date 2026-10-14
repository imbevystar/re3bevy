//! re3 reference: references/re-GTA/src/control/Record.cpp
//! re3 reference: references/re-GTA/src/control/Record.h
//!
//! `CRecordDataForGame::Init` and `CRecordDataForChase::Init` state.

use std::io;

use bevy::prelude::*;
use rustgta3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use rustgta3_core::core::Coordinates::CVector;

use crate::modelinfo::ModelIndices::{
    MI_CHEETAH, MI_ENFORCER, MI_KURUMA, MI_PATRIOT, MI_POLICE, MI_RUMPO, MI_STINGER, MI_TAXI,
};

pub const RECORD_STATE_NONE: u16 = 0;
pub const NUM_CHASE_CARS: usize = 20;
pub const CHASE_SCENE_LENGTH_SECONDS: f32 = 80.0;
pub const CHASE_SCENE_FRAMES_PER_SECOND: f32 = 15.0;
pub const CHASE_SCENE_FRAMES: usize = 1_200;
const CHASE_FILE_FRAME_SIZE: usize = 28;
const CHASE_FILE_FRAME_STRIDE: usize = CHASE_FILE_FRAME_SIZE * 2;

pub const CHASE_CARS: [(usize, usize, CVector, f32, u8, u8); 13] = [
    (
        0,
        MI_POLICE,
        CVector::new(273.54221, -1167.1907, 24.880601),
        63.0,
        2,
        1,
    ),
    (
        1,
        MI_ENFORCER,
        CVector::new(231.1783, -1388.8322, 25.978201),
        90.0,
        2,
        1,
    ),
    (
        4,
        MI_TAXI,
        CVector::new(184.3156, -1473.251, 25.978201),
        0.0,
        6,
        6,
    ),
    (
        6,
        MI_CHEETAH,
        CVector::new(173.8868, -1377.6514, 25.978201),
        0.0,
        4,
        5,
    ),
    (
        7,
        MI_STINGER,
        CVector::new(102.5946, -943.93628, 25.9781),
        270.0,
        53,
        53,
    ),
    (
        10,
        MI_CHEETAH,
        CVector::new(-177.7157, -862.18652, 25.978201),
        155.0,
        41,
        1,
    ),
    (
        11,
        MI_STINGER,
        CVector::new(-170.56979, -889.02362, 25.978201),
        154.0,
        10,
        10,
    ),
    (
        14,
        MI_KURUMA,
        CVector::new(402.60809, -917.49628, 37.381001),
        90.0,
        34,
        1,
    ),
    (
        16,
        MI_TAXI,
        CVector::new(-33.496201, -938.4563, 25.9781),
        266.0,
        6,
        6,
    ),
    (
        18,
        MI_KURUMA,
        CVector::new(49.363098, -987.60498, 25.9781),
        0.0,
        51,
        1,
    ),
    (
        19,
        MI_TAXI,
        CVector::new(179.0049, -1154.6686, 25.9781),
        0.0,
        6,
        76,
    ),
    (
        2,
        MI_RUMPO,
        CVector::new(-28.9762, -1031.3367, 25.990601),
        242.0,
        1,
        75,
    ),
    (
        3,
        MI_PATRIOT,
        CVector::new(114.1564, -796.69379, 24.978201),
        180.0,
        0,
        0,
    ),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CCarStateEachFrame {
    pub velocity: CVector,
    pub right: CVector,
    pub forward: CVector,
    pub steer_angle: f32,
    pub gas_pedal: f32,
    pub brake_pedal: f32,
    pub handbrake: bool,
    pub position: CVector,
}

impl CCarStateEachFrame {
    fn decode(bytes: &[u8]) -> Self {
        let i16_at = |offset| i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let f32_at = |offset| {
            f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ])
        };
        Self {
            velocity: CVector::new(
                i16_at(0) as f32 / i16::MAX as f32 / 0.5,
                i16_at(2) as f32 / i16::MAX as f32 / 0.5,
                i16_at(4) as f32 / i16::MAX as f32 / 0.5,
            ),
            right: CVector::new(
                bytes[6] as i8 as f32 / i8::MAX as f32,
                bytes[7] as i8 as f32 / i8::MAX as f32,
                bytes[8] as i8 as f32 / i8::MAX as f32,
            ),
            forward: CVector::new(
                bytes[9] as i8 as f32 / i8::MAX as f32,
                bytes[10] as i8 as f32 / i8::MAX as f32,
                bytes[11] as i8 as f32 / i8::MAX as f32,
            ),
            steer_angle: bytes[12] as i8 as f32 / 20.0,
            gas_pedal: bytes[13] as i8 as f32 / 100.0,
            brake_pedal: bytes[14] as i8 as f32 / 100.0,
            handbrake: bytes[15] != 0,
            position: CVector::new(f32_at(16), f32_at(20), f32_at(24)),
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CRecordDataForGame {
    pub recording_state: u16,
    pub has_data_buffer: bool,
    pub has_data_buffer_pointer: bool,
    pub file_id: i32,
}

impl CRecordDataForGame {
    pub fn init_no_record_files() -> Self {
        Self {
            recording_state: RECORD_STATE_NONE,
            has_data_buffer: false,
            has_data_buffer_pointer: false,
            file_id: 0,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CRecordDataForChase {
    pub status: u8,
    pub position_changes: i32,
    pub current_car: u8,
    pub chase_car_present: [bool; NUM_CHASE_CARS],
    pub anim_start_time: u32,
    pub anim_time: f32,
    pub time_multiplier: f32,
    pub car_handles: [Option<i32>; NUM_CHASE_CARS],
    /// Cooperative backend state for C++ START_CHASE_SCENE's blocking
    /// DeleteAllRwObjects + model residency prefix. Not part of save/gameplay
    /// semantics; prevents the prefix from running twice when the effect is
    /// retried after the streaming barrier.
    pub(crate) streaming_prepared: bool,
    recordings: [Option<Vec<CCarStateEachFrame>>; NUM_CHASE_CARS],
}

impl CRecordDataForChase {
    pub fn init() -> Self {
        Self {
            status: RECORD_STATE_NONE as u8,
            position_changes: 0,
            current_car: 0,
            chase_car_present: [false; NUM_CHASE_CARS],
            anim_start_time: 0,
            anim_time: 0.0,
            time_multiplier: 0.0,
            car_handles: [None; NUM_CHASE_CARS],
            streaming_prepared: false,
            recordings: std::array::from_fn(|_| None),
        }
    }

    pub fn start_chase_scene(&mut self, start_time: f32, time_ms: u32) {
        self.status = 3;
        self.anim_time = start_time;
        self.anim_start_time = time_ms;
        self.time_multiplier = 0.0;
        self.streaming_prepared = false;
    }

    pub fn load_recording(&mut self, install: &OriginalGtaInstall, index: usize) -> io::Result<()> {
        let bytes = install.read(format!("data\\paths\\chase{index}.dat"))?;
        let required = CHASE_SCENE_FRAMES * CHASE_FILE_FRAME_STRIDE;
        if bytes.len() < required {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "chase{index}.dat is {} bytes, expected at least {required}",
                    bytes.len()
                ),
            ));
        }
        self.recordings[index] = Some(
            (0..CHASE_SCENE_FRAMES)
                .map(|frame| {
                    let offset = frame * CHASE_FILE_FRAME_STRIDE;
                    CCarStateEachFrame::decode(&bytes[offset..offset + CHASE_FILE_FRAME_SIZE])
                })
                .collect(),
        );
        Ok(())
    }

    pub fn playback_sample(
        &self,
        index: usize,
    ) -> Option<(CCarStateEachFrame, Option<CCarStateEachFrame>, f32)> {
        let frames = self.recordings.get(index)?.as_ref()?;
        let frame_time =
            CHASE_SCENE_FRAMES_PER_SECOND * self.time_multiplier.min(CHASE_SCENE_LENGTH_SECONDS);
        if frame_time >= (CHASE_SCENE_FRAMES - 1) as f32 {
            return Some((frames[CHASE_SCENE_FRAMES - 1], None, 0.0));
        }
        let frame = frame_time.floor() as usize;
        Some((
            frames[frame],
            Some(frames[frame + 1]),
            frame_time - frame as f32,
        ))
    }

    pub fn remove_car_from_chase(&mut self, index: i32) -> Option<i32> {
        let index = usize::try_from(index).ok()?;
        *self.chase_car_present.get_mut(index)? = false;
        self.recordings[index] = None;
        self.car_handles[index].take()
    }

    pub fn clean_up_chase_scene(&mut self) -> Vec<i32> {
        self.streaming_prepared = false;
        if self.status == 2 || self.status == 3 {
            self.status = RECORD_STATE_NONE as u8;
            self.chase_car_present.fill(false);
            self.recordings.fill(None);
            return self
                .car_handles
                .iter_mut()
                .filter_map(Option::take)
                .collect();
        }
        Vec::new()
    }
}
