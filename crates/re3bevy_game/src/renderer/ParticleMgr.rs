//! re3 reference: references/re-GTA/src/renderer/ParticleMgr.cpp
//! re3 reference: references/re-GTA/src/renderer/ParticleMgr.h
//!
//! `PARTICLE.CFG` parser and particle-system table.

use std::io;

use bevy::prelude::Resource;
use re3bevy_assets::install::FileMgr::CFileMgr;

use crate::{objects::ParticleObject::CRGBA, renderer::ParticleType::*};

pub const ZCHECK_FIRST: u32 = 1 << 0;
pub const ZCHECK_STEP: u32 = 1 << 1;
pub const DRAW_OPAQUE: u32 = 1 << 2;
pub const SCREEN_TRAIL: u32 = 1 << 3;
pub const SPEED_TRAIL: u32 = 1 << 4;
pub const RAND_VERT_V: u32 = 1 << 5;
pub const CYCLE_ANIM: u32 = 1 << 6;
pub const DRAW_DARK: u32 = 1 << 7;
pub const VERT_TRAIL: u32 = 1 << 8;
pub const DRAWTOP2D: u32 = 1 << 10;
pub const CLIPOUT2D: u32 = 1 << 11;
pub const ZCHECK_BUMP: u32 = 1 << 12;
pub const ZCHECK_BUMP_FIRST: u32 = 1 << 13;

const PARTICLE_FILENAME: &str = "DATA\\PARTICLE.CFG";

#[derive(Debug, Clone, PartialEq)]
pub struct tParticleSystemData {
    pub particle_type: tParticleType,
    pub name: String,
    pub create_range: f32,
    pub default_initial_radius: f32,
    pub expansion_rate: f32,
    pub z_rotation_initial_angle: u16,
    pub z_rotation_angle_change_amount: i16,
    pub z_rotation_change_time: u16,
    pub z_radius_change_time: u16,
    pub initial_z_radius: f32,
    pub z_radius_change_amount: f32,
    pub fade_to_black_time: u16,
    pub fade_to_black_amount: i16,
    pub fade_to_black_initial_intensity: u8,
    pub fade_alpha_initial_intensity: u8,
    pub fade_alpha_time: u16,
    pub fade_alpha_amount: i16,
    pub start_animation_frame: u16,
    pub final_animation_frame: u16,
    pub animation_speed: u16,
    pub rotation_speed: u16,
    pub gravitational_acceleration: f32,
    pub friction_decceleration: i32,
    pub life_span: i32,
    pub position_random_error: f32,
    pub velocity_random_error: f32,
    pub expansion_rate_error: f32,
    pub rotation_rate_error: i32,
    pub life_span_error_shape: u32,
    pub trail_length_multiplier: f32,
    pub flags: u32,
    pub render_colouring: CRGBA,
    pub initial_color_variation: u8,
    pub fade_destination_color: CRGBA,
    pub color_fade_time: u32,
    pub raster_name: Option<String>,
    pub particle_head: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct cParticleSystemMgr {
    pub particles: Vec<tParticleSystemData>,
}

impl cParticleSystemMgr {
    pub fn initialise(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let text = file_mgr.load_file(PARTICLE_FILENAME, 64 * 1024, "r")?;
        Self::from_particle_cfg(&String::from_utf8_lossy(&text))
    }

    pub fn from_particle_cfg(text: &str) -> io::Result<Self> {
        let mut particles = vec![tParticleSystemData::default(); MAX_PARTICLES as usize];
        let mut particle_type = PARTICLE_FIRST;

        for line in text.lines() {
            let line = line.trim();
            if line == ";the end" {
                break;
            }
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            if particle_type >= MAX_PARTICLES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "PARTICLE.CFG contains more entries than MAX_PARTICLES",
                ));
            }

            let index = particle_type as usize;
            parse_particle_line(line, &mut particles[index], particle_type)?;
            // re3 `CParticle::Initialise` binds each configured system to the
            // authored raster loaded from the PARTICLE TXD. Keep the binding
            // explicit instead of letting the Bevy billboard path fall back to
            // an untextured white quad.
            particles[index].raster_name = particle_raster_name(particle_type).map(str::to_owned);
            particle_type += 1;
        }

        Ok(Self { particles })
    }

    pub fn particle(&self, particle_type: tParticleType) -> Option<&tParticleSystemData> {
        usize::try_from(particle_type)
            .ok()
            .and_then(|index| self.particles.get(index))
    }

    pub fn particle_mut(
        &mut self,
        particle_type: tParticleType,
    ) -> Option<&mut tParticleSystemData> {
        usize::try_from(particle_type)
            .ok()
            .and_then(|index| self.particles.get_mut(index))
    }
}

impl Default for cParticleSystemMgr {
    fn default() -> Self {
        Self {
            particles: vec![tParticleSystemData::default(); MAX_PARTICLES as usize],
        }
    }
}

impl Default for tParticleSystemData {
    fn default() -> Self {
        Self {
            particle_type: PARTICLE_FIRST,
            name: String::new(),
            create_range: 0.0,
            default_initial_radius: 0.0,
            expansion_rate: 0.0,
            z_rotation_initial_angle: 0,
            z_rotation_angle_change_amount: 0,
            z_rotation_change_time: 0,
            z_radius_change_time: 0,
            initial_z_radius: 0.0,
            z_radius_change_amount: 0.0,
            fade_to_black_time: 0,
            fade_to_black_amount: 0,
            fade_to_black_initial_intensity: 0,
            fade_alpha_initial_intensity: 0,
            fade_alpha_time: 0,
            fade_alpha_amount: 0,
            start_animation_frame: 0,
            final_animation_frame: 0,
            animation_speed: 0,
            rotation_speed: 0,
            gravitational_acceleration: 0.0,
            friction_decceleration: 0,
            life_span: 0,
            position_random_error: 0.0,
            velocity_random_error: 0.0,
            expansion_rate_error: 0.0,
            rotation_rate_error: 0,
            life_span_error_shape: 0,
            trail_length_multiplier: 0.0,
            flags: 0,
            render_colouring: CRGBA::new(0, 0, 0, 0),
            initial_color_variation: 0,
            fade_destination_color: CRGBA::new(0, 0, 0, 0),
            color_fade_time: 0,
            raster_name: None,
            particle_head: None,
        }
    }
}

fn parse_particle_line(
    line: &str,
    entry: &mut tParticleSystemData,
    particle_type: tParticleType,
) -> io::Result<()> {
    let mut values = line.split_whitespace();
    entry.particle_type = particle_type;
    entry.name = values.next().unwrap_or_default().to_owned();
    entry.render_colouring.red = parse_atoi_next(&mut values)? as u8;
    entry.render_colouring.green = parse_atoi_next(&mut values)? as u8;
    entry.render_colouring.blue = parse_atoi_next(&mut values)? as u8;
    entry.render_colouring.alpha = 255;
    entry.initial_color_variation = parse_atoi_next(&mut values)?.min(100) as u8;
    entry.fade_destination_color.red = parse_atoi_next(&mut values)? as u8;
    entry.fade_destination_color.green = parse_atoi_next(&mut values)? as u8;
    entry.fade_destination_color.blue = parse_atoi_next(&mut values)? as u8;
    entry.fade_destination_color.alpha = 255;
    entry.color_fade_time = parse_atoi_next(&mut values)? as u32;
    entry.default_initial_radius = parse_atof_next(&mut values)?;
    entry.expansion_rate = parse_atof_next(&mut values)?;
    entry.fade_to_black_initial_intensity = parse_atoi_next(&mut values)? as u8;
    entry.fade_to_black_time = parse_atoi_next(&mut values)? as u16;
    entry.fade_to_black_amount = parse_atoi_next(&mut values)? as i16;
    entry.fade_alpha_initial_intensity = parse_atoi_next(&mut values)? as u8;
    entry.fade_alpha_time = parse_atoi_next(&mut values)? as u16;
    entry.fade_alpha_amount = parse_atoi_next(&mut values)? as i16;
    entry.z_rotation_initial_angle = parse_atoi_next(&mut values)? as u16;
    entry.z_rotation_change_time = parse_atoi_next(&mut values)? as u16;
    entry.z_rotation_angle_change_amount = parse_atoi_next(&mut values)? as i16;
    entry.initial_z_radius = parse_atof_next(&mut values)?;
    entry.z_radius_change_time = parse_atoi_next(&mut values)? as u16;
    entry.z_radius_change_amount = parse_atof_next(&mut values)?;
    entry.animation_speed = parse_atoi_next(&mut values)? as u16;
    entry.start_animation_frame = parse_atoi_next(&mut values)? as u16;
    entry.final_animation_frame = parse_atoi_next(&mut values)? as u16;
    entry.rotation_speed = parse_atoi_next(&mut values)? as u16;
    entry.gravitational_acceleration = parse_atof_next(&mut values)?;
    entry.friction_decceleration = parse_atoi_next(&mut values)?;
    entry.life_span = parse_atoi_next(&mut values)?;
    entry.position_random_error = parse_atof_next(&mut values)?;
    entry.velocity_random_error = parse_atof_next(&mut values)?;
    entry.expansion_rate_error = parse_atof_next(&mut values)?;
    // re3 deliberately uses atoi here. Original PARTICLE.CFG contains values
    // such as `2.5`; C atoi consumes the integer prefix and produces 2.
    entry.rotation_rate_error = parse_atoi_next(&mut values)?;
    entry.life_span_error_shape = parse_atoi_next(&mut values)? as u32;
    entry.trail_length_multiplier = parse_atof_next(&mut values)?;
    let create_range = parse_atof_next(&mut values)?;
    entry.create_range = create_range * create_range;
    entry.flags = parse_atoi_next(&mut values)? as u32;
    Ok(())
}

fn next_cfg_value<'a>(values: &mut impl Iterator<Item = &'a str>) -> io::Result<&'a str> {
    values
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PARTICLE.CFG line truncated"))
}

fn parse_atof_next<'a>(values: &mut impl Iterator<Item = &'a str>) -> io::Result<f32> {
    let raw = next_cfg_value(values)?;
    raw.parse::<f32>().map_err(|_| invalid_cfg_value(raw))
}

/// C `atoi` semantics used by every integer field in re3 ParticleMgr.cpp:
/// optional sign followed by the decimal digit prefix, with parsing stopping
/// at the first non-digit (`"2.5" -> 2`).
fn parse_atoi_next<'a>(values: &mut impl Iterator<Item = &'a str>) -> io::Result<i32> {
    let raw = next_cfg_value(values)?;
    let bytes = raw.as_bytes();
    let mut end = usize::from(
        bytes
            .first()
            .is_some_and(|byte| matches!(byte, b'+' | b'-')),
    );
    let digit_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if end == digit_start {
        return Err(invalid_cfg_value(raw));
    }
    raw[..end]
        .parse::<i32>()
        .map_err(|_| invalid_cfg_value(raw))
}

fn invalid_cfg_value(raw: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid PARTICLE.CFG value `{raw}`"),
    )
}

fn particle_raster_name(particle_type: tParticleType) -> Option<&'static str> {
    particle_raster_names(particle_type).first().copied()
}

/// RenderWare raster arrays selected by `CParticle::Render`. The current
/// animation frame indexes these arrays directly.
pub fn particle_raster_names(particle_type: tParticleType) -> &'static [&'static str] {
    match particle_type {
        PARTICLE_SMOKE
        | PARTICLE_SMOKE_SLOWMOTION
        | PARTICLE_GARAGEPAINT_SPRAY
        | PARTICLE_STEAM
        | PARTICLE_BOAT_SPLASH
        | PARTICLE_WATER_CANNON
        | PARTICLE_EXTINGUISH_STEAM
        | PARTICLE_HELI_DUST
        | PARTICLE_PAINT_SMOKE
        | PARTICLE_BULLETHIT_SMOKE => &["smoke1", "smoke2", "smoke3", "smoke4", "smoke5"],
        PARTICLE_GUNSMOKE2 | PARTICLE_RUBBER_SMOKE => {
            &["rubber1", "rubber2", "rubber3", "rubber4", "rubber5"]
        }
        PARTICLE_RAIN_SPLASH | PARTICLE_RAIN_SPLASH_BIGGROW => {
            &["splash1", "splash2", "splash3", "splash4", "splash5"]
        }
        PARTICLE_WATERSPRAY => &["waterspray1", "waterspray2", "waterspray3"],
        PARTICLE_GUNFLASH | PARTICLE_GUNFLASH_NOANIM => {
            &["gunflash1", "gunflash2", "gunflash3", "gunflash4"]
        }
        PARTICLE_RAIN_SPLASHUP => &["splash_up1", "splash_up2"],
        PARTICLE_SHARD | PARTICLE_RAINDROP | PARTICLE_RAINDROP_2D => {
            &["raindrop1", "raindrop2", "raindrop3", "raindrop4"]
        }
        PARTICLE_EXPLOSION_MEDIUM
        | PARTICLE_EXPLOSION_LARGE
        | PARTICLE_EXPLOSION_MFAST
        | PARTICLE_EXPLOSION_LFAST => &[
            "explo01", "explo02", "explo03", "explo04", "explo05", "explo06",
        ],
        PARTICLE_CAR_SPLASH | PARTICLE_WATER_HYDRANT | PARTICLE_PED_SPLASH => &[
            "carsplash_01",
            "carsplash_02",
            "carsplash_03",
            "carsplash_04",
        ],
        PARTICLE_CAR_DEBRIS | PARTICLE_HELI_DEBRIS => &[
            "cardebris_01",
            "cardebris_02",
            "cardebris_03",
            "cardebris_04",
        ],
        PARTICLE_BIRD_FRONT => &["birdf_01", "birdf_02", "birdf_03", "birdf_04"],
        PARTICLE_BLOOD => &["blood"],
        PARTICLE_BLOOD_SMALL | PARTICLE_BLOOD_SPURT => &["bloodsplat2"],
        PARTICLE_DEBRIS2 => &["gunge"],
        PARTICLE_FLAME | PARTICLE_CARFLAME => &["flame1"],
        PARTICLE_FIREBALL => &["flame5"],
        PARTICLE_BOAT_WAKE => &["wake_old"],
        PARTICLE_SPARK | PARTICLE_SPARK_SMALL | PARTICLE_RAINDROP_SMALL | PARTICLE_HELI_ATTACK => {
            &["rainsmall"]
        }
        PARTICLE_DEBRIS | PARTICLE_TREE_LEAVES => &["gameleaf01_64"],
        PARTICLE_WHEEL_DIRT
        | PARTICLE_STEAM2
        | PARTICLE_STEAM_NY
        | PARTICLE_STEAM_NY_SLOWMOTION
        | PARTICLE_ENGINE_STEAM
        | PARTICLE_BOAT_THRUSTJET
        | PARTICLE_PEDFOOT_DUST
        | PARTICLE_EXHAUST_FUMES => &[
            "smokeII_1",
            "smokeII_2",
            "smokeII_3",
            "smokeII_4",
            "smokeII_5",
        ],
        PARTICLE_CARCOLLISION_DUST | PARTICLE_BURNINGRUBBER_SMOKE => &["collisionsmoke"],
        PARTICLE_WHEEL_WATER | PARTICLE_WATER => &["smoke1"],
        PARTICLE_GUNSHELL_FIRST
        | PARTICLE_GUNSHELL
        | PARTICLE_GUNSHELL_BUMP1
        | PARTICLE_GUNSHELL_BUMP2 => &["gunshell"],
        PARTICLE_ENGINE_SMOKE
        | PARTICLE_ENGINE_SMOKE2
        | PARTICLE_CARFLAME_SMOKE
        | PARTICLE_FIREBALL_SMOKE
        | PARTICLE_TEST => &["cloudmasked"],
        PARTICLE_GUNSMOKE | PARTICLE_SPLASH => &[],
        _ => &[],
    }
}
