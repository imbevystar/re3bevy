//! re3 reference: references/re-GTA/src/extras/custompipes.cpp
//! re3 reference: references/re-GTA/src/extras/custompipes.h
//!
//! Extended pipeline settings and time/weather interpolation.

use bevy::prelude::Resource;

pub const NUMWEATHERS: usize = 4;
pub const NUMHOURS: usize = 24;
pub const VEHICLEPIPE_MATFX: i32 = 0;
pub const VEHICLEPIPE_NEO: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterpolatedFloat {
    pub data: [[f32; NUMWEATHERS]; NUMHOURS],
    pub cur_interpolator: u8,
    pub cur_val: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterpolatedColor {
    pub data: [[Color; NUMWEATHERS]; NUMHOURS],
    pub cur_interpolator: u8,
    pub cur_val: Color,
}

pub type InterpolatedLight = InterpolatedColor;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CustomPipesState {
    pub neo_txd_loaded: bool,
    pub rendering_env_map: bool,
    pub env_map_size: i32,
    pub env_map_camera_ready: bool,
    pub env_map_texture_ready: bool,
    pub env_mask_texture_ready: bool,
    pub vehicle_pipe_switch: i32,
    pub vehicle_shininess: f32,
    pub vehicle_specularity: f32,
    pub fresnel: InterpolatedFloat,
    pub power: InterpolatedFloat,
    pub diff_color: InterpolatedLight,
    pub spec_color: InterpolatedLight,
    pub lightmap_enable: bool,
    pub lightmap_mult: f32,
    pub world_lightmap_blend: InterpolatedFloat,
    pub gloss_enable: bool,
    pub gloss_mult: f32,
    pub rimlight_enable: bool,
    pub rimlight_mult: f32,
    pub ramp_start: InterpolatedColor,
    pub ramp_end: InterpolatedColor,
    pub offset: InterpolatedFloat,
    pub scale: InterpolatedFloat,
    pub scaling: InterpolatedFloat,
    pub attached: Vec<CustomPipeAttachment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomPipeAttachment {
    VehicleAtomic(usize),
    VehicleClump(usize),
    WorldAtomic(usize),
    WorldClump(usize),
    GlossAtomic(usize),
    GlossClump(usize),
    RimAtomic(usize),
    RimClump(usize),
}

impl InterpolatedFloat {
    pub fn new(init: f32) -> Self {
        Self {
            data: [[init; NUMWEATHERS]; NUMHOURS],
            cur_interpolator: 61,
            cur_val: init,
        }
    }

    pub fn read(&mut self, value: &str, line: usize, field: usize) -> bool {
        let Ok(parsed) = value.trim().parse::<f32>() else {
            return false;
        };
        if line < NUMHOURS && field < NUMWEATHERS {
            self.data[line][field] = parsed;
            return true;
        }
        false
    }

    pub fn get(&mut self, context: InterpolationContext) -> f32 {
        if self.cur_interpolator != context.second {
            self.cur_val = interp_f32(&self.data, context);
            self.cur_interpolator = context.second;
        }
        self.cur_val
    }
}

impl InterpolatedColor {
    pub fn new(init: Color) -> Self {
        Self {
            data: [[init; NUMWEATHERS]; NUMHOURS],
            cur_interpolator: 61,
            cur_val: init,
        }
    }

    pub fn read(&mut self, value: &str, line: usize, field: usize) -> bool {
        let vals: Vec<f32> = value
            .split_whitespace()
            .filter_map(|raw| raw.parse::<f32>().ok())
            .collect();
        if vals.len() < 3 || line >= NUMHOURS || field >= NUMWEATHERS {
            return false;
        }
        self.data[line][field] = Color {
            r: vals[0],
            g: vals[1],
            b: vals[2],
            a: vals.get(3).copied().unwrap_or(1.0),
        };
        true
    }

    pub fn get(&mut self, context: InterpolationContext) -> Color {
        if self.cur_interpolator != context.second {
            self.cur_val = interp_color(&self.data, context);
            self.cur_interpolator = context.second;
        }
        self.cur_val
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterpolationContext {
    pub hour: usize,
    pub minute: u8,
    pub second: u8,
    pub old_weather: usize,
    pub new_weather: usize,
    pub weather_interpolation: f32,
}

impl CustomPipesState {
    pub fn custom_pipe_register(&mut self) {}

    pub fn custom_pipe_init(&mut self) {
        self.env_map_camera_ready = true;
        self.env_map_texture_ready = true;
    }

    pub fn custom_pipe_shutdown(&mut self) {
        self.env_map_camera_ready = false;
        self.env_map_texture_ready = false;
        self.attached.clear();
    }

    pub fn env_map_render(&mut self) -> bool {
        if self.vehicle_pipe_switch != VEHICLEPIPE_NEO {
            return false;
        }
        self.rendering_env_map = true;
        self.rendering_env_map = false;
        true
    }

    pub fn attach_vehicle_pipe_atomic(&mut self, atomic: usize) {
        self.attached
            .push(CustomPipeAttachment::VehicleAtomic(atomic));
    }

    pub fn attach_vehicle_pipe_clump(&mut self, clump: usize) {
        self.attached
            .push(CustomPipeAttachment::VehicleClump(clump));
    }
}

impl Default for CustomPipesState {
    fn default() -> Self {
        Self {
            neo_txd_loaded: false,
            rendering_env_map: false,
            env_map_size: 128,
            env_map_camera_ready: false,
            env_map_texture_ready: false,
            env_mask_texture_ready: false,
            vehicle_pipe_switch: VEHICLEPIPE_MATFX,
            vehicle_shininess: 0.7,
            vehicle_specularity: 1.0,
            fresnel: InterpolatedFloat::new(0.4),
            power: InterpolatedFloat::new(18.0),
            diff_color: InterpolatedColor::new(Color::BLACK),
            spec_color: InterpolatedColor::new(Color { r: 0.7, g: 0.7, b: 0.7, a: 1.0 }),
            lightmap_enable: false,
            lightmap_mult: 1.0,
            world_lightmap_blend: InterpolatedFloat::new(1.0),
            gloss_enable: false,
            gloss_mult: 1.0,
            rimlight_enable: false,
            rimlight_mult: 1.0,
            ramp_start: InterpolatedColor::new(Color::BLACK),
            ramp_end: InterpolatedColor::new(Color::WHITE),
            offset: InterpolatedFloat::new(0.0),
            scale: InterpolatedFloat::new(1.0),
            scaling: InterpolatedFloat::new(1.0),
            attached: Vec::new(),
        }
    }
}

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
}

fn interp_f32(data: &[[f32; NUMWEATHERS]; NUMHOURS], context: InterpolationContext) -> f32 {
    let (h1, h2, w1, w2, c0, c1, c2, c3) = interp_coefficients(context);
    data[h1][w1] * c0 + data[h2][w1] * c1 + data[h1][w2] * c2 + data[h2][w2] * c3
}

fn interp_color(data: &[[Color; NUMWEATHERS]; NUMHOURS], context: InterpolationContext) -> Color {
    let (h1, h2, w1, w2, c0, c1, c2, c3) = interp_coefficients(context);
    Color {
        r: data[h1][w1].r * c0 + data[h2][w1].r * c1 + data[h1][w2].r * c2 + data[h2][w2].r * c3,
        g: data[h1][w1].g * c0 + data[h2][w1].g * c1 + data[h1][w2].g * c2 + data[h2][w2].g * c3,
        b: data[h1][w1].b * c0 + data[h2][w1].b * c1 + data[h1][w2].b * c2 + data[h2][w2].b * c3,
        a: data[h1][w1].a * c0 + data[h2][w1].a * c1 + data[h1][w2].a * c2 + data[h2][w2].a * c3,
    }
}

fn interp_coefficients(
    context: InterpolationContext,
) -> (usize, usize, usize, usize, f32, f32, f32, f32) {
    let h1 = context.hour % NUMHOURS;
    let h2 = (h1 + 1) % NUMHOURS;
    let w1 = context.old_weather.min(NUMWEATHERS - 1);
    let w2 = context.new_weather.min(NUMWEATHERS - 1);
    let time_interp = (context.second as f32 / 60.0 + context.minute as f32) / 60.0;
    let c0 = (1.0 - time_interp) * (1.0 - context.weather_interpolation);
    let c1 = time_interp * (1.0 - context.weather_interpolation);
    let c2 = (1.0 - time_interp) * context.weather_interpolation;
    let c3 = time_interp * context.weather_interpolation;
    (h1, h2, w1, w2, c0, c1, c2, c3)
}
