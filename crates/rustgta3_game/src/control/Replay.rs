//! re3 reference: references/re-GTA/src/control/Replay.cpp
//! re3 reference: references/re-GTA/src/control/Replay.h
//!
//! `CReplay::Init` state.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter};

pub const NUM_REPLAYBUFFERS: usize = 8;
pub const REPLAYBUFFERSIZE: usize = 100_000;
pub const REPLAYPACKET_END: u8 = 0;
pub const REPLAYPACKET_VEHICLE: u8 = 1;
pub const REPLAYPACKET_PED_HEADER: u8 = 2;
pub const REPLAYPACKET_PED_UPDATE: u8 = 3;
pub const REPLAYPACKET_GENERAL: u8 = 4;
pub const REPLAYPACKET_CLOCK: u8 = 5;
pub const REPLAYPACKET_WEATHER: u8 = 6;
pub const REPLAYPACKET_ENDOFFRAME: u8 = 7;
pub const REPLAYPACKET_TIMER: u8 = 8;
pub const REPLAYPACKET_BULLET_TRACES: u8 = 9;

pub const REPLAYPACKET_END_SIZE: usize = 4;
pub const REPLAYPACKET_VEHICLE_SIZE: usize = 48;
pub const REPLAYPACKET_PED_HEADER_SIZE: usize = 8;
pub const REPLAYPACKET_PED_UPDATE_SIZE: usize = 40;
pub const REPLAYPACKET_GENERAL_SIZE: usize = 88;
pub const REPLAYPACKET_CLOCK_SIZE: usize = 4;
pub const REPLAYPACKET_WEATHER_SIZE: usize = 8;
pub const REPLAYPACKET_ENDOFFRAME_SIZE: usize = 4;
pub const REPLAYPACKET_TIMER_SIZE: usize = 8;
pub const REPLAYPACKET_BULLET_TRACES_SIZE: usize = 28;
pub const COMPRESSED_MATRIX_NOT_ALIGNED_SIZE: usize = 20;
pub const STORED_ANIMATION_STATE_SIZE: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReplayMode {
    Record = 0,
    Playback = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReplayBufferStatus {
    Unused = 0,
    Playback = 1,
    Record = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReplayPacketKind {
    End = REPLAYPACKET_END,
    Vehicle = REPLAYPACKET_VEHICLE,
    PedHeader = REPLAYPACKET_PED_HEADER,
    PedUpdate = REPLAYPACKET_PED_UPDATE,
    General = REPLAYPACKET_GENERAL,
    Clock = REPLAYPACKET_CLOCK,
    Weather = REPLAYPACKET_WEATHER,
    EndOfFrame = REPLAYPACKET_ENDOFFRAME,
    Timer = REPLAYPACKET_TIMER,
    BulletTraces = REPLAYPACKET_BULLET_TRACES,
}

impl ReplayPacketKind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            REPLAYPACKET_END => Some(Self::End),
            REPLAYPACKET_VEHICLE => Some(Self::Vehicle),
            REPLAYPACKET_PED_HEADER => Some(Self::PedHeader),
            REPLAYPACKET_PED_UPDATE => Some(Self::PedUpdate),
            REPLAYPACKET_GENERAL => Some(Self::General),
            REPLAYPACKET_CLOCK => Some(Self::Clock),
            REPLAYPACKET_WEATHER => Some(Self::Weather),
            REPLAYPACKET_ENDOFFRAME => Some(Self::EndOfFrame),
            REPLAYPACKET_TIMER => Some(Self::Timer),
            REPLAYPACKET_BULLET_TRACES => Some(Self::BulletTraces),
            _ => None,
        }
    }

    pub fn packet_size(self) -> usize {
        CReplay::find_size_of_packet(self as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CAddressInReplayBuffer {
    pub offset: u32,
    pub slot: u8,
    pub has_base: bool,
}

#[derive(Debug, Clone, Resource)]
pub struct CReplay {
    pub mode: ReplayMode,
    pub record: CAddressInReplayBuffer,
    pub playback: CAddressInReplayBuffer,
    pub buffer_status: [ReplayBufferStatus; NUM_REPLAYBUFFERS],
    pub buffers: Vec<Vec<u8>>,
    pub first_record_packet: u8,
    pub playing_back_from_file: bool,
    pub replay_enabled: bool,
    pub slow_motion: u32,
    pub frames_active_look_around_cam: u32,
    pub do_load_scene_when_done: bool,
    pub transient_pointers_cleared: bool,
    pub replay_buffer_size: usize,
    pub time1: u32,
    pub time2: u32,
    pub time3: u32,
    pub time4: u32,
    pub frame: u32,
    pub clock_hours: u8,
    pub clock_minutes: u8,
    pub old_weather_type: u16,
    pub new_weather_type: u16,
    pub weather_interpolation_value: f32,
    pub time_step_non_clipped: f32,
    pub time_step: f32,
    pub time_scale: f32,
    pub camera_fixed: [f32; 3],
    pub old_radio_station: i32,
    pub camera_mode: i8,
    pub allow_look_around_cam: bool,
    pub load_scene: [f32; 3],
    pub camera_focus: [f32; 3],
    pub player_in_rc_buggy: bool,
    pub distance_look_around_cam: f32,
    pub beta_angle_look_around_cam: f32,
    pub alpha_angle_look_around_cam: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayPacketView<'a> {
    pub slot: u8,
    pub offset: u32,
    pub kind: ReplayPacketKind,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CCompressedMatrixNotAlignedReplay {
    pub position: [f32; 3],
    pub right: [i8; 3],
    pub up: [i8; 3],
}

impl CCompressedMatrixNotAlignedReplay {
    fn read(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let matrix = Self {
            position: [
                reader.read::<f32>()?,
                reader.read::<f32>()?,
                reader.read::<f32>()?,
            ],
            right: [
                reader.read::<i8>()?,
                reader.read::<i8>()?,
                reader.read::<i8>()?,
            ],
            up: [
                reader.read::<i8>()?,
                reader.read::<i8>()?,
                reader.read::<i8>()?,
            ],
        };
        reader.skip(2)?;
        Ok(matrix)
    }

    fn write(self, writer: &mut SaveBufWriter) {
        for value in self.position {
            writer.write(value);
        }
        for value in self.right {
            writer.write(value);
        }
        for value in self.up {
            writer.write(value);
        }
        writer.zero(2);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CStoredAnimationStateReplay {
    pub anim_id: u8,
    pub time: u8,
    pub speed: u8,
    pub sec_anim_id: u8,
    pub sec_time: u8,
    pub sec_speed: u8,
    pub blend_amount: u8,
    pub part_anim_id: u8,
    pub part_anim_time: u8,
    pub part_anim_speed: u8,
    pub part_blend_amount: u8,
}

impl CStoredAnimationStateReplay {
    fn read(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        Ok(Self {
            anim_id: reader.read::<u8>()?,
            time: reader.read::<u8>()?,
            speed: reader.read::<u8>()?,
            sec_anim_id: reader.read::<u8>()?,
            sec_time: reader.read::<u8>()?,
            sec_speed: reader.read::<u8>()?,
            blend_amount: reader.read::<u8>()?,
            part_anim_id: reader.read::<u8>()?,
            part_anim_time: reader.read::<u8>()?,
            part_anim_speed: reader.read::<u8>()?,
            part_blend_amount: reader.read::<u8>()?,
        })
    }

    fn write(self, writer: &mut SaveBufWriter) {
        writer.write(self.anim_id);
        writer.write(self.time);
        writer.write(self.speed);
        writer.write(self.sec_anim_id);
        writer.write(self.sec_time);
        writer.write(self.sec_speed);
        writer.write(self.blend_amount);
        writer.write(self.part_anim_id);
        writer.write(self.part_anim_time);
        writer.write(self.part_anim_speed);
        writer.write(self.part_blend_amount);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayPacket {
    End,
    Vehicle(TVehicleUpdatePacket),
    PedHeader(TPedHeaderPacket),
    PedUpdate(TPedUpdatePacket),
    General(TGeneralPacket),
    Clock(TClockPacket),
    Weather(TWeatherPacket),
    EndOfFrame,
    Timer(TTimerPacket),
    BulletTraces(TBulletTracePacket),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TGeneralPacket {
    pub in_rcvehicle: bool,
    pub camera_matrix: [f32; 16],
    pub player_pos: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TClockPacket {
    pub hours: u8,
    pub minutes: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TWeatherPacket {
    pub old_weather: u8,
    pub new_weather: u8,
    pub interpolation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TTimerPacket {
    pub timer: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TPedHeaderPacket {
    pub index: u8,
    pub model_index: u16,
    pub ped_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TBulletTracePacket {
    pub frames: u8,
    pub lifetime: u8,
    pub index: u8,
    pub inf: [f32; 3],
    pub sup: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TPedUpdatePacket {
    pub index: u8,
    pub heading: i8,
    pub vehicle_index: i8,
    pub anim_state: CStoredAnimationStateReplay,
    pub matrix: CCompressedMatrixNotAlignedReplay,
    pub assoc_group_id: i8,
    pub weapon_model: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TVehicleUpdatePacket {
    pub index: u8,
    pub health: u8,
    pub acceleration: u8,
    pub matrix: CCompressedMatrixNotAlignedReplay,
    pub door_angles: [i8; 2],
    pub model_index: u16,
    pub panels: u32,
    pub velocity: [i8; 3],
    pub car_gun_or_wheel_state: i8,
    pub wheel_susp_dist: [u8; 4],
    pub wheel_rotation: [u8; 4],
    pub door_status: u8,
    pub primary_color: u8,
    pub secondary_color: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayPlaybackFrame {
    pub interpolation: f32,
    pub general: Option<TGeneralPacket>,
    pub clock: Option<TClockPacket>,
    pub weather: Option<TWeatherPacket>,
    pub timer: Option<TTimerPacket>,
    pub vehicle_updates: Vec<TVehicleUpdatePacket>,
    pub ped_headers: Vec<TPedHeaderPacket>,
    pub ped_updates: Vec<TPedUpdatePacket>,
    pub bullet_traces: Vec<TBulletTracePacket>,
    pub wrapped_slots: u8,
    pub reached_record_slot: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayWorldMutationPlan {
    pub interpolation: f32,
    pub camera_focus: Option<[f32; 3]>,
    pub player_in_rc_buggy: Option<bool>,
    pub clock: Option<TClockPacket>,
    pub weather: Option<TWeatherPacket>,
    pub timer: Option<TTimerPacket>,
    pub vehicle_ops: Vec<ReplayVehicleMutation>,
    pub ped_ops: Vec<ReplayPedMutation>,
    pub bullet_traces: Vec<TBulletTracePacket>,
    pub reached_record_slot: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayVehicleMutation {
    RemoveMissingBefore { start: u8, end_exclusive: u8 },
    EnsureExists { index: u8, model_index: u16 },
    ApplyUpdate(TVehicleUpdatePacket),
    RemoveMissingAfter { start: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayPedMutation {
    EnsureExists(TPedHeaderPacket),
    RemoveMissingBefore { start: u8, end_exclusive: u8 },
    ApplyUpdate(TPedUpdatePacket),
    RemoveMissingAfter { start: u8 },
}

#[derive(Debug, Clone, PartialEq, Default, Resource)]
pub struct ReplayRuntimeWorldState {
    pub camera_focus: [f32; 3],
    pub player_in_rc_buggy: bool,
    pub clock: Option<TClockPacket>,
    pub weather: Option<TWeatherPacket>,
    pub timer: Option<TTimerPacket>,
    pub bullet_traces: Vec<TBulletTracePacket>,
    pub reached_record_slot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplayVehiclePoolOverlay {
    pub replay_index: u8,
    pub model_index: u16,
    pub update: TVehicleUpdatePacket,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplayPedPoolOverlay {
    pub replay_index: u8,
    pub header: Option<TPedHeaderPacket>,
    pub update: Option<TPedUpdatePacket>,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct ReplayVehicleRuntimeState {
    pub replay_index: u8,
    pub model_index: u16,
    pub update: TVehicleUpdatePacket,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct ReplayPedRuntimeState {
    pub replay_index: u8,
    pub header: Option<TPedHeaderPacket>,
    pub update: Option<TPedUpdatePacket>,
}

impl ReplayPlaybackFrame {
    pub fn new(interpolation: f32) -> Self {
        Self {
            interpolation,
            general: None,
            clock: None,
            weather: None,
            timer: None,
            vehicle_updates: Vec::new(),
            ped_headers: Vec::new(),
            ped_updates: Vec::new(),
            bullet_traces: Vec::new(),
            wrapped_slots: 0,
            reached_record_slot: false,
        }
    }

    pub fn build_world_mutation_plan(&self) -> ReplayWorldMutationPlan {
        let mut vehicle_ops = Vec::new();
        let mut vehicle_min_index = 0_u8;
        for update in &self.vehicle_updates {
            if vehicle_min_index < update.index {
                vehicle_ops.push(ReplayVehicleMutation::RemoveMissingBefore {
                    start: vehicle_min_index,
                    end_exclusive: update.index,
                });
            }
            vehicle_ops.push(ReplayVehicleMutation::EnsureExists {
                index: update.index,
                model_index: update.model_index,
            });
            vehicle_ops.push(ReplayVehicleMutation::ApplyUpdate(*update));
            vehicle_min_index = update.index.saturating_add(1);
        }
        vehicle_ops.push(ReplayVehicleMutation::RemoveMissingAfter {
            start: vehicle_min_index,
        });

        let mut ped_ops = Vec::new();
        for header in &self.ped_headers {
            ped_ops.push(ReplayPedMutation::EnsureExists(*header));
        }
        let mut ped_min_index = 0_u8;
        for update in &self.ped_updates {
            if ped_min_index < update.index {
                ped_ops.push(ReplayPedMutation::RemoveMissingBefore {
                    start: ped_min_index,
                    end_exclusive: update.index,
                });
            }
            ped_ops.push(ReplayPedMutation::ApplyUpdate(*update));
            ped_min_index = update.index.saturating_add(1);
        }
        ped_ops.push(ReplayPedMutation::RemoveMissingAfter {
            start: ped_min_index,
        });

        ReplayWorldMutationPlan {
            interpolation: self.interpolation,
            camera_focus: self.general.map(|general| general.player_pos),
            player_in_rc_buggy: self.general.map(|general| general.in_rcvehicle),
            clock: self.clock,
            weather: self.weather,
            timer: self.timer,
            vehicle_ops,
            ped_ops,
            bullet_traces: self.bullet_traces.clone(),
            reached_record_slot: self.reached_record_slot,
        }
    }
}

impl ReplayWorldMutationPlan {
    pub fn apply_to_runtime_state(&self, state: &mut ReplayRuntimeWorldState) {
        if let Some(focus) = self.camera_focus {
            let split = 1.0 - self.interpolation;
            state.camera_focus[0] = split * state.camera_focus[0] + self.interpolation * focus[0];
            state.camera_focus[1] = split * state.camera_focus[1] + self.interpolation * focus[1];
            state.camera_focus[2] = split * state.camera_focus[2] + self.interpolation * focus[2];
        }
        if let Some(in_rc_buggy) = self.player_in_rc_buggy {
            state.player_in_rc_buggy = in_rc_buggy;
        }
        if let Some(clock) = self.clock {
            state.clock = Some(clock);
        }
        if let Some(weather) = self.weather {
            state.weather = Some(weather);
        }
        if let Some(timer) = self.timer {
            state.timer = Some(timer);
        }

        state.bullet_traces = self.bullet_traces.clone();
        state.reached_record_slot = self.reached_record_slot;
    }
}

pub(crate) fn empty_vehicle_update(index: u8, model_index: u16) -> TVehicleUpdatePacket {
    TVehicleUpdatePacket {
        index,
        health: 0,
        acceleration: 0,
        matrix: CCompressedMatrixNotAlignedReplay {
            position: [0.0; 3],
            right: [0; 3],
            up: [0; 3],
        },
        door_angles: [0; 2],
        model_index,
        panels: 0,
        velocity: [0; 3],
        car_gun_or_wheel_state: 0,
        wheel_susp_dist: [0; 4],
        wheel_rotation: [0; 4],
        door_status: 0,
        primary_color: 0,
        secondary_color: 0,
    }
}

impl CReplay {
    pub fn init() -> Self {
        let mut buffer_status = [ReplayBufferStatus::Unused; NUM_REPLAYBUFFERS];
        buffer_status[0] = ReplayBufferStatus::Record;

        let mut buffers = vec![vec![REPLAYPACKET_END; REPLAYBUFFERSIZE]; NUM_REPLAYBUFFERS];
        buffers[0][0] = REPLAYPACKET_END;

        Self {
            mode: ReplayMode::Record,
            record: CAddressInReplayBuffer {
                offset: 0,
                slot: 0,
                has_base: true,
            },
            playback: CAddressInReplayBuffer {
                offset: 0,
                slot: 0,
                has_base: false,
            },
            buffer_status,
            buffers,
            first_record_packet: REPLAYPACKET_END,
            playing_back_from_file: false,
            replay_enabled: true,
            slow_motion: 1,
            frames_active_look_around_cam: 0,
            do_load_scene_when_done: false,
            transient_pointers_cleared: true,
            replay_buffer_size: REPLAYBUFFERSIZE,
            time1: 0,
            time2: 0,
            time3: 0,
            time4: 0,
            frame: 0,
            clock_hours: 0,
            clock_minutes: 0,
            old_weather_type: 0,
            new_weather_type: 0,
            weather_interpolation_value: 0.0,
            time_step_non_clipped: 0.0,
            time_step: 0.0,
            time_scale: 1.0,
            camera_fixed: [0.0; 3],
            old_radio_station: -1,
            camera_mode: 0,
            allow_look_around_cam: false,
            load_scene: [0.0; 3],
            camera_focus: [0.0; 3],
            player_in_rc_buggy: false,
            distance_look_around_cam: 0.0,
            beta_angle_look_around_cam: 0.0,
            alpha_angle_look_around_cam: 0.0,
        }
    }

    pub fn disable_replays(&mut self) {
        self.replay_enabled = false;
    }

    pub fn enable_replays(&mut self) {
        self.replay_enabled = true;
    }

    pub fn empty_replay_buffer(&mut self) {
        self.buffer_status = [ReplayBufferStatus::Unused; NUM_REPLAYBUFFERS];
        for buffer in &mut self.buffers {
            buffer.fill(REPLAYPACKET_END);
        }
        self.record = CAddressInReplayBuffer {
            offset: 0,
            slot: 0,
            has_base: true,
        };
        self.playback = CAddressInReplayBuffer {
            offset: 0,
            slot: 0,
            has_base: false,
        };
        self.buffer_status[0] = ReplayBufferStatus::Record;
    }

    pub fn record_bytes(&mut self, bytes: &[u8]) {
        let required = bytes.len() + 1;
        if self.record.offset as usize + required > REPLAYBUFFERSIZE {
            self.finish_current_record_slot();
        }
        let slot = self.record.slot as usize;
        let offset = self.record.offset as usize;
        self.buffers[slot][offset..offset + bytes.len()].copy_from_slice(bytes);
        self.record.offset += bytes.len() as u32;
        self.buffers[slot][self.record.offset as usize] = REPLAYPACKET_END;
    }

    pub fn record_packet(&mut self, kind: ReplayPacketKind, payload_after_type: &[u8]) {
        let size = kind.packet_size();
        if size == 0 {
            return;
        }
        let mut packet = vec![0_u8; size];
        packet[0] = kind as u8;
        let len = payload_after_type.len().min(size.saturating_sub(1));
        packet[1..1 + len].copy_from_slice(&payload_after_type[..len]);
        self.record_bytes(&packet);
    }

    pub fn finish_current_record_slot(&mut self) {
        let slot = self.record.slot as usize;
        self.buffers[slot][self.record.offset as usize] = REPLAYPACKET_END;
        self.buffer_status[slot] = ReplayBufferStatus::Playback;
        self.record.slot = (self.record.slot + 1) % NUM_REPLAYBUFFERS as u8;
        self.record.offset = 0;
        let next = self.record.slot as usize;
        self.buffer_status[next] = ReplayBufferStatus::Record;
        self.buffers[next].fill(REPLAYPACKET_END);
    }

    pub fn trigger_playback(
        &mut self,
        cam_mode: i8,
        cam_x: f32,
        cam_y: f32,
        cam_z: f32,
        load_scene: bool,
    ) {
        let Some(first) = self.first_playback_slot() else {
            return;
        };
        self.mode = ReplayMode::Playback;
        self.playback = CAddressInReplayBuffer {
            offset: 0,
            slot: first,
            has_base: true,
        };
        self.camera_mode = cam_mode;
        self.camera_fixed = [cam_x, cam_y, cam_z];
        self.do_load_scene_when_done = load_scene;
    }

    pub fn finish_playback(&mut self) {
        self.mode = ReplayMode::Record;
        self.playback = CAddressInReplayBuffer {
            offset: 0,
            slot: 0,
            has_base: false,
        };
        self.empty_replay_buffer();
    }

    pub fn first_playback_slot(&self) -> Option<u8> {
        let current = self
            .buffer_status
            .iter()
            .position(|status| *status == ReplayBufferStatus::Record)?;
        for step in 1..=NUM_REPLAYBUFFERS {
            let slot = (current + step) % NUM_REPLAYBUFFERS;
            let status = self.buffer_status[slot];
            if status == ReplayBufferStatus::Playback || status == ReplayBufferStatus::Record {
                return Some(slot as u8);
            }
        }
        None
    }

    pub fn packet_at(&self, address: CAddressInReplayBuffer) -> Option<ReplayPacketView<'_>> {
        let slot = address.slot as usize;
        let offset = address.offset as usize;
        let buffer = self.buffers.get(slot)?;
        let packet_type = *buffer.get(offset)?;
        let kind = ReplayPacketKind::from_u8(packet_type)?;
        let size = kind.packet_size();
        let bytes = buffer.get(offset..offset + size)?;
        Some(ReplayPacketView {
            slot: address.slot,
            offset: address.offset,
            kind,
            bytes,
        })
    }

    pub fn decoded_packet_at(&self, address: CAddressInReplayBuffer) -> Option<ReplayPacket> {
        self.packet_at(address)
            .and_then(|view| Self::decode_packet(view.bytes).ok())
    }

    pub fn advance_replay_address(
        &self,
        address: &mut CAddressInReplayBuffer,
    ) -> Option<ReplayPacketKind> {
        let view = self.packet_at(*address)?;
        match view.kind {
            ReplayPacketKind::End => {
                let next_slot = (address.slot as usize + 1) % NUM_REPLAYBUFFERS;
                if self.buffer_status[next_slot] == ReplayBufferStatus::Unused {
                    return None;
                }
                address.slot = next_slot as u8;
                address.offset = 0;
                address.has_base = true;
                Some(ReplayPacketKind::End)
            }
            kind => {
                address.offset += kind.packet_size() as u32;
                Some(kind)
            }
        }
    }

    pub fn collect_playback_frame(
        &self,
        address: &mut CAddressInReplayBuffer,
        interpolation: f32,
    ) -> Option<ReplayPlaybackFrame> {
        let mut frame = ReplayPlaybackFrame::new(interpolation);
        loop {
            let packet = self.decoded_packet_at(*address)?;
            match packet {
                ReplayPacket::End => {
                    let slot = address.slot as usize;
                    if self.buffer_status[slot] == ReplayBufferStatus::Record {
                        frame.reached_record_slot = true;
                        return Some(frame);
                    }
                    let next_slot = (slot + 1) % NUM_REPLAYBUFFERS;
                    if self.buffer_status[next_slot] == ReplayBufferStatus::Unused {
                        frame.reached_record_slot = true;
                        return Some(frame);
                    }
                    address.slot = next_slot as u8;
                    address.offset = 0;
                    address.has_base = true;
                    frame.wrapped_slots = frame.wrapped_slots.saturating_add(1);
                }
                ReplayPacket::EndOfFrame => {
                    address.offset += REPLAYPACKET_ENDOFFRAME_SIZE as u32;
                    return Some(frame);
                }
                ReplayPacket::Vehicle(packet) => {
                    frame.vehicle_updates.push(packet);
                    address.offset += REPLAYPACKET_VEHICLE_SIZE as u32;
                }
                ReplayPacket::PedHeader(packet) => {
                    frame.ped_headers.push(packet);
                    address.offset += REPLAYPACKET_PED_HEADER_SIZE as u32;
                }
                ReplayPacket::PedUpdate(packet) => {
                    frame.ped_updates.push(packet);
                    address.offset += REPLAYPACKET_PED_UPDATE_SIZE as u32;
                }
                ReplayPacket::General(packet) => {
                    frame.general = Some(packet);
                    address.offset += REPLAYPACKET_GENERAL_SIZE as u32;
                }
                ReplayPacket::Clock(packet) => {
                    frame.clock = Some(packet);
                    address.offset += REPLAYPACKET_CLOCK_SIZE as u32;
                }
                ReplayPacket::Weather(packet) => {
                    frame.weather = Some(packet);
                    address.offset += REPLAYPACKET_WEATHER_SIZE as u32;
                }
                ReplayPacket::Timer(packet) => {
                    frame.timer = Some(packet);
                    address.offset += REPLAYPACKET_TIMER_SIZE as u32;
                }
                ReplayPacket::BulletTraces(packet) => {
                    frame.bullet_traces.push(packet);
                    address.offset += REPLAYPACKET_BULLET_TRACES_SIZE as u32;
                }
            }
        }
    }

    pub fn collect_playback_mutation_plan(
        &self,
        address: &mut CAddressInReplayBuffer,
        interpolation: f32,
    ) -> Option<ReplayWorldMutationPlan> {
        self.collect_playback_frame(address, interpolation)
            .map(|frame| frame.build_world_mutation_plan())
    }

    pub fn apply_non_entity_playback_plan(&mut self, plan: &ReplayWorldMutationPlan) {
        if let Some(focus) = plan.camera_focus {
            let split = 1.0 - plan.interpolation;
            self.camera_focus[0] = split * self.camera_focus[0] + plan.interpolation * focus[0];
            self.camera_focus[1] = split * self.camera_focus[1] + plan.interpolation * focus[1];
            self.camera_focus[2] = split * self.camera_focus[2] + plan.interpolation * focus[2];
        }
        if let Some(in_rc_buggy) = plan.player_in_rc_buggy {
            self.player_in_rc_buggy = in_rc_buggy;
        }
        if let Some(clock) = plan.clock {
            self.clock_hours = clock.hours;
            self.clock_minutes = clock.minutes;
        }
        if let Some(weather) = plan.weather {
            self.old_weather_type = weather.old_weather as u16;
            self.new_weather_type = weather.new_weather as u16;
            self.weather_interpolation_value = weather.interpolation;
        }
        if let Some(timer) = plan.timer {
            self.time1 = timer.timer;
        }
    }

    pub fn find_size_of_packet(packet_type: u8) -> usize {
        match packet_type {
            REPLAYPACKET_END => REPLAYPACKET_END_SIZE,
            REPLAYPACKET_VEHICLE => REPLAYPACKET_VEHICLE_SIZE,
            REPLAYPACKET_PED_HEADER => REPLAYPACKET_PED_HEADER_SIZE,
            REPLAYPACKET_PED_UPDATE => REPLAYPACKET_PED_UPDATE_SIZE,
            REPLAYPACKET_GENERAL => REPLAYPACKET_GENERAL_SIZE,
            REPLAYPACKET_CLOCK => REPLAYPACKET_CLOCK_SIZE,
            REPLAYPACKET_WEATHER => REPLAYPACKET_WEATHER_SIZE,
            REPLAYPACKET_ENDOFFRAME => REPLAYPACKET_ENDOFFRAME_SIZE,
            REPLAYPACKET_TIMER => REPLAYPACKET_TIMER_SIZE,
            REPLAYPACKET_BULLET_TRACES => REPLAYPACKET_BULLET_TRACES_SIZE,
            _ => 0,
        }
    }

    pub fn decode_packet(bytes: &[u8]) -> Result<ReplayPacket, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let packet_type = reader.read::<u8>()?;
        match ReplayPacketKind::from_u8(packet_type).unwrap_or(ReplayPacketKind::End) {
            ReplayPacketKind::End => Ok(ReplayPacket::End),
            ReplayPacketKind::Vehicle => {
                Ok(ReplayPacket::Vehicle(read_vehicle_packet(&mut reader)?))
            }
            ReplayPacketKind::PedHeader => Ok(ReplayPacket::PedHeader(read_ped_header_packet(
                &mut reader,
            )?)),
            ReplayPacketKind::PedUpdate => Ok(ReplayPacket::PedUpdate(read_ped_update_packet(
                &mut reader,
            )?)),
            ReplayPacketKind::General => {
                Ok(ReplayPacket::General(read_general_packet(&mut reader)?))
            }
            ReplayPacketKind::Clock => Ok(ReplayPacket::Clock(read_clock_packet(&mut reader)?)),
            ReplayPacketKind::Weather => {
                Ok(ReplayPacket::Weather(read_weather_packet(&mut reader)?))
            }
            ReplayPacketKind::EndOfFrame => Ok(ReplayPacket::EndOfFrame),
            ReplayPacketKind::Timer => Ok(ReplayPacket::Timer(read_timer_packet(&mut reader)?)),
            ReplayPacketKind::BulletTraces => Ok(ReplayPacket::BulletTraces(
                read_bullet_trace_packet(&mut reader)?,
            )),
        }
    }

    pub fn encode_packet(packet: ReplayPacket) -> Vec<u8> {
        let kind = packet.kind();
        let mut writer = SaveBufWriter::new();
        writer.write(kind as u8);
        match packet {
            ReplayPacket::End => writer.zero(REPLAYPACKET_END_SIZE - 1),
            ReplayPacket::Vehicle(packet) => write_vehicle_packet(packet, &mut writer),
            ReplayPacket::PedHeader(packet) => write_ped_header_packet(packet, &mut writer),
            ReplayPacket::PedUpdate(packet) => write_ped_update_packet(packet, &mut writer),
            ReplayPacket::General(packet) => write_general_packet(packet, &mut writer),
            ReplayPacket::Clock(packet) => write_clock_packet(packet, &mut writer),
            ReplayPacket::Weather(packet) => write_weather_packet(packet, &mut writer),
            ReplayPacket::EndOfFrame => writer.zero(REPLAYPACKET_ENDOFFRAME_SIZE - 1),
            ReplayPacket::Timer(packet) => write_timer_packet(packet, &mut writer),
            ReplayPacket::BulletTraces(packet) => write_bullet_trace_packet(packet, &mut writer),
        }
        debug_assert_eq!(writer.offset(), kind.packet_size());
        writer.into_inner()
    }
}

impl ReplayPacket {
    pub fn kind(&self) -> ReplayPacketKind {
        match self {
            Self::End => ReplayPacketKind::End,
            Self::Vehicle(_) => ReplayPacketKind::Vehicle,
            Self::PedHeader(_) => ReplayPacketKind::PedHeader,
            Self::PedUpdate(_) => ReplayPacketKind::PedUpdate,
            Self::General(_) => ReplayPacketKind::General,
            Self::Clock(_) => ReplayPacketKind::Clock,
            Self::Weather(_) => ReplayPacketKind::Weather,
            Self::EndOfFrame => ReplayPacketKind::EndOfFrame,
            Self::Timer(_) => ReplayPacketKind::Timer,
            Self::BulletTraces(_) => ReplayPacketKind::BulletTraces,
        }
    }
}

fn read_general_packet(reader: &mut SaveBufReader<'_>) -> Result<TGeneralPacket, SaveBufError> {
    let in_rcvehicle = reader.read::<u8>()? != 0;
    reader.skip(2)?;
    let mut camera_matrix = [0.0_f32; 16];
    for value in &mut camera_matrix {
        *value = reader.read::<f32>()?;
    }
    reader.skip(4)?;
    reader.skip(1)?;
    reader.skip(3)?;
    let player_pos = [
        reader.read::<f32>()?,
        reader.read::<f32>()?,
        reader.read::<f32>()?,
    ];
    Ok(TGeneralPacket {
        in_rcvehicle,
        camera_matrix,
        player_pos,
    })
}

fn write_general_packet(packet: TGeneralPacket, writer: &mut SaveBufWriter) {
    writer.write(bool_to_u8(packet.in_rcvehicle));
    writer.zero(2);
    for value in packet.camera_matrix {
        writer.write(value);
    }
    writer.zero(4);
    writer.write(0_u8);
    writer.zero(3);
    for value in packet.player_pos {
        writer.write(value);
    }
}

fn read_clock_packet(reader: &mut SaveBufReader<'_>) -> Result<TClockPacket, SaveBufError> {
    let packet = TClockPacket {
        hours: reader.read::<u8>()?,
        minutes: reader.read::<u8>()?,
    };
    reader.skip(1)?;
    Ok(packet)
}

fn write_clock_packet(packet: TClockPacket, writer: &mut SaveBufWriter) {
    writer.write(packet.hours);
    writer.write(packet.minutes);
    writer.zero(1);
}

fn read_weather_packet(reader: &mut SaveBufReader<'_>) -> Result<TWeatherPacket, SaveBufError> {
    let old_weather = reader.read::<u8>()?;
    let new_weather = reader.read::<u8>()?;
    reader.skip(1)?;
    Ok(TWeatherPacket {
        old_weather,
        new_weather,
        interpolation: reader.read::<f32>()?,
    })
}

fn write_weather_packet(packet: TWeatherPacket, writer: &mut SaveBufWriter) {
    writer.write(packet.old_weather);
    writer.write(packet.new_weather);
    writer.zero(1);
    writer.write(packet.interpolation);
}

fn read_timer_packet(reader: &mut SaveBufReader<'_>) -> Result<TTimerPacket, SaveBufError> {
    reader.skip(3)?;
    Ok(TTimerPacket {
        timer: reader.read::<u32>()?,
    })
}

fn write_timer_packet(packet: TTimerPacket, writer: &mut SaveBufWriter) {
    writer.zero(3);
    writer.write(packet.timer);
}

fn read_ped_header_packet(
    reader: &mut SaveBufReader<'_>,
) -> Result<TPedHeaderPacket, SaveBufError> {
    let packet = TPedHeaderPacket {
        index: reader.read::<u8>()?,
        model_index: reader.read::<u16>()?,
        ped_type: reader.read::<u8>()?,
    };
    reader.skip(3)?;
    Ok(packet)
}

fn write_ped_header_packet(packet: TPedHeaderPacket, writer: &mut SaveBufWriter) {
    writer.write(packet.index);
    writer.write(packet.model_index);
    writer.write(packet.ped_type);
    writer.zero(3);
}

fn read_bullet_trace_packet(
    reader: &mut SaveBufReader<'_>,
) -> Result<TBulletTracePacket, SaveBufError> {
    Ok(TBulletTracePacket {
        frames: reader.read::<u8>()?,
        lifetime: reader.read::<u8>()?,
        index: reader.read::<u8>()?,
        inf: [
            reader.read::<f32>()?,
            reader.read::<f32>()?,
            reader.read::<f32>()?,
        ],
        sup: [
            reader.read::<f32>()?,
            reader.read::<f32>()?,
            reader.read::<f32>()?,
        ],
    })
}

fn write_bullet_trace_packet(packet: TBulletTracePacket, writer: &mut SaveBufWriter) {
    writer.write(packet.frames);
    writer.write(packet.lifetime);
    writer.write(packet.index);
    for value in packet.inf {
        writer.write(value);
    }
    for value in packet.sup {
        writer.write(value);
    }
}

fn read_ped_update_packet(
    reader: &mut SaveBufReader<'_>,
) -> Result<TPedUpdatePacket, SaveBufError> {
    let index = reader.read::<u8>()?;
    let heading = reader.read::<i8>()?;
    let vehicle_index = reader.read::<i8>()?;
    let anim_state = CStoredAnimationStateReplay::read(reader)?;
    reader.skip(1)?;
    let packet = TPedUpdatePacket {
        index,
        heading,
        vehicle_index,
        anim_state,
        matrix: CCompressedMatrixNotAlignedReplay::read(reader)?,
        assoc_group_id: reader.read::<i8>()?,
        weapon_model: reader.read::<u8>()?,
    };
    reader.skip(2)?;
    Ok(packet)
}

fn write_ped_update_packet(packet: TPedUpdatePacket, writer: &mut SaveBufWriter) {
    writer.write(packet.index);
    writer.write(packet.heading);
    writer.write(packet.vehicle_index);
    packet.anim_state.write(writer);
    writer.zero(1);
    packet.matrix.write(writer);
    writer.write(packet.assoc_group_id);
    writer.write(packet.weapon_model);
    writer.zero(2);
}

fn read_vehicle_packet(
    reader: &mut SaveBufReader<'_>,
) -> Result<TVehicleUpdatePacket, SaveBufError> {
    let packet = TVehicleUpdatePacket {
        index: reader.read::<u8>()?,
        health: reader.read::<u8>()?,
        acceleration: reader.read::<u8>()?,
        matrix: CCompressedMatrixNotAlignedReplay::read(reader)?,
        door_angles: [reader.read::<i8>()?, reader.read::<i8>()?],
        model_index: reader.read::<u16>()?,
        panels: reader.read::<u32>()?,
        velocity: [
            reader.read::<i8>()?,
            reader.read::<i8>()?,
            reader.read::<i8>()?,
        ],
        car_gun_or_wheel_state: reader.read::<i8>()?,
        wheel_susp_dist: [
            reader.read::<u8>()?,
            reader.read::<u8>()?,
            reader.read::<u8>()?,
            reader.read::<u8>()?,
        ],
        wheel_rotation: [
            reader.read::<u8>()?,
            reader.read::<u8>()?,
            reader.read::<u8>()?,
            reader.read::<u8>()?,
        ],
        door_status: reader.read::<u8>()?,
        primary_color: reader.read::<u8>()?,
        secondary_color: reader.read::<u8>()?,
    };
    reader.skip(1)?;
    Ok(packet)
}

fn write_vehicle_packet(packet: TVehicleUpdatePacket, writer: &mut SaveBufWriter) {
    writer.write(packet.index);
    writer.write(packet.health);
    writer.write(packet.acceleration);
    packet.matrix.write(writer);
    for value in packet.door_angles {
        writer.write(value);
    }
    writer.write(packet.model_index);
    writer.write(packet.panels);
    for value in packet.velocity {
        writer.write(value);
    }
    writer.write(packet.car_gun_or_wheel_state);
    for value in packet.wheel_susp_dist {
        writer.write(value);
    }
    for value in packet.wheel_rotation {
        writer.write(value);
    }
    writer.write(packet.door_status);
    writer.write(packet.primary_color);
    writer.write(packet.secondary_color);
    writer.zero(1);
}

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

// === FAZ10a: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn service_replay_playback_runtime(
    replay: Option<ResMut<CReplay>>,
    mut replay_runtime: ResMut<ReplayRuntimeWorldState>,
    mut commands: Commands,
    replay_vehicles: Query<(Entity, &ReplayVehicleRuntimeState)>,
    replay_peds: Query<(Entity, &ReplayPedRuntimeState)>,
) {
    let Some(mut replay) = replay else {
        clear_replay_runtime_entities(&mut commands, &replay_vehicles, &replay_peds);
        return;
    };
    if replay.mode != ReplayMode::Playback {
        clear_replay_runtime_entities(&mut commands, &replay_vehicles, &replay_peds);
        return;
    }

    let mut playback = replay.playback;
    let Some(plan) = replay.collect_playback_mutation_plan(&mut playback, 1.0) else {
        clear_replay_runtime_entities(&mut commands, &replay_vehicles, &replay_peds);
        replay.finish_playback();
        return;
    };

    replay.playback = playback;
    replay.apply_non_entity_playback_plan(&plan);
    plan.apply_to_runtime_state(&mut replay_runtime);
    apply_replay_entity_mutations(
        &plan,
        &mut commands,
        &replay_vehicles,
        &replay_peds,
        plan.reached_record_slot,
    );
    if plan.reached_record_slot {
        replay.finish_playback();
    }
}

pub(crate) fn clear_replay_runtime_entities(
    commands: &mut Commands,
    replay_vehicles: &Query<(Entity, &ReplayVehicleRuntimeState)>,
    replay_peds: &Query<(Entity, &ReplayPedRuntimeState)>,
) {
    for (entity, _) in replay_vehicles.iter() {
        commands.entity(entity).despawn();
    }
    for (entity, _) in replay_peds.iter() {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn apply_replay_entity_mutations(
    plan: &ReplayWorldMutationPlan,
    commands: &mut Commands,
    replay_vehicles: &Query<(Entity, &ReplayVehicleRuntimeState)>,
    replay_peds: &Query<(Entity, &ReplayPedRuntimeState)>,
    clear_after: bool,
) {
    // Same-frame command application is deferred, so keep only a transient
    // derived index while folding this replay frame. Persisted entity payload
    // lives exclusively in the components spawned below.
    let mut vehicle_entities: HashMap<u8, (Entity, ReplayVehicleRuntimeState)> = replay_vehicles
        .iter()
        .map(|(entity, state)| (state.replay_index, (entity, *state)))
        .collect();
    let mut ped_entities: HashMap<u8, (Entity, ReplayPedRuntimeState)> = replay_peds
        .iter()
        .map(|(entity, state)| (state.replay_index, (entity, *state)))
        .collect();

    for op in &plan.vehicle_ops {
        match *op {
            ReplayVehicleMutation::RemoveMissingBefore {
                start,
                end_exclusive,
            } => {
                let removed = vehicle_entities
                    .keys()
                    .copied()
                    .filter(|index| *index >= start && *index < end_exclusive)
                    .collect::<Vec<_>>();
                for index in removed {
                    if let Some((entity, _)) = vehicle_entities.remove(&index) {
                        commands.entity(entity).despawn();
                    }
                }
            }
            ReplayVehicleMutation::EnsureExists { index, model_index } => {
                let state = vehicle_entities.get(&index).map_or_else(
                    || ReplayVehicleRuntimeState {
                        replay_index: index,
                        model_index,
                        update: empty_vehicle_update(index, model_index),
                    },
                    |(_, current)| ReplayVehicleRuntimeState {
                        replay_index: index,
                        model_index,
                        update: current.update,
                    },
                );
                let entity = if let Some((entity, _)) = vehicle_entities.get(&index) {
                    commands.entity(*entity).insert(state);
                    *entity
                } else {
                    commands.spawn(state).id()
                };
                vehicle_entities.insert(index, (entity, state));
            }
            ReplayVehicleMutation::ApplyUpdate(update) => {
                let state = ReplayVehicleRuntimeState {
                    replay_index: update.index,
                    model_index: update.model_index,
                    update,
                };
                let entity = if let Some((entity, _)) = vehicle_entities.get(&update.index) {
                    commands.entity(*entity).insert(state);
                    *entity
                } else {
                    commands.spawn(state).id()
                };
                vehicle_entities.insert(update.index, (entity, state));
            }
            ReplayVehicleMutation::RemoveMissingAfter { start } => {
                let removed = vehicle_entities
                    .keys()
                    .copied()
                    .filter(|index| *index >= start)
                    .collect::<Vec<_>>();
                for index in removed {
                    if let Some((entity, _)) = vehicle_entities.remove(&index) {
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }

    for op in &plan.ped_ops {
        match *op {
            ReplayPedMutation::EnsureExists(header) => {
                let state = ped_entities.get(&header.index).map_or(
                    ReplayPedRuntimeState {
                        replay_index: header.index,
                        header: Some(header),
                        update: None,
                    },
                    |(_, current)| ReplayPedRuntimeState {
                        replay_index: header.index,
                        header: Some(header),
                        update: current.update,
                    },
                );
                let entity = if let Some((entity, _)) = ped_entities.get(&header.index) {
                    commands.entity(*entity).insert(state);
                    *entity
                } else {
                    commands.spawn(state).id()
                };
                ped_entities.insert(header.index, (entity, state));
            }
            ReplayPedMutation::RemoveMissingBefore {
                start,
                end_exclusive,
            } => {
                let removed = ped_entities
                    .keys()
                    .copied()
                    .filter(|index| *index >= start && *index < end_exclusive)
                    .collect::<Vec<_>>();
                for index in removed {
                    if let Some((entity, _)) = ped_entities.remove(&index) {
                        commands.entity(entity).despawn();
                    }
                }
            }
            ReplayPedMutation::ApplyUpdate(update) => {
                let state = ReplayPedRuntimeState {
                    replay_index: update.index,
                    header: ped_entities
                        .get(&update.index)
                        .and_then(|(_, current)| current.header),
                    update: Some(update),
                };
                let entity = if let Some((entity, _)) = ped_entities.get(&update.index) {
                    commands.entity(*entity).insert(state);
                    *entity
                } else {
                    commands.spawn(state).id()
                };
                ped_entities.insert(update.index, (entity, state));
            }
            ReplayPedMutation::RemoveMissingAfter { start } => {
                let removed = ped_entities
                    .keys()
                    .copied()
                    .filter(|index| *index >= start)
                    .collect::<Vec<_>>();
                for index in removed {
                    if let Some((entity, _)) = ped_entities.remove(&index) {
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }

    if clear_after {
        for (_, (entity, _)) in vehicle_entities {
            commands.entity(entity).despawn();
        }
        for (_, (entity, _)) in ped_entities {
            commands.entity(entity).despawn();
        }
    }
}
