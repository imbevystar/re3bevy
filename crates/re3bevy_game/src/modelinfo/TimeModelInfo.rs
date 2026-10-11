//! re3 reference: references/re-GTA/src/modelinfo/TimeModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/TimeModelInfo.h
//!
//! Time-window helpers for `CSimpleModelInfo` entries of type `MITYPE_TIME`.

pub use super::ModelInfo::CSimpleModelInfo as CTimeModelInfo;

impl CTimeModelInfo {
    pub fn GetTimeOn(&self) -> Option<i32> {
        self.time_on
    }

    pub fn GetTimeOff(&self) -> Option<i32> {
        self.time_off
    }

    pub fn SetTimes(&mut self, on: i32, off: i32) {
        self.time_on = Some(on);
        self.time_off = Some(off);
    }

    pub fn GetOtherTimeModel(&self) -> Option<usize> {
        self.other_time_model_id
    }

    pub fn SetOtherTimeModel(&mut self, other: usize) {
        self.other_time_model_id = Some(other);
    }

    pub fn IsVisibleAtHour(&self, hour: i32) -> bool {
        match (self.time_on, self.time_off) {
            (Some(on), Some(off)) if on < off => hour >= on && hour < off,
            (Some(on), Some(off)) => hour >= on || hour < off,
            _ => true,
        }
    }
}
