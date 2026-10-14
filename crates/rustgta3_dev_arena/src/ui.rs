use bevy::{
    camera::visibility::RenderLayers,
    ecs::system::SystemParam,
    input::mouse::MouseWheel,
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::Button,
    window::{CursorOptions, PrimaryWindow},
};
use rustgta3_core::core::Coordinates::CVector;
use rustgta3_game::{
    core::LocalCoop::LocalCoopSession,
    renderer::BevyRender::{GtaWorldTextureCache, sync_model_texture_cache},
    renderer::RendererMaterials::GtaUnlitMaterial,
};

#[cfg(test)]
use rustgta3_game::core::{
    Game::ScriptVehicleEntity,
    Pools::{CPoolEntry, CPoolEntryKind, CPools},
};

use crate::{
    ARENA_PANEL_WIDTH, ARENA_RENDER_LAYER, ArenaAction, ArenaAnimation, ArenaCharacterVisual,
    ArenaCoopPlayerTwo, ArenaGrid, ArenaMouseCursor, ArenaOriginalRuntime, ArenaOwned, ArenaPlayer,
    ArenaRuntimeAssets, ArenaSceneRoot, ArenaSpawned, ArenaStatusText, ArenaTeleportLabel,
    ArenaTeleportList, ArenaTeleportOption, ArenaVehicle, ArenaWorldMap, DeveloperArena,
    catalog::{CHARACTERS, GTA_TELEPORTS, PROPS, VEHICLES},
    scene,
};

const PANEL: Color = Color::srgba(0.035, 0.043, 0.055, 0.985);
const SECTION: Color = Color::srgb(0.42, 0.68, 0.9);
const TEXT: Color = Color::srgb(0.88, 0.91, 0.95);
const MUTED: Color = Color::srgb(0.56, 0.61, 0.68);
const BUTTON: Color = Color::srgb(0.10, 0.13, 0.17);
const BUTTON_HOVER: Color = Color::srgb(0.16, 0.23, 0.3);
const BUTTON_PRESSED: Color = Color::srgb(0.12, 0.42, 0.58);

pub(crate) fn spawn_arena_ui(commands: &mut Commands, cursor_image: Option<Handle<Image>>) {
    let ui_camera = commands
        .spawn((
            ArenaOwned,
            Camera2d,
            Camera {
                order: 102,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Name::new("Developer Arena Full-window UI Camera"),
        ))
        .id();
    commands
        .spawn((
            ArenaOwned,
            UiTargetCamera(ui_camera),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(ARENA_PANEL_WIDTH),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                padding: UiRect::all(Val::Px(8.0)),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(PANEL),
            GlobalZIndex(100_000),
            Name::new("Developer Arena Panel"),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("RUSTGTA3 DEV ARENA"),
                TextFont::from_font_size(17.0),
                TextColor(TEXT),
            ));
            panel.spawn((
                Text::new("Insert closes • GTA frame is suspended"),
                TextFont::from_font_size(10.0),
                TextColor(MUTED),
            ));

            spawn_action_row(
                panel,
                &[
                    ("Reset", ArenaAction::Reset),
                    ("Clear Spawned", ArenaAction::ClearSpawned),
                ],
            );
            spawn_action_row(panel, &[("2 PLAYER", ArenaAction::ToggleLocalCoop)]);
            spawn_section(panel, "CHARACTER / CONTROL");
            spawn_action_row(
                panel,
                &[
                    ("<", ArenaAction::PreviousCharacter),
                    ("Apply / Spawn", ArenaAction::SpawnCharacter),
                    (">", ArenaAction::NextCharacter),
                ],
            );
            spawn_action_row(
                panel,
                &[
                    ("Idle", ArenaAction::Animation(ArenaAnimation::Idle)),
                    ("Walk", ArenaAction::Animation(ArenaAnimation::Walk)),
                    ("Run", ArenaAction::Animation(ArenaAnimation::Run)),
                    ("Sprint", ArenaAction::Animation(ArenaAnimation::Sprint)),
                    ("Jump", ArenaAction::Animation(ArenaAnimation::Jump)),
                    ("Fight", ArenaAction::Animation(ArenaAnimation::Fight)),
                ],
            );
            spawn_action_row(
                panel,
                &[
                    ("Anim -", ArenaAction::AnimationSlower),
                    (
                        "Enter anim",
                        ArenaAction::Animation(ArenaAnimation::EnterVehicle),
                    ),
                    ("Anim +", ArenaAction::AnimationFaster),
                ],
            );

            spawn_section(panel, "VEHICLE");
            spawn_action_row(
                panel,
                &[
                    ("<", ArenaAction::PreviousVehicle),
                    ("Spawn Ahead", ArenaAction::SpawnVehicle),
                    (">", ArenaAction::NextVehicle),
                ],
            );
            spawn_action_row(
                panel,
                &[("Enter / Exit nearest (F)", ArenaAction::EnterExitVehicle)],
            );

            spawn_section(panel, "PROP / MODEL");
            spawn_action_row(
                panel,
                &[
                    ("<", ArenaAction::PreviousProp),
                    ("Spawn Ahead", ArenaAction::SpawnProp),
                    (">", ArenaAction::NextProp),
                ],
            );
            spawn_action_row(
                panel,
                &[
                    ("-10", ArenaAction::CustomModelMinus10),
                    ("-1", ArenaAction::CustomModelMinus1),
                    ("+1", ArenaAction::CustomModelPlus1),
                    ("+10", ArenaAction::CustomModelPlus10),
                ],
            );
            spawn_action_row(
                panel,
                &[("Spawn direct model index", ArenaAction::SpawnCustomModel)],
            );
            spawn_action_row(
                panel,
                &[("Spawn GTA obstacle course", ArenaAction::SpawnTestCourse)],
            );

            spawn_section(panel, "TELEPORT");
            spawn_action_row(
                panel,
                &[("Choose destination", ArenaAction::ToggleTeleportDropdown)],
            );
            panel.spawn((
                ArenaTeleportLabel,
                Text::new("Destination: Arena Origin"),
                TextFont::from_font_size(9.5),
                TextColor(MUTED),
            ));
            panel
                .spawn((
                    ArenaTeleportList,
                    Visibility::Hidden,
                    ScrollPosition::default(),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(132.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.025, 0.032, 0.042)),
                ))
                .with_children(|list| {
                    for (index, (label, _)) in GTA_TELEPORTS.iter().enumerate() {
                        list.spawn((
                            Button,
                            Hovered::default(),
                            ArenaTeleportOption,
                            ArenaAction::SelectTeleport(index),
                            Node {
                                width: Val::Percent(100.0),
                                min_height: Val::Px(24.0),
                                padding: UiRect::horizontal(Val::Px(7.0)),
                                align_items: AlignItems::Center,
                                flex_shrink: 0.0,
                                ..default()
                            },
                            BackgroundColor(BUTTON),
                        ))
                        .with_child((
                            Text::new(*label),
                            TextFont::from_font_size(9.5),
                            TextColor(TEXT),
                        ));
                    }
                });
            spawn_action_row(panel, &[("Teleport", ArenaAction::ExecuteTeleport)]);
            spawn_action_row(
                panel,
                &[("Look direction +20m", ArenaAction::TeleportForward)],
            );

            spawn_section(panel, "LIVE STATUS");
            panel.spawn((
                ArenaStatusText,
                Text::new("initialising…"),
                TextFont::from_font_size(10.5),
                TextColor(TEXT),
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
            ));
            panel.spawn((
                Text::new(
                    "WASD move/drive • Shift sprint • Space jump/brake\n\
                     RMB+mouse or arrows orbit • Home toggles unlimited wheel zoom • E vehicle\n\
                     PAD: X/LT brake • A/RT accelerate • Y vehicle • RB handbrake\n\
                     X jumps on foot • A sprints on foot\n\
                     Each player's right stick controls only their own camera",
                ),
                TextFont::from_font_size(9.5),
                TextColor(MUTED),
            ));
        });

    commands
        .spawn((
            ArenaOwned,
            UiTargetCamera(ui_camera),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(ARENA_PANEL_WIDTH + 10.0),
                top: Val::Px(9.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.025, 0.035, 0.78)),
            GlobalZIndex(99_999),
            Name::new("Developer Arena Viewport Help"),
        ))
        .with_child((
            Text::new("ISOLATED GRID VIEWPORT  |  DFF-ONLY REVISION 3  |  production RW material"),
            TextFont::from_font_size(10.0),
            TextColor(TEXT),
        ));

    if let Some(cursor_image) = cursor_image {
        commands.spawn((
            ArenaOwned,
            ArenaMouseCursor,
            bevy::picking::Pickable::IGNORE,
            UiTargetCamera(ui_camera),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(75.0),
                height: Val::Px(75.0),
                ..default()
            },
            ImageNode {
                image: cursor_image,
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(200_000),
            Name::new("Developer Arena GTA Cursor"),
        ));
    }
}

pub(crate) fn sync_arena_custom_cursor(
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursors: Query<(&mut Node, &mut Visibility), With<ArenaMouseCursor>>,
) {
    if cursors.is_empty() {
        return;
    }
    let Ok((window, mut cursor_options)) = windows.single_mut() else {
        return;
    };
    cursor_options.visible = false;
    let Some(position) = window.cursor_position() else {
        for (_, mut visibility) in &mut cursors {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let scale = (window.height() / 448.0).max(0.5);
    for (mut node, mut visibility) in &mut cursors {
        node.left = Val::Px(position.x);
        node.top = Val::Px(position.y);
        // Match CMenuManager's authored 75x75 frontend cursor exactly.
        node.width = Val::Px(75.0 * scale);
        node.height = Val::Px(75.0 * scale);
        *visibility = Visibility::Visible;
    }
}

fn spawn_section(parent: &mut ChildSpawnerCommands, text: &'static str) {
    parent.spawn((
        Text::new(text),
        TextFont::from_font_size(10.5),
        TextColor(SECTION),
        Node {
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        },
    ));
}

fn spawn_action_row(parent: &mut ChildSpawnerCommands, buttons: &[(&'static str, ArenaAction)]) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(25.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            for (label, action) in buttons {
                row.spawn((
                    Button,
                    Hovered::default(),
                    *action,
                    Node {
                        height: Val::Px(24.0),
                        flex_grow: 1.0,
                        padding: UiRect::horizontal(Val::Px(5.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON),
                ))
                .with_child((
                    Text::new(*label),
                    TextFont::from_font_size(10.0),
                    TextColor(TEXT),
                ));
            }
        });
}

#[derive(SystemParam)]
pub(crate) struct ArenaButtonWorld<'w, 's> {
    primary: crate::player_access::ArenaPrimaryPlayers<'w, 's>,
    scene_root: Query<'w, 's, Entity, With<ArenaSceneRoot>>,
    players: Query<
        'w,
        's,
        (Entity, &'static mut Transform),
        (
            With<ArenaPlayer>,
            Without<ArenaVehicle>,
            Without<ArenaCoopPlayerTwo>,
        ),
    >,
    player_two: Query<
        'w,
        's,
        (&'static mut Transform, crate::player_access::ArenaCoopActor),
        (With<ArenaCoopPlayerTwo>, Without<ArenaVehicle>),
    >,
    vehicles: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Transform,
            &'static mut ArenaVehicle,
            Option<crate::ArenaAutomobileRuntime>,
        ),
        (With<ArenaVehicle>, Without<ArenaPlayer>),
    >,
    world_maps: Query<'w, 's, Entity, With<ArenaWorldMap>>,
    grids: Query<'w, 's, &'static mut Visibility, With<ArenaGrid>>,
    spawned: Query<'w, 's, Entity, With<ArenaSpawned>>,
    character_visuals: Query<'w, 's, (Entity, &'static ArenaCharacterVisual)>,
}

pub(crate) fn handle_arena_buttons(
    mut commands: Commands,
    mut buttons: Query<(&Hovered, Option<Ref<Pressed>>, &ArenaAction, &mut BackgroundColor)>,
    mut arena: ResMut<DeveloperArena>,
    mut world: ArenaButtonWorld,
    mut runtime: ArenaRuntimeAssets,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut local_coop: ResMut<LocalCoopSession>,
    mut vehicle_diagnostics: ResMut<crate::vehicle_construction::ArenaVehicleSpawnDiagnostics>,
    mut random: ResMut<crate::ArenaRandomState>,
) {
    for (hovered, pressed, action, mut background) in &mut buttons {
        background.set_if_neq(BackgroundColor(if pressed.is_some() {
            BUTTON_PRESSED
        } else if hovered.get() {
            BUTTON_HOVER
        } else {
            BUTTON
        }));
        if !pressed.is_some_and(|pressed| pressed.is_added()) {
            continue;
        }

        match *action {
            ArenaAction::ToggleLocalCoop => {
                let enabled = local_coop.toggle();
                arena.status = if enabled {
                    format!(
                        "Local co-op enabled in Developer Arena | {} gamepad(s) detected",
                        local_coop.connected_gamepads
                    )
                } else {
                    format!(
                        "Local co-op disabled in Developer Arena | {} gamepad(s) detected",
                        local_coop.connected_gamepads
                    )
                };
            }
            ArenaAction::ToggleTeleportDropdown => {
                arena.teleport_dropdown_open = !arena.teleport_dropdown_open;
            }
            ArenaAction::SelectTeleport(index) => {
                if index < GTA_TELEPORTS.len() {
                    arena.selected_teleport = index;
                    arena.teleport_dropdown_open = false;
                    arena.status = format!("Teleport selected: {}", GTA_TELEPORTS[index].0);
                }
            }
            ArenaAction::ExecuteTeleport => {
                let index = arena.selected_teleport;
                if let Some((label, gta)) = GTA_TELEPORTS.get(index).copied() {
                    let gta_position = CVector::new(gta[0], gta[1], gta[2]);
                    if index == 0 {
                        scene::clear_arena_world_map(
                            &mut commands,
                            &world.world_maps,
                            &mut world.grids,
                        );
                        teleport_arena_focus(&mut arena, gta_position.to_bevy(), &mut world);
                        arena.status = "Arena focus teleported to origin".to_owned();
                        continue;
                    }
                    let Ok(scene_root) = world.scene_root.single() else {
                        arena.status =
                            "GTA map load failed: Arena scene root is unavailable".to_owned();
                        continue;
                    };
                    match scene::replace_arena_world_map(
                        &mut commands,
                        scene_root,
                        gta_position,
                        &world.world_maps,
                        &mut world.grids,
                        &mut runtime.original,
                        &mut runtime.texture_cache,
                        &mut runtime.images,
                        &mut meshes,
                        &mut materials,
                    ) {
                        Ok((rendered, ground_z)) => {
                            let resolved_position =
                                CVector::new(gta_position.x, gta_position.y, ground_z);
                            teleport_arena_focus(
                                &mut arena,
                                resolved_position.to_bevy(),
                                &mut world,
                            );
                            arena.status = format!(
                                "{label}: original GTA district loaded inside Arena ({rendered} IPL models)"
                            );
                        }
                        Err(error) => {
                            arena.status = format!("{label}: Arena GTA map load failed: {error}");
                        }
                    }
                }
            }
            ArenaAction::Reset => {
                teleport_arena_focus(&mut arena, Vec3::ZERO, &mut world);
                for (_, _, mut vehicle, _) in &mut world.vehicles {
                    vehicle.speed = 0.0;
                    vehicle.steering = 0.0;
                }
                arena.status = "Arena reset to origin".to_owned();
            }
            ArenaAction::ClearSpawned => {
                for entity in &world.spawned {
                    commands.entity(entity).despawn();
                }
                arena.controlled_vehicle = None;
                arena.vehicle_transition = None;
                arena.spawned_vehicles = 0;
                arena.spawned_props = 0;
                arena.status = "Spawned vehicles and props cleared".to_owned();
            }
            ArenaAction::PreviousCharacter => {
                arena.selected_character =
                    wrap_previous(arena.selected_character, CHARACTERS.len());
                arena.status = format!(
                    "Character selected: {}",
                    CHARACTERS[arena.selected_character].label
                );
            }
            ArenaAction::NextCharacter => {
                arena.selected_character = (arena.selected_character + 1) % CHARACTERS.len();
                arena.status = format!(
                    "Character selected: {}",
                    CHARACTERS[arena.selected_character].label
                );
            }
            ArenaAction::SpawnCharacter => {
                let Ok((player, _)) = world.players.single_mut() else {
                    continue;
                };
                let entry = CHARACTERS[arena.selected_character];
                if let Err(error) = make_model_resident_and_sync(
                    entry.model_index,
                    &mut runtime.original,
                    &mut runtime.texture_cache,
                    &mut runtime.images,
                ) {
                    arena.status = format!("Character load failed: {error}");
                    continue;
                }
                for (visual_entity, visual) in &world.character_visuals {
                    if visual.local_player == 0 {
                        commands.entity(visual_entity).despawn();
                    }
                }
                let texture_cache = &*runtime.texture_cache;
                let file_loader = runtime.original.data.as_ref().map(|data| &data.file_loader);
                let loaded = scene::spawn_character_visual(
                    &mut commands,
                    player,
                    0,
                    entry.model_index,
                    file_loader,
                    texture_cache,
                    &mut meshes,
                    &mut materials,
                    &RenderLayers::layer(ARENA_RENDER_LAYER),
                );
                arena.spawned_characters = arena.spawned_characters.saturating_add(1);
                arena.status = model_status("Character", entry.label, entry.model_index, loaded);
            }
            ArenaAction::Animation(animation) => {
                arena.animation = animation;
                // Re-pressing the same button must restart a one-shot jump or
                // vehicle-enter association from time zero.
                arena.runtime_animation_selection = None;
                arena.status = format!("Animation test: {}", animation.label());
            }
            ArenaAction::AnimationSlower => {
                arena.animation_speed = (arena.animation_speed - 0.25).max(0.25);
                arena.status = format!("Animation speed {:.2}x", arena.animation_speed);
            }
            ArenaAction::AnimationFaster => {
                arena.animation_speed = (arena.animation_speed + 0.25).min(4.0);
                arena.status = format!("Animation speed {:.2}x", arena.animation_speed);
            }
            ArenaAction::PreviousVehicle => {
                arena.selected_vehicle = wrap_previous(arena.selected_vehicle, VEHICLES.len());
                arena.status = format!(
                    "Vehicle selected: {}",
                    VEHICLES[arena.selected_vehicle].label
                );
            }
            ArenaAction::NextVehicle => {
                arena.selected_vehicle = (arena.selected_vehicle + 1) % VEHICLES.len();
                arena.status = format!(
                    "Vehicle selected: {}",
                    VEHICLES[arena.selected_vehicle].label
                );
            }
            ArenaAction::SpawnVehicle => {
                let Ok(scene_root) = world.scene_root.single() else {
                    continue;
                };
                let position = match vehicle_spawn_position(&world, 3.5) {
                    Ok(position) => position,
                    Err(error) => {
                        arena.status = format!("Vehicle spawn failed: {error}");
                        continue;
                    }
                };
                let entry = VEHICLES[arena.selected_vehicle];
                if let Err(error) = make_model_resident_and_sync(
                    entry.model_index,
                    &mut runtime.original,
                    &mut runtime.texture_cache,
                    &mut runtime.images,
                ) {
                    arena.status = format!("Vehicle load failed: {error}");
                    continue;
                }
                let texture_cache = &*runtime.texture_cache;
                let Some(data) = runtime.original.data.as_mut() else {
                    arena.status = "Vehicle runtime unavailable".to_owned();
                    continue;
                };
                let spawn = scene::spawn_vehicle(
                    &mut commands,
                    scene_root,
                    position,
                    entry.model_index,
                    Some(&mut data.file_loader),
                    texture_cache,
                    Some(&data.handling),
                    Some(&data.surface_table),
                    Some(&mut random.0),
                    Some(&data.vehicle_colours),
                    &mut meshes,
                    &mut materials,
                    &mut vehicle_diagnostics,
                );
                let loaded = spawn.visual_loaded;
                let handling_bound = spawn.runtime_ready();
                if spawn.entity.is_some() {
                    arena.spawned_vehicles = arena.spawned_vehicles.saturating_add(1);
                }
                arena.status = match &spawn.runtime {
                    crate::vehicle_construction::ArenaVehicleRuntimeOutcome::Failed(error) => error.to_string(),
                    crate::vehicle_construction::ArenaVehicleRuntimeOutcome::UnsupportedKind =>
                        format!("{} | Original model viewer", model_status("Vehicle", entry.label, entry.model_index, loaded)),
                    crate::vehicle_construction::ArenaVehicleRuntimeOutcome::Ready =>
                        vehicle_model_status(entry.label, entry.model_index, loaded, handling_bound),
                };
                info!(
                    model_index = entry.model_index,
                    label = entry.label,
                    loaded,
                    handling_bound,
                    "Developer arena vehicle spawned"
                );
            }
            ArenaAction::EnterExitVehicle => {
                arena.request_enter_exit = true;
            }
            ArenaAction::PreviousProp => {
                arena.selected_prop = wrap_previous(arena.selected_prop, PROPS.len());
                arena.status = format!("Prop selected: {}", PROPS[arena.selected_prop].label);
            }
            ArenaAction::NextProp => {
                arena.selected_prop = (arena.selected_prop + 1) % PROPS.len();
                arena.status = format!("Prop selected: {}", PROPS[arena.selected_prop].label);
            }
            ArenaAction::SpawnProp => {
                let Ok(scene_root) = world.scene_root.single() else {
                    continue;
                };
                let position = spawn_position(&arena, &world, 5.0);
                let entry = PROPS[arena.selected_prop];
                if let Err(error) = make_model_resident_and_sync(
                    entry.model_index,
                    &mut runtime.original,
                    &mut runtime.texture_cache,
                    &mut runtime.images,
                ) {
                    arena.status = format!("Prop load failed: {error}");
                    continue;
                }
                let texture_cache = &*runtime.texture_cache;
                let file_loader = runtime.original.data.as_ref().map(|data| &data.file_loader);
                let (_, loaded) = scene::spawn_prop(
                    &mut commands,
                    scene_root,
                    position,
                    entry.model_index,
                    file_loader,
                    texture_cache,
                    &mut meshes,
                    &mut materials,
                );
                arena.spawned_props = arena.spawned_props.saturating_add(1);
                arena.status = model_status("Prop", entry.label, entry.model_index, loaded);
            }
            ArenaAction::SpawnTestCourse => {
                let Ok(scene_root) = world.scene_root.single() else {
                    continue;
                };
                let forward = scene::view_forward(&arena);
                let right = Vec3::new(-forward.z, 0.0, forward.x);
                let focus = spawn_position(&arena, &world, 0.0);
                let mut loaded_count = 0u32;
                for (slot, entry) in PROPS.iter().copied().cycle().take(6).enumerate() {
                    let position = focus
                        + forward * (50.0 + slot as f32 * 50.0)
                        + right * if slot % 2 == 0 { 8.0 } else { -8.0 };
                    if make_model_resident_and_sync(
                        entry.model_index,
                        &mut runtime.original,
                        &mut runtime.texture_cache,
                        &mut runtime.images,
                    )
                    .is_err()
                    {
                        continue;
                    }
                    let texture_cache = &*runtime.texture_cache;
                    let file_loader = runtime.original.data.as_ref().map(|data| &data.file_loader);
                    let (_, loaded) = scene::spawn_prop(
                        &mut commands,
                        scene_root,
                        position,
                        entry.model_index,
                        file_loader,
                        texture_cache,
                        &mut meshes,
                        &mut materials,
                    );
                    loaded_count += u32::from(loaded);
                    arena.spawned_props = arena.spawned_props.saturating_add(1);
                }
                arena.status =
                    format!("GTA obstacle course spawned: {loaded_count}/6 real DFF props");
            }
            ArenaAction::CustomModelMinus10 => {
                arena.custom_model_index = arena.custom_model_index.saturating_sub(10);
                arena.status = format!("Direct model index: {}", arena.custom_model_index);
            }
            ArenaAction::CustomModelMinus1 => {
                arena.custom_model_index = arena.custom_model_index.saturating_sub(1);
                arena.status = format!("Direct model index: {}", arena.custom_model_index);
            }
            ArenaAction::CustomModelPlus1 => {
                arena.custom_model_index = arena.custom_model_index.saturating_add(1).min(20_000);
                arena.status = format!("Direct model index: {}", arena.custom_model_index);
            }
            ArenaAction::CustomModelPlus10 => {
                arena.custom_model_index = arena.custom_model_index.saturating_add(10).min(20_000);
                arena.status = format!("Direct model index: {}", arena.custom_model_index);
            }
            ArenaAction::SpawnCustomModel => {
                let Ok(scene_root) = world.scene_root.single() else {
                    continue;
                };
                let position = spawn_position(&arena, &world, 5.0);
                let model_index = arena.custom_model_index;
                if let Err(error) = make_model_resident_and_sync(
                    model_index,
                    &mut runtime.original,
                    &mut runtime.texture_cache,
                    &mut runtime.images,
                ) {
                    arena.status = format!("Direct model load failed: {error}");
                    continue;
                }
                let texture_cache = &*runtime.texture_cache;
                let file_loader = runtime.original.data.as_ref().map(|data| &data.file_loader);
                let (_, loaded) = scene::spawn_prop(
                    &mut commands,
                    scene_root,
                    position,
                    model_index,
                    file_loader,
                    texture_cache,
                    &mut meshes,
                    &mut materials,
                );
                arena.spawned_props = arena.spawned_props.saturating_add(1);
                arena.status = model_status("Direct model", "custom", model_index, loaded);
            }
            ArenaAction::TeleportForward => {
                let destination = spawn_position(&arena, &world, 20.0);
                teleport_arena_focus(&mut arena, destination, &mut world);
                arena.status = "Teleported 20m along camera look direction".to_owned();
            }
        }
    }
}

pub(crate) fn sync_teleport_dropdown(
    arena: Res<DeveloperArena>,
    mut lists: Query<&mut Visibility, With<ArenaTeleportList>>,
    mut labels: Query<&mut Text, With<ArenaTeleportLabel>>,
) {
    for mut visibility in &mut lists {
        *visibility = if arena.teleport_dropdown_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Some((label, _)) = GTA_TELEPORTS.get(arena.selected_teleport).copied() {
        for mut text in &mut labels {
            text.0 = format!("Destination: {label}");
        }
    }
}

pub(crate) fn scroll_teleport_dropdown(
    arena: Res<DeveloperArena>,
    mut wheel: MessageReader<MouseWheel>,
    mut lists: Query<&mut ScrollPosition, With<ArenaTeleportList>>,
) {
    let delta = wheel.read().map(|event| event.y).sum::<f32>();
    if !arena.teleport_dropdown_open || delta == 0.0 {
        return;
    }
    for mut position in &mut lists {
        position.0.y = (position.0.y - delta * 28.0).max(0.0);
    }
}

#[cfg(test)]
fn teleport_script_vehicle(vehicle: &mut ScriptVehicleEntity, destination: CVector) -> bool {
    match vehicle {
        ScriptVehicleEntity::Automobile { car, .. } => {
            let mut position = destination;
            position.z += car.height_above_road;
            car.vehicle.physical.entity.set_position(position);
            car.vehicle.physical.move_speed = CVector::ZERO;
            car.vehicle.physical.turn_speed = CVector::ZERO;
        }
        ScriptVehicleEntity::Boat { boat, .. } => {
            boat.vehicle.physical.entity.set_position(destination);
            boat.vehicle.physical.move_speed = CVector::ZERO;
            boat.vehicle.physical.turn_speed = CVector::ZERO;
        }
        ScriptVehicleEntity::Heli { heli, .. } => {
            heli.vehicle.physical.entity.set_position(destination);
            heli.vehicle.physical.move_speed = CVector::ZERO;
            heli.vehicle.physical.turn_speed = CVector::ZERO;
        }
        ScriptVehicleEntity::Plane { plane, .. } => {
            plane.vehicle.physical.entity.set_position(destination);
            plane.vehicle.physical.move_speed = CVector::ZERO;
            plane.vehicle.physical.turn_speed = CVector::ZERO;
        }
    }
    true
}

#[cfg(test)]
fn promote_arena_automobile(
    pools: &mut CPools,
    mut car: rustgta3_game::vehicles::Automobile::CAutomobile,
    _driver_slot: usize,
    destination: CVector,
) -> Option<usize> {
    let handle = pools
        .vehicle_pool
        .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))?;
    let slot = handle.index();
    let mut position = destination;
    position.z += car.height_above_road;
    car.vehicle.physical.entity.set_position(position);
    car.vehicle.physical.move_speed = CVector::ZERO;
    car.vehicle.physical.turn_speed = CVector::ZERO;
    car.vehicle.drive.engine_on = true;
    let _entity = ScriptVehicleEntity::Automobile {
        handle: handle.0,
        car,
    };
    Some(slot)
}

fn make_model_resident_and_sync(
    model_index: usize,
    original: &mut ArenaOriginalRuntime,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
) -> Result<(), String> {
    let data = original.ensure_loaded()?;
    crate::ensure_arena_model_resident(
        model_index,
        Some(&mut data.file_loader),
        Some(&mut data.streaming),
        Some(&mut data.before_rw),
    )?;
    if !sync_model_texture_cache(model_index, &data.file_loader, texture_cache, images) {
        return Err(format!(
            "GTA model #{model_index} TXD raster upload did not complete"
        ));
    }
    Ok(())
}

fn spawn_position(arena: &DeveloperArena, world: &ArenaButtonWorld, distance: f32) -> Vec3 {
    let focus = if let Some(vehicle) = arena.controlled_vehicle {
        world
            .vehicles
            .get(vehicle)
            .ok()
            .map(|(_, transform, _, _)| transform.translation)
    } else {
        world
            .players
            .iter()
            .next()
            .map(|(_, transform)| transform.translation)
    }
    .unwrap_or(Vec3::ZERO);
    let mut position = focus + scene::view_forward(arena) * distance;
    // Spawn requests name a point on the arena road plane. Vehicle and ped
    // constructors apply their own re3 matrix-origin height above that plane.
    position.y = 0.0;
    position
}

/// Put a debug-spawned vehicle directly in front of Claude. re3's `SpawnCar`
/// raises the final vehicle matrix by `m_fHeightAboveRoad` inside the
/// constructor. This helper therefore supplies the road/release point at the
/// player's feet; passing the ped matrix origin here would add both ped height
/// and vehicle height a second time.
fn vehicle_spawn_position(world: &ArenaButtonWorld, distance: f32) -> Result<Vec3, String> {
    let (_, player) = world
        .players
        .iter()
        .next()
        .ok_or_else(|| "Arena player entity is unavailable".to_owned())?;
    let forward = player.forward().as_vec3();
    let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let mut position = player.translation + horizontal_forward * distance;
    position.y = player.translation.y - crate::ARENA_PED_BASE_OFFSET + 0.10;
    Ok(position)
}

fn teleport_arena_focus(
    arena: &mut DeveloperArena,
    destination: Vec3,
    world: &mut ArenaButtonWorld,
) {
    if let Some(vehicle) = arena.controlled_vehicle
        && let Ok((_, mut transform, mut vehicle, automobile)) = world.vehicles.get_mut(vehicle)
    {
        if let Some(automobile) = automobile {
            let mut automobile = automobile.into_view();
            let mut gta = CVector::from_bevy(destination);
            gta.z += automobile.car.height_above_road;
            automobile.car.vehicle.physical.entity.set_position(gta);
            automobile
                .car
                .vehicle
                .physical
                .entity
                .set_orientation(0.0, 0.0, 0.0);
            automobile.car.vehicle.physical.move_speed = CVector::ZERO;
            automobile.car.vehicle.physical.turn_speed = CVector::ZERO;
            automobile.car.reset_suspension();
            transform.translation = gta.to_bevy();
            transform.rotation = Quat::IDENTITY;
            vehicle.speed = 0.0;
            vehicle.steering = 0.0;
        }
        return;
    }
    if let Ok((_, mut transform)) = world.players.single_mut() {
        transform.translation = destination + Vec3::Y * crate::ARENA_PED_BASE_OFFSET;
    }
    if let Some(mut runtime) = world.primary.get_mut(arena.player_entity) {
        let mut gta = CVector::from_bevy(destination);
        gta.z += crate::ARENA_PED_BASE_OFFSET;
        runtime.set_position(gta);
    }
    for (mut transform, player) in &mut world.player_two {
        let mut player = player.into_view();
        let player_position = destination + Vec3::X * 2.0 + Vec3::Y * crate::ARENA_PED_BASE_OFFSET;
        transform.translation = player_position;
        player
            .runtime
            .set_position(CVector::from_bevy(player_position));
    }
}

pub(crate) fn update_status_text(
    time: Res<Time>,
    arena: Res<DeveloperArena>,
    local_coop: Res<LocalCoopSession>,
    player: Query<&Transform, With<ArenaPlayer>>,
    vehicles: Query<(&Transform, &ArenaVehicle)>,
    mut texts: Query<&mut Text, With<ArenaStatusText>>,
    mut elapsed: Local<f32>,
    mut previous_text: Local<String>,
) {
    *elapsed += time.delta_secs();
    if *elapsed < 0.25 && !previous_text.is_empty() {
        return;
    }
    *elapsed = 0.0;
    let position = if let Some(vehicle) = arena.controlled_vehicle {
        vehicles
            .get(vehicle)
            .ok()
            .map(|(transform, _)| transform.translation)
    } else {
        player.iter().next().map(|transform| transform.translation)
    }
    .unwrap_or(Vec3::ZERO);
    let selected_character = CHARACTERS[arena.selected_character];
    let selected_vehicle = VEHICLES[arena.selected_vehicle];
    let selected_prop = PROPS[arena.selected_prop];
    let text = format!(
        "{}\n\
         pos [{:.1}, {:.1}, {:.1}]  anim {} {:.2}x\n\
         runtime {} {:.2}/{:.2}s mesh {}\n\
         char {} #{} | car {} #{}\n\
         prop {} #{} | direct #{}\n\
         co-op {} | gamepads {} | split {:?}\n\
         spawned C/V/P {}/{}/{}",
        arena.status,
        position.x,
        position.y,
        position.z,
        arena.animation.label(),
        arena.animation_speed,
        arena
            .runtime_animation_name
            .as_deref()
            .unwrap_or("unavailable"),
        arena.runtime_animation_time,
        arena.runtime_animation_duration,
        if arena.runtime_animation_mesh_bound {
            "original DFF + PED.IFP"
        } else {
            "original DFF bind pose / animation unavailable"
        },
        selected_character.label,
        selected_character.model_index,
        selected_vehicle.label,
        selected_vehicle.model_index,
        selected_prop.label,
        selected_prop.model_index,
        arena.custom_model_index,
        if local_coop.enabled { "ON" } else { "OFF" },
        local_coop.connected_gamepads,
        local_coop.split_layout,
        arena.spawned_characters,
        arena.spawned_vehicles,
        arena.spawned_props,
    );
    if *previous_text == text {
        return;
    }
    previous_text.clone_from(&text);
    for mut status in &mut texts {
        status.0.clone_from(&text);
    }
}

fn wrap_previous(current: usize, len: usize) -> usize {
    if current == 0 { len - 1 } else { current - 1 }
}

fn model_status(kind: &str, label: &str, model_index: usize, loaded: bool) -> String {
    if loaded {
        format!("{kind} {label} #{model_index}: loaded GTA DFF geometry")
    } else {
        format!("{kind} {label} #{model_index}: ERROR — original DFF geometry unavailable")
    }
}

fn vehicle_model_status(
    label: &str,
    model_index: usize,
    loaded: bool,
    handling_bound: bool,
) -> String {
    format!(
        "{} | {}",
        model_status("Vehicle", label, model_index, loaded),
        if handling_bound {
            "HANDLING.CFG + CAutomobile runtime"
        } else {
            "ERROR — HANDLING.CFG/CAutomobile runtime unavailable"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustgta3_core::core::General::CGeneral;
    use rustgta3_game::vehicles::Automobile::CAutomobile;

    #[test]
    fn selection_wraps_back_from_zero() {
        assert_eq!(wrap_previous(0, 8), 7);
        assert_eq!(wrap_previous(3, 8), 2);
    }

    #[test]
    fn model_status_reports_real_geometry_without_ambiguity() {
        assert!(model_status("Vehicle", "Taxi", 110, true).contains("loaded GTA DFF"));
        assert!(model_status("Vehicle", "Taxi", 110, false).contains("ERROR"));
        assert!(!model_status("Vehicle", "Taxi", 110, false).contains("proxy"));
        assert!(
            vehicle_model_status("Taxi", 110, true, true).contains("HANDLING.CFG + CAutomobile")
        );
    }

    #[test]
    fn script_vehicle_teleport_uses_road_origin_and_clears_motion() {
        let mut general = CGeneral::with_seed(3);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        car.height_above_road = 1.25;
        car.vehicle.physical.move_speed = CVector::new(1.0, 2.0, 3.0);
        car.vehicle.physical.turn_speed = CVector::new(0.2, 0.3, 0.4);
        let mut vehicle = ScriptVehicleEntity::Automobile {
            handle: (7 << 8) | 1,
            car,
        };

        assert!(teleport_script_vehicle(
            &mut vehicle,
            CVector::new(10.0, 20.0, 4.0),
        ));
        let ScriptVehicleEntity::Automobile { car, .. } = &vehicle else {
            panic!("test vehicle changed type");
        };
        assert_eq!(
            car.vehicle.physical.entity.get_position(),
            CVector::new(10.0, 20.0, 5.25)
        );
        assert_eq!(car.vehicle.physical.move_speed, CVector::ZERO);
        assert_eq!(car.vehicle.physical.turn_speed, CVector::ZERO);
    }

    #[test]
    fn promoted_arena_car_owns_a_real_vehicle_pool_slot() {
        let mut general = CGeneral::with_seed(5);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        car.height_above_road = 0.8;
        let mut pools = CPools::initialise();

        let slot = promote_arena_automobile(&mut pools, car, 11, CVector::new(100.0, 200.0, 6.0))
            .expect("vehicle pool has free slots");
        let handle = pools
            .get_vehicle_ref(slot)
            .expect("promoted automobile must retain its pool generation");
        assert!(pools.get_vehicle(handle).is_some());
    }
}
