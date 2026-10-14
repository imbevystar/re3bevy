//! re3 reference: references/re-GTA/src/core/TimeStep.cpp
//! re3 reference: references/re-GTA/src/core/TimeStep.h
//!
//! Fixed-step integration helpers matching re3's frame-step vocabulary.

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CTimeStep {
    pub step: f32,
    pub step_non_clipped: f32,
}

impl Default for CTimeStep {
    fn default() -> Self {
        Self {
            step: 1.0,
            step_non_clipped: 1.0,
        }
    }
}

impl CTimeStep {
    #[allow(non_snake_case)]
    pub fn from_milliseconds(delta_ms: u32, time_scale: f32) -> Self {
        let step_non_clipped = delta_ms as f32 / 1000.0 * 50.0 * time_scale;
        Self {
            step: step_non_clipped.clamp(0.0, 3.0),
            step_non_clipped,
        }
    }
}
