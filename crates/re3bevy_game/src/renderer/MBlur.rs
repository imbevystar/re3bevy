//! re3 reference: references/re-GTA/src/renderer/MBlur.cpp
//! re3 reference: references/re-GTA/src/renderer/MBlur.h
//!
//! Motion blur front-buffer state and overlay render intents.

use bevy::prelude::Resource;

use crate::objects::ParticleObject::CRGBA;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CMBlur {
    pub front_buffer: Option<MotionBlurRaster>,
    pub just_initialised: bool,
    pub scaled_blur: bool,
    pub blur_on: bool,
    pub vertices: [MBlurVertex; 4],
    pub indices: [u16; 6],
    pub render_intents: Vec<MotionBlurRenderIntent>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionBlurRaster {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MBlurVertex {
    pub screen: [f32; 3],
    pub camera_z: f32,
    pub recip_camera_z: f32,
    pub uv: [f32; 2],
    pub colour: CRGBA,
}

impl Default for MBlurVertex {
    fn default() -> Self {
        Self {
            screen: [0.0; 3],
            camera_z: 0.0,
            recip_camera_z: 0.0,
            uv: [0.0; 2],
            colour: CRGBA::new(255, 255, 255, 255),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionBlurRenderIntent {
    pub colour: CRGBA,
    pub blur_type: i32,
    pub blur_alpha: u32,
    pub uses_front_buffer: bool,
}

impl CMBlur {
    pub fn motion_blur_open(
        &mut self,
        width: u32,
        height: u32,
        depth: u32,
        near_clip: f32,
    ) -> bool {
        if self.front_buffer.is_some() {
            self.motion_blur_close();
        }

        if self.blur_on {
            self.front_buffer = Some(MotionBlurRaster {
                width,
                height,
                depth,
            });
            self.just_initialised = true;
        }

        self.create_immediate_mode_data(width as f32, height as f32, depth, near_clip);
        true
    }

    pub fn motion_blur_close(&mut self) -> bool {
        let had_front_buffer = self.front_buffer.take().is_some();
        if had_front_buffer {
            self.just_initialised = false;
        }
        had_front_buffer
    }

    pub fn create_immediate_mode_data(
        &mut self,
        width: f32,
        height: f32,
        depth: u32,
        near_clip: f32,
    ) {
        let half_px = 0.5;
        let (zero, xmax, ymax) = if depth == 16 {
            (half_px, width + half_px, height + half_px)
        } else {
            (-half_px, width - half_px, height - half_px)
        };
        let recip = 1.0 / near_clip;
        self.vertices = [
            mblur_vertex(zero, zero, near_clip, recip, [0.0, 0.0]),
            mblur_vertex(zero, ymax, near_clip, recip, [0.0, 1.0]),
            mblur_vertex(xmax, ymax, near_clip, recip, [1.0, 1.0]),
            mblur_vertex(xmax, zero, near_clip, recip, [1.0, 0.0]),
        ];
    }

    pub fn motion_blur_render(
        &mut self,
        red: u32,
        green: u32,
        blue: u32,
        blur: u32,
        blur_type: i32,
        blur_alpha: u32,
    ) {
        self.render_intents.push(MotionBlurRenderIntent {
            colour: CRGBA::new(red as u8, green as u8, blue as u8, blur as u8),
            blur_type,
            blur_alpha,
            uses_front_buffer: self.blur_on && self.front_buffer.is_some(),
        });
        self.just_initialised = false;
    }

    pub fn drain_render_intents(&mut self) -> Vec<MotionBlurRenderIntent> {
        std::mem::take(&mut self.render_intents)
    }
}

impl Default for CMBlur {
    fn default() -> Self {
        Self {
            front_buffer: None,
            just_initialised: false,
            scaled_blur: false,
            blur_on: true,
            vertices: [MBlurVertex::default(); 4],
            indices: [0, 1, 2, 0, 2, 3],
            render_intents: Vec::new(),
        }
    }
}

fn mblur_vertex(x: f32, y: f32, near_clip: f32, recip: f32, uv: [f32; 2]) -> MBlurVertex {
    MBlurVertex {
        screen: [x, y, 0.0],
        camera_z: near_clip,
        recip_camera_z: recip,
        uv,
        colour: CRGBA::new(255, 255, 255, 255),
    }
}
