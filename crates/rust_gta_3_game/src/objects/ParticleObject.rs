//! re3 reference: references/re-GTA/src/objects/ParticleObject.cpp
//! re3 reference: references/re-GTA/src/objects/ParticleObject.h
//!
//! Particle object state.

use bevy::prelude::{Res, ResMut, Resource};
use rust_gta_3_core::core::GameState::CGameState;
use rust_gta_3_core::core::{Coordinates::CVector, Timer::CTimer};

use crate::audio::AudioLogic::{PositionalAudioSnapshot, process_fire_hydrant};
use crate::audio::DMAudio::cDMAudio;
use crate::audio::audio_enums::AUDIOTYPE_FIREHYDRANT;
use crate::core::Camera::CCamera;

use crate::core::General::CGeneral;
use crate::renderer::Particle::{CParticle, ParticleColour, ParticleEmitter, SIN_COS_TABLE_SIZE};
use crate::renderer::ParticleMgr::cParticleSystemMgr;
use crate::renderer::ParticleType::*;

use crate::save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter};

pub const MAX_PARTICLEOBJECTS: usize = 100;
pub const MAX_AUDIOHYDRANTS: usize = 8;
pub const AEHANDLE_NONE: i32 = -5;
pub const PARTICLE_OBJECT_SAVE_SIZE: usize = 0x88;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum eParticleObjectType {
    POBJECT_PAVEMENT_STEAM = 0,
    POBJECT_PAVEMENT_STEAM_SLOWMOTION = 1,
    POBJECT_WALL_STEAM = 2,
    POBJECT_WALL_STEAM_SLOWMOTION = 3,
    POBJECT_DARK_SMOKE = 4,
    POBJECT_FIRE_HYDRANT = 5,
    POBJECT_CAR_WATER_SPLASH = 6,
    POBJECT_PED_WATER_SPLASH = 7,
    POBJECT_SPLASHES_AROUND = 8,
    POBJECT_SMALL_FIRE = 9,
    POBJECT_BIG_FIRE = 10,
    POBJECT_DRY_ICE = 11,
    POBJECT_DRY_ICE_SLOWMOTION = 12,
    POBJECT_FIRE_TRAIL = 13,
    POBJECT_SMOKE_TRAIL = 14,
    POBJECT_FIREBALL_AND_SMOKE = 15,
    POBJECT_ROCKET_TRAIL = 16,
    POBJECT_EXPLOSION_ONCE = 17,
    POBJECT_CATALINAS_GUNFLASH = 18,
    POBJECT_CATALINAS_SHOTGUNFLASH = 19,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum eParticleObjectState {
    POBJECTSTATE_INITIALISED = 0,
    POBJECTSTATE_UPDATE_CLOSE = 1,
    POBJECTSTATE_UPDATE_FAR = 2,
    POBJECTSTATE_FREE = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CRGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl CRGBA {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CParticleObject {
    pub matrix: [f32; 16],
    pub next: Option<usize>,
    pub prev: Option<usize>,
    pub particle: Option<u64>,
    pub particle_anchor: CVector,
    pub remove_timer: u32,
    pub object_type: u16,
    pub particle_type: u16,
    pub num_effect_cycles: u8,
    pub skip_frames: u8,
    pub frame_counter: u16,
    pub state: eParticleObjectState,
    pub target: CVector,
    pub rand_val: f32,
    pub size: f32,
    pub color: CRGBA,
    pub remove: bool,
    pub creation_chance: i8,
}

impl Default for CParticleObject {
    fn default() -> Self {
        Self::new()
    }
}

impl CParticleObject {
    pub fn new() -> Self {
        Self {
            matrix: identity_matrix(),
            next: None,
            prev: None,
            particle: None,
            particle_anchor: CVector::ZERO,
            remove_timer: 0,
            object_type: 0,
            particle_type: 0,
            num_effect_cycles: 0,
            skip_frames: 0,
            frame_counter: 0,
            state: eParticleObjectState::POBJECTSTATE_INITIALISED,
            target: CVector::ZERO,
            rand_val: 0.0,
            size: 0.0,
            color: CRGBA::new(0, 0, 0, 0),
            remove: false,
            creation_chance: 0,
        }
    }

    pub fn position(&self) -> CVector {
        CVector::new(self.matrix[12], self.matrix[13], self.matrix[14])
    }

    pub fn set_position(&mut self, position: CVector) {
        self.matrix[12] = position.x;
        self.matrix[13] = position.y;
        self.matrix[14] = position.z;
    }

    fn write_compatible_save(&self, writer: &mut SaveBufWriter) {
        writer.zero(4);
        for value in self.matrix {
            writer.write(value);
        }
        writer.zero(4);
        writer.write(0_u8);
        writer.zero(3);
        writer.zero(4);
        writer.zero(4);
        writer.zero(4);
        writer.write(self.remove_timer);
        writer.write(self.object_type as i32);
        writer.write(self.particle_type as i32);
        writer.write(self.num_effect_cycles);
        writer.write(self.skip_frames);
        writer.write(self.frame_counter);
        writer.write(self.state as u16);
        writer.zero(2);
        writer.write(self.target.x);
        writer.write(self.target.y);
        writer.write(self.target.z);
        writer.write(self.rand_val);
        writer.write(self.size);
        writer.write(self.color.red);
        writer.write(self.color.green);
        writer.write(self.color.blue);
        writer.write(self.color.alpha);
        writer.write(bool_to_u8(self.remove));
        writer.write(self.creation_chance);
        writer.zero(2);
    }

    fn read_compatible_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        reader.skip(4)?;
        let mut matrix = [0.0_f32; 16];
        for value in &mut matrix {
            *value = reader.read::<f32>()?;
        }
        reader.skip(4)?;
        let _has_rw_matrix = reader.read::<u8>()?;
        reader.skip(3)?;
        reader.skip(4)?;
        reader.skip(4)?;
        reader.skip(4)?;
        let remove_timer = reader.read::<u32>()?;
        let object_type = reader.read::<i32>()? as u16;
        let particle_type = reader.read::<i32>()? as u16;
        let num_effect_cycles = reader.read::<u8>()?;
        let skip_frames = reader.read::<u8>()?;
        let frame_counter = reader.read::<u16>()?;
        let _saved_state = reader.read::<u16>()?;
        reader.skip(2)?;
        let target = CVector::new(
            reader.read::<f32>()?,
            reader.read::<f32>()?,
            reader.read::<f32>()?,
        );
        let rand_val = reader.read::<f32>()?;
        let size = reader.read::<f32>()?;
        let color = CRGBA::new(
            reader.read::<u8>()?,
            reader.read::<u8>()?,
            reader.read::<u8>()?,
            reader.read::<u8>()?,
        );
        let remove = reader.read::<u8>()? != 0;
        let creation_chance = reader.read::<i8>()?;
        reader.skip(2)?;
        Ok(Self {
            matrix,
            next: None,
            prev: None,
            particle: None,
            particle_anchor: CVector::new(matrix[12], matrix[13], matrix[14]),
            remove_timer,
            object_type,
            particle_type,
            num_effect_cycles,
            skip_frames,
            frame_counter,
            state: eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE,
            target,
            rand_val,
            size,
            color,
            remove,
            creation_chance,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CParticleObjectRuntime {
    pub objects: Vec<CParticleObject>,
    pub close_list_head: Option<usize>,
    pub far_list_head: Option<usize>,
    pub unused_list_head: Option<usize>,
    pub audio_hydrants: [CAudioHydrant; MAX_AUDIOHYDRANTS],
}

impl CParticleObjectRuntime {
    pub fn initialise() -> Self {
        let mut objects = vec![CParticleObject::new(); MAX_PARTICLEOBJECTS];
        for i in 0..MAX_PARTICLEOBJECTS {
            objects[i].prev = i.checked_sub(1);
            objects[i].next = if i == MAX_PARTICLEOBJECTS - 1 {
                None
            } else {
                Some(i + 1)
            };
            objects[i].state = eParticleObjectState::POBJECTSTATE_FREE;
        }
        Self {
            objects,
            close_list_head: None,
            far_list_head: None,
            unused_list_head: Some(0),
            audio_hydrants: [CAudioHydrant::default(); MAX_AUDIOHYDRANTS],
        }
    }

    /// re3 `CParticleObject::AddObject` list allocation and common field setup.
    pub fn add_object(
        &mut self,
        object_type: i32,
        position: CVector,
        target: CVector,
        size: f32,
        life_time: u32,
        color: CRGBA,
        remove: bool,
        time_ms: u32,
    ) -> Option<usize> {
        let index = self.unused_list_head?;
        let next = self.objects[index].next;
        self.unused_list_head = next;
        if let Some(next) = next {
            self.objects[next].prev = None;
        }
        let old_head = self.close_list_head;
        self.objects[index] = CParticleObject::new();
        let object = &mut self.objects[index];
        object.next = old_head;
        object.state = eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE;
        object.object_type = object_type as u16;
        object.set_position(position);
        object.target = target;
        object.num_effect_cycles = 1;
        object.skip_frames = 1;
        object.creation_chance = 0;
        object.remove = remove;
        object.remove_timer = if life_time != 0 {
            time_ms.saturating_add(life_time)
        } else {
            0
        };
        object.color = if color.alpha != 0 {
            color
        } else {
            CRGBA::new(0, 0, 0, 0)
        };
        object.size = size;
        // CParticleObject::AddObject selects the backing particle system and
        // its cadence from the object type before the first UpdateClose.
        match object.object_type as i32 {
            0 | 2 => {
                object.particle_type = PARTICLE_STEAM_NY as u16;
                object.skip_frames = 3;
                object.creation_chance = 8;
            }
            1 | 3 => {
                object.particle_type = PARTICLE_STEAM_NY_SLOWMOTION as u16;
                object.skip_frames = 1;
                object.creation_chance = 8;
            }
            4 => {
                object.particle_type = PARTICLE_STEAM_NY as u16;
                object.skip_frames = 3;
                object.creation_chance = 8;
                object.color = CRGBA::new(16, 16, 16, 255);
            }
            5 => {
                object.particle_type = PARTICLE_WATER_HYDRANT as u16;
                object.num_effect_cycles = 4;
                object.skip_frames = 1;
                object.target = CVector::new(0.0, 0.0, 0.3);
                object.remove_timer = time_ms.saturating_add(5_000);
            }
            6 | 7 => {
                object.particle_type = PARTICLE_CAR_SPLASH as u16;
                object.num_effect_cycles = 0;
                object.skip_frames = 1;
            }
            8 => {
                object.particle_type = PARTICLE_SPLASH as u16;
                object.num_effect_cycles = 15;
                object.skip_frames = 2;
            }
            9 => {
                object.particle_type = PARTICLE_FLAME as u16;
                object.skip_frames = 2;
                object.creation_chance = 2;
                object.target = CVector::ZERO;
            }
            10 => {
                object.particle_type = PARTICLE_FLAME as u16;
                object.skip_frames = 2;
                object.creation_chance = 4;
                object.target = CVector::ZERO;
            }
            11 => {
                object.particle_type = PARTICLE_SMOKE as u16;
                object.skip_frames = 1;
                object.target = CVector::ZERO;
            }
            12 => {
                object.particle_type = PARTICLE_SMOKE_SLOWMOTION as u16;
                object.skip_frames = 1;
                object.target = CVector::ZERO;
            }
            13 => {
                object.particle_type = PARTICLE_EXPLOSION_MEDIUM as u16;
                object.skip_frames = 3;
                object.creation_chance = 2;
                object.rand_val = 0.01;
            }
            14 => {
                object.particle_type = PARTICLE_FIREBALL_SMOKE as u16;
                object.skip_frames = 1;
                object.creation_chance = 2;
                object.rand_val = 0.02;
            }
            15 => {
                object.particle_type = PARTICLE_FLAME as u16;
                object.skip_frames = 1;
                object.creation_chance = 2;
                object.rand_val = 0.1;
            }
            16 => {
                object.particle_type = PARTICLE_FLAME as u16;
                object.skip_frames = 2;
                object.creation_chance = 8;
                object.rand_val = 0.1;
            }
            17 => {
                object.particle_type = PARTICLE_EXPLOSION_LARGE as u16;
                object.skip_frames = 1;
                object.remove_timer = time_ms;
            }
            18 | 19 => {
                object.particle_type = PARTICLE_GUNFLASH_NOANIM as u16;
                object.skip_frames = 1;
                object.remove_timer = time_ms;
                let magnitude = object.target.magnitude();
                if magnitude > f32::EPSILON {
                    object.target.x /= magnitude;
                    object.target.y /= magnitude;
                    object.target.z /= magnitude;
                }
            }
            _ => {}
        }
        if let Some(old_head) = old_head {
            self.objects[old_head].prev = Some(index);
        }
        self.close_list_head = Some(index);
        Some(index)
    }

    /// re3 `CParticleObject::UpdateAll`/`UpdateClose` subset used by the
    /// mission SCM effects. The object itself owns the cadence and lifetime;
    /// only the resulting CParticle intents cross into the renderer.
    pub fn update_all(
        &mut self,
        camera_position: CVector,
        playing_intro: bool,
        frame_counter: u32,
        time_ms: u32,
        general: &mut CGeneral,
        particles: &mut CParticle,
        systems: &cParticleSystemMgr,
    ) {
        let mut particles = particles.emitter(systems, camera_position);
        // Traverse the close list in place instead of snapshotting it into a
        // heap Vec every frame. `next` is fetched before the body runs
        // because removal and close->far moves relink the list; indices are
        // stable (RemoveObject overwrites in place, never swap-removes), so
        // the visit order matches the old snapshot order exactly. The step
        // bound only guards against a corrupted cycle and never triggers on
        // well-formed lists.
        let mut head = self.close_list_head;
        let mut steps = 0;
        let step_limit = self.objects.len();
        while let Some(index) = head {
            if index >= step_limit || steps >= step_limit {
                break;
            }
            steps += 1;
            head = self.objects[index].next;
            if self.objects[index].state != eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE {
                continue;
            }
            let position = self.objects[index].position();
            if !playing_intro
                && (position.x - camera_position.x).powi(2)
                    + (position.y - camera_position.y).powi(2)
                    > 100.0_f32.powi(2)
            {
                if self.objects[index].remove {
                    self.remove_object(index);
                } else {
                    self.move_close_to_far(index);
                }
                continue;
            }

            let should_emit = {
                let object = &mut self.objects[index];
                object.frame_counter = object.frame_counter.wrapping_add(1);
                if object.frame_counter < object.skip_frames as u16 {
                    false
                } else {
                    object.frame_counter = 0;
                    if object.creation_chance == 0 {
                        true
                    } else {
                        let random = general.get_random_number()
                            % object.creation_chance.unsigned_abs() as u16;
                        if object.creation_chance < 0 {
                            random == 0
                        } else {
                            random != 0
                        }
                    }
                }
            };
            if should_emit {
                self.emit_close(index, general, &mut particles);
            }
            if self.objects[index].remove_timer != 0 && self.objects[index].remove_timer < time_ms {
                self.remove_object(index);
            }
        }

        // Same in-place traversal for the far list. `counter` numbers live
        // nodes in head-to-tail order, exactly like the old snapshot
        // enumerate, so the 1/32 sharding is unchanged.
        let mut head = self.far_list_head;
        let mut steps = 0;
        let step_limit = self.objects.len();
        let frame = (frame_counter & 31) as usize;
        let mut counter = 0_usize;
        while let Some(index) = head {
            if index >= step_limit || steps >= step_limit {
                break;
            }
            steps += 1;
            head = self.objects[index].next;
            let counter_now = counter;
            counter += 1;
            if counter_now & 31 != frame
                || self.objects[index].state != eParticleObjectState::POBJECTSTATE_UPDATE_FAR
            {
                continue;
            }
            if self.objects[index].remove_timer != 0 && self.objects[index].remove_timer < time_ms {
                self.remove_object(index);
                continue;
            }
            let position = self.objects[index].position();
            if (position.x - camera_position.x).powi(2) + (position.y - camera_position.y).powi(2)
                < 100.0_f32.powi(2)
            {
                self.move_far_to_close(index);
            }
        }
    }

    fn emit_close(&mut self, index: usize, general: &mut CGeneral, particles: &mut ParticleEmitter<'_>) {
        let object = self.objects[index].clone();
        let pos = object.position();
        let vel = object.target;
        let size = object.size;
        match object.object_type as i32 {
            6 => {
                let smoke = ParticleColour {
                    red: 255,
                    green: 255,
                    blue: 255,
                    alpha: 196,
                };
                let splash_colour = ParticleColour {
                    red: object.color.red,
                    green: object.color.green,
                    blue: object.color.blue,
                    alpha: object.color.alpha,
                };
                let spray_size = general.get_random_number_in_range_f32(1.0, 2.5);
                for i in 0..3 {
                    let angle_index = 90 * i;
                    let angle =
                        angle_index as f32 * std::f32::consts::TAU / SIN_COS_TABLE_SIZE as f32;
                    let f_cos = angle.cos();
                    let f_sin = angle.sin();
                    for (x_sign, y_sign) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                        let splash_pos = CVector::new(
                            pos.x + 2.0 * f_cos * x_sign,
                            pos.y + 2.0 * f_sin * y_sign,
                            pos.z,
                        );
                        let splash_vel = CVector::new(
                            vel.x
                                + general.get_random_number_in_range_f32(-0.5, 0.5)
                                    * f_cos
                                    * x_sign,
                            vel.y
                                + general.get_random_number_in_range_f32(-0.5, 0.5)
                                    * f_sin
                                    * y_sign,
                            vel.z + general.get_random_number_in_range_f32(0.01, 0.03),
                        );
                        particles.add_particle_coloured(
                            PARTICLE_RUBBER_SMOKE,
                            splash_pos,
                            splash_vel,
                            None,
                            spray_size,
                            smoke,
                            0,
                            0,
                            0,
                            0,
                        );
                        particles.add_particle_coloured(
                            PARTICLE_CAR_SPLASH,
                            splash_pos,
                            splash_vel,
                            None,
                            0.0,
                            splash_colour,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                }

                let angle = 180.0 * std::f32::consts::TAU / SIN_COS_TABLE_SIZE as f32;
                let f_cos = angle.cos();
                let f_sin = angle.sin();
                for (x_sign, y_sign) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                    let splash_pos = CVector::new(
                        pos.x + 1.25 * f_cos * x_sign,
                        pos.y + 1.25 * f_sin * y_sign,
                        pos.z,
                    );
                    let splash_vel = CVector::new(
                        vel.x + general.get_random_number_in_range_f32(-0.1, 0.1) * f_cos * x_sign,
                        vel.y + general.get_random_number_in_range_f32(-0.1, 0.1) * f_sin * y_sign,
                        vel.z + general.get_random_number_in_range_f32(0.26, 0.53),
                    );
                    particles.add_particle_coloured(
                        PARTICLE_CAR_SPLASH,
                        splash_pos,
                        splash_vel,
                        None,
                        0.0,
                        splash_colour,
                        0,
                        0,
                        0,
                        0,
                    );
                }
            }
            7 => {
                let smoke = ParticleColour {
                    red: 255,
                    green: 255,
                    blue: 255,
                    alpha: 196,
                };
                let splash_colour = ParticleColour {
                    red: object.color.red,
                    green: object.color.green,
                    blue: object.color.blue,
                    alpha: object.color.alpha,
                };
                for i in 0..3 {
                    let angle_index = 90 * i;
                    let angle =
                        angle_index as f32 * std::f32::consts::TAU / SIN_COS_TABLE_SIZE as f32;
                    let f_cos = angle.cos();
                    let f_sin = angle.sin();
                    for (x_sign, y_sign) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                        let splash_pos = CVector::new(
                            pos.x + 0.75 * f_cos * x_sign,
                            pos.y + 0.75 * f_sin * y_sign,
                            pos.z,
                        );
                        let splash_vel = CVector::new(
                            vel.x + 0.05 * f_cos * x_sign,
                            vel.y + 0.05 * f_sin * y_sign,
                            vel.z + general.get_random_number_in_range_f32(0.04, 0.08),
                        );
                        particles.add_particle_coloured(
                            PARTICLE_RUBBER_SMOKE,
                            splash_pos,
                            splash_vel,
                            None,
                            general.get_random_number_in_range_f32(0.1, 0.8),
                            smoke,
                            0,
                            0,
                            0,
                            0,
                        );
                        particles.add_particle_coloured(
                            PARTICLE_CAR_SPLASH,
                            splash_pos,
                            splash_vel,
                            None,
                            general.get_random_number_in_range_f32(0.1, 0.5),
                            splash_colour,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                }

                let angle = 180.0 * std::f32::consts::TAU / SIN_COS_TABLE_SIZE as f32;
                let f_cos = angle.cos();
                let f_sin = angle.sin();
                for (x_sign, y_sign) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                    let splash_pos = CVector::new(
                        pos.x + 0.5 * f_cos * x_sign,
                        pos.y + 0.5 * f_sin * y_sign,
                        pos.z,
                    );
                    let splash_vel = CVector::new(
                        vel.x
                            + general.get_random_number_in_range_f32(-0.25, 0.25) * f_cos * x_sign,
                        vel.y
                            + general.get_random_number_in_range_f32(-0.25, 0.25) * f_sin * y_sign,
                        vel.z + general.get_random_number_in_range_f32(0.05, 0.25),
                    );
                    particles.add_particle_coloured(
                        PARTICLE_CAR_SPLASH,
                        splash_pos,
                        splash_vel,
                        None,
                        general.get_random_number_in_range_f32(0.4, 1.0),
                        splash_colour,
                        0,
                        0,
                        0,
                        0,
                    );
                }
            }
            8 => {
                let splash_colour = ParticleColour {
                    red: object.color.red,
                    green: object.color.green,
                    blue: object.color.blue,
                    alpha: object.color.alpha,
                };
                for _ in 0..object.num_effect_cycles {
                    let splash_pos = CVector::new(
                        pos.x + general.get_random_number_in_range_f32(-size, size),
                        pos.y + general.get_random_number_in_range_f32(-size, size),
                        pos.z,
                    );
                    let (particle_type, particle_size) = if general.get_random_number() & 1 != 0 {
                        (PARTICLE_RAIN_SPLASH, 0.1)
                    } else {
                        (PARTICLE_RAIN_SPLASHUP, 0.12)
                    };
                    particles.add_particle_coloured(
                        particle_type,
                        splash_pos,
                        CVector::ZERO,
                        None,
                        particle_size,
                        splash_colour,
                        0,
                        0,
                        0,
                        0,
                    );
                }
            }
            10 => {
                let scaled = 0.7 * size;
                let flame_velocity = CVector::new(
                    vel.x,
                    vel.y,
                    general.get_random_number_in_range_f32(0.0125 * scaled, 0.1 * scaled),
                );
                particles.add_particle(
                    PARTICLE_FLAME,
                    pos,
                    flame_velocity,
                    None,
                    0.8 * size,
                    0,
                    0,
                    0,
                    0,
                );
                for _ in 0..4 {
                    let smoke = CVector::new(
                        pos.x + general.get_random_number_in_range_f32(-0.625 * size, 0.625 * size),
                        pos.y + general.get_random_number_in_range_f32(-0.625 * size, 0.625 * size),
                        pos.z + general.get_random_number_in_range_f32(0.625 * size, 3.5 * size),
                    );
                    particles.add_particle(
                        PARTICLE_CARFLAME_SMOKE,
                        smoke,
                        vel,
                        None,
                        0.0,
                        0,
                        0,
                        0,
                        0,
                    );
                }
            }
            9 => {
                let flame_velocity = CVector::new(
                    vel.x,
                    vel.y,
                    general.get_random_number_in_range_f32(0.0125 * size, 0.1 * size),
                );
                particles.add_particle(PARTICLE_FLAME, pos, flame_velocity, None, size, 0, 0, 0, 0);
                let smoke = CVector::new(
                    pos.x + general.get_random_number_in_range_f32(-0.625 * size, 0.625 * size),
                    pos.y + general.get_random_number_in_range_f32(-0.625 * size, 0.625 * size),
                    pos.z + general.get_random_number_in_range_f32(0.625 * size, 2.5 * size),
                );
                particles.add_particle(PARTICLE_CARFLAME_SMOKE, smoke, vel, None, 0.0, 0, 0, 0, 0);
            }
            13 => {
                for _ in 0..object.num_effect_cycles {
                    let mut velocity = vel;
                    if velocity.x != 0.0 {
                        velocity.x += general
                            .get_random_number_in_range_f32(-object.rand_val, object.rand_val);
                    }
                    if velocity.y != 0.0 {
                        velocity.y += general
                            .get_random_number_in_range_f32(-object.rand_val, object.rand_val);
                    }
                    if velocity.z != 0.0 {
                        velocity.z += general
                            .get_random_number_in_range_f32(-object.rand_val, object.rand_val);
                    }
                    particles.add_particle(
                        object.particle_type as i32,
                        pos,
                        velocity,
                        None,
                        size,
                        general.get_random_number_in_range_f32(-6.0, 6.0) as i32,
                        0,
                        0,
                        0,
                    );
                }
            }
            15 => {
                if object.particle.is_none() {
                    let tracking_id = particles.add_tracked_particle(
                        PARTICLE_EXPLOSION_MEDIUM,
                        pos,
                        CVector::new(1.2 * vel.x, 1.2 * vel.y, 1.2 * vel.z),
                        None,
                        1.2 * size,
                        0,
                        0,
                        0,
                        0,
                    );
                    self.objects[index].particle = tracking_id;
                    self.objects[index].particle_anchor = pos;
                } else {
                    let offset = CVector::new(0.35 * vel.x, 0.35 * vel.y, 0.35 * vel.z);
                    let mut fireball_velocity = vel;
                    for _ in 0..object.num_effect_cycles {
                        fireball_velocity.x +=
                            general.get_random_number_in_range_f32(-offset.x, offset.x);
                        fireball_velocity.y +=
                            general.get_random_number_in_range_f32(-offset.y, offset.y);
                        fireball_velocity.z +=
                            general.get_random_number_in_range_f32(-offset.z, offset.z);
                        particles.add_particle(
                            PARTICLE_FIREBALL_SMOKE,
                            pos,
                            fireball_velocity,
                            None,
                            size,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                }
            }
            16 => {
                if object.particle.is_none() {
                    let tracking_id = particles.add_tracked_particle(
                        PARTICLE_EXPLOSION_MEDIUM,
                        pos,
                        vel,
                        None,
                        size,
                        0,
                        0,
                        0,
                        0,
                    );
                    self.objects[index].particle = tracking_id;
                    self.objects[index].particle_anchor = pos;
                } else {
                    let anchor = object
                        .particle
                        .and_then(|tracking_id| particles.tracked_particle_position(tracking_id))
                        .unwrap_or(object.particle_anchor);
                    self.objects[index].particle_anchor = anchor;
                    let fireball_velocity = CVector::new(-0.8 * vel.x, -0.8 * vel.y, -0.8 * vel.z);
                    for _ in 0..object.num_effect_cycles {
                        particles.add_particle(
                            PARTICLE_FIREBALL_SMOKE,
                            anchor,
                            fireball_velocity,
                            None,
                            1.5 * size,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                }
            }
            18 => {
                let scale = if size != 0.0 { size } else { 1.0 };
                let flash = ParticleColour {
                    red: 120,
                    green: 120,
                    blue: 120,
                    alpha: 255,
                };
                let mut flash_pos = pos;
                for (step, flash_size) in [(0.0, 0.12), (0.06, 0.08), (0.04, 0.04)] {
                    flash_pos.x += scale * step * vel.x;
                    flash_pos.y += scale * step * vel.y;
                    flash_pos.z += scale * step * vel.z;
                    particles.add_particle_coloured(
                        PARTICLE_GUNFLASH,
                        flash_pos,
                        CVector::ZERO,
                        None,
                        flash_size * scale,
                        flash,
                        0,
                        0,
                        0,
                        0,
                    );
                }
                particles.add_particle(
                    PARTICLE_GUNSMOKE2,
                    pos,
                    CVector::new(0.1 * vel.x, 0.1 * vel.y, 0.1 * vel.z),
                    None,
                    0.005 * scale,
                    0,
                    0,
                    0,
                    0,
                );
            }
            19 => {
                let scale = if size != 0.0 { size } else { 1.0 };
                let flash = ParticleColour {
                    red: 120,
                    green: 120,
                    blue: 120,
                    alpha: 255,
                };
                let step = CVector::new(
                    scale * 0.1 * vel.x,
                    scale * 0.1 * vel.y,
                    scale * 0.1 * vel.z,
                );
                let mut flash_pos = pos;
                for flash_size in [0.0, 0.15 * scale, 0.2 * scale] {
                    flash_pos.x += step.x;
                    flash_pos.y += step.y;
                    flash_pos.z += step.z;
                    particles.add_particle_coloured(
                        PARTICLE_GUNFLASH,
                        flash_pos,
                        CVector::ZERO,
                        None,
                        flash_size,
                        flash,
                        0,
                        0,
                        0,
                        0,
                    );
                }
                particles.add_particle_coloured(
                    PARTICLE_GUNFLASH,
                    pos,
                    CVector::ZERO,
                    None,
                    0.0,
                    flash,
                    0,
                    0,
                    0,
                    0,
                );
                particles.add_particle(
                    PARTICLE_GUNSMOKE2,
                    pos,
                    CVector::new(0.1 * vel.x, 0.1 * vel.y, 0.1 * vel.z),
                    None,
                    0.1 * scale,
                    0,
                    0,
                    0,
                    0,
                );
            }
            _ => {
                for _ in 0..object.num_effect_cycles {
                    let mut velocity = vel;
                    if object.rand_val != 0.0 {
                        if velocity.x != 0.0 {
                            velocity.x += general
                                .get_random_number_in_range_f32(-object.rand_val, object.rand_val);
                        }
                        if velocity.y != 0.0 {
                            velocity.y += general
                                .get_random_number_in_range_f32(-object.rand_val, object.rand_val);
                        }
                        if velocity.z != 0.0 {
                            velocity.z += general
                                .get_random_number_in_range_f32(-object.rand_val, object.rand_val);
                        }
                    }
                    if object.color.alpha != 0 {
                        particles.add_particle_coloured(
                            object.particle_type as i32,
                            pos,
                            velocity,
                            None,
                            size,
                            ParticleColour {
                                red: object.color.red,
                                green: object.color.green,
                                blue: object.color.blue,
                                alpha: object.color.alpha,
                            },
                            0,
                            0,
                            0,
                            0,
                        );
                    } else {
                        particles.add_particle(
                            object.particle_type as i32,
                            pos,
                            velocity,
                            None,
                            size,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                }
            }
        }
    }

    fn move_close_to_far(&mut self, index: usize) {
        self.unlink_from_list(index, eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE);
        self.link_to_list(index, eParticleObjectState::POBJECTSTATE_UPDATE_FAR);
    }

    fn move_far_to_close(&mut self, index: usize) {
        self.unlink_from_list(index, eParticleObjectState::POBJECTSTATE_UPDATE_FAR);
        self.link_to_list(index, eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE);
    }

    fn unlink_from_list(&mut self, index: usize, state: eParticleObjectState) {
        let previous = self.objects[index].prev;
        let next = self.objects[index].next;
        if let Some(previous) = previous {
            self.objects[previous].next = next;
        } else if state == eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE {
            self.close_list_head = next;
        } else {
            self.far_list_head = next;
        }
        if let Some(next) = next {
            self.objects[next].prev = previous;
        }
        self.objects[index].prev = None;
        self.objects[index].next = None;
    }

    fn link_to_list(&mut self, index: usize, state: eParticleObjectState) {
        let head = if state == eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE {
            &mut self.close_list_head
        } else {
            &mut self.far_list_head
        };
        let old_head = *head;
        self.objects[index].state = state;
        self.objects[index].next = old_head;
        self.objects[index].prev = None;
        if let Some(old_head) = old_head {
            self.objects[old_head].prev = Some(index);
        }
        *head = Some(index);
    }

    /// re3 `CParticleObject::RemoveObject`.
    pub fn remove_object(&mut self, index: usize) {
        let Some(object) = self.objects.get(index) else {
            return;
        };
        if object.state == eParticleObjectState::POBJECTSTATE_FREE {
            return;
        }
        let state = object.state;
        let previous = object.prev;
        let next = object.next;
        if let Some(previous) = previous {
            self.objects[previous].next = next;
        } else {
            match state {
                eParticleObjectState::POBJECTSTATE_UPDATE_CLOSE => self.close_list_head = next,
                eParticleObjectState::POBJECTSTATE_UPDATE_FAR => self.far_list_head = next,
                _ => {}
            }
        }
        if let Some(next) = next {
            self.objects[next].prev = previous;
        }

        let unused_head = self.unused_list_head;
        self.objects[index] = CParticleObject::new();
        self.objects[index].state = eParticleObjectState::POBJECTSTATE_FREE;
        self.objects[index].next = unused_head;
        self.objects[index].prev = None;
        if let Some(unused_head) = unused_head {
            self.objects[unused_head].prev = Some(index);
        }
        self.unused_list_head = Some(index);
    }

    pub fn remove_effects_in_area(&mut self, first: CVector, second: CVector) {
        let min = CVector::new(
            first.x.min(second.x),
            first.y.min(second.y),
            first.z.min(second.z),
        );
        let max = CVector::new(
            first.x.max(second.x),
            first.y.max(second.y),
            first.z.max(second.z),
        );
        let removable: Vec<usize> = self
            .active_indices()
            .into_iter()
            .filter(|&index| {
                let position = self.objects[index].position();
                position.x >= min.x
                    && position.x <= max.x
                    && position.y >= min.y
                    && position.y <= max.y
                    && position.z >= min.z
                    && position.z <= max.z
            })
            .collect();
        for index in removable {
            self.remove_object(index);
        }
    }

    /// Maintain re3's eight CAudioHydrant slots and feed the existing
    /// ProcessFireHydrant audio logic from live particle-object positions.
    pub fn process_hydrant_audio(&mut self, audio: &mut cDMAudio, camera_position: CVector) {
        for slot_index in 0..MAX_AUDIOHYDRANTS {
            let particle_object = self.audio_hydrants[slot_index].particle_object;
            let still_active = particle_object.is_some_and(|index| {
                self.objects.get(index).is_some_and(|object| {
                    object.state != eParticleObjectState::POBJECTSTATE_FREE
                        && object.object_type == eParticleObjectType::POBJECT_FIRE_HYDRANT as u16
                })
            });
            if still_active {
                continue;
            }
            let audio_entity = self.audio_hydrants[slot_index].audio_entity;
            if audio_entity != AEHANDLE_NONE {
                audio.DestroyEntity(audio_entity);
            }
            self.audio_hydrants[slot_index] = CAudioHydrant::default();
        }

        let hydrants = self
            .active_indices()
            .into_iter()
            .filter(|&index| {
                self.objects[index].object_type == eParticleObjectType::POBJECT_FIRE_HYDRANT as u16
            })
            .collect::<Vec<_>>();
        for index in hydrants {
            let slot_index = if let Some(slot) = self
                .audio_hydrants
                .iter()
                .position(|entry| entry.particle_object == Some(index))
            {
                slot
            } else {
                let Some(slot) = self
                    .audio_hydrants
                    .iter()
                    .position(|entry| entry.audio_entity == AEHANDLE_NONE)
                else {
                    continue;
                };
                let audio_entity = audio.CreateEntity(AUDIOTYPE_FIREHYDRANT, Some(index as i32));
                if audio_entity < 0 {
                    continue;
                }
                audio.SetEntityStatus(audio_entity, true);
                self.audio_hydrants[slot] = CAudioHydrant {
                    audio_entity,
                    particle_object: Some(index),
                };
                slot
            };

            let position = self.objects[index].position();
            let audio_entity = self.audio_hydrants[slot_index].audio_entity;
            audio.AudioManager.SetEntityPosition(audio_entity, position);
            let dx = position.x - camera_position.x;
            let dy = position.y - camera_position.y;
            let dz = position.z - camera_position.z;
            let snapshot = PositionalAudioSnapshot {
                audio_entity,
                index: index as u32,
                position,
                distance_squared: dx * dx + dy * dy + dz * dz,
                distance_calculated: false,
            };
            process_fire_hydrant(&mut audio.AudioManager, &snapshot);
        }
    }

    pub fn save_particle(&self) -> Vec<u8> {
        let active_indices = self.active_indices();
        let mut writer = SaveBufWriter::new();
        writer.write(active_indices.len() as i32);
        for index in active_indices {
            self.objects[index].write_compatible_save(&mut writer);
        }
        writer.zero(PARTICLE_OBJECT_SAVE_SIZE);
        writer.into_inner()
    }

    pub fn load_particle(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let num_objects = reader.read::<i32>()?.max(0) as usize;
        let expected = 4 + PARTICLE_OBJECT_SAVE_SIZE * (num_objects + 1);
        if bytes.len() != expected || num_objects > MAX_PARTICLEOBJECTS {
            return Err(SaveBufError::ReadPastEnd {
                offset: bytes.len(),
                len: expected,
                available: bytes.len(),
            });
        }

        let mut runtime = Self::initialise();
        runtime.close_list_head = None;
        runtime.far_list_head = None;
        runtime.unused_list_head = None;

        for index in 0..MAX_PARTICLEOBJECTS {
            runtime.objects[index] = if index < num_objects {
                let mut object = CParticleObject::read_compatible_save(&mut reader)?;
                object.prev = index.checked_sub(1);
                object.next = if index + 1 < num_objects {
                    Some(index + 1)
                } else {
                    None
                };
                object
            } else {
                let mut object = CParticleObject::new();
                object.state = eParticleObjectState::POBJECTSTATE_FREE;
                object.prev = if index == num_objects {
                    None
                } else {
                    index.checked_sub(1)
                };
                object.next = if index == MAX_PARTICLEOBJECTS - 1 {
                    None
                } else {
                    Some(index + 1)
                };
                object
            };
        }
        reader.skip(PARTICLE_OBJECT_SAVE_SIZE)?;
        runtime.close_list_head = (num_objects > 0).then_some(0);
        runtime.unused_list_head = (num_objects < MAX_PARTICLEOBJECTS).then_some(num_objects);
        Ok(runtime)
    }

    fn active_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        self.collect_list_indices(self.close_list_head, &mut indices);
        self.collect_list_indices(self.far_list_head, &mut indices);
        indices
    }

    fn collect_list_indices(&self, mut head: Option<usize>, indices: &mut Vec<usize>) {
        while let Some(index) = head {
            if index >= self.objects.len() || indices.contains(&index) {
                break;
            }
            indices.push(index);
            head = self.objects[index].next;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CAudioHydrant {
    pub audio_entity: i32,
    pub particle_object: Option<usize>,
}

impl Default for CAudioHydrant {
    fn default() -> Self {
        Self {
            audio_entity: AEHANDLE_NONE,
            particle_object: None,
        }
    }
}

fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_fireball_and_rocket_seeds_retry_without_a_phantom_reference() {
        let mut systems = cParticleSystemMgr::default();
        systems.particle_mut(PARTICLE_EXPLOSION_MEDIUM).unwrap().create_range = 4.0;
        for kind in [eParticleObjectType::POBJECT_FIREBALL_AND_SMOKE,
            eParticleObjectType::POBJECT_ROCKET_TRAIL] {
            let mut runtime = CParticleObjectRuntime::initialise();
            let position = CVector::new(0.0, 0.0, 3.0);
            let index = runtime.add_object(kind as i32, position, CVector::ZERO,
                1.0, 0, CRGBA::new(0, 0, 0, 0), false, 0).unwrap();
            let mut particles = CParticle::initialise();
            let mut general = CGeneral::with_seed(4);
            for _ in 0..3 {
                runtime.emit_close(index, &mut general,
                    &mut particles.emitter(&systems, CVector::ZERO));
                assert!(runtime.objects[index].particle.is_none());
                assert!(particles.add_intents.is_empty());
            }
            runtime.objects[index].set_position(CVector::ZERO);
            runtime.emit_close(index, &mut general,
                &mut particles.emitter(&systems, CVector::ZERO));
            assert_eq!(runtime.objects[index].particle, Some(1));
            assert_eq!(particles.add_intents.len(), 1);
            assert!(particles.add_intents[0].range_admitted);
        }
    }

    #[test]
    fn ped_water_splash_emits_original_pc_spray_mix() {
        let mut runtime = CParticleObjectRuntime::initialise();
        runtime.add_object(
            eParticleObjectType::POBJECT_PED_WATER_SPLASH as i32,
            CVector::new(1.0, 2.0, 3.0),
            CVector::new(0.0, 0.0, 0.1),
            0.0,
            200,
            CRGBA::new(80, 90, 100, 64),
            true,
            0,
        );
        let mut particles = CParticle::initialise();
        let mut general = CGeneral::with_seed(1);

        runtime.update_all(CVector::ZERO, false, 0, 1, &mut general, &mut particles, &cParticleSystemMgr::default());

        assert_eq!(particles.add_intents.len(), 28);
        assert_eq!(
            particles
                .add_intents
                .iter()
                .filter(|intent| intent.particle_type == PARTICLE_RUBBER_SMOKE)
                .count(),
            12
        );
        assert_eq!(
            particles
                .add_intents
                .iter()
                .filter(|intent| intent.particle_type == PARTICLE_CAR_SPLASH)
                .count(),
            16
        );
    }

    #[test]
    fn car_water_splash_emits_original_pc_spray_mix() {
        let mut runtime = CParticleObjectRuntime::initialise();
        runtime.add_object(
            eParticleObjectType::POBJECT_CAR_WATER_SPLASH as i32,
            CVector::new(1.0, 2.0, 3.0),
            CVector::new(0.1, 0.2, 0.3),
            0.0,
            75,
            CRGBA::new(80, 90, 100, 160),
            true,
            0,
        );
        let mut particles = CParticle::initialise();
        let mut general = CGeneral::with_seed(3);

        runtime.update_all(CVector::ZERO, false, 0, 1, &mut general, &mut particles, &cParticleSystemMgr::default());

        assert_eq!(particles.add_intents.len(), 28);
        assert_eq!(
            particles
                .add_intents
                .iter()
                .filter(|intent| intent.particle_type == PARTICLE_RUBBER_SMOKE)
                .count(),
            12
        );
        assert_eq!(
            particles
                .add_intents
                .iter()
                .filter(|intent| intent.particle_type == PARTICLE_CAR_SPLASH)
                .count(),
            16
        );
        let spray_sizes = particles
            .add_intents
            .iter()
            .filter(|intent| intent.particle_type == PARTICLE_RUBBER_SMOKE)
            .map(|intent| intent.size)
            .collect::<Vec<_>>();
        assert!(spray_sizes.iter().all(|size| *size == spray_sizes[0]));
    }

    #[test]
    fn rocket_trail_follows_seed_particle_position() {
        let mut runtime = CParticleObjectRuntime::initialise();
        let index = runtime
            .add_object(
                eParticleObjectType::POBJECT_ROCKET_TRAIL as i32,
                CVector::new(1.0, 2.0, 3.0),
                CVector::new(0.1, -0.2, 0.3),
                2.0,
                500,
                CRGBA::new(0, 0, 0, 0),
                true,
                0,
            )
            .unwrap();
        let mut particles = CParticle::initialise();
        let mut general = CGeneral::with_seed(4);

        runtime.emit_close(index, &mut general, &mut particles.emitter(&cParticleSystemMgr::default(), CVector::ZERO));
        let tracking_id = runtime.objects[index].particle.unwrap();
        assert_eq!(particles.add_intents.len(), 1);
        assert_eq!(
            particles.add_intents[0].particle_type,
            PARTICLE_EXPLOSION_MEDIUM
        );
        assert_eq!(particles.add_intents[0].tracking_id, Some(tracking_id));

        let live_position = CVector::new(8.0, 9.0, 10.0);
        particles.set_tracked_particle_position(tracking_id, live_position);
        particles.add_intents.clear();
        runtime.emit_close(index, &mut general, &mut particles.emitter(&cParticleSystemMgr::default(), CVector::ZERO));

        assert_eq!(particles.add_intents.len(), 1);
        let trail = particles.add_intents[0];
        assert_eq!(trail.particle_type, PARTICLE_FIREBALL_SMOKE);
        assert_eq!(trail.position, live_position);
        assert!((trail.direction - CVector::new(-0.08, 0.16, -0.24)).magnitude() < 1.0e-6);
        assert_eq!(trail.size, 3.0);
    }

    #[test]
    fn splashes_around_emits_visible_rain_splash_rasters() {
        let mut runtime = CParticleObjectRuntime::initialise();
        runtime.add_object(
            eParticleObjectType::POBJECT_SPLASHES_AROUND as i32,
            CVector::new(1.0, 2.0, 3.0),
            CVector::ZERO,
            4.5,
            1_500,
            CRGBA::new(0, 0, 0, 0),
            true,
            0,
        );
        let mut particles = CParticle::initialise();
        let mut general = CGeneral::with_seed(2);
        runtime.update_all(CVector::ZERO, false, 0, 1, &mut general, &mut particles, &cParticleSystemMgr::default());
        runtime.update_all(CVector::ZERO, false, 1, 2, &mut general, &mut particles, &cParticleSystemMgr::default());

        assert_eq!(particles.add_intents.len(), 15);
        assert!(particles.add_intents.iter().all(|intent| matches!(
            intent.particle_type,
            PARTICLE_RAIN_SPLASH | PARTICLE_RAIN_SPLASHUP
        )));
    }

    #[test]
    fn remove_effects_in_area_unlinks_both_sides_and_recycles_slots() {
        let mut runtime = CParticleObjectRuntime::initialise();
        let outside = runtime
            .add_object(
                1,
                CVector::new(20.0, 20.0, 20.0),
                CVector::ZERO,
                1.0,
                0,
                CRGBA::new(0, 0, 0, 0),
                false,
                0,
            )
            .unwrap();
        let inside = runtime
            .add_object(
                1,
                CVector::new(2.0, 3.0, 4.0),
                CVector::ZERO,
                1.0,
                0,
                CRGBA::new(0, 0, 0, 0),
                false,
                0,
            )
            .unwrap();

        runtime.remove_effects_in_area(CVector::new(10.0, 10.0, 10.0), CVector::new(0.0, 0.0, 0.0));

        assert_eq!(runtime.close_list_head, Some(outside));
        assert_eq!(runtime.objects[outside].prev, None);
        assert_eq!(
            runtime.objects[inside].state,
            eParticleObjectState::POBJECTSTATE_FREE
        );
        assert_eq!(runtime.unused_list_head, Some(inside));
    }
}

// === FAZ10c-particle: moved from core/Game.rs (verbatim, parity-preserving) ===

/// re3 `CParticle::Update` begins with `CParticleObject::UpdateAll`, after
/// events and before the fire manager. SCM particle objects therefore produce
/// their file-defined CParticle emissions in this process slot.
pub(crate) fn update_particle_objects(
    mut particle_objects: ResMut<CParticleObjectRuntime>,
    mut particles: ResMut<CParticle>,
    systems: Res<cParticleSystemMgr>,
    mut general: ResMut<CGeneral>,
    mut audio: Option<ResMut<cDMAudio>>,
    camera: Res<CCamera>,
    game_state: Res<CGameState>,
    timer: Res<CTimer>,
) {
    particle_objects.update_all(
        camera.position,
        game_state.playing_intro,
        timer.frame_counter(),
        timer.time_in_milliseconds(),
        &mut general,
        &mut particles,
        &systems,
    );
    if let Some(audio) = audio.as_deref_mut() {
        particle_objects.process_hydrant_audio(audio, camera.position);
    }
}
