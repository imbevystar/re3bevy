//! re3 reference: references/re-GTA/src/control/Script3.cpp
//!
//! `CRunningScript::ProcessCommands300To399` lives here in re3.
//! The Rust VM keeps opcode ids in `ScriptCommands.rs` and routes currently
//! unsupported vehicle/pickup/streaming opcodes through `ScriptVmError::UnsupportedCommand`.
//! Gameplay side effects from this shard must be ported from the C++ reference.

pub const COMMAND_RANGE_START: i32 = 300;
pub const COMMAND_RANGE_END_EXCLUSIVE: i32 = 400;
