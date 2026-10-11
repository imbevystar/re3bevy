//! re3 reference: references/re-GTA/src/rw/RwMatFX.cpp
//!
//! PS2 MatFX material-pass descriptors.

pub const RPMATFX_EFFECT_NULL: i32 = 0;
pub const RPMATFX_EFFECT_BUMPMAP: i32 = 1;
pub const RPMATFX_EFFECT_ENVMAP: i32 = 2;
pub const RPMATFX_EFFECT_BUMPENVMAP: i32 = 3;
pub const RPMATFX_EFFECT_DUAL: i32 = 4;
pub const RPMATFX_EFFECT_UVTRANSFORM: i32 = 5;
pub const RPMATFX_EFFECT_DUALUVTRANSFORM: i32 = 6;

#[derive(Debug, Clone, PartialEq)]
pub struct MatFX {
    pub fx: [MatFXSlot; 2],
    pub effects: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatFXSlot {
    Nothing { effect: i32 },
    Bump(MatFXBump),
    Env(MatFXEnv),
    Dual(MatFXDual),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatFXBump {
    pub bump_frame: Option<usize>,
    pub bumped_tex: Option<usize>,
    pub bump_tex: Option<usize>,
    pub neg_bump_coefficient: f32,
    pub effect: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatFXEnv {
    pub env_frame: Option<usize>,
    pub env_tex: Option<usize>,
    pub env_coeff: f32,
    pub env_fb_alpha: i32,
    pub effect: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatFXDual {
    pub dual_tex: Option<usize>,
    pub src_blend: i32,
    pub dst_blend: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatFXRenderPass {
    Default,
    Env {
        intensity: u8,
        framebuffer_alpha: bool,
    },
    Bump,
    Dual {
        src_blend: i32,
        dst_blend: i32,
    },
    Black,
}

impl MatFX {
    pub fn render_passes(&self) -> [Option<MatFXRenderPass>; 2] {
        [
            matfx_slot_pass(&self.fx[0], true),
            matfx_slot_pass(&self.fx[1], false),
        ]
    }
}

impl Default for MatFX {
    fn default() -> Self {
        Self {
            fx: [
                MatFXSlot::Nothing {
                    effect: RPMATFX_EFFECT_NULL,
                },
                MatFXSlot::Nothing {
                    effect: RPMATFX_EFFECT_NULL,
                },
            ],
            effects: RPMATFX_EFFECT_NULL,
        }
    }
}

fn matfx_slot_pass(slot: &MatFXSlot, first: bool) -> Option<MatFXRenderPass> {
    match slot {
        MatFXSlot::Nothing { .. } if first => Some(MatFXRenderPass::Default),
        MatFXSlot::Nothing { .. } => None,
        MatFXSlot::Bump(_) => Some(MatFXRenderPass::Bump),
        MatFXSlot::Env(env) => {
            let intensity = (env.env_coeff * 255.0) as u8;
            if intensity == 0 || env.env_tex.is_none() {
                first.then_some(MatFXRenderPass::Default)
            } else {
                Some(MatFXRenderPass::Env {
                    intensity,
                    framebuffer_alpha: env.env_fb_alpha != 0,
                })
            }
        }
        MatFXSlot::Dual(dual) => Some(MatFXRenderPass::Dual {
            src_blend: dual.src_blend,
            dst_blend: dual.dst_blend,
        }),
    }
}
