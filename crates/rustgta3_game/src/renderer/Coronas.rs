//! re3 reference: references/re-GTA/src/renderer/Coronas.cpp
//! re3 reference: references/re-GTA/src/renderer/Coronas.h
//!
//! Registered corona list + render intents for Bevy billboards.

use bevy::prelude::*;
use rustgta3_core::core::Coordinates::CVector;

pub const NUMCORONAS: usize = 128;
pub const CORONA_SPRITE_NAMES: [&str; 9] = [
    "coronastar",
    "corona",
    "coronamoon",
    "coronareflect",
    "coronaheadlightline",
    "coronahex",
    "coronacircle",
    "coronaringa",
    "streek",
];

pub const TYPE_STAR: u8 = 0;
pub const TYPE_NORMAL: u8 = 1;
pub const TYPE_MOON: u8 = 2;
pub const TYPE_REFLECT: u8 = 3;
pub const TYPE_HEADLIGHT: u8 = 4;
pub const TYPE_HEX: u8 = 5;
pub const TYPE_CIRCLE: u8 = 6;
pub const TYPE_RING: u8 = 7;
pub const TYPE_STREAK: u8 = 8;

pub const FLARE_NONE: i8 = 0;
pub const FLARE_SUN: i8 = 1;
pub const FLARE_HEADLIGHTS: i8 = 2;

pub const REFLECTION_OFF: u8 = 0;
pub const REFLECTION_ON: u8 = 1;
pub const LOSCHECK_OFF: u8 = 0;
pub const LOSCHECK_ON: u8 = 1;
pub const STREAK_OFF: u8 = 0;
pub const STREAK_ON: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CRegisteredCorona {
    pub id: u32,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
    pub fade_alpha: u8,
    pub coors: CVector,
    pub size: f32,
    pub some_angle: f32,
    pub registered_this_frame: bool,
    pub draw_dist: f32,
    pub flare_type: i8,
    pub reflection: u8,
    pub texture_type: u8,
    pub los_check: bool,
    pub off_screen: bool,
    pub first_update: bool,
    pub draw_streak: bool,
    pub sight_clear: bool,
}

impl Default for CRegisteredCorona {
    fn default() -> Self {
        Self {
            id: 0,
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
            fade_alpha: 0,
            coors: CVector::ZERO,
            size: 0.0,
            some_angle: 0.0,
            registered_this_frame: false,
            draw_dist: 0.0,
            flare_type: FLARE_NONE,
            reflection: REFLECTION_OFF,
            texture_type: TYPE_STAR,
            los_check: false,
            off_screen: false,
            first_update: true,
            draw_streak: false,
            sight_clear: true,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CCoronas {
    pub corona_ids: [u32; NUMCORONAS],
    pub coronas: [CRegisteredCorona; NUMCORONAS],
    pub texture_names: [&'static str; 9],
    pub small_moon: bool,
    pub change_brightness_immediately: i32,
    pub lights_mult: f32,
    pub sun_screen_x: f32,
    pub sun_screen_y: f32,
    pub sun_blocked_by_clouds: bool,
}

impl CCoronas {
    pub fn init() -> Self {
        Self {
            corona_ids: [0; NUMCORONAS],
            coronas: [CRegisteredCorona::default(); NUMCORONAS],
            texture_names: CORONA_SPRITE_NAMES,
            small_moon: false,
            change_brightness_immediately: 0,
            lights_mult: 1.0,
            sun_screen_x: 0.0,
            sun_screen_y: 0.0,
            sun_blocked_by_clouds: false,
        }
    }

    /// re3 `CCoronas::RegisterCorona` (type overload).
    pub fn register_corona(
        &mut self,
        id: u32,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
        coors: CVector,
        size: f32,
        draw_dist: f32,
        texture_type: u8,
        flare_type: i8,
        reflection: u8,
        los_check: u8,
        draw_streak: u8,
        some_angle: f32,
    ) {
        if alpha == 0 || size == 0.0 || draw_dist == 0.0 {
            // Fade out existing registration with this id.
            if let Some(slot) = self.coronas.iter_mut().find(|c| c.id == id && c.id != 0) {
                slot.alpha = 0;
                slot.registered_this_frame = true;
            }
            return;
        }

        if let Some(existing) = self.coronas.iter_mut().find(|c| c.id == id && c.id != 0) {
            existing.red = red;
            existing.green = green;
            existing.blue = blue;
            existing.alpha = alpha;
            existing.coors = coors;
            existing.size = size;
            existing.draw_dist = draw_dist;
            existing.texture_type = texture_type.min(8);
            existing.flare_type = flare_type;
            existing.reflection = reflection;
            existing.los_check = los_check != 0;
            existing.draw_streak = draw_streak != 0;
            existing.some_angle = some_angle;
            existing.registered_this_frame = true;
            return;
        }

        if let Some(slot) = self.coronas.iter_mut().find(|c| c.id == 0) {
            *slot = CRegisteredCorona {
                id,
                red,
                green,
                blue,
                alpha,
                fade_alpha: 0,
                coors,
                size,
                some_angle,
                registered_this_frame: true,
                draw_dist,
                flare_type,
                reflection,
                texture_type: texture_type.min(8),
                los_check: los_check != 0,
                off_screen: true,
                first_update: true,
                draw_streak: draw_streak != 0,
                sight_clear: false,
            };
        }
    }

    /// re3 `CCoronas::UpdateCoronaCoors`.
    pub fn update_corona_coors(
        &mut self,
        id: u32,
        coors: CVector,
        draw_dist: f32,
        some_angle: f32,
    ) {
        if let Some(slot) = self.coronas.iter_mut().find(|c| c.id == id && c.id != 0) {
            slot.coors = coors;
            slot.draw_dist = draw_dist;
            slot.some_angle = some_angle;
            slot.registered_this_frame = true;
        }
    }

    /// re3 `CRegisteredCorona::Update`: the preceding render pass determines
    /// offScreen; registration from the preceding world pass determines alpha.
    pub fn update_with_time_step(&mut self, time_step: f32) {
        let fade_step = (15.0 * time_step.max(0.0)).max(0.0);
        for corona in &mut self.coronas {
            if corona.id == 0 {
                continue;
            }
            if !corona.registered_this_frame {
                corona.alpha = 0;
            }
            let fade = f32::from(corona.fade_alpha);
            let alpha = f32::from(corona.alpha);
            corona.fade_alpha = if corona.off_screen {
                (fade - fade_step).max(0.0) as u8
            } else if alpha > fade {
                (fade + fade_step).min(alpha) as u8
            } else if alpha < fade {
                (fade - fade_step).max(alpha) as u8
            } else {
                corona.fade_alpha
            };
            if corona.fade_alpha == 0 && !corona.first_update {
                *corona = CRegisteredCorona::default();
                continue;
            }
            corona.first_update = false;
            corona.registered_this_frame = false;
        }
    }

    pub fn update(&mut self) {
        self.update_with_time_step(1.0);
    }

    /// Active coronas for this frame's Bevy flush.
    pub fn active_coronas(&self) -> impl Iterator<Item = &CRegisteredCorona> {
        self.coronas
            .iter()
            .filter(|c| c.id != 0 && c.fade_alpha > 0 && c.size > 0.0)
    }

    pub fn texture_name_for_type(&self, texture_type: u8) -> &'static str {
        self.texture_names
            .get(texture_type as usize)
            .copied()
            .unwrap_or("corona")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unseen_corona_releases_pool_slot_and_visible_corona_fades_in() {
        let mut coronas = CCoronas::init();
        let register = |coronas: &mut CCoronas| {
            coronas.register_corona(
                42, 255, 160, 50, 255, CVector::ZERO, 2.0, 100.0,
                TYPE_STAR, FLARE_NONE, REFLECTION_OFF, LOSCHECK_OFF, STREAK_OFF, 0.0,
            );
        };
        register(&mut coronas);
        assert_eq!(coronas.coronas[0].fade_alpha, 0);
        assert!(coronas.coronas[0].off_screen);
        coronas.update_with_time_step(1.0);
        assert_eq!(coronas.coronas[0].id, 42);
        register(&mut coronas);
        coronas.update_with_time_step(1.0);
        assert_eq!(coronas.coronas[0].id, 0);

        register(&mut coronas);
        coronas.coronas[0].off_screen = false;
        coronas.update_with_time_step(1.0);
        assert_eq!(coronas.coronas[0].fade_alpha, 15);
        assert_eq!(coronas.coronas[0].id, 42);
        register(&mut coronas);
        coronas.coronas[0].off_screen = true;
        coronas.update_with_time_step(1.0);
        assert_eq!(coronas.coronas[0].fade_alpha, 0);
        assert_eq!(coronas.coronas[0].id, 0);
    }
}
