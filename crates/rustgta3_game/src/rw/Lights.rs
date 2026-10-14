//! re3 reference: references/re-GTA/src/rw/Lights.cpp
//! re3 reference: references/re-GTA/src/rw/Lights.h
//!
//! RenderWare world-light state mapped to Bevy light intents.

use bevy::prelude::Resource;
use rustgta3_core::core::Coordinates::CVector;

use crate::renderer::Timecycle::TimeCycleCurrent;

pub const NUMEXTRADIRECTIONALS: usize = 4;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GtaRwLights {
    pub ambient: Option<RwLightState>,
    pub directional: Option<RwLightState>,
    pub extra_directionals: [Option<RwLightState>; NUMEXTRADIRECTIONALS],
    pub light_strengths: [i32; NUMEXTRADIRECTIONALS],
    pub num_extra_dir_lights_in_world: usize,
    pub ambient_light_colour_for_frame: RwRGBAReal,
    pub ambient_light_colour_for_frame_peds_cars_and_objects: RwRGBAReal,
    pub directional_light_colour_for_frame: RwRGBAReal,
    pub ambient_light_colour: RwRGBAReal,
    pub directional_light_colour: RwRGBAReal,
    pub directional_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwLightState {
    pub light_type: RwLightType,
    pub colour: RwRGBAReal,
    pub radius: f32,
    pub direction: CVector,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwLightType {
    Ambient,
    Directional,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RwRGBAReal {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl GtaRwLights {
    pub fn lights_create(&mut self) -> bool {
        self.ambient = Some(RwLightState {
            light_type: RwLightType::Ambient,
            colour: RwRGBAReal::rgb(0.25, 0.25, 0.2),
            radius: 0.0,
            direction: CVector::ZERO,
            flags: 1,
        });
        self.directional = Some(RwLightState {
            light_type: RwLightType::Directional,
            colour: RwRGBAReal::rgb(1.0, 0.85, 0.45),
            radius: 2.0,
            direction: CVector::new(1.0, 1.0, 0.0),
            flags: 1,
        });
        for light in &mut self.extra_directionals {
            *light = Some(RwLightState {
                light_type: RwLightType::Directional,
                colour: RwRGBAReal::rgb(1.0, 0.5, 0.0),
                radius: 2.0,
                direction: CVector::ZERO,
                flags: 0,
            });
        }
        true
    }

    pub fn lights_destroy(&mut self) {
        self.ambient = None;
        self.directional = None;
        self.extra_directionals = [None; NUMEXTRADIRECTIONALS];
        self.num_extra_dir_lights_in_world = 0;
    }

    pub fn set_lights_with_time_of_day_colour(
        &mut self,
        current: &TimeCycleCurrent,
        sun_direction: CVector,
        coronas_lights_mult: f32,
        lightning_flash: bool,
        cam_no_rain: bool,
        prefs_brightness: i32,
    ) {
        self.ambient_light_colour_for_frame = RwRGBAReal::rgb(
            current.ambient[0] * coronas_lights_mult,
            current.ambient[1] * coronas_lights_mult,
            current.ambient[2] * coronas_lights_mult,
        );
        if lightning_flash && !cam_no_rain {
            self.ambient_light_colour_for_frame = RwRGBAReal::rgb(1.0, 1.0, 1.0);
        }
        self.ambient_light_colour_for_frame_peds_cars_and_objects =
            self.ambient_light_colour_for_frame.scale_clamped(1.3);
        if let Some(ambient) = &mut self.ambient {
            ambient.colour = self.ambient_light_colour_for_frame;
        }

        self.directional_light_colour_for_frame = RwRGBAReal::rgb(
            current.directional[0] * coronas_lights_mult,
            current.directional[1] * coronas_lights_mult,
            current.directional[2] * coronas_lights_mult,
        );
        if let Some(directional) = &mut self.directional {
            directional.colour = self.directional_light_colour_for_frame;
            directional.direction =
                CVector::new(-sun_direction.x, -sun_direction.y, -sun_direction.z);
        }

        if prefs_brightness > 256 {
            let excess = prefs_brightness as f32 / 256.0 - 1.0;
            let f1 = 2.0 * excess * 0.6 + 1.0;
            let f2 = 3.0 * excess * 0.6 + 1.0;
            self.ambient_light_colour_for_frame =
                self.ambient_light_colour_for_frame.scale_clamped(f2);
            self.ambient_light_colour_for_frame_peds_cars_and_objects = self
                .ambient_light_colour_for_frame_peds_cars_and_objects
                .scale_clamped(f1);
            self.directional_light_colour_for_frame =
                self.directional_light_colour_for_frame.scale_clamped(f1);
        }
    }

    pub fn world_replace_normal_lights_with_scorched(&mut self, level: f32) {
        let colour = RwRGBAReal::rgb(level, level, level);
        if let Some(ambient) = &mut self.ambient {
            ambient.colour = colour;
        }
        if let Some(directional) = &mut self.directional {
            directional.flags = 0;
        }
    }

    pub fn world_replace_scorched_lights_with_normal(&mut self) {
        if let Some(ambient) = &mut self.ambient {
            ambient.colour = self.ambient_light_colour_for_frame;
        }
        if let Some(directional) = &mut self.directional {
            directional.flags = u32::from(self.directional_active);
        }
    }

    pub fn add_an_extra_directional_light(
        &mut self,
        direction: CVector,
        red: f32,
        green: f32,
        blue: f32,
    ) -> bool {
        if self.num_extra_dir_lights_in_world >= NUMEXTRADIRECTIONALS {
            return false;
        }
        let index = self.num_extra_dir_lights_in_world;
        if let Some(light) = &mut self.extra_directionals[index] {
            light.direction = direction;
            light.colour = RwRGBAReal::rgb(red, green, blue);
            light.flags = 1;
        }
        self.light_strengths[index] = ((red + green + blue) * 255.0) as i32;
        self.num_extra_dir_lights_in_world += 1;
        true
    }

    pub fn remove_extra_directional_lights(&mut self) {
        for light in &mut self.extra_directionals {
            if let Some(light) = light {
                light.flags = 0;
            }
        }
        self.num_extra_dir_lights_in_world = 0;
    }

    pub fn deactivate_directional(&mut self) {
        self.directional_active = false;
        if let Some(directional) = &mut self.directional {
            directional.flags = 0;
        }
    }

    pub fn activate_directional(&mut self) {
        self.directional_active = true;
        if let Some(directional) = &mut self.directional {
            directional.flags = 1;
        }
    }
}

impl Default for GtaRwLights {
    fn default() -> Self {
        let mut lights = Self {
            ambient: None,
            directional: None,
            extra_directionals: [None; NUMEXTRADIRECTIONALS],
            light_strengths: [0; NUMEXTRADIRECTIONALS],
            num_extra_dir_lights_in_world: 0,
            ambient_light_colour_for_frame: RwRGBAReal::default(),
            ambient_light_colour_for_frame_peds_cars_and_objects: RwRGBAReal::default(),
            directional_light_colour_for_frame: RwRGBAReal::default(),
            ambient_light_colour: RwRGBAReal::default(),
            directional_light_colour: RwRGBAReal::default(),
            directional_active: true,
        };
        lights.lights_create();
        lights
    }
}

impl RwRGBAReal {
    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    pub fn scale_clamped(self, scale: f32) -> Self {
        Self {
            red: (self.red * scale).min(1.0),
            green: (self.green * scale).min(1.0),
            blue: (self.blue * scale).min(1.0),
            alpha: self.alpha,
        }
    }
}
