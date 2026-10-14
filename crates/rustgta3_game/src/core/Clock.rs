//! re3 reference: references/re-GTA/src/core/Clock.cpp
//! re3 reference: references/re-GTA/src/core/Clock.h
//!
//! Runtime clock behavior lives in `rustgta3_core::core::Clock`.

use bevy::prelude::*;
use rustgta3_core::core::Clock::{CClock, CClockInput};

use crate::core::{Pad::CPads, Stats::CStats};

/// Feed the raw second-pad R1 state used directly by `CClock::Update` in re3.
pub(crate) fn sync_clock_input_from_pads(pads: Res<CPads>, mut clock_input: ResMut<CClockInput>) {
    clock_input.debug_fast_forward = pads.get_pad(1).new_state.right_shoulder1 != 0;
}

/// re3 increments `CStats::DaysPassed` only on a normal clock midnight rollover.
/// The core clock records that edge so the game layer can update its stats resource
/// without introducing a dependency from `rustgta3_core` back into this crate.
pub(crate) fn apply_clock_day_rollovers(mut clock: ResMut<CClock>, stats: Option<ResMut<CStats>>) {
    let Some(mut stats) = stats else {
        return;
    };
    let rollovers = clock.take_day_rollovers();
    stats.days_passed = stats.days_passed.wrapping_add(rollovers as i32);
}
