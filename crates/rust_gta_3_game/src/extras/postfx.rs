//! re3 reference: references/re-GTA/src/extras/postfx.cpp
//! re3 reference: references/re-GTA/src/extras/postfx.h
//!
//! Extended colour-filter/post-process state and render intents.

use bevy::prelude::Resource;

use crate::{
    core::Camera::{
        CCamera, MOTION_BLUR_INTRO, MOTION_BLUR_INTRO2, MOTION_BLUR_INTRO3, MOTION_BLUR_INTRO4,
        MOTION_BLUR_NONE, MOTION_BLUR_SECURITY_CAM,
    },
    objects::ParticleObject::CRGBA,
    renderer::MBlur::MBlurVertex,
};

pub const POSTFX_OFF: i32 = 0;
pub const POSTFX_SIMPLE: i32 = 1;
pub const POSTFX_NORMAL: i32 = 2;
pub const POSTFX_MOBILE: i32 = 3;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CPostFX {
    pub front_buffer: Option<PostFxRaster>,
    pub back_buffer: Option<PostFxRaster>,
    pub just_initialised: bool,
    pub effect_switch: i32,
    pub motion_blur_on: bool,
    pub intensity: f32,
    pub vertex: [MBlurVertex; 4],
    pub vertex2: [MBlurVertex; 4],
    pub indices: [u16; 6],
    pub intents: Vec<PostFxIntent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostFxRaster {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostFxIntentKind {
    OverlayBlur,
    OverlaySimple,
    OverlaySniper,
    OverlayShader,
    MotionBlur,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostFxIntent {
    pub kind: PostFxIntentKind,
    pub colour: CRGBA,
    pub blur_type: i32,
    pub blur_alpha: u32,
}

impl CPostFX {
    fn override_colour_for_blur_type(
        blur_type: i32,
        mut red: u32,
        mut green: u32,
        mut blue: u32,
        mut blur: u32,
    ) -> [u32; 4] {
        match blur_type {
            MOTION_BLUR_SECURITY_CAM => {
                red = 0;
                green = 255;
                blue = 0;
                blur = 128;
            }
            MOTION_BLUR_INTRO => {
                red = 100;
                green = 220;
                blue = 230;
                blur = 158;
            }
            MOTION_BLUR_INTRO2 => {
                red = 80;
                green = 255;
                blue = 230;
                blur = 138;
            }
            MOTION_BLUR_INTRO3 => {
                red = 255;
                green = 60;
                blue = 60;
                blur = 200;
            }
            MOTION_BLUR_INTRO4 => {
                red = 255;
                green = 180;
                blue = 180;
                blur = 128;
            }
            _ => {}
        }
        [red, green, blue, blur]
    }

    /// Current-frame colour-filter input used by the default re3 POSTFX_NORMAL
    /// path. The values come from the camera, not directly from timecycle, so
    /// SCM/cutscene blur modes and the special intro/security overrides win in
    /// exactly the same place they do in CPostFX::Render.
    pub fn effective_colour_filter(&self, camera: &CCamera) -> Option<[f32; 4]> {
        if self.effect_switch != POSTFX_NORMAL
            || self.motion_blur_on
            || camera.blur_type == MOTION_BLUR_NONE
        {
            return None;
        }
        let rgba = Self::override_colour_for_blur_type(
            camera.blur_type,
            camera.blur_red.max(0) as u32,
            camera.blur_green.max(0) as u32,
            camera.blur_blue.max(0) as u32,
            camera.motion_blur.max(0) as u32,
        );
        Some([
            rgba[0].min(255) as f32 / 255.0,
            rgba[1].min(255) as f32 / 255.0,
            rgba[2].min(255) as f32 / 255.0,
            (rgba[3].min(255) as f32 * self.intensity / 255.0).clamp(0.0, 1.0),
        ])
    }

    /// colourfilterIII.frag is point-wise multiplicative after its five
    /// iterations. This coefficient lets pre-projected sprite paths receive
    /// the same filter without sampling the already-rendered camera texture.
    pub fn effective_colour_multiplier(&self, camera: &CCamera) -> [f32; 3] {
        let Some(colour) = self.effective_colour_filter(camera) else {
            return [1.0; 3];
        };
        let a = colour[3];
        let mut multiplier = [1.0; 3];
        for _ in 0..5 {
            for channel in 0..3 {
                multiplier[channel] =
                    ((1.0 - a) + multiplier[channel] * colour[channel] * a).clamp(0.0, 1.0);
            }
        }
        multiplier
    }

    pub fn init_once(&mut self) {}

    pub fn open(&mut self, width: u32, height: u32, depth: u32, near_clip: f32) {
        self.close();
        let pow2_width = width.next_power_of_two();
        let pow2_height = height.next_power_of_two();
        self.front_buffer = Some(PostFxRaster {
            width: pow2_width,
            height: pow2_height,
            depth,
        });
        self.back_buffer = Some(PostFxRaster {
            width: pow2_width,
            height: pow2_height,
            depth,
        });
        self.just_initialised = true;
        self.create_vertices(pow2_width as f32, pow2_height as f32, depth, near_clip);
    }

    pub fn close(&mut self) {
        self.front_buffer = None;
        self.back_buffer = None;
        self.just_initialised = false;
    }

    pub fn render(
        &mut self,
        red: u32,
        green: u32,
        blue: u32,
        blur: u32,
        blur_type: i32,
        blur_alpha: u32,
    ) {
        if self.effect_switch == POSTFX_OFF {
            return;
        }
        let [red, green, blue, blur] =
            Self::override_colour_for_blur_type(blur_type, red, green, blue, blur);
        let kind = if self.effect_switch == POSTFX_SIMPLE {
            PostFxIntentKind::OverlaySimple
        } else if self.motion_blur_on || blur != 0 {
            PostFxIntentKind::MotionBlur
        } else {
            PostFxIntentKind::OverlayShader
        };
        self.intents.push(PostFxIntent {
            kind,
            colour: CRGBA::new(red as u8, green as u8, blue as u8, blur as u8),
            blur_type,
            blur_alpha,
        });
        self.just_initialised = false;
    }

    pub fn need_back_buffer(&self) -> bool {
        self.effect_switch == POSTFX_NORMAL || self.effect_switch == POSTFX_MOBILE
    }

    pub fn need_front_buffer(&self, blur_type: i32) -> bool {
        self.effect_switch != POSTFX_OFF && (self.motion_blur_on || blur_type != 0)
    }

    pub fn use_blur_colours(&self) -> bool {
        self.effect_switch != POSTFX_SIMPLE
    }

    pub fn drain_intents(&mut self) -> Vec<PostFxIntent> {
        std::mem::take(&mut self.intents)
    }

    fn create_vertices(&mut self, width: f32, height: f32, depth: u32, near_clip: f32) {
        let half_px = 0.5;
        let (zero, xmax, ymax) = if depth == 16 {
            (half_px, width + half_px, height + half_px)
        } else {
            (-half_px, width - half_px, height - half_px)
        };
        let recip = 1.0 / near_clip;
        self.vertex = [
            postfx_vertex(zero, zero, near_clip, recip, [0.0, 0.0]),
            postfx_vertex(zero, ymax, near_clip, recip, [0.0, 1.0]),
            postfx_vertex(xmax, ymax, near_clip, recip, [1.0, 1.0]),
            postfx_vertex(xmax, zero, near_clip, recip, [1.0, 0.0]),
        ];
        self.vertex2 = [
            postfx_vertex(zero + 2.0, zero + 2.0, near_clip, recip, [0.0, 0.0]),
            postfx_vertex(2.0, ymax + 2.0, near_clip, recip, [0.0, 1.0]),
            postfx_vertex(xmax + 2.0, ymax + 2.0, near_clip, recip, [1.0, 1.0]),
            postfx_vertex(xmax + 2.0, zero + 2.0, near_clip, recip, [1.0, 0.0]),
        ];
    }
}

impl Default for CPostFX {
    fn default() -> Self {
        Self {
            front_buffer: None,
            back_buffer: None,
            just_initialised: false,
            effect_switch: POSTFX_NORMAL,
            motion_blur_on: false,
            intensity: 1.0,
            vertex: [MBlurVertex::default(); 4],
            vertex2: [MBlurVertex::default(); 4],
            indices: [0, 1, 2, 0, 2, 3],
            intents: Vec::new(),
        }
    }
}

fn postfx_vertex(x: f32, y: f32, near_clip: f32, recip: f32, uv: [f32; 2]) -> MBlurVertex {
    MBlurVertex {
        screen: [x, y, 0.0],
        camera_z: near_clip,
        recip_camera_z: recip,
        uv,
        colour: CRGBA::new(255, 255, 255, 255),
    }
}
