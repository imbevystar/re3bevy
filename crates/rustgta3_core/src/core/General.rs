//! re3 reference: references/re-GTA/src/core/General.h
//! re3 reference: references/re-GTA/src/core/re3.cpp
//!
//! Shared `CGeneral` utility state. The random generator mirrors the default
//! PC `myrand` path used by re3.

use bevy::prelude::*;

#[derive(Debug, Clone, Resource)]
pub struct CGeneral {
    myrand_seed: u32,
}

impl Default for CGeneral {
    fn default() -> Self {
        Self::new()
    }
}

impl CGeneral {
    pub const MYRAND_MAX: u16 = 32767;

    pub const fn new() -> Self {
        Self { myrand_seed: 1 }
    }

    pub const fn with_seed(seed: u32) -> Self {
        Self { myrand_seed: seed }
    }

    pub fn mysrand(&mut self, seed: u32) {
        self.myrand_seed = seed;
    }

    pub fn myrand(&mut self) -> i32 {
        self.myrand_seed = self
            .myrand_seed
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12_345);
        ((self.myrand_seed >> 16) & 0x7FFF) as i32
    }

    pub fn get_random_number(&mut self) -> u16 {
        (self.myrand() as u16) & Self::MYRAND_MAX
    }

    pub fn get_random_true_false(&mut self) -> bool {
        self.get_random_number() < Self::MYRAND_MAX / 2
    }

    pub fn get_random_number_in_range_f32(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * (self.get_random_number() as f32 / (Self::MYRAND_MAX as f32 + 1.0))
    }

    pub fn get_random_number_in_range_i32(&mut self, low: i32, high: i32) -> i32 {
        low + ((high - low) as f32
            * (self.get_random_number() as f32 / (Self::MYRAND_MAX as f32 + 1.0)))
            as i32
    }

    /// re3 `CGeneral::LimitAngle`.
    pub fn limit_angle(mut angle: f32) -> f32 {
        while angle >= 180.0 {
            angle -= 360.0;
        }
        while angle < -180.0 {
            angle += 360.0;
        }
        angle
    }

    /// re3 `CGeneral::LimitRadianAngle`.
    pub fn limit_radian_angle(angle: f32) -> f32 {
        let mut angle = angle.clamp(-25.0, 25.0);
        while angle >= std::f32::consts::PI {
            angle -= std::f32::consts::TAU;
        }
        while angle < -std::f32::consts::PI {
            angle += std::f32::consts::TAU;
        }
        angle
    }

    pub fn get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let x = x2 - x1;
        let mut y = y2 - y1;
        if y == 0.0 {
            y = 0.0001;
        }
        if x > 0.0 {
            if y > 0.0 {
                std::f32::consts::PI - (x / y).atan2(1.0)
            } else {
                -(x / y).atan2(1.0)
            }
        } else if y > 0.0 {
            -(std::f32::consts::PI + (x / y).atan2(1.0))
        } else {
            -(x / y).atan2(1.0)
        }
    }

    /// re3 `CGeneral::GetATanOfXY` (`core/General.h:8-38`).  The branch order
    /// is preserved instead of replacing it with a differently oriented
    /// `atan2(y, x)` call.
    pub fn get_atan_of_xy(x: f32, y: f32) -> f32 {
        if x == 0.0 && y == 0.0 {
            return 0.0;
        }
        let x_abs = x.abs();
        let y_abs = y.abs();
        if x_abs < y_abs {
            if y > 0.0 {
                if x > 0.0 {
                    0.5 * std::f32::consts::PI - (x / y).atan2(1.0)
                } else {
                    0.5 * std::f32::consts::PI + (-x / y).atan2(1.0)
                }
            } else if x > 0.0 {
                1.5 * std::f32::consts::PI + (x / -y).atan2(1.0)
            } else {
                1.5 * std::f32::consts::PI - (-x / -y).atan2(1.0)
            }
        } else if y > 0.0 {
            if x > 0.0 {
                (y / x).atan2(1.0)
            } else {
                std::f32::consts::PI - (y / -x).atan2(1.0)
            }
        } else if x > 0.0 {
            2.0 * std::f32::consts::PI - (-y / x).atan2(1.0)
        } else {
            std::f32::consts::PI + (-y / -x).atan2(1.0)
        }
    }

    pub fn seed(&self) -> u32 {
        self.myrand_seed
    }
}

pub struct GtaGeneralPlugin;

impl Plugin for GtaGeneralPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CGeneral>();
    }
}
