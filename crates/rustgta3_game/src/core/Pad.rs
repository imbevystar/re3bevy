//! re3 reference: references/re-GTA/src/core/Pad.cpp
//! re3 reference: references/re-GTA/src/core/Pad.h
//!
//! CPad input snapshot. Bevy-native keys fill NewState; script GetPadState
//! and on-foot control read the same re3-shaped fields.

use bevy::{
    input::{
        ButtonInput,
        keyboard::KeyCode,
        mouse::{MouseButton, MouseWheel},
    },
    prelude::{MessageReader, NextState, Query, Res, ResMut, Resource, Vec2, With, default},
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window},
};
use rustgta3_core::core::GameState::GtaAppState;
use rustgta3_core::core::Timer::CTimer;

use crate::{
    control::Script::ScriptGameplayBridge,
    core::{
        Camera::{CCamera, CameraScriptMode},
        EnhancedInput::GtaGameplayInput,
        Frontend::CMenuManager,
        Game::GameplayCursorCapture,
    },
    entities::Physical::GTA_PHYSICS_HZ,
    peds::PlayerPed::CWorldPlayers,
};

pub const PLAYERCONTROL_ENABLED: u8 = 0;
pub const PLAYERCONTROL_CAMERA: u8 = 1;
pub const PLAYERCONTROL_UNK2: u8 = 2;
pub const PLAYERCONTROL_GARAGE: u8 = 4;
pub const PLAYERCONTROL_UNK8: u8 = 8;
pub const PLAYERCONTROL_UNK10: u8 = 16;
pub const PLAYERCONTROL_PLAYERINFO: u8 = 32;
pub const PLAYERCONTROL_PHONE: u8 = 64;
pub const PLAYERCONTROL_CUTSCENE: u8 = 128;

/// re3 `CControllerState` — button fields are 0 or non-zero (typically 255).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct CControllerState {
    pub left_stick_x: i16,
    pub left_stick_y: i16,
    pub right_stick_x: i16,
    pub right_stick_y: i16,
    pub left_shoulder1: i16,
    pub left_shoulder2: i16,
    pub right_shoulder1: i16,
    pub right_shoulder2: i16,
    pub dpad_up: i16,
    pub dpad_down: i16,
    pub dpad_left: i16,
    pub dpad_right: i16,
    pub start: i16,
    pub select: i16,
    pub square: i16,
    pub triangle: i16,
    pub cross: i16,
    pub circle: i16,
    pub left_shock: i16,
    pub right_shock: i16,
}

impl CControllerState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn has_input(&self) -> bool {
        self.left_stick_x != 0
            || self.left_stick_y != 0
            || self.right_stick_x != 0
            || self.right_stick_y != 0
            || self.left_shoulder1 != 0
            || self.left_shoulder2 != 0
            || self.right_shoulder1 != 0
            || self.right_shoulder2 != 0
            || self.dpad_up != 0
            || self.dpad_down != 0
            || self.dpad_left != 0
            || self.dpad_right != 0
            || self.start != 0
            || self.select != 0
            || self.square != 0
            || self.triangle != 0
            || self.cross != 0
            || self.circle != 0
            || self.left_shock != 0
            || self.right_shock != 0
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Resource)]
pub struct CPad {
    pub new_state: CControllerState,
    pub old_state: CControllerState,
    /// re3 `CPad::Mode` / frontend controller configuration (0..=3).
    pub mode: i16,
    pub disable_player_controls: bool,
    /// re3 `CPad::DisablePlayerControls`; each subsystem owns one bit.
    pub disable_player_controls_mask: u8,
    /// re3 `CPad::bApplyBrakes`, controlled by SCM for the player's car.
    pub apply_brakes: bool,
    /// Motor command installed by `CPad::StartShake`. Platform input adapters
    /// consume these values; gameplay owns the re3 duration/frequency state.
    pub shake_duration: u32,
    pub shake_frequency: u32,
    /// re3 `NewMouseControllerState.WHEELUP`
    pub mouse_wheel_up: bool,
    /// re3 `NewMouseControllerState.WHEELDN`
    pub mouse_wheel_down: bool,
    /// Number of weapon slot steps to cycle (supports multi-notch fast scrolling).
    pub cycle_weapon_steps: i32,
    /// re3 horn history, newest first; five fixed GTA pad samples.
    pub horn_history: [bool; 5],
}

/// Dual pads like re3 `Pads[2]`.
#[derive(Debug, Clone, Resource)]
pub struct CPads {
    pub pads: [CPad; 2],
}

impl Default for CPads {
    fn default() -> Self {
        Self {
            pads: [CPad::default(), CPad::default()],
        }
    }
}

impl CPads {
    pub fn get_pad(&self, index: i32) -> &CPad {
        let i = if index <= 0 { 0 } else { 1 };
        &self.pads[i]
    }

    pub fn get_pad_mut(&mut self, index: i32) -> &mut CPad {
        let i = if index <= 0 { 0 } else { 1 };
        &mut self.pads[i]
    }

    /// re3 `CPad::StopPadsShaking` stops pad zero, not both pad slots.
    pub fn stop_pads_shaking(&mut self) {
        self.pads[0].shake_duration = 0;
        self.pads[0].shake_frequency = 0;
    }
}

impl CPad {
    pub fn set_disable_player_controls(&mut self, owner: u8) {
        self.disable_player_controls_mask |= owner;
        self.disable_player_controls = self.disable_player_controls_mask != PLAYERCONTROL_ENABLED;
    }

    pub fn set_enable_player_controls(&mut self, owner: u8) {
        self.disable_player_controls_mask &= !owner;
        self.disable_player_controls = self.disable_player_controls_mask != PLAYERCONTROL_ENABLED;
    }

    pub fn is_player_controls_disabled_by(&self, owner: u8) -> bool {
        self.disable_player_controls_mask & owner != 0
    }

    #[allow(non_snake_case)]
    pub fn Update(&mut self, state: CControllerState) {
        self.old_state = self.new_state;
        self.new_state = state;
        self.horn_history.rotate_right(1);
        self.horn_history[0] = self.get_horn();
    }

    pub fn set_mode(&mut self, mode: i16) {
        self.mode = mode.clamp(0, 3);
    }

    pub fn start_shake(
        &mut self,
        duration: u32,
        frequency: u32,
        vibration_enabled: bool,
        cutscene_running: bool,
        playing_intro: bool,
    ) {
        if !vibration_enabled || cutscene_running || playing_intro {
            return;
        }
        if frequency == 0 {
            self.shake_duration = 0;
            self.shake_frequency = 0;
        } else if duration > self.shake_duration {
            self.shake_duration = duration;
            self.shake_frequency = frequency.min(u8::MAX as u32);
        }
    }

    /// PC/XInput `CPad::Update` decrements the active motor command by GTA's
    /// scaled frame duration and clears frequency on the terminal frame.
    pub fn update_shake(&mut self, time_step_ms: u32) {
        if self.shake_duration < time_step_ms {
            self.shake_duration = 0;
        } else {
            self.shake_duration -= time_step_ms;
        }
        if self.shake_duration == 0 {
            self.shake_frequency = 0;
        }
    }

    /// re3 `CPad::GetPedWalkLeftRight`.
    pub fn get_ped_walk_left_right(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 2 => {
                let axis = self.new_state.left_stick_x;
                let dpad = (self.new_state.dpad_right - self.new_state.dpad_left) / 2;
                if axis.abs() > dpad.abs() { axis } else { dpad }
            }
            1 | 3 => self.new_state.left_stick_x,
            _ => 0,
        }
    }

    /// re3 `CPad::GetPedWalkUpDown`.
    pub fn get_ped_walk_up_down(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 2 => {
                let axis = self.new_state.left_stick_y;
                let dpad = (self.new_state.dpad_down - self.new_state.dpad_up) / 2;
                if axis.abs() > dpad.abs() { axis } else { dpad }
            }
            1 | 3 => self.new_state.left_stick_y,
            _ => 0,
        }
    }

    /// re3 `CPad::GetSprint`.
    pub fn get_sprint(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 3 => self.new_state.cross != 0,
            2 => self.new_state.circle != 0,
            _ => false,
        }
    }

    /// re3 `CPad::GetSteeringLeftRight`.
    pub fn get_steering_left_right(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 2 => {
                let axis = self.new_state.left_stick_x;
                let dpad = (self.new_state.dpad_right - self.new_state.dpad_left) / 2;
                if axis.abs() > dpad.abs() { axis } else { dpad }
            }
            1 | 3 => self.new_state.left_stick_x,
            _ => 0,
        }
    }

    /// re3 `CPad::GetHandBrake`.
    pub fn get_hand_brake(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 1 => self.new_state.right_shoulder1,
            2 => self.new_state.triangle,
            3 => self.new_state.left_shoulder1,
            _ => 0,
        }
    }

    /// re3 `CPad::GetBrake`.
    pub fn get_brake(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.square,
            3 => (2 * self.new_state.right_stick_y).max(0),
            _ => 0,
        }
    }

    /// re3 `CPad::GetAccelerate`.
    pub fn get_accelerate(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.cross,
            3 => (-2 * self.new_state.right_stick_y).max(0),
            _ => 0,
        }
    }

    /// re3 `CPad::GetExitVehicle` / `ExitVehicleJustDown`.
    pub fn get_exit_vehicle(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 3 => self.new_state.triangle != 0,
            2 => self.new_state.left_shoulder1 != 0,
            _ => false,
        }
    }

    pub fn exit_vehicle_just_down(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 3 => self.new_state.triangle != 0 && self.old_state.triangle == 0,
            2 => self.new_state.left_shoulder1 != 0 && self.old_state.left_shoulder1 == 0,
            _ => false,
        }
    }

    /// Raw re3 `GetCrossJustDown`; MESSAGE_WAIT uses this independently of
    /// the active controller mode and disabled-player-control mapping.
    pub fn cross_just_down(&self) -> bool {
        self.new_state.cross != 0 && self.old_state.cross == 0
    }

    pub fn jump_just_down(&self) -> bool {
        !self.disable_player_controls && self.new_state.square != 0 && self.old_state.square == 0
    }

    pub fn start_just_down(&self) -> bool {
        self.new_state.start != 0 && self.old_state.start == 0
    }

    /// re3 `CPad::LookAroundLeftRight` for the third-person mouse camera.
    pub fn look_around_left_right(&self) -> i16 {
        let axis = self.new_state.right_stick_x as f32;
        if axis.abs() > 85.0 {
            ((axis + if axis > 0.0 { -85.0 } else { 85.0 }) * (127.0 / 32.0)) as i16
        } else if axis.abs() > 10.0 {
            ((axis + if axis > 0.0 { -10.0 } else { 10.0 }) * (127.0 / 64.0)) as i16
        } else {
            0
        }
    }

    /// re3 `CPad::LookAroundUpDown`, including the PC `FIX_BUGS` inversion.
    pub fn look_around_up_down(&self) -> i16 {
        let axis = -(self.new_state.right_stick_y as f32);
        if axis.abs() > 85.0 {
            ((axis + if axis > 0.0 { -85.0 } else { 85.0 }) * (127.0 / 32.0)) as i16
        } else if axis.abs() > 40.0 {
            ((axis + if axis > 0.0 { -40.0 } else { 40.0 }) * (127.0 / 64.0)) as i16
        } else {
            0
        }
    }

    /// re3 `CPad::GetWeapon`.
    pub fn get_weapon(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 => self.new_state.circle != 0,
            2 => self.new_state.cross != 0,
            3 => self.new_state.right_shoulder1 != 0,
            _ => false,
        }
    }

    pub fn weapon_just_down(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 => self.new_state.circle != 0 && self.old_state.circle == 0,
            2 => self.new_state.cross != 0 && self.old_state.cross == 0,
            3 => self.new_state.right_shoulder1 != 0 && self.old_state.right_shoulder1 == 0,
            _ => false,
        }
    }

    /// re3 `CPad::GetTarget`.
    pub fn get_target(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.right_shoulder1 != 0,
            3 => self.new_state.left_shoulder1 != 0,
            _ => false,
        }
    }

    /// re3 `CPad::TargetJustDown`.
    pub fn target_just_down(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.right_shoulder1 != 0 && self.old_state.right_shoulder1 == 0,
            3 => self.new_state.left_shoulder1 != 0 && self.old_state.left_shoulder1 == 0,
            _ => false,
        }
    }

    /// re3 `CPad::ShiftTargetLeftJustDown`.
    pub fn shift_target_left_just_down(&self) -> bool {
        !self.disable_player_controls
            && self.new_state.left_shoulder2 != 0
            && self.old_state.left_shoulder2 == 0
    }

    /// re3 `CPad::ShiftTargetRightJustDown`.
    pub fn shift_target_right_just_down(&self) -> bool {
        !self.disable_player_controls
            && self.new_state.right_shoulder2 != 0
            && self.old_state.right_shoulder2 == 0
    }

    /// re3 `CPad::CarGunJustDown`; mode 2 intentionally uses Circle rather
    /// than the on-foot weapon mapping.
    pub fn car_gun_just_down(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.circle != 0 && self.old_state.circle == 0,
            3 => self.new_state.right_shoulder1 != 0 && self.old_state.right_shoulder1 == 0,
            _ => false,
        }
    }

    /// re3 `CPad::GetCarGunFired`.
    pub fn get_car_gun_fired(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.circle != 0,
            3 => self.new_state.right_shoulder1 != 0,
            _ => false,
        }
    }

    /// re3 `CPad::GetCarGunUpDown`.
    pub fn get_car_gun_up_down(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.right_stick_y,
            3 => (self.new_state.dpad_up - self.new_state.dpad_down) / 2,
            _ => 0,
        }
    }

    /// re3 `CPad::GetCarGunLeftRight`.
    pub fn get_car_gun_left_right(&self) -> i16 {
        if self.disable_player_controls {
            return 0;
        }
        match self.mode {
            0 | 1 | 2 => self.new_state.right_stick_x,
            3 => (self.new_state.dpad_right - self.new_state.dpad_left) / 2,
            _ => 0,
        }
    }

    /// re3 `CPad::GetLookLeft`.
    pub fn get_look_left(&self) -> bool {
        !self.disable_player_controls
            && self.new_state.left_shoulder2 != 0
            && self.new_state.right_shoulder2 == 0
    }

    /// re3 `CPad::GetLookRight`.
    pub fn get_look_right(&self) -> bool {
        !self.disable_player_controls
            && self.new_state.right_shoulder2 != 0
            && self.new_state.left_shoulder2 == 0
    }

    /// re3 `CPad::GetLookBehindForCar`.
    pub fn get_look_behind_for_car(&self) -> bool {
        !self.disable_player_controls
            && self.new_state.left_shoulder2 != 0
            && self.new_state.right_shoulder2 != 0
    }

    /// re3 `CPad::GetLookBehindForPed`.
    pub fn get_look_behind_for_ped(&self) -> bool {
        !self.disable_player_controls && self.new_state.right_shock != 0
    }

    /// re3 `CPad::GetHorn`.
    pub fn get_horn(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 3 => self.new_state.left_shock != 0,
            1 => self.new_state.left_shoulder1 != 0,
            2 => self.new_state.right_shoulder1 != 0,
            _ => false,
        }
    }

    /// re3 `CPad::ChangeStationJustDown`.
    pub fn change_station_just_down(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 => self.new_state.left_shoulder1 != 0 && self.old_state.left_shoulder1 == 0,
            1 => self.new_state.select != 0 && self.old_state.select == 0,
            2 => self.new_state.left_shock != 0 && self.old_state.left_shock == 0,
            3 => self.new_state.circle != 0 && self.old_state.circle == 0,
            _ => false,
        }
    }

    /// re3 `CPad::CycleCameraModeUpJustDown`.
    pub fn cycle_camera_mode_up_just_down(&self) -> bool {
        if self.mode == 1 {
            self.new_state.dpad_up != 0 && self.old_state.dpad_up == 0
        } else {
            self.new_state.select != 0 && self.old_state.select == 0
        }
    }

    /// re3 `CPad::CycleCameraModeDownJustDown`.
    pub fn cycle_camera_mode_down_just_down(&self) -> bool {
        self.mode == 1 && self.new_state.dpad_down != 0 && self.old_state.dpad_down == 0
    }

    /// re3 `CPad::ForceCameraBehindPlayer`.
    pub fn force_camera_behind_player(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 => self.new_state.left_shoulder1 != 0,
            2 => self.new_state.triangle != 0,
            3 => self.new_state.circle != 0,
            _ => false,
        }
    }

    pub fn cycle_weapon_right_just_down(&self) -> bool {
        !self.disable_player_controls
            && ((self.new_state.right_shoulder2 != 0 && self.old_state.right_shoulder2 == 0)
                || self.mouse_wheel_down)
    }

    pub fn cycle_weapon_left_just_down(&self) -> bool {
        !self.disable_player_controls
            && ((self.new_state.left_shoulder2 != 0 && self.old_state.left_shoulder2 == 0)
                || self.mouse_wheel_up)
    }

    /// re3 `CPad::SniperZoomIn`.
    pub fn sniper_zoom_in(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 3 => self.new_state.square != 0,
            2 => self.new_state.triangle != 0,
            _ => false,
        }
    }

    /// re3 `CPad::SniperZoomOut`.
    pub fn sniper_zoom_out(&self) -> bool {
        if self.disable_player_controls {
            return false;
        }
        match self.mode {
            0 | 1 | 3 => self.new_state.cross != 0,
            2 => self.new_state.square != 0,
            _ => false,
        }
    }

    /// FIX_BUGS re3 `CPad::SniperModeLookLeftRight`.
    pub fn sniper_mode_look_left_right(&self) -> i16 {
        let axis = self.new_state.left_stick_x;
        let dpad = (self.new_state.dpad_right - self.new_state.dpad_left) / 2;
        if axis.abs() > dpad.abs() { axis } else { dpad }
    }

    /// FIX_BUGS re3 `CPad::SniperModeLookUpDown`.
    pub fn sniper_mode_look_up_down(&self) -> i16 {
        let axis = -self.new_state.left_stick_y;
        let dpad = (self.new_state.dpad_up - self.new_state.dpad_down) / 2;
        if axis.abs() > dpad.abs() { axis } else { dpad }
    }

    /// re3 `CRunningScript::GetPadState(pad, button)` field table.
    pub fn get_pad_state_button(&self, button: i32) -> i16 {
        match button {
            0 => self.new_state.left_stick_x,
            1 => self.new_state.left_stick_y,
            2 => self.new_state.right_stick_x,
            3 => self.new_state.right_stick_y,
            4 => self.new_state.left_shoulder1,
            5 => self.new_state.left_shoulder2,
            6 => self.new_state.right_shoulder1,
            7 => self.new_state.right_shoulder2,
            8 => self.new_state.dpad_up,
            9 => self.new_state.dpad_down,
            10 => self.new_state.dpad_left,
            11 => self.new_state.dpad_right,
            12 => self.new_state.start,
            13 => self.new_state.select,
            14 => self.new_state.square,
            15 => self.new_state.triangle,
            16 => self.new_state.cross,
            17 => self.new_state.circle,
            18 => self.new_state.left_shock,
            19 => self.new_state.right_shock,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_owners_clear_only_their_own_disable_bit() {
        let mut pad = CPad::default();
        pad.set_disable_player_controls(PLAYERCONTROL_CAMERA);
        pad.set_disable_player_controls(PLAYERCONTROL_GARAGE);
        pad.set_enable_player_controls(PLAYERCONTROL_CAMERA);
        assert!(!pad.is_player_controls_disabled_by(PLAYERCONTROL_CAMERA));
        assert!(pad.is_player_controls_disabled_by(PLAYERCONTROL_GARAGE));
        assert!(pad.disable_player_controls);
        pad.set_enable_player_controls(PLAYERCONTROL_GARAGE);
        assert!(!pad.disable_player_controls);
    }

    #[test]
    fn vehicle_getters_match_re3_mode_zero_pad_fields() {
        let mut pad = CPad::default();
        pad.new_state.left_stick_x = -64;
        pad.new_state.cross = 255;
        pad.new_state.square = 255;
        pad.new_state.right_shoulder1 = 255;
        pad.new_state.triangle = 255;
        pad.old_state.triangle = 255;
        assert_eq!(pad.get_steering_left_right(), -64);
        assert_eq!(pad.get_accelerate(), 255);
        assert_eq!(pad.get_brake(), 255);
        assert_eq!(pad.get_hand_brake(), 255);
        assert!(pad.get_exit_vehicle());
        assert!(!pad.exit_vehicle_just_down());
        pad.old_state.triangle = 0;
        assert!(pad.exit_vehicle_just_down());
        assert!(pad.get_target());
        assert!(pad.target_just_down());
        pad.old_state.right_shoulder1 = 255;
        assert!(!pad.target_just_down());
    }

    #[test]
    fn target_shift_and_force_camera_getters_match_re3_fields() {
        let mut pad = CPad::default();
        pad.new_state.left_shoulder2 = 255;
        assert!(pad.shift_target_left_just_down());
        pad.old_state.left_shoulder2 = 255;
        assert!(!pad.shift_target_left_just_down());

        pad.new_state.right_shoulder2 = 255;
        assert!(pad.shift_target_right_just_down());
        pad.old_state.right_shoulder2 = 255;
        assert!(!pad.shift_target_right_just_down());

        pad.new_state.left_shoulder1 = 255;
        assert!(pad.force_camera_behind_player());
        pad.mode = 2;
        pad.new_state.left_shoulder1 = 0;
        pad.new_state.triangle = 255;
        assert!(pad.force_camera_behind_player());
        pad.mode = 3;
        pad.new_state.triangle = 0;
        pad.new_state.circle = 255;
        assert!(pad.force_camera_behind_player());
        pad.disable_player_controls = true;
        assert!(!pad.force_camera_behind_player());
    }

    #[test]
    fn mouse_wheel_weapon_cycle_detection() {
        let mut pad = CPad::default();
        pad.mouse_wheel_up = true;
        pad.cycle_weapon_steps = 1;
        assert!(pad.cycle_weapon_left_just_down());
        assert!(!pad.cycle_weapon_right_just_down());

        // Consecutive frame scroll up should not be blocked by old_state
        pad.mouse_wheel_up = true;
        assert!(pad.cycle_weapon_left_just_down());

        pad.mouse_wheel_up = false;
        pad.mouse_wheel_down = true;
        assert!(pad.cycle_weapon_right_just_down());
        assert!(!pad.cycle_weapon_left_just_down());
    }
}

/// Build a controller state from discrete keyboard flags (Bevy KeyCode mapping).
/// re3 PC maps WASD-style movement onto LeftStick axes (−128..128).
pub fn controller_state_from_keyboard(
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
    sprint: bool,
    jump: bool,
    fire: bool,
    enter: bool,
) -> CControllerState {
    let mut state = CControllerState::default();
    if forward {
        state.left_stick_y = -128;
    } else if back {
        state.left_stick_y = 128;
    }
    if left {
        state.left_stick_x = -128;
    } else if right {
        state.left_stick_x = 128;
    }
    if sprint {
        state.cross = 255;
    }
    if jump {
        state.square = 255;
    }
    if fire {
        state.circle = 255;
    }
    if enter {
        state.triangle = 255;
    }
    state
}

// === FAZ8c-pad: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn update_gameplay_mouse_capture(
    buttons: Res<ButtonInput<MouseButton>>,
    input: Res<GtaGameplayInput>,
    pads: Res<CPads>,
    timer: Res<CTimer>,
    bridge: Res<ScriptGameplayBridge>,
    players: CWorldPlayers,
    menu: Option<Res<CMenuManager>>,
    mut camera: Option<ResMut<CCamera>>,
    mut capture: ResMut<GameplayCursorCapture>,
    mut next_state: ResMut<NextState<GtaAppState>>,
    mut cursor_query: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
) {
    let Ok((window, mut cursor)) = cursor_query.single_mut() else {
        return;
    };
    let escape_pressed = pads.get_pad(0).start_just_down();
    let can_control_camera = bridge.player_control_enabled && !bridge.playing_intro;
    let mut just_captured = false;
    if !window.focused || escape_pressed || !can_control_camera {
        if window.focused && escape_pressed && can_control_camera {
            next_state.set(GtaAppState::PauseMenu);
        }
        capture.captured = false;
    } else if !capture.captured && buttons.just_pressed(MouseButton::Left) && can_control_camera {
        capture.captured = true;
        just_captured = true;
    }

    if std::env::var_os("RUSTGTA3_DISABLE_MOUSE_GRAB").is_some_and(|v| v != "0") {
        capture.captured = false;
        just_captured = false;
    }
    capture.just_captured = just_captured;

    // Android has no desktop cursor-grab API; do not retry it every frame.
    if !cfg!(target_os = "android") {
        cursor.grab_mode = if capture.captured {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        cursor.visible = !capture.captured;
    }

    let delta = input.mouse_look;
    let Some(camera) = camera.as_mut() else {
        return;
    };
    camera.free_cam = menu.as_ref().is_some_and(|menu| menu.prefs_free_cam);
    if can_control_camera {
        camera.process_view_selection(
            pads.get_pad(0),
            players.relation(bridge.player_index).is_some(),
            menu.as_ref()
                .is_none_or(|menu| menu.control_method == crate::core::Frontend::CONTROL_STANDARD),
        );
        if (input.center_camera || pads.get_pad(0).force_camera_behind_player())
            && let Some(player) = players.player(bridge.player_index)
        {
            camera.set_camera_directly_behind_for_follow_ped(player.locomotion.rotation_cur);
        }
    }
    if !camera.follow_initialised {
        let heading = players
            .player(bridge.player_index)
            .map(|player| player.locomotion.rotation_cur)
            .unwrap_or_else(|| bridge.player_heading_degrees.to_radians());
        let forward_x = -heading.sin();
        let forward_y = heading.cos();
        camera.follow_beta = forward_y.atan2(forward_x);
        camera.follow_alpha = -0.22;
        camera.follow_initialised = true;
    }
    if !bridge.player_control_enabled
        || bridge.playing_intro
        || camera.script_mode != CameraScriptMode::FollowPlayer
    {
        return;
    }
    if camera.weapon_mode_active() { return; }
    let sensitivity = menu
        .as_ref()
        .map(|menu| menu.mouse_sensitivity as f32 / 100.0)
        .unwrap_or(1.0);
    let invert = menu.as_ref().is_some_and(|menu| menu.mouse_invert);
    let use_mouse = (capture.captured || cfg!(target_os = "android"))
        && menu.as_ref().is_none_or(|menu| menu.control_method == 0)
        && delta != Vec2::ZERO;
    let (beta_delta, pitch_delta) = if use_mouse {
        // CCam::Process_FollowPedWithMouse: LookLeftRight=-2.5*MouseX,
        // LookUpDown=4*MouseY, then the PC camera accelerations.
        (
            -2.5 * delta.x * 0.0025 * sensitivity,
            4.0 * delta.y * 0.003 * sensitivity * if invert { -1.0 } else { 1.0 },
        )
    } else {
        // Same function's non-mouse branch. CTimer::GetTimeStep is based on
        // GTA's 50 Hz physics unit, not seconds.
        let pad = pads.get_pad(0);
        let gta_time_step = timer.time_step_in_seconds().clamp(0.0, 0.1) * GTA_PHYSICS_HZ;
        let fov_scale = 70.0 / 80.0;
        let look_left_right = -(pad.look_around_left_right() as f32);
        let look_up_down = pad.look_around_up_down() as f32;
        (
            look_left_right * 0.01 * (1.0 / 14.0) * fov_scale * gta_time_step,
            look_up_down * 0.01 * (0.6 / 14.0) * fov_scale * gta_time_step,
        )
    };
    camera.follow_beta += beta_delta;
    camera.follow_alpha =
        (camera.follow_alpha + pitch_delta).clamp(-89.5_f32.to_radians(), 45.0_f32.to_radians());
}

fn keyboard_movement_dpad(
    keyboard: Option<&ButtonInput<KeyCode>>,
    controller: Option<&crate::core::ControllerConfig::CControllerConfigManager>,
) -> (bool, bool, bool, bool) {
    use crate::core::ControllerConfig::e_ControllerAction::{
        GO_BACK, GO_FORWARD, GO_LEFT, GO_RIGHT,
    };
    let Some(keys) = keyboard else {
        return (false, false, false, false);
    };
    let pressed = |action, fallback| {
        controller
            .map(|config| config.action_pressed(action, keys))
            .unwrap_or_else(|| keys.pressed(fallback))
    };
    (
        pressed(GO_FORWARD, KeyCode::KeyW),
        pressed(GO_BACK, KeyCode::KeyS),
        pressed(GO_LEFT, KeyCode::KeyA),
        pressed(GO_RIGHT, KeyCode::KeyD),
    )
}

pub fn update_pads_from_bevy_input(
    input: Res<GtaGameplayInput>,
    timer: Res<CTimer>,
    world_players: CWorldPlayers,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut pads: ResMut<CPads>,
    capture: Option<Res<GameplayCursorCapture>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    controller: Option<Res<crate::core::ControllerConfig::CControllerConfigManager>>,
    camera: Option<Res<CCamera>>,
    menu: Option<Res<CMenuManager>>,
) {
    // win.cpp/XInput and Pad.cpp keep the raw stick pair only when either
    // component exceeds 0.3. Do not use Enhanced Input's remapping dead-zone:
    // re3 preserves the original magnitude above the threshold.
    let re3_stick_dead_zone = |value: Vec2| {
        if value.x.abs() > 0.3 || value.y.abs() > 0.3 {
            value
        } else {
            Vec2::ZERO
        }
    };
    let movement = re3_stick_dead_zone(input.movement);
    let stick_look = re3_stick_dead_zone(input.stick_look);
    let axis = |value: f32| (value.clamp(-1.0, 1.0) * 128.0).round() as i16;
    let button = |pressed: bool| if pressed { 255 } else { 0 };
    let (keyboard_forward, keyboard_back, keyboard_left, keyboard_right) =
        keyboard_movement_dpad(keyboard.as_deref(), controller.as_deref());
    let player_in_vehicle = world_players
        .relation(world_players.player_in_focus() as i32)
        .is_some();

    let mouse_fire_suppressed = !cfg!(target_os = "android") && capture
        .as_ref()
        .is_some_and(|c| !c.captured || c.just_captured)
        && mouse_buttons
            .as_ref()
            .is_some_and(|mb| mb.pressed(MouseButton::Left));
    // Enhanced Input already resolves bindings from all input devices.
    // Rechecking physical keyboard/mouse here rejects gamepad and touch actions.
    let fire = input.fire && !mouse_fire_suppressed;
    let target = input.target;

    let mut wheel_y = 0.0_f32;
    for ev in mouse_wheel.read() {
        wheel_y += ev.y;
    }
    let wheel_up = wheel_y > 0.0;
    let wheel_down = wheel_y < 0.0;
    let wheel_steps = if wheel_y != 0.0 {
        (wheel_y.abs().round() as i32).max(1)
    } else {
        1
    };

    let first_person = camera.as_ref().is_some_and(|camera| {
        camera.cams.get(camera.active_cam).is_some_and(|cam| {
            matches!(
                cam.mode,
                crate::core::Cam::eCamMode::MODE_SNIPER
                    | crate::core::Cam::eCamMode::MODE_ROCKETLAUNCHER
                    | crate::core::Cam::eCamMode::MODE_1STPERSON
            )
        })
    });
    let classic_controls = menu
        .as_ref()
        .is_some_and(|menu| menu.control_method != crate::core::Frontend::CONTROL_STANDARD);
    let key_axis = |negative, positive| {
        let pressed = |action| {
            keyboard
                .as_ref()
                .zip(controller.as_ref())
                .is_some_and(|(keyboard, controller)| controller.action_pressed(action, keyboard))
        };
        (i16::from(pressed(positive)) - i16::from(pressed(negative))) * 128
    };
    use crate::core::ControllerConfig::e_ControllerAction::*;
    let keyboard_look_x = if player_in_vehicle {
        key_axis(VEHICLE_TURRETLEFT, VEHICLE_TURRETRIGHT)
    } else {
        key_axis(PED_1RST_PERSON_LOOK_LEFT, PED_1RST_PERSON_LOOK_RIGHT)
    };
    let keyboard_look_y = if player_in_vehicle {
        key_axis(VEHICLE_TURRETDOWN, VEHICLE_TURRETUP)
    } else if classic_controls {
        key_axis(PED_1RST_PERSON_LOOK_DOWN, PED_1RST_PERSON_LOOK_UP)
    } else {
        0
    };
    let state = CControllerState {
        left_stick_x: axis(movement.x),
        // re3 forward is negative LeftStickY; Enhanced Input north is +Y.
        left_stick_y: axis(-movement.y),
        right_stick_x: (axis(stick_look.x) + keyboard_look_x).clamp(-128, 128),
        // Pad.cpp maps the host right-stick Y into the opposite GTA axis.
        right_stick_y: (axis(-stick_look.y) + keyboard_look_y).clamp(-128, 128),
        dpad_up: button(keyboard_forward || input.dpad.y > 0.5),
        dpad_down: button(keyboard_back || input.dpad.y < -0.5),
        dpad_left: button(keyboard_left || input.dpad.x < -0.5),
        dpad_right: button(keyboard_right || input.dpad.x > 0.5),
        start: button(input.pause),
        select: button(input.change_camera),
        left_shock: button(player_in_vehicle && input.horn),
        right_shock: button(if player_in_vehicle {
            input.toggle_submissions
        } else {
            input.look_behind
        }),
        // CPad exposes the same physical fields through mode-specific getters:
        // on foot Square is jump, while in a car it is brake/reverse.
        square: button(if player_in_vehicle {
            input.vehicle_brake
        } else if first_person {
            input.sniper_zoom_in
        } else {
            input.jump
        }),
        triangle: button(input.enter_exit),
        // Likewise Cross is sprint on foot and accelerate in a vehicle.
        cross: button(if player_in_vehicle {
            input.vehicle_accelerate
        } else if first_person {
            input.sniper_zoom_out
        } else {
            input.sprint
        }),
        circle: button(fire),
        left_shoulder1: button(if player_in_vehicle {
            input.change_radio || wheel_up
        } else {
            input.center_camera
        }),
        right_shoulder1: button(if player_in_vehicle {
            input.vehicle_handbrake
        } else {
            target
        }),
        left_shoulder2: button(if player_in_vehicle {
            input.look_left || input.vehicle_look_behind
        } else {
            input.cycle_weapon_left || wheel_up || (classic_controls && input.cycle_target_left)
        }),
        right_shoulder2: button(if player_in_vehicle {
            input.look_right || input.vehicle_look_behind
        } else {
            input.cycle_weapon_right || wheel_down || (classic_controls && input.cycle_target_right)
        }),
        ..default()
    };
    let pad0 = pads.get_pad_mut(0);
    pad0.Update(state);
    pad0.mouse_wheel_up = !player_in_vehicle && wheel_up;
    pad0.mouse_wheel_down = !player_in_vehicle && wheel_down;
    pad0.cycle_weapon_steps = wheel_steps;

    let time_step_ms = timer.time_step_in_milliseconds();
    for pad in &mut pads.pads {
        pad.update_shake(time_step_ms);
    }
}

// FAZ4b: moved to peds/PlayerPed.rs verbatim (19 items): fn set_player_jump_animation, fn begin_player_jump, fn advance_player_jump_launch, fn update_player_jump_animation, fn apply_player_running_land, fn with_player_native_collision_speed, fn begin_player_collision_frame, fn finish_player_collision_frame, fn apply_player_ped_air_resistance, fn player_ground_support_extension, fn player_check_if_in_air_has_ground, fn player_check_if_in_air_has_static_foot_sphere, struct PlayerPhysicalSurfaceMotion, fn player_current_surface_motion, fn apply_player_standing_update_position, fn player_can_start_jump_from_surface, fn add_player_movement_association, fn add_player_standard_association, fn should_start_player_run_stop

// FAZ8c-remote: moved to control/Remote.rs verbatim (1 items): fn process_remote_vehicle

#[cfg(test)]
mod keyboard_diagonal_parity_tests {
    use super::*;
    use crate::core::ControllerConfig::CControllerConfigManager;

    fn keyboard_axes(keys: &[KeyCode]) -> (i16, i16) {
        let mut keyboard = ButtonInput::<KeyCode>::default();
        for key in keys {
            keyboard.press(*key);
        }
        let controller = CControllerConfigManager::default();
        let (up, down, left, right) = keyboard_movement_dpad(Some(&keyboard), Some(&controller));
        let pad = CPad {
            new_state: CControllerState {
                dpad_up: if up { 255 } else { 0 },
                dpad_down: if down { 255 } else { 0 },
                dpad_left: if left { 255 } else { 0 },
                dpad_right: if right { 255 } else { 0 },
                ..default()
            },
            ..default()
        };
        (pad.get_ped_walk_left_right(), pad.get_ped_walk_up_down())
    }

    #[test]
    fn keyboard_cardinals_map_to_distinct_native_axes() {
        assert_eq!(keyboard_axes(&[KeyCode::KeyW]), (0, -127));
        assert_eq!(keyboard_axes(&[KeyCode::KeyS]), (0, 127));
        assert_eq!(keyboard_axes(&[KeyCode::KeyA]), (-127, 0));
        assert_eq!(keyboard_axes(&[KeyCode::KeyD]), (127, 0));
    }

    #[test]
    fn keyboard_wasd_diagonals_map_to_native_axes() {
        assert_eq!(keyboard_axes(&[KeyCode::KeyW, KeyCode::KeyA]), (-127, -127));
        assert_eq!(keyboard_axes(&[KeyCode::KeyW, KeyCode::KeyD]), (127, -127));
        assert_eq!(keyboard_axes(&[KeyCode::KeyS, KeyCode::KeyA]), (-127, 127));
        assert_eq!(keyboard_axes(&[KeyCode::KeyS, KeyCode::KeyD]), (127, 127));
    }

    #[test]
    fn keyboard_diagonals_keep_re3_dpad_half_range_on_both_axes() {
        let cases = [
            // W+D, W+A, S+A, S+D
            (255, 0, 0, 255, 127, -127),
            (255, 0, 255, 0, -127, -127),
            (0, 255, 255, 0, -127, 127),
            (0, 255, 0, 255, 127, 127),
        ];

        for (up, down, left, right, expected_x, expected_y) in cases {
            let pad = CPad {
                new_state: CControllerState {
                    dpad_up: up,
                    dpad_down: down,
                    dpad_left: left,
                    dpad_right: right,
                    ..default()
                },
                ..default()
            };
            assert_eq!(pad.get_ped_walk_left_right(), expected_x);
            assert_eq!(pad.get_ped_walk_up_down(), expected_y);
        }
    }

    #[test]
    fn keyboard_wd_diagonal_has_native_magnitude_not_analogue_128_magnitude() {
        let pad = CPad {
            new_state: CControllerState {
                dpad_up: 255,
                dpad_right: 255,
                ..default()
            },
            ..default()
        };
        let x = pad.get_ped_walk_left_right() as f32;
        let y = pad.get_ped_walk_up_down() as f32;
        let native = (x * x + y * y).sqrt() / 60.0;
        let analogue_128 = (128.0_f32 * 128.0 * 2.0).sqrt() / 60.0;

        assert!((native - (127.0_f32 * 127.0 * 2.0).sqrt() / 60.0).abs() < 1.0e-6);
        assert!(native < analogue_128);
    }
}
