//! CPickups::Update / CPickup::Update, with live actors owned by Bevy.
use super::{
    Pickups::*,
    ScriptEntities::remove_script_object,
    ScriptEntities::{ScriptEntityIndex, ScriptVehicleIdentity},
    ScriptSnapshots::materialize_pickup_object,
};
use crate::{
    audio::{DMAudio::cDMAudio, soundlist::*},
    control::{Darkel::CDarkel, Garages::CGarages, Script::CTheScripts},
    core::{FileLoader::CFileLoader, PlayerInfo::CPlayerInfo, Pools::CPools},
    objects::Object::CObject,
    peds::{
        Ped::{CPed, give_ped_weapon_with_info},
        PedVitality::PedVitality,
        PlayerPed::{CWorldPlayerState, PlayerPedState},
        VehicleRelations::InVehicle,
    },
    renderer::{WaterLevel::CWaterLevel, Weather::CWeather},
    vehicles::Vehicle::VehicleCore,
    weapons::{
        Explosion::{CExplosionPool, eExplosionType},
        WeaponInfo::CWeaponInfoStore,
        WeaponType::eWeaponType,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use rustgta3_core::core::{
    GameState::GtaAppState, General::CGeneral, Schedules::GtaProcessSet, Timer::CTimer,
};

pub struct GtaPickupPlugin;
impl Plugin for GtaPickupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PickupRuntimeDiagnostics>()
            .init_resource::<PickupCameraState>()
            .add_systems(
                OnEnter(GtaAppState::LoadingGame),
                |mut state: ResMut<PickupCameraState>| {
                    state.active = None;
                },
            )
            .add_systems(
                Update,
                update_pickups
                    .in_set(GtaProcessSet::Pickups)
                    .run_if(in_state(GtaAppState::InGame)),
            );
    }
}
#[derive(Default, Resource)]
struct PickupCameraState {
    active: Option<PickupCamera>,
}
struct PickupCamera {
    vehicle: Entity,
    position: rustgta3_core::core::Coordinates::CVector,
    started: u32,
}
#[derive(Resource, Default, Debug)]
pub struct PickupRuntimeDiagnostics {
    pub collected: u64,
    pub respawned: u64,
    pub expired: u64,
    pub waiting_player: u64,
    pub waiting_model: u64,
    pub waiting_object: u64,
    pub waiting_pool: u64,
    pub waiting_weapon_info: u64,
    pub waiting_water: u64,
    pub waiting_collision: u64,
    pub waiting_camera: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickupModel {
    Adrenaline,
    Armour,
    Info,
    Health,
    Bonus,
    Bribe,
    Frenzy,
    Camera,
    Weapon(eWeaponType),
    Other,
}
pub(crate) fn pickup_model(name: &str) -> PickupModel {
    use PickupModel::*;
    // MatchModelString resolves these identifiers from the installed IDE.
    match name {
        "adrenaline" => Adrenaline,
        "bodyarmour" => Armour,
        "info" => Info,
        "health" => Health,
        "bonus" => Bonus,
        "bribe" => Bribe,
        "killfrenzy" => Frenzy,
        "camerapickup" => Camera,
        _ => WEAPON_PICKUP_NAMES.iter().find(|(key, _)| *key == name)
            .map_or(Other, |(_, weapon)| Weapon(*weapon)),
    }
}

/// CPickups::ModelForWeapon / WeaponForModel at the MatchModelString boundary.
const WEAPON_PICKUP_NAMES: [(&str, eWeaponType); 11] = {
    use eWeaponType::*;
    [
        ("bat", WEAPONTYPE_BASEBALLBAT), ("colt45", WEAPONTYPE_COLT45),
        ("uzi", WEAPONTYPE_UZI), ("shotgun", WEAPONTYPE_SHOTGUN),
        ("ak47", WEAPONTYPE_AK47), ("m16", WEAPONTYPE_M16),
        ("sniper", WEAPONTYPE_SNIPERRIFLE), ("rocket", WEAPONTYPE_ROCKETLAUNCHER),
        ("flame", WEAPONTYPE_FLAMETHROWER), ("molotov", WEAPONTYPE_MOLOTOV),
        ("grenade", WEAPONTYPE_GRENADE),
    ]
};

pub(crate) fn weapon_pickup_model_name(weapon: eWeaponType) -> Option<&'static str> {
    WEAPON_PICKUP_NAMES.iter().find(|(_, value)| *value == weapon).map(|(name, _)| *name)
}
const AMMO: [u16; 20] = [
    0, 1, 45, 125, 25, 150, 300, 25, 5, 250, 5, 5, 0, 500, 0, 100, 0, 0, 0, 0,
];
const WEAPON_COST: [i32; 20] = [
    0, 10, 250, 800, 1500, 3000, 5000, 10000, 25000, 25000, 2000, 2000, 0, 50000, 0, 3000, 0, 0, 0,
    0,
];

/// Native six-frame general-pickup cadence; scripted tail slots update every frame.
pub(crate) fn pickup_slots(frame: u32) -> impl Iterator<Item = usize> {
    let slice = frame as usize % 6;
    (NUMGENERALPICKUPS * slice / 6..NUMGENERALPICKUPS * (slice + 1) / 6)
        .chain(NUMGENERALPICKUPS..NUMPICKUPS)
}

#[derive(SystemParam)]
struct PickupServices<'w> {
    audio: Option<ResMut<'w, cDMAudio>>,
    weapons: Option<Res<'w, CWeaponInfoStore>>,
    scripts: Option<Res<'w, CTheScripts>>,
    darkel: Option<Res<'w, CDarkel>>,
    garages: Option<ResMut<'w, CGarages>>,
    game: Res<'w, rustgta3_core::core::GameState::CGameState>,
    water: Option<Res<'w, CWaterLevel>>,
    weather: Option<Res<'w, CWeather>>,
    explosions: ResMut<'w, CExplosionPool>,
    pickup_camera: ResMut<'w, PickupCameraState>,
    camera: Option<ResMut<'w, crate::core::Camera::CCamera>>,
    pads: ResMut<'w, crate::core::Pad::CPads>,
    menu: Res<'w, crate::core::Frontend::CMenuManager>,
    cutscene: Res<'w, crate::animation::CutsceneMgr::CCutsceneMgr>,
    replay: Res<'w, super::Replay::CReplay>,
}

#[allow(clippy::too_many_arguments)]
fn update_pickups(
    mut pickups: Option<ResMut<CPickups>>,
    timer: Res<CTimer>,
    loader: Res<CFileLoader>,
    player_index: Res<CWorldPlayerState>,
    mut players: Query<(
        &mut CPed,
        &mut PedVitality,
        &mut PlayerPedState,
        Option<&InVehicle>,
    )>,
    mut info: ResMut<CPlayerInfo>,
    mut index: ResMut<ScriptEntityIndex>,
    mut objects: Query<&mut CObject>,
    vehicles: Query<(&ScriptVehicleIdentity, &VehicleCore)>,
    mut pools: ResMut<CPools>,
    mut general: ResMut<CGeneral>,
    mut commands: Commands,
    services: PickupServices,
    mut diagnostics: ResMut<PickupRuntimeDiagnostics>,
) {
    let Some(pickups) = pickups.as_deref_mut() else {
        return;
    };
    if timer.get_is_paused() {
        return;
    }
    let Some(player_entity) = player_index.entities[player_index.player_in_focus] else {
        diagnostics.waiting_player += 1;
        return;
    };
    let Ok((mut ped, mut vitality, mut player_state, relation)) = players.get_mut(player_entity)
    else {
        diagnostics.waiting_player += 1;
        return;
    };
    let relation = relation.copied();
    let position = ped.physical.entity.get_position();
    let now = timer.time_in_milliseconds();
    let PickupServices {
        mut audio,
        weapons,
        scripts,
        darkel,
        mut garages,
        game,
        water,
        weather,
        mut explosions,
        mut pickup_camera,
        mut camera,
        mut pads,
        menu,
        cutscene,
        replay,
    } = services;
    if replay.mode == super::Replay::ReplayMode::Playback {
        return;
    }
    if let Some(active) = pickup_camera.active.as_ref() {
        let distance = position - active.position;
        let expired = now.wrapping_sub(active.started) > 60000
            || distance.magnitude() > 40.0
            || relation.is_none_or(|relation| relation.vehicle != active.vehicle)
            || vehicles.get(active.vehicle).is_err();
        if let Some(camera) = camera.as_deref_mut() {
            if expired {
                camera.restore_with_jump_cut();
                pickup_camera.active = None;
            } else if let Ok((identity, vehicle)) = vehicles.get(active.vehicle) {
                if index.vehicle(identity.handle) != Some(active.vehicle) {
                    pickup_camera.active = None;
                    camera.restore_with_jump_cut();
                } else if let Some(col) = loader
                    .model_info
                    .col_model(vehicle.physical.entity.model_index.max(0) as usize)
                {
                    let distance_xy =
                        (position.x - active.position.x).hypot(position.y - active.position.y);
                    let mult = if distance_xy < 10.0 {
                        1.0 - distance_xy / 10.0
                    } else {
                        0.0
                    };
                    let mut eye = active.position;
                    eye.z += (col.bounding_box.max.z - col.bounding_box.min.z + 2.0) * mult;
                    if now.wrapping_sub(active.started) > 750 {
                        camera.SetCamPositionForFixedMode(
                            eye,
                            rustgta3_core::core::Coordinates::CVector::ZERO,
                        );
                        camera.take_control_script_entity(
                            crate::core::Camera::CameraScriptTarget::Vehicle(
                                crate::control::Script::ScriptHandle(identity.handle),
                            ),
                            vehicle.physical.entity.get_position(),
                            crate::core::Cam::eCamMode::MODE_FIXED as i32,
                            2,
                        );
                    }
                } else {
                    diagnostics.waiting_collision += 1;
                }
            }
        } else {
            diagnostics.waiting_camera += 1;
        }
    }
    let frenzy_unavailable = scripts.as_ref().is_some_and(|s| s.is_player_on_a_mission())
        || darkel.as_ref().is_some_and(|d| d.frenzy_on_going())
        || !game.nasty_game;
    for slot in pickup_slots(timer.frame_counter()) {
        let pickup = pickups.pickups[slot];
        if pickup.pickup_type == PICKUP_NONE {
            continue;
        }
        let handle = slot as i32 | (i32::from(pickup.index) << 16);
        let dx = position.x - pickup.pos.x;
        let dy = position.y - pickup.pos.y;
        let distance_sq = dx * dx + dy * dy;
        if pickup.removed {
            if now > pickup.timer
                && (distance_sq > 100.0
                    || pickup.pickup_type == PICKUP_IN_SHOP && distance_sq > 2.4)
            {
                if materialize_pickup_object(
                    handle,
                    pickups,
                    &mut commands,
                    &mut index,
                    &mut pools,
                    &mut general,
                    &loader,
                )
                .is_some()
                {
                    pickups.pickups[slot].removed = false;
                    diagnostics.respawned += 1;
                } else {
                    diagnostics.waiting_pool += 1;
                }
            }
            continue;
        }
        // Failed pool allocations and restored saves are retried through the same constructor.
        if pickup.object_pool_index.is_none() {
            if materialize_pickup_object(
                handle,
                pickups,
                &mut commands,
                &mut index,
                &mut pools,
                &mut general,
                &loader,
            )
            .is_none()
            {
                diagnostics.waiting_pool += 1;
            }
            continue;
        }
        let Some(model) = loader
            .model_info
            .get_model_info(pickup.model_index as usize)
        else {
            diagnostics.waiting_model += 1;
            continue;
        };
        let kind = pickup_model(&model.base().name.to_ascii_lowercase());
        let object_handle = pickup.object_pool_index.and_then(|slot| {
            pools
                .object_pool
                .get_slot(slot as usize)
                .map(|_| pools.object_pool.get_index_from_slot(slot as usize).0)
                .filter(|handle| index.object(*handle).is_some())
        });
        let Some(object_entity) = object_handle.and_then(|h| index.objects.get(&h).copied()) else {
            diagnostics.waiting_object += 1;
            continue;
        };
        let Ok(mut object) = objects.get_mut(object_entity) else {
            diagnostics.waiting_object += 1;
            continue;
        };
        let mut pickup_position = object.physical.entity.get_position();
        let mut special_removed = false;
        let mut special_collected = false;
        if (PICKUP_MINE_INACTIVE..=PICKUP_FLOATINGPACKAGE_FLOATING).contains(&pickup.pickup_type) {
            let water_z = if matches!(
                pickup.pickup_type,
                PICKUP_NAUTICAL_MINE_INACTIVE
                    | PICKUP_NAUTICAL_MINE_ARMED
                    | PICKUP_FLOATINGPACKAGE
                    | PICKUP_FLOATINGPACKAGE_FLOATING
            ) {
                if let (Some(water), Some(weather)) = (water.as_deref(), weather.as_deref()) {
                    water
                        .get_water_level(
                            pickup_position.x,
                            pickup_position.y,
                            pickup_position.z + 5.0,
                            false,
                            now,
                            weather.wind,
                        )
                        .map(|sample| sample.level)
                } else {
                    diagnostics.waiting_water += 1;
                    None
                }
            } else {
                None
            };
            if matches!(
                pickup.pickup_type,
                PICKUP_NAUTICAL_MINE_INACTIVE | PICKUP_NAUTICAL_MINE_ARMED
            ) {
                if let Some(z) = water_z {
                    pickup_position.z = z + 0.6;
                }
            }
            let vehicle_touches = |vehicle: &VehicleCore, radius: f32| {
                loader
                    .model_info
                    .col_model(vehicle.physical.entity.model_index.max(0) as usize)
                    .is_some_and(|col| {
                        vehicle.is_sphere_touching_vehicle(
                            pickup_position,
                            radius,
                            &col.bounding_box,
                        )
                    })
            };
            match pickup.pickup_type {
                PICKUP_MINE_INACTIVE => {
                    if relation
                        .and_then(|r| vehicles.get(r.vehicle).ok())
                        .is_some_and(|(_, v)| !vehicle_touches(v, 2.0))
                    {
                        pickups.pickups[slot].pickup_type = PICKUP_MINE_ARMED;
                        pickups.pickups[slot].timer = now.wrapping_add(10000);
                    }
                }
                PICKUP_NAUTICAL_MINE_INACTIVE => {
                    if !vehicles.iter().any(|(identity, v)| {
                        index.vehicles.contains_key(&identity.handle) && vehicle_touches(v, 1.5)
                    }) {
                        pickups.pickups[slot].pickup_type = PICKUP_NAUTICAL_MINE_ARMED;
                        pickups.pickups[slot].timer = now.wrapping_add(10000);
                    }
                }
                PICKUP_MINE_ARMED | PICKUP_NAUTICAL_MINE_ARMED => {
                    if now > pickup.timer
                        || vehicles.iter().any(|(identity, v)| {
                            index.vehicles.contains_key(&identity.handle) && vehicle_touches(v, 1.5)
                        })
                    {
                        if explosions.AddExplosion(
                            None,
                            None,
                            eExplosionType::EXPLOSION_MINE,
                            pickup_position,
                            0,
                            now,
                        ) {
                            special_removed = true;
                        } else {
                            warn_once!(target: "rustgta3::pickups", "CExplosion mine allocation failed");
                        }
                    }
                }
                PICKUP_FLOATINGPACKAGE => {
                    // CPhysical velocity is metres/second here; convert the native 50 Hz units.
                    object.physical.move_speed.z -= 0.01 * timer.time_step() * 50.0;
                    pickup_position =
                        pickup_position + object.physical.move_speed * timer.time_step_in_seconds();
                    if water_z.is_some_and(|z| z >= pickup_position.z) {
                        pickups.pickups[slot].pickup_type = PICKUP_FLOATINGPACKAGE_FLOATING;
                    }
                }
                PICKUP_FLOATINGPACKAGE_FLOATING => {
                    if let Some(z) = water_z {
                        pickup_position.z = z;
                    }
                    special_collected = relation
                        .and_then(|r| vehicles.get(r.vehicle).ok())
                        .is_some_and(|(_, v)| {
                            loader
                                .model_info
                                .col_model(v.physical.entity.model_index.max(0) as usize)
                                .is_some_and(|col| {
                                    v.is_sphere_touching_vehicle(
                                        pickup_position,
                                        2.0,
                                        &col.bounding_box,
                                    )
                                })
                        });
                    special_removed = special_collected;
                    if special_collected && let Some(audio) = audio.as_deref_mut() {
                        audio.PlayFrontEndSound(SOUND_PICKUP_FLOAT_PACKAGE as u16, 0);
                    }
                }
                _ => {}
            }
            object.physical.entity.set_position(pickup_position);
            object.object_matrix = object.physical.entity.matrix;
        }
        drop(object);
        let touches_vehicle = relation
            .and_then(|r| vehicles.get(r.vehicle).ok())
            .is_some_and(|(identity, vehicle)| {
                if index.vehicles.get(&identity.handle) != relation.as_ref().map(|r| &r.vehicle) {
                    return false;
                }
                let Some(col) = loader
                    .model_info
                    .col_model(vehicle.physical.entity.model_index.max(0) as usize)
                else {
                    return false;
                };
                vehicle.is_sphere_touching_vehicle(pickup_position, 2.0, &col.bounding_box)
            });
        let touched = if kind == PickupModel::Bribe {
            if relation.is_some() {
                touches_vehicle
            } else {
                distance_sq < 1.8 && (position.z - pickup_position.z).abs() < 2.0
            }
        } else if kind == PickupModel::Camera {
            touches_vehicle
        } else {
            relation.is_none() && distance_sq < 1.8 && (position.z - pickup_position.z).abs() < 2.0
        };
        let available = match kind {
            PickupModel::Armour => vitality.armour <= 99.5,
            PickupModel::Health => vitality.health <= 99.5,
            PickupModel::Bribe => player_state.wanted.get_wanted_level() > 0,
            PickupModel::Frenzy => !frenzy_unavailable,
            _ => true,
        };
        let mut collected = special_collected;
        let mut respawn = None;
        if touched && available {
            pads.get_pad_mut(0).start_shake(
                120,
                100,
                menu.prefs_use_vibration != 0,
                cutscene.is_cutscene_processing(),
                game.playing_intro,
            );
            match pickup.pickup_type {
                PICKUP_IN_SHOP
                | PICKUP_ON_STREET
                | PICKUP_ON_STREET_SLOW
                | PICKUP_ONCE
                | PICKUP_ONCE_TIMEOUT => {
                    let weapon = match kind {
                        PickupModel::Weapon(w) => w as usize,
                        PickupModel::Armour => eWeaponType::WEAPONTYPE_ARMOUR as usize,
                        _ => 0,
                    };
                    if pickup.pickup_type == PICKUP_IN_SHOP && info.m_nMoney < WEAPON_COST[weapon] {
                        if let Some(garages) = garages.as_deref_mut() {
                            garages.trigger_message(b"PU_MONY", -1, 6000, -1, now);
                        }
                        continue;
                    }
                    let sound = match kind {
                        PickupModel::Adrenaline => {
                            player_state.adrenaline_active = true;
                            player_state.adrenaline_time = now.wrapping_add(20_000);
                            player_state.current_stamina = player_state.max_stamina;
                            SOUND_PICKUP_ADRENALINE
                        }
                        PickupModel::Armour => {
                            vitality.armour = 100.0;
                            SOUND_PICKUP_ARMOUR
                        }
                        PickupModel::Health => {
                            vitality.health = 100.0;
                            SOUND_PICKUP_HEALTH
                        }
                        PickupModel::Bribe => {
                            let level = player_state.wanted.get_wanted_level();
                            player_state
                                .wanted
                                .set_wanted_level_at((level - 1).max(0), now);
                            SOUND_PICKUP_BONUS
                        }
                        PickupModel::Camera => {
                            if matches!(
                                pickup.pickup_type,
                                PICKUP_ON_STREET | PICKUP_ON_STREET_SLOW
                            ) && let Some(relation) = relation
                            {
                                pickup_camera.active = Some(PickupCamera {
                                    vehicle: relation.vehicle,
                                    position: pickup_position,
                                    started: now,
                                });
                            }
                            SOUND_PICKUP_BONUS
                        }
                        PickupModel::Weapon(w) => {
                            let Some(weapons) = weapons.as_deref() else {
                                diagnostics.waiting_weapon_info += 1;
                                continue;
                            };
                            let on_street = matches!(
                                pickup.pickup_type,
                                PICKUP_ON_STREET | PICKUP_ON_STREET_SLOW
                            );
                            let ammo = if pickup.pickup_type == PICKUP_IN_SHOP {
                                AMMO[w as usize]
                            } else if pickup.quantity != 0 {
                                pickup.quantity
                            } else if on_street {
                                STREET_AMMO[w as usize]
                            } else {
                                AMMO[w as usize]
                            };
                            give_ped_weapon_with_info(&mut ped, w as i32, i32::from(ammo), weapons);
                            if pickup.pickup_type == PICKUP_IN_SHOP
                                || player_state.selected_wep_slot == eWeaponType::WEAPONTYPE_UNARMED
                            {
                                player_state.selected_wep_slot = w;
                            }
                            if pickup.pickup_type == PICKUP_IN_SHOP {
                                SOUND_PICKUP_WEAPON_BOUGHT
                            } else {
                                SOUND_PICKUP_WEAPON
                            }
                        }
                        _ => SOUND_PICKUP_BONUS,
                    };
                    if pickup.pickup_type == PICKUP_IN_SHOP {
                        info.m_nMoney -= WEAPON_COST[weapon];
                    }
                    if let Some(audio) = audio.as_deref_mut() {
                        let parameter = if matches!(kind, PickupModel::Weapon(_)) {
                            loader
                                .model_info
                                .get_model_info_by_name("grenade")
                                .map_or(0, |(grenade, _)| {
                                    pickup.model_index as u32 - grenade as u32
                                })
                        } else {
                            0
                        };
                        audio.PlayFrontEndSound(sound as u16, parameter);
                    }
                    respawn = match pickup.pickup_type {
                        PICKUP_IN_SHOP => Some(5000),
                        PICKUP_ON_STREET => Some(30000),
                        PICKUP_ON_STREET_SLOW => Some(if kind == PickupModel::Bribe {
                            300000
                        } else {
                            720000
                        }),
                        _ => None,
                    };
                    collected = true;
                }
                PICKUP_COLLECTABLE1 => {
                    info.m_nCollectedPackages += 1;
                    info.m_nMoney += 1000;
                    let all = info.m_nCollectedPackages == info.m_nTotalPackages;
                    if all {
                        info.m_nMoney += 1_000_000;
                    }
                    if let Some(garages) = garages.as_deref_mut() {
                        garages.trigger_message(
                            if all { b"CO_ALL" } else { b"CO_ONE" },
                            if all { -1 } else { info.m_nCollectedPackages },
                            5000,
                            if all { -1 } else { info.m_nTotalPackages },
                            now,
                        );
                    }
                    if let Some(audio) = audio.as_deref_mut() {
                        audio.PlayFrontEndSound(SOUND_PICKUP_HIDDEN_PACKAGE as u16, 0);
                    }
                    collected = true;
                }
                PICKUP_MONEY => {
                    info.m_nMoney += i32::from(pickup.quantity);
                    if let Some(audio) = audio.as_deref_mut() {
                        audio.PlayFrontEndSound(SOUND_PICKUP_MONEY as u16, 0);
                    }
                    collected = true;
                }
                _ => {}
            }
        }
        let expired =
            matches!(pickup.pickup_type, PICKUP_ONCE_TIMEOUT | PICKUP_MONEY) && now > pickup.timer;
        if collected || expired || special_removed {
            if let Some(object_handle) = object_handle {
                remove_script_object(&mut commands, &mut index, Some(&mut pools), object_handle);
            }
            let state = &mut pickups.pickups[slot];
            state.object_present = false;
            state.object_pool_index = None;
            state.removed = true;
            if let Some(delay) = respawn {
                state.timer = now.wrapping_add(delay);
            } else {
                state.pickup_type = PICKUP_NONE;
            }
            if collected {
                let next = pickups.collected_pickup_index as usize;
                pickups.pickups_collected[next] = handle;
                pickups.collected_pickup_index = ((next + 1) % NUMCOLLECTEDPICKUPS) as i16;
                diagnostics.collected += 1;
            } else {
                diagnostics.expired += 1;
            }
        }
    }
}
