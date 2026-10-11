//! re3 reference: references/re-GTA/src/control/Script6.cpp
//!
//! PC re3 stores `CRunningScript::ProcessCommands1000To1099` and
//! `CRunningScript::ProcessCommands1100To1199` here. Those commands are mostly
//! gameplay/frontend/audio/world side effects and remain explicit unsupported
//! opcode results until their owning systems are ported.

pub const FIRST_COMMAND_RANGE_START: i32 = 1000;
pub const FIRST_COMMAND_RANGE_END_EXCLUSIVE: i32 = 1100;
pub const SECOND_COMMAND_RANGE_START: i32 = 1100;
pub const SECOND_COMMAND_RANGE_END_EXCLUSIVE: i32 = 1200;
