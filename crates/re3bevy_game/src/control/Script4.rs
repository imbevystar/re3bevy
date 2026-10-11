//! re3 reference: references/re-GTA/src/control/Script4.cpp
//!
//! `CRunningScript::ProcessCommands400To499` and
//! `CRunningScript::ProcessCommands500To599` live here in re3.
//! The Rust VM keeps opcode ids in `ScriptCommands.rs` and routes currently
//! unsupported mission-world opcodes through `ScriptVmError::UnsupportedCommand`.
//! Gameplay side effects from this shard must be ported from the C++ reference.

pub const FIRST_COMMAND_RANGE_START: i32 = 400;
pub const FIRST_COMMAND_RANGE_END_EXCLUSIVE: i32 = 500;
pub const SECOND_COMMAND_RANGE_START: i32 = 500;
pub const SECOND_COMMAND_RANGE_END_EXCLUSIVE: i32 = 600;
