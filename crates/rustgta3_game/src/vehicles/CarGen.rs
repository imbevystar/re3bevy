//! re3 reference: references/re-GTA/src/vehicles/CarGen.cpp
//! re3 reference: references/re-GTA/src/vehicles/CarGen.h
//!
//! Car generator table + Process range logic (parked spawn hand-off is intentional).

use bevy::prelude::*;
use rustgta3_core::core::Coordinates::CVector;

use crate::save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter};

pub const NUM_CARGENS: usize = 160;
pub const CAR_GENERATOR_SAVE_SIZE: u32 = 72;
pub const CAR_GENERATORS_GENERAL_DATA_SIZE: u32 = 12;
pub const CAR_GENERATORS_SAVE_SIZE: u32 = SAVE_HEADER_SIZE as u32
    + 4
    + CAR_GENERATORS_GENERAL_DATA_SIZE
    + 4
    + CAR_GENERATOR_SAVE_SIZE * NUM_CARGENS as u32;

/// re3 far/near clip base for generator activation (scaled by camera later if available).
pub const CARGEN_FAR_CLIP: f32 = 120.0;
pub const CARGEN_NEAR_CLIP_OFFSET: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CCarGenerator {
    pub model_index: i32,
    pub pos: CVector,
    pub angle: f32,
    pub color1: i16,
    pub color2: i16,
    pub force_spawn: u8,
    pub alarm: u8,
    pub doorlock: u8,
    pub min_delay: i16,
    pub max_delay: i16,
    pub timer: u32,
    pub vehicle_handle: i32,
    pub uses_remaining: u16,
    pub is_blocking: bool,
    pub inf: CVector,
    pub sup: CVector,
    pub size: f32,
}

impl CCarGenerator {
    pub const fn init() -> Self {
        Self {
            model_index: 0,
            pos: CVector::ZERO,
            angle: 0.0,
            color1: 0,
            color2: 0,
            force_spawn: 0,
            alarm: 0,
            doorlock: 0,
            min_delay: 0,
            max_delay: 0,
            timer: 0,
            vehicle_handle: -1,
            uses_remaining: 0,
            is_blocking: false,
            inf: CVector::ZERO,
            sup: CVector::ZERO,
            size: 0.0,
        }
    }

    pub fn setup(
        &mut self,
        pos: CVector,
        angle: f32,
        model_index: i32,
        color1: i16,
        color2: i16,
        force_spawn: u8,
        alarm: u8,
        doorlock: u8,
        min_delay: u16,
        max_delay: u16,
    ) {
        self.pos = pos;
        self.angle = angle;
        self.model_index = model_index;
        self.color1 = color1;
        self.color2 = color2;
        self.force_spawn = force_spawn;
        self.alarm = alarm;
        self.doorlock = doorlock;
        self.min_delay = min_delay as i16;
        self.max_delay = max_delay as i16;
        self.vehicle_handle = -1;
        self.timer = 1;
        self.uses_remaining = 0;
        self.is_blocking = false;
    }

    /// re3 `CCarGenerator::SwitchOff`
    pub fn switch_off(&mut self, active_count: &mut i32) {
        if self.uses_remaining != 0 {
            self.uses_remaining = 0;
            *active_count = (*active_count - 1).max(0);
        }
    }

    /// re3 `CCarGenerator::SwitchOn`
    pub fn switch_on(&mut self, time_ms: u32, active_count: &mut i32) {
        self.uses_remaining = u16::MAX;
        self.timer = Self::calc_next_gen(time_ms);
        *active_count += 1;
    }

    /// re3 `CCarGenerator::SetUsesRemaining` (after SwitchOn with count ≤ 100).
    pub fn set_uses_remaining(&mut self, uses: u16) {
        self.uses_remaining = uses;
    }

    /// re3 `CCarGenerator::CalcNextGen`
    pub fn calc_next_gen(time_ms: u32) -> u32 {
        time_ms.saturating_add(4)
    }

    /// re3 `CheckIfWithinRangeOfAnyPlayer` (CarGen.cpp:187-205), including the
    /// player-speed dot suppression and the far-clip `m_bIsBlocking` reset.
    pub fn check_if_within_range_of_player(
        &mut self,
        player_pos: CVector,
        player_speed: CVector,
        generation_dist_multiplier: f32,
        generate_even_if_close: bool,
    ) -> bool {
        let dx = player_pos.x - self.pos.x;
        let dy = player_pos.y - self.pos.y;
        let distance = (dx * dx + dy * dy).sqrt();
        let farclip = CARGEN_FAR_CLIP * generation_dist_multiplier;
        let nearclip = farclip - CARGEN_NEAR_CLIP_OFFSET;
        if distance >= farclip {
            if self.is_blocking {
                self.is_blocking = false;
            }
            return false;
        }
        if generate_even_if_close {
            return true;
        }
        if self.is_blocking {
            return false;
        }
        if distance < nearclip {
            return false;
        }
        // re3 DotProduct2D(direction, FindPlayerSpeed()) <= 0: only spawn while
        // the player is not driving towards the generator.
        dx * player_speed.x + dy * player_speed.y <= 0.0
    }

    /// re3 `CCarGenerator::Process` condition for attempting a spawn.
    pub fn should_attempt_spawn(
        &mut self,
        time_ms: u32,
        player_pos: CVector,
        player_speed: CVector,
        generation_dist_multiplier: f32,
        generate_even_if_close: bool,
    ) -> bool {
        if self.vehicle_handle != -1 {
            return false;
        }
        if self.uses_remaining == 0 {
            return false;
        }
        if !(generate_even_if_close || time_ms >= self.timer) {
            return false;
        }
        self.check_if_within_range_of_player(
            player_pos,
            player_speed,
            generation_dist_multiplier,
            generate_even_if_close,
        )
    }
}

impl Default for CCarGenerator {
    fn default() -> Self {
        Self::init()
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CTheCarGenerators {
    pub generate_even_if_player_is_close_counter: i32,
    pub num_of_car_generators: i32,
    pub process_counter: i32,
    pub current_active_count: i32,
    pub car_generator_array: [CCarGenerator; NUM_CARGENS],
}

/// Intent emitted when a generator is ready to spawn a parked car (re3 DoInternalProcessing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarGeneratorSpawnIntent {
    pub generator_index: usize,
    pub model_index: i32,
    pub position: CVector,
    pub angle_degrees: f32,
    pub color1: i16,
    pub color2: i16,
    /// re3 `m_nAlarm` chance 0..100
    pub alarm: u8,
    /// re3 `m_nDoorlock` chance 0..100
    pub doorlock: u8,
}

impl CTheCarGenerators {
    pub fn init() -> Self {
        Self {
            generate_even_if_player_is_close_counter: 0,
            num_of_car_generators: 0,
            process_counter: 0,
            current_active_count: 0,
            car_generator_array: [CCarGenerator::init(); NUM_CARGENS],
        }
    }

    pub fn create_car_generator(
        &mut self,
        pos: CVector,
        angle: f32,
        model_index: i32,
        color1: i16,
        color2: i16,
        force_spawn: u8,
        alarm: u8,
        doorlock: u8,
        min_delay: u16,
        max_delay: u16,
    ) -> i32 {
        let index = self.num_of_car_generators as usize;
        if index >= NUM_CARGENS {
            return -1;
        }
        self.car_generator_array[index].setup(
            pos,
            angle,
            model_index,
            color1,
            color2,
            force_spawn,
            alarm,
            doorlock,
            min_delay,
            max_delay,
        );
        self.num_of_car_generators += 1;
        index as i32
    }

    /// re3 SwitchOn after IPL/script registration of a parked generator.
    pub fn switch_on_generator(&mut self, index: i32, time_ms: u32) {
        let idx = index.max(0) as usize;
        if idx >= NUM_CARGENS || idx >= self.num_of_car_generators as usize {
            return;
        }
        self.car_generator_array[idx].switch_on(time_ms, &mut self.current_active_count);
    }

    /// re3 `COMMAND_SWITCH_CAR_GENERATOR`: 0=off, 1..=100=on+uses, >100=on forever.
    pub fn switch_car_generator(&mut self, index: i32, count: i32, time_ms: u32) {
        let idx = index.max(0) as usize;
        if idx >= NUM_CARGENS {
            return;
        }
        let generator = &mut self.car_generator_array[idx];
        if count == 0 {
            generator.switch_off(&mut self.current_active_count);
        } else if count <= 100 {
            generator.switch_on(time_ms, &mut self.current_active_count);
            generator.set_uses_remaining(count.max(0) as u16);
        } else {
            generator.switch_on(time_ms, &mut self.current_active_count);
        }
    }

    pub fn switch_off_generator(&mut self, index: i32) {
        let idx = index.max(0) as usize;
        if idx >= self.num_of_car_generators as usize {
            return;
        }
        self.car_generator_array[idx].switch_off(&mut self.current_active_count);
    }

    /// re3 `CTheCarGenerators::Process` — stride-4 over generators; returns spawn intents.
    /// Does not invent vehicles: caller may place parked cars via production vehicle insert.
    pub fn process(
        &mut self,
        time_ms: u32,
        player_pos: CVector,
        player_speed: CVector,
        cutscene_processing: bool,
        generation_dist_multiplier: f32,
    ) -> Vec<CarGeneratorSpawnIntent> {
        if cutscene_processing {
            return Vec::new();
        }
        self.process_counter += 1;
        if self.process_counter == 4 {
            self.process_counter = 0;
        }
        let generate_close = self.generate_even_if_player_is_close_counter > 0;
        let mut intents = Vec::new();
        let n = self.num_of_car_generators.max(0) as usize;
        let mut i = self.process_counter.max(0) as usize;
        while i < n {
            let generator = &mut self.car_generator_array[i];
            if generator.should_attempt_spawn(
                time_ms,
                player_pos,
                player_speed,
                generation_dist_multiplier,
                generate_close,
            ) {
                // re3: RequestModel then spawn. We emit intent; vehicle_handle stays -1 until
                // production places a vehicle and writes the handle back.
                intents.push(CarGeneratorSpawnIntent {
                    generator_index: i,
                    model_index: generator.model_index,
                    position: generator.pos,
                    angle_degrees: generator.angle,
                    color1: generator.color1,
                    color2: generator.color2,
                    alarm: generator.alarm,
                    doorlock: generator.doorlock,
                });
            }
            // re3: clear handle if vehicle pool slot gone — handled by caller.
            i += 4;
        }
        if self.generate_even_if_player_is_close_counter > 0 {
            self.generate_even_if_player_is_close_counter -= 1;
        }
        intents
    }

    /// re3 `DoInternalProcessing` blockage branch. A blocked attempt neither
    /// consumes a use nor advances to the normal next-generation timestamp.
    pub fn note_spawn_blocked(&mut self, generator_index: usize) {
        let Some(generator) = self.car_generator_array.get_mut(generator_index) else {
            return;
        };
        generator.timer = generator.timer.saturating_add(4);
    }

    /// re3 `CCarGenerator::Setup` COL-derived bounds (CarGen.cpp:175-177).
    /// `m_fSize` is the maximum magnitude of the authored COL box corners and
    /// is the `CheckForBlockage` query radius. Cached here so the blockage
    /// test never silently disables itself while the model streams.
    pub fn set_col_bounds(&mut self, generator_index: usize, inf: CVector, sup: CVector) {
        let Some(generator) = self.car_generator_array.get_mut(generator_index) else {
            return;
        };
        generator.inf = inf;
        generator.sup = sup;
        generator.size = inf.magnitude().max(sup.magnitude());
    }

    /// re3 `CCarGenerator::Process` pool-slot-gone branch (CarGen.cpp:145-149).
    pub fn note_vehicle_gone(&mut self, generator_index: usize) {
        let Some(generator) = self.car_generator_array.get_mut(generator_index) else {
            return;
        };
        generator.vehicle_handle = -1;
    }

    /// re3 `CCarGenerator::Process` "player took the car" branch (CarGen.cpp:150-156):
    /// the generator stays silent for a minute and is then blocked from spawning
    /// until the player leaves the far clip.
    pub fn note_vehicle_taken_by_player(&mut self, generator_index: usize) {
        let Some(generator) = self.car_generator_array.get_mut(generator_index) else {
            return;
        };
        generator.timer = generator.timer.saturating_add(60000);
        generator.vehicle_handle = -1;
        generator.is_blocking = true;
    }

    /// Finish the non-blocked `DoInternalProcessing` path. re3 consumes a use
    /// even when ground lookup fails; a handle is stored only when construction
    /// actually succeeded.
    pub fn note_spawn_attempt_finished(
        &mut self,
        generator_index: usize,
        time_ms: u32,
        vehicle_handle: Option<i32>,
    ) {
        let Some(generator) = self.car_generator_array.get_mut(generator_index) else {
            return;
        };
        if let Some(vehicle_handle) = vehicle_handle {
            generator.vehicle_handle = vehicle_handle;
        }
        if generator.uses_remaining < u16::MAX {
            generator.uses_remaining = generator.uses_remaining.saturating_sub(1);
        }
        generator.timer = CCarGenerator::calc_next_gen(time_ms);
        if generator.uses_remaining == 0 {
            self.current_active_count = (self.current_active_count - 1).max(0);
        }
    }

    pub fn save_all_car_generators(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write_header(
            *b"CGN\0",
            CAR_GENERATORS_SAVE_SIZE - SAVE_HEADER_SIZE as u32,
        );
        writer.write(CAR_GENERATORS_GENERAL_DATA_SIZE);
        writer.write(self.num_of_car_generators);
        writer.write(self.current_active_count);
        writer.write(self.process_counter as u8);
        writer.write(self.generate_even_if_player_is_close_counter as u8);
        writer.write(0_i16);
        writer.write(CAR_GENERATOR_SAVE_SIZE * NUM_CARGENS as u32);
        for generator in self.car_generator_array {
            write_car_generator(&mut writer, generator);
        }
        writer.into_inner()
    }

    pub fn load_all_car_generators(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut out = Self::init();
        let mut reader = SaveBufReader::new(bytes);
        reader.check_header(
            *b"CGN\0",
            CAR_GENERATORS_SAVE_SIZE - SAVE_HEADER_SIZE as u32,
        )?;
        let general_size = reader.read::<u32>()?;
        if general_size != CAR_GENERATORS_GENERAL_DATA_SIZE {
            return Err(SaveBufError::HeaderSizeMismatch {
                expected: CAR_GENERATORS_GENERAL_DATA_SIZE,
                found: general_size,
            });
        }
        out.num_of_car_generators = reader.read::<i32>()?;
        out.current_active_count = reader.read::<i32>()?;
        out.process_counter = i32::from(reader.read::<u8>()?);
        out.generate_even_if_player_is_close_counter = i32::from(reader.read::<u8>()?);
        reader.skip(2)?;
        let array_size = reader.read::<u32>()?;
        let expected_array_size = CAR_GENERATOR_SAVE_SIZE * NUM_CARGENS as u32;
        if array_size != expected_array_size {
            return Err(SaveBufError::HeaderSizeMismatch {
                expected: expected_array_size,
                found: array_size,
            });
        }
        for generator in &mut out.car_generator_array {
            *generator = read_car_generator(&mut reader)?;
        }
        Ok(out)
    }
}

fn write_car_generator(writer: &mut SaveBufWriter, generator: CCarGenerator) {
    writer.write(generator.model_index);
    write_cvector(writer, generator.pos);
    writer.write(generator.angle);
    writer.write(generator.color1);
    writer.write(generator.color2);
    writer.write(generator.force_spawn);
    writer.write(generator.alarm);
    writer.write(generator.doorlock);
    writer.zero(1);
    writer.write(generator.min_delay);
    writer.write(generator.max_delay);
    writer.write(generator.timer);
    writer.write(generator.vehicle_handle);
    writer.write(generator.uses_remaining);
    writer.write(generator.is_blocking as u8);
    writer.zero(1);
    write_cvector(writer, generator.inf);
    write_cvector(writer, generator.sup);
    writer.write(generator.size);
}

fn read_car_generator(reader: &mut SaveBufReader<'_>) -> Result<CCarGenerator, SaveBufError> {
    let model_index = reader.read::<i32>()?;
    let pos = read_cvector(reader)?;
    let angle = reader.read::<f32>()?;
    let color1 = reader.read::<i16>()?;
    let color2 = reader.read::<i16>()?;
    let force_spawn = reader.read::<u8>()?;
    let alarm = reader.read::<u8>()?;
    let doorlock = reader.read::<u8>()?;
    reader.skip(1)?;
    let min_delay = reader.read::<i16>()?;
    let max_delay = reader.read::<i16>()?;
    let timer = reader.read::<u32>()?;
    let vehicle_handle = reader.read::<i32>()?;
    let uses_remaining = reader.read::<u16>()?;
    let is_blocking = reader.read::<u8>()? != 0;
    reader.skip(1)?;
    let inf = read_cvector(reader)?;
    let sup = read_cvector(reader)?;
    let size = reader.read::<f32>()?;
    Ok(CCarGenerator {
        model_index,
        pos,
        angle,
        color1,
        color2,
        force_spawn,
        alarm,
        doorlock,
        min_delay,
        max_delay,
        timer,
        vehicle_handle,
        uses_remaining,
        is_blocking,
        inf,
        sup,
        size,
    })
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

/// Pure range check for harness / unit exercise of shipped Process logic.
pub fn cargen_in_range(
    gen_pos: CVector,
    player_pos: CVector,
    player_speed: CVector,
    generation_dist_multiplier: f32,
    generate_even_if_close: bool,
    is_blocking: bool,
) -> bool {
    let mut generator = CCarGenerator::init();
    generator.pos = gen_pos;
    generator.is_blocking = is_blocking;
    generator.check_if_within_range_of_player(
        player_pos,
        player_speed,
        generation_dist_multiplier,
        generate_even_if_close,
    )
}
