//! re3 reference: references/re-GTA/src/extras/screendroplets.cpp
//! re3 reference: references/re-GTA/src/extras/screendroplets.h
//!
//! Screen droplet state and render-list construction.

use bevy::prelude::Resource;
use rustgta3_core::core::Coordinates::CVector;

use crate::objects::ParticleObject::CRGBA;

pub const MAXDROPS: usize = 2000;
pub const MAXDROPSMOVING: usize = 700;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct ScreenDroplets {
    pub initialised: bool,
    pub mask_texture_ready: bool,
    pub screen_texture_ready: bool,
    pub enabled: bool,
    pub moving_enabled: bool,
    pub drops: Vec<ScreenDrop>,
    pub drops_moving: Vec<ScreenDropMoving>,
    pub prev_cam_up: CVector,
    pub prev_cam_pos: CVector,
    pub cam_move_delta: CVector,
    pub cam_move_dist: f32,
    pub screen_move_delta: CVector,
    pub screen_move_dist: f32,
    pub cam_up_angle: f32,
    pub splash_duration: i32,
    pub splash_object: Option<usize>,
    pub render_list: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenDrop {
    pub x: f32,
    pub y: f32,
    pub time: f32,
    pub size: f32,
    pub magnification: f32,
    pub lifetime: f32,
    pub color: CRGBA,
    pub active: bool,
    pub fades: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenDropMoving {
    pub drop: usize,
    pub dist: f32,
}

impl ScreenDroplets {
    pub fn initialise(&mut self) {
        self.clear();
        self.splash_duration = -1;
        self.splash_object = None;
    }

    pub fn init_draw(&mut self) {
        self.mask_texture_ready = true;
        self.screen_texture_ready = true;
        self.initialised = true;
    }

    pub fn shutdown(&mut self) {
        self.mask_texture_ready = false;
        self.screen_texture_ready = false;
        self.initialised = false;
    }

    pub fn process(&mut self) {
        self.process_moving();
        self.fade();
    }

    /// re3 `ScreenDroplets::ProcessCameraMovement` (screendroplets.cpp): derive
    /// the per-frame camera translation in camera space from the live camera
    /// basis. `cam_right/cam_up/cam_at` are the RenderWare matrix axes, i.e.
    /// Bevy `Transform::{right,up,forward}` converted with `CVector::from_bevy`.
    pub fn process_camera_movement(
        &mut self,
        cam_pos: CVector,
        cam_right: CVector,
        cam_up: CVector,
        cam_at: CVector,
    ) {
        let delta = CVector::new(
            cam_pos.x - self.prev_cam_pos.x,
            cam_pos.y - self.prev_cam_pos.y,
            cam_pos.z - self.prev_cam_pos.z,
        );
        self.cam_move_delta = delta;
        self.cam_move_dist = delta.magnitude();

        self.prev_cam_up = cam_up;
        self.prev_cam_pos = cam_pos;

        let mut screen_delta =
            CVector::new(-cam_right.dot(delta), cam_up.dot(delta), cam_at.dot(delta));
        screen_delta.x *= 10.0;
        screen_delta.y *= 10.0;
        screen_delta.z *= 10.0;
        self.screen_move_delta = screen_delta;
        self.screen_move_dist =
            (screen_delta.x * screen_delta.x + screen_delta.y * screen_delta.y).sqrt();

        self.cam_up_angle = cam_up.z.clamp(-1.0, 1.0).acos().to_degrees();
    }

    pub fn render(&mut self) {
        self.render_list.clear();
        for index in 0..self.drops.len() {
            if self.drops[index].active {
                self.add_to_render_list(index);
            }
        }
    }

    pub fn clear(&mut self) {
        self.drops.clear();
        self.drops.resize(MAXDROPS, ScreenDrop::default());
        self.drops_moving.clear();
        self.render_list.clear();
    }

    pub fn new_drop(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        lifetime: f32,
        fades: bool,
        colour: CRGBA,
    ) -> Option<usize> {
        let index = self.drops.iter().position(|drop| !drop.active)?;
        self.drops[index] = ScreenDrop {
            x,
            y,
            time: 0.0,
            size,
            magnification: 1.0,
            lifetime,
            color: colour,
            active: true,
            fades,
        };
        Some(index)
    }

    pub fn set_moving(&mut self, drop: usize, dist: f32) -> bool {
        if self.drops_moving.len() >= MAXDROPSMOVING || drop >= self.drops.len() {
            return false;
        }
        self.drops_moving.push(ScreenDropMoving { drop, dist });
        true
    }

    pub fn register_splash(&mut self, particle_object: usize, duration: i32) {
        self.splash_object = Some(particle_object);
        self.splash_duration = duration;
    }

    fn add_to_render_list(&mut self, drop: usize) {
        self.render_list.push(drop);
    }

    fn process_moving(&mut self) {
        if !self.moving_enabled {
            return;
        }
        for moving in &self.drops_moving {
            if let Some(drop) = self.drops.get_mut(moving.drop) {
                drop.x += self.screen_move_delta.x * moving.dist;
                drop.y += self.screen_move_delta.y * moving.dist;
            }
        }
    }

    fn fade(&mut self) {
        for drop in &mut self.drops {
            drop.fade();
        }
    }
}

impl ScreenDrop {
    pub fn fade(&mut self) {
        if !self.active || !self.fades {
            return;
        }
        self.time += 1.0;
        if self.time >= self.lifetime {
            self.active = false;
        } else {
            let alpha = 1.0 - self.time / self.lifetime.max(1.0);
            self.color.alpha = (255.0 * alpha).clamp(0.0, 255.0) as u8;
        }
    }
}

impl Default for ScreenDroplets {
    fn default() -> Self {
        let mut droplets = Self {
            initialised: false,
            mask_texture_ready: false,
            screen_texture_ready: false,
            enabled: true,
            moving_enabled: true,
            drops: Vec::new(),
            drops_moving: Vec::new(),
            prev_cam_up: CVector::ZERO,
            prev_cam_pos: CVector::ZERO,
            cam_move_delta: CVector::ZERO,
            cam_move_dist: 0.0,
            screen_move_delta: CVector::ZERO,
            screen_move_dist: 0.0,
            cam_up_angle: 0.0,
            splash_duration: -1,
            splash_object: None,
            render_list: Vec::new(),
        };
        droplets.clear();
        droplets
    }
}

impl Default for ScreenDrop {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            time: 0.0,
            size: 0.0,
            magnification: 1.0,
            lifetime: 0.0,
            color: CRGBA::new(255, 255, 255, 255),
            active: false,
            fades: false,
        }
    }
}
