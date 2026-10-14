mod collision_runtime;
pub(crate) use collision_runtime::resolve_arena_vehicle_collisions;
use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};
use rustgta3_game::{
    animation::{
        AnimAssocDefinitions::{ASSOCGRP_PLAYER, ASSOCGRP_STD},
        AnimBlendAssociation::CAnimBlendAssociation,
        AnimationId::{
            ANIM_STD_CAR_ALIGN_DOOR_LHS, ANIM_STD_CAR_ALIGN_DOOR_RHS,
            ANIM_STD_CAR_ALIGNHI_DOOR_LHS, ANIM_STD_CAR_ALIGNHI_DOOR_RHS, ANIM_STD_CAR_GET_IN_LHS,
            ANIM_STD_CAR_SIT, ANIM_STD_FIGHT_IDLE, ANIM_STD_IDLE, ANIM_STD_JUMP_LAUNCH,
            ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_WALK,
        },
        PedVehicleAnimation::PedVehicleAnimationPhase,
    },
    collision::Collision::{CCollision, resolve_player_ped_static_collision_result},
    core::Game::{
        AutomobileRoadContacts, PedAutomobileProfile, advance_player_jump_launch,
        automobile_door_flag, automobile_road_contacts, automobile_static_collision_step,
        begin_player_jump, store_automobile_wheel_contacts, update_player_jump_animation,
        update_player_movement_animation,
    },
    core::LocalCoop::{LocalCoopSession, controller_state_from_gamepad},
    core::Pad::CPad,
    entities::{Entity::eEntityStatus, Physical::CPhysicalCollisionOptions},
    math::Matrix::CMatrix,
    peds::{
        Ped::{CPed, PedFightMoves, PedState, automobile_driver_world_position, eMoveState},
        PedAI::{automobile_ped_door_world_position, ped_vehicle_line_up_seat_multiplier},
        PedFight::{
            FIGHTSTATE_JUST_ATTACKED, FIGHTSTATE_MOVE_FINISHED, FIGHTSTATE_NO_MOVE,
            FightEndType, FightMoveHitLevel, NUM_FIGHTMOVES, end_fight, fight_move_from_index,
            start_fight_attack, start_fight_move,
        },
        PlayerPed::{
            apply_player_mouse_movement_direction, apply_player_ped_air_resistance,
            begin_player_collision_frame, finish_player_collision_frame,
            prepare_player_physical_for_control,
        },
    },
    renderer::BevyRender::{
        build_animated_ped_material_drawables,
        build_animated_ped_material_drawables_from_association,
        rigid_ped_part_transforms_from_association, rigid_ped_part_transforms_from_clump,
    },
    vehicles::Automobile::{
        CAutomobileControlInput, CAutomobileDriveStepInput, NUM_CAR_WHEELS, VehicleCollisionState, eCarPiece,
    },
};

use crate::{
    ArenaAnimation, ArenaAutomobileRuntime, ArenaAutomobileView, ArenaAutomobileMut, ArenaCharacterModelPart, ArenaCharacterVisual,
    ArenaCoopPlayerTwo, ArenaOriginalData, ArenaOriginalRuntime, ArenaPlayer, ArenaProp,
    ArenaVehicle, ArenaVehicleTransition, ArenaWorldMap, DeveloperArena,
};

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ArenaControlClock<'w, 's> {
    timer: Option<ResMut<'w, CTimer>>,
    random: ResMut<'w, crate::ArenaRandomState>,
    primary: crate::player_access::ArenaPrimaryPlayers<'w, 's>,
}

pub(crate) fn control_arena(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut arena: ResMut<DeveloperArena>,
    mut session: ResMut<LocalCoopSession>,
    original: Res<ArenaOriginalRuntime>,
    clock: ArenaControlClock,
    mut players: Query<
        (Entity, &mut Transform, &mut ArenaPlayer, &mut Visibility),
        Without<ArenaCoopPlayerTwo>,
    >,
    mut player_two_targets: Query<
        (&Transform, crate::player_access::ArenaCoopActor),
        (With<ArenaCoopPlayerTwo>, Without<ArenaPlayer>),
    >,
    mut histories: Query<&mut VehicleCollisionState>,
    arena_world_maps: Query<(), With<ArenaWorldMap>>,
    mut vehicles: Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
) {
    let ArenaControlClock { mut timer, mut random, mut primary } = clock;
    let dt = time.delta_secs().min(0.05);
    let arena_world_map_active = !arena_world_maps.is_empty();
    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        mouse_delta += event.delta;
    }
    if mouse_buttons.pressed(MouseButton::Right) {
        arena.camera_yaw -= mouse_delta.x * 0.0035;
        // Window mouse Y grows downwards. Roblox-style RMB orbit therefore
        // adds the delta: dragging upward lowers the orbit pitch and looks up.
        // Keep the angle wrapped, not clamped, so the camera can pass freely
        // over and under the focus point.
        arena.camera_pitch = wrap_camera_angle(arena.camera_pitch + mouse_delta.y * 0.0025);
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        arena.camera_yaw += 1.5 * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        arena.camera_yaw -= 1.5 * dt;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        arena.camera_pitch = wrap_camera_angle(arena.camera_pitch - 0.9 * dt);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        arena.camera_pitch = wrap_camera_angle(arena.camera_pitch + 0.9 * dt);
    }
    if keys.just_pressed(KeyCode::Home) {
        arena.zoom_mode = !arena.zoom_mode;
        arena.status = if arena.zoom_mode {
            "Free camera zoom mode ON — mouse wheel has unlimited range".to_owned()
        } else {
            "Free camera zoom mode OFF".to_owned()
        };
    }
    for event in mouse_wheel.read() {
        if arena.zoom_mode && !arena.teleport_dropdown_open {
            let next = arena.camera_distance * (-event.y * 0.18).exp();
            if next.is_finite() && next > 0.0 {
                arena.camera_distance = next;
            }
        }
    }

    let mut connected = gamepads.iter().collect::<Vec<_>>();
    connected.sort_unstable_by_key(|(entity, _)| entity.to_bits());
    session.connected_gamepads = connected.len();
    let mut pad_state = connected
        .first()
        .map(|(_, gamepad)| {
            controller_state_from_gamepad(gamepad, arena.controlled_vehicle.is_some())
        })
        .unwrap_or_default();
    let raw_x = axis(&keys, KeyCode::KeyA, KeyCode::KeyD);
    let raw_y = axis(&keys, KeyCode::KeyS, KeyCode::KeyW);
    let button = |pressed: bool| if pressed { 255 } else { 0 };
    if raw_x != 0.0 {
        pad_state.left_stick_x = (raw_x * 128.0) as i16;
    }
    if raw_y != 0.0 {
        pad_state.left_stick_y = (-raw_y * 128.0) as i16;
    }
    if arena.controlled_vehicle.is_some() {
        // re3 CPad mode 0 reads the handbrake from RightShoulder1 and brake
        // from Square. Feeding Space to both made a stopped car receive
        // negative gas through ProcessControlInputs' normal reverse branch.
        pad_state.right_shoulder1 = pad_state
            .right_shoulder1
            .max(button(keys.pressed(KeyCode::Space)));
    } else {
        pad_state.square = pad_state.square.max(button(keys.pressed(KeyCode::Space)));
    }
    pad_state.triangle = pad_state.triangle.max(button(keys.pressed(KeyCode::KeyF)));
    pad_state.cross = pad_state.cross.max(button(
        keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
    ));
    // The stock PC binding is PED_FIREWEAPON (CTRL/LMB). In the unarmed
    // state re3 routes the same CPad weapon edge into StartFightAttack.
    pad_state.circle = pad_state.circle.max(button(
        keys.pressed(KeyCode::ControlLeft)
            || keys.pressed(KeyCode::ControlRight)
            || mouse_buttons.pressed(MouseButton::Left),
    ));
    arena.test_pad.Update(pad_state);
    let test_pad = arena.test_pad;
    arena.camera_yaw -= f32::from(test_pad.look_around_left_right()) / 128.0 * 1.5 * dt;
    arena.camera_pitch = wrap_camera_angle(
        arena.camera_pitch - f32::from(test_pad.look_around_up_down()) / 128.0 * 0.9 * dt,
    );

    let enter_exit = keys.just_pressed(KeyCode::KeyF)
        || test_pad.exit_vehicle_just_down()
        || arena.request_enter_exit;
    arena.request_enter_exit = false;
    if arena.vehicle_transition.is_some() {
        let status = advance_player_one_vehicle_transition(
            &mut arena,
            &mut primary,
            &mut players,
            &mut vehicles,
            original.data.as_ref(),
            dt,
            &test_pad,
            timer.as_deref_mut(),
        );
        if let Some(status) = status {
            arena.status = status;
        }
        return;
    }
    if enter_exit {
        begin_player_one_vehicle_transition(
            &mut arena,
            &mut primary,
            &mut players,
            &mut vehicles,
            original.data.as_ref(),
        );
        if arena.vehicle_transition.is_some() {
            return;
        }
    }

    if let Some(controlled) = arena.controlled_vehicle {
        let Ok((_, mut transform, mut vehicle, automobile)) = vehicles.get_mut(controlled) else {
            arena.controlled_vehicle = None;
            return;
        };
        if let Some(automobile) = automobile {
            let mut automobile = automobile.into_view();
            let Ok(mut history) = histories.get_mut(controlled) else {
                warn_once!("arena_vehicle_control_missing_collision_history");
                return;
            };
            let world_data = arena_world_map_active
                .then(|| original.data.as_ref())
                .flatten();
            let time_ms = timer.as_deref().map(CTimer::time_in_milliseconds);
            refresh_arena_suspension_model(&mut automobile, original.data.as_ref());
            let damage_nodes = original.data.as_ref().and_then(|data|
                rustgta3_game::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(
                    &data.file_loader, automobile.car.vehicle.core.physical.entity.model_index));
            let telemetry = step_ported_automobile(
                &mut automobile,
                &mut history,
                &mut random.0,
                Some(&keys),
                &test_pad,
                dt,
                world_data,
                time_ms,
                damage_nodes,
            );
            *transform = telemetry.transform;
            vehicle.speed = telemetry.speed_mps;
            vehicle.steering = automobile.car.vehicle.drive.steer_input;
            if let Ok((_, mut player_transform, _, mut player_visibility)) = players.single_mut() {
                player_transform.translation = original
                    .data
                    .as_ref()
                    .and_then(|data| {
                        automobile_driver_world_position(&data.file_loader, &automobile.car)
                    })
                    .map(CVector::to_bevy)
                    .unwrap_or(telemetry.transform.translation);
                player_transform.rotation = telemetry.transform.rotation;
                *player_visibility = Visibility::Visible;
            }
            if let Some(mut player) = primary.get_mut(arena.player_entity) {
                let seated = original
                    .data
                    .as_ref()
                    .and_then(|data| {
                        automobile_driver_world_position(&data.file_loader, &automobile.car)
                    })
                    .unwrap_or_else(|| CVector::from_bevy(telemetry.transform.translation));
                player.set_position(seated);
                player.set_heading_degrees(
                    telemetry
                        .transform
                        .rotation
                        .to_euler(EulerRot::YXZ)
                        .0
                        .to_degrees(),
                );
                player.ped.ped_state = PedState::PED_DRIVING;
            }
            arena.status = format!(
                "CAutomobile model {} | {:>5.1} m/s | gear {} | F exits",
                vehicle.model_index, telemetry.speed_mps, automobile.car.vehicle.drive.current_gear
            );
            return;
        }
        // A vehicle without HANDLING.CFG + CAutomobile is not driveable.
        // Never disguise an asset/runtime failure with arcade fallback motion.
        arena.controlled_vehicle = None;
        vehicle.occupied = false;
        arena.status = format!(
            "Vehicle model {} cannot be driven: CAutomobile runtime missing",
            vehicle.model_index
        );
        return;
    }

    let Ok((_, mut transform, mut player, mut visibility)) = players.single_mut() else {
        return;
    };
    // Collision is resolved after control. Keep the pre-control pose so the
    // static COL solver can catch a thin wall crossed by one render frame.
    player.previous_position = transform.translation;
    *visibility = Visibility::Visible;
    let camera_heading = gta_camera_orientation(arena.camera_yaw);
    let vehicle_colliders = arena_vehicle_collision_snapshots(&vehicles);

    let mut runtime_animation = None;
    let controlled_vehicle = arena.controlled_vehicle.is_some();
    if let Some(mut runtime) = primary.get_mut(arena.player_entity) {
        if player.grounded && runtime.ped.flags.is_in_the_air {
            runtime.ped.flags.is_in_the_air = false;
            runtime.ped.flags.is_standing = true;
            runtime.ped.ped_state = PedState::PED_IDLE;
            runtime.jump_association = None;
            runtime.jump_animation_time = 0.0;
            runtime.ped.physical.move_speed = CVector::ZERO;
            player.horizontal_velocity = Vec2::ZERO;
            player.vertical_velocity = 0.0;
        }
        if let Some(data) = original.data.as_ref() {
            let attack_edge = test_pad.weapon_just_down();
            if runtime.ped.ped_state == PedState::PED_FIGHT {
                if test_pad.get_weapon() {
                    runtime.ped.fight_button_pressure =
                        test_pad.new_state.circle.clamp(0, 255) as u8;
                }
            } else if attack_edge
                && runtime.ped.current_weapon
                    == rustgta3_game::weapons::WeaponType::eWeaponType::WEAPONTYPE_UNARMED
                && runtime.move_speed < 1.0
            {
                start_arena_fight(&mut runtime.as_mut(),
                    data,
                    controlled_vehicle,
                    timer.as_deref().map_or(0, CTimer::time_in_milliseconds),
                );
            }
        }
        let fighting_before_step = runtime.ped.ped_state == PedState::PED_FIGHT;
        let old_position = runtime.position();
        let jump_owns_movement = runtime.ped.ped_state == PedState::PED_JUMP
            || runtime.ped.flags.is_in_the_air
            || runtime.ped.flags.is_landing;
        let mut previous_animation_world_delta = CVector::ZERO;
        if player.grounded && !jump_owns_movement && !fighting_before_step {
            let gta_time_step = dt * 50.0;
            let _ = prepare_player_physical_for_control(&mut runtime.ped);
            apply_player_ped_air_resistance(&mut runtime.ped, dt);
            let mut moved_native = runtime.ped.calculate_new_velocity(&mut runtime.locomotion, &runtime.vitality, gta_time_step, true);
            runtime.mouse_leg_turn_angle = None;
            if runtime.ped.can_strafe_or_mouse_control() {
                moved_native =
                    apply_player_mouse_movement_direction(&mut runtime.as_mut(), &test_pad, moved_native);
            }
            runtime
                .ped
                .physical
                .entity
                .set_heading(runtime.locomotion.rotation_cur);
            previous_animation_world_delta = CVector::new(
                moved_native.x * gta_time_step,
                moved_native.y * gta_time_step,
                0.0,
            );
        }
        let _moved = if fighting_before_step {
            // `PlayerControlFighter` changes heading and records
            // m_takeAStepAfterAttack; Fight() owns root motion until the
            // association finishes, so a running punch never loses its
            // authored attack pose to locomotion.
            let walk_x = test_pad.get_ped_walk_left_right() as f32;
            let walk_y = test_pad.get_ped_walk_up_down() as f32;
            let pad_move = walk_x.hypot(walk_y);
            if pad_move > 0.0 {
                runtime.locomotion.rotation_dest = CGeneral::limit_radian_angle(
                    CGeneral::get_radian_angle_between_points(0.0, 0.0, -walk_x, walk_y)
                        - camera_heading,
                );
                runtime.ped.take_step_after_attack = pad_move > 2.0 * 60.0;
                let needed = CGeneral::limit_radian_angle(
                    runtime.locomotion.rotation_dest - runtime.locomotion.rotation_cur,
                );
                runtime.locomotion.rotation_cur = CGeneral::limit_radian_angle(
                    runtime.locomotion.rotation_cur
                        + needed.clamp(
                            -runtime.locomotion.heading_rate.to_radians() * dt * 50.0 * 0.25,
                            runtime.locomotion.heading_rate.to_radians() * dt * 50.0 * 0.25,
                        ),
                );
                runtime
                    .ped
                    .physical
                    .entity
                    .set_heading(runtime.locomotion.rotation_cur);
            }
            runtime.move_speed = 0.0;
            runtime.ped.move_state = eMoveState::PEDMOVE_NONE;
            false
        } else {
            runtime.apply_on_foot_mouse_runaround_input_camera(
                &test_pad,
                dt,
                true,
                Some(camera_heading),
                false,
                false,
            )
        };
        let local_delta = if player.grounded && !jump_owns_movement && !fighting_before_step {
            match (original.data.as_ref(), timer.as_deref_mut()) {
                (Some(data), Some(timer)) => update_player_movement_animation(&mut runtime.as_mut(),
                    &data.anim_manager,
                    &data.file_loader,
                    dt,
                    true,
                    false,
                    timer.time_in_milliseconds(),
                    timer,
                ),
                _ => CVector::ZERO,
            }
        } else {
            CVector::ZERO
        };
        runtime.stage_anim_move_delta(Some(local_delta));
        if player.grounded
            && !jump_owns_movement
            && !fighting_before_step
            && previous_animation_world_delta.magnitude_sqr() > 0.0
        {
            let mut position = runtime.position();
            position.x += previous_animation_world_delta.x;
            position.y += previous_animation_world_delta.y;
            if let Some(data) = original.data.as_ref() {
                position = resolve_arena_player_vehicle_sweep(
                    old_position,
                    position,
                    &runtime.ped,
                    &vehicle_colliders,
                    data,
                );
            }
            runtime.set_position(position);
        }
        let current_position = runtime.position();
        if dt > 0.0
            && ((current_position.x - old_position.x).abs() > 1.0e-6
                || (current_position.y - old_position.y).abs() > 1.0e-6)
        {
            let gta_velocity = CVector::new(
                (current_position.x - old_position.x) / dt,
                (current_position.y - old_position.y) / dt,
                0.0,
            );
            runtime.ped.physical.move_speed.x = gta_velocity.x;
            runtime.ped.physical.move_speed.y = gta_velocity.y;
            player.horizontal_velocity = Vec2::new(gta_velocity.x, -gta_velocity.y);
        } else if player.grounded && !jump_owns_movement {
            runtime.ped.physical.move_speed.x = 0.0;
            runtime.ped.physical.move_speed.y = 0.0;
            player.horizontal_velocity = Vec2::ZERO;
        }
        let height = transform.translation.y;
        transform.translation = runtime.position().to_bevy();
        transform.translation.y = height;
        transform.rotation = Quat::from_rotation_y(runtime.locomotion.rotation_cur);
        if player.grounded && !jump_owns_movement && !fighting_before_step {
            runtime_animation = Some(match runtime.ped.move_state {
                eMoveState::PEDMOVE_SPRINT => ArenaAnimation::Sprint,
                eMoveState::PEDMOVE_RUN => ArenaAnimation::Run,
                eMoveState::PEDMOVE_WALK => ArenaAnimation::Walk,
                _ => ArenaAnimation::Idle,
            });
        } else if fighting_before_step {
            runtime_animation = Some(ArenaAnimation::Fight);
        }
        if let Some(data) = original.data.as_ref()
            && runtime.ped.ped_state == PedState::PED_FIGHT
        {
            let target = player_two_targets.iter_mut().next();
            if let Some((target_transform, target)) = target {
                let mut target = target.into_view();
                let target_position = CVector::from_bevy(target_transform.translation);
                advance_arena_fight(&mut runtime.as_mut(),
                    &test_pad,
                    dt,
                    timer.as_deref().map(CTimer::time_in_milliseconds),
                    data,
                    Some(target_position),
                    Some(&mut target.runtime.as_mut()),
                );
            } else {
                advance_arena_fight(&mut runtime.as_mut(), &test_pad, dt,
                    timer.as_deref().map(CTimer::time_in_milliseconds), data, None, None);
            }
            if runtime.ped.ped_state != PedState::PED_FIGHT {
                runtime_animation = Some(ArenaAnimation::Idle);
            }
        }
    } else if player.grounded
        && matches!(
            arena.animation,
            ArenaAnimation::Walk | ArenaAnimation::Run | ArenaAnimation::Sprint
        )
    {
        arena.animation = ArenaAnimation::Idle;
    }
    if let Some(animation) = runtime_animation {
        arena.animation = animation;
    }

    if (keys.just_pressed(KeyCode::Space) || test_pad.jump_just_down()) && player.grounded {
        if let (Some(mut runtime), Some(data)) = (primary.get_mut(arena.player_entity), original.data.as_ref()) {
            begin_player_jump(&mut runtime.as_mut(), &data.anim_manager);
            player.horizontal_velocity = Vec2::new(
                runtime.ped.physical.move_speed.x,
                -runtime.ped.physical.move_speed.y,
            );
            player.vertical_velocity = 0.0;
            arena.animation = ArenaAnimation::Jump;
        } else {
            arena.status = "Jump unavailable: original PED.IFP runtime is not loaded".to_owned();
        }
    }

    let mut launched_this_frame = false;
    let launch_pending = primary.get(arena.player_entity).is_some_and(|runtime| {
        runtime.ped.ped_state == PedState::PED_JUMP
            && !runtime.ped.flags.is_in_the_air
            && runtime.jump_association.is_some()
    });
    if launch_pending {
        arena.animation = ArenaAnimation::Jump;
        if let (Some(mut runtime), Some(data)) = (primary.get_mut(arena.player_entity), original.data.as_ref()) {
            if let Some(velocity) = advance_player_jump_launch(&mut runtime.as_mut(), &data.anim_manager, dt) {
                player.horizontal_velocity = Vec2::new(velocity.x, -velocity.y);
                player.vertical_velocity = velocity.z;
                player.grounded = false;
                launched_this_frame = true;
            }
        }
    }
    if !player.grounded {
        if !launched_this_frame && let Some(mut runtime) = primary.get_mut(arena.player_entity) {
            let _ = update_player_jump_animation(&mut runtime.as_mut(), dt);
        }
        transform.translation.x += player.horizontal_velocity.x * dt;
        transform.translation.z += player.horizontal_velocity.y * dt;
        // Production rustgta3 integrates re3's .008 gravity in 50 Hz GTA
        // time units, i.e. 20 world units/s². Using 7.2 here made arena jumps
        // hang in the air far longer than the real game.
        player.vertical_velocity -= 20.0 * dt;
        transform.translation.y += player.vertical_velocity * dt;
        arena.animation = ArenaAnimation::Jump;
        // The diagnostic grid is a zero-height floor. Once an original GTA
        // district is loaded, CWorld's authored COL ground is resolved by
        // `resolve_arena_static_collisions` below; clamping here would place
        // the ped beneath every real road/bridge after a teleport.
        if !arena_world_map_active && transform.translation.y <= crate::ARENA_PED_BASE_OFFSET {
            transform.translation.y = crate::ARENA_PED_BASE_OFFSET;
            player.horizontal_velocity = Vec2::ZERO;
            player.vertical_velocity = 0.0;
            player.grounded = true;
            if let Some(mut runtime) = primary.get_mut(arena.player_entity) {
                runtime.ped.flags.is_in_the_air = false;
                runtime.ped.flags.is_standing = true;
                runtime.ped.ped_state = PedState::PED_IDLE;
                runtime.jump_association = None;
                runtime.jump_animation_time = 0.0;
                runtime.ped.physical.move_speed = CVector::ZERO;
            }
            arena.animation = ArenaAnimation::Idle;
        }
    }
    if let Some(mut runtime) = primary.get_mut(arena.player_entity) {
        runtime.set_position(CVector::from_bevy(transform.translation));
        runtime.ped.physical.move_speed = CVector::new(
            player.horizontal_velocity.x,
            -player.horizontal_velocity.y,
            player.vertical_velocity,
        );
    }
}

pub(crate) fn control_arena_player_two(
    time: Res<Time>,
    gamepads: Query<(Entity, &Gamepad)>,
    session: Res<LocalCoopSession>,
    mut arena: ResMut<DeveloperArena>,
    original: Res<ArenaOriginalRuntime>,
    clock: ArenaControlClock,
    mut players: Query<
        (&mut Transform, crate::player_access::ArenaCoopActor, &mut Visibility),
        Without<ArenaVehicle>,
    >,
    arena_world_maps: Query<(), With<ArenaWorldMap>>,
    mut histories: Query<&mut VehicleCollisionState>,
    mut vehicles: Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
) {
    let ArenaControlClock { mut timer, mut random, mut primary } = clock;
    if !session.enabled {
        return;
    }
    let Ok((mut transform, player_two, mut visibility)) = players.single_mut() else {
        return;
    };
    let mut player_two = player_two.into_view();
    player_two.previous_position = transform.translation;
    let mut connected = gamepads.iter().collect::<Vec<_>>();
    connected.sort_unstable_by_key(|(entity, _)| entity.to_bits());
    let state = connected
        .get(1)
        .map(|(_, gamepad)| {
            controller_state_from_gamepad(gamepad, player_two.controlled_vehicle.is_some())
        })
        .unwrap_or_default();
    player_two.pad.Update(state);
    let pad = player_two.pad;
    let dt = time.delta_secs().min(0.05);
    let arena_world_map_active = !arena_world_maps.is_empty();
    player_two.camera_yaw -= f32::from(pad.look_around_left_right()) / 128.0 * 1.5 * dt;
    player_two.camera_pitch = wrap_camera_angle(
        player_two.camera_pitch - f32::from(pad.look_around_up_down()) / 128.0 * 0.9 * dt,
    );

    if pad.exit_vehicle_just_down() {
        begin_player_two_vehicle_transition(
            &mut player_two,
            &mut transform,
            &mut visibility,
            &mut vehicles,
            original.data.as_ref(),
        );
    }
    if player_two.vehicle_transition.is_some() {
        advance_player_two_vehicle_transition(
            &mut player_two,
            &mut transform,
            &mut visibility,
            &mut vehicles,
            original.data.as_ref(),
            dt,
            &pad,
            timer.as_deref_mut(),
        );
        return;
    }

    if let Some(controlled) = player_two.controlled_vehicle {
        let Ok((_, mut vehicle_transform, mut vehicle, automobile)) = vehicles.get_mut(controlled)
        else {
            player_two.controlled_vehicle = None;
            *visibility = Visibility::Visible;
            return;
        };
        let Some(automobile) = automobile else {
            player_two.controlled_vehicle = None;
            vehicle.occupied = false;
            *visibility = Visibility::Visible;
            arena.status = format!(
                "P2 cannot drive vehicle model {}: CAutomobile runtime missing",
                vehicle.model_index
            );
            return;
        };
        let mut automobile = automobile.into_view();
        let world_data = arena_world_map_active
            .then(|| original.data.as_ref())
            .flatten();
        let time_ms = timer.as_deref().map(CTimer::time_in_milliseconds);
        let Ok(mut history) = histories.get_mut(controlled) else {
            warn_once!("arena_coop_vehicle_control_missing_collision_history");
            return;
        };
        let telemetry =
            {
                refresh_arena_suspension_model(&mut automobile, original.data.as_ref());
                let damage_nodes = original.data.as_ref().and_then(|data|
                    rustgta3_game::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(
                        &data.file_loader, automobile.car.vehicle.core.physical.entity.model_index));
                step_ported_automobile(&mut automobile, &mut history, &mut random.0, None, &pad, dt, world_data, time_ms, damage_nodes)
            };
        *vehicle_transform = telemetry.transform;
        vehicle.speed = telemetry.speed_mps;
        vehicle.steering = automobile.car.vehicle.drive.steer_input;
        transform.translation = original
            .data
            .as_ref()
            .and_then(|data| automobile_driver_world_position(&data.file_loader, &automobile.car))
            .map(CVector::to_bevy)
            .unwrap_or(telemetry.transform.translation);
        transform.rotation = telemetry.transform.rotation;
        *visibility = Visibility::Visible;
        player_two
            .runtime
            .set_position(CVector::from_bevy(transform.translation));
        player_two.runtime.set_heading_degrees(
            telemetry
                .transform
                .rotation
                .to_euler(EulerRot::YXZ)
                .0
                .to_degrees(),
        );
        player_two.runtime.ped.ped_state = PedState::PED_DRIVING;
        arena.status = format!(
            "P2 CAutomobile model {} | {:>5.1} m/s | gear {} | Y exits",
            vehicle.model_index, telemetry.speed_mps, automobile.car.vehicle.drive.current_gear
        );
        return;
    }

    *visibility = Visibility::Visible;
    if player_two.grounded && player_two.runtime.ped.flags.is_in_the_air {
        player_two.runtime.ped.flags.is_in_the_air = false;
        player_two.runtime.ped.flags.is_standing = true;
        player_two.runtime.ped.ped_state = PedState::PED_IDLE;
        player_two.runtime.jump_association = None;
        player_two.runtime.jump_animation_time = 0.0;
        player_two.runtime.ped.physical.move_speed = CVector::ZERO;
        player_two.horizontal_velocity = Vec2::ZERO;
        player_two.vertical_velocity = 0.0;
    }

    let controlled_vehicle = player_two.controlled_vehicle.is_some();
    if let Some(data) = original.data.as_ref() {
        let attack_edge = pad.weapon_just_down();
        if player_two.runtime.ped.ped_state == PedState::PED_FIGHT {
            if pad.get_weapon() {
                player_two.runtime.ped.fight_button_pressure =
                    pad.new_state.circle.clamp(0, 255) as u8;
            }
        } else if attack_edge
            && player_two.runtime.ped.current_weapon
                == rustgta3_game::weapons::WeaponType::eWeaponType::WEAPONTYPE_UNARMED
            && player_two.runtime.move_speed < 1.0
        {
            start_arena_fight(&mut player_two.runtime.as_mut(),
                data,
                controlled_vehicle,
                timer.as_deref().map_or(0, CTimer::time_in_milliseconds),
            );
        }
    }

    let camera_heading = gta_camera_orientation(player_two.camera_yaw);
    let vehicle_colliders = arena_vehicle_collision_snapshots(&vehicles);
    let old_position = player_two.runtime.position();
    let fighting_before_step = player_two.runtime.ped.ped_state == PedState::PED_FIGHT;
    let jump_owns_movement = player_two.runtime.ped.ped_state == PedState::PED_JUMP
        || player_two.runtime.ped.flags.is_in_the_air
        || player_two.runtime.ped.flags.is_landing;
    let mut previous_animation_world_delta = CVector::ZERO;
    if player_two.grounded && !jump_owns_movement && !fighting_before_step {
        let gta_time_step = dt * 50.0;
        let _ = prepare_player_physical_for_control(&mut player_two.runtime.ped);
        apply_player_ped_air_resistance(&mut player_two.runtime.ped, dt);
        let moved_native = {
            let runtime = &mut player_two.runtime;
            runtime.ped.calculate_new_velocity(&mut runtime.locomotion, &runtime.vitality, gta_time_step, true)
        };
        let heading = player_two.runtime.locomotion.rotation_cur;
        player_two.runtime.ped.physical.entity.set_heading(heading);
        previous_animation_world_delta = CVector::new(
            moved_native.x * gta_time_step,
            moved_native.y * gta_time_step,
            0.0,
        );
    }
    let _moved = if fighting_before_step {
        let walk_x = pad.get_ped_walk_left_right() as f32;
        let walk_y = pad.get_ped_walk_up_down() as f32;
        let pad_move = walk_x.hypot(walk_y);
        if pad_move > 0.0 {
            player_two.runtime.locomotion.rotation_dest = CGeneral::limit_radian_angle(
                CGeneral::get_radian_angle_between_points(0.0, 0.0, -walk_x, walk_y)
                    - camera_heading,
            );
            player_two.runtime.ped.take_step_after_attack = pad_move > 2.0 * 60.0;
            let needed = CGeneral::limit_radian_angle(
                player_two.runtime.locomotion.rotation_dest - player_two.runtime.locomotion.rotation_cur,
            );
            player_two.runtime.locomotion.rotation_cur = CGeneral::limit_radian_angle(
                player_two.runtime.locomotion.rotation_cur
                    + needed.clamp(
                        -player_two.runtime.locomotion.heading_rate.to_radians() * dt * 50.0 * 0.25,
                        player_two.runtime.locomotion.heading_rate.to_radians() * dt * 50.0 * 0.25,
                    ),
            );
            let heading = player_two.runtime.locomotion.rotation_cur;
            player_two.runtime.ped.physical.entity.set_heading(heading);
        }
        player_two.runtime.move_speed = 0.0;
        player_two.runtime.ped.move_state = eMoveState::PEDMOVE_NONE;
        false
    } else {
        player_two.runtime.apply_on_foot_pad_input_camera(
            &pad,
            dt,
            true,
            Some(camera_heading),
            false,
            false,
        )
    };
    let local_delta = if player_two.grounded && !jump_owns_movement && !fighting_before_step {
        match (original.data.as_ref(), timer.as_deref_mut()) {
            (Some(data), Some(timer)) => update_player_movement_animation(&mut player_two.runtime.as_mut(),
                &data.anim_manager,
                    &data.file_loader,
                dt,
                true,
                false,
                timer.time_in_milliseconds(),
                timer,
            ),
            _ => CVector::ZERO,
        }
    } else {
        CVector::ZERO
    };
    player_two.runtime.stage_anim_move_delta(Some(local_delta));
    if player_two.grounded
        && !jump_owns_movement
        && !fighting_before_step
        && previous_animation_world_delta.magnitude_sqr() > 0.0
    {
        let mut position = player_two.runtime.position();
        position.x += previous_animation_world_delta.x;
        position.y += previous_animation_world_delta.y;
        if let Some(data) = original.data.as_ref() {
            position = resolve_arena_player_vehicle_sweep(
                old_position,
                position,
                &player_two.runtime.ped,
                &vehicle_colliders,
                data,
            );
        }
        player_two.runtime.set_position(position);
    }
    let current_position = player_two.runtime.position();
    if dt > 0.0
        && ((current_position.x - old_position.x).abs() > 1.0e-6
            || (current_position.y - old_position.y).abs() > 1.0e-6)
    {
        let velocity = Vec2::new(
            (current_position.x - old_position.x) / dt,
            -(current_position.y - old_position.y) / dt,
        );
        player_two.horizontal_velocity = velocity;
        player_two.runtime.ped.physical.move_speed.x = velocity.x;
        player_two.runtime.ped.physical.move_speed.y = -velocity.y;
    } else if player_two.grounded && !jump_owns_movement {
        player_two.horizontal_velocity = Vec2::ZERO;
        player_two.runtime.ped.physical.move_speed.x = 0.0;
        player_two.runtime.ped.physical.move_speed.y = 0.0;
    }

    let height = transform.translation.y;
    transform.translation = player_two.runtime.position().to_bevy();
    transform.translation.y = height;
    transform.rotation = Quat::from_rotation_y(player_two.runtime.locomotion.rotation_cur);

    if fighting_before_step {
        if let Some(data) = original.data.as_ref() {
            let target_position = primary.get(arena.player_entity).map(|player| player.position());
            if let Some(target_position) = target_position {
                let mut target_view = primary.get_mut(arena.player_entity);
                advance_arena_fight(&mut player_two.runtime.as_mut(),
                    &pad,
                    dt,
                    timer.as_deref().map(CTimer::time_in_milliseconds),
                    data,
                    Some(target_position),
                    target_view.as_mut(),
                );
            } else {
                advance_arena_fight(&mut player_two.runtime.as_mut(), &pad, dt,
                    timer.as_deref().map(CTimer::time_in_milliseconds), data, None, None);
            }
        }
    }

    if pad.jump_just_down()
        && player_two.grounded
        && let Some(data) = original.data.as_ref()
    {
        begin_player_jump(&mut player_two.runtime.as_mut(), &data.anim_manager);
        player_two.horizontal_velocity = Vec2::new(
            player_two.runtime.ped.physical.move_speed.x,
            -player_two.runtime.ped.physical.move_speed.y,
        );
        player_two.vertical_velocity = 0.0;
    }

    let launch_pending = player_two.runtime.ped.ped_state == PedState::PED_JUMP
        && !player_two.runtime.ped.flags.is_in_the_air
        && player_two.runtime.jump_association.is_some();
    let mut launched_this_frame = false;
    if launch_pending
        && let Some(data) = original.data.as_ref()
        && let Some(velocity) =
            advance_player_jump_launch(&mut player_two.runtime.as_mut(), &data.anim_manager, dt)
    {
        player_two.horizontal_velocity = Vec2::new(velocity.x, -velocity.y);
        player_two.vertical_velocity = velocity.z;
        player_two.grounded = false;
        launched_this_frame = true;
    }

    if !player_two.grounded {
        if !launched_this_frame {
            let _ = update_player_jump_animation(&mut player_two.runtime.as_mut(), dt);
        }
        transform.translation.x += player_two.horizontal_velocity.x * dt;
        transform.translation.z += player_two.horizontal_velocity.y * dt;
        player_two.vertical_velocity -= 20.0 * dt;
        transform.translation.y += player_two.vertical_velocity * dt;
        if !arena_world_map_active && transform.translation.y <= crate::ARENA_PED_BASE_OFFSET {
            transform.translation.y = crate::ARENA_PED_BASE_OFFSET;
            player_two.horizontal_velocity = Vec2::ZERO;
            player_two.vertical_velocity = 0.0;
            player_two.grounded = true;
            player_two.runtime.ped.flags.is_in_the_air = false;
            player_two.runtime.ped.flags.is_standing = true;
            player_two.runtime.ped.ped_state = PedState::PED_IDLE;
            player_two.runtime.jump_association = None;
            player_two.runtime.jump_animation_time = 0.0;
        }
    }
    player_two
        .runtime
        .set_position(CVector::from_bevy(transform.translation));
    player_two.runtime.ped.physical.move_speed = CVector::new(
        player_two.horizontal_velocity.x,
        -player_two.horizontal_velocity.y,
        player_two.vertical_velocity,
    );
}

/// Advance every arena automobile that is not currently owned by a local
/// player. re3 continues to run `CAutomobile::ProcessControl` for PHYSICS,
/// ABANDONED and WRECKED cars; limiting this to the controlled entity made an
/// exited or collision-pushed arena car freeze on the exact next frame.
pub(crate) fn step_uncontrolled_arena_automobiles(
    time: Res<Time>,
    arena: Res<DeveloperArena>,
    original: Res<ArenaOriginalRuntime>,
    player_two: Query<&ArenaCoopPlayerTwo>,
    arena_world_maps: Query<(), With<ArenaWorldMap>>,
    timer: Option<Res<CTimer>>,
    mut random: ResMut<crate::ArenaRandomState>,
    mut vehicles: Query<(
        Entity,
        &mut Transform,
        &mut ArenaVehicle,
        ArenaAutomobileRuntime,
        &mut VehicleCollisionState,
    )>,
) {
    let dt = time.delta_secs().clamp(0.0, 0.05);
    let time_step = dt * 50.0;
    let second_controlled = player_two
        .single()
        .ok()
        .and_then(|player| player.controlled_vehicle);
    let world_data = (!arena_world_maps.is_empty())
        .then(|| original.data.as_ref())
        .flatten();
    let now_ms = timer.as_deref().map(CTimer::time_in_milliseconds);

    for (entity, mut transform, mut vehicle, runtime, mut history) in &mut vehicles {
        let mut runtime = runtime.into_view();
        if arena.controlled_vehicle == Some(entity) || second_controlled == Some(entity) {
            continue;
        }
        if runtime.car.vehicle.physical.entity.status == eEntityStatus::STATUS_PLAYER {
            runtime
                .car
                .vehicle
                .set_status(eEntityStatus::STATUS_ABANDONED);
        }
        vehicle.occupied = false;
        runtime.car.vehicle.drive.gas_pedal = 0.0;
        refresh_arena_suspension_model(&mut runtime, original.data.as_ref());

        let contacts = world_data.map_or_else(
            || flat_arena_automobile_contacts(runtime.car.as_ref(), runtime.handling, runtime.wheel_scale),
            |data| automobile_road_contacts(&runtime.car, &data.file_loader, dt * 50.0),
        );
        let status = runtime.car.vehicle.physical.entity.status;
        let (brake, handbrake) = match status {
            eEntityStatus::STATUS_WRECKED => (0.05 * 255.0, true),
            eEntityStatus::STATUS_PLAYER_DISABLED => (255.0, true),
            _ => (0.2 * 255.0, false),
        };
        let surface_adhesion = runtime.surface_adhesion;
        let wheel_friction = runtime.wheel_friction;
        let collision_damage_multiplier = runtime.handling.collision_damage_multiplier;
        let ArenaAutomobileView {
            car,
            handling,
            ..
        } = &mut runtime;
        history.previous_matrix = car.vehicle.physical.entity.matrix;
        let damage_nodes = original.data.as_ref().and_then(|data| rustgta3_game::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(&data.file_loader, car.vehicle.core.physical.entity.model_index));
        car.process_player_drive_step(
            CAutomobileDriveStepInput {
                control: CAutomobileControlInput {
                    steering_left_right: 0.0,
                    mouse_x: 0.0,
                    mouse_steering: false,
                    accelerate: 0.0,
                    brake,
                    handbrake,
                    exit_vehicle: false,
                    controls_disabled: false,
                },
                local_contact_points: contacts.local_points,
                contact_points: contacts.relative_world_points,
                spring_ratios: contacts.spring_ratios,
                spring_directions: contacts.spring_directions,
                adhesive_limits: [surface_adhesion; NUM_CAR_WHEELS],
                grounded: contacts.grounded,
                collision_entities: contacts.collision_entities,
                collision_time_ms: now_ms.unwrap_or(0),
                wheel_scale: contacts.wheel_scale,
                time_step,
                time_step_seconds: dt,
            },
            handling,
            wheel_friction,
            &mut random.0,
        damage_nodes,
        );
        if let Some(data) = world_data {
            automobile_static_collision_step(
                car,
                history.previous_matrix,
                None,
                &data.file_loader,
                data.file_loader.renderer.camera_position,
                &data.surface_table,
                time_step,
                now_ms,
                collision_damage_multiplier,
                &mut random.0,
                None,
            );
        }
        car.vehicle.physical.distance_travelled =
            car.vehicle.physical.move_speed.magnitude() * time_step;
        let telemetry = automobile_telemetry(&car.as_ref());
        *transform = telemetry.transform;
        vehicle.speed = telemetry.speed_mps;
        vehicle.steering = car.vehicle.drive.steer_input;
    }
}

/// Apply the authored GTA collision boxes of spawned props to the arena
/// actors. This keeps the diagnostic space useful for jump/obstacle tests
/// without introducing a second proxy physics implementation.
pub(crate) fn resolve_arena_static_collisions(
    time: Res<Time>,
    timer: Option<Res<CTimer>>,
    mut arena: ResMut<DeveloperArena>,
    original: Res<ArenaOriginalRuntime>,
    mut primary: crate::player_access::ArenaPrimaryPlayers,
    mut players: Query<
        (&mut Transform, &mut ArenaPlayer),
        (With<ArenaPlayer>, Without<ArenaVehicle>, Without<ArenaProp>),
    >,
    mut player_two: Query<
        (&mut Transform, crate::player_access::ArenaCoopActor),
        (
            Without<ArenaVehicle>,
            Without<ArenaPlayer>,
            Without<ArenaProp>,
        ),
    >,
    mut vehicles: Query<
        (&mut Transform, &ArenaVehicle),
        (With<ArenaVehicle>, Without<ArenaPlayer>, Without<ArenaProp>),
    >,
    props: Query<
        (&Transform, &ArenaProp),
        (
            With<ArenaProp>,
            Without<ArenaPlayer>,
            Without<ArenaVehicle>,
            Without<ArenaCoopPlayerTwo>,
        ),
    >,
    arena_world_maps: Query<(), With<ArenaWorldMap>>,
) {
    let Some(data) = original.data.as_ref() else {
        return;
    };
    if !arena_world_maps.is_empty() {
        let time_step = time.delta_secs().clamp(0.0, 0.05) * 50.0;
        let now_ms = timer.as_deref().map_or(0, CTimer::time_in_milliseconds);
        // `CStreaming::LoadScene` populated this isolated CWorld with the
        // selected district's original COL models. The render subtree is only
        // a Bevy projection of that same world; collision must query COL, not
        // broad-phase boxes made from render meshes.
        for (mut transform, mut player) in &mut players {
            if arena
                .vehicle_transition
                .as_ref()
                .is_some_and(|transition| !transition.seeking)
            {
                continue;
            }
            let Some(mut runtime) = primary.get_mut(arena.player_entity) else {
                continue;
            };
            let (resolved, grounded) = resolve_arena_world_ped_position(
                player.previous_position,
                transform.translation,
                player.vertical_velocity,
                &mut runtime.ped,
                data,
                time_step,
                now_ms,
            );
            transform.translation = resolved;
            player.grounded = grounded;
            if grounded {
                player.vertical_velocity = 0.0;
            }
            let mut finished_jump = false;
            {
                if grounded && (runtime.ped.flags.is_in_the_air || runtime.ped.flags.is_landing) {
                    // Re3 clears the launch/glide association at the ground
                    // contact boundary. Keeping PED_JUMP with no association
                    // would make the Arena relaunch every frame.
                    runtime.ped.ped_state = PedState::PED_IDLE;
                    runtime.ped.move_state = eMoveState::PEDMOVE_STILL;
                    runtime.ped.flags.is_in_the_air = false;
                    runtime.ped.flags.is_landing = false;
                    runtime.ped.flags.is_standing = true;
                    runtime.jump_association = None;
                    runtime.jump_animation_time = 0.0;
                    finished_jump = true;
                }
                runtime.set_position(CVector::from_bevy(resolved));
                runtime.ped.flags.is_in_the_air = !grounded;
                runtime.ped.flags.is_standing = grounded;
                runtime.ped.physical.move_speed = CVector::new(
                    player.horizontal_velocity.x,
                    -player.horizontal_velocity.y,
                    player.vertical_velocity,
                );
            }
            if finished_jump {
                arena.animation = ArenaAnimation::Idle;
                arena.runtime_animation_selection = None;
            }
        }
        for (mut transform, player) in &mut player_two {
            let mut player = player.into_view();
            if player
                .vehicle_transition
                .as_ref()
                .is_some_and(|transition| !transition.seeking)
            {
                continue;
            }
            let (resolved, grounded) = resolve_arena_world_ped_position(
                player.previous_position,
                transform.translation,
                player.vertical_velocity,
                &mut player.runtime.ped,
                data,
                time_step,
                now_ms,
            );
            transform.translation = resolved;
            player.grounded = grounded;
            if grounded {
                player.vertical_velocity = 0.0;
            }
            if grounded
                && (player.runtime.ped.flags.is_in_the_air || player.runtime.ped.flags.is_landing)
            {
                player.runtime.ped.ped_state = PedState::PED_IDLE;
                player.runtime.ped.move_state = eMoveState::PEDMOVE_STILL;
                player.runtime.ped.flags.is_in_the_air = false;
                player.runtime.ped.flags.is_landing = false;
                player.runtime.ped.flags.is_standing = true;
                player.runtime.jump_association = None;
                player.runtime.jump_animation_time = 0.0;
            }
            player.runtime.set_position(CVector::from_bevy(resolved));
            player.runtime.ped.flags.is_in_the_air = !grounded;
            player.runtime.ped.flags.is_standing = grounded;
            player.runtime.ped.physical.move_speed = CVector::new(
                player.horizontal_velocity.x,
                -player.horizontal_velocity.y,
                player.vertical_velocity,
            );
        }
        return;
    }
    let obstacles = props
        .iter()
        .filter_map(|(transform, prop)| {
            let col = data.file_loader.model_info.col_model(prop.model_index)?;
            let bounds = &col.bounding_box;
            Some((
                Vec3::new(
                    transform.translation.x + bounds.min.x,
                    transform.translation.y + bounds.min.z,
                    transform.translation.z + bounds.min.y,
                ),
                Vec3::new(
                    transform.translation.x + bounds.max.x,
                    transform.translation.y + bounds.max.z,
                    transform.translation.z + bounds.max.y,
                ),
            ))
        })
        .collect::<Vec<_>>();

    for (mut transform, mut player) in &mut players {
        if arena
            .vehicle_transition
            .as_ref()
            .is_some_and(|transition| !transition.seeking)
        {
            continue;
        }
        let radius = 0.38;
        let feet_before = transform.translation.y - crate::ARENA_PED_BASE_OFFSET;
        for (min, max) in &obstacles {
            let inside_x = transform.translation.x > min.x - radius
                && transform.translation.x < max.x + radius;
            let inside_z = transform.translation.z > min.z - radius
                && transform.translation.z < max.z + radius;
            if !inside_x || !inside_z {
                continue;
            }
            // A horizontal contact is a wall unless the ped's feet are
            // already at the authored top surface. The previous code also
            // assigned max.y here, turning any side bump into an instant climb.
            if feet_before < max.y - 0.25 && transform.translation.y > min.y - 0.2 {
                let dx = (max.x + radius) - transform.translation.x;
                let dz = (max.z + radius) - transform.translation.z;
                let dx2 = transform.translation.x - (min.x - radius);
                let dz2 = transform.translation.z - (min.z - radius);
                let (push_x, push_z) = [
                    (dx.abs(), Vec3::X * dx.signum()),
                    (dx2.abs(), -Vec3::X * dx2.signum()),
                    (dz.abs(), Vec3::Z * dz.signum()),
                    (dz2.abs(), -Vec3::Z * dz2.signum()),
                ]
                .into_iter()
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, direction)| (direction.x, direction.z))
                .unwrap_or((0.0, 0.0));
                if push_x != 0.0 {
                    transform.translation.x += push_x;
                } else {
                    transform.translation.z += push_z;
                }
            }
        }

        // Re-evaluate support after wall sliding. Arena road height is zero;
        // a prop top is support only while the feet are actually over its XZ
        // footprint and near/above that top. Walking off a prop starts a fall
        // instead of permanently redefining the arena ground height.
        let mut support_y = 0.0f32;
        for (min, max) in &obstacles {
            let over_top = transform.translation.x >= min.x
                && transform.translation.x <= max.x
                && transform.translation.z >= min.z
                && transform.translation.z <= max.z;
            if over_top && feet_before >= max.y - 0.30 {
                support_y = support_y.max(max.y);
            }
        }
        let target_y = support_y + crate::ARENA_PED_BASE_OFFSET;
        if player.grounded {
            if transform.translation.y > target_y + 0.12 {
                player.grounded = false;
                player.vertical_velocity = player.vertical_velocity.min(0.0);
            } else {
                transform.translation.y = target_y;
                player.vertical_velocity = 0.0;
            }
        } else if player.vertical_velocity <= 0.0
            && transform.translation.y <= target_y + 0.12
            && transform.translation.y >= target_y - 0.35
        {
            transform.translation.y = target_y;
            player.vertical_velocity = 0.0;
            player.grounded = true;
        }

        if let Some(mut runtime) = primary.get_mut(arena.player_entity) {
            runtime.set_position(CVector::from_bevy(transform.translation));
        }
    }

    for (mut transform, _) in &mut vehicles {
        for (min, max) in &obstacles {
            let half = Vec3::new(1.1, 0.8, 2.2);
            if transform.translation.x + half.x < min.x
                || transform.translation.x - half.x > max.x
                || transform.translation.z + half.z < min.z
                || transform.translation.z - half.z > max.z
            {
                continue;
            }
            transform.translation.y = transform.translation.y.max(max.y + half.y);
        }
    }
}

fn arena_vehicle_collision_snapshots(
    vehicles: &Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
) -> Vec<(CMatrix, usize)> {
    vehicles
        .iter()
        .filter_map(|(_, _, _, runtime)| {
            let runtime = runtime?.into_view();
            let vehicle = &runtime.car.vehicle;
            vehicle.physical.entity.flags.uses_collision.then_some((
                vehicle.physical.entity.matrix,
                vehicle.physical.entity.model_index.max(0) as usize,
            ))
        })
        .collect()
}

/// Arena equivalent of the production player's subdivided dynamic-vehicle
/// probe. re3 samples player motion in 0.2-unit increments and restores the
/// last safe matrix when a vehicle contact responds, so a render-frame-sized
/// walk step cannot tunnel through a thin car body.
fn resolve_arena_player_vehicle_sweep(
    start: CVector,
    end: CVector,
    ped: &CPed,
    vehicle_colliders: &[(CMatrix, usize)],
    data: &ArenaOriginalData,
) -> CVector {
    let player_model_index = ped.physical.entity.model_index.max(0) as usize;
    let player_col = data
        .file_loader
        .model_info
        .col_model(player_model_index)
        .unwrap_or(&data.file_loader.temp_col_models.col_model_ped1);
    let delta = CVector::new(end.x - start.x, end.y - start.y, end.z - start.z);
    let distance = delta.magnitude();
    let steps = if distance <= f32::EPSILON {
        1
    } else {
        ((distance / 0.2).ceil() as usize).max(2)
    };
    let mut safe = start;
    let mut player_matrix = ped.physical.entity.matrix;
    for step in 1..=steps {
        let amount = step as f32 / steps as f32;
        let candidate = CVector::new(
            start.x + delta.x * amount,
            start.y + delta.y * amount,
            start.z + delta.z * amount,
        );
        player_matrix.position = candidate;
        let blocked = vehicle_colliders
            .iter()
            .any(|(vehicle_matrix, model_index)| {
                let Some(vehicle_col) = data.file_loader.model_info.col_model(*model_index) else {
                    return false;
                };
                let local = vehicle_matrix.multiply_inverse_point(candidate);
                let feet_local_z = local.z - crate::ARENA_PED_BASE_OFFSET;
                if feet_local_z > vehicle_col.bounding_box.max.z + 0.1 {
                    return false;
                }
                CCollision::test_col_models_spheres(
                    &player_matrix,
                    player_col,
                    vehicle_matrix,
                    vehicle_col,
                )
            });
        if blocked {
            return safe;
        }
        safe = candidate;
    }
    end
}

/// Exact arena-side use of re3's `CPed::ProcessCollision` static-world
/// inputs: three 0.35m body spheres followed by `FindGroundZFor3DCoord`.
/// The production game uses the same CWorld calls in its on-foot controller.
fn resolve_arena_world_ped_position(
    previous_position: Vec3,
    position: Vec3,
    vertical_speed: f32,
    ped: &mut CPed,
    data: &ArenaOriginalData,
    time_step: f32,
    now_ms: u32,
) -> (Vec3, bool) {
    let previous = CVector::from_bevy(previous_position);
    let target = CVector::from_bevy(position);
    begin_player_collision_frame(ped);
    let collision = resolve_player_ped_static_collision_result(
        previous,
        target,
        ped,
        &data.file_loader,
        &data.surface_table,
        time_step,
        now_ms,
    );
    let mut gta = collision.position;
    let ground = data.file_loader.world.find_ground_z_for_3d_coord(
        gta.x,
        gta.y,
        gta.z + 0.65,
        &data.file_loader.model_info,
    );
    let grounded = ped.flags.is_standing
        || ground.is_some_and(|z| {
            gta.z <= z + crate::ARENA_PED_BASE_OFFSET + 0.08 && vertical_speed <= 0.0
        });
    if grounded {
        if let Some(ground_z) = ground {
            gta.z = ground_z + crate::ARENA_PED_BASE_OFFSET;
        }
        ped.physical.move_speed.z = 0.0;
        ped.flags.is_standing = true;
        ped.flags.is_in_the_air = false;
    }
    finish_player_collision_frame(ped, previous, gta, collision.movement_rejected);
    (gta.to_bevy(), grounded)
}

/// StartFightAttack/CPed::StartFightAttack for an unarmed arena ped. The
/// animation and all timing still come from the original PED.IFP and
/// fistfite.dat; the arena only supplies the isolated ECS owner.
fn start_arena_fight(
    player: &mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>,
    data: &ArenaOriginalData,
    controlled_vehicle: bool,
    now_ms: u32,
) {
    if controlled_vehicle || player.ped.flags.is_in_the_air || player.ped.dying_or_dead() {
        return;
    }
    let _ = start_fight_attack(
        &mut player.ped,
        &player.vitality,
        &mut player.animation.movement_anim_clump,
        &data.anim_manager,
        &data.fight_moves,
        PedFightMoves::FIGHTMOVE_STDPUNCH,
        now_ms,
    );
}

fn start_arena_fight_move(player: &mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>, data: &ArenaOriginalData, move_id: usize) {
    let Some(move_id) = fight_move_from_index(move_id) else {
        end_arena_fight(&mut player.as_mut(), data);
        return;
    };
    if !start_fight_move(
        &mut player.ped,
        &mut player.animation.movement_anim_clump,
        &data.anim_manager,
        &data.fight_moves,
        move_id,
        4.0,
        None,
    ) {
        end_arena_fight(&mut player.as_mut(), data);
    };
}

fn end_arena_fight(player: &mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>, data: &ArenaOriginalData) {
    end_fight(
        &mut player.ped,
        &mut player.animation.movement_anim_clump,
        &data.anim_manager,
        &data.fight_moves,
        FightEndType::Fast,
    );
    player.ped.move_state = eMoveState::PEDMOVE_STILL;
    player.move_speed = 0.0;
}

/// Advance CPed::Fight and perform its authored strike window. Only the two
/// local arena peds are eligible targets, so no production world/pool state is
/// fabricated for this isolated diagnostic scene.
fn advance_arena_fight(
    player: &mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>,
    pad: &CPad,
    dt: f32,
    now_ms: Option<u32>,
    data: &ArenaOriginalData,
    target_position: Option<CVector>,
    mut target: Option<&mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>>,
) {
    if player.ped.ped_state != PedState::PED_FIGHT {
        return;
    }
    let Some(now_ms) = now_ms else {
        warn_once!("arena_fight_waiting_canonical_timer");
        return;
    };
    if pad.get_weapon() {
        player.ped.fight_button_pressure = pad.new_state.circle.clamp(0, 255) as u8;
    }
    let move_id = player.ped.cur_fight_move as usize;
    let Some(fight_move) = data.fight_moves.get(move_id).copied() else {
        end_arena_fight(&mut player.as_mut(), data);
        return;
    };
    let Some(association_index) = player.animation.movement_anim_clump
        .associations
        .iter()
        .position(|association| association.anim_id == fight_move.anim_id)
    else {
        end_arena_fight(&mut player.as_mut(), data);
        return;
    };
    let (anim_time, association_running) = {
        let association = &mut player.animation.movement_anim_clump.associations[association_index];
        association.update_blend(dt);
        association.update_time(dt, 1.0);
        (association.current_time, association.is_running())
    };

    if fight_move.hit_level != FightMoveHitLevel::HITLEVEL_NULL
        && anim_time > fight_move.start_fire_time
        && anim_time <= fight_move.end_fire_time
        && player.ped.fight_state >= FIGHTSTATE_NO_MOVE
    {
        if player.ped.fight_state != FIGHTSTATE_JUST_ATTACKED {
            let attacker_position = player.position();
            let in_range = target_position.is_some_and(|position| {
                let distance = (attacker_position.x - position.x)
                    .hypot(attacker_position.y - position.y)
                    .hypot(attacker_position.z - position.z);
                distance <= 0.38 + fight_move.strike_radius
            });
            if in_range {
                if let Some(victim) = target.as_deref_mut() {
                    let damage = f32::from(fight_move.damage) * 3.0 + 1.0;
                    victim.vitality.health = (victim.vitality.health - damage * 0.33).max(0.0);
                    victim.ped.flags.is_attacking = false;
                    victim.ped.fight_button_pressure = 0;
                    if victim.vitality.health < 1.0 {
                        victim.ped.set_die(&mut victim.vitality,
                            Some(rustgta3_game::animation::AnimationId::ANIM_STD_KO_FRONT),
                            4.0, 0.0, now_ms);
                    }
                }
                player.animation.movement_anim_clump.associations[association_index].speed = 0.6;
                player.ped.fight_state = FIGHTSTATE_JUST_ATTACKED;
            } else {
                player.ped.fight_state = FIGHTSTATE_NO_MOVE + 1;
            }
        }
        player.ped.fight_button_pressure = 0;
    }
    if fight_move.hit_level != FightMoveHitLevel::HITLEVEL_NULL
        && anim_time > fight_move.end_fire_time
    {
        player.animation.movement_anim_clump.associations[association_index].speed = 1.0;
    }

    if association_running {
        if fight_move.combo_follow_on_time > 0.0
            && player.ped.fight_button_pressure != 0
            && anim_time > fight_move.combo_follow_on_time
            && !player.ped.fight_combo_move
            && move_id + 1 < NUM_FIGHTMOVES
        {
            player.ped.fight_combo_move = true;
            player.ped.fight_button_pressure = 0;
            start_arena_fight_move(&mut player.as_mut(), data, move_id + 1);
        }
        return;
    }

    player.ped.fight_state = FIGHTSTATE_MOVE_FINISHED;
    player.ped.fight_combo_move = false;
    if player.ped.fight_button_pressure != 0 {
        let next = if player.ped.fight_button_pressure > 190 {
            PedFightMoves::FIGHTMOVE_HEADBUTT as usize
        } else {
            PedFightMoves::FIGHTMOVE_KNEE as usize
        };
        player.ped.fight_button_pressure = 0;
        start_arena_fight_move(&mut player.as_mut(), data, next);
    } else if player.ped.take_step_after_attack
        && player.ped.cur_fight_move != PedFightMoves::FIGHTMOVE_SHUFFLE_F
    {
        player.ped.take_step_after_attack = false;
        start_arena_fight_move(&mut player.as_mut(), data, PedFightMoves::FIGHTMOVE_SHUFFLE_F as usize);
    } else {
        end_arena_fight(&mut player.as_mut(), data);
    }
}

fn begin_player_one_vehicle_transition(
    arena: &mut DeveloperArena,
    primary: &mut crate::player_access::ArenaPrimaryPlayers,
    players: &mut Query<
        (Entity, &mut Transform, &mut ArenaPlayer, &mut Visibility),
        Without<ArenaCoopPlayerTwo>,
    >,
    vehicles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
    data: Option<&ArenaOriginalData>,
) {
    let Some(data) = data else {
        arena.status = "Original GTA vehicle animation data unavailable".to_owned();
        return;
    };
    let Ok((_, mut player_transform, _, mut player_visibility)) = players.single_mut() else {
        return;
    };
    let Some(mut player) = primary.get_mut(arena.player_entity) else {
        return;
    };
    if let Some(vehicle) = arena.controlled_vehicle.take() {
        let Some(transition) = start_exit_transition(
            vehicle,
            &mut player.as_mut(),
            &mut player_transform,
            &mut player_visibility,
            vehicles,
            data,
        ) else {
            arena.controlled_vehicle = Some(vehicle);
            arena.status = "Vehicle exit animation could not start".to_owned();
            return;
        };
        arena.vehicle_transition = Some(transition);
        arena.status = "Exiting vehicle".to_owned();
        return;
    }
    let Some((transition, distance)) = start_enter_transition(&mut player.as_mut(),
        &mut player_transform,
        &mut player_visibility,
        vehicles,
        data,
    ) else {
        arena.status = "No enterable vehicle within 5m".to_owned();
        return;
    };
    arena.vehicle_transition = Some(transition);
    arena.animation = if distance < 2.0 {
        ArenaAnimation::Walk
    } else {
        ArenaAnimation::Run
    };
    arena.status = format!("Entering vehicle ({distance:.1}m)");
}

fn begin_player_two_vehicle_transition(
    player: &mut crate::player_access::ArenaCoopMut<'_>,
    player_transform: &mut Transform,
    player_visibility: &mut Visibility,
    vehicles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
    data: Option<&ArenaOriginalData>,
) {
    let Some(data) = data else {
        return;
    };
    if let Some(vehicle) = player.controlled_vehicle.take() {
        player.vehicle_transition = start_exit_transition(
            vehicle,
            &mut player.runtime.as_mut(),
            player_transform,
            player_visibility,
            vehicles,
            data,
        );
        if player.vehicle_transition.is_none() {
            player.controlled_vehicle = Some(vehicle);
        }
        return;
    }
    player.vehicle_transition = start_enter_transition(
        &mut player.runtime.as_mut(),
        player_transform,
        player_visibility,
        vehicles,
        data,
    )
    .map(|(transition, _)| transition);
}

fn create_vehicle_transition(
    data: &ArenaOriginalData,
    vehicle: Entity,
    door: usize,
    animation_id: i32,
    phase: PedVehicleAnimationPhase,
) -> Option<ArenaVehicleTransition> {
    let mut association = data
        .anim_manager
        .create_anim_association(ASSOCGRP_STD, animation_id as usize)?;
    association.start(0.0);
    Some(ArenaVehicleTransition {
        vehicle,
        door,
        association,
        phase,
        seeking: false,
    })
}

fn start_enter_transition(
    player: &mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>,
    player_transform: &mut Transform,
    player_visibility: &mut Visibility,
    vehicles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
    data: &ArenaOriginalData,
) -> Option<(ArenaVehicleTransition, f32)> {
    let player_position = CVector::from_bevy(player_transform.translation);
    let nearest = vehicles
        .iter_mut()
        .filter(|(_, _, vehicle, automobile)| !vehicle.occupied && automobile.is_some())
        .filter_map(|(entity, transform, _, automobile)| {
            let automobile = automobile?.into_view();
            let distance = transform.translation.distance(player_transform.translation);
            if distance > 5.0 {
                return None;
            }
            [2_usize, 3]
                .into_iter()
                .filter(|door| {
                    let flag = automobile_door_flag(*door);
                    automobile.car.vehicle.entry.getting_in_flags & flag == 0
                        && automobile.car.vehicle.entry.getting_out_flags & flag == 0
                })
                .filter_map(|door| {
                    let position = automobile_ped_door_world_position(
                        &data.file_loader,
                        &automobile.car,
                        &automobile.handling,
                        &data.ped_animation_offsets,
                        door,
                        1.0,
                    )?;
                    let dx = position.x - player_position.x;
                    let dy = position.y - player_position.y;
                    Some((door, position, dx * dx + dy * dy))
                })
                .min_by(|left, right| left.2.total_cmp(&right.2))
                .map(|(door, position, _)| (entity, door, position, distance))
        })
        .min_by(|left, right| left.3.total_cmp(&right.3))?;
    let (vehicle_entity, door, door_position, distance) = nearest;
    let (_, _, vehicle, _automobile) = vehicles.get_mut(vehicle_entity).ok()?;
    let rhs = door == 3;
    let high = door_position.z - player_position.z > 4.4;
    let animation = match (rhs, high) {
        (true, true) => ANIM_STD_CAR_ALIGNHI_DOOR_RHS,
        (true, false) => ANIM_STD_CAR_ALIGN_DOOR_RHS,
        (false, true) => ANIM_STD_CAR_ALIGNHI_DOOR_LHS,
        (false, false) => ANIM_STD_CAR_ALIGN_DOOR_LHS,
    };
    let transition = create_vehicle_transition(
        data,
        vehicle_entity,
        door,
        animation,
        PedVehicleAnimationPhase::Align,
    )?;
    // CPed::SetSeekCar does not reserve the door, disable collision, or snap
    // the ped. Those belong to SetEnterCar_AllClear after Seek() succeeds.
    debug_assert!(!vehicle.occupied);
    let mut transition = transition;
    transition.seeking = true;
    player.ped.ped_state = PedState::PED_SEEK_CAR;
    player.ped.seek_pos = door_position;
    player.ped.distance_to_count_seek_done = 0.5;
    player.ped.veh_door = door as i32;
    player.ped.physical.entity.flags.uses_collision = true;
    *player_visibility = Visibility::Visible;
    Some((transition, distance))
}

fn start_exit_transition(
    vehicle_entity: Entity,
    player: &mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>,
    player_transform: &mut Transform,
    player_visibility: &mut Visibility,
    vehicles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
    data: &ArenaOriginalData,
) -> Option<ArenaVehicleTransition> {
    let (_, _, _, automobile) = vehicles.get_mut(vehicle_entity).ok()?;
    let mut automobile = automobile?.into_view();
    let door = 2;
    let profile = PedAutomobileProfile::from_car(&automobile.car);
    let transition = create_vehicle_transition(
        data,
        vehicle_entity,
        door,
        profile.exit_animation(door),
        PedVehicleAnimationPhase::Exit,
    )?;
    automobile.car.vehicle.entry.getting_out_flags |= automobile_door_flag(door);
    automobile
        .car
        .vehicle
        .set_status(eEntityStatus::STATUS_PLAYER_DISABLED);
    automobile.car.vehicle.drive.gas_pedal = 0.0;
    player.ped.ped_state = PedState::PED_EXIT_CAR;
    player.ped.veh_door = door as i32;
    player_transform.translation = automobile_ped_door_world_position(
        &data.file_loader,
        &automobile.car,
        &automobile.handling,
        &data.ped_animation_offsets,
        door,
        0.0,
    )?
    .to_bevy();
    let vehicle_heading = automobile.car.vehicle.physical.entity.get_heading();
    player_transform.rotation = Quat::from_rotation_y(vehicle_heading);
    player.locomotion.rotation_dest = vehicle_heading;
    player.locomotion.rotation_cur = vehicle_heading;
    player.ped.physical.entity.set_heading(vehicle_heading);
    *player_visibility = Visibility::Visible;
    Some(transition)
}

fn advance_player_one_vehicle_transition(
    arena: &mut DeveloperArena,
    primary: &mut crate::player_access::ArenaPrimaryPlayers,
    players: &mut Query<
        (Entity, &mut Transform, &mut ArenaPlayer, &mut Visibility),
        Without<ArenaCoopPlayerTwo>,
    >,
    vehicles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
    data: Option<&ArenaOriginalData>,
    dt: f32,
    pad: &CPad,
    timer: Option<&mut CTimer>,
) -> Option<String> {
    let data = data?;
    let (_, mut transform, mut arena_player, mut visibility) = players.single_mut().ok()?;
    arena_player.previous_position = transform.translation;
    let DeveloperArena {
        vehicle_transition,
        controlled_vehicle,
        player_entity,
        animation,
        runtime_animation_selection,
        ..
    } = &mut *arena;
    let mut player = primary.get_mut(*player_entity)?;
    let status = advance_vehicle_transition(
        vehicle_transition,
        controlled_vehicle,
        &mut player.as_mut(),
        &mut transform,
        &mut visibility,
        vehicles,
        data,
        dt,
        pad,
        timer,
    );
    if vehicle_transition
        .as_ref()
        .is_some_and(|transition| transition.seeking)
    {
        *animation = match player.ped.move_state {
            eMoveState::PEDMOVE_SPRINT => ArenaAnimation::Sprint,
            eMoveState::PEDMOVE_RUN => ArenaAnimation::Run,
            eMoveState::PEDMOVE_WALK => ArenaAnimation::Walk,
            _ => ArenaAnimation::Idle,
        };
    }
    if status.as_deref() == Some("Exited vehicle") {
        *animation = ArenaAnimation::Idle;
        *runtime_animation_selection = None;
    }
    status
}

fn advance_player_two_vehicle_transition(
    player: &mut crate::player_access::ArenaCoopMut<'_>,
    transform: &mut Transform,
    visibility: &mut Visibility,
    vehicles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
    data: Option<&ArenaOriginalData>,
    dt: f32,
    pad: &CPad,
    timer: Option<&mut CTimer>,
) {
    let Some(data) = data else {
        return;
    };
    let crate::player_access::ArenaCoopView { runtime, fixture } = player;
    let ArenaCoopPlayerTwo {
        vehicle_transition,
        controlled_vehicle,
        ..
    } = &mut **fixture;
    let _ = advance_vehicle_transition(
        vehicle_transition,
        controlled_vehicle,
        &mut runtime.as_mut(),
        transform,
        visibility,
        vehicles,
        data,
        dt,
        pad,
        timer,
    );
    player.horizontal_velocity = Vec2::ZERO;
    player.vertical_velocity = 0.0;
}

fn advance_vehicle_transition(
    transition_slot: &mut Option<ArenaVehicleTransition>,
    controlled_vehicle: &mut Option<Entity>,
    player: &mut rustgta3_game::peds::PlayerPed::PlayerPedMut<'_>,
    player_transform: &mut Transform,
    player_visibility: &mut Visibility,
    vehicles: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut ArenaVehicle,
            Option<ArenaAutomobileRuntime>,
        ),
        (Without<ArenaPlayer>, Without<ArenaCoopPlayerTwo>),
    >,
    data: &ArenaOriginalData,
    dt: f32,
    pad: &CPad,
    mut timer: Option<&mut CTimer>,
) -> Option<String> {
    let mut transition = transition_slot.take()?;
    let Ok((_, vehicle_transform, mut vehicle, automobile)) = vehicles.get_mut(transition.vehicle)
    else {
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.physical.entity.flags.uses_collision = true;
        *controlled_vehicle = None;
        *player_visibility = Visibility::Visible;
        return Some("Vehicle disappeared during enter/exit".to_owned());
    };
    let Some(automobile) = automobile else {
        vehicle.occupied = false;
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.physical.entity.flags.uses_collision = true;
        *controlled_vehicle = None;
        *player_visibility = Visibility::Visible;
        return Some("CAutomobile runtime missing during enter/exit".to_owned());
    };
    let mut automobile = automobile.into_view();

    if transition.seeking {
        let movement_cancelled = (pad.new_state.left_stick_x != 0
            && pad.old_state.left_stick_x == 0)
            || (pad.new_state.left_stick_y != 0 && pad.old_state.left_stick_y == 0)
            || (pad.new_state.dpad_up != 0 && pad.old_state.dpad_up == 0)
            || (pad.new_state.dpad_down != 0 && pad.old_state.dpad_down == 0)
            || (pad.new_state.dpad_left != 0 && pad.old_state.dpad_left == 0)
            || (pad.new_state.dpad_right != 0 && pad.old_state.dpad_right == 0);
        if movement_cancelled || vehicle.occupied {
            player.ped.ped_state = PedState::PED_IDLE;
            player.ped.move_state = eMoveState::PEDMOVE_STILL;
            player.ped.physical.move_speed = CVector::ZERO;
            return Some("Vehicle entry cancelled".to_owned());
        }
        let flag = automobile_door_flag(transition.door);
        if automobile.car.vehicle.entry.getting_in_flags & flag != 0
            || automobile.car.vehicle.entry.getting_out_flags & flag != 0
        {
            player.ped.move_state = eMoveState::PEDMOVE_STILL;
            *transition_slot = Some(transition);
            return None;
        }
        let Some(door_position) = automobile_ped_door_world_position(
            &data.file_loader,
            &automobile.car,
            &automobile.handling,
            &data.ped_animation_offsets,
            transition.door,
            1.0,
        ) else {
            player.ped.ped_state = PedState::PED_IDLE;
            return Some("Vehicle door position unavailable".to_owned());
        };
        player.ped.seek_pos = door_position;
        let position = player.position();
        let delta = CVector::new(
            door_position.x - position.x,
            door_position.y - position.y,
            0.0,
        );
        let distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
        if distance > player.ped.distance_to_count_seek_done {
            player.move_speed = (2.0 * distance).min(2.0);
            player.ped.move_state = if pad.new_state.cross != 0 {
                eMoveState::PEDMOVE_SPRINT
            } else if distance < 2.0 {
                eMoveState::PEDMOVE_WALK
            } else {
                eMoveState::PEDMOVE_RUN
            };
            player.locomotion.rotation_dest = CGeneral::get_radian_angle_between_points(
                door_position.x,
                door_position.y,
                position.x,
                position.y,
            );
            let needed =
                CGeneral::limit_radian_angle(player.locomotion.rotation_dest - player.locomotion.rotation_cur);
            let max_turn = player.locomotion.heading_rate.to_radians() * dt * 50.0;
            player.locomotion.rotation_cur = CGeneral::limit_radian_angle(
                player.locomotion.rotation_cur + needed.clamp(-max_turn, max_turn),
            );
            player
                .ped
                .physical
                .entity
                .set_heading(player.locomotion.rotation_cur);
            if let Some(timer) = timer.as_deref_mut() {
                let local_delta = update_player_movement_animation(&mut player.as_mut(),
                    &data.anim_manager,
                    &data.file_loader,
                    dt,
                    true,
                    false,
                    timer.time_in_milliseconds(),
                    timer,
                );
                let (sin, cos) = player.locomotion.rotation_cur.sin_cos();
                let mut next = player.position();
                next.x += local_delta.x * cos - local_delta.y * sin;
                next.y += local_delta.y * cos + local_delta.x * sin;
                player.set_position(next);
                player_transform.translation = next.to_bevy();
                player_transform.rotation = Quat::from_rotation_y(player.locomotion.rotation_cur);
            }
            *transition_slot = Some(transition);
            return None;
        }

        // SetEnterCar_AllClear: reserve the selected door only after Seek()
        // has reached the authored opening position.
        transition.seeking = false;
        transition.association.set_current_time(0.0);
        transition.association.set_run();
        vehicle.occupied = true;
        automobile.car.vehicle.entry.num_getting_in =
            automobile.car.vehicle.entry.num_getting_in.saturating_add(1);
        automobile.car.vehicle.entry.getting_in_flags |= flag;
        automobile.car.vehicle.drive.gas_pedal = 0.0;
        player.ped.ped_state = PedState::PED_ENTER_CAR;
        player.ped.physical.entity.flags.uses_collision = false;
    }

    transition.association.update_time(dt, 1.0);
    transition.association.update_blend(dt);
    automobile.car.process_open_door(
        transition.door,
        transition.association.anim_id,
        transition.association.current_time,
    );
    let seat_multiplier =
        ped_vehicle_line_up_seat_multiplier(transition.phase, &transition.association);
    if let Some(position) = automobile_ped_door_world_position(
        &data.file_loader,
        &automobile.car,
        &automobile.handling,
        &data.ped_animation_offsets,
        transition.door,
        seat_multiplier,
    ) {
        player_transform.translation = position.to_bevy();
        player.set_position(position);
    }
    let vehicle_heading = automobile.car.vehicle.physical.entity.get_heading();
    player_transform.rotation = Quat::from_rotation_y(vehicle_heading);
    player.locomotion.rotation_dest = vehicle_heading;
    player.locomotion.rotation_cur = vehicle_heading;
    player.ped.physical.entity.set_heading(vehicle_heading);
    *player_visibility = Visibility::Visible;

    if transition.association.is_running() && !transition.association.deleted {
        *transition_slot = Some(transition);
        return None;
    }

    let door = transition.door;
    let entity = transition.vehicle;
    let profile = PedAutomobileProfile::from_car(&automobile.car);
    let next = match transition.phase {
        PedVehicleAnimationPhase::Align => {
            let door_missing = automobile.car.damage.get_door_status(door as u8)
                == rustgta3_game::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_MISSING as u32;
            let (animation, phase) = if door_missing {
                (
                    profile.get_in_animation(door),
                    PedVehicleAnimationPhase::GetIn,
                )
            } else {
                (
                    profile.open_animation(door),
                    PedVehicleAnimationPhase::OpenDoor,
                )
            };
            create_vehicle_transition(data, entity, door, animation, phase)
        }
        PedVehicleAnimationPhase::OpenDoor => create_vehicle_transition(
            data,
            entity,
            door,
            profile.get_in_animation(door),
            PedVehicleAnimationPhase::GetIn,
        ),
        PedVehicleAnimationPhase::PullOut => create_vehicle_transition(
            data,
            entity,
            door,
            profile.get_in_animation(door),
            PedVehicleAnimationPhase::GetIn,
        ),
        PedVehicleAnimationPhase::DraggedOut => {
            let mut offset = if transition.association.anim_id
                == rustgta3_game::animation::AnimationId::ANIM_STD_QUICKJACKED as i32
            {
                data.ped_animation_offsets.ped_quick_dragged_out_car
            } else {
                data.ped_animation_offsets.ped_dragged_out_car
            };
            if matches!(door, 3 | 5) {
                offset.x = -offset.x;
            }
            let position = player.ped.physical.entity.get_position();
            let translated = player.ped.physical.entity.transform_vector(offset);
            let exit_position = CVector::new(
                position.x + translated.x,
                position.y + translated.y,
                position.z + translated.z,
            );
            player.set_position(exit_position);
            player_transform.translation = exit_position.to_bevy();
            player.ped.ped_state = PedState::PED_IDLE;
            player.ped.move_state = eMoveState::PEDMOVE_STILL;
            player.ped.physical.move_speed = CVector::ZERO;
            player.ped.physical.entity.flags.uses_collision = true;
            automobile.car.vehicle.entry.getting_out_flags &= !automobile_door_flag(door);
            automobile
                .car
                .vehicle
                .set_status(eEntityStatus::STATUS_ABANDONED);
            vehicle.occupied = false;
            *controlled_vehicle = None;
            *player_visibility = Visibility::Visible;
            return Some("Dragged out of vehicle".to_owned());
        }
        PedVehicleAnimationPhase::GetIn => {
            let door_missing = automobile.car.damage.get_door_status(door as u8)
                == rustgta3_game::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_MISSING as u32;
            if automobile.car.vehicle.flags.is_bus || door_missing {
                automobile.car.vehicle.entry.num_getting_in =
                    automobile.car.vehicle.entry.num_getting_in.saturating_sub(1);
                automobile.car.vehicle.entry.getting_in_flags &= !automobile_door_flag(door);
                automobile.car.vehicle.flags.has_been_owned_by_player = true;
                automobile.car.vehicle.drive.engine_on = true;
                automobile
                    .car
                    .vehicle
                    .set_status(eEntityStatus::STATUS_PLAYER);
                player.ped.ped_state = PedState::PED_DRIVING;
                *controlled_vehicle = Some(entity);
                if let Some(seat) =
                    automobile_driver_world_position(&data.file_loader, &automobile.car)
                {
                    player_transform.translation = seat.to_bevy();
                    player.set_position(seat);
                }
                player_transform.rotation = vehicle_transform.rotation;
                *player_visibility = Visibility::Visible;
                return Some("Entered vehicle".to_owned());
            }
            create_vehicle_transition(
                data,
                entity,
                door,
                profile.close_enter_animation(door),
                PedVehicleAnimationPhase::CloseEnter,
            )
        }
        PedVehicleAnimationPhase::CloseEnter | PedVehicleAnimationPhase::Shuffle => {
            automobile.car.vehicle.entry.num_getting_in =
                automobile.car.vehicle.entry.num_getting_in.saturating_sub(1);
            automobile.car.vehicle.entry.getting_in_flags &= !automobile_door_flag(door);
            automobile.car.vehicle.flags.has_been_owned_by_player = true;
            automobile.car.vehicle.drive.engine_on = true;
            automobile
                .car
                .vehicle
                .set_status(eEntityStatus::STATUS_PLAYER);
            player.ped.ped_state = PedState::PED_DRIVING;
            *controlled_vehicle = Some(entity);
            if let Some(seat) = automobile_driver_world_position(&data.file_loader, &automobile.car)
            {
                player_transform.translation = seat.to_bevy();
                player.set_position(seat);
            }
            player_transform.rotation = vehicle_transform.rotation;
            *player_visibility = Visibility::Visible;
            return Some("Entered vehicle".to_owned());
        }
        PedVehicleAnimationPhase::Exit => {
            vehicle.occupied = false;
            automobile.car.vehicle.physical.static_frames = 0;
            let move_speed = automobile.car.vehicle.physical.move_speed;
            automobile.car.vehicle.physical.move_speed = CVector::new(
                move_speed.x + 0.001,
                move_speed.y + 0.001,
                move_speed.z + 0.001,
            );
            let turn_speed = automobile.car.vehicle.physical.turn_speed;
            automobile.car.vehicle.physical.turn_speed = CVector::new(
                turn_speed.x + 0.001,
                turn_speed.y + 0.001,
                turn_speed.z + 0.001,
            );
            automobile
                .car
                .vehicle
                .set_status(eEntityStatus::STATUS_ABANDONED);
            vehicle.occupied = false;
            player.ped.physical.entity.flags.uses_collision = true;
            player.ped.ped_state = PedState::PED_EXIT_CAR;
            *controlled_vehicle = None;
            let door_missing = automobile.car.damage.get_door_status(door as u8)
                == rustgta3_game::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_MISSING as u32;
            if automobile.car.vehicle.flags.is_bus
                || door_missing
                || automobile.car.damage.get_engine_status() >= 225
            {
                automobile.car.vehicle.entry.getting_out_flags &= !automobile_door_flag(door);
                player.ped.ped_state = PedState::PED_IDLE;
                player.ped.move_state = eMoveState::PEDMOVE_STILL;
                return Some("Exited vehicle".to_owned());
            }
            create_vehicle_transition(
                data,
                entity,
                door,
                profile.close_exit_animation(door),
                PedVehicleAnimationPhase::CloseExit,
            )
        }
        PedVehicleAnimationPhase::CloseExit => {
            automobile.car.vehicle.entry.getting_out_flags &= !automobile_door_flag(door);
            player.ped.ped_state = PedState::PED_IDLE;
            player.ped.move_state = eMoveState::PEDMOVE_STILL;
            player.ped.physical.move_speed = CVector::ZERO;
            *player_visibility = Visibility::Visible;
            return Some("Exited vehicle".to_owned());
        }
        PedVehicleAnimationPhase::RollingDoorClose => {
            // Shared runtime phase; the arena does not normally start it, but
            // when it is present mirror `PedAnimDoorCloseRollingCB` without
            // ejecting the local player from the vehicle.
            let animation = if automobile.car.vehicle.flags.low_vehicle {
                rustgta3_game::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LO_LHS
            } else {
                rustgta3_game::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS
            };
            automobile.car.process_open_door(door, animation, 1.0);
            automobile.car.vehicle.entry.getting_out_flags &= !automobile_door_flag(door);
            if automobile.car.damage.get_door_status(door as u8)
                == rustgta3_game::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
            {
                automobile.car.damage.set_door_status(
                    door as u8,
                    rustgta3_game::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_OK as u32,
                );
            }
            return None;
        }
        PedVehicleAnimationPhase::LockedDoor => None,
        PedVehicleAnimationPhase::External => None,
    };
    if next.is_none() && transition.phase != PedVehicleAnimationPhase::External {
        // A real PED.IFP association is mandatory.  If the original asset is
        // absent/corrupt, do not leave the half-finished re3 reservation and
        // an open door behind, and do not invent a fallback animation.
        automobile.car.process_open_door(
            door,
            rustgta3_game::animation::AnimationId::ANIM_STD_NUM as i32,
            0.0,
        );
        automobile.car.vehicle.entry.num_getting_in =
            automobile.car.vehicle.entry.num_getting_in.saturating_sub(1);
        automobile.car.vehicle.entry.getting_in_flags &= !automobile_door_flag(door);
        automobile.car.vehicle.entry.getting_out_flags &= !automobile_door_flag(door);
        automobile
            .car
            .vehicle
            .set_status(eEntityStatus::STATUS_ABANDONED);
        vehicle.occupied = false;
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        player.ped.physical.entity.flags.uses_collision = true;
        *controlled_vehicle = None;
        *player_visibility = Visibility::Visible;
        return Some(format!(
            "Original PED.IFP vehicle animation missing after {:?}",
            transition.phase
        ));
    }
    *transition_slot = next;
    None
}

#[derive(Debug, Clone, Copy)]
struct AutomobileTelemetry {
    transform: Transform,
    speed_mps: f32,
}

fn step_ported_automobile(
    runtime: &mut ArenaAutomobileMut<'_>,
    history: &mut VehicleCollisionState,
    general: &mut CGeneral,
    keys: Option<&ButtonInput<KeyCode>>,
    pad: &rustgta3_game::core::Pad::CPad,
    dt: f32,
    world_data: Option<&ArenaOriginalData>,
    time_ms: Option<u32>,
    damage_nodes: Option<&rustgta3_game::modelinfo::VehicleModelInfo::VehicleDamageNodes>,
) -> AutomobileTelemetry {
    let time_step = dt.max(0.0) * 50.0;
    let button = |pressed: bool| -> f32 { if pressed { 255.0 } else { 0.0 } };
    let contacts = world_data.map_or_else(
        || flat_arena_automobile_contacts(runtime.car.as_ref(), runtime.handling, runtime.wheel_scale),
        |data| automobile_road_contacts(&runtime.car, &data.file_loader, time_step),
    );
    // CCollision writes m_aWheelColPoints in world space before
    // CAutomobile::ProcessControl emits tyre smoke and skidmarks. Keep the
    // world-space effect cache synchronized with the collision contacts.
    store_automobile_wheel_contacts(
        &mut runtime.car,
        &contacts,
        world_data.map(|data| &data.file_loader),
    );
    let local_contacts = contacts.local_points;
    let contact_points = contacts.relative_world_points;
    let spring_ratios = contacts.spring_ratios;
    let spring_directions = contacts.spring_directions;
    let grounded = contacts.grounded;
    let keyboard_steering = keys
        .map(|keys| axis(keys, KeyCode::KeyA, KeyCode::KeyD) * 128.0)
        .unwrap_or(0.0);
    let gamepad_steering = f32::from(pad.get_steering_left_right());
    let steering_left_right = if keyboard_steering.abs() > gamepad_steering.abs() {
        keyboard_steering
    } else {
        gamepad_steering
    };
    history.previous_matrix = runtime.car.vehicle.physical.entity.matrix;
    runtime.car.process_player_drive_step(
        CAutomobileDriveStepInput {
            control: CAutomobileControlInput {
                steering_left_right,
                mouse_x: 0.0,
                mouse_steering: false,
                accelerate: button(keys.is_some_and(|keys| keys.pressed(KeyCode::KeyW)))
                    .max(f32::from(pad.get_accelerate()))
                    .max(f32::from(pad.new_state.right_shoulder2)),
                brake: button(keys.is_some_and(|keys| keys.pressed(KeyCode::KeyS)))
                    .max(f32::from(pad.get_brake()))
                    .max(f32::from(pad.new_state.left_shoulder2)),
                handbrake: keys.is_some_and(|keys| keys.pressed(KeyCode::Space))
                    || pad.get_hand_brake() != 0,
                exit_vehicle: keys.is_some_and(|keys| keys.just_pressed(KeyCode::KeyF))
                    || pad.exit_vehicle_just_down(),
                controls_disabled: false,
            },
            local_contact_points: local_contacts,
            contact_points,
            spring_ratios,
            spring_directions,
            adhesive_limits: [runtime.surface_adhesion; NUM_CAR_WHEELS],
            grounded,
            collision_entities: contacts.collision_entities,
            collision_time_ms: time_ms.unwrap_or(0),
            wheel_scale: contacts.wheel_scale,
            time_step,
            time_step_seconds: dt,
        },
        &mut runtime.handling,
        runtime.wheel_friction,
        general,
    damage_nodes,
    );
    if let Some(data) = world_data {
        automobile_static_collision_step(
            &mut runtime.car,
            history.previous_matrix,
            None,
            &data.file_loader,
            data.file_loader.renderer.camera_position,
            &data.surface_table,
            time_step,
            time_ms,
            runtime.handling.collision_damage_multiplier,
            general,
            None,
        );
    }
    automobile_telemetry(&runtime.car.as_ref())
}

fn refresh_arena_suspension_model(runtime: &mut ArenaAutomobileMut<'_>,
    original: Option<&ArenaOriginalData>) {
    let Some(data) = original else {
        runtime.car.state.invalidate_suspension_source();
        warn_once!("arena_suspension_rebind_waiting_original_data");
        return;
    };
    let Some(rustgta3_game::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(model)) =
        usize::try_from(runtime.car.vehicle.core.physical.entity.model_index).ok()
            .and_then(|id| data.file_loader.model_info.get_model_info(id)) else {
        runtime.car.state.invalidate_suspension_source();
        warn_once!("arena_suspension_rebind_waiting_original_model");
        return;
    };
    runtime.car.setup_suspension_from_model(model, runtime.handling);
}

fn automobile_telemetry(
    car: &rustgta3_game::vehicles::Automobile::AutomobileRef<'_>,
) -> AutomobileTelemetry {
    let matrix = &car.vehicle.physical.entity.matrix;
    let right = matrix.right.to_bevy().normalize_or_zero();
    let up = matrix.up.to_bevy().normalize_or_zero();
    let back = (-matrix.forward.to_bevy()).normalize_or_zero();
    let rotation = if right.length_squared() > f32::EPSILON
        && up.length_squared() > f32::EPSILON
        && back.length_squared() > f32::EPSILON
    {
        Quat::from_mat3(&Mat3::from_cols(right, up, back)).normalize()
    } else {
        Quat::IDENTITY
    };
    AutomobileTelemetry {
        transform: Transform {
            translation: matrix.position.to_bevy(),
            rotation,
            scale: Vec3::ONE,
        },
        speed_mps: car.vehicle.physical.move_speed.magnitude() * 50.0,
    }
}

fn flat_arena_automobile_contacts(car: rustgta3_game::vehicles::Automobile::AutomobileRef<'_>, handling: &rustgta3_game::vehicles::HandlingMgr::tHandlingData, wheel_scale: f32) -> AutomobileRoadContacts {
    let (local_starts, local_contacts) = if car.suspension_initialized {
        (
            car.suspension_line_start,
            car.suspension_line_end,
        )
    } else {
        let half_width = handling.dimension.x.abs().max(1.0) * 0.38;
        let half_length = handling.dimension.y.abs().max(2.0) * 0.34;
        let contact_height = -handling.dimension.z.abs().max(1.0) * 0.35;
        let ends = [
            CVector::new(-half_width, half_length, contact_height),
            CVector::new(-half_width, -half_length, contact_height),
            CVector::new(half_width, half_length, contact_height),
            CVector::new(half_width, -half_length, contact_height),
        ];
        let starts = ends.map(|point| CVector::new(point.x, point.y, point.z + 0.7));
        (starts, ends)
    };
    let relative_starts: [CVector; NUM_CAR_WHEELS] = std::array::from_fn(|wheel| {
        car
            .vehicle
            .physical
            .entity
            .transform_vector(local_starts[wheel])
    });
    let relative_ends = std::array::from_fn(|wheel| {
        car
            .vehicle
            .physical
            .entity
            .transform_vector(local_contacts[wheel])
    });
    let origin_z = car.vehicle.physical.entity.get_position().z;
    let mut contact_points = relative_ends;
    let mut spring_ratios = [1.0; NUM_CAR_WHEELS];
    let mut spring_directions = [CVector::new(0.0, 0.0, -1.0); NUM_CAR_WHEELS];
    let mut grounded = [false; NUM_CAR_WHEELS];
    for wheel in 0..NUM_CAR_WHEELS {
        let start = relative_starts[wheel];
        let end = relative_ends[wheel];
        let line = CVector::new(end.x - start.x, end.y - start.y, end.z - start.z);
        let length = line.magnitude().max(1.0e-5);
        spring_directions[wheel] = CVector::new(line.x / length, line.y / length, line.z / length);
        if line.z.abs() <= 1.0e-5 {
            continue;
        }
        let raw_ratio = ((-origin_z - start.z) / line.z).clamp(0.0, 1.0);
        let hit_z = origin_z + start.z + line.z * raw_ratio;
        let min_z = (origin_z + start.z).min(origin_z + end.z) - 0.06;
        let max_z = (origin_z + start.z).max(origin_z + end.z) + 0.06;
        if hit_z.abs() > 0.06 || !(min_z..=max_z).contains(&0.0) {
            continue;
        }
        grounded[wheel] = true;
        contact_points[wheel] = CVector::new(
            start.x + line.x * raw_ratio,
            start.y + line.y * raw_ratio,
            -origin_z,
        );
        let wheel_fraction = if car.suspension_initialized {
            (1.0 - car.suspension_spring_length[wheel]
                / car.suspension_line_length[wheel].max(1.0e-5))
            .clamp(0.0, 0.999)
        } else {
            0.0
        };
        spring_ratios[wheel] =
            ((raw_ratio - wheel_fraction) / (1.0 - wheel_fraction)).clamp(0.0, 1.0);
    }
    AutomobileRoadContacts {
        local_points: local_contacts,
        relative_world_points: contact_points,
        spring_ratios,
        spring_directions,
        grounded,
        collision_entities: [None; NUM_CAR_WHEELS],
        ground_entities: [None; NUM_CAR_WHEELS],
        surfaces: [rustgta3_game::core::SurfaceTable::eSurfaceType::SURFACE_TARMAC as u8;
            NUM_CAR_WHEELS],
        wheel_scale: wheel_scale,
    }
}

struct ArenaAnimationReadout { name: String, time: f32, duration: f32 }
impl ArenaAnimationReadout {
    fn from_association(association: &CAnimBlendAssociation) -> Self {
        Self { name: association.hierarchy.name.clone(), time: association.current_time,
            duration: association.hierarchy.total_length }
    }
}

pub(crate) fn advance_arena_primary_animation(
    time: Res<Time>,
    mut arena: ResMut<DeveloperArena>,
    original: Res<ArenaOriginalRuntime>,
    mut actors: Query<(&CPed, &rustgta3_game::peds::PlayerPed::PlayerPedState,
        &mut rustgta3_game::peds::PedAnimation::PedAnimation),
        (With<ArenaPlayer>, Without<ArenaCoopPlayerTwo>)>,
) {
    let dt = time.delta_secs().min(0.05);
    let selection = arena.animation;
    let animation_speed = arena.animation_speed;
    let previous_selection = arena.runtime_animation_selection;
    let transition_active = arena
        .vehicle_transition
        .as_ref()
        .filter(|transition| !transition.seeking)
        .is_some();
    let mut runtime_snapshot = arena.vehicle_transition.as_ref()
        .filter(|transition| !transition.seeking)
        .map(|transition| ArenaAnimationReadout::from_association(&transition.association));
    if arena.player_entity.is_some_and(|entity| !actors.contains(entity)) {
        warn_once!("arena_primary_animation_missing_owner");
    }
    if let (Some(data), Some((ped, player, mut animation))) =
        (original.data.as_ref(), arena.player_entity.and_then(|entity| actors.get_mut(entity).ok())) {
        let manager = &data.anim_manager;
        let (group, animation_id) = animation_spec(selection);
        let jump_driven_by_player_state =
            if let Some(association) = player.jump_association.as_ref() {
                runtime_snapshot = Some(ArenaAnimationReadout::from_association(association));
                true
            } else {
                false
            };
        let runtime_driven_locomotion = matches!(
            selection,
            ArenaAnimation::Idle
                | ArenaAnimation::Walk
                | ArenaAnimation::Run
                | ArenaAnimation::Sprint
        ) && animation.movement_anim_clump
            .associations
            .iter()
            .any(|association| association.anim_id == animation_id);
        // CPed::Fight owns every attack association and advances it in the
        // control system. Do not let the UI animation selector clear or
        // restart the punch halfway through its authored strike window.
        let fight_driven = ped.ped_state == PedState::PED_FIGHT;
        if !transition_active
            && !jump_driven_by_player_state
            && !runtime_driven_locomotion
            && !fight_driven
        {
            let needs_restart = previous_selection != Some(selection)
                || animation.movement_anim_clump
                    .associations
                    .first()
                    .is_none_or(|association| association.anim_id != animation_id);
            if needs_restart {
                animation.movement_anim_clump.associations.clear();
                if let Some(mut association) =
                    manager.create_anim_association(group, animation_id as usize)
                {
                    association.set_blend(1.0, 0.0);
                    association.start(0.0);
                    animation.movement_anim_clump.associations.push(association);
                } else {
                    warn_once!(group, animation_id, "arena_animation_missing_original_association");
                }
            }
            if let Some(association) = animation.movement_anim_clump.associations.first_mut() {
                association.speed = animation_speed;
                association.update_time(dt, animation_speed);
                association.update_blend(dt);
            }
        }
        if runtime_snapshot.is_none() {
            runtime_snapshot = animation.movement_anim_clump
                .associations
                .iter()
                .find(|association| association.anim_id == animation_id)
                .map(ArenaAnimationReadout::from_association);
        }
    }
    arena.runtime_animation_selection = Some(selection);
    if let Some(readout) = runtime_snapshot {
        arena.runtime_animation_name = Some(readout.name);
        arena.runtime_animation_time = readout.time;
        arena.runtime_animation_duration = readout.duration;
    } else {
        arena.runtime_animation_name = None;
        arena.runtime_animation_time = 0.0;
        arena.runtime_animation_duration = 0.0;
    }

}

pub(crate) fn present_arena_primary_pose(
    mut arena: ResMut<DeveloperArena>,
    original: Res<ArenaOriginalRuntime>,
    actors: Query<(&rustgta3_game::peds::PedAnimation::PedAnimation,
        &rustgta3_game::peds::PlayerPed::PlayerPedState),
        (With<ArenaPlayer>, Without<ArenaCoopPlayerTwo>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut visuals: Query<(Entity, &ArenaCharacterVisual, &crate::ArenaCharacterPoseManifest, &mut Transform), Without<ArenaCharacterModelPart>>,
    mut parts: Query<(&ArenaCharacterModelPart, &Mesh3d, &ChildOf, &mut Transform), With<ArenaCharacterModelPart>>,
) {
    if arena.player_entity.is_some_and(|entity| !actors.contains(entity)) {
        warn_once!("arena_primary_pose_missing_animation_owner");
    }
    if original.data.is_none() && arena.player_entity.is_some() {
        warn_once!("arena_primary_pose_waiting_for_original_data");
    }
    let vehicle_association = arena.vehicle_transition.as_ref()
        .filter(|transition| !transition.seeking)
        .map(|transition| &transition.association);
    let mesh_bound = original
        .data
        .as_ref()
        .zip(arena.player_entity.and_then(|entity| actors.get(entity).ok()))
        .is_some_and(|(data, (animation, player))| {
            bind_arena_player_pose(0, animation, player,
                data,
                player.jump_association.is_some(),
                vehicle_association,
                &mut meshes,
                &mut visuals,
                &mut parts,
            )
        });
    arena.runtime_animation_mesh_bound = mesh_bound;
}

pub(crate) fn animate_arena_player_two(
    original: Res<ArenaOriginalRuntime>,
    players: Query<(&ArenaCoopPlayerTwo,
        &rustgta3_game::peds::PedAnimation::PedAnimation,
        &rustgta3_game::peds::PlayerPed::PlayerPedState)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut visuals: Query<(Entity, &ArenaCharacterVisual, &crate::ArenaCharacterPoseManifest, &mut Transform), Without<ArenaCharacterModelPart>>,
    mut parts: Query<
        (&ArenaCharacterModelPart, &Mesh3d, &ChildOf, &mut Transform),
        With<ArenaCharacterModelPart>,
    >,
) {
    let (Some(data), Ok((player_two, animation, player_state))) = (original.data.as_ref(), players.single()) else {
        return;
    };
    let transition_association = player_two
        .vehicle_transition
        .as_ref()
        .filter(|transition| !transition.seeking)
        .map(|transition| &transition.association);
    let seated_association = transition_association.is_none().then(|| {
        let animation_id = player_two
            .controlled_vehicle
            .is_some()
            .then_some(ANIM_STD_CAR_SIT as usize)?;
        data.anim_manager
            .create_anim_association(ASSOCGRP_STD, animation_id)
    });
    let seated_association = seated_association.flatten();
    let vehicle_association = transition_association.or(seated_association.as_ref());
    bind_arena_player_pose(1, animation, player_state,
        data,
        player_state.jump_association.is_some(),
        vehicle_association,
        &mut meshes,
        &mut visuals,
        &mut parts,
    );
}

fn bind_arena_player_pose(
    local_player: u8,
    animation: &rustgta3_game::peds::PedAnimation::PedAnimation,
    player: &rustgta3_game::peds::PlayerPed::PlayerPedState,
    data: &ArenaOriginalData,
    use_jump_association: bool,
    vehicle_association: Option<&CAnimBlendAssociation>,
    meshes: &mut Assets<Mesh>,
    visuals: &mut Query<(Entity, &ArenaCharacterVisual, &crate::ArenaCharacterPoseManifest, &mut Transform), Without<ArenaCharacterModelPart>>,
    parts: &mut Query<
        (&ArenaCharacterModelPart, &Mesh3d, &ChildOf, &mut Transform),
        With<ArenaCharacterModelPart>,
    >,
) -> bool {
    let Some((visual_entity, model_index, loaded_dff, manifest)) = visuals
        .iter_mut()
        .find(|(_, visual, _, _)| visual.local_player == local_player)
        .map(|(entity, visual, manifest, mut transform)| {
            // Never fabricate a proxy pose. The authored bind pose remains the
            // fallback when the real PED.IFP hierarchy cannot be bound.
            *transform = Transform::IDENTITY;
            (entity, visual.model_index, visual.loaded_dff, manifest)
        })
    else {
        warn_once!(local_player, "arena_pose_missing_visual");
        return false;
    };
    if !loaded_dff {
        warn_once!(local_player, "arena_pose_waiting_for_original_dff");
        return false;
    }

    if manifest.model_index != model_index {
        warn_once!(local_player, "arena_pose_manifest_model_changed");
        return false;
    }
    let part_count = manifest.parts.len();
    if part_count == 0 {
        warn_once!(local_player, "arena_pose_missing_drawable_parts");
        return false;
    }
    if manifest.rigid_frame_indices.as_ref().is_some_and(|frames| frames.len() != part_count) {
        warn_once!(local_player, "arena_pose_manifest_frame_count_mismatch");
        return false;
    }
    for (source_index, entity) in manifest.parts.iter().enumerate() {
        let Ok((part, mesh, parent, _)) = parts.get(*entity) else {
            warn_once!(local_player, "arena_pose_manifest_member_retired");
            return false;
        };
        let expected_frame = manifest.rigid_frame_indices.as_ref().map(|frames| frames[source_index]);
        if parent.parent() != visual_entity || part.local_player != local_player
            || part.part_index != source_index || part.rigid_frame_index != expected_frame {
            warn_once!(local_player, "arena_pose_manifest_member_changed");
            return false;
        }
        if meshes.get(mesh.0.id()).is_none() {
            warn_once!(local_player, "arena_pose_missing_mesh_asset");
            return false;
        }
    }
    let rigid_transforms = manifest.rigid_frame_indices.as_ref()
        .and_then(|rigid_frame_indices| {
            if let Some(association) = vehicle_association {
                rigid_ped_part_transforms_from_association(
                    model_index,
                    &data.file_loader,
                    association,
                    rigid_frame_indices,
                )
            } else if use_jump_association {
                player.jump_association.as_ref().and_then(|association| {
                    rigid_ped_part_transforms_from_association(
                        model_index,
                        &data.file_loader,
                        association,
                        rigid_frame_indices,
                    )
                })
            } else {
                rigid_ped_part_transforms_from_clump(
                    model_index,
                    &data.file_loader,
                    &animation.movement_anim_clump,
                    rigid_frame_indices,
                )
            }
        });
    if let Some(rigid_transforms) = rigid_transforms
        && rigid_transforms.len() == part_count
    {
        for (entity, local) in manifest.parts.iter().zip(rigid_transforms.iter()) {
            let Ok((_, _, _, mut transform)) = parts.get_mut(*entity) else {
                warn_once!(local_player, "arena_pose_manifest_member_retired");
                return false;
            };
            *transform = *local;
        }
        return true;
    }

    let drawables = if let Some(association) = vehicle_association {
        build_animated_ped_material_drawables_from_association(
            model_index,
            &data.file_loader,
            association,
        )
    } else if use_jump_association {
        player.jump_association.as_ref().map_or_else(
            || {
                build_animated_ped_material_drawables(
                    model_index,
                    &data.file_loader,
                    &animation.movement_anim_clump,
                )
            },
            |association| {
                build_animated_ped_material_drawables_from_association(
                    model_index,
                    &data.file_loader,
                    association,
                )
            },
        )
    } else {
        build_animated_ped_material_drawables(
            model_index,
            &data.file_loader,
            &animation.movement_anim_clump,
        )
    };
    if drawables.is_empty() || drawables.len() != part_count {
        warn_once!(local_player, "arena_pose_unresolved_original_hierarchy_or_part_count");
        return false;
    }
    let mut animated_meshes = drawables
        .into_iter()
        .map(|drawable| Some(drawable.mesh))
        .collect::<Vec<_>>();
    let mut all_meshes_applied = true;
    for entity in &manifest.parts {
        let Ok((part, mesh_handle, _, _)) = parts.get(*entity) else {
            warn_once!(local_player, "arena_pose_manifest_member_retired");
            return false;
        };
        let Some(animated_mesh) = animated_meshes
            .get_mut(part.part_index)
            .and_then(Option::take)
        else {
            warn_once!(local_player, "arena_pose_missing_animated_part");
            all_meshes_applied = false;
            continue;
        };
        if let Some(mut mesh) = meshes.get_mut(mesh_handle.0.id()) {
            *mesh = animated_mesh;
        } else {
            warn_once!(local_player, "arena_pose_missing_mesh_asset");
            all_meshes_applied = false;
        }
    }
    // The CPU-skinned path already bakes every bone transform into vertex
    // positions. A prior rigid-frame pose must not remain on the child entity
    // or a run/jump association switch applies two unrelated transforms and
    // tears the ped into separate limbs.
    for entity in &manifest.parts {
        let Ok((_, _, _, mut transform)) = parts.get_mut(*entity) else {
            warn_once!(local_player, "arena_pose_manifest_member_retired");
            return false;
        };
        *transform = Transform::IDENTITY;
    }
    all_meshes_applied && animated_meshes.iter().all(Option::is_none)
}

fn animation_spec(animation: ArenaAnimation) -> (usize, i32) {
    match animation {
        ArenaAnimation::Idle => (ASSOCGRP_PLAYER, ANIM_STD_IDLE),
        ArenaAnimation::Walk => (ASSOCGRP_PLAYER, ANIM_STD_WALK),
        ArenaAnimation::Run => (ASSOCGRP_PLAYER, ANIM_STD_RUN),
        ArenaAnimation::Sprint => (ASSOCGRP_PLAYER, ANIM_STD_RUNFAST),
        ArenaAnimation::Jump => (ASSOCGRP_STD, ANIM_STD_JUMP_LAUNCH),
        ArenaAnimation::EnterVehicle => (ASSOCGRP_STD, ANIM_STD_CAR_GET_IN_LHS),
        ArenaAnimation::Fight => (ASSOCGRP_STD, ANIM_STD_FIGHT_IDLE),
    }
}

fn axis(keys: &ButtonInput<KeyCode>, negative: KeyCode, positive: KeyCode) -> f32 {
    f32::from(keys.pressed(positive)) - f32::from(keys.pressed(negative))
}

fn wrap_camera_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Convert the arena's Bevy orbit yaw to re3's
/// `CCamera::Orientation = Atan2(GetForward().x, GetForward().y)`.
///
/// `view_forward` is `(-sin(yaw), 0, -cos(yaw))` in Bevy. `CVector::from_bevy`
/// maps that to GTA `(-sin(yaw), cos(yaw), 0)`, whose re3 orientation is
/// `-yaw`. Adding PI here used to make every on-foot input face away from the
/// viewport even though the orbit camera itself was placed correctly.
fn gta_camera_orientation(camera_yaw: f32) -> f32 {
    CGeneral::limit_radian_angle(-camera_yaw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_animation_buttons_map_to_re3_association_groups_and_ids() {
        assert_eq!(
            animation_spec(ArenaAnimation::Idle),
            (ASSOCGRP_PLAYER, ANIM_STD_IDLE)
        );
        assert_eq!(
            animation_spec(ArenaAnimation::Jump),
            (ASSOCGRP_STD, ANIM_STD_JUMP_LAUNCH)
        );
        assert_eq!(
            animation_spec(ArenaAnimation::EnterVehicle),
            (ASSOCGRP_STD, ANIM_STD_CAR_GET_IN_LHS)
        );
    }

    #[test]
    fn arena_orbit_yaw_converts_to_re3_camera_orientation() {
        assert_eq!(gta_camera_orientation(0.0), 0.0);
        assert!(
            (gta_camera_orientation(std::f32::consts::FRAC_PI_2) + std::f32::consts::FRAC_PI_2)
                .abs()
                < 1.0e-6
        );
        assert!(
            (gta_camera_orientation(-std::f32::consts::FRAC_PI_2) - std::f32::consts::FRAC_PI_2)
                .abs()
                < 1.0e-6
        );
    }

    #[test]
    fn arena_pitch_wraps_without_the_old_vertical_clamp() {
        let above_old_limit = 1.4;
        assert!((wrap_camera_angle(above_old_limit) - above_old_limit).abs() < 1.0e-6);
        assert!(
            (wrap_camera_angle(std::f32::consts::PI + 0.2) - (-std::f32::consts::PI + 0.2)).abs()
                < 1.0e-6
        );
    }

    #[test]
    fn forward_pad_heading_matches_the_arena_view_direction() {
        for yaw in [
            0.0,
            std::f32::consts::FRAC_PI_2,
            -std::f32::consts::FRAC_PI_2,
        ] {
            let pad_heading = CGeneral::get_radian_angle_between_points(0.0, 0.0, 0.0, -128.0);
            let ped_heading =
                CGeneral::limit_radian_angle(pad_heading - gta_camera_orientation(yaw));
            let ped_forward =
                Vec3::new(-ped_heading.sin(), 0.0, -ped_heading.cos()).normalize_or_zero();
            let camera_forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos()).normalize_or_zero();
            assert!(
                ped_forward.dot(camera_forward) > 0.999_99,
                "W must face the same horizontal direction as the viewport at yaw {yaw}"
            );
        }
    }
}
