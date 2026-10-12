//! Local split-screen extension.
//!
//! re3 references:
//! - `core/config.h`: PC `NUMPLAYERS=1` (the mod does not alter SCM ownership).
//! - `core/Pad.cpp`: two `CPad` snapshots and original dead-zone/button ranges.
//! - `peds/PlayerPed.cpp`: `SetupPlayerPed` and per-frame player control.
//! - `core/Cam.cpp`: follow-ped and cam-on-a-string camera state.
//!
//! Player zero remains `CWorld::PlayerInFocus` for SCM, saves, missions and
//! cutscenes. Player one reuses the same ported control/physics algorithms as
//! a Bevy-owned local companion and never introduces a fake world or asset.

use bevy::{
    camera::Viewport,
    ecs::system::SystemParam,
    input::gamepad::GamepadConnectionEvent,
    prelude::*,
    render::view::{DebandDither, Msaa, NoIndirectDrawing, Tonemapping},
    window::PrimaryWindow,
};
use re3bevy_assets::assets::WorldAssets::CSurfaceTableAsset;
use re3bevy_core::core::{
    Coordinates::CVector, GameState::GtaAppState, General::CGeneral, Schedules::GtaProcessSet,
    Timer::CTimer,
};

use crate::{
    animation::{AnimManager::CAnimManager, CutsceneMgr::CCutsceneMgr},
    audio::{
        DMAudio::cDMAudio,
        soundlist::{SOUND_FALL_COLLAPSE, SOUND_FALL_LAND},
    },
    control::{
        Darkel::CDarkel,
        Replay::{CReplay, ReplayMode},
    },
    core::{
        Camera::{CCamera, CameraScriptMode},
        EventList::CEventList,
        FileLoader::CFileLoader,
        Game::{
            ScriptEntityWorld, ScriptGameplayBridge,
            apply_player_ped_air_resistance, apply_player_running_land,
            apply_player_standing_update_position, automobile_road_contacts,
            begin_player_collision_frame, finish_player_collision_frame,
            follow_script_player_camera, player_air_ground_below,
            player_can_start_jump_from_surface, player_check_if_in_air_has_ground,
            player_check_if_in_air_has_static_foot_sphere, player_current_surface_motion,
            player_dynamic_ground_below, player_dynamic_ground_support,
            player_dynamic_support_from_air_probe, player_ground_support_extension,
            player_jump_dynamic_sphere_blocked, player_static_ground_support,
            prepare_player_physical_for_control, process_player_weapon,
            process_player_weapon_switch, process_script_entity_collisions,
            resolve_player_dynamic_motion, resolve_player_ped_static_collision_result,
            script_object_collision_snapshots, service_player_air_state_before_collision,
            set_player_jump_animation, update_gameplay_mouse_capture, update_pads_from_bevy_input,
            update_player_current_phys_surface, update_player_jump_animation,
            update_player_movement_animation, update_player_on_foot_control,
        },
        Pad::{CControllerState, CPads},
        PlayerInfo::CPlayerInfo,
        Stats::CStats,
    },
    entities::{
        Entity::eEntityStatus,
        Physical::{GRAVITY_WORLD_UNITS_PER_SECOND_SQ, GTA_PHYSICS_HZ},
    },
    modelinfo::ModelInfo::ModelInfoSlot,
    peds::{
        Ped::{PedState, eMoveState, eObjective},
        PedFight::CFightMoveTable,
        PedType::{CPedStatsRuntime, CPedTypeRuntime},
        PlayerPed::{
            CPlayerPed, CWorldPlayers, begin_player_jump, process_player_unarmed_fight,
            update_player_weapon_attack_input,
        },
        VehicleRelations::{InVehicle, VehicleSeat},
    },
    renderer::{BevyRender::GtaSceneCamera, Particle::CParticle, Weather::CWeather},
    vehicles::{
        Automobile::{
            CAutomobileControlInput, CAutomobileDriveStepInput, CarDebrisComponent,
            vehicle_component_world_matrix,
        },
        HandlingMgr::cHandlingDataMgr,
        Vehicle::eCarLock,
    },
    weapons::{
        WeaponInfo::{CWeaponInfoStore, WEAPONFLAG_HEAVY},
        WeaponType::WEAPONTYPE_TOTAL_INVENTORY_WEAPONS,
    },
};

pub const LOCAL_COOP_PLAYER_COUNT: usize = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LocalCoopSplitLayout {
    #[default]
    TopBottom,
    LeftRight,
}

impl LocalCoopSplitLayout {
    pub fn toggled(self) -> Self {
        match self {
            Self::TopBottom => Self::LeftRight,
            Self::LeftRight => Self::TopBottom,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct LocalCoopSession {
    pub enabled: bool,
    pub player_two_spawned: bool,
    pub connected_gamepads: usize,
    pub split_layout: LocalCoopSplitLayout,
    pub swap_sides: bool,
    pub revive_active: bool,
    pub revive_timer: f32,
    pub revive_target_player: usize,
    mirrored_weapon_slots: [bool; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
}

impl Default for LocalCoopSession {
    fn default() -> Self {
        Self {
            enabled: false,
            player_two_spawned: false,
            connected_gamepads: 0,
            split_layout: LocalCoopSplitLayout::TopBottom,
            swap_sides: false,
            revive_active: false,
            revive_timer: 0.0,
            revive_target_player: 0,
            mirrored_weapon_slots: [false; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
        }
    }
}

impl LocalCoopSession {
    pub fn enable(&mut self) {
        self.enabled = true;
        self.player_two_spawned = false;
        self.mirrored_weapon_slots = [false; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS];
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.player_two_spawned = false;
        self.mirrored_weapon_slots = [false; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS];
    }

    pub fn toggle(&mut self) -> bool {
        if self.enabled {
            self.disable();
        } else {
            self.enable();
        }
        self.enabled
    }
}

#[derive(Debug, Clone, Resource)]
pub struct LocalCoopCameras {
    pub player_two: CCamera,
}

impl Default for LocalCoopCameras {
    fn default() -> Self {
        let mut player_two = CCamera::default();
        player_two.restore_with_jump_cut();
        Self { player_two }
    }
}

pub struct LocalCoopPlugin;

impl Plugin for LocalCoopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalCoopSession>()
            .init_resource::<LocalCoopCameras>()
            .add_systems(
                PreUpdate,
                track_connected_gamepads.after(bevy::input::InputSystems),
            )
            .add_systems(OnEnter(GtaAppState::InGame), reset_local_coop_runtime)
            .add_systems(
                Update,
                route_local_gamepads
                    .after(update_pads_from_bevy_input)
                    .before(update_gameplay_mouse_capture)
                    .in_set(GtaProcessSet::Pad)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    sync_second_player_lifecycle
                        .after(update_player_on_foot_control)
                        .before(process_script_entity_collisions),
                    process_second_player
                        .after(sync_second_player_lifecycle)
                        .run_if(crate::core::World::world_actor_processing_enabled)
                        .before(process_script_entity_collisions),
                    process_local_coop_revive
                        .after(process_second_player)
                        .run_if(crate::core::World::world_actor_processing_enabled)
                        .before(process_script_entity_collisions),
                )
                    .chain()
                    .in_set(GtaProcessSet::World)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                sync_split_screen_cameras
                    .after(follow_script_player_camera)
                    .in_set(GtaProcessSet::Camera)
                    .run_if(in_state(GtaAppState::InGame)),
            );
    }
}

fn track_connected_gamepads(
    gamepads: Query<Entity, With<Gamepad>>,
    mut events: MessageReader<GamepadConnectionEvent>,
    mut session: ResMut<LocalCoopSession>,
) {
    session.connected_gamepads = gamepads.iter().count();
    for event in events.read() {
        info!(
            "gamepad {:?}: {:?} | connected total {}",
            event.gamepad, event.connection, session.connected_gamepads
        );
    }
}

fn reset_local_coop_runtime(
    mut session: ResMut<LocalCoopSession>,
    mut cameras: ResMut<LocalCoopCameras>,
) {
    session.player_two_spawned = false;
    session.mirrored_weapon_slots = [false; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS];
    cameras.player_two.restore_with_jump_cut();
}

fn route_local_gamepads(
    gamepads: Query<(Entity, &Gamepad)>,
    players: CWorldPlayers,
    timer: Res<CTimer>,
    mut pads: ResMut<CPads>,
    mut session: ResMut<LocalCoopSession>,
    mut cameras: ResMut<LocalCoopCameras>,
) {
    if !session.enabled {
        return;
    }
    let mut connected = gamepads.iter().collect::<Vec<_>>();
    connected.sort_unstable_by_key(|(entity, _)| entity.to_bits());
    session.connected_gamepads = connected.len();

    for local_player in 0..LOCAL_COOP_PLAYER_COUNT {
        let pad_index = if session.swap_sides {
            (LOCAL_COOP_PLAYER_COUNT - 1).saturating_sub(local_player)
        } else {
            local_player
        };
        let state = connected
            .get(pad_index)
            .map(|(_, gamepad)| {
                controller_state_from_gamepad(
                    gamepad,
                    players.relation(local_player as i32).is_some(),
                )
            })
            .unwrap_or_default();
        if state.has_input() || connected.len() > pad_index {
            pads.get_pad_mut(local_player as i32).Update(state);
        }
    }

    // `CCam::Process_FollowPedWithMouse` non-mouse branch, applied to P2's
    // independent camera state with the same thresholds and re3's 50 Hz time unit.
    let pad = pads.get_pad(1);
    let gta_time_step = timer.time_step_in_seconds().clamp(0.0, 0.1) * GTA_PHYSICS_HZ;
    let fov_scale = 70.0 / 80.0;
    cameras.player_two.follow_beta +=
        -(pad.look_around_left_right() as f32) * 0.01 * (1.0 / 14.0) * fov_scale * gta_time_step;
    cameras.player_two.follow_alpha = (cameras.player_two.follow_alpha
        + pad.look_around_up_down() as f32 * 0.01 * (0.6 / 14.0) * fov_scale * gta_time_step)
        .clamp(-89.5_f32.to_radians(), 45.0_f32.to_radians());
}

pub fn controller_state_from_gamepad(
    gamepad: &Gamepad,
    player_in_vehicle: bool,
) -> CControllerState {
    let dead_zone = |x: f32, y: f32| {
        if x.abs() > 0.25 || y.abs() > 0.25 {
            Vec2::new(x, y)
        } else {
            Vec2::ZERO
        }
    };
    let left = dead_zone(
        gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0),
        gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0),
    );
    let right = dead_zone(
        gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0),
        gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0),
    );
    let axis = |value: f32| (value.clamp(-1.0, 1.0) * 128.0).round() as i16;
    let button = |pressed: bool| if pressed { 255 } else { 0 };
    let pressure = |button: GamepadButton| {
        (gamepad.get(button).unwrap_or(0.0).clamp(0.0, 1.0) * 255.0).round() as i16
    };

    let dpad_left = gamepad.pressed(GamepadButton::DPadLeft);
    let dpad_right = gamepad.pressed(GamepadButton::DPadRight);

    let cycle_weapon_left =
        button(dpad_left || (!player_in_vehicle && gamepad.pressed(GamepadButton::LeftTrigger)));
    let cycle_weapon_right =
        button(dpad_right || (!player_in_vehicle && gamepad.pressed(GamepadButton::RightTrigger)));

    let attack = button(
        gamepad.pressed(GamepadButton::East)
            || (!player_in_vehicle
                && gamepad.get(GamepadButton::RightTrigger2).unwrap_or(0.0) > 0.2),
    );

    CControllerState {
        left_stick_x: axis(left.x),
        left_stick_y: axis(-left.y),
        right_stick_x: axis(right.x),
        right_stick_y: axis(-right.y),
        left_shoulder1: button(gamepad.pressed(GamepadButton::LeftTrigger)),
        left_shoulder2: if player_in_vehicle {
            pressure(GamepadButton::LeftTrigger2)
        } else {
            cycle_weapon_left
        },
        right_shoulder1: button(gamepad.pressed(GamepadButton::RightTrigger)),
        right_shoulder2: if player_in_vehicle {
            pressure(GamepadButton::RightTrigger2)
        } else {
            cycle_weapon_right
        },
        dpad_up: button(gamepad.pressed(GamepadButton::DPadUp)),
        dpad_down: button(gamepad.pressed(GamepadButton::DPadDown)),
        dpad_left: button(dpad_left),
        dpad_right: button(dpad_right),
        start: button(gamepad.pressed(GamepadButton::Start)),
        select: button(gamepad.pressed(GamepadButton::Select)),
        square: button(gamepad.pressed(GamepadButton::West)).max(if player_in_vehicle {
            pressure(GamepadButton::LeftTrigger2)
        } else {
            0
        }),
        triangle: button(gamepad.pressed(GamepadButton::North)),
        cross: button(gamepad.pressed(GamepadButton::South)).max(if player_in_vehicle {
            pressure(GamepadButton::RightTrigger2)
        } else {
            0
        }),
        circle: attack,
        left_shock: button(gamepad.pressed(GamepadButton::LeftThumb)),
        right_shock: button(gamepad.pressed(GamepadButton::RightThumb)),
    }
}

fn sync_second_player_lifecycle(
    mut session: ResMut<LocalCoopSession>,
    bridge: Res<ScriptGameplayBridge>,
    _cutscene: Option<Res<CCutsceneMgr>>,
    mut players: CWorldPlayers,
    mut entities: ScriptEntityWorld,
    mut general: ResMut<CGeneral>,
    ped_types: Option<Res<CPedTypeRuntime>>,
) {
    if !session.enabled {
        if players.relation(1).is_some()
            && let Some(player_entity) = players.player_entity(1)
        {
            let _ = entities.detach_ped_from_vehicle(player_entity);
        }
        players.remove_player(1);
        session.player_two_spawned = false;
        session.mirrored_weapon_slots = [false; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS];
        return;
    }
    // The script bridge is the authoritative owner of player visibility and
    // controls (re3's MakePlayerSafeForCutscene/DeleteCutsceneData).  The
    // cutscene manager keeps `loaded` true between LOAD_CUTSCENE and CLEAR;
    // using that flag here made a local companion remain disabled after the
    // first mission intro.  Do not infer gameplay ownership from asset state.
    let cutscene_active = bridge.playing_intro
        || !bridge.player_visible
        || !bridge.player_control_enabled
        || bridge.cutscene_started;
    let player_one_arsenal = players
        .player(0)
        .map(|player| (player.ped.weapons, player.ped.current_weapon));
    if let Some(mut player_two) = players.player_mut(1) {
        if let Some((weapons, current_weapon)) = player_one_arsenal {
            for (slot, weapon) in weapons.into_iter().enumerate().skip(1) {
                if !session.mirrored_weapon_slots[slot] && weapon.ammo_total > 0 {
                    player_two.ped.weapons[slot] = weapon;
                    session.mirrored_weapon_slots[slot] = true;
                }
            }
            let current_slot = current_weapon as usize;
            if player_two.ped.current_weapon as usize == 0
                && session
                    .mirrored_weapon_slots
                    .get(current_slot)
                    .copied()
                    .unwrap_or(false)
            {
                player_two.ped.current_weapon = current_weapon;
                player_two.selected_wep_slot = current_weapon;
            }
        }
        player_two.active_in_world = !cutscene_active;
        session.player_two_spawned = true;
        players.set_player_in_focus(0);
        return;
    }
    if cutscene_active || !bridge.player_spawned || !bridge.player_control_enabled {
        return;
    }
    let Some(player_one) = players.player(0) else {
        return;
    };
    let heading = player_one.locomotion.rotation_cur;
    let origin = player_one.position();
    let arsenal = player_one.ped.weapons;
    let current_weapon = player_one.ped.current_weapon;
    let spawn = CVector::new(
        origin.x + heading.cos() * 1.5,
        origin.y + heading.sin() * 1.5,
        origin.z,
    );
    CPlayerPed::setup_player_ped(&mut players, 1, &mut general, ped_types.as_deref());
    if let Some(mut player_two) = players.player_mut(1) {
        player_two.set_position(spawn);
        player_two.set_heading_degrees(player_one_heading_degrees(heading));
        player_two.ped.weapons = arsenal;
        player_two.ped.current_weapon = current_weapon;
        player_two.selected_wep_slot = current_weapon;
        player_two.active_in_world = true;
    }
    for (slot, weapon) in arsenal.into_iter().enumerate().skip(1) {
        session.mirrored_weapon_slots[slot] = weapon.ammo_total > 0;
    }
    players.set_player_in_focus(0);
    session.player_two_spawned = players.player(1).is_some();
    players.flush_pending();
}

fn player_one_heading_degrees(heading: f32) -> f32 {
    heading.to_degrees()
}

#[derive(SystemParam)]
struct SecondPlayerFrame<'w> {
    vehicle_spatial: Res<'w, crate::collision::VehicleSpatial::VehicleCollisionSpatial>,
    pads: Res<'w, CPads>,
    timer: ResMut<'w, CTimer>,
    general: ResMut<'w, CGeneral>,
    handling: ResMut<'w, cHandlingDataMgr>,
    surface_table: Res<'w, CSurfaceTableAsset>,
    weather: Res<'w, CWeather>,
    file_loader: Res<'w, CFileLoader>,
    anim_manager: Res<'w, CAnimManager>,
    weapon_info: Res<'w, CWeaponInfoStore>,
    ped_stats: Res<'w, CPedStatsRuntime>,
    fight_moves: Res<'w, CFightMoveTable>,
    particles: ResMut<'w, CParticle>,
    particle_systems: Option<Res<'w, crate::renderer::ParticleMgr::cParticleSystemMgr>>,
    audio: Option<ResMut<'w, cDMAudio>>,
    darkel: Option<ResMut<'w, CDarkel>>,
    stats: Option<ResMut<'w, CStats>>,
    events: ResMut<'w, CEventList>,
    replay: Option<Res<'w, CReplay>>,
    player_info: Option<Res<'w, CPlayerInfo>>,
    projectiles: Option<ResMut<'w, crate::weapons::ProjectileInfo::CProjectileInfoPool>>,
    explosions: Option<ResMut<'w, crate::weapons::Explosion::CExplosionPool>>,
    special_fx: Option<ResMut<'w, crate::renderer::SpecialFX::CSpecialFX>>,
}

fn process_second_player(
    mut session: ResMut<LocalCoopSession>,
    bridge: Res<ScriptGameplayBridge>,
    mut cameras: ResMut<LocalCoopCameras>,
    mut players: CWorldPlayers,
    mut entities: ScriptEntityWorld,
    mut frame: SecondPlayerFrame,
) {
    if !session.enabled || !session.player_two_spawned || bridge.playing_intro {
        return;
    }
    let object_colliders = script_object_collision_snapshots(&entities.objects);
    // A missing/stale connection event must not become a gameplay lock.  Pad
    // one is always a valid re3 `Pads[1]` snapshot; when no second device is
    // present it simply contains the neutral state.  This mirrors re3's pad
    // ownership without coupling player control to Bevy's connection count.
    let control_enabled =
        bridge.player_control_enabled && !bridge.playing_intro && !bridge.cutscene_started;
    let pad = *frame.pads.get_pad(1);
    let dt = frame.timer.time_step_in_seconds().clamp(0.0, 0.05);
    let now_ms = frame.timer.time_in_milliseconds();
    let player_relation = players.relation(1);
    let Some(player_entity) = players.player_entity(1) else {
        return;
    };
    let (player_opt, mut other_player) = players.get_pair_mut(1, 0);
    let Some(mut player) = player_opt else {
        return;
    };
    let player = &mut player;
    if !player.active_in_world {
        return;
    }
    player.dynamic_collision_probe = None;
    player.dynamic_collision_target = None;

    // Player two keeps the same ECS InVehicle relation after death as player
    // one.  re3 never re-enters the driving branch from PED_DIE/PED_DEAD.
    if player.ped.dying_or_dead() && player_relation.is_some() {
        return;
    }
    if player.ped.ped_state == PedState::PED_DEAD {
        return;
    }

    if player_relation.is_some() {
        process_second_player_vehicle(
            player_entity,
            player_relation.expect("vehicle branch requires relation"),
            player,
            &pad,
            control_enabled,
            &mut cameras.player_two,
            &mut entities,
            &mut frame,
            dt,
        );
        return;
    }

    if control_enabled
        && pad.exit_vehicle_just_down()
        && !matches!(
            player.ped.ped_state,
            PedState::PED_FALL | PedState::PED_DIE | PedState::PED_DEAD
        )
    {
        if !session.revive_active {
            if let Some(other) = other_player.as_mut() {
                let other_dead = other.vitality.health <= 0.0
                    || matches!(other.ped.ped_state, PedState::PED_DIE | PedState::PED_DEAD);
                let ppos = player.position();
                let opos = other.position();
                let dx = opos.x - ppos.x;
                let dy = opos.y - ppos.y;
                let dz = opos.z - ppos.z;
                if other_dead && (dx * dx + dy * dy + dz * dz) <= 3.0 * 3.0 {
                    session.revive_active = true;
                    session.revive_timer = 2.5;
                    session.revive_target_player = 0;
                    player.ped.physical.move_speed = CVector::ZERO;
                    player.move_speed = 0.0;
                    player.ped.move_state = eMoveState::PEDMOVE_STILL;
                    let angle = dy.atan2(dx) - std::f32::consts::FRAC_PI_2;
                    player.locomotion.rotation_cur = angle;
                    player.locomotion.rotation_dest = angle;
                    if let Some(mut association) = frame.anim_manager.create_anim_association(
                        crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
                        crate::animation::AnimationId::ANIM_MEDIC_CPR as usize,
                    ) {
                        association.set_current_time(0.0);
                        association.set_run();
                        player.animation.movement_anim_clump.associations.push(association);
                    }
                    if let Some(audio) = frame.audio.as_deref_mut() {
                        audio.PlayOneShot(
                            player.ped.physical.audio_entity_id,
                            crate::audio::soundlist::SOUND_PED_HEALING as u16,
                            0.0,
                        );
                    }
                    return;
                }
            }
        }
        let player_position = player.position();
        let nearest = entities
            .vehicles
            .iter()
            .filter_map(|(&handle, vehicle)| {
                let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
                    return None;
                };
                let base = &car.vehicle;
                if base.flags.is_locked
                    || base.entry.door_lock == eCarLock::CARLOCK_LOCKED
                    || base.physical.entity.status == eEntityStatus::STATUS_WRECKED
                {
                    return None;
                }
                let position = base.physical.entity.get_position();
                let delta = CVector::new(
                    position.x - player_position.x,
                    position.y - player_position.y,
                    position.z - player_position.z,
                );
                let distance_sq = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;
                (distance_sq <= 25.0).then_some((handle, distance_sq))
            })
            .min_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(handle, _)| handle);
        if let Some(handle) = nearest {
            let vehicle_ent = entities.vehicle_entity(handle);
            let driver_occupied = vehicle_ent.is_some_and(|veh| {
                entities.relations.iter().any(|(_, rel)| {
                    rel.is_some_and(|r| r.vehicle == veh && r.seat == VehicleSeat::Driver)
                })
            });
            let on_passenger_side = if let Some(veh) = entities.vehicles.get(&entities.index, &handle) {
                let right = veh.base().physical.entity.get_right();
                let vpos = veh.base().physical.entity.get_position();
                let dx = player_position.x - vpos.x;
                let dy = player_position.y - vpos.y;
                (dx * right.x + dy * right.y) > 0.0
            } else {
                false
            };
            let seats_to_try = if driver_occupied || on_passenger_side {
                [
                    VehicleSeat::Passenger(0),
                    VehicleSeat::Passenger(1),
                    VehicleSeat::Passenger(2),
                    VehicleSeat::Driver,
                ]
            } else {
                [
                    VehicleSeat::Driver,
                    VehicleSeat::Passenger(0),
                    VehicleSeat::Passenger(1),
                    VehicleSeat::Passenger(2),
                ]
            };
            for seat in seats_to_try {
                if entities.warp_player_into_car(
                    player_entity,
                    player,
                    handle,
                    seat,
                    &frame.file_loader,
                ) {
                    cameras.player_two.follow_initialised = false;
                    return;
                }
            }
        }
    }

    // Same transient/friction half of CPhysical::ProcessControl as P1.
    let hit_something_last_frame = prepare_player_physical_for_control(&mut player.ped);
    apply_player_ped_air_resistance(&mut player.ped, dt);

    // Same CPed::Fall/Die control gate used by player zero. P2 remains a
    // companion (SCM never owns it), but its physical state is still re3's.
    if matches!(
        player.ped.ped_state,
        PedState::PED_FALL | PedState::PED_GETUP | PedState::PED_DIE | PedState::PED_DEAD
    ) {
        const PED_BASE_OFFSET: f32 = 1.04;
        let old_position = player.position();
        let moved = player.ped.calculate_new_velocity(player.locomotion, player.vitality,
            frame.timer.time_step(), false);
        if player.ped.flags.is_standing {
            player.ped.physical.entity.set_heading(player.locomotion.rotation_cur);
        }
        let surface = player_current_surface_motion(player.ped, &entities);
        apply_player_standing_update_position(player.ped, player.locomotion,
            CVector::new(moved.x * frame.timer.time_step(), moved.y * frame.timer.time_step(), 0.0),
            surface, dt);
        begin_player_collision_frame(&mut player.ped);
        let mut collision_rejected_move = false;
        let mut position = old_position;
        let next_vertical_speed =
            player.ped.physical.move_speed.z - GRAVITY_WORLD_UNITS_PER_SECOND_SQ * dt;
        position.x += player.ped.physical.move_speed.x * dt;
        position.y += player.ped.physical.move_speed.y * dt;
        position.z += next_vertical_speed * dt;
        player.ped.physical.move_speed.z = next_vertical_speed;
        let attempted_position = position;
        let static_collision = resolve_player_ped_static_collision_result(
            old_position,
            position,
            &mut player.ped,
            &frame.file_loader,
            &frame.surface_table,
            frame.timer.time_step(),
            now_ms,
        );
        position = static_collision.position;
        collision_rejected_move |= static_collision.movement_rejected;
        player.ped.flags.hit_steep_slope |= static_collision.hit_steep_slope;
        let attempted_delta_z = attempted_position.z - old_position.z;
        if (attempted_delta_z > 0.0 && position.z < attempted_position.z - 1.0e-5)
            || (attempted_delta_z < 0.0 && position.z > attempted_position.z + 1.0e-5)
        {
            player.ped.physical.move_speed.z = 0.0;
        }
        if (position.x - attempted_position.x).abs() > 1.0e-5
            || (position.y - attempted_position.y).abs() > 1.0e-5
            || (position.z - attempted_position.z).abs() > 1.0e-5
        {
            player.ped.physical.entity.flags.has_hit_wall = true;
        }
        let dynamic_target_position = position;
        let dynamic_motion = resolve_player_dynamic_motion(
            old_position,
            dynamic_target_position,
            &player.ped,
            &entities,
            &object_colliders,
            &frame.file_loader,
            None,
        );
        position = dynamic_motion.position;
        if let Some(support) = dynamic_motion.landing_support {
            player.ped.physical.move_speed.z = 0.0;
            player.ped.flags.is_standing = true;
            player.ped.flags.is_in_the_air = false;
            update_player_current_phys_surface(&mut player.ped, None, Some(support), true);
        }
        if let Some(probe) = dynamic_motion.collision_probe {
            collision_rejected_move = true;
            player.dynamic_collision_probe = Some(probe);
            player.dynamic_collision_target = Some(CVector::new(
                dynamic_target_position.x,
                dynamic_target_position.y,
                if dynamic_motion.landing_support.is_some() {
                    dynamic_motion.position.z
                } else {
                    dynamic_target_position.z
                },
            ));
        }
        let static_support = player_static_ground_support(
            position.x,
            position.y,
            position.z,
            &player.ped,
            dt,
            &frame.file_loader,
        );
        let dynamic_support = player_dynamic_ground_support(
            position.x,
            position.y,
            position.z,
            &player.ped,
            dt,
            &entities,
            &object_colliders,
            &frame.file_loader,
        );
        let ground = match (
            static_support.map(|support| support.z),
            dynamic_support.map(|support| support.z),
        ) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (left, right) => left.or(right),
        };
        let collision_established_standing = player.ped.flags.is_standing;
        let grounded = crate::peds::PlayerPed::player_grounded_after_collision_or_support(
            collision_established_standing,
            position.z,
            ground,
            player_ground_support_extension(player.ped, dt),
            player.ped.physical.move_speed.z,
        );
        if !collision_established_standing || static_support.is_some() || dynamic_support.is_some() {
            update_player_current_phys_surface(player.ped, static_support, dynamic_support, grounded);
        }
        if grounded {
            if let Some(ground) = ground {
                position.z = ground + PED_BASE_OFFSET;
            }
            player.ped.physical.move_speed.z = 0.0;
            player.ped.flags.is_in_the_air = false;
            player.ped.flags.is_standing = true;
        } else {
            player.ped.flags.is_in_the_air = true;
            player.ped.flags.is_standing = false;
        }
        player.set_position(position);
        let recovery_inputs = crate::peds::PedRecovery::PedRecoveryInput {
            anim_manager: &frame.anim_manager,
            file_loader: &frame.file_loader,
            timer: &frame.timer,
            player_can_be_damaged: player.can_be_damaged,
            player_controls_disabled: frame.pads.get_pad(0).disable_player_controls,
        };
        crate::peds::PedRecovery::service_player_fall_getup(player, &recovery_inputs,
            &entities.index, &frame.vehicle_spatial, |entity| {
                let vehicle = entities.vehicles.get_entity(entity)?;
                (entities.index.vehicle(vehicle.handle()) == Some(entity))
                    .then_some(vehicle.base().core)
            }, &mut frame.general);
        finish_player_collision_frame(
            &mut player.ped,
            old_position,
            position,
            collision_rejected_move,
        );
        player.set_position(position);
        return;
    }

    let old_pos = player.position();
    // Collision contact clears bIsInTheAir before the jump state machine runs.
    // Preserve the frame-entry value so touchdown is routed through SetLanding
    // semantics instead of being mistaken for the grounded launch phase.
    let was_airborne = player.ped.flags.is_in_the_air;
    let started_standing = player.ped.flags.is_standing;
    let physical_motion_active = !started_standing;
    let mut integrated_physical_vertical_speed = None;
    if physical_motion_active {
        player.ped.physical.move_speed.z -= GRAVITY_WORLD_UNITS_PER_SECOND_SQ * dt;
        integrated_physical_vertical_speed = Some(player.ped.physical.move_speed.z);
    }
    let gta_time_step = dt * GTA_PHYSICS_HZ;
    let previous_surface_motion = player_current_surface_motion(&player.ped, &entities);
    let moved_native = player.ped.calculate_new_velocity(player.locomotion, &player.vitality, gta_time_step, true);
    player
        .ped
        .physical
        .entity
        .set_heading(player.locomotion.rotation_cur);
    let previous_animation_world_delta = CVector::new(
        moved_native.x * gta_time_step,
        moved_native.y * gta_time_step,
        0.0,
    );
    let mut position = player.position();
    let mut standing_motion_active = false;
    if started_standing {
        let standing_delta = apply_player_standing_update_position(
            &mut player.ped,
            &mut player.locomotion,
            previous_animation_world_delta,
            previous_surface_motion,
            dt,
        );
        position.x += standing_delta.x;
        position.y += standing_delta.y;
        standing_motion_active = standing_delta.x.abs() > 1.0e-6 || standing_delta.y.abs() > 1.0e-6;
    }
    let pre_collision_air_ground =
        player_air_ground_below(old_pos, &entities, &object_colliders, &frame.file_loader);
    if let Some(was_falling) = service_player_air_state_before_collision(
        player,
        &frame.anim_manager,
        old_pos,
        pre_collision_air_ground,
        &frame.file_loader,
    ) && let Some(audio) = frame.audio.as_deref_mut()
        && player.ped.physical.audio_entity_id >= 0
    {
        audio.PlayOneShot(
            player.ped.physical.audio_entity_id,
            if was_falling {
                SOUND_FALL_COLLAPSE
            } else {
                SOUND_FALL_LAND
            } as u16,
            0.0,
        );
    }
    let camera_heading = Some(cameras.player_two.gameplay_orientation());
    let camera_front = cameras
        .player_two
        .cams
        .get(cameras.player_two.active_cam)
        .map(|cam| cam.front);
    if player.ped.objective == eObjective::OBJECTIVE_NONE {
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
    }
    if player.ped.flags.is_landing {
        apply_player_running_land(player, &pad, &frame.anim_manager);
    }
    let heavy_weapon = frame
        .weapon_info
        .GetWeaponInfo(player.ped.current_weapon)
        .is_some_and(|info| info.IsFlagSet(WEAPONFLAG_HEAVY));
    let _moved = player.apply_on_foot_pad_input_camera(
        &pad,
        dt,
        control_enabled,
        camera_heading,
        player_relation.is_some(),
        heavy_weapon,
    );

    if player.ped.ped_state == PedState::PED_JUMP
        && player.ped.flags.is_in_the_air
        && control_enabled
        && !player.ped.flags.hit_steep_slope
        && !hit_something_last_frame
        && player.ped.physical.distance_travelled < dt
        && player.ped.physical.move_speed.magnitude_sqr() < 0.01 * GTA_PHYSICS_HZ * GTA_PHYSICS_HZ
    {
        let (sin, cos) = player.locomotion.rotation_cur.sin_cos();
        player.ped.physical.apply_move_force(CVector::new(
            -sin * 3.0 * GTA_PHYSICS_HZ,
            cos * 3.0 * GTA_PHYSICS_HZ,
            0.05 * GTA_PHYSICS_HZ,
        ));
    }

    let local_delta = update_player_movement_animation(
        player,
        &frame.anim_manager,
        &frame.file_loader,
        dt,
        false,
        frame
            .player_info
            .as_ref()
            .is_some_and(|info| info.m_bInfiniteSprint),
        now_ms,
        &mut frame.timer,
    );
    player.stage_anim_move_delta(Some(local_delta));

    if physical_motion_active {
        position.x += player.ped.physical.move_speed.x * dt;
        position.y += player.ped.physical.move_speed.y * dt;
        position.z += player.ped.physical.move_speed.z * dt;
        integrated_physical_vertical_speed = Some(player.ped.physical.move_speed.z);
    }
    begin_player_collision_frame(&mut player.ped);
    let mut collision_rejected_move = false;
    let moved = (position.x - old_pos.x).abs() > 1.0e-5
        || (position.y - old_pos.y).abs() > 1.0e-5
        || (position.z - old_pos.z).abs() > 1.0e-5;
    if standing_motion_active
        || player.ped.flags.is_in_the_air
        || player.ped.flags.is_landing
        || moved
    {
        let attempted_position = position;
        let static_collision = resolve_player_ped_static_collision_result(
            old_pos,
            position,
            &mut player.ped,
            &frame.file_loader,
            &frame.surface_table,
            frame.timer.time_step(),
            now_ms,
        );
        position = static_collision.position;
        collision_rejected_move |= static_collision.movement_rejected;
        if integrated_physical_vertical_speed.is_some() {
            integrated_physical_vertical_speed = Some(player.ped.physical.move_speed.z);
        }
        player.ped.flags.hit_steep_slope |= static_collision.hit_steep_slope;
        if (position.x - attempted_position.x).abs() > 1.0e-5
            || (position.y - attempted_position.y).abs() > 1.0e-5
            || (position.z - attempted_position.z).abs() > 1.0e-5
        {
            player.ped.physical.entity.flags.has_hit_wall = true;
        }
        let dynamic_target_position = position;
        let dynamic_motion = resolve_player_dynamic_motion(
            old_pos,
            dynamic_target_position,
            &player.ped,
            &entities,
            &object_colliders,
            &frame.file_loader,
            None,
        );
        position = dynamic_motion.position;
        if let Some(support) = dynamic_motion.landing_support {
            player.ped.physical.move_speed.z = 0.0;
            if integrated_physical_vertical_speed.is_some() {
                integrated_physical_vertical_speed = Some(0.0);
            }
            player.ped.flags.is_standing = true;
            player.ped.flags.is_in_the_air = false;
            update_player_current_phys_surface(&mut player.ped, None, Some(support), true);
        }
        if let Some(probe) = dynamic_motion.collision_probe {
            collision_rejected_move = true;
            player.dynamic_collision_probe = Some(probe);
            player.dynamic_collision_target = Some(CVector::new(
                dynamic_target_position.x,
                dynamic_target_position.y,
                if dynamic_motion.landing_support.is_some() {
                    dynamic_motion.position.z
                } else {
                    dynamic_target_position.z
                },
            ));
        }
    }
    let collision_established_standing = player.ped.flags.is_standing;

    let mut vertical_speed =
        integrated_physical_vertical_speed.unwrap_or(player.ped.physical.move_speed.z);
    let static_ground = frame
        .file_loader
        .world
        .process_line_of_sight_static_contact(
            [position.x, position.y, position.z],
            [position.x, position.y, position.z - 4.04],
            &frame.file_loader.model_info,
            None,
            false,
        )
        .map(|contact| contact.point[2]);
    let static_support = player_static_ground_support(
        position.x,
        position.y,
        position.z,
        &player.ped,
        dt,
        &frame.file_loader,
    );
    let dynamic_ground_below = player_dynamic_ground_below(
        position.x,
        position.y,
        position.z,
        &entities,
        &object_colliders,
        &frame.file_loader,
    );
    let dynamic_support =
        player_dynamic_support_from_air_probe(position.z, &player.ped, dynamic_ground_below)
            .or_else(|| {
                if player.ped.flags.was_standing {
                    player_dynamic_ground_support(
                        position.x,
                        position.y,
                        position.z,
                        &player.ped,
                        dt,
                        &entities,
                        &object_colliders,
                        &frame.file_loader,
                    )
                } else {
                    None
                }
            });
    let dynamic_ground = dynamic_ground_below.map(|support| support.z);
    let air_ground = match (static_ground, dynamic_ground) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    };
    let support_ground = match (
        static_support.map(|support| support.z),
        dynamic_support.map(|support| support.z),
    ) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    };
    const PED_BASE_OFFSET: f32 = 1.04;
    let support_extension = player_ground_support_extension(&player.ped, dt);
    let grounded = crate::peds::PlayerPed::player_grounded_after_collision_or_support(
        collision_established_standing,
        position.z,
        support_ground,
        support_extension,
        vertical_speed,
    );
    if !collision_established_standing || static_support.is_some() || dynamic_support.is_some() {
        update_player_current_phys_surface(
            &mut player.ped,
            static_support,
            dynamic_support,
            grounded,
        );
    }
    let mut horizontal_speed = CVector::new(
        player.ped.physical.move_speed.x,
        player.ped.physical.move_speed.y,
        0.0,
    );

    if player.ped.flags.is_landing {
        let touching_ground = crate::peds::PlayerPed::player_landing_touching_ground(
            collision_established_standing,
            position.z,
            support_ground,
        );
        if touching_ground {
            if let Some(z) = support_ground {
                position.z = z + PED_BASE_OFFSET;
            }
            vertical_speed = 0.0;
            player.ped.physical.move_speed.z = 0.0;
        }
        player.ped.flags.is_in_the_air = false;
        player.ped.flags.is_standing = touching_ground;
        if player.ped.ped_state == PedState::PED_JUMP {
            player.move_speed = 0.0;
        }
        if update_player_jump_animation(player, dt) {
            player.ped.flags.is_landing = false;
            if player.ped.ped_state == PedState::PED_JUMP {
                player.ped.ped_state = match player.ped.last_ped_state {
                    PedState::PED_NONE | PedState::PED_IDLE | PedState::PED_JUMP => {
                        PedState::PED_IDLE
                    }
                    previous => previous,
                };
                player.ped.last_ped_state = PedState::PED_NONE;
            }
            player.jump_association = None;
            player.jump_animation_time = 0.0;
        }
    } else if control_enabled
        && !player.ped.flags.is_in_the_air
        && player.ped.ped_state != PedState::PED_JUMP
        && pad.jump_just_down()
        && frame
            .weapon_info
            .GetWeaponInfo(player.ped.current_weapon)
            .is_none_or(|info| !info.IsFlagSet(WEAPONFLAG_HEAVY))
        && player_can_start_jump_from_surface(&player.ped, &player.locomotion)
    {
        begin_player_jump(player, &frame.anim_manager);
    } else if player.ped.ped_state == PedState::PED_JUMP
        && !player.ped.flags.is_in_the_air
        && !was_airborne
        && player.jump_association.as_ref().is_some_and(|association| {
            association.anim_id == crate::animation::AnimationId::ANIM_STD_JUMP_LAUNCH
        })
    {
        if update_player_jump_animation(player, dt) {
            // Keep local co-op on the same `CPed::FinishLaunchCB` path as
            // player zero: test the two forward head spheres against both
            // static COL and live vehicles/objects before applying the jump
            // impulse.  Without this dynamic half, P2 could launch through a
            // parked car while P1 correctly covered the hit-head animation.
            let forward = CVector::new(
                -player.locomotion.rotation_cur.sin(),
                player.locomotion.rotation_cur.cos(),
                0.0,
            );
            let ped_col_z = frame
                .file_loader
                .model_info
                .col_model(player.ped.physical.entity.model_index.max(0) as usize)
                .and_then(|col| col.spheres.first().map(|sphere| sphere.center.z))
                .unwrap_or(-0.25);
            let mut probe = CVector::new(
                position.x + forward.x * 0.15,
                position.y + forward.y * 0.15,
                position.z + ped_col_z + 0.25,
            );
            let mut obstacle = frame
                .file_loader
                .world
                .sphere_contact_buildings_only(
                    [probe.x, probe.y, probe.z],
                    0.25,
                    &frame.file_loader.model_info,
                )
                .is_some();
            obstacle |= player_jump_dynamic_sphere_blocked(
                probe,
                0.25,
                &entities,
                &object_colliders,
                &frame.file_loader,
            );
            if !obstacle {
                probe = CVector::new(
                    probe.x + forward.x * 0.15,
                    probe.y + forward.y * 0.15,
                    probe.z + 0.15,
                );
                obstacle = frame
                    .file_loader
                    .world
                    .sphere_contact_buildings_only(
                        [probe.x, probe.y, probe.z],
                        0.25,
                        &frame.file_loader.model_info,
                    )
                    .is_some();
                obstacle |= player_jump_dynamic_sphere_blocked(
                    probe,
                    0.25,
                    &entities,
                    &object_colliders,
                    &frame.file_loader,
                );
            }
            if obstacle {
                set_player_jump_animation(
                    player,
                    &frame.anim_manager,
                    crate::animation::AnimationId::ANIM_STD_HANDSCOWER,
                );
                player.ped.flags.is_landing = true;
                vertical_speed = 0.0;
            } else {
                // Native `ApplyMoveForce(0,0,8.5)` divides by mass.  The
                // player controller stores the resulting velocity in
                // world-units/second, hence the same 50 Hz conversion used by
                // the single-player path.
                vertical_speed += 8.5 / player.ped.physical.mass.max(1.0) * GTA_PHYSICS_HZ;
            }
            if !obstacle {
                let sprint_blend = player.animation.movement_anim_clump
                    .associations
                    .iter()
                    .find(|association| {
                        association.anim_id == crate::animation::AnimationId::ANIM_STD_RUNFAST
                    })
                    .map(|association| association.blend_amount);
                let run_blend = player.animation.movement_anim_clump
                    .associations
                    .iter()
                    .find(|association| {
                        association.anim_id == crate::animation::AnimationId::ANIM_STD_RUN
                    })
                    .map(|association| association.blend_amount);
                let minimum_speed = if let Some(blend) = sprint_blend {
                    (0.05 * blend + 0.17) * GTA_PHYSICS_HZ
                } else if let Some(blend) = run_blend {
                    (0.07 * blend + 0.1) * GTA_PHYSICS_HZ
                } else {
                    0.1 * GTA_PHYSICS_HZ
                };
                if horizontal_speed.magnitude_sqr() < minimum_speed * minimum_speed {
                    let (sin, cos) = player.locomotion.rotation_cur.sin_cos();
                    horizontal_speed.x = -sin * minimum_speed;
                    horizontal_speed.y = cos * minimum_speed;
                }
                player.ped.flags.is_standing = false;
                player.ped.flags.is_in_the_air = true;
                set_player_jump_animation(
                    player,
                    &frame.anim_manager,
                    crate::animation::AnimationId::ANIM_STD_JUMP_GLIDE,
                );
            }
        }
    } else if grounded && !was_airborne && player.ped.ped_state != PedState::PED_JUMP {
        if let Some(z) = support_ground {
            position.z = z + PED_BASE_OFFSET;
        }
        vertical_speed = 0.0;
        player.ped.flags.is_in_the_air = false;
        player.ped.flags.is_standing = true;
    } else {
        let vertical_ground = player_check_if_in_air_has_ground(position.z, air_ground);
        let foot_sphere_ground = !vertical_ground
            && player_check_if_in_air_has_static_foot_sphere(
                player.ped.ped_state,
                position,
                &frame.file_loader,
            );
        let process_in_air = was_airborne || !(vertical_ground || foot_sphere_ground);
        if process_in_air && player.jump_association.is_none() {
            set_player_jump_animation(
                player,
                &frame.anim_manager,
                crate::animation::AnimationId::ANIM_STD_FALL_GLIDE,
            );
        }
        let ground_within_reach = air_ground.is_some();
        if process_in_air
            && !ground_within_reach
            && vertical_speed < -0.1 * GTA_PHYSICS_HZ
            && player.jump_association.as_ref().is_none_or(|association| {
                association.anim_id != crate::animation::AnimationId::ANIM_STD_FALL
            })
        {
            set_player_jump_animation(
                player,
                &frame.anim_manager,
                crate::animation::AnimationId::ANIM_STD_FALL,
            );
        }
        if process_in_air {
            let _ = update_player_jump_animation(player, dt);
        }
        if let Some(z) = support_ground
            && (position.z <= z + PED_BASE_OFFSET + 0.08 || grounded)
        {
            let impact_speed = vertical_speed;
            position.z = z + PED_BASE_OFFSET;
            vertical_speed = 0.0;
            player.ped.physical.move_speed.z = 0.0;
            player.ped.flags.is_in_the_air = false;
            player.ped.flags.is_standing = true;
            if was_airborne
                && (player.ped.ped_state == PedState::PED_JUMP
                    || player.jump_animation_time > 0.1
                    || impact_speed < -0.5)
            {
                let was_falling = player.jump_association.as_ref().is_some_and(|association| {
                    association.anim_id == crate::animation::AnimationId::ANIM_STD_FALL
                });
                let landing_animation = if was_falling {
                    crate::animation::AnimationId::ANIM_STD_FALL_COLLAPSE
                } else {
                    crate::animation::AnimationId::ANIM_STD_FALL_LAND
                };
                player.ped.flags.is_landing = true;
                set_player_jump_animation(player, &frame.anim_manager, landing_animation);
                if let Some(audio) = frame.audio.as_deref_mut()
                    && player.ped.physical.audio_entity_id >= 0
                {
                    let sound = if was_falling {
                        SOUND_FALL_COLLAPSE
                    } else {
                        SOUND_FALL_LAND
                    };
                    audio.PlayOneShot(player.ped.physical.audio_entity_id, sound as u16, 0.0);
                }
            } else {
                player.ped.ped_state = PedState::PED_IDLE;
            }
        } else {
            player.ped.flags.is_in_the_air = process_in_air;
            player.ped.flags.is_standing = false;
        }
    }
    player.ped.physical.move_speed =
        CVector::new(horizontal_speed.x, horizontal_speed.y, vertical_speed);
    finish_player_collision_frame(&mut player.ped, old_pos, position, collision_rejected_move);
    player.set_position(position);

    process_player_weapon_switch(player, &pad, &frame.weapon_info);
    update_player_weapon_attack_input(player, &pad, frame.timer.time_step_non_clipped());
    let replay_playing_back = frame
        .replay
        .as_ref()
        .is_some_and(|replay| replay.mode == ReplayMode::Playback);
    let fight_owns_attack = process_player_unarmed_fight(
        player,
        other_player.as_mut(),
        &pad,
        &frame.fight_moves,
        &frame.anim_manager,
        &frame.file_loader,
        &mut entities,
        &mut frame.particles.emitter_with_context(
            frame.particle_systems.as_deref(), Some(cameras.player_two.position)),
        &mut frame.general,
        Some(&frame.ped_stats),
        &mut frame.events,
        frame.timer.time_in_milliseconds(),
        frame.audio.as_deref_mut(),
        frame.darkel.as_deref_mut(),
        frame.stats.as_deref_mut(),
        replay_playing_back,
    
    frame.timer.frame_counter(),
);
    if !fight_owns_attack {
        process_player_weapon(
            player,
            other_player.as_mut(),
            &pad,
            camera_heading,
            camera_front,
            false,
            frame.timer.previous_time_in_milliseconds(),
            frame.timer.time_in_milliseconds(),
            frame
                .player_info
                .as_ref()
                .is_some_and(|info| info.m_bFastReload),
            &frame.weapon_info,
            Some(&frame.ped_stats),
            &frame.anim_manager,
            &frame.file_loader,
            &mut entities,
            &mut frame.particles.emitter_with_context(
                frame.particle_systems.as_deref(), Some(cameras.player_two.position)),
            frame.special_fx.as_deref_mut(),
            &mut frame.general,
            frame.darkel.as_deref_mut(),
            frame.stats.as_deref_mut(),
            &mut frame.events,
            replay_playing_back,
            frame.audio.as_deref_mut(),
            frame.projectiles.as_deref_mut(),
            frame.explosions.as_deref_mut(),
            None,
            None,
            None,
            Some(cameras.player_two.position),
        
    frame.timer.frame_counter(),
);
    }
    cameras.player_two.process_follow_ped_with_mouse(
        player.position(),
        player.locomotion.rotation_cur,
        |target, source| {
            frame
                .file_loader
                .world
                .process_line_of_sight_static_hit(
                    [target.x, target.y, target.z],
                    [source.x, source.y, source.z],
                    &frame.file_loader.model_info,
                    None,
                    false,
                )
                .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
        },
    );
}

fn process_local_coop_revive(
    mut session: ResMut<LocalCoopSession>,
    timer: Res<CTimer>,
    mut players: CWorldPlayers,
    anim_manager: Res<CAnimManager>,
) {
    if !session.enabled || !session.revive_active {
        return;
    }
    let dt = timer.time_step_in_seconds().clamp(0.0, 0.05);
    session.revive_timer -= dt;
    if session.revive_timer <= 0.0 {
        session.revive_active = false;
        let target_idx = session.revive_target_player as i32;
        let reviver_idx = if target_idx == 0 { 1 } else { 0 };
        let (mut target_opt, mut reviver_opt) = players.get_pair_mut(target_idx, reviver_idx);
        if let Some(target) = target_opt.as_mut() {
            target.vitality.health = 70.0;
            target.ped.ped_state = PedState::PED_IDLE;
            target.ped.last_ped_state = PedState::PED_NONE;
            target.ped.move_state = eMoveState::PEDMOVE_STILL;
            target.ped.flags.is_ped_die_anim_playing = false;
            target.ped.flags.fallen_down = false;
            target.ped.flags.is_in_the_air = false;
            target.ped.flags.is_standing = true;
            target.ped.flags.is_landing = false;
            target.ped.physical.entity.flags.uses_collision = true;
            target.ped.physical.move_speed = CVector::ZERO;
            target.move_speed = 0.0;
            target.animation.movement_anim_clump.associations.retain(|assoc| {
                !(13..=22).contains(&assoc.anim_id)
                    && assoc.anim_id != crate::animation::AnimationId::ANIM_STD_FALL_COLLAPSE
            });
            if let Some(mut association) = anim_manager.create_anim_association(
                crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
                crate::animation::AnimationId::ANIM_STD_IDLE as usize,
            ) {
                association.set_current_time(0.0);
                association.set_run();
                target.animation.movement_anim_clump.associations.push(association);
            }
        }
        if let Some(reviver) = reviver_opt.as_mut() {
            reviver.ped.ped_state = PedState::PED_IDLE;
            reviver.ped.last_ped_state = PedState::PED_NONE;
            reviver.ped.move_state = eMoveState::PEDMOVE_STILL;
            reviver.ped.physical.move_speed = CVector::ZERO;
            reviver.move_speed = 0.0;
            reviver.animation.movement_anim_clump
                .associations
                .retain(|assoc| assoc.anim_id != crate::animation::AnimationId::ANIM_MEDIC_CPR);
            if let Some(mut association) = anim_manager.create_anim_association(
                crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
                crate::animation::AnimationId::ANIM_STD_IDLE as usize,
            ) {
                association.set_current_time(0.0);
                association.set_run();
                reviver.animation.movement_anim_clump.associations.push(association);
            }
        }
    }
}

fn process_second_player_vehicle(
    player_entity: Entity,
    relation: InVehicle,
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    pad: &crate::core::Pad::CPad,
    control_enabled: bool,
    camera: &mut CCamera,
    entities: &mut ScriptEntityWorld,
    frame: &mut SecondPlayerFrame,
    dt: f32,
) {
    let Some(handle) = entities.vehicle_handle_for_entity(relation.vehicle) else {
        entities.clear_relation_for_ped_entity(player_entity);
        return;
    };
    let exit = control_enabled && pad.exit_vehicle_just_down();
    let mut exit_position = None;
    let mut target = None;
    if entities.vehicles.get(&entities.index, &handle).is_some_and(|vehicle| {
        matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Automobile { .. })
            && crate::peds::PlayerPed::player_can_drive_related_vehicle(
                player.as_ref(), relation, vehicle.base().physical.entity.status)
    }) {
        entities.vehicles.capture_collision_pose(&entities.index, handle);
    }
    if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) = entities.vehicles.get_mut(&entities.index, &handle) {
        let is_driver = matches!(relation.seat, VehicleSeat::Driver);
        let can_drive = crate::peds::PlayerPed::player_can_drive_related_vehicle(player.as_ref(),
            relation,
            car.vehicle.core.physical.entity.status,
        );
        if can_drive {
            if let Some(ModelInfoSlot::Vehicle(model)) = frame
                    .file_loader
                    .model_info
                    .get_model_info(car.vehicle.core.physical.entity.model_index.max(0) as usize)

                && let Some(handling_id) = car.vehicle.core.handling_id
                && let Some(handling) = usize::try_from(handling_id)
                    .ok()
                    .and_then(|id| frame.handling.get_handling_data(id))
            {
                car.setup_suspension_from_model(model, handling);
            }
            let contacts =
                automobile_road_contacts(&car, &frame.file_loader, frame.timer.time_step());
            super::Game::store_automobile_wheel_contacts(&mut car, &contacts, Some(&frame.file_loader));
            let wheel_friction = frame.handling.wheel_friction;
            let adhesive_limits = contacts.surfaces.map(|surface| {
                crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                    &frame.surface_table.data,
                    crate::core::SurfaceTable::eSurfaceType::SURFACE_WHEELBASE as u8,
                    surface,
                ) * crate::core::SurfaceTable::CSurfaceTable::get_wet_multiplier(
                    surface,
                    frame.weather.wet_roads,
                )
            });
            if let Some(handling_id) = car.vehicle.core.handling_id
                && let Some(handling) = usize::try_from(handling_id)
                    .ok()
                    .and_then(|id| frame.handling.get_handling_data_mut(id))
            {
                let controls_disabled = !control_enabled || pad.disable_player_controls;
                car.vehicle.set_status(eEntityStatus::STATUS_PLAYER);
                car.vehicle.drive.engine_on = true;
                let damage_nodes = crate::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(&frame.file_loader, car.vehicle.core.physical.entity.model_index);
                car.process_player_drive_step(
                    CAutomobileDriveStepInput {
                        control: CAutomobileControlInput {
                            steering_left_right: pad.get_steering_left_right() as f32,
                            mouse_x: 0.0,
                            mouse_steering: false,
                            accelerate: pad.get_accelerate() as f32,
                            brake: pad.get_brake() as f32,
                            handbrake: pad.get_hand_brake() != 0,
                            exit_vehicle: exit,
                            controls_disabled,
                        },
                        local_contact_points: contacts.local_points,
                        contact_points: contacts.relative_world_points,
                        spring_ratios: contacts.spring_ratios,
                        spring_directions: contacts.spring_directions,
                        grounded: contacts.grounded,
                        collision_entities: contacts.collision_entities,
                        collision_time_ms: frame.timer.time_in_milliseconds(),
                        adhesive_limits,
                        wheel_scale: contacts.wheel_scale,
                        time_step: frame.timer.time_step(),
                        time_step_seconds: dt,
                    },
                    handling,
                    wheel_friction,
                    &mut frame.general,
                damage_nodes,
                );
            }
        }
        let base = &car.vehicle;
        let model_id = base.physical.entity.model_index.max(0) as usize;
        if exit {
            let component = if is_driver {
                CarDebrisComponent::DoorLeftFront
            } else {
                CarDebrisComponent::DoorRightFront
            };
            exit_position = vehicle_component_world_matrix(
                &frame.file_loader,
                model_id,
                base.physical.entity.matrix,
                component,
            )
            .map(|matrix| matrix.position);
        }
        let dimensions = frame.file_loader.model_info.col_model(model_id).map(|col| {
            CVector::new(
                col.bounding_box.max.x - col.bounding_box.min.x,
                col.bounding_box.max.y - col.bounding_box.min.y,
                col.bounding_box.max.z - col.bounding_box.min.z,
            )
        });
        player.set_position(base.physical.entity.get_position());
        player.set_heading_degrees(base.physical.entity.get_heading_degrees());
        target = dimensions.map(|dimensions| {
            (
                base.physical.entity.get_position(),
                base.physical.entity.get_forward(),
                base.physical.move_speed,
                base.physical.entity.get_heading_degrees(),
                dimensions,
            )
        });
    }
    if exit {
        let _ = entities.detach_ped_from_vehicle(player_entity);
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        player.ped.physical.entity.flags.uses_collision = true;
        player.ped.physical.move_speed = CVector::ZERO;
        if let Some(position) = exit_position {
            player.set_position(position);
        }
        camera.follow_initialised = false;
        camera.restore();
        return;
    }
    if let Some((position, forward, speed, heading, dimensions)) = target {
        player.set_position(position);
        player.set_heading_degrees(heading);
        camera.process_cam_on_a_string(
            position,
            forward,
            speed,
            dimensions,
            frame.timer.time_step(),
            if pad.get_look_behind_for_car() {
                crate::core::Cam::LOOKING_BEHIND
            } else if pad.get_look_left() {
                crate::core::Cam::LOOKING_LEFT
            } else if pad.get_look_right() {
                crate::core::Cam::LOOKING_RIGHT
            } else {
                crate::core::Cam::LOOKING_FORWARD
            },
            |x, y, z| {
                frame.file_loader.world.find_ground_z_for_3d_coord(
                    x,
                    y,
                    z,
                    &frame.file_loader.model_info,
                )
            },
            |x, y, z| {
                frame.file_loader.world.find_roof_z_for_3d_coord(
                    x,
                    y,
                    z,
                    &frame.file_loader.model_info,
                )
            },
            |target, source| {
                frame
                    .file_loader
                    .world
                    .process_line_of_sight_static_hit(
                        [target.x, target.y, target.z],
                        [source.x, source.y, source.z],
                        &frame.file_loader.model_info,
                        None,
                        false,
                    )
                    .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
            },
            |_target, _source| None,
        );
    }
}

fn sync_split_screen_cameras(
    mut commands: Commands,
    session: Res<LocalCoopSession>,
    bridge: Res<ScriptGameplayBridge>,
    cutscene: Option<Res<CCutsceneMgr>>,
    primary_camera: Option<Res<CCamera>>,
    mut cameras: ResMut<LocalCoopCameras>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scene_cameras: Query<(
        Entity,
        &GtaSceneCamera,
        &mut Camera,
        &mut Transform,
        &mut Projection,
    )>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let win_w = window.resolution.physical_width();
    let win_h = window.resolution.physical_height();
    if win_w == 0 || win_h == 0 {
        for (_, marker, mut camera, _, _) in &mut scene_cameras {
            camera.viewport = None;
            if marker.local_player == 1 {
                camera.is_active = false;
            }
        }
        return;
    }
    let primary_target_size = scene_cameras
        .iter()
        .find(|(_, marker, _, _, _)| marker.local_player == 0)
        .and_then(|(_, _, camera, _, _)| camera.physical_target_size())
        .filter(|size| size.x > 0 && size.y > 0);
    let target_size = primary_target_size.unwrap_or_else(|| UVec2::new(win_w, win_h));
    let width = win_w.min(target_size.x).max(2);
    let height = win_h.min(target_size.y).max(2);
    let cutscene_active = bridge.playing_intro
        || !bridge.player_visible
        || !bridge.player_control_enabled
        || bridge.cutscene_started;
    let cutscene_running = cutscene
        .as_ref()
        .is_some_and(|cutscene| cutscene.is_running());
    let split = session.enabled && session.player_two_spawned && !cutscene_active;
    let (player_one_position, player_one_size, player_two_position, player_two_size) =
        match session.split_layout {
            LocalCoopSplitLayout::TopBottom => {
                let top_height = height / 2;
                (
                    UVec2::ZERO,
                    UVec2::new(width, top_height),
                    UVec2::new(0, top_height),
                    UVec2::new(width, height - top_height),
                )
            }
            LocalCoopSplitLayout::LeftRight => {
                let left_width = width / 2;
                (
                    UVec2::ZERO,
                    UVec2::new(left_width, height),
                    UVec2::new(left_width, 0),
                    UVec2::new(width - left_width, height),
                )
            }
        };

    let make_viewport = |pos: UVec2, sz: UVec2| {
        let mut vp = Viewport {
            physical_position: pos,
            physical_size: sz,
            depth: 0.0..1.0,
        };
        crate::renderer::CameraViewport::clamp_viewport_to_target(&mut vp, target_size);
        vp
    };

    let mut has_player_two_camera = false;
    for (_, marker, mut camera, mut transform, mut projection) in &mut scene_cameras {
        match marker.local_player {
            0 => {
                camera.viewport = split.then(|| make_viewport(player_one_position, player_one_size));
                camera.is_active = true;
                camera.order = 0;
                if split {
                    set_projection_aspect(
                        &mut projection,
                        player_one_size.x,
                        player_one_size.y,
                        70.0,
                        false,
                    );
                } else {
                    let horizontal_fov = primary_camera
                        .as_ref()
                        .map(|camera| {
                            if camera.script_mode == CameraScriptMode::FlyBy {
                                camera.flyby_fov
                            } else {
                                camera
                                    .cams
                                    .get(camera.active_cam)
                                    .map_or(70.0, |cam| cam.fov)
                            }
                        })
                        .unwrap_or(70.0);
                    set_projection_aspect(
                        &mut projection,
                        width,
                        height,
                        horizontal_fov,
                        cutscene_running,
                    );
                }
            }
            1 => {
                has_player_two_camera = true;
                camera.is_active = split;
                camera.order = 1;
                camera.viewport = split.then(|| make_viewport(player_two_position, player_two_size));
                if split {
                    cameras.player_two.aspect_ratio =
                        player_two_size.x as f32 / player_two_size.y.max(1) as f32;
                    let eye = cameras.player_two.position.to_bevy();
                    let look = cameras.player_two.look_at.to_bevy();
                    *transform = Transform::from_translation(eye).looking_at(look, Vec3::Y);
                    set_projection_aspect(
                        &mut projection,
                        player_two_size.x,
                        player_two_size.y,
                        70.0,
                        false,
                    );
                }
            }
            _ => {}
        }
    }
    if !has_player_two_camera {
        commands.spawn((
            Camera3d::default(),
            // Match the primary scene camera: the GTA world is unlit and only
            // billboards are Bevy-lit, so a single cluster avoids the per-frame
            // GPU clustering pass without changing lighting.
            bevy::light::cluster::ClusterConfig::Single,
            // Draw directly; the re3 render plan already owns culling.
            NoIndirectDrawing,
            Camera {
                is_active: false,
                order: 1,
                clear_color: ClearColorConfig::Default,
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 42.995_f32.to_radians(),
                near: 0.3,
                far: 3000.0,
                ..default()
            }),
            Tonemapping::None,
            DebandDither::Disabled,
            Msaa::Off,
            Transform::default(),
            GlobalTransform::default(),
            GtaSceneCamera { local_player: 1 },
            Name::new("Local Co-op Player 2 Camera"),
        ));
    }
}

fn set_projection_aspect(
    projection: &mut Projection,
    width: u32,
    height: u32,
    horizontal_fov: f32,
    cutscene_running: bool,
) {
    let Projection::Perspective(perspective) = projection else {
        return;
    };
    perspective.aspect_ratio = width as f32 / height.max(1) as f32;
    perspective.fov =
        CCamera::viewport_vertical_fov(horizontal_fov, perspective.aspect_ratio, cutscene_running);
}

#[cfg(test)]
mod projection_aspect_tests {
    use super::*;

    fn perspective(projection: &Projection) -> &PerspectiveProjection {
        let Projection::Perspective(perspective) = projection else {
            panic!("expected perspective projection");
        };
        perspective
    }

    #[test]
    fn arbitrary_viewports_keep_the_authored_four_three_view_as_minimum_coverage() {
        let mut baseline = Projection::Perspective(PerspectiveProjection::default());
        set_projection_aspect(&mut baseline, 4, 3, 70.0, false);
        let baseline_vertical = perspective(&baseline).fov;

        let mut wide = Projection::Perspective(PerspectiveProjection::default());
        set_projection_aspect(&mut wide, 32, 9, 70.0, false);
        assert!(
            (perspective(&wide).fov - baseline_vertical).abs() < 1.0e-6,
            "wide viewports must add horizontal coverage without zooming"
        );

        let mut tall = Projection::Perspective(PerspectiveProjection::default());
        set_projection_aspect(&mut tall, 1, 4, 70.0, false);
        let tall = perspective(&tall);
        let tall_horizontal = 2.0
            * ((tall.fov * 0.5).tan() * tall.aspect_ratio)
                .atan()
                .to_degrees();
        assert!(
            (tall_horizontal - 70.0).abs() < 1.0e-4,
            "tall viewports must not crop the authored horizontal view"
        );
    }

    #[test]
    fn split_screen_cameras_safely_clamp_to_target_and_handle_window_resizing() {
        let mut app = App::new();
        let window_entity = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1995, 1228),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        assert_eq!(
            app.world().get::<Window>(window_entity).unwrap().resolution.physical_width(),
            1995
        );

        let mut session = LocalCoopSession::default();
        session.enabled = true;
        session.player_two_spawned = true;
        app.insert_resource(session);
        app.insert_resource(ScriptGameplayBridge {
            player_control_enabled: true,
            player_visible: true,
            ..default()
        });
        app.init_resource::<LocalCoopCameras>();

        let p1_cam = app
            .world_mut()
            .spawn((
                Camera {
                    computed: bevy::camera::ComputedCameraValues {
                        target_info: Some(bevy::camera::RenderTargetInfo {
                            physical_size: UVec2::new(1920, 1080),
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    ..default()
                },
                Transform::default(),
                Projection::Perspective(PerspectiveProjection::default()),
                GtaSceneCamera { local_player: 0 },
            ))
            .id();

        let p2_cam = app
            .world_mut()
            .spawn((
                Camera {
                    computed: bevy::camera::ComputedCameraValues {
                        target_info: Some(bevy::camera::RenderTargetInfo {
                            physical_size: UVec2::new(1920, 1080),
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    ..default()
                },
                Transform::default(),
                Projection::Perspective(PerspectiveProjection::default()),
                GtaSceneCamera { local_player: 1 },
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(sync_split_screen_cameras);
        schedule.run(app.world_mut());

        let p1 = app.world().get::<Camera>(p1_cam).unwrap();
        let p2 = app.world().get::<Camera>(p2_cam).unwrap();

        let vp1 = p1.viewport.as_ref().expect("P1 must have viewport in split-screen");
        let vp2 = p2.viewport.as_ref().expect("P2 must have viewport in split-screen");

        // The target is 1920x1080 even though window resolution reports 1995x1228.
        // Viewports must be strictly contained in 1920x1080.
        assert!(vp1.physical_position.x + vp1.physical_size.x <= 1920);
        assert!(vp1.physical_position.y + vp1.physical_size.y <= 1080);
        assert!(vp2.physical_position.x + vp2.physical_size.x <= 1920);
        assert!(vp2.physical_position.y + vp2.physical_size.y <= 1080);

        // Now test window minimization (0x0 physical resolution)
        let mut win = app.world_mut().get_mut::<Window>(window_entity).unwrap();
        win.resolution.set_physical_resolution(0, 0);

        schedule.run(app.world_mut());

        let p1_min = app.world().get::<Camera>(p1_cam).unwrap();
        let p2_min = app.world().get::<Camera>(p2_cam).unwrap();
        assert!(p1_min.viewport.is_none(), "Minimized window must clear viewport");
        assert!(p2_min.viewport.is_none(), "Minimized window must clear viewport");
        assert!(!p2_min.is_active, "P2 camera must be deactivated when minimized");
    }
}
