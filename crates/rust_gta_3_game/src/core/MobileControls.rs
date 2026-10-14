//! Android touch devices feed the existing semantic input / CPad boundary.
use std::collections::{BTreeMap, BTreeSet};

use super::{
    EnhancedInput::{GtaGameplayInput, collect_gameplay_input},
    FrontendFlow::update_frontend,
    Pad::update_pads_from_bevy_input,
};
use crate::peds::{PlayerPed::CWorldPlayerState, VehicleRelations::InVehicle};
use bevy::{
    audio::AudioSinkPlayback,
    input::touch::TouchPhase,
    prelude::*,
    window::{AppLifecycle, PrimaryWindow},
};
use rust_gta_3_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

pub struct MobileControlsPlugin;

impl Plugin for MobileControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MobileTouches>()
            .init_resource::<MobileMenuInput>()
            .add_systems(
                Update,
                (
                    read_touches
                        .before(collect_gameplay_input)
                        .before(update_frontend),
                    merge_gameplay
                        .after(collect_gameplay_input)
                        .before(update_pads_from_bevy_input)
                        .in_set(GtaProcessSet::Pad)
                        .run_if(in_state(GtaAppState::InGame)),
                    update_overlay.after(read_touches),
                    suspend_audio.before(read_touches),
                ),
            );
    }
}

#[derive(Default, Resource)]
pub(super) struct MobileMenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub back: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Move,
    Look,
    Fire,
    Aim,
    Jump,
    Sprint,
    Enter,
    Brake,
    Gas,
    Handbrake,
    Horn,
    Previous,
    Next,
    Camera,
    Radio,
    Mission,
    Behind,
    Pause,
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

#[derive(Clone, Copy)]
struct Contact {
    control: Control,
    origin: Vec2,
    position: Vec2,
}

#[derive(Default, Resource)]
struct MobileTouches {
    contacts: BTreeMap<u64, Contact>,
    blocked: BTreeSet<u64>,
    state: Option<GtaAppState>,
    vehicle: bool,
    output: GtaGameplayInput,
    background: bool,
}

#[derive(Component)]
struct MobileRoot;
#[derive(Component)]
struct MobileButton(Control);

#[derive(Component)]
struct MobileStick {
    knob: bool,
}

// Positions are relative to the actual window; button size follows its shorter
// dimension. Rendering and hit testing use the same rectangles.
fn buttons(state: GtaAppState, vehicle: bool, size: Vec2) -> Vec<(Control, &'static str, Rect)> {
    let mut specs = Vec::new();
    if matches!(state, GtaAppState::Frontend | GtaAppState::PauseMenu) {
        for (index, (action, label)) in [
            (Control::Up, "UP"),
            (Control::Down, "DOWN"),
            (Control::Left, "-"),
            (Control::Right, "+"),
            (Control::Select, "OK"),
            (Control::Back, "BACK"),
        ]
        .into_iter()
        .enumerate()
        {
            specs.push((action, label, Vec2::new(0.27 + index as f32 * 0.092, 0.91)));
        }
    } else if state == GtaAppState::InGame {
        specs.extend([
            (Control::Pause, "PAUSE", Vec2::new(0.93, 0.12)),
            (Control::Camera, "CAM", Vec2::new(0.69, 0.12)),
            (Control::Behind, "BACK", Vec2::new(0.81, 0.12)),
            (Control::Enter, "CAR", Vec2::new(0.69, 0.79)),
            (Control::Fire, "FIRE", Vec2::new(0.93, 0.77)),
        ]);
        if vehicle {
            specs.extend([
                (Control::Gas, "GAS", Vec2::new(0.81, 0.62)),
                (Control::Brake, "BRAKE", Vec2::new(0.81, 0.85)),
                (Control::Handbrake, "H.BRAKE", Vec2::new(0.93, 0.53)),
                (Control::Horn, "HORN", Vec2::new(0.57, 0.79)),
                (Control::Radio, "RADIO", Vec2::new(0.57, 0.12)),
                (Control::Mission, "JOB", Vec2::new(0.45, 0.12)),
            ]);
        } else {
            specs.extend([
                (Control::Sprint, "RUN", Vec2::new(0.81, 0.62)),
                (Control::Jump, "JUMP", Vec2::new(0.81, 0.85)),
                (Control::Aim, "AIM", Vec2::new(0.93, 0.53)),
                (Control::Previous, "W-", Vec2::new(0.45, 0.79)),
                (Control::Next, "W+", Vec2::new(0.57, 0.79)),
            ]);
        }
    }
    let diameter = (size.y * 0.115).clamp(36.0, 82.0).min(size.y * 0.15);
    specs
        .into_iter()
        .map(|(control, label, center)| {
            (
                control,
                label,
                Rect::from_center_size(center * size, Vec2::splat(diameter)),
            )
        })
        .collect()
}

fn set_control(input: &mut GtaGameplayInput, control: Control) {
    match control {
        Control::Fire => input.fire = true,
        Control::Aim => input.target = true,
        Control::Jump => {
            input.jump = true;
            input.sniper_zoom_in = true;
        }
        Control::Sprint => {
            input.sprint = true;
            input.sniper_zoom_out = true;
        }
        Control::Enter => input.enter_exit = true,
        Control::Gas => input.vehicle_accelerate = true,
        Control::Brake => input.vehicle_brake = true,
        Control::Handbrake => input.vehicle_handbrake = true,
        Control::Horn => input.horn = true,
        Control::Previous => {
            input.cycle_weapon_left = true;
            input.cycle_target_left = true;
        }
        Control::Next => {
            input.cycle_weapon_right = true;
            input.cycle_target_right = true;
        }
        Control::Camera => input.change_camera = true,
        Control::Radio => input.change_radio = true,
        Control::Mission => input.toggle_submissions = true,
        Control::Behind => {
            input.look_behind = true;
            input.vehicle_look_behind = true;
        }
        Control::Pause => input.pause = true,
        _ => {}
    }
}

fn read_touches(
    mut events: MessageReader<TouchInput>,
    mut lifecycle: MessageReader<AppLifecycle>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Option<Res<CWorldPlayerState>>,
    relations: Query<&InVehicle>,
    state: Res<State<GtaAppState>>,
    mut touches: ResMut<MobileTouches>,
    mut menu: ResMut<MobileMenuInput>,
    mut next: ResMut<NextState<GtaAppState>>,
) {
    *menu = MobileMenuInput::default();
    touches.output = GtaGameplayInput::default();
    for event in lifecycle.read() {
        match event {
            AppLifecycle::Suspended => {
                touches.background = true;
                if *state.get() == GtaAppState::InGame {
                    next.set(GtaAppState::PauseMenu);
                }
            }
            AppLifecycle::Running => touches.background = false,
            _ => {}
        }
    }
    let Ok(window) = windows.single() else {
        events.clear();
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let vehicle = players
        .as_ref()
        .and_then(|players| {
            players
                .entities
                .get(players.player_in_focus)
                .copied()
                .flatten()
        })
        .is_some_and(|entity| relations.contains(entity));
    if touches.state != Some(*state.get())
        || touches.vehicle != vehicle
        || !window.focused
        || touches.background
    {
        let ids: Vec<_> = touches.contacts.keys().copied().collect();
        touches.blocked.extend(ids);
        touches.contacts.clear();
        touches.state = Some(*state.get());
        touches.vehicle = vehicle;
    }
    let layout = buttons(*state.get(), vehicle, size);
    for event in events.read() {
        if matches!(event.phase, TouchPhase::Ended | TouchPhase::Canceled) {
            touches.contacts.remove(&event.id);
            touches.blocked.remove(&event.id);
            continue;
        }
        if !window.focused || touches.background || touches.blocked.contains(&event.id) {
            continue;
        }
        match event.phase {
            TouchPhase::Started => {
                let button = layout
                    .iter()
                    .find(|(_, _, rect)| rect.contains(event.position));
                let control = button.map(|item| item.0).or_else(|| {
                    if *state.get() != GtaAppState::InGame {
                        return None;
                    }
                    if event.position.x < size.x * 0.36 && event.position.y > size.y * 0.40 {
                        Some(Control::Move)
                    } else if event.position.x > size.x * 0.40 {
                        Some(Control::Look)
                    } else {
                        None
                    }
                });
                if let Some(control) = control {
                    if matches!(control, Control::Move | Control::Look)
                        && touches.contacts.values().any(|c| c.control == control)
                    {
                        continue;
                    }
                    // Capture by finger identity: moving over another button
                    // cannot accidentally fire, and each release stops its owner.
                    touches.contacts.insert(
                        event.id,
                        Contact {
                            control,
                            origin: event.position,
                            position: event.position,
                        },
                    );
                    set_control(&mut touches.output, control);
                    match control {
                        Control::Up => menu.up = true,
                        Control::Down => menu.down = true,
                        Control::Left => menu.left = true,
                        Control::Right => menu.right = true,
                        Control::Select => menu.select = true,
                        Control::Back => menu.back = true,
                        _ => {}
                    }
                }
            }
            TouchPhase::Moved => {
                if let Some(contact) = touches.contacts.get_mut(&event.id) {
                    let delta = event.position - contact.position;
                    let look = contact.control == Control::Look;
                    contact.position = event.position;
                    if look && !vehicle {
                        touches.output.mouse_look += delta * (720.0 / size.y.max(1.0));
                    }
                }
            }
            _ => {}
        }
    }
    let contacts: Vec<_> = touches.contacts.values().copied().collect();
    for contact in contacts {
        if contact.control == Control::Move {
            let delta = (contact.position - contact.origin) / (size.y * 0.14).max(1.0);
            touches.output.movement = Vec2::new(delta.x, -delta.y).clamp_length_max(1.0);
        } else if contact.control == Control::Look && vehicle {
            // GTA III's car camera has native side-look views, rather than
            // the on-foot mouse orbit. Feed its existing CPad getters.
            let horizontal = contact.position.x - contact.origin.x;
            touches.output.look_left |= horizontal < -size.x * 0.025;
            touches.output.look_right |= horizontal > size.x * 0.025;
        } else {
            set_control(&mut touches.output, contact.control);
        }
    }
}

fn merge_gameplay(touches: Res<MobileTouches>, mut input: ResMut<GtaGameplayInput>) {
    let touch = touches.output;
    input.movement = (input.movement + touch.movement).clamp_length_max(1.0);
    input.mouse_look += touch.mouse_look;
    macro_rules! merge { ($($field:ident),*) => { $(input.$field |= touch.$field;)* }; }
    merge!(
        fire,
        target,
        jump,
        sprint,
        enter_exit,
        vehicle_accelerate,
        vehicle_brake,
        vehicle_handbrake,
        horn,
        cycle_weapon_left,
        cycle_weapon_right,
        cycle_target_left,
        cycle_target_right,
        sniper_zoom_in,
        sniper_zoom_out,
        change_camera,
        change_radio,
        toggle_submissions,
        look_behind,
        look_left,
        look_right,
        vehicle_look_behind,
        pause
    );
}

fn update_overlay(
    mut commands: Commands,
    touches: Res<MobileTouches>,
    state: Res<State<GtaAppState>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    roots: Query<Entity, With<MobileRoot>>,
    mut colors: Query<(&MobileButton, &mut BackgroundColor)>,
    mut sticks: Query<(&MobileStick, &mut Node)>,
    mut previous: Local<Option<(GtaAppState, bool, Vec2)>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let key = (*state.get(), touches.vehicle, size);
    if *previous != Some(key) {
        *previous = Some(key);
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        let layout = buttons(*state.get(), touches.vehicle, size);
        if layout.is_empty() {
            return;
        }
        commands
            .spawn((
                MobileRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                GlobalZIndex(30_000),
                Pickable::IGNORE,
            ))
            .with_children(|parent| {
                for (control, label, rect) in layout {
                    parent
                        .spawn((
                            MobileButton(control),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(rect.min.x),
                                top: px(rect.min.y),
                                width: px(rect.width()),
                                height: px(rect.height()),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(px(14)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.60)),
                        ))
                        .with_child((
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px((size.y * 0.029).clamp(11.0, 20.0)),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Pickable::IGNORE,
                        ));
                }
                if *state.get() == GtaAppState::InGame {
                    for knob in [false, true] {
                        parent.spawn((
                            MobileStick { knob },
                            Node {
                                position_type: PositionType::Absolute,
                                border_radius: BorderRadius::all(percent(50)),
                                ..default()
                            },
                            BackgroundColor(if knob {
                                Color::srgba(0.8, 0.9, 1.0, 0.4)
                            } else {
                                Color::srgba(0.03, 0.03, 0.04, 0.35)
                            }),
                            Pickable::IGNORE,
                        ));
                    }
                    parent
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: percent(5),
                                top: percent(72),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_child((
                            Text::new("MOVE"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.65)),
                            Pickable::IGNORE,
                        ));
                }
            });
    }
    for (button, mut color) in &mut colors {
        let held = touches
            .contacts
            .values()
            .any(|contact| contact.control == button.0);
        color.0 = if held {
            Color::srgba(0.15, 0.55, 0.72, 0.85)
        } else {
            Color::srgba(0.03, 0.03, 0.04, 0.60)
        };
    }
    let contact = touches
        .contacts
        .values()
        .find(|contact| contact.control == Control::Move);
    let radius = size.y * 0.14;
    let center = contact.map_or(Vec2::new(size.x * 0.16, size.y * 0.74), |contact| {
        contact.origin
    });
    let offset = contact.map_or(Vec2::ZERO, |contact| {
        (contact.position - contact.origin).clamp_length_max(radius)
    });
    for (stick, mut node) in &mut sticks {
        let half_size = if stick.knob { radius * 0.4 } else { radius };
        let position = center + if stick.knob { offset } else { Vec2::ZERO };
        node.left = px(position.x - half_size);
        node.top = px(position.y - half_size);
        node.width = px(half_size * 2.0);
        node.height = px(half_size * 2.0);
    }
}

fn suspend_audio(
    mut lifecycle: MessageReader<AppLifecycle>,
    sinks: Query<(Entity, &AudioSink)>,
    spatial: Query<(Entity, &SpatialAudioSink)>,
    mut paused: Local<Vec<Entity>>,
) {
    for event in lifecycle.read() {
        match event {
            AppLifecycle::Suspended => {
                paused.clear();
                for (entity, sink) in &sinks {
                    if !sink.is_paused() {
                        sink.pause();
                        paused.push(entity);
                    }
                }
                for (entity, sink) in &spatial {
                    if !sink.is_paused() {
                        sink.pause();
                        paused.push(entity);
                    }
                }
            }
            AppLifecycle::Running => {
                for entity in paused.drain(..) {
                    if let Ok((_, sink)) = sinks.get(entity) {
                        sink.play();
                    }
                    if let Ok((_, sink)) = spatial.get(entity) {
                        sink.play();
                    }
                }
            }
            _ => {}
        }
    }
}
