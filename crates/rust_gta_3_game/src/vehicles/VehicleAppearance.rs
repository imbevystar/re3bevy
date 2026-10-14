//! Common visual state, distinct from physical/control/AI ownership.
//! re3 references: CVehicle constructor/SetModelIndex and CAutomobile PreRender/Render.

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct VehicleAppearance {
    pub current_colour1: u8,
    pub current_colour2: u8,
    /// Original DFF optional component indices; -1 means no selected component.
    pub extras: [i8; 2],
    pub lights_on: bool,
    pub fade_out: bool,
    pub is_damaged: bool,
}

impl Default for VehicleAppearance {
    fn default() -> Self {
        Self {
            current_colour1: 0,
            current_colour2: 0,
            extras: [-1; 2],
            lights_on: false,
            fade_out: false,
            is_damaged: false,
        }
    }
}
