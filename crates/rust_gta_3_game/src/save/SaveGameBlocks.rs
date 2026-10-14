//! re3 reference: references/re-GTA/src/save/GenericGameStorage.cpp
//! re3 reference: references/re-GTA/src/control/PathFind.cpp
//! re3 reference: references/re-GTA/src/control/Restart.cpp
//!
//! Bridges already-ported subsystem save/load routines to the re3 PC save block order.

use crate::{
    audio::{AudioScriptObject::cAudioScriptObjectPool, DMAudio::cDMAudio},
    control::{
        Garages::CGarages, PathFind::CPathFind, Phones::CPhoneInfo, Pickups::CPickups,
        Restart::CRestart,
    },
    core::{
        PlayerInfo::CPlayerInfo, Pools::CPools, Radar::CRadar, Stats::CStats,
        Streaming::CStreaming, World, Zones::CTheZones,
    },
    objects::ParticleObject::CParticleObjectRuntime,
    peds::{Gangs::CGangs, PedType::CPedTypeRuntime},
    vehicles::{CarGen::CTheCarGenerators, Cranes::CCranes},
};

use super::GenericGameStorage::{GenericLoadBlocks, SaveBlock, SaveBlockKind, SaveStorageError};
use super::SaveRestorePlan::CSaveRestorePlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownSubsystemSaveBlocks {
    pub blocks: Vec<SaveBlock>,
}

impl KnownSubsystemSaveBlocks {
    pub fn from_pathfind_and_restart(pathfind: &CPathFind, restart: &CRestart) -> Self {
        Self {
            blocks: vec![save_paths_block(pathfind), save_restart_block(restart)],
        }
    }

    pub fn into_blocks(self) -> Vec<SaveBlock> {
        self.blocks
    }
}

pub fn save_paths_block(pathfind: &CPathFind) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Paths,
        data: pathfind.save_path_flags(),
    }
}

pub fn load_paths_block(
    pathfind: &mut CPathFind,
    block: &SaveBlock,
) -> Result<(), SaveStorageError> {
    expect_kind(block, SaveBlockKind::Paths)?;
    pathfind.load_path_flags(&block.data);
    Ok(())
}

pub fn save_restart_block(restart: &CRestart) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Restart,
        data: restart.save_all_restart_points(),
    }
}

pub fn load_restart_block(block: &SaveBlock) -> Result<CRestart, SaveStorageError> {
    expect_kind(block, SaveBlockKind::Restart)?;
    CRestart::load_all_restart_points(&block.data)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_gang_data_block(gangs: &CGangs) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::GangData,
        data: gangs.save_all_gang_data(),
    }
}

pub fn load_gang_data_block(block: &SaveBlock) -> Result<CGangs, SaveStorageError> {
    expect_kind(block, SaveBlockKind::GangData)?;
    CGangs::load_all_gang_data(&block.data)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_stats_block(stats: &CStats) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Stats,
        data: stats.save_stats(),
    }
}

pub fn load_stats_block(block: &SaveBlock) -> Result<CStats, SaveStorageError> {
    expect_kind(block, SaveBlockKind::Stats)?;
    CStats::load_stats(&block.data).map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_radar_blips_block(radar: &CRadar) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::RadarBlips,
        data: radar.save_all_radar_blips(),
    }
}

pub fn load_radar_blips_block(
    radar: &mut CRadar,
    block: &SaveBlock,
) -> Result<(), SaveStorageError> {
    expect_kind(block, SaveBlockKind::RadarBlips)?;
    radar
        .load_all_radar_blips(&block.data)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_streaming_block(streaming: &CStreaming) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Streaming,
        data: streaming.memory_card_save(),
    }
}

pub fn load_streaming_block(
    streaming: &mut CStreaming,
    block: &SaveBlock,
) -> Result<(), SaveStorageError> {
    expect_kind(block, SaveBlockKind::Streaming)?;
    streaming
        .memory_card_load(&block.data)
        .map_err(|len| SaveStorageError::Buffer(format!("streaming block size mismatch: {len}")))
}

pub fn save_car_generators_block(car_generators: &CTheCarGenerators) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::CarGenerators,
        data: car_generators.save_all_car_generators(),
    }
}

pub fn load_car_generators_block(block: &SaveBlock) -> Result<CTheCarGenerators, SaveStorageError> {
    expect_kind(block, SaveBlockKind::CarGenerators)?;
    CTheCarGenerators::load_all_car_generators(&block.data)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_phone_info_block(phone_info: &CPhoneInfo) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::PhoneInfo,
        data: phone_info.save(),
    }
}

pub fn load_phone_info_block(block: &SaveBlock) -> Result<CPhoneInfo, SaveStorageError> {
    expect_kind(block, SaveBlockKind::PhoneInfo)?;
    CPhoneInfo::load(&block.data).map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_ped_type_block(ped_type: &CPedTypeRuntime) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::PedType,
        data: ped_type.save(),
    }
}

pub fn load_ped_type_block(block: &SaveBlock) -> Result<CPedTypeRuntime, SaveStorageError> {
    expect_kind(block, SaveBlockKind::PedType)?;
    CPedTypeRuntime::load(&block.data).map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_audio_script_objects_block(audio_objects: &cAudioScriptObjectPool) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::AudioScriptObjects,
        data: audio_objects.SaveAllAudioScriptObjects(),
    }
}

pub fn load_audio_script_objects_block(
    block: &SaveBlock,
) -> Result<cAudioScriptObjectPool, SaveStorageError> {
    load_audio_script_objects_block_with_audio(block, None)
}

pub fn load_audio_script_objects_block_with_audio(
    block: &SaveBlock,
    dmaudio: Option<&mut cDMAudio>,
) -> Result<cAudioScriptObjectPool, SaveStorageError> {
    expect_kind(block, SaveBlockKind::AudioScriptObjects)?;
    cAudioScriptObjectPool::LoadAllAudioScriptObjects(&block.data, dmaudio)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_player_info_block(player_info: &CPlayerInfo) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::PlayerInfo,
        data: player_info.SavePlayerInfo(),
    }
}

pub fn load_player_info_block(block: &SaveBlock) -> Result<CPlayerInfo, SaveStorageError> {
    expect_kind(block, SaveBlockKind::PlayerInfo)?;
    CPlayerInfo::LoadPlayerInfo(&block.data)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_pickups_block(pickups: &CPickups) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Pickups,
        data: pickups.save(),
    }
}

pub fn load_pickups_block(block: &SaveBlock) -> Result<CPickups, SaveStorageError> {
    expect_kind(block, SaveBlockKind::Pickups)?;
    CPickups::load(&block.data).map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_garages_block(garages: &CGarages) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Garages,
        data: garages.save(),
    }
}

pub fn load_garages_block(block: &SaveBlock) -> Result<CGarages, SaveStorageError> {
    expect_kind(block, SaveBlockKind::Garages)?;
    CGarages::load(&block.data).map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_zones_block(zones: &CTheZones) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Zones,
        data: zones.save_all_zones(),
    }
}

pub fn save_world_zones_block(zones: &World::CTheZones) -> SaveBlock {
    use crate::core::Zones::{CZone, CZoneInfo, EZoneType, NUMMAPZONES, NUMZONES};
    use rust_gta_3_core::core::GameState::ELevelName;

    fn level(value: i32) -> ELevelName {
        match value {
            -1 => ELevelName::Ignore,
            1 => ELevelName::Industrial,
            2 => ELevelName::Commercial,
            3 => ELevelName::Suburban,
            _ => ELevelName::Generic,
        }
    }

    fn name_bytes(name: &str) -> [u8; crate::core::Zones::ZONE_NAME_LEN] {
        let mut out = [0_u8; crate::core::Zones::ZONE_NAME_LEN];
        let bytes = name.as_bytes();
        let len = bytes.len().min(out.len());
        out[..len].copy_from_slice(&bytes[..len]);
        out
    }

    fn zone_info(info: World::ZonePedInfo) -> CZoneInfo {
        CZoneInfo {
            car_density: info.car_density,
            car_threshold: info.car_threshold,
            cop_threshold: info.cop_threshold,
            gang_threshold: info.gang_threshold,
            ped_density: info.ped_density,
            cop_density: info.cop_density,
            gang_density: info.gang_density,
            ped_group: info.ped_group,
        }
    }

    let mut saved = CTheZones::init();
    saved.curr_level = level(zones.curr_level);
    saved.total_number_of_zones = zones.zones.len().min(NUMZONES) as u16;
    saved.total_number_of_map_zones = zones.map_zones.len().min(NUMMAPZONES) as u16;

    let mut next_zone_info = 0_usize;
    for (index, zone) in zones.zones.iter().take(NUMZONES).enumerate() {
        let day = next_zone_info;
        let night = (next_zone_info + 1).min(saved.zone_info_array.len() - 1);
        next_zone_info = (next_zone_info + 2).min(saved.zone_info_array.len());
        saved.zone_info_array[day] = zone_info(zone.ped_info_day);
        saved.zone_info_array[night] = zone_info(zone.ped_info_night);
        saved.zone_array[index] = CZone {
            name: name_bytes(&zone.name),
            minx: zone.min[0],
            miny: zone.min[1],
            minz: zone.min[2],
            maxx: zone.max[0],
            maxy: zone.max[1],
            maxz: zone.max[2],
            zone_type: match zone.zone_type {
                1 => EZoneType::Navig,
                2 => EZoneType::Info,
                3 => EZoneType::Mapzone,
                _ => EZoneType::Default,
            },
            level: level(zone.level),
            zoneinfo_day: day as i16,
            zoneinfo_night: night as i16,
            child: -1,
            parent: -1,
            next: -1,
        };
    }
    saved.total_number_of_zone_infos = next_zone_info.max(1) as u16;

    for (index, zone) in zones.map_zones.iter().take(NUMMAPZONES).enumerate() {
        saved.map_zone_array[index] = CZone {
            name: name_bytes(&zone.name),
            minx: zone.min[0],
            miny: zone.min[1],
            minz: zone.min[2],
            maxx: zone.max[0],
            maxy: zone.max[1],
            maxz: zone.max[2],
            zone_type: EZoneType::Mapzone,
            level: level(zone.level),
            zoneinfo_day: 0,
            zoneinfo_night: 0,
            child: -1,
            parent: -1,
            next: -1,
        };
    }
    save_zones_block(&saved)
}

pub fn load_zones_block(block: &SaveBlock) -> Result<CTheZones, SaveStorageError> {
    expect_kind(block, SaveBlockKind::Zones)?;
    CTheZones::load_all_zones(&block.data)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn load_zones_block_into_world(
    block: &SaveBlock,
    zones: &mut World::CTheZones,
) -> Result<(), SaveStorageError> {
    let loaded = load_zones_block(block)?;
    zones.zones = loaded
        .zone_array
        .iter()
        .take(loaded.total_number_of_zones as usize)
        .map(|zone| world_zone_from_save_zone(zone, &loaded.zone_info_array))
        .collect();
    zones.map_zones = loaded
        .map_zone_array
        .iter()
        .take(loaded.total_number_of_map_zones as usize)
        .map(|zone| world_zone_from_save_zone(zone, &loaded.zone_info_array))
        .collect();
    zones.curr_level = loaded.curr_level as i32;
    Ok(())
}

pub fn save_particles_block(particles: &CParticleObjectRuntime) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Particles,
        data: particles.save_particle(),
    }
}

pub fn load_particles_block(block: &SaveBlock) -> Result<CParticleObjectRuntime, SaveStorageError> {
    expect_kind(block, SaveBlockKind::Particles)?;
    CParticleObjectRuntime::load_particle(&block.data)
        .map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_cranes_block(cranes: &CCranes) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::Cranes,
        data: cranes.save(),
    }
}

pub fn load_cranes_block(block: &SaveBlock) -> Result<CCranes, SaveStorageError> {
    expect_kind(block, SaveBlockKind::Cranes)?;
    CCranes::load(&block.data).map_err(|error| SaveStorageError::Buffer(error.to_string()))
}

pub fn save_ped_pool_block(pools: &CPools) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::PedPool,
        data: pools.save_ped_pool_compatible(),
    }
}

pub fn save_vehicle_pool_block(pools: &CPools) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::VehiclePool,
        data: pools.save_vehicle_pool_compatible(),
    }
}

pub fn save_object_pool_block(pools: &CPools) -> SaveBlock {
    SaveBlock {
        kind: SaveBlockKind::ObjectPool,
        data: pools.save_object_pool_compatible(),
    }
}

pub fn build_post_load_restore_plan(
    pools: &CPools,
    pickups: Option<&CPickups>,
) -> CSaveRestorePlan {
    CSaveRestorePlan::build(pools, pickups)
}

pub fn build_post_load_restore_plan_from_generic_load(
    pools: &CPools,
    pickups: Option<&CPickups>,
    load: &GenericLoadBlocks,
) -> CSaveRestorePlan {
    CSaveRestorePlan::build_for_level(
        pools,
        pickups,
        load.simple_vars.curr_level,
        load.simple_vars.camera_position,
    )
}

pub fn find_block(blocks: &[SaveBlock], kind: SaveBlockKind) -> Option<&SaveBlock> {
    blocks.iter().find(|block| block.kind == kind)
}

pub fn apply_known_subsystem_blocks(
    blocks: &[SaveBlock],
    pathfind: &mut CPathFind,
    restart: &mut CRestart,
    gangs: Option<&mut CGangs>,
    stats: Option<&mut CStats>,
    radar: Option<&mut CRadar>,
    streaming: Option<&mut CStreaming>,
    car_generators: Option<&mut CTheCarGenerators>,
    phone_info: Option<&mut CPhoneInfo>,
    ped_type: Option<&mut CPedTypeRuntime>,
    audio_script_objects: Option<&mut cAudioScriptObjectPool>,
    player_info: Option<&mut CPlayerInfo>,
    pickups: Option<&mut CPickups>,
    garages: Option<&mut CGarages>,
    zones: Option<&mut CTheZones>,
    particles: Option<&mut CParticleObjectRuntime>,
    cranes: Option<&mut CCranes>,
    pools: Option<&mut CPools>,
) -> Result<(), SaveStorageError> {
    if let Some(block) = find_block(blocks, SaveBlockKind::Paths) {
        load_paths_block(pathfind, block)?;
    }
    if let Some(block) = find_block(blocks, SaveBlockKind::Restart) {
        *restart = load_restart_block(block)?;
    }
    if let Some(gangs) = gangs {
        if let Some(block) = find_block(blocks, SaveBlockKind::GangData) {
            *gangs = load_gang_data_block(block)?;
        }
    }
    if let Some(stats) = stats {
        if let Some(block) = find_block(blocks, SaveBlockKind::Stats) {
            *stats = load_stats_block(block)?;
        }
    }
    if let Some(radar) = radar {
        if let Some(block) = find_block(blocks, SaveBlockKind::RadarBlips) {
            load_radar_blips_block(radar, block)?;
        }
    }
    if let Some(streaming) = streaming {
        if let Some(block) = find_block(blocks, SaveBlockKind::Streaming) {
            load_streaming_block(streaming, block)?;
        }
    }
    if let Some(car_generators) = car_generators {
        if let Some(block) = find_block(blocks, SaveBlockKind::CarGenerators) {
            *car_generators = load_car_generators_block(block)?;
        }
    }
    if let Some(phone_info) = phone_info {
        if let Some(block) = find_block(blocks, SaveBlockKind::PhoneInfo) {
            *phone_info = load_phone_info_block(block)?;
        }
    }
    if let Some(ped_type) = ped_type {
        if let Some(block) = find_block(blocks, SaveBlockKind::PedType) {
            *ped_type = load_ped_type_block(block)?;
        }
    }
    if let Some(audio_script_objects) = audio_script_objects {
        if let Some(block) = find_block(blocks, SaveBlockKind::AudioScriptObjects) {
            *audio_script_objects = load_audio_script_objects_block(block)?;
        }
    }
    if let Some(player_info) = player_info {
        if let Some(block) = find_block(blocks, SaveBlockKind::PlayerInfo) {
            *player_info = load_player_info_block(block)?;
        }
    }
    if let Some(pickups) = pickups {
        if let Some(block) = find_block(blocks, SaveBlockKind::Pickups) {
            *pickups = load_pickups_block(block)?;
        }
    }
    if let Some(garages) = garages {
        if let Some(block) = find_block(blocks, SaveBlockKind::Garages) {
            *garages = load_garages_block(block)?;
        }
    }
    if let Some(zones) = zones {
        if let Some(block) = find_block(blocks, SaveBlockKind::Zones) {
            *zones = load_zones_block(block)?;
        }
    }
    if let Some(particles) = particles {
        if let Some(block) = find_block(blocks, SaveBlockKind::Particles) {
            *particles = load_particles_block(block)?;
        }
    }
    if let Some(cranes) = cranes {
        if let Some(block) = find_block(blocks, SaveBlockKind::Cranes) {
            *cranes = load_cranes_block(block)?;
        }
    }
    if let Some(pools) = pools {
        if let Some(block) = find_block(blocks, SaveBlockKind::PedPool) {
            pools
                .load_ped_pool_compatible(&block.data)
                .map_err(|error| SaveStorageError::Buffer(error.to_string()))?;
        }
        if let Some(block) = find_block(blocks, SaveBlockKind::VehiclePool) {
            pools
                .load_vehicle_pool_compatible(&block.data)
                .map_err(|error| SaveStorageError::Buffer(error.to_string()))?;
        }
        if let Some(block) = find_block(blocks, SaveBlockKind::ObjectPool) {
            pools
                .load_object_pool_compatible(&block.data)
                .map_err(|error| SaveStorageError::Buffer(error.to_string()))?;
        }
    }
    Ok(())
}

fn expect_kind(block: &SaveBlock, expected: SaveBlockKind) -> Result<(), SaveStorageError> {
    if block.kind == expected {
        Ok(())
    } else {
        let index = SaveBlockKind::RE3_ORDER
            .iter()
            .position(|kind| *kind == expected)
            .unwrap_or(usize::MAX);
        Err(SaveStorageError::InvalidBlockOrder { index })
    }
}

fn world_zone_from_save_zone(
    zone: &crate::core::Zones::CZone,
    infos: &[crate::core::Zones::CZoneInfo],
) -> World::CZone {
    fn ped_info(
        infos: &[crate::core::Zones::CZoneInfo],
        index: i16,
    ) -> World::ZonePedInfo {
        let Some(info) = usize::try_from(index).ok().and_then(|index| infos.get(index)) else {
            return World::ZonePedInfo::default();
        };
        World::ZonePedInfo {
            car_density: info.car_density,
            car_threshold: info.car_threshold,
            cop_threshold: info.cop_threshold,
            gang_threshold: info.gang_threshold,
            ped_density: info.ped_density,
            cop_density: info.cop_density,
            gang_density: info.gang_density,
            ped_group: info.ped_group,
        }
    }

    World::CZone {
        name: zone_name_string(&zone.name),
        zone_type: zone.zone_type as i32,
        min: [zone.minx, zone.miny, zone.minz],
        max: [zone.maxx, zone.maxy, zone.maxz],
        level: zone.level as i32,
        ped_info_day: ped_info(infos, zone.zoneinfo_day),
        ped_info_night: ped_info(infos, zone.zoneinfo_night),
    }
}

fn zone_name_string(name: &[u8; crate::core::Zones::ZONE_NAME_LEN]) -> String {
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    String::from_utf8_lossy(&name[..end]).into_owned()
}
