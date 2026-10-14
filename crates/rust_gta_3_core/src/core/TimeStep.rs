//! re3 reference: references/re-GTA/src/core/TimeStep.cpp
//! re3 reference: references/re-GTA/src/core/TimeStep.h
//!
//! re3 notes this class is probably unused, but its static defaults are kept.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CTimeStep {
    pub time_scale: f32,
    pub frames_per_update: f32,
    pub time_step: f32,
}

impl Default for CTimeStep {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            frames_per_update: 1.0,
            time_step: 1.0,
        }
    }
}
