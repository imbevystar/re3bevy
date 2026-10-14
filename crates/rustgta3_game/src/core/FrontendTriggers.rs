//! re3 reference: references/re-GTA/src/core/FrontendTriggers.h
//!
//! Frontend trigger ids.

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum eFrontendTrigger {
    FE_TRIGGER_NONE,
    FE_TRIGGER_NEW_GAME,
    FE_TRIGGER_LOAD_GAME,
    FE_TRIGGER_OPTIONS,
    FE_TRIGGER_QUIT,
}
