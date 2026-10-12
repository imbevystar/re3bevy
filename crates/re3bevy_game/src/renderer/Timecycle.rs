//! re3 reference: references/re-GTA/src/renderer/Timecycle.cpp
//! re3 reference: references/re-GTA/src/renderer/Timecycle.h
//!
//! `CTimeCycle::Initialise` data tables and `CTimeCycle::Update` interpolation state.

use std::io;

use bevy::prelude::{Res, ResMut, Resource};
use re3bevy_assets::install::FileMgr::CFileMgr;
use re3bevy_core::core::{Clock::CClock, Coordinates::CVector};

use super::Weather::CWeather;

use crate::{
    animation::CutsceneMgr::CCutsceneMgr,
    core::{
        Camera::{CCamera, MOTION_BLUR_LIGHT_SCENE, MOTION_BLUR_NONE},
        FileLoader::CFileLoader,
    },
};

pub const NUMHOURS: usize = 24;
pub const NUMWEATHERS: usize = 4;

#[derive(Debug, Clone, Resource)]
pub struct CTimeCycle {
    pub ambient_red: TimeCycleI32Table,
    pub ambient_green: TimeCycleI32Table,
    pub ambient_blue: TimeCycleI32Table,
    pub directional_red: TimeCycleI32Table,
    pub directional_green: TimeCycleI32Table,
    pub directional_blue: TimeCycleI32Table,
    pub sky_top_red: TimeCycleI32Table,
    pub sky_top_green: TimeCycleI32Table,
    pub sky_top_blue: TimeCycleI32Table,
    pub sky_bottom_red: TimeCycleI32Table,
    pub sky_bottom_green: TimeCycleI32Table,
    pub sky_bottom_blue: TimeCycleI32Table,
    pub sun_core_red: TimeCycleI32Table,
    pub sun_core_green: TimeCycleI32Table,
    pub sun_core_blue: TimeCycleI32Table,
    pub sun_corona_red: TimeCycleI32Table,
    pub sun_corona_green: TimeCycleI32Table,
    pub sun_corona_blue: TimeCycleI32Table,
    pub sun_size: TimeCycleF32Table,
    pub sprite_size: TimeCycleF32Table,
    pub sprite_brightness: TimeCycleF32Table,
    pub shadow_strength: TimeCycleI16Table,
    pub light_shadow_strength: TimeCycleI16Table,
    pub tree_shadow_strength: TimeCycleI16Table,
    pub far_clip: TimeCycleF32Table,
    pub fog_start: TimeCycleF32Table,
    pub lights_on_ground_brightness: TimeCycleF32Table,
    pub low_clouds_red: TimeCycleI32Table,
    pub low_clouds_green: TimeCycleI32Table,
    pub low_clouds_blue: TimeCycleI32Table,
    pub fluffy_clouds_top_red: TimeCycleI32Table,
    pub fluffy_clouds_top_green: TimeCycleI32Table,
    pub fluffy_clouds_top_blue: TimeCycleI32Table,
    pub fluffy_clouds_bottom_red: TimeCycleI32Table,
    pub fluffy_clouds_bottom_green: TimeCycleI32Table,
    pub fluffy_clouds_bottom_blue: TimeCycleI32Table,
    pub blur_red: TimeCycleF32Table,
    pub blur_green: TimeCycleF32Table,
    pub blur_blue: TimeCycleF32Table,
    pub blur_alpha: TimeCycleF32Table,
    pub fog_reduction: i32,
    pub current_stored_value: i32,
    pub current: TimeCycleCurrent,
    pub vector_to_sun: [CVector; 16],
    pub shadow_front_x: [f32; 16],
    pub shadow_front_y: [f32; 16],
    pub shadow_side_x: [f32; 16],
    pub shadow_side_y: [f32; 16],
    pub shadow_displacement_x: [f32; 16],
    pub shadow_displacement_y: [f32; 16],
}

pub type TimeCycleI32Table = [[i32; NUMWEATHERS]; NUMHOURS];
pub type TimeCycleI16Table = [[i16; NUMWEATHERS]; NUMHOURS];
pub type TimeCycleF32Table = [[f32; NUMWEATHERS]; NUMHOURS];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeCycleUpdateContext {
    pub hour: u8,
    pub minute: u8,
    pub camera_forward_z: f32,
    pub camera_motion_blur_light_scene: bool,
    pub no_rain_from_cullzone: bool,
    pub cutscene_running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeCycleUpdateResult {
    pub motion_blur: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeCycleCurrent {
    pub ambient: [f32; 3],
    pub directional: [f32; 3],
    pub sky_top: [i32; 3],
    pub sky_bottom: [i32; 3],
    pub sun_core: [i32; 3],
    pub sun_corona: [i32; 3],
    pub sun_size: f32,
    pub sprite_size: f32,
    pub sprite_brightness: f32,
    pub shadow_strength: i32,
    pub light_shadow_strength: i32,
    pub tree_shadow_strength: i32,
    pub far_clip: f32,
    pub fog_start: f32,
    pub lights_on_ground_brightness: f32,
    pub low_clouds: [i32; 3],
    pub fluffy_clouds_top: [i32; 3],
    pub fluffy_clouds_bottom: [i32; 3],
    pub blur: [f32; 4],
    pub fog_colour: [i32; 3],
}

impl Default for TimeCycleCurrent {
    fn default() -> Self {
        Self {
            ambient: [0.0; 3],
            directional: [0.0; 3],
            sky_top: [0; 3],
            sky_bottom: [0; 3],
            sun_core: [0; 3],
            sun_corona: [0; 3],
            sun_size: 0.0,
            sprite_size: 0.0,
            sprite_brightness: 0.0,
            shadow_strength: 0,
            light_shadow_strength: 0,
            tree_shadow_strength: 0,
            far_clip: 0.0,
            fog_start: 0.0,
            lights_on_ground_brightness: 0.0,
            low_clouds: [0; 3],
            fluffy_clouds_top: [0; 3],
            fluffy_clouds_bottom: [0; 3],
            blur: [0.0; 4],
            fog_colour: [0; 3],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeCycleRecord {
    pub ambient: [i32; 3],
    pub directional: [i32; 3],
    pub sky_top: [i32; 3],
    pub sky_bottom: [i32; 3],
    pub sun_core: [i32; 3],
    pub sun_corona: [i32; 3],
    pub sun_size: f32,
    pub sprite_size: f32,
    pub sprite_brightness: f32,
    pub shadow_strength: i16,
    pub light_shadow_strength: i16,
    pub tree_shadow_strength: i16,
    pub far_clip: f32,
    pub fog_start: f32,
    pub lights_on_ground_brightness: f32,
    pub low_clouds: [i32; 3],
    pub fluffy_clouds_top: [i32; 3],
    pub fluffy_clouds_bottom: [i32; 3],
    pub blur: [f32; 4],
}

impl CTimeCycle {
    pub const FILENAME: &'static str = "DATA\\TIMECYC.DAT";

    /// re3 `CTimeCycle::Initialise` reloads the authored tables and resets
    /// `m_FogReduction`. The tables are immutable runtime data in this port,
    /// so restart only needs the state reset.
    pub fn reset_for_restart(&mut self) {
        self.fog_reduction = 0;
    }

    pub fn initialise(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let bytes = file_mgr.load_file(Self::FILENAME, 128 * 1024, "rb")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        parse_timecycle(&text)
    }

    pub fn update(
        &mut self,
        weather: &CWeather,
        context: TimeCycleUpdateContext,
    ) -> TimeCycleUpdateResult {
        let h1 = (context.hour as usize) % NUMHOURS;
        let h2 = (h1 + 1) % NUMHOURS;
        let w1 = clamp_weather(weather.old_weather_type);
        let w2 = clamp_weather(weather.new_weather_type);
        let time_interp = context.minute as f32 / 60.0;

        let c0 = (1.0 - time_interp) * (1.0 - weather.interpolation_value);
        let c1 = time_interp * (1.0 - weather.interpolation_value);
        let c2 = (1.0 - time_interp) * weather.interpolation_value;
        let c3 = time_interp * weather.interpolation_value;

        self.current.sky_top = [
            interp_i32(&self.sky_top_red, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sky_top_green, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sky_top_blue, h1, h2, w1, w2, c0, c1, c2, c3),
        ];
        self.current.sky_bottom = [
            interp_i32(&self.sky_bottom_red, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sky_bottom_green, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sky_bottom_blue, h1, h2, w1, w2, c0, c1, c2, c3),
        ];
        self.current.ambient = [
            interp_i32_as_f32(&self.ambient_red, h1, h2, w1, w2, c0, c1, c2, c3) / 255.0,
            interp_i32_as_f32(&self.ambient_green, h1, h2, w1, w2, c0, c1, c2, c3) / 255.0,
            interp_i32_as_f32(&self.ambient_blue, h1, h2, w1, w2, c0, c1, c2, c3) / 255.0,
        ];
        self.current.directional = [
            interp_i32_as_f32(&self.directional_red, h1, h2, w1, w2, c0, c1, c2, c3) / 255.0,
            interp_i32_as_f32(&self.directional_green, h1, h2, w1, w2, c0, c1, c2, c3) / 255.0,
            interp_i32_as_f32(&self.directional_blue, h1, h2, w1, w2, c0, c1, c2, c3) / 255.0,
        ];
        self.current.sun_core = [
            interp_i32(&self.sun_core_red, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sun_core_green, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sun_core_blue, h1, h2, w1, w2, c0, c1, c2, c3),
        ];
        self.current.sun_corona = [
            interp_i32(&self.sun_corona_red, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sun_corona_green, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.sun_corona_blue, h1, h2, w1, w2, c0, c1, c2, c3),
        ];
        self.current.sun_size = interp_f32(&self.sun_size, h1, h2, w1, w2, c0, c1, c2, c3);
        self.current.sprite_size = interp_f32(&self.sprite_size, h1, h2, w1, w2, c0, c1, c2, c3);
        self.current.sprite_brightness =
            interp_f32(&self.sprite_brightness, h1, h2, w1, w2, c0, c1, c2, c3);
        self.current.shadow_strength =
            interp_i16(&self.shadow_strength, h1, h2, w1, w2, c0, c1, c2, c3) as i32;
        self.current.light_shadow_strength =
            interp_i16(&self.light_shadow_strength, h1, h2, w1, w2, c0, c1, c2, c3) as i32;
        self.current.tree_shadow_strength =
            interp_i16(&self.tree_shadow_strength, h1, h2, w1, w2, c0, c1, c2, c3) as i32;
        self.current.far_clip = interp_f32(&self.far_clip, h1, h2, w1, w2, c0, c1, c2, c3);
        self.current.fog_start = interp_f32(&self.fog_start, h1, h2, w1, w2, c0, c1, c2, c3);
        self.current.lights_on_ground_brightness = interp_f32(
            &self.lights_on_ground_brightness,
            h1,
            h2,
            w1,
            w2,
            c0,
            c1,
            c2,
            c3,
        );
        self.current.low_clouds = [
            interp_i32(&self.low_clouds_red, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.low_clouds_green, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(&self.low_clouds_blue, h1, h2, w1, w2, c0, c1, c2, c3),
        ];
        self.current.fluffy_clouds_top = [
            interp_i32(&self.fluffy_clouds_top_red, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_i32(
                &self.fluffy_clouds_top_green,
                h1,
                h2,
                w1,
                w2,
                c0,
                c1,
                c2,
                c3,
            ),
            interp_i32(&self.fluffy_clouds_top_blue, h1, h2, w1, w2, c0, c1, c2, c3),
        ];
        self.current.fluffy_clouds_bottom = [
            interp_i32(
                &self.fluffy_clouds_bottom_red,
                h1,
                h2,
                w1,
                w2,
                c0,
                c1,
                c2,
                c3,
            ),
            interp_i32(
                &self.fluffy_clouds_bottom_green,
                h1,
                h2,
                w1,
                w2,
                c0,
                c1,
                c2,
                c3,
            ),
            interp_i32(
                &self.fluffy_clouds_bottom_blue,
                h1,
                h2,
                w1,
                w2,
                c0,
                c1,
                c2,
                c3,
            ),
        ];
        self.current.blur = [
            interp_f32(&self.blur_red, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_f32(&self.blur_green, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_f32(&self.blur_blue, h1, h2, w1, w2, c0, c1, c2, c3),
            interp_f32(&self.blur_alpha, h1, h2, w1, w2, c0, c1, c2, c3),
        ];

        let motion_blur = if context.camera_motion_blur_light_scene {
            Some(self.current.blur)
        } else {
            None
        };

        if self.fog_reduction != 0 {
            self.current.far_clip = self
                .current
                .far_clip
                .max(self.fog_reduction as f32 / 64.0 * 650.0);
        }
        self.current.fog_colour = [
            (self.current.sky_top[0] + 2 * self.current.sky_bottom[0]) / 3,
            (self.current.sky_top[1] + 2 * self.current.sky_bottom[1]) / 3,
            (self.current.sky_top[2] + 2 * self.current.sky_bottom[2]) / 3,
        ];

        self.current_stored_value = (self.current_stored_value + 1) & 0xF;
        let slot = self.current_stored_value as usize;
        let sun_angle =
            2.0 * std::f32::consts::PI * (context.minute as f32 + context.hour as f32 * 60.0)
                / (24.0 * 60.0);
        let mut sun = CVector::new(sun_angle.sin(), 1.0, 0.2 - sun_angle.cos());
        normalise(&mut sun);
        self.vector_to_sun[slot] = sun;
        let shadow = calc_ped_shadow_values(sun);
        self.shadow_front_x[slot] = shadow.front_x;
        self.shadow_front_y[slot] = shadow.front_y;
        self.shadow_side_x[slot] = shadow.side_x;
        self.shadow_side_y[slot] = shadow.side_y;
        self.shadow_displacement_x[slot] = shadow.displacement_x;
        self.shadow_displacement_y[slot] = shadow.displacement_y;

        if context.camera_forward_z < -0.9
            || (!weather.scripts_force_rain
                && (context.no_rain_from_cullzone || context.cutscene_running))
        {
            self.fog_reduction = (self.fog_reduction + 1).min(64);
        } else {
            self.fog_reduction = (self.fog_reduction - 1).max(0);
        }

        TimeCycleUpdateResult { motion_blur }
    }
}

fn parse_timecycle(text: &str) -> io::Result<CTimeCycle> {
    let mut records = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('/'))
        .map(parse_record);

    let mut out = empty_timecycle();
    for weather in 0..NUMWEATHERS {
        for hour in 0..NUMHOURS {
            let record = records.next().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "DATA\\TIMECYC.DAT has fewer than NUMWEATHERS * NUMHOURS records",
                )
            })??;
            set_record(&mut out, hour, weather, record);
        }
    }

    Ok(out)
}

fn parse_record(line: &str) -> io::Result<TimeCycleRecord> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 40 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DATA\\TIMECYC.DAT row has fewer than 40 fields",
        ));
    }

    Ok(TimeCycleRecord {
        ambient: [
            parse_i32(&parts, 0)?,
            parse_i32(&parts, 1)?,
            parse_i32(&parts, 2)?,
        ],
        directional: [
            parse_i32(&parts, 3)?,
            parse_i32(&parts, 4)?,
            parse_i32(&parts, 5)?,
        ],
        sky_top: [
            parse_i32(&parts, 6)?,
            parse_i32(&parts, 7)?,
            parse_i32(&parts, 8)?,
        ],
        sky_bottom: [
            parse_i32(&parts, 9)?,
            parse_i32(&parts, 10)?,
            parse_i32(&parts, 11)?,
        ],
        sun_core: [
            parse_i32(&parts, 12)?,
            parse_i32(&parts, 13)?,
            parse_i32(&parts, 14)?,
        ],
        sun_corona: [
            parse_i32(&parts, 15)?,
            parse_i32(&parts, 16)?,
            parse_i32(&parts, 17)?,
        ],
        sun_size: parse_f32(&parts, 18)?,
        sprite_size: parse_f32(&parts, 19)?,
        sprite_brightness: parse_f32(&parts, 20)?,
        shadow_strength: parse_i16(&parts, 21)?,
        light_shadow_strength: parse_i16(&parts, 22)?,
        tree_shadow_strength: parse_i16(&parts, 23)?,
        far_clip: parse_f32(&parts, 24)?,
        fog_start: parse_f32(&parts, 25)?,
        lights_on_ground_brightness: parse_f32(&parts, 26)?,
        low_clouds: [
            parse_i32(&parts, 27)?,
            parse_i32(&parts, 28)?,
            parse_i32(&parts, 29)?,
        ],
        fluffy_clouds_top: [
            parse_i32(&parts, 30)?,
            parse_i32(&parts, 31)?,
            parse_i32(&parts, 32)?,
        ],
        fluffy_clouds_bottom: [
            parse_i32(&parts, 33)?,
            parse_i32(&parts, 34)?,
            parse_i32(&parts, 35)?,
        ],
        blur: [
            parse_f32(&parts, 36)?,
            parse_f32(&parts, 37)?,
            parse_f32(&parts, 38)?,
            parse_f32(&parts, 39)?,
        ],
    })
}

fn set_record(out: &mut CTimeCycle, hour: usize, weather: usize, record: TimeCycleRecord) {
    out.ambient_red[hour][weather] = record.ambient[0];
    out.ambient_green[hour][weather] = record.ambient[1];
    out.ambient_blue[hour][weather] = record.ambient[2];
    out.directional_red[hour][weather] = record.directional[0];
    out.directional_green[hour][weather] = record.directional[1];
    out.directional_blue[hour][weather] = record.directional[2];
    out.sky_top_red[hour][weather] = record.sky_top[0];
    out.sky_top_green[hour][weather] = record.sky_top[1];
    out.sky_top_blue[hour][weather] = record.sky_top[2];
    out.sky_bottom_red[hour][weather] = record.sky_bottom[0];
    out.sky_bottom_green[hour][weather] = record.sky_bottom[1];
    out.sky_bottom_blue[hour][weather] = record.sky_bottom[2];
    out.sun_core_red[hour][weather] = record.sun_core[0];
    out.sun_core_green[hour][weather] = record.sun_core[1];
    out.sun_core_blue[hour][weather] = record.sun_core[2];
    out.sun_corona_red[hour][weather] = record.sun_corona[0];
    out.sun_corona_green[hour][weather] = record.sun_corona[1];
    out.sun_corona_blue[hour][weather] = record.sun_corona[2];
    out.sun_size[hour][weather] = record.sun_size;
    out.sprite_size[hour][weather] = record.sprite_size;
    out.sprite_brightness[hour][weather] = record.sprite_brightness;
    out.shadow_strength[hour][weather] = record.shadow_strength;
    out.light_shadow_strength[hour][weather] = record.light_shadow_strength;
    out.tree_shadow_strength[hour][weather] = record.tree_shadow_strength;
    out.far_clip[hour][weather] = record.far_clip;
    out.fog_start[hour][weather] = record.fog_start;
    out.lights_on_ground_brightness[hour][weather] = record.lights_on_ground_brightness;
    out.low_clouds_red[hour][weather] = record.low_clouds[0];
    out.low_clouds_green[hour][weather] = record.low_clouds[1];
    out.low_clouds_blue[hour][weather] = record.low_clouds[2];
    out.fluffy_clouds_top_red[hour][weather] = record.fluffy_clouds_top[0];
    out.fluffy_clouds_top_green[hour][weather] = record.fluffy_clouds_top[1];
    out.fluffy_clouds_top_blue[hour][weather] = record.fluffy_clouds_top[2];
    out.fluffy_clouds_bottom_red[hour][weather] = record.fluffy_clouds_bottom[0];
    out.fluffy_clouds_bottom_green[hour][weather] = record.fluffy_clouds_bottom[1];
    out.fluffy_clouds_bottom_blue[hour][weather] = record.fluffy_clouds_bottom[2];
    out.blur_red[hour][weather] = record.blur[0];
    out.blur_green[hour][weather] = record.blur[1];
    out.blur_blue[hour][weather] = record.blur[2];
    out.blur_alpha[hour][weather] = record.blur[3];
}

pub(super) fn empty_timecycle() -> CTimeCycle {
    CTimeCycle {
        ambient_red: [[0; NUMWEATHERS]; NUMHOURS],
        ambient_green: [[0; NUMWEATHERS]; NUMHOURS],
        ambient_blue: [[0; NUMWEATHERS]; NUMHOURS],
        directional_red: [[0; NUMWEATHERS]; NUMHOURS],
        directional_green: [[0; NUMWEATHERS]; NUMHOURS],
        directional_blue: [[0; NUMWEATHERS]; NUMHOURS],
        sky_top_red: [[0; NUMWEATHERS]; NUMHOURS],
        sky_top_green: [[0; NUMWEATHERS]; NUMHOURS],
        sky_top_blue: [[0; NUMWEATHERS]; NUMHOURS],
        sky_bottom_red: [[0; NUMWEATHERS]; NUMHOURS],
        sky_bottom_green: [[0; NUMWEATHERS]; NUMHOURS],
        sky_bottom_blue: [[0; NUMWEATHERS]; NUMHOURS],
        sun_core_red: [[0; NUMWEATHERS]; NUMHOURS],
        sun_core_green: [[0; NUMWEATHERS]; NUMHOURS],
        sun_core_blue: [[0; NUMWEATHERS]; NUMHOURS],
        sun_corona_red: [[0; NUMWEATHERS]; NUMHOURS],
        sun_corona_green: [[0; NUMWEATHERS]; NUMHOURS],
        sun_corona_blue: [[0; NUMWEATHERS]; NUMHOURS],
        sun_size: [[0.0; NUMWEATHERS]; NUMHOURS],
        sprite_size: [[0.0; NUMWEATHERS]; NUMHOURS],
        sprite_brightness: [[0.0; NUMWEATHERS]; NUMHOURS],
        shadow_strength: [[0; NUMWEATHERS]; NUMHOURS],
        light_shadow_strength: [[0; NUMWEATHERS]; NUMHOURS],
        tree_shadow_strength: [[0; NUMWEATHERS]; NUMHOURS],
        far_clip: [[0.0; NUMWEATHERS]; NUMHOURS],
        fog_start: [[0.0; NUMWEATHERS]; NUMHOURS],
        lights_on_ground_brightness: [[0.0; NUMWEATHERS]; NUMHOURS],
        low_clouds_red: [[0; NUMWEATHERS]; NUMHOURS],
        low_clouds_green: [[0; NUMWEATHERS]; NUMHOURS],
        low_clouds_blue: [[0; NUMWEATHERS]; NUMHOURS],
        fluffy_clouds_top_red: [[0; NUMWEATHERS]; NUMHOURS],
        fluffy_clouds_top_green: [[0; NUMWEATHERS]; NUMHOURS],
        fluffy_clouds_top_blue: [[0; NUMWEATHERS]; NUMHOURS],
        fluffy_clouds_bottom_red: [[0; NUMWEATHERS]; NUMHOURS],
        fluffy_clouds_bottom_green: [[0; NUMWEATHERS]; NUMHOURS],
        fluffy_clouds_bottom_blue: [[0; NUMWEATHERS]; NUMHOURS],
        blur_red: [[0.0; NUMWEATHERS]; NUMHOURS],
        blur_green: [[0.0; NUMWEATHERS]; NUMHOURS],
        blur_blue: [[0.0; NUMWEATHERS]; NUMHOURS],
        blur_alpha: [[0.0; NUMWEATHERS]; NUMHOURS],
        fog_reduction: 0,
        current_stored_value: 0,
        current: TimeCycleCurrent::default(),
        vector_to_sun: [CVector::ZERO; 16],
        shadow_front_x: [0.0; 16],
        shadow_front_y: [0.0; 16],
        shadow_side_x: [0.0; 16],
        shadow_side_y: [0.0; 16],
        shadow_displacement_x: [0.0; 16],
        shadow_displacement_y: [0.0; 16],
    }
}

fn clamp_weather(weather: i16) -> usize {
    weather.clamp(0, (NUMWEATHERS - 1) as i16) as usize
}

#[allow(clippy::too_many_arguments)]
fn interp_i32(
    table: &TimeCycleI32Table,
    h1: usize,
    h2: usize,
    w1: usize,
    w2: usize,
    c0: f32,
    c1: f32,
    c2: f32,
    c3: f32,
) -> i32 {
    interp_i32_as_f32(table, h1, h2, w1, w2, c0, c1, c2, c3) as i32
}

#[allow(clippy::too_many_arguments)]
fn interp_i32_as_f32(
    table: &TimeCycleI32Table,
    h1: usize,
    h2: usize,
    w1: usize,
    w2: usize,
    c0: f32,
    c1: f32,
    c2: f32,
    c3: f32,
) -> f32 {
    table[h1][w1] as f32 * c0
        + table[h2][w1] as f32 * c1
        + table[h1][w2] as f32 * c2
        + table[h2][w2] as f32 * c3
}

#[allow(clippy::too_many_arguments)]
fn interp_i16(
    table: &TimeCycleI16Table,
    h1: usize,
    h2: usize,
    w1: usize,
    w2: usize,
    c0: f32,
    c1: f32,
    c2: f32,
    c3: f32,
) -> i16 {
    (table[h1][w1] as f32 * c0
        + table[h2][w1] as f32 * c1
        + table[h1][w2] as f32 * c2
        + table[h2][w2] as f32 * c3) as i16
}

#[allow(clippy::too_many_arguments)]
fn interp_f32(
    table: &TimeCycleF32Table,
    h1: usize,
    h2: usize,
    w1: usize,
    w2: usize,
    c0: f32,
    c1: f32,
    c2: f32,
    c3: f32,
) -> f32 {
    table[h1][w1] * c0 + table[h2][w1] * c1 + table[h1][w2] * c2 + table[h2][w2] * c3
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PedShadowValues {
    front_x: f32,
    front_y: f32,
    side_x: f32,
    side_y: f32,
    displacement_x: f32,
    displacement_y: f32,
}

fn calc_ped_shadow_values(light_dir: CVector) -> PedShadowValues {
    let mut front_x = -light_dir.x;
    let mut front_y = -light_dir.y;
    let dist = (front_y * front_y + front_x * front_x)
        .sqrt()
        .max(f32::EPSILON);
    let mult = (dist + 1.0) / dist;
    front_x *= mult;
    front_y *= mult;

    PedShadowValues {
        front_x: front_x / 2.0,
        front_y: front_y / 2.0,
        side_x: (-light_dir.y / dist) / 2.0,
        side_y: (light_dir.x / dist) / 2.0,
        displacement_x: -light_dir.x / 2.0,
        displacement_y: -light_dir.y / 2.0,
    }
}

fn normalise(value: &mut CVector) {
    let magnitude = value.magnitude();
    if magnitude > 0.0 {
        value.x /= magnitude;
        value.y /= magnitude;
        value.z /= magnitude;
    }
}

fn parse_i32(parts: &[&str], index: usize) -> io::Result<i32> {
    parts[index]
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn parse_i16(parts: &[&str], index: usize) -> io::Result<i16> {
    parts[index]
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn parse_f32(parts: &[&str], index: usize) -> io::Result<f32> {
    parts[index]
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

// === FAZ10c-timecycle: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn update_timecycle(
    mut timecycle: ResMut<CTimeCycle>,
    weather: Res<CWeather>,
    clock: Res<CClock>,
    cutscene: Res<CCutsceneMgr>,
    file_loader: Res<CFileLoader>,
    mut camera: Option<ResMut<CCamera>>,
) {
    let camera_forward_z = camera.as_ref().map_or(0.0, |camera| camera.orientation.z);
    let camera_motion_blur_light_scene = camera.as_ref().is_some_and(|camera| {
        camera.blur_type == MOTION_BLUR_NONE || camera.blur_type == MOTION_BLUR_LIGHT_SCENE
    });
    let result = timecycle.update(
        &weather,
        TimeCycleUpdateContext {
            hour: clock.hours(),
            minute: clock.minutes(),
            camera_forward_z,
            camera_motion_blur_light_scene,
            no_rain_from_cullzone: file_loader.world.cull_zones.camera_no_rain(),
            cutscene_running: cutscene.running,
        },
    );
    if let (Some(blur), Some(camera)) = (result.motion_blur, camera.as_deref_mut()) {
        // re3 CTimeCycle::Update owns the normal gameplay colour filter every frame.
        camera.set_motion_blur(
            blur[0] as i32,
            blur[1] as i32,
            blur[2] as i32,
            blur[3] as i32,
            MOTION_BLUR_LIGHT_SCENE,
        );
    }
}
