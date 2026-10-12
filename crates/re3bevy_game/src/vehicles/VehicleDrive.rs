//! Control and transmission component shared by native vehicle kinds.
//! re3 references: Vehicle.h / CVehicle::CVehicle / ProcessControlInputs.

/// Input/transmission state shared by every native vehicle kind.
#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct VehicleDriveState {
    pub steer_angle: f32,
    pub gas_pedal: f32,
    pub brake_pedal: f32,
    pub current_gear: u8,
    pub change_gear_time: f32,
    pub steer_input: f32,
    /// The previous steering device, for re3's mouse steering hold/keyboard fallback.
    pub mouse_steering_active: bool,
    pub comedy_control_state: i8,
    pub engine_on: bool,
    pub is_handbrake_on: bool,
    pub comedy_controls: bool,
}

impl Default for VehicleDriveState {
    fn default() -> Self {
        Self {
            steer_angle: 0.0,
            gas_pedal: 0.0,
            brake_pedal: 0.0,
            current_gear: 1,
            change_gear_time: 0.0,
            steer_input: 0.0,
            mouse_steering_active: false,
            comedy_control_state: 0,
            engine_on: true,
            is_handbrake_on: false,
            comedy_controls: false,
        }
    }
}

impl VehicleDriveState {
    /// PC `CAutomobile::ProcessControlInputs` comedy branch. One draw per
    /// enabled input pass, after normal steering and before disabled brakes.
    pub(crate) fn apply_comedy_controls(
        &mut self,
        general: &mut re3bevy_core::core::General::CGeneral,
    ) {
        if !self.comedy_controls {
            self.comedy_control_state = 0;
            return;
        }
        let random = general.get_random_number() % 10;
        match self.comedy_control_state {
            0 => {
                if random < 2 {
                    self.comedy_control_state = 1;
                } else if random < 4 {
                    self.comedy_control_state = 2;
                }
            }
            1 => {
                self.steer_angle += 0.05;
                if random < 2 {
                    self.comedy_control_state = 0;
                }
            }
            2 => {
                self.steer_angle -= 0.05;
                if random < 2 {
                    self.comedy_control_state = 0;
                }
            }
            state => {
                bevy::log::warn_once!(target: "re3bevy::vehicle_controls", state,
                    "invalid native comedy-control state; no transition applied");
            }
        }
    }
}

#[cfg(test)]
mod comedy_tests {
    use super::*;
    use re3bevy_core::core::General::CGeneral;

    fn random_with_remainder(remainder: u16) -> CGeneral {
        (0..1000)
            .map(CGeneral::with_seed)
            .find(|general| {
                let mut sample = general.clone();
                sample.get_random_number() % 10 == remainder
            })
            .unwrap()
    }

    #[test]
    fn native_thresholds_select_next_state_without_steering_until_next_pass() {
        for (remainder, state) in [(0, 1), (1, 1), (2, 2), (3, 2), (4, 0), (9, 0)] {
            let mut drive = VehicleDriveState {
                comedy_controls: true,
                steer_angle: 0.2,
                ..Default::default()
            };
            let mut general = random_with_remainder(remainder);
            let mut expected = general.clone();
            expected.get_random_number();
            drive.apply_comedy_controls(&mut general);
            assert_eq!(drive.comedy_control_state, state);
            assert_eq!(drive.steer_angle, 0.2);
            for _ in 0..4 {
                assert_eq!(general.get_random_number(), expected.get_random_number());
            }
        }
    }

    #[test]
    fn steering_precedes_reset_and_disabled_control_does_not_consume_randomness() {
        for (state, offset) in [(1, 0.05), (2, -0.05)] {
            let mut drive = VehicleDriveState {
                comedy_controls: true,
                comedy_control_state: state,
                ..Default::default()
            };
            drive.apply_comedy_controls(&mut random_with_remainder(1));
            assert_eq!(drive.steer_angle, offset);
            assert_eq!(drive.comedy_control_state, 0);
        }
        let mut drive = VehicleDriveState {
            comedy_control_state: 2,
            steer_angle: 0.2,
            ..Default::default()
        };
        let mut general = CGeneral::with_seed(17);
        let mut expected = general.clone();
        drive.apply_comedy_controls(&mut general);
        assert_eq!(drive.comedy_control_state, 0);
        assert_eq!(drive.steer_angle, 0.2);
        assert_eq!(general.get_random_number(), expected.get_random_number());
    }
}
