//! Bevy Enhanced Input adapter for re3's `CControllerState` boundary.
//!
//! Input devices produce semantic actions here. `CPad` remains the gameplay /
//! SCM-facing state machine, matching `CControllerConfigManager -> CPad::Update`
//! in the C++ reference while allowing keyboard, mouse and gamepad bindings to
//! share one Bevy-native context.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use rust_gta_3_core::core::GameState::GtaAppState;

use super::ControllerConfig::{CControllerConfigManager, e_ControllerAction, eControllerType};

#[derive(Component)]
pub struct GtaGameplayInputContext;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct MoveAction;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct DPadAction;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct MouseLookAction;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct StickLookAction;

macro_rules! bool_action {
    ($name:ident) => {
        #[derive(InputAction)]
        #[action_output(bool)]
        pub struct $name;
    };
}

bool_action!(SprintAction);
bool_action!(JumpAction);
bool_action!(FireAction);
bool_action!(TargetAction);
bool_action!(EnterExitAction);
bool_action!(VehicleAccelerateAction);
bool_action!(VehicleBrakeAction);
bool_action!(VehicleHandbrakeAction);
bool_action!(CycleWeaponLeftAction);
bool_action!(CycleWeaponRightAction);
bool_action!(PauseAction);
bool_action!(HornAction);
bool_action!(ChangeCameraAction);
bool_action!(ChangeRadioAction);
bool_action!(ToggleSubmissionsAction);
bool_action!(LookLeftAction);
bool_action!(LookRightAction);
bool_action!(LookBehindAction);
bool_action!(VehicleLookBehindAction);
bool_action!(CenterCameraAction);
bool_action!(SniperZoomInAction);
bool_action!(SniperZoomOutAction);
bool_action!(CycleTargetLeftAction);
bool_action!(CycleTargetRightAction);

/// Device-neutral action snapshot consumed once per GTA pad update.
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct GtaGameplayInput {
    pub movement: Vec2,
    pub dpad: Vec2,
    pub mouse_look: Vec2,
    pub stick_look: Vec2,
    pub sprint: bool,
    pub jump: bool,
    pub fire: bool,
    pub target: bool,
    pub enter_exit: bool,
    pub vehicle_accelerate: bool,
    pub vehicle_brake: bool,
    pub vehicle_handbrake: bool,
    pub cycle_weapon_left: bool,
    pub cycle_weapon_right: bool,
    pub pause: bool,
    pub horn: bool,
    pub change_camera: bool,
    pub change_radio: bool,
    pub toggle_submissions: bool,
    pub look_left: bool,
    pub look_right: bool,
    pub look_behind: bool,
    pub vehicle_look_behind: bool,
    pub center_camera: bool,
    pub sniper_zoom_in: bool,
    pub sniper_zoom_out: bool,
    pub cycle_target_left: bool,
    pub cycle_target_right: bool,
}

pub struct GtaEnhancedInputPlugin;

impl Plugin for GtaEnhancedInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<GtaGameplayInputContext>()
            .init_resource::<GtaGameplayInput>()
            .add_systems(OnEnter(GtaAppState::InGame), spawn_gameplay_input_context)
            .add_systems(OnExit(GtaAppState::InGame), despawn_gameplay_input_context);
        #[cfg(target_os = "android")]
        app.add_plugins(super::MobileControls::MobileControlsPlugin);
    }
}

fn key_binding(
    controller: &CControllerConfigManager,
    action: e_ControllerAction,
    kind: eControllerType,
) -> Binding {
    controller
        .key_for(action, kind)
        .map(Binding::from)
        .unwrap_or(Binding::None)
}

fn mouse_binding(controller: &CControllerConfigManager, action: e_ControllerAction) -> Binding {
    controller
        .mouse_bindings
        .get(&action)
        .copied()
        .map(Binding::from)
        .unwrap_or(Binding::None)
}

fn spawn_gameplay_input_context(
    mut commands: Commands,
    controller: Res<CControllerConfigManager>,
    existing: Query<Entity, With<GtaGameplayInputContext>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }

    use e_ControllerAction::*;
    use eControllerType::{KEYBOARD, OPTIONAL_EXTRA};

    commands.spawn((
        Name::new("re3 CPad Enhanced Input Context"),
        GtaGameplayInputContext,
        actions!(GtaGameplayInputContext[
            (
                Action::<MoveAction>::new(),
                Clamp::splat(-1.0, 1.0),
                // Only the physical analogue stick belongs in LeftStickX/Y.
                Bindings::spawn(Axial::left_stick()),
            ),
            (
                Action::<DPadAction>::new(),
                // Keyboard GO_* bindings are written directly into CPad in
                // Pad.rs, matching re3 PCTempKeyState. This action is only the
                // physical gamepad D-pad; mixing keyboard Cardinal bindings
                // here caused A/S/D to collapse onto the forward component.
                Bindings::spawn(Cardinal::dpad()),
            ),
            (
                Action::<MouseLookAction>::new(),
                bindings![Binding::mouse_motion()],
            ),
            (
                Action::<StickLookAction>::new(),
                Bindings::spawn(Axial::right_stick()),
            ),
            (
                Action::<SprintAction>::new(),
                bindings![
                    key_binding(&controller, PED_SPRINT, KEYBOARD),
                    key_binding(&controller, PED_SPRINT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_SPRINT),
                    GamepadButton::South,
                ],
            ),
            (
                Action::<JumpAction>::new(),
                bindings![
                    key_binding(&controller, PED_JUMPING, KEYBOARD),
                    key_binding(&controller, PED_JUMPING, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_JUMPING),
                    GamepadButton::West,
                ],
            ),
            (
                Action::<FireAction>::new(),
                bindings![
                    key_binding(&controller, PED_FIREWEAPON, KEYBOARD),
                    key_binding(&controller, PED_FIREWEAPON, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_FIREWEAPON),
                    GamepadButton::East,
                    GamepadButton::RightTrigger2,
                ],
            ),
            (
                Action::<TargetAction>::new(),
                bindings![
                    key_binding(&controller, PED_LOCK_TARGET, KEYBOARD),
                    key_binding(&controller, PED_LOCK_TARGET, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_LOCK_TARGET),
                    GamepadButton::LeftTrigger2,
                ],
            ),
            (
                Action::<EnterExitAction>::new(),
                bindings![
                    key_binding(&controller, VEHICLE_ENTER_EXIT, KEYBOARD),
                    key_binding(&controller, VEHICLE_ENTER_EXIT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_ENTER_EXIT),
                    GamepadButton::North,
                ],
            ),
            (
                Action::<VehicleAccelerateAction>::new(),
                bindings![
                    key_binding(&controller, VEHICLE_ACCELERATE, KEYBOARD),
                    key_binding(&controller, VEHICLE_ACCELERATE, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_ACCELERATE),
                    GamepadButton::South,
                    GamepadButton::RightTrigger2,
                ],
            ),
            (
                Action::<VehicleBrakeAction>::new(),
                bindings![
                    key_binding(&controller, VEHICLE_BRAKE, KEYBOARD),
                    key_binding(&controller, VEHICLE_BRAKE, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_BRAKE),
                    GamepadButton::West,
                    GamepadButton::LeftTrigger2,
                ],
            ),
            (
                Action::<VehicleHandbrakeAction>::new(),
                bindings![
                    key_binding(&controller, VEHICLE_HANDBRAKE, KEYBOARD),
                    key_binding(&controller, VEHICLE_HANDBRAKE, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_HANDBRAKE),
                    GamepadButton::RightTrigger,
                ],
            ),
            (
                Action::<CycleWeaponLeftAction>::new(),
                bindings![
                    key_binding(&controller, PED_CYCLE_WEAPON_LEFT, KEYBOARD),
                    key_binding(&controller, PED_CYCLE_WEAPON_LEFT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_CYCLE_WEAPON_LEFT),
                    GamepadButton::DPadLeft,
                    GamepadButton::LeftTrigger,
                ],
            ),
            (
                Action::<CycleWeaponRightAction>::new(),
                bindings![
                    key_binding(&controller, PED_CYCLE_WEAPON_RIGHT, KEYBOARD),
                    key_binding(&controller, PED_CYCLE_WEAPON_RIGHT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_CYCLE_WEAPON_RIGHT),
                    GamepadButton::DPadRight,
                    GamepadButton::RightTrigger,
                ],
            ),
            (Action::<HornAction>::new(), bindings![
                    key_binding(&controller, VEHICLE_HORN, KEYBOARD),
                    key_binding(&controller, VEHICLE_HORN, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_HORN),
                    GamepadButton::LeftThumb,
            ]),
            (Action::<ChangeCameraAction>::new(), bindings![
                    key_binding(&controller, CAMERA_CHANGE_VIEW_ALL_SITUATIONS, KEYBOARD),
                    key_binding(&controller, CAMERA_CHANGE_VIEW_ALL_SITUATIONS, OPTIONAL_EXTRA),
                    mouse_binding(&controller, CAMERA_CHANGE_VIEW_ALL_SITUATIONS),
                    GamepadButton::Select,
            ]),
            (Action::<ChangeRadioAction>::new(), bindings![
                    key_binding(&controller, VEHICLE_CHANGE_RADIO_STATION, KEYBOARD),
                    key_binding(&controller, VEHICLE_CHANGE_RADIO_STATION, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_CHANGE_RADIO_STATION),
                    GamepadButton::LeftTrigger,
            ]),
            (Action::<ToggleSubmissionsAction>::new(), bindings![
                    key_binding(&controller, TOGGLE_SUBMISSIONS, KEYBOARD),
                    key_binding(&controller, TOGGLE_SUBMISSIONS, OPTIONAL_EXTRA),
                    mouse_binding(&controller, TOGGLE_SUBMISSIONS),
                    GamepadButton::RightThumb,
            ]),
            (Action::<LookLeftAction>::new(), bindings![
                    key_binding(&controller, VEHICLE_LOOKLEFT, KEYBOARD),
                    key_binding(&controller, VEHICLE_LOOKLEFT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_LOOKLEFT),
                    GamepadButton::LeftTrigger2,
            ]),
            (Action::<LookRightAction>::new(), bindings![
                    key_binding(&controller, VEHICLE_LOOKRIGHT, KEYBOARD),
                    key_binding(&controller, VEHICLE_LOOKRIGHT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_LOOKRIGHT),
                    GamepadButton::RightTrigger2,
            ]),
            (Action::<LookBehindAction>::new(), bindings![
                    key_binding(&controller, PED_LOOKBEHIND, KEYBOARD),
                    key_binding(&controller, PED_LOOKBEHIND, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_LOOKBEHIND),
                    GamepadButton::RightThumb,
            ]),
            (Action::<VehicleLookBehindAction>::new(), bindings![
                    key_binding(&controller, VEHICLE_LOOKBEHIND, KEYBOARD),
                    key_binding(&controller, VEHICLE_LOOKBEHIND, OPTIONAL_EXTRA),
                    mouse_binding(&controller, VEHICLE_LOOKBEHIND),
            ]),
            (Action::<CenterCameraAction>::new(), bindings![
                    key_binding(&controller, PED_CENTER_CAMERA_BEHIND_PLAYER, KEYBOARD),
                    key_binding(&controller, PED_CENTER_CAMERA_BEHIND_PLAYER, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_CENTER_CAMERA_BEHIND_PLAYER),
            ]),
            (Action::<SniperZoomInAction>::new(), bindings![
                    key_binding(&controller, PED_SNIPER_ZOOM_IN, KEYBOARD),
                    key_binding(&controller, PED_SNIPER_ZOOM_IN, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_SNIPER_ZOOM_IN),
            ]),
            (Action::<SniperZoomOutAction>::new(), bindings![
                    key_binding(&controller, PED_SNIPER_ZOOM_OUT, KEYBOARD),
                    key_binding(&controller, PED_SNIPER_ZOOM_OUT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_SNIPER_ZOOM_OUT),
            ]),
            (Action::<CycleTargetLeftAction>::new(), bindings![
                    key_binding(&controller, PED_CYCLE_TARGET_LEFT, KEYBOARD),
                    key_binding(&controller, PED_CYCLE_TARGET_LEFT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_CYCLE_TARGET_LEFT),
            ]),
            (Action::<CycleTargetRightAction>::new(), bindings![
                    key_binding(&controller, PED_CYCLE_TARGET_RIGHT, KEYBOARD),
                    key_binding(&controller, PED_CYCLE_TARGET_RIGHT, OPTIONAL_EXTRA),
                    mouse_binding(&controller, PED_CYCLE_TARGET_RIGHT),
            ]),
            (
                Action::<PauseAction>::new(),
                bindings![KeyCode::Escape, GamepadButton::Start],
            ),
        ]),
    ));
}

fn despawn_gameplay_input_context(
    mut commands: Commands,
    contexts: Query<Entity, With<GtaGameplayInputContext>>,
    mut input: ResMut<GtaGameplayInput>,
) {
    *input = GtaGameplayInput::default();
    for entity in &contexts {
        // `Actions` and `Bindings` are linked-spawn relationships, so this
        // recursively cleans the action graph without leaking input entities.
        commands.entity(entity).despawn();
    }
}

fn action_vec2<A: InputAction<Output = Vec2>>(query: &Query<&Action<A>>) -> Vec2 {
    query
        .iter()
        .next()
        .map(|action| **action)
        .unwrap_or(Vec2::ZERO)
}

fn action_bool<A: InputAction<Output = bool>>(query: &Query<&Action<A>>) -> bool {
    // Use `any` rather than the first match: if more than one action entity is
    // alive for a frame (input context respawn, or a second gameplay context),
    // reading only the first can flip the held value on and off. A flickering
    // bool turns an edge-triggered jump (`JumpJustDown`) into a repeated one.
    query.iter().any(|action| **action)
}

#[derive(SystemParam)]
pub struct AdditionalGameplayActions<'w, 's> {
    pause: Query<'w, 's, &'static Action<PauseAction>>,
    horn: Query<'w, 's, &'static Action<HornAction>>,
    change_camera: Query<'w, 's, &'static Action<ChangeCameraAction>>,
    change_radio: Query<'w, 's, &'static Action<ChangeRadioAction>>,
    toggle_submissions: Query<'w, 's, &'static Action<ToggleSubmissionsAction>>,
    look_left: Query<'w, 's, &'static Action<LookLeftAction>>,
    look_right: Query<'w, 's, &'static Action<LookRightAction>>,
    look_behind: Query<'w, 's, &'static Action<LookBehindAction>>,
    vehicle_look_behind: Query<'w, 's, &'static Action<VehicleLookBehindAction>>,
    center_camera: Query<'w, 's, &'static Action<CenterCameraAction>>,
    sniper_zoom_in: Query<'w, 's, &'static Action<SniperZoomInAction>>,
    sniper_zoom_out: Query<'w, 's, &'static Action<SniperZoomOutAction>>,
    cycle_target_left: Query<'w, 's, &'static Action<CycleTargetLeftAction>>,
    cycle_target_right: Query<'w, 's, &'static Action<CycleTargetRightAction>>,
}

#[allow(clippy::too_many_arguments)]
pub fn collect_gameplay_input(
    movement: Query<&Action<MoveAction>>,
    dpad: Query<&Action<DPadAction>>,
    mouse_look: Query<&Action<MouseLookAction>>,
    stick_look: Query<&Action<StickLookAction>>,
    sprint: Query<&Action<SprintAction>>,
    jump: Query<&Action<JumpAction>>,
    fire: Query<&Action<FireAction>>,
    target: Query<&Action<TargetAction>>,
    enter_exit: Query<&Action<EnterExitAction>>,
    vehicle_accelerate: Query<&Action<VehicleAccelerateAction>>,
    vehicle_brake: Query<&Action<VehicleBrakeAction>>,
    vehicle_handbrake: Query<&Action<VehicleHandbrakeAction>>,
    cycle_left: Query<&Action<CycleWeaponLeftAction>>,
    cycle_right: Query<&Action<CycleWeaponRightAction>>,
    additional: AdditionalGameplayActions,
    mut input: ResMut<GtaGameplayInput>,
) {
    input.movement = action_vec2(&movement).clamp(Vec2::splat(-1.0), Vec2::ONE);
    input.dpad = action_vec2(&dpad).clamp(Vec2::splat(-1.0), Vec2::ONE);
    input.mouse_look = action_vec2(&mouse_look);
    input.stick_look = action_vec2(&stick_look).clamp(Vec2::splat(-1.0), Vec2::ONE);
    input.sprint = action_bool(&sprint);
    input.jump = action_bool(&jump);
    input.fire = action_bool(&fire);
    input.target = action_bool(&target);
    input.enter_exit = action_bool(&enter_exit);
    input.vehicle_accelerate = action_bool(&vehicle_accelerate);
    input.vehicle_brake = action_bool(&vehicle_brake);
    input.vehicle_handbrake = action_bool(&vehicle_handbrake);
    input.cycle_weapon_left = action_bool(&cycle_left);
    input.cycle_weapon_right = action_bool(&cycle_right);
    input.pause = action_bool(&additional.pause);
    input.horn = action_bool(&additional.horn);
    input.change_camera = action_bool(&additional.change_camera);
    input.change_radio = action_bool(&additional.change_radio);
    input.toggle_submissions = action_bool(&additional.toggle_submissions);
    input.look_left = action_bool(&additional.look_left);
    input.look_right = action_bool(&additional.look_right);
    input.look_behind = action_bool(&additional.look_behind);
    input.vehicle_look_behind = action_bool(&additional.vehicle_look_behind);
    input.center_camera = action_bool(&additional.center_camera);
    input.sniper_zoom_in = action_bool(&additional.sniper_zoom_in);
    input.sniper_zoom_out = action_bool(&additional.sniper_zoom_out);
    input.cycle_target_left = action_bool(&additional.cycle_target_left);
    input.cycle_target_right = action_bool(&additional.cycle_target_right);
}
