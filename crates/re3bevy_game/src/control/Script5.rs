//! re3 reference: references/re-GTA/src/control/Script5.cpp
//!
//! re3 stores `CRunningScript::UpdateCompareFlag` and command shards
//! 600..999 here. `UpdateCompareFlag` has been ported into `Script.rs`
//! because Rust keeps the VM state in one `CRunningScript` implementation.
//! Gameplay side effects from the remaining shards must be ported from C++.

pub const FIRST_COMMAND_RANGE_START: i32 = 600;
pub const FIRST_COMMAND_RANGE_END_EXCLUSIVE: i32 = 700;
pub const SECOND_COMMAND_RANGE_START: i32 = 700;
pub const SECOND_COMMAND_RANGE_END_EXCLUSIVE: i32 = 800;
pub const THIRD_COMMAND_RANGE_START: i32 = 800;
pub const THIRD_COMMAND_RANGE_END_EXCLUSIVE: i32 = 900;
pub const FOURTH_COMMAND_RANGE_START: i32 = 900;
pub const FOURTH_COMMAND_RANGE_END_EXCLUSIVE: i32 = 1000;
