//! re3 reference: references/re-GTA/src/core/ControllerConfig.cpp
//! re3 reference: references/re-GTA/src/core/ControllerConfig.h
//!
//! PC controller bindings.  The original has one ordered binding slot for
//! keyboard, optional keyboard, mouse and joystick per action; Bevy input is
//! the platform adapter that consumes the keyboard slots.

use std::collections::BTreeMap;

use bevy::{
    input::{keyboard::KeyCode, mouse::MouseButton},
    prelude::Resource,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum e_ControllerAction {
    PED_FIREWEAPON = 0,
    PED_CYCLE_WEAPON_RIGHT = 1,
    PED_CYCLE_WEAPON_LEFT = 2,
    GO_FORWARD = 3,
    GO_BACK = 4,
    GO_LEFT = 5,
    GO_RIGHT = 6,
    PED_SNIPER_ZOOM_IN = 7,
    PED_SNIPER_ZOOM_OUT = 8,
    VEHICLE_ENTER_EXIT = 9,
    CAMERA_CHANGE_VIEW_ALL_SITUATIONS = 10,
    PED_JUMPING = 11,
    PED_SPRINT = 12,
    PED_LOOKBEHIND = 13,
    VEHICLE_ACCELERATE = 14,
    VEHICLE_BRAKE = 15,
    VEHICLE_CHANGE_RADIO_STATION = 16,
    VEHICLE_HORN = 17,
    TOGGLE_SUBMISSIONS = 18,
    VEHICLE_HANDBRAKE = 19,
    PED_1RST_PERSON_LOOK_LEFT = 20,
    PED_1RST_PERSON_LOOK_RIGHT = 21,
    VEHICLE_LOOKLEFT = 22,
    VEHICLE_LOOKRIGHT = 23,
    VEHICLE_LOOKBEHIND = 24,
    VEHICLE_TURRETLEFT = 25,
    VEHICLE_TURRETRIGHT = 26,
    VEHICLE_TURRETUP = 27,
    VEHICLE_TURRETDOWN = 28,
    PED_CYCLE_TARGET_LEFT = 29,
    PED_CYCLE_TARGET_RIGHT = 30,
    PED_CENTER_CAMERA_BEHIND_PLAYER = 31,
    PED_LOCK_TARGET = 32,
    NETWORK_TALK = 33,
    PED_1RST_PERSON_LOOK_UP = 34,
    PED_1RST_PERSON_LOOK_DOWN = 35,
    UNUSED_36 = 36,
    TOGGLE_DPAD = 37,
    SWITCH_DEBUG_CAM_ON = 38,
    TAKE_SCREEN_SHOT = 39,
    SHOW_MOUSE_POINTER_TOGGLE = 40,
}

impl e_ControllerAction {
    /// Names written between `~k~~...~` in GTA III GXT strings. re3 fills
    /// `m_aActionNames` from these exact enum identifiers in
    /// `CControllerConfigManager::InitDefaultControlConfiguration`.
    pub fn from_gxt_name(name: &str) -> Option<Self> {
        use e_ControllerAction::*;
        Some(match name {
            "PED_FIREWEAPON" => PED_FIREWEAPON,
            "PED_CYCLE_WEAPON_RIGHT" => PED_CYCLE_WEAPON_RIGHT,
            "PED_CYCLE_WEAPON_LEFT" => PED_CYCLE_WEAPON_LEFT,
            "GO_FORWARD" => GO_FORWARD,
            "GO_BACK" => GO_BACK,
            "GO_LEFT" => GO_LEFT,
            "GO_RIGHT" => GO_RIGHT,
            "PED_SNIPER_ZOOM_IN" => PED_SNIPER_ZOOM_IN,
            "PED_SNIPER_ZOOM_OUT" => PED_SNIPER_ZOOM_OUT,
            "VEHICLE_ENTER_EXIT" => VEHICLE_ENTER_EXIT,
            "CAMERA_CHANGE_VIEW_ALL_SITUATIONS" => CAMERA_CHANGE_VIEW_ALL_SITUATIONS,
            "PED_JUMPING" => PED_JUMPING,
            "PED_SPRINT" => PED_SPRINT,
            "PED_LOOKBEHIND" => PED_LOOKBEHIND,
            "VEHICLE_ACCELERATE" => VEHICLE_ACCELERATE,
            "VEHICLE_BRAKE" => VEHICLE_BRAKE,
            "VEHICLE_CHANGE_RADIO_STATION" => VEHICLE_CHANGE_RADIO_STATION,
            "VEHICLE_HORN" => VEHICLE_HORN,
            "TOGGLE_SUBMISSIONS" => TOGGLE_SUBMISSIONS,
            "VEHICLE_HANDBRAKE" => VEHICLE_HANDBRAKE,
            "PED_1RST_PERSON_LOOK_LEFT" => PED_1RST_PERSON_LOOK_LEFT,
            "PED_1RST_PERSON_LOOK_RIGHT" => PED_1RST_PERSON_LOOK_RIGHT,
            "VEHICLE_LOOKLEFT" => VEHICLE_LOOKLEFT,
            "VEHICLE_LOOKRIGHT" => VEHICLE_LOOKRIGHT,
            "VEHICLE_LOOKBEHIND" => VEHICLE_LOOKBEHIND,
            "VEHICLE_TURRETLEFT" => VEHICLE_TURRETLEFT,
            "VEHICLE_TURRETRIGHT" => VEHICLE_TURRETRIGHT,
            "VEHICLE_TURRETUP" => VEHICLE_TURRETUP,
            "VEHICLE_TURRETDOWN" => VEHICLE_TURRETDOWN,
            "PED_CYCLE_TARGET_LEFT" => PED_CYCLE_TARGET_LEFT,
            "PED_CYCLE_TARGET_RIGHT" => PED_CYCLE_TARGET_RIGHT,
            "PED_CENTER_CAMERA_BEHIND_PLAYER" => PED_CENTER_CAMERA_BEHIND_PLAYER,
            "PED_LOCK_TARGET" => PED_LOCK_TARGET,
            "NETWORK_TALK" => NETWORK_TALK,
            "PED_1RST_PERSON_LOOK_UP" => PED_1RST_PERSON_LOOK_UP,
            "PED_1RST_PERSON_LOOK_DOWN" => PED_1RST_PERSON_LOOK_DOWN,
            "TOGGLE_DPAD" => TOGGLE_DPAD,
            "SWITCH_DEBUG_CAM_ON" => SWITCH_DEBUG_CAM_ON,
            "TAKE_SCREEN_SHOT" => TAKE_SCREEN_SHOT,
            "SHOW_MOUSE_POINTER_TOGGLE" => SHOW_MOUSE_POINTER_TOGGLE,
            _ => return None,
        })
    }

    pub const KEYBOARD_MENU_ACTIONS: [Self; 35] = [
        Self::PED_FIREWEAPON,
        Self::PED_CYCLE_WEAPON_RIGHT,
        Self::PED_CYCLE_WEAPON_LEFT,
        Self::GO_FORWARD,
        Self::GO_BACK,
        Self::GO_LEFT,
        Self::GO_RIGHT,
        Self::PED_SNIPER_ZOOM_IN,
        Self::PED_SNIPER_ZOOM_OUT,
        Self::VEHICLE_ENTER_EXIT,
        Self::CAMERA_CHANGE_VIEW_ALL_SITUATIONS,
        Self::PED_JUMPING,
        Self::PED_SPRINT,
        Self::PED_LOOKBEHIND,
        Self::PED_LOCK_TARGET,
        Self::VEHICLE_ACCELERATE,
        Self::VEHICLE_BRAKE,
        Self::VEHICLE_CHANGE_RADIO_STATION,
        Self::VEHICLE_HORN,
        Self::TOGGLE_SUBMISSIONS,
        Self::VEHICLE_HANDBRAKE,
        Self::VEHICLE_LOOKLEFT,
        Self::VEHICLE_LOOKRIGHT,
        Self::PED_CYCLE_TARGET_LEFT,
        Self::PED_CYCLE_TARGET_RIGHT,
        Self::PED_1RST_PERSON_LOOK_LEFT,
        Self::PED_1RST_PERSON_LOOK_RIGHT,
        Self::PED_1RST_PERSON_LOOK_UP,
        Self::PED_1RST_PERSON_LOOK_DOWN,
        Self::VEHICLE_LOOKBEHIND,
        Self::VEHICLE_TURRETLEFT,
        Self::VEHICLE_TURRETRIGHT,
        Self::VEHICLE_TURRETUP,
        Self::VEHICLE_TURRETDOWN,
        Self::PED_CENTER_CAMERA_BEHIND_PLAYER,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PED_FIREWEAPON => "FIRE WEAPON",
            Self::PED_CYCLE_WEAPON_RIGHT => "NEXT WEAPON",
            Self::PED_CYCLE_WEAPON_LEFT => "PREVIOUS WEAPON",
            Self::GO_FORWARD => "GO FORWARD",
            Self::GO_BACK => "GO BACK",
            Self::GO_LEFT => "GO LEFT",
            Self::GO_RIGHT => "GO RIGHT",
            Self::PED_SNIPER_ZOOM_IN => "SNIPER ZOOM IN",
            Self::PED_SNIPER_ZOOM_OUT => "SNIPER ZOOM OUT",
            Self::VEHICLE_ENTER_EXIT => "ENTER / EXIT VEHICLE",
            Self::CAMERA_CHANGE_VIEW_ALL_SITUATIONS => "CHANGE CAMERA",
            Self::PED_JUMPING => "JUMP",
            Self::PED_SPRINT => "SPRINT",
            Self::PED_LOOKBEHIND => "LOOK BEHIND",
            Self::VEHICLE_ACCELERATE => "ACCELERATE",
            Self::VEHICLE_BRAKE => "BRAKE / REVERSE",
            Self::VEHICLE_CHANGE_RADIO_STATION => "CHANGE RADIO STATION",
            Self::VEHICLE_HORN => "HORN",
            Self::TOGGLE_SUBMISSIONS => "TOGGLE SUBMISSIONS",
            Self::VEHICLE_HANDBRAKE => "HANDBRAKE",
            Self::PED_1RST_PERSON_LOOK_LEFT => "FIRST PERSON LOOK LEFT",
            Self::PED_1RST_PERSON_LOOK_RIGHT => "FIRST PERSON LOOK RIGHT",
            Self::VEHICLE_LOOKLEFT => "VEHICLE LOOK LEFT",
            Self::VEHICLE_LOOKRIGHT => "VEHICLE LOOK RIGHT",
            Self::VEHICLE_LOOKBEHIND => "VEHICLE LOOK BEHIND",
            Self::VEHICLE_TURRETLEFT => "TURRET LEFT",
            Self::VEHICLE_TURRETRIGHT => "TURRET RIGHT",
            Self::VEHICLE_TURRETUP => "TURRET UP",
            Self::VEHICLE_TURRETDOWN => "TURRET DOWN",
            Self::PED_CYCLE_TARGET_LEFT => "PREVIOUS TARGET",
            Self::PED_CYCLE_TARGET_RIGHT => "NEXT TARGET",
            Self::PED_CENTER_CAMERA_BEHIND_PLAYER => "CENTER CAMERA",
            Self::PED_LOCK_TARGET => "LOCK TARGET",
            Self::NETWORK_TALK => "NETWORK TALK",
            Self::PED_1RST_PERSON_LOOK_UP => "FIRST PERSON LOOK UP",
            Self::PED_1RST_PERSON_LOOK_DOWN => "FIRST PERSON LOOK DOWN",
            Self::UNUSED_36 => "",
            Self::TOGGLE_DPAD => "TOGGLE DPAD",
            Self::SWITCH_DEBUG_CAM_ON => "DEBUG CAMERA",
            Self::TAKE_SCREEN_SHOT => "SCREENSHOT",
            Self::SHOW_MOUSE_POINTER_TOGGLE => "SHOW MOUSE POINTER",
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum eControllerType {
    KEYBOARD,
    OPTIONAL_EXTRA,
    MOUSE,
    JOYSTICK,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ControllerBinding {
    pub key: KeyCode,
    /// re3 `m_ContSetOrder`, one-based within an action.
    pub order: u8,
}

#[derive(Debug, Clone, Eq, PartialEq, Resource)]
pub struct CControllerConfigManager {
    pub bindings: BTreeMap<(e_ControllerAction, eControllerType), ControllerBinding>,
    pub mouse_bindings: BTreeMap<e_ControllerAction, MouseButton>,
}

impl Default for CControllerConfigManager {
    fn default() -> Self {
        let mut config = Self {
            bindings: BTreeMap::new(),
            mouse_bindings: BTreeMap::new(),
        };
        config.InitDefaultControlConfiguration();
        config
    }
}

impl CControllerConfigManager {
    #[allow(non_snake_case)]
    pub fn MakeControllerActionsBlank(&mut self) {
        self.bindings.clear();
        self.mouse_bindings.clear();
    }

    #[allow(non_snake_case)]
    pub fn InitDefaultControlConfiguration(&mut self) {
        self.MakeControllerActionsBlank();
        use e_ControllerAction::*;
        use eControllerType::{KEYBOARD, OPTIONAL_EXTRA};
        for (action, primary, alternate) in [
            (VEHICLE_LOOKLEFT, KeyCode::End, KeyCode::KeyQ),
            (VEHICLE_LOOKRIGHT, KeyCode::PageDown, KeyCode::KeyE),
            (VEHICLE_HORN, KeyCode::ShiftRight, KeyCode::ShiftLeft),
            (VEHICLE_HANDBRAKE, KeyCode::ControlRight, KeyCode::Space),
            (VEHICLE_ENTER_EXIT, KeyCode::Enter, KeyCode::KeyF),
            (VEHICLE_ACCELERATE, KeyCode::ArrowUp, KeyCode::KeyW),
            (VEHICLE_CHANGE_RADIO_STATION, KeyCode::Insert, KeyCode::KeyR),
            (VEHICLE_BRAKE, KeyCode::ArrowDown, KeyCode::KeyS),
            (TOGGLE_SUBMISSIONS, KeyCode::NumpadAdd, KeyCode::CapsLock),
            (GO_LEFT, KeyCode::ArrowLeft, KeyCode::KeyA),
            (GO_RIGHT, KeyCode::ArrowRight, KeyCode::KeyD),
            (GO_FORWARD, KeyCode::ArrowUp, KeyCode::KeyW),
            (GO_BACK, KeyCode::ArrowDown, KeyCode::KeyS),
            (PED_LOOKBEHIND, KeyCode::End, KeyCode::CapsLock),
            (PED_FIREWEAPON, KeyCode::Insert, KeyCode::ControlLeft),
            (PED_CYCLE_WEAPON_LEFT, KeyCode::Delete, KeyCode::Delete),
            (
                PED_CYCLE_WEAPON_RIGHT,
                KeyCode::NumpadEnter,
                KeyCode::NumpadEnter,
            ),
            (PED_LOCK_TARGET, KeyCode::Delete, KeyCode::Delete),
            (PED_JUMPING, KeyCode::ControlRight, KeyCode::Space),
            (PED_SPRINT, KeyCode::ShiftRight, KeyCode::ShiftLeft),
            (
                PED_CYCLE_TARGET_LEFT,
                KeyCode::BracketLeft,
                KeyCode::BracketLeft,
            ),
            (
                PED_CYCLE_TARGET_RIGHT,
                KeyCode::BracketRight,
                KeyCode::BracketRight,
            ),
            (
                PED_CENTER_CAMERA_BEHIND_PLAYER,
                KeyCode::Backslash,
                KeyCode::Backslash,
            ),
            (PED_SNIPER_ZOOM_IN, KeyCode::PageUp, KeyCode::KeyZ),
            (PED_SNIPER_ZOOM_OUT, KeyCode::PageDown, KeyCode::KeyX),
            (
                PED_1RST_PERSON_LOOK_LEFT,
                KeyCode::Numpad4,
                KeyCode::Numpad4,
            ),
            (
                PED_1RST_PERSON_LOOK_RIGHT,
                KeyCode::Numpad6,
                KeyCode::Numpad6,
            ),
            (PED_1RST_PERSON_LOOK_UP, KeyCode::Numpad8, KeyCode::Numpad8),
            (
                PED_1RST_PERSON_LOOK_DOWN,
                KeyCode::Numpad5,
                KeyCode::Numpad5,
            ),
            (VEHICLE_TURRETLEFT, KeyCode::Numpad4, KeyCode::Numpad4),
            (VEHICLE_TURRETRIGHT, KeyCode::Numpad5, KeyCode::Numpad5),
            (VEHICLE_TURRETUP, KeyCode::Numpad9, KeyCode::Numpad9),
            (VEHICLE_TURRETDOWN, KeyCode::Numpad6, KeyCode::Numpad6),
            (
                CAMERA_CHANGE_VIEW_ALL_SITUATIONS,
                KeyCode::Home,
                KeyCode::KeyC,
            ),
        ] {
            self.SetControllerKeyAssociatedWithAction(action, primary, KEYBOARD);
            self.SetControllerKeyAssociatedWithAction(action, alternate, OPTIONAL_EXTRA);
        }
        self.mouse_bindings
            .insert(PED_FIREWEAPON, MouseButton::Left);
        self.mouse_bindings
            .insert(PED_LOCK_TARGET, MouseButton::Right);
        self.mouse_bindings
            .insert(VEHICLE_HANDBRAKE, MouseButton::Right);
        self.mouse_bindings
            .insert(PED_LOOKBEHIND, MouseButton::Middle);
        self.mouse_bindings
            .insert(PED_CYCLE_WEAPON_LEFT, MouseButton::Back);
        self.mouse_bindings
            .insert(PED_CYCLE_WEAPON_RIGHT, MouseButton::Forward);
    }

    #[allow(non_snake_case)]
    pub fn SetControllerKeyAssociatedWithAction(
        &mut self,
        action: e_ControllerAction,
        key: KeyCode,
        controller: eControllerType,
    ) {
        let order = self
            .bindings
            .keys()
            .filter(|(bound_action, _)| *bound_action == action)
            .count()
            .saturating_add(1) as u8;
        self.bindings
            .insert((action, controller), ControllerBinding { key, order });
    }

    #[allow(non_snake_case)]
    pub fn ClearSettingsAssociatedWithAction(
        &mut self,
        action: e_ControllerAction,
        controller: eControllerType,
    ) {
        self.bindings.remove(&(action, controller));
    }

    /// Frontend `CheckCodesForControls`: choose primary/alternate slot and
    /// remove an identical initiator from actions active in the same context.
    pub fn assign_frontend_keyboard_binding(&mut self, action: e_ControllerAction, key: KeyCode) {
        use eControllerType::{KEYBOARD, OPTIONAL_EXTRA};
        let destination = match self.key_for(action, KEYBOARD) {
            Some(primary) if primary != key => OPTIONAL_EXTRA,
            _ => KEYBOARD,
        };
        let context = binding_context(action);
        for other in e_ControllerAction::KEYBOARD_MENU_ACTIONS {
            if other == action || binding_context(other) & context == 0 {
                continue;
            }
            for kind in [KEYBOARD, OPTIONAL_EXTRA] {
                if self.key_for(other, kind) == Some(key) {
                    self.ClearSettingsAssociatedWithAction(other, kind);
                }
            }
        }
        self.ClearSettingsAssociatedWithAction(action, destination);
        self.SetControllerKeyAssociatedWithAction(action, key, destination);
    }

    pub fn clear_frontend_keyboard_binding(&mut self, action: e_ControllerAction) {
        self.ClearSettingsAssociatedWithAction(action, eControllerType::KEYBOARD);
        self.ClearSettingsAssociatedWithAction(action, eControllerType::OPTIONAL_EXTRA);
    }

    pub fn assign_frontend_mouse_binding(
        &mut self,
        action: e_ControllerAction,
        button: MouseButton,
    ) {
        let context = binding_context(action);
        self.mouse_bindings.retain(|other, bound| {
            *bound != button || *other == action || binding_context(*other) & context == 0
        });
        self.mouse_bindings.insert(action, button);
    }

    pub fn key_for(
        &self,
        action: e_ControllerAction,
        controller: eControllerType,
    ) -> Option<KeyCode> {
        self.bindings
            .get(&(action, controller))
            .map(|binding| binding.key)
    }

    pub fn binding_text(&self, action: e_ControllerAction) -> String {
        use eControllerType::{KEYBOARD, OPTIONAL_EXTRA};
        let mut values = [KEYBOARD, OPTIONAL_EXTRA]
            .into_iter()
            .filter_map(|kind| self.key_for(action, kind))
            .map(key_name)
            .collect::<Vec<_>>();
        if let Some(button) = self.mouse_bindings.get(&action) {
            values.push(mouse_button_name(*button));
        }
        values.join(" or ")
    }

    pub fn action_pressed(
        &self,
        action: e_ControllerAction,
        keys: &bevy::input::ButtonInput<KeyCode>,
    ) -> bool {
        use eControllerType::{KEYBOARD, OPTIONAL_EXTRA};
        [KEYBOARD, OPTIONAL_EXTRA]
            .into_iter()
            .filter_map(|kind| self.key_for(action, kind))
            .any(|key| keys.pressed(key))
    }

    pub fn mouse_action_pressed(
        &self,
        action: e_ControllerAction,
        buttons: &bevy::input::ButtonInput<MouseButton>,
    ) -> bool {
        self.mouse_bindings
            .get(&action)
            .is_some_and(|button| buttons.pressed(*button))
    }
}

fn mouse_button_name(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "LMB".to_owned(),
        MouseButton::Right => "RMB".to_owned(),
        MouseButton::Middle => "MMB".to_owned(),
        MouseButton::Back => "MOUSE 4".to_owned(),
        MouseButton::Forward => "MOUSE 5".to_owned(),
        MouseButton::Other(index) => format!("MOUSE {index}"),
    }
}

fn binding_context(action: e_ControllerAction) -> u8 {
    use e_ControllerAction::*;
    match action {
        CAMERA_CHANGE_VIEW_ALL_SITUATIONS | PED_FIREWEAPON | GO_LEFT | GO_RIGHT => 0b11,
        VEHICLE_ENTER_EXIT => 0b11,
        VEHICLE_ACCELERATE
        | VEHICLE_BRAKE
        | VEHICLE_CHANGE_RADIO_STATION
        | VEHICLE_HORN
        | TOGGLE_SUBMISSIONS
        | VEHICLE_HANDBRAKE
        | VEHICLE_LOOKLEFT
        | VEHICLE_LOOKRIGHT
        | VEHICLE_LOOKBEHIND
        | VEHICLE_TURRETLEFT
        | VEHICLE_TURRETRIGHT
        | VEHICLE_TURRETUP
        | VEHICLE_TURRETDOWN => 0b10,
        _ => 0b01,
    }
}

pub fn key_name(key: KeyCode) -> String {
    match key {
        KeyCode::KeyW => "W".into(),
        KeyCode::KeyA => "A".into(),
        KeyCode::KeyS => "S".into(),
        KeyCode::KeyD => "D".into(),
        KeyCode::KeyQ => "Q".into(),
        KeyCode::KeyE => "E".into(),
        KeyCode::KeyF => "F".into(),
        KeyCode::KeyR => "R".into(),
        KeyCode::KeyZ => "Z".into(),
        KeyCode::KeyX => "X".into(),
        KeyCode::ArrowUp => "UP".into(),
        KeyCode::ArrowDown => "DOWN".into(),
        KeyCode::ArrowLeft => "LEFT".into(),
        KeyCode::ArrowRight => "RIGHT".into(),
        KeyCode::ControlLeft | KeyCode::ControlRight => "CTRL".into(),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "SHIFT".into(),
        KeyCode::Space => "SPACE".into(),
        KeyCode::Enter | KeyCode::NumpadEnter => "ENTER".into(),
        KeyCode::PageUp => "PGUP".into(),
        KeyCode::PageDown => "PGDN".into(),
        KeyCode::BracketLeft => "[".into(),
        KeyCode::BracketRight => "]".into(),
        KeyCode::Backslash => "#".into(),
        KeyCode::Home => "HOME".into(),
        KeyCode::End => "END".into(),
        KeyCode::Insert => "INS".into(),
        KeyCode::Delete => "DEL".into(),
        KeyCode::CapsLock => "CAPSLOCK".into(),
        KeyCode::NumpadAdd => "+".into(),
        _ => format!("{key:?}").to_uppercase(),
    }
}
