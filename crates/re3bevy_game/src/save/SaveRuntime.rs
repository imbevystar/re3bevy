//! Live Bevy-world adapter for re3 PC save requests.
//!
//! re3 reference: references/re-GTA/src/save/GenericGameStorage.cpp
//! re3 reference: references/re-GTA/src/save/PCSave.cpp

use std::path::Path;

use bevy::{ecs::system::SystemParam, prelude::*};
use re3bevy_core::core::{
    Clock::CClock, GameState::CGameState, General::CGeneral, TimeStep::CTimeStep, Timer::CTimer,
};

use crate::{
    audio::AudioScriptObject::cAudioScriptObjectPool,
    control::{
        GameLogic::CGameLogic, Garages::CGarages, Phones::CPhoneInfo, Pickups::CPickups,
        Restart::CRestart, Script::CTheScripts,
        ScriptEntities::ScriptObjectIdentity,
    },
    core::{
        Camera::CCamera, FileLoader::CFileLoader, Pad::CPads, PlayerInfo::CPlayerInfo,
        Pools::{CPools, CompatiblePedBaseSave, CompatiblePlayerPedSave, CompatibleWeaponSave,
            MAX_MODEL_NAME, encode_compatible_player_ped}, Stats::CStats, Streaming::CStreaming,
    },
    objects::ParticleObject::CParticleObjectRuntime,
    peds::{Gangs::CGangs, PedType::{CPedTypeRuntime, ePedType},
        PlayerPed::{CPlayerPed, CWorldPlayers, WorldPlayerAccess}},
    peds::VehicleRelations::InVehicle,
    renderer::Weather::CWeather,
    text::Text::{CText, unicode_to_ascii},
    vehicles::{CarGen::CTheCarGenerators, Cranes::CCranes},
    weapons::{Weapon::CWeapon, WeaponType::{eWeaponState, eWeaponType, script_weapon_type}},
};

use super::{
    GenericGameStorage::{CGenericGameStorage, SLOT_COUNT, SaveBlock, SaveBlockKind, SaveSimpleVars, SaveSystemTime},
    SaveBuf::SaveBufWriter,
    MemoryCard::CMemoryCard,
    PCSave::C_PcSave,
    SaveGameBlocks::{
        save_audio_script_objects_block, save_car_generators_block, save_cranes_block,
        save_gang_data_block, save_garages_block, save_particles_block,
        save_paths_block, save_ped_type_block, save_phone_info_block,
        save_pickups_block, save_player_info_block, save_radar_blips_block, save_restart_block,
        save_stats_block, save_streaming_block, save_world_zones_block,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PcSaveRequestState {
    #[default]
    Idle,
    Requested(usize),
    Succeeded,
    Failed,
}

/// re3 CPools::LoadPedPool creates a real CPlayerPed, not a saved mesh.
/// Stage all records first so invalid weapon data cannot replace live players.
pub(crate) fn restore_live_player_pool(
    pools: &mut CPools,
    players: &mut CWorldPlayers,
    general: &mut CGeneral,
    ped_types: Option<&CPedTypeRuntime>,
) -> Result<(), &'static str> {
    let mut restored = None;
    for (_, record) in pools.compatible_ped_records() {
        if record.ped_type != ePedType::PEDTYPE_PLAYER1 as u32 || restored.is_some() {
            return Err("saved ped pool must contain at most one PLAYER1");
        }
        let mut player = CPlayerPed::new(general, ped_types);
        player.ped.physical.entity.set_model_index_no_create(record.model_index as usize);
        player.ped.physical.entity.set_position(record.base.position);
        player.ped.char_created_by = record.base.char_created_by;
        player.vitality.health = record.base.health;
        player.vitality.armour = record.base.armour;
        player.ped.max_weapon_type_allowed = record.base.max_weapon_type_allowed;
        for (weapon, saved) in player.ped.weapons.iter_mut().zip(record.base.weapons.iter()) {
            let weapon_type = script_weapon_type(saved.weapon_type)
                .filter(|kind| (*kind as usize) < crate::weapons::WeaponType::WEAPONTYPE_TOTAL_INVENTORY_WEAPONS)
                .ok_or("saved inventory weapon type is invalid")?;
            let weapon_state = match saved.weapon_state {
                0 => eWeaponState::WEAPONSTATE_READY,
                1 => eWeaponState::WEAPONSTATE_FIRING,
                2 => eWeaponState::WEAPONSTATE_RELOADING,
                3 => eWeaponState::WEAPONSTATE_OUT_OF_AMMO,
                4 => eWeaponState::WEAPONSTATE_MELEE_MADECONTACT,
                _ => return Err("saved inventory weapon state is invalid"),
            };
            *weapon = CWeapon {
                weapon_type, weapon_state,
                ammo_in_clip: saved.ammo_in_clip,
                ammo_total: saved.ammo_total,
                timer: saved.timer,
                add_rot_offset: saved.add_rot_offset,
            };
        }
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;
        player.selected_wep_slot = eWeaponType::WEAPONTYPE_UNARMED;
        player.ped.wep_accuracy = 100;
        player.max_stamina = record.player.max_stamina;
        // CPlayerPed::Load restores the maximum only; current stamina retains
        // the value established by the CPlayerPed constructor.
        player.targettable_objects = record.player.targettable_objects;
        player.pool_handle = record.handle.0;
        player.active_in_world = true;
        player.wanted.maximum_wanted_level = record.maximum_wanted_level;
        player.wanted.maximum_chaos = record.n_maximum_wanted_level;
        restored = Some(player);
    }
    let Some(player) = restored else {
        return Err("saved ped pool has no PLAYER1 outside a vehicle");
    };
    players.set_maximum_wanted_level(player.wanted.maximum_wanted_level);
    players.replace_player(0, player);
    players.set_player_in_focus(0);
    // The renderer's compatible-pool path must not spawn a second inert copy
    // of the ped now owned by CWorldPlayers. Keep the pool handle allocated.
    for slot in 0..pools.ped_pool.get_size() {
        if let Some(entry) = pools.ped_pool.get_slot_mut(slot) {
            entry.compatible_save = None;
        }
    }
    Ok(())
}

#[derive(Debug, Resource, Default)]
pub struct PcSaveRequest {
    pub state: PcSaveRequestState,
}

#[derive(SystemParam)]
pub struct PcSaveParams<'w, 's> {
    request: ResMut<'w, PcSaveRequest>,
    storage: ResMut<'w, CGenericGameStorage>,
    pc_save: ResMut<'w, C_PcSave>,
    memory_card: ResMut<'w, CMemoryCard>,
    game_state: Res<'w, CGameState>,
    clock: ResMut<'w, CClock>,
    timer: Res<'w, CTimer>,
    stats: ResMut<'w, CStats>,
    pickups: ResMut<'w, CPickups>,
    scripts: Res<'w, CTheScripts>,
    player_info: Res<'w, CPlayerInfo>,
    file_loader: Res<'w, CFileLoader>,
    streaming: Res<'w, CStreaming>,
    restart: Res<'w, CRestart>,
    gangs: Res<'w, CGangs>,
    car_generators: Res<'w, CTheCarGenerators>,
    phone_info: Res<'w, CPhoneInfo>,
    ped_type: Res<'w, CPedTypeRuntime>,
    audio_script_objects: Res<'w, cAudioScriptObjectPool>,
    garages: Res<'w, CGarages>,
    particles: Res<'w, CParticleObjectRuntime>,
    cranes: Res<'w, CCranes>,
    camera: Res<'w, CCamera>,
    pads: Res<'w, CPads>,
    weather: Res<'w, CWeather>,
    text: Option<Res<'w, CText>>,
    world_players: CWorldPlayers<'w, 's>,
    vehicles: crate::control::ScriptEntities::VehicleReadStore<'w, 's>,
    objects: Query<'w, 's, (Entity, &'static ScriptObjectIdentity, &'static crate::objects::Object::CObject)>,
    occupants: Query<'w, 's, &'static InVehicle>,
}

pub fn process_pending_pc_save(mut params: PcSaveParams) {
    let PcSaveRequestState::Requested(slot) = params.request.state else {
        return;
    };
    params.request.state = PcSaveRequestState::Idle;

    if slot >= SLOT_COUNT {
        error!(target: "re3bevy::save", "PC save rejected invalid slot {slot}");
        params.request.state = PcSaveRequestState::Failed;
        return;
    }

    // re3 CGenericGameStorage::DoGameSpecificStuffBeforeSave.
    CGameLogic::pass_time(
        360,
        params.timer.time_in_milliseconds(),
        &mut params.clock,
        &mut params.stats,
        &mut params.pickups,
    );
    let Some(mut player) = params.world_players.focused_mut() else {
        error!(target: "re3bevy::save", "PC save aborted: focused player is unavailable");
        params.request.state = PcSaveRequestState::Failed;
        return;
    };
    player.current_stamina = player.max_stamina;

    // CGame::TidyUpMemory(true, false) is an RW heap/streaming-memory cleanup.
    // Bevy owns those allocations, so there is no equivalent world mutation to serialize here.

    let live_object_handles = params
        .objects
        .iter()
        .map(|(_, identity, _)| identity.handle)
        .collect::<std::collections::HashSet<_>>();
    let scripts = match params.scripts.save_all_scripts_with_live_objects(
        &params.file_loader.world,
        &live_object_handles,
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            error!(target: "re3bevy::save", "CTheScripts::SaveAllScripts failed: {error}");
            params.request.state = PcSaveRequestState::Failed;
            return;
        }
    };

    let timestep = CTimeStep::default();
    let simple = SaveSimpleVars {
        save_name: params
            .text
            .as_deref()
            .map(|text| {
                unicode_to_ascii(&text.get_key_bytes(&params.stats.last_mission_passed_name))
            })
            .unwrap_or_else(|| {
                let end = params
                    .stats
                    .last_mission_passed_name
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(params.stats.last_mission_passed_name.len());
                String::from_utf8_lossy(&params.stats.last_mission_passed_name[..end]).into_owned()
            }),
        save_time: local_system_time(),
        curr_level: params.game_state.curr_level,
        camera_position: params.camera.position,
        milliseconds_per_game_minute: params.clock.milliseconds_per_game_minute() as i32,
        last_clock_tick: params.clock.last_clock_tick() as i32,
        game_clock_hours: params.clock.hours(),
        game_clock_minutes: params.clock.minutes(),
        pad_mode: params.pads.get_pad(0).mode,
        timer_ms: params.timer.time_in_milliseconds(),
        time_scale: params.timer.time_scale(),
        time_step: params.timer.time_step(),
        time_step_non_clipped: params.timer.time_step_non_clipped(),
        frame_counter: params.timer.frame_counter(),
        timestep_time_step: timestep.time_step,
        frames_per_update: timestep.frames_per_update,
        timestep_time_scale: timestep.time_scale,
        old_weather_type: params.weather.old_weather_type,
        new_weather_type: params.weather.new_weather_type,
        forced_weather_type: params.weather.forced_weather_type,
        weather_interpolation: params.weather.interpolation_value,
        compile_date: params.storage.CompileDateAndTime,
        weather_type_in_list: params.weather.weather_type_in_list as i16,
        car_zoom_indicator: params.camera.car_zoom_indicator as f32,
        ped_zoom_indicator: params.camera.ped_zoom_indicator as f32,
        ..SaveSimpleVars::default()
    };

    let ped_block = match save_live_player_pool(&params.world_players, &params.file_loader) {
        Ok(block) => block,
        Err(reason) => {
            error!(target: "re3bevy::save", "CPools::SavePedPool failed: {reason}");
            params.request.state = PcSaveRequestState::Failed;
            return;
        }
    };
    let blocks = vec![
        ped_block,
        save_garages_block(&params.garages),
        super::SavePoolRuntime::save_live_vehicle_pool(&params.vehicles, &params.occupants),
        super::SavePoolRuntime::save_live_object_pool(&params.objects),
        save_paths_block(&params.file_loader.path_find),
        save_cranes_block(&params.cranes),
        save_pickups_block(&params.pickups),
        save_phone_info_block(&params.phone_info),
        save_restart_block(&params.restart),
        save_radar_blips_block(&params.file_loader.radar),
        save_world_zones_block(&params.file_loader.world.zones),
        save_gang_data_block(&params.gangs),
        save_car_generators_block(&params.car_generators),
        save_particles_block(&params.particles),
        save_audio_script_objects_block(&params.audio_script_objects),
        save_player_info_block(&params.player_info),
        save_stats_block(&params.stats),
        save_streaming_block(&params.streaming),
        save_ped_type_block(&params.ped_type),
    ];

    match params
        .pc_save
        .SaveSlotGeneric(&mut params.storage, slot, &simple, &scripts, blocks)
    {
        Ok(true) => {
            let save_file_name = params.storage.DefaultPCSaveFileName.clone();
            if let Some(root) = Path::new(&save_file_name).parent() {
                if let Err(error) = params.pc_save.PopulateSlotInfo(&mut params.storage, root) {
                    error!(target: "re3bevy::save", "saved slot but slot metadata refresh failed: {error}");
                }
            }
            C_PcSave::SyncMemoryCardSlots(&params.storage, &mut params.memory_card);
            params.memory_card.ClearError();
            params.request.state = PcSaveRequestState::Succeeded;
            info!(target: "re3bevy::save", "saved GTA III slot {}", slot + 1);
        }
        Ok(false) => {
            error!(target: "re3bevy::save", "PC save returned false for slot {}", slot + 1);
            params.request.state = PcSaveRequestState::Failed;
        }
        Err(error) => {
            error!(target: "re3bevy::save", "PC save failed for slot {}: {error}", slot + 1);
            params
                .memory_card
                .ApplyPcSaveStatus(params.pc_save.nErrorCode);
            params.request.state = PcSaveRequestState::Failed;
        }
    }
}

/// re3 CPools::SavePedPool saves only PLAYER1 peds outside vehicles.
/// Read the ECS owner at request time, never the saved mesh/pool cache.
fn save_live_player_pool(
    players: &CWorldPlayers,
    file_loader: &CFileLoader,
) -> Result<SaveBlock, &'static str> {
    let mut records = Vec::new();
    for (index, player) in players.iter_slots() {
        let Some(player) = player else { continue };
        if player.ped.ped_type != ePedType::PEDTYPE_PLAYER1
            || players.vehicle_relation(index as i32).is_some()
        {
            continue;
        }
        let model_index = player.ped.physical.entity.model_index;
        let model = file_loader.model_info.get_model_info(model_index as usize)
            .ok_or("live player model metadata unavailable")?;
        let mut model_name = [0u8; MAX_MODEL_NAME];
        let name = model.base().name.as_bytes();
        if name.is_empty() {
            return Err("live player special model name unavailable");
        }
        let name_len = name.len().min(MAX_MODEL_NAME);
        model_name[..name_len].copy_from_slice(&name[..name_len]);
        let base = CompatiblePedBaseSave {
            position: player.ped.physical.entity.get_position(),
            char_created_by: player.ped.char_created_by,
            health: player.vitality.health,
            armour: player.vitality.armour,
            weapons: std::array::from_fn(|index| {
                let weapon = &player.ped.weapons[index];
                CompatibleWeaponSave {
                    weapon_type: weapon.weapon_type as i32,
                    weapon_state: weapon.weapon_state as i32,
                    ammo_in_clip: weapon.ammo_in_clip,
                    ammo_total: weapon.ammo_total,
                    timer: weapon.timer,
                    add_rot_offset: weapon.add_rot_offset,
                }
            }),
            max_weapon_type_allowed: player.ped.max_weapon_type_allowed as i32,
        };
        let payload = encode_compatible_player_ped(&base, &CompatiblePlayerPedSave {
            max_stamina: player.max_stamina,
            targettable_objects: player.targettable_objects,
        });
        records.push((player, model_index, model_name, payload));
    }
    let mut writer = SaveBufWriter::new();
    writer.write(records.len() as i32);
    for (player, model_index, model_name, payload) in records {
        writer.write(player.ped.ped_type as u32);
        writer.write(model_index);
        writer.write(player.pool_handle);
        writer.write_bytes(&payload);
        writer.write(player.wanted.maximum_wanted_level);
        writer.write(player.wanted.maximum_chaos);
        writer.write_bytes(&model_name);
    }
    Ok(SaveBlock { kind: SaveBlockKind::PedPool, data: writer.into_inner() })
}

#[cfg(windows)]
fn local_system_time() -> SaveSystemTime {
    #[repr(C)]
    struct WinSystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(time: *mut WinSystemTime);
    }
    let mut time = WinSystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    unsafe { GetLocalTime(&mut time) };
    SaveSystemTime {
        year: time.year,
        month: time.month,
        day_of_week: time.day_of_week,
        day: time.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        milliseconds: time.milliseconds,
    }
}

#[cfg(not(windows))]
fn local_system_time() -> SaveSystemTime {
    use std::time::{SystemTime, UNIX_EPOCH};

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_seconds = elapsed.as_secs();
    let days = (total_seconds / 86_400) as i64;
    let seconds_of_day = total_seconds % 86_400;
    let (year, month, day) = civil_date_from_unix_days(days);
    SaveSystemTime {
        year: year as u16,
        month: month as u16,
        day_of_week: ((days + 4).rem_euclid(7)) as u16,
        day: day as u16,
        hour: (seconds_of_day / 3600) as u16,
        minute: ((seconds_of_day / 60) % 60) as u16,
        second: (seconds_of_day % 60) as u16,
        milliseconds: elapsed.subsec_millis() as u16,
    }
}

#[cfg(not(windows))]
fn civil_date_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}
